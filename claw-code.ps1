[CmdletBinding(PositionalBinding = $false)]
param(
    [Parameter(Position = 0, ValueFromRemainingArguments = $true)]
    [string[]]$ClawArgs,
    [ValidateSet("Anthropic", "OpenAI", "OpenRouter", "xAI")]
    [string]$Provider,
    [switch]$ResetAuth,
    [switch]$SetupOnly
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

if ($null -eq $ClawArgs) {
    $ClawArgs = @()
}

$repoRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
$manifestPath = Join-Path $repoRoot "rust\Cargo.toml"
$workspaceRoot = (Get-Location).Path
$legacyConfigHome = if ([string]::IsNullOrWhiteSpace($env:CLAW_CODE_HOME)) {
    Join-Path $HOME ".claw-code"
} else {
    $env:CLAW_CODE_HOME
}
$configHome = if (-not [string]::IsNullOrWhiteSpace($env:CLAW_CONFIG_HOME)) {
    $env:CLAW_CONFIG_HOME
} elseif (-not [string]::IsNullOrWhiteSpace($env:CLAW_CODE_HOME)) {
    $env:CLAW_CODE_HOME
} else {
    Join-Path $HOME ".claw"
}
$launcherStatePath = Join-Path $configHome "provider-auth.json"
$runtimeCredentialsPath = Join-Path $configHome "credentials.json"
$providerOrder = @("openrouter", "anthropic", "openai", "xai")
$providerDefinitions = @{
    openrouter = @{
        Key = "openrouter"
        Display = "OpenRouter"
        ApiEnv = "OPENROUTER_API_KEY"
        Description = "broad model marketplace"
        SupportsOAuth = $false
    }
    anthropic = @{
        Key = "anthropic"
        Display = "Anthropic"
        ApiEnv = "ANTHROPIC_API_KEY"
        Description = "Claude API key or browser login"
        SupportsOAuth = $true
    }
    openai = @{
        Key = "openai"
        Display = "OpenAI"
        ApiEnv = "OPENAI_API_KEY"
        Description = "OpenAI API key"
        SupportsOAuth = $false
    }
    xai = @{
        Key = "xai"
        Display = "xAI"
        ApiEnv = "XAI_API_KEY"
        Description = "xAI API key"
        SupportsOAuth = $false
    }
}

function Test-VisualCppToolchain {
    if (-not (Get-Command link.exe -ErrorAction SilentlyContinue)) {
        return $false
    }

    foreach ($libPath in ($env:LIB -split ";")) {
        if ([string]::IsNullOrWhiteSpace($libPath)) {
            continue
        }

        if (Test-Path (Join-Path $libPath "kernel32.lib")) {
            return $true
        }
    }

    return $false
}

function Import-VisualStudioDevEnvironment {
    param(
        [Parameter(Mandatory = $true)]
        [string]$VsDevCmdPath
    )

    cmd /c "`"$VsDevCmdPath`" -arch=x64 -host_arch=x64 && set" |
        ForEach-Object {
            if ($_ -match "^(.*?)=(.*)$") {
                Set-Item -Path ("Env:" + $matches[1]) -Value $matches[2]
            }
        }

    return (Test-VisualCppToolchain)
}

function Get-VsDevCmdCandidates {
    $candidates = New-Object System.Collections.Generic.List[string]
    $seen = @{}

    function Add-Candidate {
        param(
            [string]$Path
        )

        if ([string]::IsNullOrWhiteSpace($Path)) {
            return
        }
        if (-not (Test-Path $Path)) {
            return
        }
        if ($seen.ContainsKey($Path)) {
            return
        }

        $seen[$Path] = $true
        $candidates.Add($Path)
    }

    $programFilesX86 = ${env:ProgramFiles(x86)}
    if ([string]::IsNullOrWhiteSpace($programFilesX86)) {
        $programFilesX86 = $env:ProgramFiles
    }

    if (-not [string]::IsNullOrWhiteSpace($programFilesX86)) {
        $vswherePath = Join-Path $programFilesX86 "Microsoft Visual Studio\Installer\vswhere.exe"
        if (Test-Path $vswherePath) {
            $installPath = & $vswherePath -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath 2>$null
            if ($LASTEXITCODE -eq 0 -and -not [string]::IsNullOrWhiteSpace($installPath)) {
                Add-Candidate (Join-Path $installPath.Trim() "Common7\Tools\VsDevCmd.bat")
            }
        }
    }

    foreach ($root in @(${env:ProgramFiles(x86)}, $env:ProgramFiles)) {
        if ([string]::IsNullOrWhiteSpace($root)) {
            continue
        }

        foreach ($year in @("2022", "2019")) {
            foreach ($edition in @("BuildTools", "Community", "Professional", "Enterprise")) {
                Add-Candidate (Join-Path $root ("Microsoft Visual Studio\{0}\{1}\Common7\Tools\VsDevCmd.bat" -f $year, $edition))
            }
        }
    }

    return $candidates
}

function Ensure-DeveloperEnvironment {
    $cargoBin = Join-Path $HOME ".cargo\bin"

    if ((Test-Path $cargoBin) -and -not (($env:Path -split ";") -contains $cargoBin)) {
        $env:Path = "$cargoBin;$env:Path"
    }

    if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
        throw "cargo was not found. Open a new terminal or run `. `$PROFILE` first."
    }

    if (-not (Test-VisualCppToolchain)) {
        foreach ($vsDevCmdPath in (Get-VsDevCmdCandidates)) {
            if (Import-VisualStudioDevEnvironment -VsDevCmdPath $vsDevCmdPath) {
                break
            }
        }
    }
}

