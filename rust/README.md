# Claw Code - Rust Implementation

This workspace contains the active Rust product for this repository: the installable `claw-code` CLI and its supporting crates.

For copy/paste usage examples, see [`../USAGE.md`](../USAGE.md).

The biggest end-user contribution in this fork is the OpenRouter path. The packaging and install work matter because they make that OpenRouter workflow usable from any project, not just from a repo checkout.

OpenRouter is the headline addition because it gives one key access to many model families and often exposes lower-cost and free-tier options when available. That makes this fork especially useful for students, low-budget builders, and teams that want to work in pricing tiers.

## Install

Before installing `claw-code`, make sure Rust is installed and `cargo` resolves to a working toolchain.

Recommended setup:

```bash
rustup default stable
cargo --version
rustc --version
```

If `rustup` says no default toolchain is configured yet, run:

```bash
rustup toolchain install stable
rustup default stable
```

Install the CLI globally from `main` with Cargo:

```bash
cargo install --git https://github.com/anupa-perera/claw-code claw-code --locked
```

Update an existing install with:

```bash
cargo install --git https://github.com/anupa-perera/claw-code claw-code --locked --force
```

If you are replacing an older package cleanly first:

```bash
claw-code logout
cargo uninstall claw-code
cargo uninstall rusty-claude-cli
```

Run `claw-code logout` or `claw logout` first if you want the CLI to clear saved provider credentials and the saved startup provider from `~/.claw/` before the package is removed.

After installation, start it from any project with either command:

```bash
claw-code
claw
```

The install includes the primary `claw-code` binary and the legacy `claw` compatibility alias. Both start the installed product.

### Global setup model

What must be true for the global workflow to work:

1. Cargo installs the executable into its global bin directory.
2. Your shell can find that directory through `PATH`.
3. `claw-code` or `claw` loads auth from env vars or saved credentials under `~/.claw/`.
4. Project-local config still lives beside your code in `.claw.json` or `.claw/settings.json`.

Cargo's default global bin directory is:

- Windows: `%USERPROFILE%\.cargo\bin`
- macOS/Linux: `~/.cargo/bin`

If `claw-code` is not found after install, add that directory to `PATH` and restart your shell.

For install, login, or Windows-specific issues, see [`../TROUBLESHOOTING.md`](../TROUBLESHOOTING.md).

Typical first-run flow:

```bash
claw-code
claw
```

On a fresh install, bare `claw-code` or `claw` now owns first-run onboarding. If no provider credentials exist yet, it asks for a provider, captures the needed credential, then continues into model selection and the REPL.

Useful direct login flows:

```bash
claw-code login --provider anthropic --auth oauth
claw-code login --provider openrouter
```

If your main goal is lower-cost usage, start with OpenRouter and pick a free-tier or budget-friendly model from the catalog after login.

## Quick start

```bash
# Inspect available commands
cd rust/
cargo run -p claw-code --bin claw-code -- --help

# Build the workspace
cargo build --workspace

# Run the interactive REPL
cargo run -p claw-code --bin claw-code --

# One-shot prompt
cargo run -p claw-code --bin claw-code -- prompt "explain this codebase"

# JSON output for automation
cargo run -p claw-code --bin claw-code -- --output-format json prompt "summarize src/main.rs"

# Inspect registered hooks and whether they are enabled
cargo run -p claw-code --bin claw-code -- hook list
```

## Authentication

The CLI supports two auth shapes:

- saved provider credentials via `claw-code login`
- environment variables for direct execution or automation

Supported environment variables:

- `ANTHROPIC_API_KEY`
- `ANTHROPIC_AUTH_TOKEN`
- `OPENAI_API_KEY`
- `OPENROUTER_API_KEY`
- `XAI_API_KEY`

Saved API keys live in `~/.claw/provider-auth.json`. Anthropic OAuth tokens live in `~/.claw/credentials.json`.

OpenRouter also supports a custom gateway:

```bash
export OPENROUTER_BASE_URL="https://openrouter.ai/api/v1"
```

If no `model` is configured yet, a fresh interactive REPL launch will ask you to choose a provider first and then a model. For non-interactive runs, pass `--model <model-id>` or set `"model"` in your `.claw/settings.json`.

## Windows launcher

If you are using this checkout directly on Windows, run the launcher from the repository root:

```powershell
powershell -ExecutionPolicy Bypass -File .\claw-code.ps1
```

