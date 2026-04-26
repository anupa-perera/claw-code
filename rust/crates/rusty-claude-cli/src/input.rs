use std::borrow::Cow;
use std::cell::RefCell;
use std::cmp;
use std::collections::BTreeSet;
use std::io::{self, IsTerminal, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crossterm::cursor::{MoveToColumn, MoveUp};
use crossterm::event::{
    self as terminal_event, Event as TerminalEvent, KeyCode as TerminalKeyCode,
};
use crossterm::event::{
    KeyEventKind as TerminalKeyEventKind, KeyModifiers as TerminalKeyModifiers,
};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, Clear, ClearType};
use crossterm::{execute, queue};
use rustyline::completion::{Completer, Pair};
use rustyline::error::ReadlineError;
use rustyline::highlight::{CmdKind, Highlighter};
use rustyline::hint::Hinter;
use rustyline::history::DefaultHistory;
use rustyline::validate::Validator;
use rustyline::{
    Cmd, CompletionType, ConditionalEventHandler, Config, Context, EditMode, Editor, Event,
    EventContext, EventHandler, Helper, KeyCode, KeyEvent, Modifiers,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReadOutcome {
    Submit(String),
    Cancel,
    Exit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlashCommandPickerEntry {
    pub command: String,
    pub insert_text: String,
    pub summary: String,
    pub execute_immediately: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum SlashCommandPickerOutcome {
    Submit(String),
    Prefill(String),
    Cancel,
}

const SLASH_COMMAND_PICKER_RENDER_LIMIT: usize = 10;

struct SlashCommandHelper {
    completions: Vec<String>,
    current_line: RefCell<String>,
}

impl SlashCommandHelper {
    fn new(completions: Vec<String>) -> Self {
        Self {
            completions: normalize_completions(completions),
            current_line: RefCell::new(String::new()),
        }
    }

    fn reset_current_line(&self) {
        self.current_line.borrow_mut().clear();
    }

    fn current_line(&self) -> String {
        self.current_line.borrow().clone()
    }

    fn set_current_line(&self, line: &str) {
        let mut current = self.current_line.borrow_mut();
        current.clear();
        current.push_str(line);
    }

    fn set_completions(&mut self, completions: Vec<String>) {
        self.completions = normalize_completions(completions);
    }
}

impl Completer for SlashCommandHelper {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Self::Candidate>)> {
        let Some(prefix) = slash_command_prefix(line, pos) else {
            return Ok((0, Vec::new()));
        };

        let matches = self
            .completions
            .iter()
            .filter(|candidate| !candidate.starts_with("/model ") || prefix.starts_with("/model "))
            .filter(|candidate| candidate.starts_with(prefix))
            .map(|candidate| Pair {
                display: candidate.clone(),
                replacement: candidate.clone(),
            })
            .collect();

        Ok((0, matches))
    }
}

impl Hinter for SlashCommandHelper {
    type Hint = String;
}

impl Highlighter for SlashCommandHelper {
    fn highlight<'l>(&self, line: &'l str, _pos: usize) -> Cow<'l, str> {
        self.set_current_line(line);
        Cow::Borrowed(line)
    }

    fn highlight_char(&self, line: &str, _pos: usize, _kind: CmdKind) -> bool {
        self.set_current_line(line);
        false
    }
}

impl Validator for SlashCommandHelper {}
impl Helper for SlashCommandHelper {}

struct SlashCommandPickerHandler {
    requested: Arc<AtomicBool>,
}

impl SlashCommandPickerHandler {
    fn new(requested: Arc<AtomicBool>) -> Self {
        Self { requested }
    }
}

impl ConditionalEventHandler for SlashCommandPickerHandler {
    fn handle(
        &self,
        _evt: &Event,
        _n: rustyline::RepeatCount,
        _positive: bool,
        ctx: &EventContext,
    ) -> Option<Cmd> {
        if should_trigger_slash_command_menu(ctx.line(), ctx.pos()) {
            self.requested.store(true, Ordering::SeqCst);
            return Some(Cmd::Interrupt);
        }

        None
    }
}

pub struct LineEditor {
    prompt: String,
    editor: Editor<SlashCommandHelper, DefaultHistory>,
    slash_picker_entries: Vec<SlashCommandPickerEntry>,
    slash_picker_requested: Arc<AtomicBool>,
}

impl LineEditor {
    #[must_use]
    pub fn new(prompt: impl Into<String>, completions: Vec<String>) -> Self {
        Self::with_slash_command_picker(prompt, completions, Vec::new())
    }