function ConvertTo-PlainValue {
    param(
        [Parameter(ValueFromPipeline = $true)]
        $Value
    )

    if ($null -eq $Value) {
        return $null
    }

    if ($Value -is [System.Collections.IDictionary]) {
        $result = @{}
        foreach ($entry in $Value.GetEnumerator()) {
            $result[$entry.Key] = ConvertTo-PlainValue $entry.Value
        }
        return $result
    }

    if ($Value -is [pscustomobject]) {
        $result = @{}
        foreach ($property in $Value.PSObject.Properties) {
            $result[$property.Name] = ConvertTo-PlainValue $property.Value
        }
        return $result
    }

    if (($Value -is [System.Collections.IEnumerable]) -and -not ($Value -is [string])) {
        $items = @()
        foreach ($item in $Value) {
            $items += ,(ConvertTo-PlainValue $item)
        }
        return $items
    }

    return $Value
}

function Get-EnvValue {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Name
    )

    $item = Get-Item -Path ("Env:" + $Name) -ErrorAction SilentlyContinue
    if ($null -eq $item) {
        return $null
    }

    $value = [string]$item.Value
    if ([string]::IsNullOrWhiteSpace($value)) {
        return $null
    }

    return $value
}

function Read-JsonObject {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Path,
        [Parameter(Mandatory = $true)]
        [hashtable]$DefaultValue
    )

    if (-not (Test-Path $Path)) {
        return $DefaultValue
    }

    $raw = Get-Content $Path -Raw
    if ([string]::IsNullOrWhiteSpace($raw)) {
        return $DefaultValue
    }

    try {
        $parsed = ConvertTo-PlainValue ($raw | ConvertFrom-Json)
    } catch {
        return $DefaultValue
    }

    if (-not ($parsed -is [System.Collections.IDictionary])) {
        return $DefaultValue
    }

    return $parsed
}

function Write-JsonObject {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Path,
        [Parameter(Mandatory = $true)]
        [System.Collections.IDictionary]$Value
    )

    $parent = Split-Path -Parent $Path
    if (-not [string]::IsNullOrWhiteSpace($parent)) {
        New-Item -ItemType Directory -Force -Path $parent | Out-Null
    }

    $json = $Value | ConvertTo-Json -Depth 8
    Set-Content -Path $Path -Value ($json + [Environment]::NewLine)
}

