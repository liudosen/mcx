import os
import posixpath
import sys
import winreg

import paramiko


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


def run(ssh: paramiko.SSHClient, command: str):
    stdin, stdout, stderr = ssh.exec_command(command)
    code = stdout.channel.recv_exit_status()
    out = stdout.read().decode("utf-8", "ignore")
    err = stderr.read().decode("utf-8", "ignore")
    return code, out, err


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
    code, out, err = run(ssh, f"mkdir -p {deploy_dir}/logs")
    if out:
        print(out, end="")
    if err:
        print(err, end="", file=sys.stderr)
    if code != 0:
        sys.exit(code)

    sftp = ssh.open_sftp()
    sftp.put(package_file, posixpath.join(deploy_dir, remote_package))
    sftp.put(merged_env_file, remote_system_env)
    sftp.close()

    code, out, err = run(
        ssh,
        f"tr -d '\\r' < {remote_system_env} > {remote_system_env}.tmp && mv {remote_system_env}.tmp {remote_system_env} && "
        f"chmod 600 {remote_system_env}",
    )
    if out:
        print(out, end="")
    if err:
        print(err, end="", file=sys.stderr)
    if code != 0:
        sys.exit(code)

    start_cmd = f"""cd {deploy_dir} && \
if [ -f /etc/mcx-system.env ]; then set -a; . /etc/mcx-system.env; set +a; fi && \
PID=$(lsof -ti :{port} 2>/dev/null || true) && \
if [ -n "$PID" ]; then kill -15 $PID; sleep 2; PID=$(lsof -ti :{port} 2>/dev/null || true); if [ -n "$PID" ]; then kill -9 $PID; fi; fi && \
tar -xzf {remote_package} -C . && chmod +x {app_name} && rm -f {remote_package} && \
nohup ./{app_name} > logs/app.log 2>&1 & sleep 3 && \
if lsof -ti :{port} > /dev/null 2>&1; then echo Deployment complete; else echo Service failed to start; tail -n 20 logs/app.log; exit 1; fi"""

    code, out, err = run(ssh, start_cmd)
    if out:
        print(out, end="")
    if err:
        print(err, end="", file=sys.stderr)
    if code != 0:
        sys.exit(code)
finally:
    ssh.close()
