# 🦞 Claw Code — Rust Implementation

A high-performance Rust rewrite of the Claw Code CLI agent harness. Built for speed, safety, and native tool execution.

For a task-oriented guide with copy/paste examples, see [`../USAGE.md`](../USAGE.md).

## Install

Install the CLI globally from `main` with Cargo:

```bash
cargo install --git https://github.com/anupa-perera/claw-code claw-code --locked
```

After installation, start it from any project with:

```bash
claw-code
```

The install includes the primary `claw-code` binary and the legacy `claw` compatibility alias.

### Global Setup

What must be true for the global workflow to work:

1. Cargo installs the executable into its global bin directory.
2. Your shell can find that directory through `PATH`.
3. `claw-code` loads auth from env vars or saved credentials.
4. Project-local config can still live beside your code in `.claw.json` or `.claw/settings.json`.

Cargo's default global bin directory is:

- Windows: `%USERPROFILE%\.cargo\bin`
- macOS/Linux: `~/.cargo/bin`

If `claw-code` is not found after install, add that directory to `PATH` and restart your shell.

Typical first-run flow:

```bash
claw-code login
claw-code
```

Update an existing install with:

```bash
cargo install --git https://github.com/anupa-perera/claw-code claw-code --locked --force
```

Or use environment variables instead:

```bash
export ANTHROPIC_API_KEY="sk-ant-..."
claw-code
```

Once installed, the intended model is:

- global auth and user-level state live under `~/.claw/`
- repo-level defaults live in `.claw.json`
- repo-local overrides live in `.claw/settings.json`
- sessions stay inside the project under `.claw/sessions/`

## Quick Start

```bash
# Inspect available commands
cd rust/
cargo run -p claw-code --bin claw-code -- --help

# Build the workspace
cargo build --workspace

# Run the interactive REPL
cargo run -p claw-code --bin claw-code -- --model claude-opus-4-6

# One-shot prompt
cargo run -p claw-code --bin claw-code -- prompt "explain this codebase"

# JSON output for automation
cargo run -p claw-code --bin claw-code -- --output-format json prompt "summarize src/main.rs"

# Inspect registered hooks and whether they are enabled
cargo run -p claw-code --bin claw-code -- hook list
```

### Simplest Windows Launcher

If you are using this checkout on Windows, run the launcher from the repository root:

```powershell
powershell -ExecutionPolicy Bypass -File .\claw-code.ps1
```

On first run it will:

1. Ask which provider you want for this session.
2. Reuse a saved API key if one already exists, or prompt you to paste one.
3. Offer Claude browser login when you choose Anthropic.
4. Start the Rust CLI so you can pick the model next.

The launcher stores user-level provider credentials under `~/.claw/` and keeps the model
selection interactive by clearing any saved user-level default model before launch. Workspace
`.claw/settings.json` still wins if that project pins a model on purpose.

If you later add `claw-code` to your shell profile or package it as a real installable command,
the same flow can be started with just:

```powershell
claw-code
```

## Configuration

Set your API credentials:

```bash
export ANTHROPIC_API_KEY="sk-ant-..."
# Or use a proxy
export ANTHROPIC_BASE_URL="https://your-proxy.com"

# Optional alternative providers
export OPENAI_API_KEY="sk-..."
export OPENROUTER_API_KEY="sk-or-..."
export XAI_API_KEY="xai-..."
```

OpenRouter also supports a custom gateway:

```bash
export OPENROUTER_BASE_URL="https://openrouter.ai/api/v1"
```

If no `model` is configured yet, a fresh interactive REPL launch will ask you to choose a provider
first and then a model. For non-interactive runs, pass `--model <model-id>` or set `"model"` in
your `.claw/settings.json`.

### First-Run OpenRouter Onboarding

If you are testing from this checkout on Windows, the cleanest path is:

```powershell
cd <path-to-claw-code>
powershell -ExecutionPolicy Bypass -File .\claw-code.ps1
```

At the picker:

1. Choose `OpenRouter`.
2. Type a provider or family name like `openai`, `anthropic`, `google`, `deepseek`, or `qwen` to narrow the list.
3. Use `n` and `p` to move between pages.
4. Type `all` to reset the filter back to the full catalog.
5. Paste an exact model id if you already know it, for example `openai/gpt-4o`.

If you prefer the older provider-specific helper, this still works too:

```powershell
powershell -ExecutionPolicy Bypass -File .\start-openrouter.ps1
```

Or authenticate via OAuth and let the CLI persist credentials locally:

```bash
cargo run -p claw-code --bin claw-code -- login
```

## Mock parity harness

The workspace now includes a deterministic Anthropic-compatible mock service and a clean-environment CLI harness for end-to-end parity checks.

```bash
cd rust/

# Run the scripted clean-environment harness
./scripts/run_mock_parity_harness.sh

# Or start the mock service manually for ad hoc CLI runs
cargo run -p mock-anthropic-service -- --bind 127.0.0.1:0
```

Harness coverage:

- `streaming_text`
- `read_file_roundtrip`
- `grep_chunk_assembly`
- `write_file_allowed`
- `write_file_denied`
- `multi_tool_turn_roundtrip`
- `bash_stdout_roundtrip`
- `bash_permission_prompt_approved`
- `bash_permission_prompt_denied`
- `plugin_tool_roundtrip`

Primary artifacts:

- `crates/mock-anthropic-service/` — reusable mock Anthropic-compatible service
- `crates/rusty-claude-cli/tests/mock_parity_harness.rs` — clean-env CLI harness
- `scripts/run_mock_parity_harness.sh` — reproducible wrapper
- `scripts/run_mock_parity_diff.py` — scenario checklist + PARITY mapping runner
- `mock_parity_scenarios.json` — scenario-to-PARITY manifest

