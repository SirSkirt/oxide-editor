param(
    [string]$KeyPath = "$HOME\.tauri\oxide-editor.key"
)

$ErrorActionPreference = 'Stop'
$ProjectRoot = Split-Path -Parent $PSScriptRoot
$ConfigPath = Join-Path $ProjectRoot 'src-tauri\tauri.conf.json'
$PublicKeyPath = "$KeyPath.pub"

function Invoke-NativeChecked {
    param(
        [Parameter(Mandatory = $true)][string]$Program,
        [Parameter(ValueFromRemainingArguments = $true)][string[]]$Arguments
    )

    & $Program @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "$Program failed with exit code $LASTEXITCODE. Fix the error above, then run updater setup again."
    }
}

Write-Host ''
Write-Host 'OXIDE UPDATER SETUP' -ForegroundColor DarkYellow
Write-Host '--------------------'
Write-Host "Key location: $KeyPath"
Write-Host ''

if (-not (Test-Path (Split-Path -Parent $KeyPath))) {
    New-Item -ItemType Directory -Force -Path (Split-Path -Parent $KeyPath) | Out-Null
}

if (-not (Test-Path $KeyPath)) {
    Write-Host 'Installing project dependencies...' -ForegroundColor DarkYellow
    Push-Location $ProjectRoot
    try {
        # npm.cmd avoids PowerShell execution-policy issues around npm.ps1 on Windows.
        Invoke-NativeChecked 'npm.cmd' 'install'

        Write-Host ''
        Write-Host 'Generating the updater signing keypair with the Tauri CLI...'
        Write-Host 'Choose a password when prompted, or leave it empty if you prefer an unencrypted CI key.' -ForegroundColor Yellow
        Invoke-NativeChecked 'npm.cmd' 'run' 'tauri' 'signer' 'generate' '--' '-w' $KeyPath
    }
    finally {
        Pop-Location
    }
} else {
    Write-Host 'Existing private updater key found. It will NOT be replaced.' -ForegroundColor Green
}

if (-not (Test-Path $PublicKeyPath)) {
    throw "The private key exists but its public key was not found at $PublicKeyPath. Do not delete or overwrite an existing private key casually. If this is a fresh failed setup with no key you need to preserve, remove the incomplete key and run this script again."
}

$PublicKey = (Get-Content $PublicKeyPath -Raw).TrimEnd()
$Config = Get-Content $ConfigPath -Raw | ConvertFrom-Json
$Config.plugins.updater.pubkey = $PublicKey
$Json = $Config | ConvertTo-Json -Depth 30
$Utf8NoBom = New-Object System.Text.UTF8Encoding($false)
[System.IO.File]::WriteAllText($ConfigPath, $Json, $Utf8NoBom)

Write-Host ''
Write-Host 'Updater public key written to src-tauri\tauri.conf.json.' -ForegroundColor Green
Write-Host 'The public key is safe to commit.'
Write-Host ''
Write-Host 'NEXT: add the PRIVATE key to GitHub:' -ForegroundColor DarkYellow
Write-Host '  Repository -> Settings -> Secrets and variables -> Actions'
Write-Host '  Secret name: TAURI_SIGNING_PRIVATE_KEY'
Write-Host "  Secret value: the complete contents of $KeyPath"
Write-Host ''
Write-Host 'If you gave the key a password, also add:'
Write-Host '  Secret name: TAURI_SIGNING_PRIVATE_KEY_PASSWORD'
Write-Host ''
Write-Host 'BACK UP THE PRIVATE KEY.' -ForegroundColor Red
Write-Host 'Installed copies of Oxide trust this keypair. Losing it means those copies cannot accept future automatic updates signed by a replacement key.'
Write-Host ''