    #[must_use]
    pub fn with_slash_command_picker(
        prompt: impl Into<String>,
        completions: Vec<String>,
        slash_picker_entries: Vec<SlashCommandPickerEntry>,
    ) -> Self {
        let config = Config::builder()
            .completion_type(CompletionType::List)
            .edit_mode(EditMode::Emacs)
            .build();
        let mut editor = Editor::<SlashCommandHelper, DefaultHistory>::with_config(config)
            .expect("rustyline editor should initialize");
        editor.set_helper(Some(SlashCommandHelper::new(completions)));
        editor.bind_sequence(KeyEvent(KeyCode::Char('J'), Modifiers::CTRL), Cmd::Newline);
        editor.bind_sequence(KeyEvent(KeyCode::Enter, Modifiers::SHIFT), Cmd::Newline);
        let slash_picker_requested = Arc::new(AtomicBool::new(false));
        editor.bind_sequence(
            KeyEvent(KeyCode::Char('/'), Modifiers::NONE),
            EventHandler::Conditional(Box::new(SlashCommandPickerHandler::new(Arc::clone(
                &slash_picker_requested,
            )))),
        );

        Self {
            prompt: prompt.into(),
            editor,
            slash_picker_entries,
            slash_picker_requested,
        }
    }

    pub fn push_history(&mut self, entry: impl Into<String>) {
        let entry = entry.into();
        if entry.trim().is_empty() {
            return;
        }

        let _ = self.editor.add_history_entry(entry);
    }

    pub fn set_completions(&mut self, completions: Vec<String>) {
        if let Some(helper) = self.editor.helper_mut() {
            helper.set_completions(completions);
        }
    }

    pub fn set_slash_picker_entries(&mut self, entries: Vec<SlashCommandPickerEntry>) {
        self.slash_picker_entries = entries;
    }

    pub fn read_line(&mut self) -> io::Result<ReadOutcome> {
        if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
            return self.read_line_fallback();
        }