function Get-LauncherState {
    $state = Read-JsonObject -Path $launcherStatePath -DefaultValue @{
        version = 1
        providers = @{}
    }

    if (-not $state.ContainsKey("providers") -or -not ($state["providers"] -is [System.Collections.IDictionary])) {
        $state["providers"] = @{}
    }

    return $state
}

function Sync-LegacyConfigHome {
    if ($legacyConfigHome -eq $configHome) {
        return
    }

    if (-not (Test-Path $legacyConfigHome)) {
        return
    }

    New-Item -ItemType Directory -Force -Path $configHome | Out-Null

    foreach ($fileName in @("provider-auth.json", "credentials.json", "settings.json")) {
        $legacyPath = Join-Path $legacyConfigHome $fileName
        $currentPath = Join-Path $configHome $fileName
        if ((Test-Path $legacyPath) -and -not (Test-Path $currentPath)) {
            Copy-Item -Path $legacyPath -Destination $currentPath
        }
    }
}

function Save-LauncherState {
    param(
        [Parameter(Mandatory = $true)]
        [System.Collections.IDictionary]$State
    )

    Write-JsonObject -Path $launcherStatePath -Value $State
}

function Get-ProviderDefinition {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Value
    )

    $normalized = $Value.Trim().ToLowerInvariant()
    switch ($normalized) {
        "1" { return $providerDefinitions["openrouter"] }
        "2" { return $providerDefinitions["anthropic"] }
        "3" { return $providerDefinitions["openai"] }
        "4" { return $providerDefinitions["xai"] }
        "openrouter" { return $providerDefinitions["openrouter"] }
        "anthropic" { return $providerDefinitions["anthropic"] }
        "openai" { return $providerDefinitions["openai"] }
        "xai" { return $providerDefinitions["xai"] }
        default { return $null }
    }
}

function Read-SecretValue {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Prompt
    )

    $secure = Read-Host -Prompt $Prompt -AsSecureString
    $bstr = [Runtime.InteropServices.Marshal]::SecureStringToBSTR($secure)
    try {
        return [Runtime.InteropServices.Marshal]::PtrToStringBSTR($bstr)
    } finally {
        [Runtime.InteropServices.Marshal]::ZeroFreeBSTR($bstr)
    }
}

function Get-SavedApiKey {
    param(
        [Parameter(Mandatory = $true)]
        [hashtable]$ProviderDefinition
    )

    $state = Get-LauncherState
    $providers = $state["providers"]
    if (-not $providers.ContainsKey($ProviderDefinition.Key)) {
        return $null
    }

    $providerState = $providers[$ProviderDefinition.Key]
    if (-not ($providerState -is [System.Collections.IDictionary])) {
        return $null
    }

    if (-not $providerState.ContainsKey("apiKey")) {
        return $null
    }

    $apiKey = [string]$providerState["apiKey"]
    if ([string]::IsNullOrWhiteSpace($apiKey)) {
        return $null
    }

    return $apiKey
}

function Save-ApiKey {
    param(
        [Parameter(Mandatory = $true)]
        [hashtable]$ProviderDefinition,
        [Parameter(Mandatory = $true)]
        [string]$ApiKey
    )

    $state = Get-LauncherState
    if (-not $state["providers"].ContainsKey($ProviderDefinition.Key)) {
        $state["providers"][$ProviderDefinition.Key] = @{}
    }

    $state["providers"][$ProviderDefinition.Key]["apiKey"] = $ApiKey
    Save-LauncherState -State $state
}

function Test-SavedAnthropicOAuth {
    $credentials = Read-JsonObject -Path $runtimeCredentialsPath -DefaultValue @{}
    if (-not $credentials.ContainsKey("oauth")) {
        return $false
    }

    $oauth = $credentials["oauth"]
    if (-not ($oauth -is [System.Collections.IDictionary])) {
        return $false
    }

    if (-not $oauth.ContainsKey("accessToken")) {
        return $false
    }

    return -not [string]::IsNullOrWhiteSpace([string]$oauth["accessToken"])
}

