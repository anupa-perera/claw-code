# Claw Code Troubleshooting

This document collects the install, login, and command-resolution issues that are most likely to show up on end-user machines, especially on Windows.

## Start Here

If the install succeeded but `claw-code` still does not behave correctly, the problem is usually one of these:

1. the wrong command is being launched
2. the global binary is installed but not on `PATH`
3. Windows is using `USERPROFILE` while the current build expected `HOME`
4. hidden API-key paste did not capture any characters
5. Windows or antivirus is locking the installed exe during uninstall or reinstall

## `cargo install` Fails Because Rust Is Not Set Up Yet

### Symptoms

- `cargo` is not recognized
- `rustup could not choose a version of cargo to run`
- `no default toolchain is configured`

### Why it happens

The package is installed with Cargo, so the machine needs a working Rust toolchain before `cargo install` can build anything.

### How to fix it

Install or activate the stable Rust toolchain:

```powershell
rustup toolchain install stable
rustup default stable
cargo --version
rustc --version
```

## `claw-code` Runs the Wrong Thing on Windows

### Symptoms

- `claw-code` opens a repo-local launcher instead of the installed binary
- `claw-code` says it cannot find `claw-code.ps1`
- `claw-code` behaves differently inside the repo checkout versus outside it

### Why it happens

On Windows PowerShell, a function or alias named `claw-code` can override the installed binary. A repo-local `claw-code.cmd` can also take precedence when you run commands from inside the checkout. The installed package also provides `claw`, which is often the simplest way to verify that the packaged binary itself works.

### How to fix it

- reserve `claw-code` for the installed global binary
- if you keep a local checkout helper, give it a different name such as `claw-code-local`
- test the global install from outside the repo checkout

Example:

```powershell
cd $HOME
mkdir claw-code-test -ErrorAction SilentlyContinue
cd claw-code-test
claw-code --help
claw --help
```

If you need to verify the exact command PowerShell is using:

```powershell
Get-Command claw-code -All
```

## `claw-code` Is Not Found After Install

### Why it happens

Cargo installs binaries into its user bin directory, but your shell only finds them if that directory is on `PATH`.

### How to fix it

Cargo's default bin directory is:

- Windows: `%USERPROFILE%\.cargo\bin`
- macOS/Linux: `~/.cargo/bin`

On Windows PowerShell, a temporary fix is:

```powershell
$env:Path += ";$HOME\.cargo\bin"
```

Then verify:

```powershell
claw-code --help
claw --help
```

## Login Fails Because `HOME` Is Not Set

### Symptoms

- login succeeds up to provider selection, then fails when saving credentials
- the error says `HOME is not set`

### Why it happens

Some Windows environments expose `USERPROFILE` but not `HOME`.

### What users should do

Install the latest build of this fork. Newer builds resolve the user home directory from:

1. `CLAW_CONFIG_HOME`
2. `HOME`
3. `USERPROFILE`
4. `HOMEDRIVE` + `HOMEPATH`

### Workaround for older builds

If you are on an older install, set one of these before running `claw-code login`:

```powershell
$env:HOME = $env:USERPROFILE
```

or

```powershell
$env:CLAW_CONFIG_HOME = "$env:USERPROFILE\.claw"
```

## Hidden API-Key Paste Captures Nothing

### Symptoms

- you choose a provider
- the hidden prompt appears
- pressing Enter gives `api key must not be empty`

### Why it happens

Some terminals do not reliably capture paste into hidden password prompts.

### How to use it correctly

- paste the key while the hidden prompt is active, even though nothing appears on screen
- press Enter once
- do not paste the key at the provider-selection prompt

### What newer builds do

Newer builds retry cleanly and offer a visible-input fallback if hidden input captures nothing.

### Workaround for older builds

Skip saved login and set the environment variable directly:

```powershell
$env:OPENROUTER_API_KEY = "sk-or-..."
claw-code
claw
```

## `cargo uninstall claw-code` Fails with `Access is denied`

### Why it happens

Windows cannot remove the installed exe while another process or security tool is holding the file open.

### Common causes

- `claw-code.exe` is still running
- a VS Code terminal still has the binary in use
- antivirus or endpoint protection is scanning the exe

### How to fix it

Stop any running processes:

```powershell
Get-Process claw-code, claw -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
```

Then retry:

```powershell
cargo uninstall claw-code
```

If it still fails:

- close open terminals that recently used `claw-code`
- wait a few seconds for antivirus scanning to finish
- retry
- reboot once if Windows still keeps the file locked

## Clean Reinstall

For a fully clean reinstall on Windows:

```powershell
Set-Location $HOME
Get-Process claw-code, claw -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
Remove-Item "$HOME\.claw\provider-auth.json" -Force -ErrorAction SilentlyContinue
Remove-Item "$HOME\.claw\credentials.json" -Force -ErrorAction SilentlyContinue
cargo uninstall claw-code
cargo uninstall rusty-claude-cli
Remove-Item "$HOME\.cargo\bin\claw-code.exe" -Force -ErrorAction SilentlyContinue
Remove-Item "$HOME\.cargo\bin\claw.exe" -Force -ErrorAction SilentlyContinue
```

Then install either from the local checkout:

```powershell
Set-Location "F:\sideProjects\claw-code\rust"
cargo install --path crates\rusty-claude-cli --locked --force
```

or from GitHub:

```powershell
cargo install --git https://github.com/anupa-perera/claw-code --branch main claw-code --locked --force
```

## OpenRouter Quick Path

If your goal is simply to start using OpenRouter quickly:

```powershell
$env:OPENROUTER_API_KEY = "sk-or-..."
claw-code
```

Or save it for future use:

```powershell
claw-code login --provider openrouter
```

If you are on Windows and want to verify the real global install, do that from outside the repo checkout.
