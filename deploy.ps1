param(
    [ValidateSet('all', 'backend', 'frontend')]
    [string]$Target = 'all',
    [string]$Server = "47.103.220.84",
    [string]$User = "root",
    [string]$SshPassword,
    [switch]$SkipFrontendBuild
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$DeployDir = Join-Path $PSScriptRoot "deploy"
$BackendScript = Join-Path $DeployDir "release.bat"
$FrontendScript = Join-Path $DeployDir "deploy-frontend.ps1"

function Assert-File {
    param([string]$Path)
    if (-not (Test-Path $Path)) {
        throw "Required deployment script not found: $Path"
    }
}

function Resolve-SshPassword {
    if (-not [string]::IsNullOrWhiteSpace($SshPassword)) {
        return $SshPassword
    }

    if (-not [string]::IsNullOrWhiteSpace($env:DEPLOY_SSH_PASSWORD)) {
        return $env:DEPLOY_SSH_PASSWORD
    }

    $securePassword = Read-Host "Enter SSH password for $User@$Server" -AsSecureString
    $bstr = [Runtime.InteropServices.Marshal]::SecureStringToBSTR($securePassword)
    try {
        return [Runtime.InteropServices.Marshal]::PtrToStringBSTR($bstr)
    }
    finally {
        if ($bstr -ne [IntPtr]::Zero) {
            [Runtime.InteropServices.Marshal]::ZeroFreeBSTR($bstr)
        }
    }
}

$ResolvedPassword = Resolve-SshPassword
$env:DEPLOY_SSH_PASSWORD = $ResolvedPassword

function Invoke-BackendDeploy {
    $env:DEPLOY_SERVER = $Server
    $env:DEPLOY_USER = $User
    if ([string]::IsNullOrWhiteSpace($env:NONINTERACTIVE)) {
        $env:NONINTERACTIVE = '1'
    }
    & $BackendScript --deploy
}

function Invoke-FrontendDeploy {
    $frontendDir = Join-Path $PSScriptRoot "frontend"

    if ($SkipFrontendBuild) {
        & $FrontendScript -Server $Server -User $User -FrontendDir $frontendDir -SkipBuild
    }
    else {
        & $FrontendScript -Server $Server -User $User -FrontendDir $frontendDir
    }
}

Assert-File $BackendScript
Assert-File $FrontendScript

Write-Host "=========================================="
Write-Host "  Welfare Store Deployment"
Write-Host "=========================================="
Write-Host "  Target: $Target"
Write-Host "  Server: $Server"
Write-Host "  User:   $User"
Write-Host "=========================================="

switch ($Target) {
    'backend' {
        Invoke-BackendDeploy
    }
    'frontend' {
        Invoke-FrontendDeploy
    }
    'all' {
        Invoke-BackendDeploy
        Invoke-FrontendDeploy
    }
}