function Reset-SavedAuth {
    Remove-Item $launcherStatePath -ErrorAction SilentlyContinue
    Remove-Item $runtimeCredentialsPath -ErrorAction SilentlyContinue
}

function Get-ProviderStatusText {
    param(
        [Parameter(Mandatory = $true)]
        [hashtable]$ProviderDefinition
    )

    $envValue = Get-EnvValue -Name $ProviderDefinition.ApiEnv
    if (-not [string]::IsNullOrWhiteSpace($envValue)) {
        return "env key ready"
    }

    if ($ProviderDefinition.Key -eq "anthropic") {
        $authToken = Get-EnvValue -Name "ANTHROPIC_AUTH_TOKEN"
        if (-not [string]::IsNullOrWhiteSpace($authToken)) {
            return "env OAuth token ready"
        }
        if (Test-SavedAnthropicOAuth) {
            return "saved OAuth ready"
        }
    }

    if (Get-SavedApiKey -ProviderDefinition $ProviderDefinition) {
        return "saved API key ready"
    }

    return "needs setup"
}

function Select-ProviderForSession {
    param(
        [string]$PresetProvider
    )

    if (-not [string]::IsNullOrWhiteSpace($PresetProvider)) {
        $definition = Get-ProviderDefinition -Value $PresetProvider
        if ($null -eq $definition) {
            throw "Unknown provider '$PresetProvider'."
        }
        return $definition
    }

    while ($true) {
        Write-Host ""
        Write-Host "Claw Code onboarding"
        Write-Host "  Step 1 of 3      choose the provider for this session."
        Write-Host "  What happens     if the provider is not set up yet, the launcher will ask for an API key or start browser login."
        Write-Host ""

        $displayIndex = 1
        foreach ($providerKey in $providerOrder) {
            $definition = $providerDefinitions[$providerKey]
            $status = Get-ProviderStatusText -ProviderDefinition $definition
            Write-Host ("  {0}. {1} - {2} ({3})" -f $displayIndex, $definition.Display, $definition.Description, $status)
            $displayIndex += 1
        }

        Write-Host ""
        $inputValue = Read-Host "Select a provider by number or 'q' to cancel"
        if ([string]::IsNullOrWhiteSpace($inputValue)) {
            continue
        }
        if ($inputValue.Trim().ToLowerInvariant() -eq "q") {
            throw "Launcher cancelled before a provider was selected."
        }

        $definition = Get-ProviderDefinition -Value $inputValue
        if ($null -ne $definition) {
            return $definition
        }

        Write-Host "That was not a valid provider selection."
    }
}

function Invoke-RustCli {
    param(
        [AllowNull()]
        [string[]]$Args = @()
    )

    if ($null -eq $Args) {
        $Args = @()
    }

    & cargo run --manifest-path $manifestPath -p claw-code --bin claw-code -- @Args
}

function Ensure-ApiKeyCredential {
    param(
        [Parameter(Mandatory = $true)]
        [hashtable]$ProviderDefinition
    )

    $envValue = Get-EnvValue -Name $ProviderDefinition.ApiEnv
    if (-not [string]::IsNullOrWhiteSpace($envValue)) {
        return @{
            Mode = "api-key"
            ApiKey = [string]$envValue
            Source = "env"
        }
    }

    $savedApiKey = Get-SavedApiKey -ProviderDefinition $ProviderDefinition
    if (-not [string]::IsNullOrWhiteSpace($savedApiKey)) {
        return @{
            Mode = "api-key"
            ApiKey = $savedApiKey
            Source = "saved"
        }
    }

    while ($true) {
        Write-Host ""
        Write-Host ("{0} setup" -f $ProviderDefinition.Display)
        Write-Host "  Step 2 of 3      connect this provider."
        Write-Host ("  Missing          {0}" -f $ProviderDefinition.ApiEnv)
        Write-Host "  Save behavior    the launcher stores the API key under your user home so future launches can reuse it."
        Write-Host ""
        $apiKey = Read-SecretValue -Prompt ("Paste your {0} (input hidden)" -f $ProviderDefinition.ApiEnv)
        $apiKey = [string]$apiKey

        if ([string]::IsNullOrWhiteSpace($apiKey)) {
            Write-Host "A non-empty API key is required."
            continue
        }

        Save-ApiKey -ProviderDefinition $ProviderDefinition -ApiKey $apiKey.Trim()
        return @{
            Mode = "api-key"
            ApiKey = $apiKey.Trim()
            Source = "prompt"
        }
    }
}

