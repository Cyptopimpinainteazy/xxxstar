#!/usr/bin/env python3
"""Loopback-only Money Swarm development pilot, with persistent PostgreSQL.

No enrollment, external submissions, spending or public deployment. Credentials
are generated locally, never put in command arguments or committed files.
"""
import argparse
import json
import os
from pathlib import Path
import secrets
import shutil
import signal
import socket
import subprocess
import sys
import time
from urllib.error import HTTPError, URLError
from urllib.request import Request, urlopen

ROOT = Path(__file__).resolve().parents[2]
LOCAL = ROOT / ".local" / "money-swarm"
SECRETS = ROOT / ".env.money-swarm.local"
STATE = LOCAL / "processes.json"
API = "http://127.0.0.1:9086"
UI = "http://127.0.0.1:5176/money-swarm"
POSTGRES_IMAGE = "postgres:16-alpine@sha256:721873c34ceb9f8d8fc265984940dc982404c105f19ad51be9fdc5970a6080ea"


def configuration():
    LOCAL.mkdir(parents=True, exist_ok=True)
    LOCAL.chmod(0o700)
    if not SECRETS.exists():
        descriptor = os.open(SECRETS, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
        with os.fdopen(descriptor, "w") as output:
            output.write(f"X3_MONEY_DB_PASSWORD={secrets.token_hex(32)}\n")
            output.write(f"FUNDING_SWARM_ADMIN_TOKEN={secrets.token_hex(32)}\n")
    values = dict(line.split("=", 1) for line in SECRETS.read_text().splitlines() if line and not line.startswith("#"))
    if set(values) != {"X3_MONEY_DB_PASSWORD", "FUNDING_SWARM_ADMIN_TOKEN"} or any(
            len(value) != 64 or any(char not in "0123456789abcdef" for char in value) for value in values.values()):
        raise RuntimeError("Invalid local configuration; expected two generated 32-byte hex secrets")
    if SECRETS.stat().st_mode & 0o077:
        raise RuntimeError("Local credentials must be private: chmod 600 .env.money-swarm.local")
    # Compose consumes a mounted secret rather than interpolating a credential.
    # This private copy is also ignored with all local runtime artifacts.
    password_file = LOCAL / "postgres-password"
    descriptor = os.open(password_file, os.O_WRONLY | os.O_CREAT | os.O_TRUNC | os.O_NOFOLLOW, 0o600)
    with os.fdopen(descriptor, "w") as output:
        os.fchmod(output.fileno(), 0o600)
        output.write(values["X3_MONEY_DB_PASSWORD"])
    environment = dict(os.environ, **values)
    environment.update(X3_GATEWAY_LISTEN="127.0.0.1:9086", X3_FUNDING_DISCOVERY_ENABLED="1",
                       X3_GATEWAY_DATABASE_URL=f"postgres://x3_money:{values['X3_MONEY_DB_PASSWORD']}@127.0.0.1:15486/x3_money")
    return environment


def compose(environment, *arguments):
    # Native Docker is also supported where legacy Python docker-compose is
    # incompatible with the installed Docker/requests versions. No host tools
    # are installed or modified to start this pilot.
    name, owner = "x3-money-swarm-pilot-db", "local-pilot"
    check = subprocess.run(["docker", "inspect", "--format",
        '{{index .Config.Labels "io.x3.money-swarm"}}', name], capture_output=True, text=True)
    if check.returncode == 0 and check.stdout.strip() != owner:
        raise RuntimeError("Database container name belongs to another owner; refusing changes")
    if arguments == ("stop",):
        if check.returncode == 0:
            subprocess.run(["docker", "stop", name], check=True)
        return
    if check.returncode == 0:
        subprocess.run(["docker", "start", name], check=True)
        return
    volume = "x3-money-swarm-pilot-data"
    volume_check = subprocess.run(["docker", "volume", "inspect", "--format",
        '{{index .Labels "io.x3.money-swarm"}}', volume], capture_output=True, text=True)
    if volume_check.returncode == 0 and volume_check.stdout.strip() != owner:
        raise RuntimeError("Database volume belongs to another owner; refusing changes")
    subprocess.run(["docker", "volume", "create", "--label", f"io.x3.money-swarm={owner}", volume], check=True)
    # Docker imports the password from the environment; the secret is never an
    # argv value and never written to the process-state or output log.
    db_env = dict(environment, POSTGRES_PASSWORD=environment["X3_MONEY_DB_PASSWORD"])
    subprocess.run(["docker", "run", "--detach", "--name", name, "--restart", "unless-stopped",
        "--label", f"io.x3.money-swarm={owner}", "-p", "127.0.0.1:15486:5432",
        "--mount", f"type=volume,source={volume},target=/var/lib/postgresql/data",
        "-e", "POSTGRES_PASSWORD", "-e", "POSTGRES_USER=x3_money", "-e", "POSTGRES_DB=x3_money",
        POSTGRES_IMAGE], env=db_env, check=True)


def available(port):
    with socket.socket() as sock:
        sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        try:
            sock.bind(("127.0.0.1", port))
        except OSError as error:
            raise RuntimeError(f"Port {port} is already in use; existing services were preserved") from error


def healthy(url):
    try:
        with urlopen(url, timeout=2) as response:
            return response.status == 200
    except (URLError, TimeoutError):
        return False


def discovery_route_available():
    try:
        with urlopen(f"{API}/api/v1/admin/funding-swarm/discovery/grants-gov", timeout=2):
            return False  # GET must not be accepted by this POST-only route.
    except HTTPError as error:
        return error.code == 405
    except (URLError, TimeoutError):
        return False


def spawn(arguments, environment, log, cwd):
    with (LOCAL / log).open("ab") as output:
        return subprocess.Popen(arguments, cwd=cwd, env=environment, stdout=output,
                                stderr=subprocess.STDOUT, start_new_session=True).pid


def start(environment, no_build):
    if STATE.exists() and healthy(f"{API}/readyz") and healthy(UI) and discovery_route_available():
        status()
        return
    if STATE.exists():
        raise RuntimeError("A previous pilot state exists. Inspect status/logs and run stop before restarting")
    available(9086)
    available(5176)
    # Only this project's database is created/started; no existing roles change.
    compose(environment, "up", "-d")
    for _ in range(60):
        probe = subprocess.run(["docker", "exec", "x3-money-swarm-pilot-db", "pg_isready",
                                "-U", "x3_money", "-d", "x3_money"], capture_output=True)
        if probe.returncode == 0:
            break
        time.sleep(0.5)
    else:
        raise RuntimeError("Pilot PostgreSQL did not become ready")
    build_env = dict(environment, SKIP_WASM_BUILD="1", CARGO_BUILD_JOBS="8")
    if not no_build:
        subprocess.run(["cargo", "build", "--locked", "-p", "x3-gateway"], cwd=ROOT, env=build_env, check=True)
    target = Path(environment.get("CARGO_TARGET_DIR", ROOT / "target"))
    source = Path(environment.get("X3_GATEWAY_BINARY", target / "debug/x3-gateway"))
    binary = LOCAL / "x3-gateway"
    shutil.copy2(source, binary)
    app = ROOT / "apps/inferstructor-dashboard"
    if not (app / "node_modules/vite/bin/vite.js").is_file():
        subprocess.run(["npm", "ci", "--no-audit", "--no-fund"], cwd=app, check=True)
    processes = {}
    try:
        processes["gateway"] = spawn([str(binary)], environment, "gateway.log", ROOT)
        STATE.write_text(json.dumps(processes))
        for _ in range(120):
            if healthy(f"{API}/readyz"):
                break
            time.sleep(0.5)
        else:
            raise RuntimeError("Gateway did not become ready; inspect .local/money-swarm/gateway.log")
        if not discovery_route_available():
            raise RuntimeError("Gateway lacks the canonical discovery route; rebuild the current gateway binary")
        processes["dashboard"] = spawn(["node", str(app / "node_modules/vite/bin/vite.js"),
            "--host", "127.0.0.1", "--port", "5176", "--strictPort"], environment, "dashboard.log", app)
        STATE.write_text(json.dumps(processes))
        for _ in range(40):
            if healthy(UI):
                status()
                return
            time.sleep(0.25)
        raise RuntimeError("Dashboard did not become ready; inspect .local/money-swarm/dashboard.log")
    except BaseException:
        # Retain state and persistent DB for diagnosis/recovery; no false ready.
        print("Startup failed; process state and logs retained for inspection", file=sys.stderr)
        raise


def status():
    api_up, ui_up = healthy(f"{API}/readyz"), healthy(UI)
    discovery = discovery_route_available()
    print(json.dumps({"gateway_ready": api_up, "dashboard_reachable": ui_up,
                      "discovery_route_available": discovery,
                      "url": UI, "credentials_file": str(SECRETS),
                      "state_file": str(STATE)}, indent=2))
    return api_up and ui_up and discovery


def stop(environment):
    if STATE.exists():
        for name, pid in json.loads(STATE.read_text()).items():
            if type(pid) is not int or pid <= 1:
                raise RuntimeError("Invalid saved pilot process identity")
            command_file = Path(f"/proc/{pid}/cmdline")
            if not command_file.exists():
                continue
            command = command_file.read_bytes().replace(b"\0", b" ").decode()
            if not command.strip():
                continue  # An already-exited/zombie process has no command.
            expected = str(LOCAL / "x3-gateway") if name == "gateway" else str(ROOT / "apps/inferstructor-dashboard/node_modules/vite/bin/vite.js")
            if expected not in command:
                raise RuntimeError("Saved PID belongs to another process; refusing to signal it")
            os.killpg(pid, signal.SIGTERM)
        STATE.unlink()
    compose(environment, "stop")
    print("Pilot stopped; PostgreSQL data volume retained")


def refresh(environment, keyword):
    request = Request(f"{API}/api/v1/admin/funding-swarm/discovery/grants-gov",
                      data=json.dumps({"keyword": keyword, "pages": 1}).encode(),
                      headers={"Content-Type": "application/json", "x-admin-token": environment["FUNDING_SWARM_ADMIN_TOKEN"]})
    with urlopen(request, timeout=125) as response:
        print(response.read().decode())


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=["start", "stop", "status", "refresh"])
    parser.add_argument("--no-build", action="store_true", help="Use an already verified gateway binary")
    parser.add_argument("--keyword", default="distributed computing")
    arguments = parser.parse_args()
    try:
        if arguments.command == "status":
            sys.exit(0 if status() else 1)
        env = configuration()
        if arguments.command == "start": start(env, arguments.no_build)
        elif arguments.command == "stop": stop(env)
        else: refresh(env, arguments.keyword)
    except (OSError, RuntimeError, subprocess.CalledProcessError) as failure:
        # Never print exception commands or environments containing credentials.
        detail = str(failure) if isinstance(failure, RuntimeError) else type(failure).__name__
        print(f"Money Swarm pilot failed: {detail}. Inspect local logs and configuration.", file=sys.stderr)
        sys.exit(1)