        self.read_line_with_editor(None)
    }

    fn read_line_with_editor(&mut self, initial: Option<&str>) -> io::Result<ReadOutcome> {
        if let Some(helper) = self.editor.helper_mut() {
            helper.reset_current_line();
        }
        self.slash_picker_requested.store(false, Ordering::SeqCst);

        let readline_result = match initial {
            Some(initial) => self
                .editor
                .readline_with_initial(&self.prompt, (initial, "")),
            None => self.editor.readline(&self.prompt),
        };

        match readline_result {
            Ok(line) => Ok(ReadOutcome::Submit(line)),
            Err(ReadlineError::Interrupted) => {
                if self.slash_picker_requested.swap(false, Ordering::SeqCst) {
                    self.clear_current_prompt_line()?;
                    return match self.read_slash_command_picker() {
                        Ok(SlashCommandPickerOutcome::Submit(line)) => {
                            Ok(ReadOutcome::Submit(line))
                        }
                        Ok(SlashCommandPickerOutcome::Prefill(line)) => {
                            self.read_line_with_prefill(line)
                        }
                        Ok(SlashCommandPickerOutcome::Cancel) => Ok(ReadOutcome::Cancel),
                        Err(error) => Err(error),
                    };
                }
                let has_input = !self.current_line().is_empty();
                self.finish_interrupted_read()?;
                if has_input {
                    Ok(ReadOutcome::Cancel)
                } else {
                    Ok(ReadOutcome::Exit)
                }
            }
            Err(ReadlineError::Eof) => {
                self.finish_interrupted_read()?;
                Ok(ReadOutcome::Exit)
            }
            Err(error) => Err(io::Error::other(error)),
        }
    }

    fn read_line_with_prefill(&mut self, initial: String) -> io::Result<ReadOutcome> {
        self.read_line_with_editor(Some(&initial))
    }

    fn current_line(&self) -> String {
        self.editor
            .helper()
            .map_or_else(String::new, SlashCommandHelper::current_line)
    }

    fn finish_interrupted_read(&mut self) -> io::Result<()> {
        if let Some(helper) = self.editor.helper_mut() {
            helper.reset_current_line();
        }
        let mut stdout = io::stdout();
        writeln!(stdout)
    }

    fn clear_current_prompt_line(&self) -> io::Result<()> {
        let mut stdout = io::stdout();
        execute!(stdout, MoveToColumn(0), Clear(ClearType::CurrentLine))
    }

    fn read_slash_command_picker(&mut self) -> io::Result<SlashCommandPickerOutcome> {
        if self.slash_picker_entries.is_empty() {
            return Ok(SlashCommandPickerOutcome::Prefill("/".to_string()));
        }

        enable_raw_mode()?;
        let mut stdout = io::stdout();
        let mut query = String::new();
        let mut selected_index = 0usize;
        let mut rendered_lines = 0usize;

        let result = loop {
            let filtered = filter_slash_command_picker_entries(&self.slash_picker_entries, &query);
            if filtered.is_empty() {
                selected_index = 0;
            } else {
                selected_index = cmp::min(selected_index, filtered.len() - 1);
            }

            rendered_lines = render_slash_command_picker_frame(
                &mut stdout,
                &self.slash_picker_entries,
                &query,
                &filtered,
                selected_index,
                rendered_lines,
            )?;

            let TerminalEvent::Key(key) = terminal_event::read()? else {
                continue;
            };
            if !matches!(
                key.kind,
                TerminalKeyEventKind::Press | TerminalKeyEventKind::Repeat
            ) {
                continue;
            }

            match key.code {
                TerminalKeyCode::Esc => break Ok(SlashCommandPickerOutcome::Cancel),
                TerminalKeyCode::Char('c')
                    if key.modifiers.contains(TerminalKeyModifiers::CONTROL) =>
                {
                    break Ok(SlashCommandPickerOutcome::Cancel);
                }
                TerminalKeyCode::Up => {
                    if !filtered.is_empty() {
                        selected_index = if selected_index == 0 {
                            filtered.len() - 1
                        } else {
                            selected_index - 1
                        };
                    }
                }
                TerminalKeyCode::Down => {
                    if !filtered.is_empty() {
                        selected_index = (selected_index + 1) % filtered.len();
                    }
                }
                TerminalKeyCode::Backspace => {
                    query.pop();
                    selected_index = 0;
                }
                TerminalKeyCode::Enter => {
                    if let Some(entry) = filtered
                        .get(selected_index)
                        .map(|index| &self.slash_picker_entries[*index])
                    {
                        if entry.execute_immediately {
                            break Ok(SlashCommandPickerOutcome::Submit(entry.insert_text.clone()));
                        }
                        break Ok(SlashCommandPickerOutcome::Prefill(
                            entry.insert_text.clone(),
                        ));
                    }
                    if query.trim().is_empty() {
                        break Ok(SlashCommandPickerOutcome::Cancel);
                    }
                    break Ok(SlashCommandPickerOutcome::Prefill(format!(
                        "/{}",
                        query.trim()
                    )));
                }
                TerminalKeyCode::Tab => {
                    if let Some(entry) = filtered
                        .get(selected_index)
                        .map(|index| &self.slash_picker_entries[*index])
                    {
                        query = entry.command.trim_start_matches('/').to_string();
                        selected_index = 0;
                    }
                }
                TerminalKeyCode::Char(character)
                    if !key.modifiers.contains(TerminalKeyModifiers::CONTROL)
                        && !key.modifiers.contains(TerminalKeyModifiers::ALT) =>
                {
                    if character != '/' {
                        query.push(character);
                        selected_index = 0;
                    }
                }
                _ => {}
            }
        };

        let clear_result = clear_slash_command_picker_frame(&mut stdout, rendered_lines);
        let disable_result = disable_raw_mode();

        match (result, clear_result, disable_result) {
            (Ok(outcome), Ok(()), Ok(())) => Ok(outcome),
            (Err(error), _, _) => Err(error),
            (_, Err(error), _) => Err(error),
            (_, _, Err(error)) => Err(error),
        }
    }

    fn read_line_fallback(&self) -> io::Result<ReadOutcome> {
        let mut stdout = io::stdout();
        write!(stdout, "{}", self.prompt)?;
        stdout.flush()?;

        let mut buffer = String::new();
        let bytes_read = io::stdin().read_line(&mut buffer)?;
        if bytes_read == 0 {
            return Ok(ReadOutcome::Exit);
        }

        while matches!(buffer.chars().last(), Some('\n' | '\r')) {
            buffer.pop();
        }
        Ok(ReadOutcome::Submit(buffer))
    }
}

fn filter_slash_command_picker_entries(
    entries: &[SlashCommandPickerEntry],
    query: &str,
) -> Vec<usize> {
    let needle = query.trim().trim_start_matches('/').to_ascii_lowercase();
    if needle.is_empty() {
        return (0..entries.len()).collect();
    }

    let mut prefix_matches = Vec::new();
    let mut contains_matches = Vec::new();
    let mut summary_matches = Vec::new();

    for (index, entry) in entries.iter().enumerate() {
        let command = entry.command.trim_start_matches('/').to_ascii_lowercase();
        let summary = entry.summary.to_ascii_lowercase();
        if command.starts_with(&needle) {
            prefix_matches.push(index);
        } else if command.contains(&needle) {
            contains_matches.push(index);
        } else if summary.contains(&needle) {
            summary_matches.push(index);
        }
    }

    prefix_matches.extend(contains_matches);
    prefix_matches.extend(summary_matches);
    prefix_matches
}

