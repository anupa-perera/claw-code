# Claw Code

<p align="center">
  <img src="assets/clawd-hero.jpeg" alt="Claw Code" width="280" />
</p>

A maintained fork of the Claw Code agent harness with an installable Rust CLI on `main`.

The active product in this repo is the Rust workspace in [`rust/`](./rust). The main outcome of this fork is that the CLI can now be installed globally and used from any project as `claw-code`, while still keeping `claw` as a compatibility alias.

The major additions in this fork are making the Rust CLI globally installable, making OpenRouter a first-class provider path, and streamlining provider onboarding so the installed product can be used directly from any project.

## What This Fork Achieves

- packages the Rust CLI as a real installable command
- standardizes the public command name as `claw-code`
- keeps `claw` available as a compatibility alias
- makes OpenRouter a first-class provider for lower-cost and free-tier model access
- adds provider-aware login and first-run onboarding
- aligns user and project state around `.claw`
- supports running from an installed binary instead of requiring a repo checkout
- keeps the Windows launcher and onboarding flow working with the packaged CLI
- preserves parity and test coverage around sessions, commands, and bundled plugins

## Major Additions

This fork is not just one change. It is a set of product changes that make the CLI easier to install, cheaper to use, and simpler to start with.

### 1. Installable Global CLI Package

The Rust CLI can now be installed globally from `main` and used from any project as `claw-code`, while still keeping `claw` as a compatibility alias. That changes the product from a repo-bound tool into something people can install and actually use as a normal CLI.

### 2. OpenRouter as a First-Class Provider

With `OPENROUTER_API_KEY` or `claw-code login --provider openrouter`, you can use the CLI without locking yourself into a single premium provider account. That matters if you are:

- a student trying to learn and build without burning through expensive tokens
- a low-budget indie developer shipping with tight cost limits
- a team working in pricing tiers and wanting cheaper models for everyday tasks
- someone frustrated by strict token limits or account gating on paid-first providers

OpenRouter is useful here because one key can expose many model families through one provider surface, and it often includes free-tier models or very low-cost models alongside paid options. Availability and rate limits can change on the OpenRouter side, so it is best to think of this as access to free-tier and budget-friendly options when available, not as a guarantee that every model is always free.

### 3. Provider-Aware Login and First-Run Onboarding

The CLI now supports provider-aware setup for Anthropic, OpenAI, OpenRouter, and xAI. On a fresh install, `claw-code` can guide the user through provider selection, credential setup, and model selection directly from the main entrypoint instead of forcing a separate setup step first.

### 4. Cleaner Installed Product Identity

This fork also cleans up the installed-product story around `.claw`, keeps compatibility with the older `claw` command, and makes the installed binary work without requiring a repo checkout layout.

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

For install, login, or Windows command-resolution issues, see [`TROUBLESHOOTING.md`](./TROUBLESHOOTING.md).

## Quick Start

Start the CLI from any project:

```bash
claw-code
```

On a fresh install, `claw-code` now handles first-run onboarding inline. If no provider credentials exist yet, it will ask you to choose a provider, collect the credential it needs, then continue straight into model selection and the interactive console.

`claw-code login` is still available as the explicit setup command. It is provider-aware, lets you choose Anthropic, OpenAI, OpenRouter, or xAI, and saves credentials under `~/.claw/` for future runs.

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
- [`TROUBLESHOOTING.md`](./TROUBLESHOOTING.md) for install, login, and Windows-specific issues
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