## Features

| Feature | Status |
|---------|--------|
| Anthropic API + streaming | ✅ |
| OpenRouter API + model discovery | ✅ |
| OAuth login/logout | ✅ |
| Interactive REPL (rustyline) | ✅ |
| Tool system (bash, read, write, edit, grep, glob) | ✅ |
| Web tools (search, fetch) | ✅ |
| Sub-agent orchestration | ✅ |
| Todo tracking | ✅ |
| Notebook editing | ✅ |
| CLAUDE.md / project memory | ✅ |
| Config file hierarchy (.claw.json / .claw/settings.json) | ✅ |
| Permission system | ✅ |
| MCP server lifecycle | ✅ |
| Session persistence + resume | ✅ |
| Extended thinking (thinking blocks) | ✅ |
| Cost tracking + usage display | ✅ |
| Git integration | ✅ |
| Markdown terminal rendering (ANSI) | ✅ |
| Model aliases (opus/sonnet/haiku) | ✅ |
| Slash commands (/status, /compact, /clear, etc.) | ✅ |
| Hooks (PreToolUse/PostToolUse) | 🔧 Config only |
| Plugin system | 📋 Planned |
| Skills registry | 📋 Planned |

## Model Aliases

Short names resolve to the latest model versions:

| Alias | Resolves To |
|-------|------------|
| `opus` | `claude-opus-4-6` |
| `sonnet` | `claude-sonnet-4-6` |
| `haiku` | `claude-haiku-4-5-20251213` |

## CLI Flags

```
claw-code [OPTIONS] [COMMAND]

Options:
  --model MODEL                    Override the active model
  --dangerously-skip-permissions   Skip all permission checks
  --permission-mode MODE           Set read-only, workspace-write, or danger-full-access
  --allowedTools TOOLS             Restrict enabled tools
  --output-format FORMAT           Non-interactive output format (text or json)
  --resume SESSION                 Re-open a saved session or inspect it with slash commands
  --version, -V                    Print version and build information locally

Commands:
  prompt <text>      One-shot prompt (non-interactive)
  login              Authenticate via OAuth
  logout             Clear stored credentials
  init               Initialize project config
  status             Show the current workspace status snapshot
  sandbox            Show the current sandbox isolation snapshot
  agents             Inspect agent definitions
  mcp                Inspect configured MCP servers
  skills             Inspect installed skills
  system-prompt      Render the assembled system prompt
```

For the current canonical help text, run `cargo run -p claw-code --bin claw-code -- --help`.

## Slash Commands (REPL)

Tab completion expands slash commands, model aliases, permission modes, and recent session IDs.

| Command | Description |
|---------|-------------|
| `/help` | Show help |
| `/status` | Show session status (model, tokens, cost) |
| `/cost` | Show cost breakdown |
| `/compact` | Compact conversation history |
| `/clear` | Clear conversation |
| `/model [name]` | Show or switch model |
| `/permissions` | Show or switch permission mode |
| `/config [section]` | Show config (env, hooks, model) |
| `/memory` | Show CLAUDE.md contents |
| `/diff` | Show git diff |
| `/export [path]` | Export conversation |
| `/resume [id]` | Resume a saved conversation |
| `/session [id]` | Resume a previous session |
| `/version` | Show version |

See [`../USAGE.md`](../USAGE.md) for examples covering interactive use, JSON automation, sessions, permissions, and the mock parity harness.

## Workspace Layout

```
rust/
├── Cargo.toml              # Workspace root
├── Cargo.lock
└── crates/
    ├── api/                # Anthropic API client + SSE streaming
    ├── commands/           # Shared slash-command registry
    ├── compat-harness/     # TS manifest extraction harness
    ├── mock-anthropic-service/ # Deterministic local Anthropic-compatible mock
    ├── plugins/            # Plugin registry and hook wiring primitives
    ├── runtime/            # Session, config, permissions, MCP, prompts
    ├── rusty-claude-cli/   # Source folder for the `claw-code` package (`claw` alias included)
    ├── telemetry/          # Session tracing and usage telemetry types
    └── tools/              # Built-in tool implementations
```

### Crate Responsibilities

- **api** — HTTP client, SSE stream parser, request/response types, auth (API key + OAuth bearer)
- **commands** — Slash command definitions and help text generation
- **compat-harness** — Extracts tool/prompt manifests from upstream TS source
- **mock-anthropic-service** — Deterministic `/v1/messages` mock for CLI parity tests and local harness runs
- **plugins** — Plugin metadata, registries, and hook integration surfaces
- **runtime** — `ConversationRuntime` agentic loop, `ConfigLoader` hierarchy, `Session` persistence, permission policy, MCP client, system prompt assembly, usage tracking
- **claw-code** — REPL, one-shot prompt, streaming display, tool call rendering, CLI argument parsing
- **telemetry** — Session trace events and supporting telemetry payloads
- **tools** — Tool specs + execution: Bash, ReadFile, WriteFile, EditFile, GlobSearch, GrepSearch, WebSearch, WebFetch, Agent, TodoWrite, NotebookEdit, Skill, ToolSearch, REPL runtimes

## Stats

- **~20K lines** of Rust
- **9 crates** in workspace
- **Primary binary:** `claw-code`
- **Compatibility alias:** `claw`
- **Startup model:** interactive provider/model prompt when unset
- **Default permissions:** `danger-full-access`

## License

See repository root.
