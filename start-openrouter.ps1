param(
    [Parameter(ValueFromRemainingArguments = $true)]
    [string[]]$ClawArgs
)

$ErrorActionPreference = "Stop"

$launcherPath = Join-Path (Split-Path -Parent $MyInvocation.MyCommand.Path) "claw-code.ps1"
& $launcherPath -Provider OpenRouter @ClawArgs
exit $LASTEXITCODE