fn render_slash_command_picker_frame(
    out: &mut impl Write,
    entries: &[SlashCommandPickerEntry],
    query: &str,
    filtered: &[usize],
    selected_index: usize,
    previous_line_count: usize,
) -> io::Result<usize> {
    if previous_line_count > 0 {
        queue!(
            out,
            MoveUp(previous_line_count as u16),
            MoveToColumn(0),
            Clear(ClearType::FromCursorDown)
        )?;
    } else {
        queue!(out, MoveToColumn(0), Clear(ClearType::FromCursorDown))?;
    }

    let mut line_count = 0usize;
    line_count += write_picker_line(out, "Slash command picker")?;
    line_count += write_picker_line(
        out,
        &format!(
            "  Filter          /{}",
            if query.is_empty() { "" } else { query }
        ),
    )?;
    line_count += write_picker_line(
        out,
        "  Controls        type to filter, Up/Down to choose, Enter to run or insert, Esc to cancel",
    )?;
    line_count += write_picker_line(
        out,
        "  Tip             commands that need arguments are inserted so you can finish editing them",
    )?;
    line_count += write_picker_line(out, "")?;

    if filtered.is_empty() {
        line_count += write_picker_line(
            out,
            "  No matches      press Backspace to broaden the filter, or Enter to keep the typed command",
        )?;
    } else {
        let total = filtered.len();
        let start = selected_index.saturating_sub(SLASH_COMMAND_PICKER_RENDER_LIMIT / 2);
        let end = cmp::min(start + SLASH_COMMAND_PICKER_RENDER_LIMIT, total);
        let start = end.saturating_sub(cmp::min(end, SLASH_COMMAND_PICKER_RENDER_LIMIT));

        for (offset, entry_index) in filtered[start..end].iter().enumerate() {
            let absolute_index = start + offset;
            let entry = &entries[*entry_index];
            let prefix = if absolute_index == selected_index {
                ">"
            } else {
                " "
            };
            line_count += write_picker_line(
                out,
                &format!(" {prefix} {:<18} {}", entry.command, entry.summary),
            )?;
        }

        if total > end {
            line_count += write_picker_line(out, &format!("  ... {} more matches", total - end))?;
        }
    }

    out.flush()?;
    Ok(line_count)
}

fn clear_slash_command_picker_frame(out: &mut impl Write, line_count: usize) -> io::Result<()> {
    if line_count == 0 {
        return Ok(());
    }
    execute!(
        out,
        MoveUp(line_count as u16),
        MoveToColumn(0),
        Clear(ClearType::FromCursorDown)
    )?;
    out.flush()
}

fn write_picker_line(out: &mut impl Write, line: &str) -> io::Result<usize> {
    writeln!(out, "{line}")?;
    Ok(1)
}

fn slash_command_prefix(line: &str, pos: usize) -> Option<&str> {
    if pos != line.len() {
        return None;
    }

    let prefix = &line[..pos];
    if !prefix.starts_with('/') {
        return None;
    }

    Some(prefix)
}

fn should_trigger_slash_command_menu(line: &str, pos: usize) -> bool {
    line.is_empty() && pos == 0
}

