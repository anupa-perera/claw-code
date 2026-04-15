# Claw Code

<p align="center">
  <img src="assets/clawd-hero.jpeg" alt="Claw Code" width="280" />
</p>

A maintained fork of the Claw Code agent harness with an installable Rust CLI on `main`.

The active product in this repo is the Rust workspace in [`rust/`](./rust). The main outcome of this fork is that the CLI can now be installed globally and used from any project as `claw-code`, while still keeping `claw` as a compatibility alias.

The biggest end-user contribution of this fork is making OpenRouter a first-class path for using the CLI with one API key across many model providers, including lower-cost and free-tier model options when OpenRouter makes them available.

## What This Fork Achieves

- makes OpenRouter the most important new provider path in this fork
- packages the Rust CLI as a real installable command
- standardizes the public command name as `claw-code`
- keeps `claw` available as a compatibility alias
- makes OpenRouter a first-class provider for lower-cost and free-tier model access
- aligns user and project state around `.claw`
- supports running from an installed binary instead of requiring a repo checkout
- keeps the Windows launcher and onboarding flow working with the packaged CLI
- preserves parity and test coverage around sessions, commands, and bundled plugins

## Biggest Addition: OpenRouter

The biggest practical addition in this fork is the OpenRouter path.

With `OPENROUTER_API_KEY` or `claw-code login --provider openrouter`, you can use the CLI without locking yourself into a single premium provider account. That matters if you are:

- a student trying to learn and build without burning through expensive tokens
- a low-budget indie developer shipping with tight cost limits
- a team working in pricing tiers and wanting cheaper models for everyday tasks
- someone frustrated by strict token limits or account gating on paid-first providers

OpenRouter is useful here because one key can expose many model families through one provider surface, and it often includes free-tier models or very low-cost models alongside paid options. Availability and rate limits can change on the OpenRouter side, so it is best to think of this as access to free-tier and budget-friendly options when available, not as a guarantee that every model is always free.

Packaging the Rust CLI for global install is a big part of this fork too, but that install story mainly matters because it makes the OpenRouter workflow usable from any project without needing a repo checkout or a premium-provider-first setup.

## Install

Install from this repository's `main` branch:

```bash
cargo install --git https://github.com/anupa-perera/claw-code claw-code --locked
```

Update an existing install with:

```bash
cargo install --git https://github.com/anupa-perera/claw-code claw-code --locked --force
```

If you are replacing an older local/global build, uninstall first:

```bash
cargo uninstall claw-code
cargo uninstall rusty-claude-cli
```

That installs:

- `claw-code` as the primary executable
- `claw` as a compatibility alias

Cargo's default global bin directory is:

- Windows: `%USERPROFILE%\.cargo\bin`
- macOS/Linux: `~/.cargo/bin`

If `claw-code` is not found after install, add that directory to `PATH` and restart your shell.

## Quick Start

Authenticate, then start the CLI from any project:

```bash
claw-code login
claw-code
```

`claw-code login` is provider-aware. It lets you choose Anthropic, OpenAI, OpenRouter, or xAI, then saves credentials under `~/.claw/` for future runs.

If you want a direct login path, these are also supported:

```bash
claw-code login --provider anthropic --auth oauth
claw-code login --provider openrouter
```

If you want the most budget-friendly path, OpenRouter is usually the best starting point:

```bash
export OPENROUTER_API_KEY="sk-or-..."
claw-code
```

Then choose OpenRouter on startup and pick a free-tier or lower-cost model from the catalog.

Or use environment variables instead:

```bash
export ANTHROPIC_API_KEY="sk-ant-..."
claw-code
```

Supported environment variables include:

- `ANTHROPIC_API_KEY`
- `ANTHROPIC_AUTH_TOKEN`
- `OPENAI_API_KEY`
- `OPENROUTER_API_KEY`
- `XAI_API_KEY`

Useful commands:

```bash
claw-code --help
claw-code prompt "summarize this repository"
claw-code status
claw-code hook list
claw-code --resume latest
```

## Configuration Model

The current setup is:

- user-level state and saved credentials live under `~/.claw/`
- repo defaults live in `.claw.json`
- repo-local overrides live in `.claw/settings.json`
- sessions are stored in `.claw/sessions/`

This matters because a globally installed CLI still needs a stable separation between user state and project state. The package install provides the binary, while each project continues to own its own local config and sessions.

## Windows Workflow

If you are using this repo directly on Windows, you can still launch through the root script:

```powershell
powershell -ExecutionPolicy Bypass -File .\claw-code.ps1
```

That path is useful when working from the checkout itself. Once the package is installed globally, the intended command is simply:

```powershell
claw-code
```

## Repository Guide

The main places to start are:

- [`rust/README.md`](./rust/README.md) for crate-level architecture and runtime details
- [`USAGE.md`](./USAGE.md) for copy/paste usage examples
- [`ROADMAP.md`](./ROADMAP.md) for planned work
- [`PARITY.md`](./PARITY.md) for parity tracking
- [`PHILOSOPHY.md`](./PHILOSOPHY.md) for the broader project motivation

## Credits

This fork builds on the original Claw Code work and the surrounding UltraWorkers ecosystem.

Original credit belongs to:

- Bellman / Yeachan Heo
- Yeongyu
- the contributors behind Claw Code and the related UltraWorkers projects

Related upstream and ecosystem projects:

- [clawhip](https://github.com/Yeachan-Heo/clawhip)
- [oh-my-openagent](https://github.com/code-yeongyu/oh-my-openagent)
- [oh-my-claudecode](https://github.com/Yeachan-Heo/oh-my-claudecode)
- [oh-my-codex](https://github.com/Yeachan-Heo/oh-my-codex)

This repository does not claim authorship of the original system design, and it is not affiliated with or endorsed by Anthropic.
