# Claw Code Usage

This guide covers the current Rust workspace under `rust/` and the `claw-code` CLI binary.

## Global install

Install from `main`:

```bash
cargo install --git https://github.com/anupa-perera/claw-code claw-code --locked
```

That installs `claw-code` as the primary executable and `claw` as a compatibility alias.

If the command is not found after install, add Cargo's bin directory to `PATH`:

- Windows: `%USERPROFILE%\.cargo\bin`
- macOS/Linux: `~/.cargo/bin`

To update an existing global install:

```bash
cargo install --git https://github.com/anupa-perera/claw-code claw-code --locked --force
```

If you are replacing an older install cleanly first:

```bash
cargo uninstall claw-code
cargo uninstall rusty-claude-cli
```

Then run the CLI from any project:

```bash
claw-code
```

## Prerequisites

- Rust toolchain with `cargo`
- One of:
  - `claw-code login` for saved provider credentials
  - `ANTHROPIC_API_KEY`
  - `ANTHROPIC_AUTH_TOKEN`
  - `OPENAI_API_KEY`
  - `OPENROUTER_API_KEY`
  - `XAI_API_KEY`
- Optional: `ANTHROPIC_BASE_URL` when targeting a proxy or local service

## Build the workspace

```bash
cd rust
cargo build --workspace
```

The primary CLI binary is available at `rust/target/debug/claw-code` after a debug build.

## Quick start

### Interactive REPL

```bash
cd rust
./target/debug/claw-code
```

### One-shot prompt

```bash
cd rust
./target/debug/claw-code prompt "summarize this repository"
```

### Shorthand prompt mode

```bash
cd rust
./target/debug/claw-code "explain rust/crates/runtime/src/lib.rs"
```

### JSON output for scripting

```bash
cd rust
./target/debug/claw-code --output-format json prompt "status"
```

## Model and permission controls

```bash
cd rust
./target/debug/claw-code --model sonnet prompt "review this diff"
./target/debug/claw-code --permission-mode read-only prompt "summarize Cargo.toml"
./target/debug/claw-code --permission-mode workspace-write prompt "update README.md"
./target/debug/claw-code --allowedTools read,glob "inspect the runtime crate"
```

Supported permission modes:

- `read-only`
- `workspace-write`
- `danger-full-access`

Model aliases currently supported by the CLI:

- `opus` -> `claude-opus-4-6`
- `sonnet` -> `claude-sonnet-4-6`
- `haiku` -> `claude-haiku-4-5-20251213`

## Authentication

### Provider-aware login

```bash
claw-code login
```

Examples:

```bash
claw-code login --provider anthropic --auth oauth
claw-code login --provider anthropic --auth api-key
claw-code login --provider openrouter
claw-code login --provider openai
claw-code login --provider xai
```

Saved API keys live in `~/.claw/provider-auth.json`. Anthropic OAuth tokens live in `~/.claw/credentials.json`.

### Environment variables

```bash
export ANTHROPIC_API_KEY="sk-ant-..."
export OPENROUTER_API_KEY="sk-or-..."
```

## Common operational commands

```bash
cd rust
./target/debug/claw-code status
./target/debug/claw-code sandbox
./target/debug/claw-code hook list
./target/debug/claw-code agents
./target/debug/claw-code mcp
./target/debug/claw-code skills
./target/debug/claw-code system-prompt --cwd .. --date 2026-04-04
```

## Session management

REPL turns are persisted under `.claw/sessions/` in the current workspace.

```bash
cd rust
./target/debug/claw-code --resume latest
./target/debug/claw-code --resume latest /status /diff
```

Useful interactive commands include `/help`, `/status`, `/cost`, `/config`, `/session`, `/model`, `/permissions`, and `/export`.

## Config file resolution order

Runtime config is loaded in this order, with later entries overriding earlier ones:

1. `~/.claw.json`
2. `~/.config/claw/settings.json`
3. `<repo>/.claw.json`
4. `<repo>/.claw/settings.json`
5. `<repo>/.claw/settings.local.json`

## Mock parity harness

The workspace includes a deterministic Anthropic-compatible mock service and parity harness.

```bash
cd rust
./scripts/run_mock_parity_harness.sh
```

Manual mock service startup:

```bash
cd rust
cargo run -p mock-anthropic-service -- --bind 127.0.0.1:0
```

## Verification

```bash
cd rust
cargo test --workspace
```

## Workspace overview

Current Rust crates:

- `api`
- `commands`
- `compat-harness`
- `mock-anthropic-service`
- `plugins`
- `runtime`
- `claw-code` (package in `crates/rusty-claude-cli/`)
- `telemetry`
- `tools`