On first run it will:

1. Ask which provider you want for this session.
2. Reuse a saved API key if one already exists, or prompt you to paste one.
3. Offer Anthropic browser OAuth when you choose Anthropic.
4. Start the Rust CLI so you can pick the model next.

The launcher stores user-level provider credentials under `~/.claw/` and keeps model selection interactive by clearing any saved user-level default model before launch. Workspace `.claw/settings.json` still wins if that project pins a model on purpose.

If you later add `claw-code` to your shell profile or install it globally, the same flow can be started with just:

```powershell
claw-code
```

## Mock parity harness

The workspace includes a deterministic Anthropic-compatible mock service and a clean-environment CLI harness for end-to-end parity checks.

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

- `crates/mock-anthropic-service/` - reusable mock Anthropic-compatible service
- `crates/rusty-claude-cli/tests/mock_parity_harness.rs` - clean-env CLI harness
- `scripts/run_mock_parity_harness.sh` - reproducible wrapper
- `scripts/run_mock_parity_diff.py` - scenario checklist plus PARITY mapping runner
- `mock_parity_scenarios.json` - scenario-to-PARITY manifest

## Features

| Feature | Status |
|---------|--------|
| Anthropic API + streaming | Yes |
| OpenRouter API + model discovery | Yes |
| Saved provider login/logout | Yes |
| Interactive REPL (rustyline) | Yes |
| Tool system (bash, read, write, edit, grep, glob) | Yes |
| Web tools (search, fetch) | Yes |
| Sub-agent orchestration | Yes |
| Todo tracking | Yes |
| Notebook editing | Yes |
| CLAUDE.md / project memory | Yes |
| Config file hierarchy (.claw.json / .claw/settings.json) | Yes |
| Permission system | Yes |
| MCP server lifecycle | Yes |
| Session persistence + resume | Yes |
| Extended thinking (thinking blocks) | Yes |
| Cost tracking + usage display | Yes |
| Git integration | Yes |
| Markdown terminal rendering (ANSI) | Yes |
| Model aliases (opus/sonnet/haiku) | Yes |
| Slash commands (/status, /compact, /clear, etc.) | Yes |
| Hooks (PreToolUse/PostToolUse) | Config only |
| Plugin system | Planned |
| Skills registry | Planned |

## Model aliases

Short names resolve to the latest model versions:

| Alias | Resolves To |
|-------|-------------|
| `opus` | `claude-opus-4-6` |
| `sonnet` | `claude-sonnet-4-6` |
| `haiku` | `claude-haiku-4-5-20251213` |

## CLI flags

```text
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
  login              Set up saved provider credentials
  logout             Clear saved provider credentials and the saved startup provider
  init               Initialize project config
  status             Show the current workspace status snapshot
  sandbox            Show the current sandbox isolation snapshot
  agents             Inspect agent definitions
  mcp                Inspect configured MCP servers
  skills             Inspect installed skills
  system-prompt      Render the assembled system prompt
```

For the current canonical help text, run `cargo run -p claw-code --bin claw-code -- --help`.

## Slash commands

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

## Workspace layout

```text
rust/
|-- Cargo.toml
|-- Cargo.lock
`-- crates/
    |-- api/
    |-- commands/
    |-- compat-harness/
    |-- mock-anthropic-service/
    |-- plugins/
    |-- runtime/
    |-- rusty-claude-cli/
    |-- telemetry/
    `-- tools/
```

### Crate responsibilities

- `api` - HTTP clients, SSE stream parsing, request/response types, provider auth
- `commands` - slash command definitions and help text generation
- `compat-harness` - tool and prompt manifest extraction from upstream TypeScript
- `mock-anthropic-service` - deterministic local Anthropic-compatible mock for parity tests
- `plugins` - plugin metadata, registries, and hook integration
- `runtime` - session state, config loading, permissions, MCP, prompts, usage tracking
- `claw-code` - REPL, one-shot prompt flow, streaming display, tool rendering, CLI parsing
- `telemetry` - session trace events and telemetry payloads
- `tools` - built-in tool implementations

## Stats

- about 20K lines of Rust
- 9 crates in the workspace
- primary binary: `claw-code`
- compatibility alias: `claw`
- startup model: interactive provider/model prompt when unset
- default permissions: `danger-full-access`

## License

See the repository root.