function Ensure-ProviderCredential {
    param(
        [Parameter(Mandatory = $true)]
        [hashtable]$ProviderDefinition
    )

    if ($ProviderDefinition.Key -eq "anthropic") {
        $authToken = Get-EnvValue -Name "ANTHROPIC_AUTH_TOKEN"
        if (-not [string]::IsNullOrWhiteSpace($authToken)) {
            return @{
                Mode = "oauth"
                AuthToken = $authToken
                Source = "env-token"
            }
        }

        $savedApiKey = Get-SavedApiKey -ProviderDefinition $ProviderDefinition
        $envApiKey = Get-EnvValue -Name "ANTHROPIC_API_KEY"

        if (-not [string]::IsNullOrWhiteSpace($envApiKey)) {
            return @{
                Mode = "api-key"
                ApiKey = [string]$envApiKey
                Source = "env"
            }
        }

        if (-not [string]::IsNullOrWhiteSpace($savedApiKey)) {
            return @{
                Mode = "api-key"
                ApiKey = $savedApiKey
                Source = "saved"
            }
        }

        if (Test-SavedAnthropicOAuth) {
            return @{
                Mode = "oauth"
                Source = "saved-oauth"
            }
        }

        while ($true) {
            Write-Host ""
            Write-Host "Anthropic setup"
            Write-Host "  Step 2 of 3      connect Anthropic."
            Write-Host "  1. API key       paste ANTHROPIC_API_KEY"
            Write-Host "  2. Browser login open the Claude OAuth flow"
            Write-Host ""

            $choice = Read-Host "Choose an auth method by number or 'q' to cancel"
            if ([string]::IsNullOrWhiteSpace($choice)) {
                continue
            }

            $choice = $choice.Trim().ToLowerInvariant()
            if ($choice -eq "q") {
                throw "Launcher cancelled during Anthropic setup."
            }
            if ($choice -eq "1") {
                return Ensure-ApiKeyCredential -ProviderDefinition $ProviderDefinition
            }
            if ($choice -eq "2") {
                Write-Host ""
                Write-Host "Opening the Claude login flow in your browser..."
                Invoke-RustCli -Args @("login", "--provider", "anthropic", "--auth", "oauth")
                if (-not (Test-SavedAnthropicOAuth)) {
                    throw "Claude OAuth login did not produce saved credentials."
                }

                return @{
                    Mode = "oauth"
                    Source = "oauth-login"
                }
            }

            Write-Host "That was not a valid auth choice."
        }
    }

    return Ensure-ApiKeyCredential -ProviderDefinition $ProviderDefinition
}

function Clear-ProviderEnvironment {
    foreach ($name in @(
        "ANTHROPIC_API_KEY",
        "ANTHROPIC_AUTH_TOKEN",
        "CLAW_STARTUP_PROVIDER",
        "OPENAI_API_KEY",
        "OPENROUTER_API_KEY",
        "XAI_API_KEY"
    )) {
        Remove-Item -Path ("Env:" + $name) -ErrorAction SilentlyContinue
    }
}