fn normalize_completions(completions: Vec<String>) -> Vec<String> {
    let mut seen = BTreeSet::new();
    completions
        .into_iter()
        .filter(|candidate| candidate.starts_with('/'))
        .filter(|candidate| seen.insert(candidate.clone()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{
        filter_slash_command_picker_entries, should_trigger_slash_command_menu,
        slash_command_prefix, LineEditor, SlashCommandHelper, SlashCommandPickerEntry,
    };
    use rustyline::completion::Completer;
    use rustyline::highlight::Highlighter;
    use rustyline::history::{DefaultHistory, History};
    use rustyline::Context;

    #[test]
    fn extracts_terminal_slash_command_prefixes_with_arguments() {
        assert_eq!(slash_command_prefix("/he", 3), Some("/he"));
        assert_eq!(slash_command_prefix("/help me", 8), Some("/help me"));
        assert_eq!(
            slash_command_prefix("/session switch ses", 19),
            Some("/session switch ses")
        );
        assert_eq!(slash_command_prefix("hello", 5), None);
        assert_eq!(slash_command_prefix("/help", 2), None);
    }

    #[test]
    fn triggers_slash_command_menu_only_for_a_fresh_slash() {
        assert!(should_trigger_slash_command_menu("", 0));
        assert!(!should_trigger_slash_command_menu("/", 1));
        assert!(!should_trigger_slash_command_menu("hello", 5));
        assert!(!should_trigger_slash_command_menu("/status", 7));
    }

    #[test]
    fn slash_picker_filters_by_command_prefix_before_summary_matches() {
        let entries = vec![
            SlashCommandPickerEntry {
                command: "/status".to_string(),
                insert_text: "/status".to_string(),
                summary: "Show session status".to_string(),
                execute_immediately: true,
            },
            SlashCommandPickerEntry {
                command: "/stats".to_string(),
                insert_text: "/stats".to_string(),
                summary: "Show usage stats".to_string(),
                execute_immediately: true,
            },
            SlashCommandPickerEntry {
                command: "/diff".to_string(),
                insert_text: "/diff".to_string(),
                summary: "Inspect git changes and status".to_string(),
                execute_immediately: true,
            },
        ];

        assert_eq!(
            filter_slash_command_picker_entries(&entries, "sta"),
            vec![0, 1, 2]
        );
        assert_eq!(
            filter_slash_command_picker_entries(&entries, "git"),
            vec![2]
        );
    }

    #[test]
    fn completes_matching_slash_commands() {
        let helper = SlashCommandHelper::new(vec![
            "/help".to_string(),
            "/hello".to_string(),
            "/status".to_string(),
        ]);
        let history = DefaultHistory::new();
        let ctx = Context::new(&history);
        let (start, matches) = helper
            .complete("/he", 3, &ctx)
            .expect("completion should work");

        assert_eq!(start, 0);
        assert_eq!(
            matches
                .into_iter()
                .map(|candidate| candidate.replacement)
                .collect::<Vec<_>>(),
            vec!["/help".to_string(), "/hello".to_string()]
        );
    }

    #[test]
    fn completes_matching_slash_command_arguments() {
        let helper = SlashCommandHelper::new(vec![
            "/model".to_string(),
            "/model opus".to_string(),
            "/model sonnet".to_string(),
            "/session switch alpha".to_string(),
        ]);
        let history = DefaultHistory::new();
        let ctx = Context::new(&history);
        let (start, matches) = helper
            .complete("/model o", 8, &ctx)
            .expect("completion should work");

        assert_eq!(start, 0);
        assert_eq!(
            matches
                .into_iter()
                .map(|candidate| candidate.replacement)
                .collect::<Vec<_>>(),
            vec!["/model opus".to_string()]
        );
    }

    #[test]
    fn does_not_dump_model_variants_before_model_argument_completion() {
        let helper = SlashCommandHelper::new(vec![
            "/model".to_string(),
            "/model openai/gpt-4o".to_string(),
            "/memory".to_string(),
        ]);
        let history = DefaultHistory::new();
        let ctx = Context::new(&history);
        let (_, matches) = helper
            .complete("/m", 2, &ctx)
            .expect("completion should work");

        assert_eq!(
            matches
                .into_iter()
                .map(|candidate| candidate.replacement)
                .collect::<Vec<_>>(),
            vec!["/memory".to_string(), "/model".to_string()]
        );
    }

    #[test]
    fn ignores_non_slash_command_completion_requests() {
        let helper = SlashCommandHelper::new(vec!["/help".to_string()]);
        let history = DefaultHistory::new();
        let ctx = Context::new(&history);
        let (_, matches) = helper
            .complete("hello", 5, &ctx)
            .expect("completion should work");

        assert!(matches.is_empty());
    }

    #[test]
    fn tracks_current_buffer_through_highlighter() {
        let helper = SlashCommandHelper::new(Vec::new());
        let _ = helper.highlight("draft", 5);

        assert_eq!(helper.current_line(), "draft");
    }

    #[test]
    fn push_history_ignores_blank_entries() {
        let mut editor = LineEditor::new("> ", vec!["/help".to_string()]);
        editor.push_history("   ");
        editor.push_history("/help");

        assert_eq!(editor.editor.history().len(), 1);
    }

    #[test]
    fn set_completions_replaces_and_normalizes_candidates() {
        let mut editor = LineEditor::new("> ", vec!["/help".to_string()]);
        editor.set_completions(vec![
            "/model opus".to_string(),
            "/model opus".to_string(),
            "status".to_string(),
        ]);

        let helper = editor.editor.helper().expect("helper should exist");
        assert_eq!(helper.completions, vec!["/model opus".to_string()]);
    }
}
