param(
    [ValidateSet('all', 'backend', 'frontend')]
    [string]$Target = 'all',
    [string]$Server = "47.103.220.84",
    [string]$User = "root",
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

function Invoke-BackendDeploy {
    $env:DEPLOY_SERVER = $Server
    $env:DEPLOY_USER = $User
    & $BackendScript --deploy
}

function Invoke-FrontendDeploy {
    $frontendArgs = @(
        "-Server", $Server,
        "-User", $User
    )

    if ($SkipFrontendBuild) {
        $frontendArgs += "-SkipBuild"
    }

    & $FrontendScript @frontendArgs
}

Assert-File $BackendScript
Assert-File $FrontendScript

if (-not $env:DEPLOY_SSH_PASSWORD) {
    throw "Required environment variable not found: DEPLOY_SSH_PASSWORD"
}

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