function Apply-ProviderEnvironment {
    param(
        [Parameter(Mandatory = $true)]
        [hashtable]$ProviderDefinition,
        [Parameter(Mandatory = $true)]
        [hashtable]$Credential
    )

    Clear-ProviderEnvironment

    if (($Credential.Mode -eq "api-key") -and -not [string]::IsNullOrWhiteSpace($Credential.ApiKey)) {
        Set-Item -Path ("Env:" + $ProviderDefinition.ApiEnv) -Value $Credential.ApiKey
    }

    if (
        ($ProviderDefinition.Key -eq "anthropic") -and
        ($Credential.Mode -eq "oauth") -and
        $Credential.ContainsKey("AuthToken") -and
        -not [string]::IsNullOrWhiteSpace([string]$Credential["AuthToken"])
    ) {
        Set-Item -Path "Env:ANTHROPIC_AUTH_TOKEN" -Value $Credential["AuthToken"]
    }

    Set-Item -Path "Env:CLAW_STARTUP_PROVIDER" -Value $ProviderDefinition.Key

    if ($ProviderDefinition.Key -eq "openrouter" -and [string]::IsNullOrWhiteSpace($env:OPENROUTER_BASE_URL)) {
        $env:OPENROUTER_BASE_URL = "https://openrouter.ai/api/v1"
    }
}

function Test-PassThroughLaunch {
    param(
        [string[]]$Args
    )

    if ($null -eq $Args -or $Args.Count -eq 0) {
        return $false
    }

    foreach ($arg in $Args) {
        if ($arg -in @("-h", "--help", "-V", "--version")) {
            return $true
        }
    }

    return $Args[0] -in @("help", "login", "logout", "version")
}

function Test-ExplicitModelArgument {
    param(
        [string[]]$Args
    )

    if ($null -eq $Args) {
        return $false
    }

    foreach ($arg in $Args) {
        if ($arg -eq "--model" -or $arg.StartsWith("--model=")) {
            return $true
        }
    }

    return $false
}

function Remove-UserModelSetting {
    $settingsPath = Join-Path $configHome "settings.json"
    $settings = Read-JsonObject -Path $settingsPath -DefaultValue @{}

    if (-not $settings.ContainsKey("model")) {
        return
    }

    $settings.Remove("model")
    Write-JsonObject -Path $settingsPath -Value $settings
}

function Get-WorkspacePinnedModel {
    $workspaceSettingsPath = Join-Path $workspaceRoot ".claw\settings.json"
    $settings = Read-JsonObject -Path $workspaceSettingsPath -DefaultValue @{}
    if (-not $settings.ContainsKey("model")) {
        return $null
    }

    $model = [string]$settings["model"]
    if ([string]::IsNullOrWhiteSpace($model)) {
        return $null
    }

    return $model
}

Ensure-DeveloperEnvironment
New-Item -ItemType Directory -Force -Path $configHome | Out-Null
Sync-LegacyConfigHome
$env:CLAW_CONFIG_HOME = $configHome

if ($ResetAuth) {
    Reset-SavedAuth
}

if (Test-PassThroughLaunch -Args $ClawArgs) {
    Invoke-RustCli -Args $ClawArgs
    exit $LASTEXITCODE
}

$providerDefinition = Select-ProviderForSession -PresetProvider $Provider
$credential = Ensure-ProviderCredential -ProviderDefinition $providerDefinition
Apply-ProviderEnvironment -ProviderDefinition $providerDefinition -Credential $credential

if (-not (Test-ExplicitModelArgument -Args $ClawArgs)) {
    Remove-UserModelSetting
}

$workspacePinnedModel = Get-WorkspacePinnedModel

Write-Host ""
Write-Host "Claw Code launch"
Write-Host ("  Step 3 of 3      starting the CLI with {0}." -f $providerDefinition.Display)
Write-Host ("  Credentials      {0}" -f $credential.Source)
if (-not (Test-ExplicitModelArgument -Args $ClawArgs)) {
    Write-Host "  Next             the Rust CLI should now ask you to pick a model for this session."
}
if (-not [string]::IsNullOrWhiteSpace($workspacePinnedModel)) {
    Write-Host ("  Workspace note   .claw/settings.json pins model '{0}', so that workspace setting will override the model picker." -f $workspacePinnedModel)
}
Write-Host ("  Config home      {0}" -f $configHome)
Write-Host ""

if ($SetupOnly) {
    return
}

Invoke-RustCli -Args $ClawArgs
exit $LASTEXITCODE
