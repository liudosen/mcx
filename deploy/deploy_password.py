import os
import posixpath
import sys
import time
import winreg

import paramiko

if hasattr(sys.stdout, "reconfigure"):
    sys.stdout.reconfigure(encoding="utf-8", errors="replace", line_buffering=True)
if hasattr(sys.stderr, "reconfigure"):
    sys.stderr.reconfigure(encoding="utf-8", errors="replace", line_buffering=True)


def read_registry_value(root, subkey, name):
    try:
        with winreg.OpenKey(root, subkey) as key:
            return winreg.QueryValueEx(key, name)[0]
    except OSError:
        return None


def read_env(name: str) -> str:
    value = os.environ.get(name)
    if value:
        return value
    value = read_registry_value(winreg.HKEY_CURRENT_USER, r"Environment", name)
    if value:
        return value
    value = read_registry_value(
        winreg.HKEY_LOCAL_MACHINE,
        r"SYSTEM\CurrentControlSet\Control\Session Manager\Environment",
        name,
    )
    if value:
        return value
    return ""


def read_required(name: str) -> str:
    value = read_env(name)
    if not value:
        print(f"ERROR: Missing environment variable {name}", file=sys.stderr)
        sys.exit(1)
    return value


def log(message: str) -> None:
    print(message, flush=True)


def drain_channel(channel: paramiko.Channel) -> None:
    while channel.recv_ready():
        chunk = channel.recv(4096).decode("utf-8", "replace")
        if chunk:
            print(chunk, end="", flush=True)
    while channel.recv_stderr_ready():
        chunk = channel.recv_stderr(4096).decode("utf-8", "replace")
        if chunk:
            print(chunk, end="", file=sys.stderr, flush=True)


server = read_required("DEPLOY_SERVER")
user = read_required("DEPLOY_USER")
deploy_dir = read_required("DEPLOY_DIR")
port = int(read_required("DEPLOY_PORT"))
remote_system_env = read_required("REMOTE_SYSTEM_ENV")
remote_package = read_required("REMOTE_PACKAGE")
package_file = read_required("PACKAGE_FILE")
merged_env_file = read_required("MERGED_ENV_FILE")
app_name = read_required("APP_NAME")
ssh_password = read_required("DEPLOY_SSH_PASSWORD")


def run(ssh: paramiko.SSHClient, command: str, description: str) -> int:
    log(f"[deploy] {description} ...")
    stdin, stdout, stderr = ssh.exec_command(command)
    channel = stdout.channel

    while True:
        drain_channel(channel)
        if channel.exit_status_ready():
            drain_channel(channel)
            code = channel.recv_exit_status()
            if code == 0:
                log(f"[deploy] {description} done")
            else:
                log(f"[deploy] {description} failed with exit code {code}")
            return code
        time.sleep(0.05)


ssh = paramiko.SSHClient()
ssh.set_missing_host_key_policy(paramiko.AutoAddPolicy())
ssh.connect(
    server,
    username=user,
    password=ssh_password,
    look_for_keys=False,
    allow_agent=False,
    timeout=30,
)

try:
    log(f"[deploy] Connected to {server} as {user}")

    code = run(ssh, f"mkdir -p {deploy_dir}/logs", "Creating remote logs directory")
    if code != 0:
        sys.exit(code)

    log("[deploy] Uploading package and environment file ...")
    sftp = ssh.open_sftp()
    sftp.put(package_file, posixpath.join(deploy_dir, remote_package))
    sftp.put(merged_env_file, remote_system_env)
    sftp.close()
    log("[deploy] Upload complete")

    code = run(
        ssh,
        f"tr -d '\\r' < {remote_system_env} > {remote_system_env}.tmp && mv {remote_system_env}.tmp {remote_system_env} && "
        f"chmod 600 {remote_system_env}",
        "Normalizing remote environment file",
    )
    if code != 0:
        sys.exit(code)

    start_cmd = f"""set -e
cd {deploy_dir}
echo '[remote] Loading environment'
if [ -f /etc/mcx-system.env ]; then
    set -a
    . /etc/mcx-system.env
    set +a
fi
echo '[remote] Stopping existing process on port {port}'
PID=$(lsof -ti :{port} 2>/dev/null || true)
if [ -n "$PID" ]; then
    kill -15 $PID || true
    sleep 2
    PID=$(lsof -ti :{port} 2>/dev/null || true)
    if [ -n "$PID" ]; then
        kill -9 $PID || true
    fi
fi
echo '[remote] Extracting package'
tar -xzf {remote_package} -C .
chmod +x {app_name}
rm -f {remote_package}
echo '[remote] Starting service'
nohup ./{app_name} > logs/app.log 2>&1 &
echo '[remote] Waiting for service health'
READY=0
for _ in $(seq 1 60); do
    if curl -fsS --max-time 2 http://127.0.0.1:{port}/health > /dev/null 2>&1; then
        READY=1
        break
    fi
    sleep 1
done
if [ "$READY" -eq 1 ]; then
    echo '[remote] Deployment complete'
else
    echo '[remote] Service failed to start'
    tail -n 20 logs/app.log
    exit 1
fi"""

    code = run(ssh, start_cmd, "Restarting remote service")
    if code != 0:
        sys.exit(code)

    log("[deploy] Remote deployment finished")
finally:
    ssh.close()
