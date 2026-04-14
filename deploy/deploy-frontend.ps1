param(
    [string]$Server = "47.103.220.84",
    [string]$User = "root",
    [string]$RemoteBase = "/root/workspace/mcx/backend",
    [string]$FrontendDir = (Join-Path (Split-Path $PSScriptRoot -Parent) "frontend"),
    [string]$ArchiveName = "frontend-dist.tar.gz",
    [switch]$SkipBuild
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

function Write-Step {
    param([string]$Message)
    Write-Host ""
    Write-Host $Message
}

function Assert-Command {
    param([string]$Name)
    if (-not (Get-Command $Name -ErrorAction SilentlyContinue)) {
        throw "Required command not found: $Name"
    }
}

function Get-RequiredEnv {
    param([string]$Name)
    $Value = [Environment]::GetEnvironmentVariable($Name, 'Process')
    if ([string]::IsNullOrWhiteSpace($Value)) {
        $Value = [Environment]::GetEnvironmentVariable($Name, 'User')
    }
    if ([string]::IsNullOrWhiteSpace($Value)) {
        $Value = [Environment]::GetEnvironmentVariable($Name, 'Machine')
    }
    if ([string]::IsNullOrWhiteSpace($Value)) {
        throw "Required environment variable not found: $Name"
    }
    return $Value
}

Assert-Command npm
Assert-Command tar
Assert-Command python

$Password = Get-RequiredEnv "DEPLOY_SSH_PASSWORD"

if (-not (Test-Path $FrontendDir)) {
    throw "Frontend directory not found: $FrontendDir"
}

$ArchivePath = Join-Path $PSScriptRoot $ArchiveName
$RemoteArchive = "$RemoteBase/$ArchiveName"
$RemoteDistDir = "$RemoteBase/dist"

Write-Host "=========================================="
Write-Host "  Welfare Store Frontend Deployment"
Write-Host "=========================================="
Write-Host "  Frontend: $FrontendDir"
Write-Host "  Server:   $Server"
Write-Host "  Remote:   $RemoteBase"
Write-Host "=========================================="

Push-Location $FrontendDir
try {
    if (-not $SkipBuild) {
        Write-Step "[1/4] Building frontend"
        if (-not (Test-Path "node_modules")) {
            Write-Host "node_modules not found, running npm install..."
            npm install
        }

        npm run build
    }
    else {
        Write-Step "[1/4] Skipping frontend build"
    }

    if (-not (Test-Path "dist")) {
        throw "dist directory not found after build"
    }
}
finally {
    Pop-Location
}

Write-Step "[2/4] Packaging dist"
if (Test-Path $ArchivePath) {
    Remove-Item $ArchivePath -Force
}

tar -czf $ArchivePath -C $FrontendDir dist

if (-not (Test-Path $ArchivePath)) {
    throw "Archive was not created: $ArchivePath"
}

Write-Host "Created archive: $ArchivePath"

$env:DEPLOY_SERVER = $Server
$env:DEPLOY_USER = $User
$env:DEPLOY_SSH_PASSWORD = $Password
$env:DEPLOY_REMOTE_BASE = $RemoteBase
$env:DEPLOY_ARCHIVE_PATH = (Resolve-Path $ArchivePath).Path
$env:DEPLOY_REMOTE_ARCHIVE = $RemoteArchive
$env:DEPLOY_REMOTE_DIST_DIR = $RemoteDistDir

Write-Step "[3/4] Uploading archive and extracting on server"
@'
import os
import posixpath
import sys

try:
    import paramiko
except ImportError as exc:
    raise SystemExit("paramiko is required: pip install paramiko") from exc

server = os.environ["DEPLOY_SERVER"]
user = os.environ["DEPLOY_USER"]
password = os.environ["DEPLOY_SSH_PASSWORD"]
remote_base = os.environ["DEPLOY_REMOTE_BASE"]
local_archive = os.environ["DEPLOY_ARCHIVE_PATH"]
remote_archive = os.environ["DEPLOY_REMOTE_ARCHIVE"]
remote_dist_dir = os.environ["DEPLOY_REMOTE_DIST_DIR"]

client = paramiko.SSHClient()
client.set_missing_host_key_policy(paramiko.AutoAddPolicy())

try:
    client.connect(server, username=user, password=password)
    sftp = client.open_sftp()
    stdin, stdout, stderr = client.exec_command(f"mkdir -p {remote_base}")
    exit_code = stdout.channel.recv_exit_status()
    if exit_code != 0:
        raise SystemExit(f"failed to create remote directory: {remote_base}")

    sftp.put(local_archive, remote_archive)
    sftp.close()

    commands = [
        f"rm -rf {remote_dist_dir}",
        f"cd {remote_base} && tar -xzf {os.path.basename(remote_archive)}",
        f"rm -f {remote_archive}",
        f"test -f {posixpath.join(remote_dist_dir, 'index.html')} && echo 'dist extraction ok'",
    ]

    for command in commands:
        stdin, stdout, stderr = client.exec_command(command)
        exit_code = stdout.channel.recv_exit_status()
        output = stdout.read().decode("utf-8", errors="ignore")
        error = stderr.read().decode("utf-8", errors="ignore")
        if output:
            print(output, end="")
        if error:
            print(error, end="", file=sys.stderr)
        if exit_code != 0:
            raise SystemExit(f"remote command failed ({exit_code}): {command}")

finally:
    client.close()
'@ | python -

Write-Step "[4/4] Verifying remote dist"
@'
import os
import paramiko

server = os.environ["DEPLOY_SERVER"]
user = os.environ["DEPLOY_USER"]
password = os.environ["DEPLOY_SSH_PASSWORD"]
remote_dist_dir = os.environ["DEPLOY_REMOTE_DIST_DIR"]

client = paramiko.SSHClient()
client.set_missing_host_key_policy(paramiko.AutoAddPolicy())
client.connect(server, username=user, password=password)

try:
    for command in [
        f"ls -la {remote_dist_dir}",
        f"ls -la {remote_dist_dir}/assets | head -n 5",
    ]:
        stdin, stdout, stderr = client.exec_command(command)
        exit_code = stdout.channel.recv_exit_status()
        output = stdout.read().decode("utf-8", errors="ignore")
        error = stderr.read().decode("utf-8", errors="ignore")
        if output:
            print(output, end="")
        if error:
            print(error, end="")
        if exit_code != 0:
            raise SystemExit(f"verification command failed ({exit_code}): {command}")
finally:
    client.close()
'@ | python -

Write-Host ""
Write-Host "Deployment complete."
Write-Host "Remote dist: $RemoteDistDir"
Write-Host "If nginx is already configured, reload it once if needed:"
Write-Host "  systemctl reload nginx"
