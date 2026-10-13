#!/usr/bin/env python3
"""X3 build-cluster node tooling. Standard library only, so a fresh node can run it.

    x3cluster.py inventory [--json]             describe this machine
    x3cluster.py bootstrap --role ROLE [--apply] [--lan]
                                                write node metadata; for role=gpu also
                                                detect GPUs, plan/adopt one Ollama worker
                                                per GPU and emit a router registration
    x3cluster.py bench                          CPU / disk / network / Rust build baseline
    x3cluster.py health [--json]                cluster table from inventory.json
    x3cluster.py gate                           health + evidence; nonzero on failure

Anything that needs root is never run: it is written to
~/.config/x3-cluster/staged-privileged.sh for an operator to review and run.
Evidence goes to audit-artifacts/x3-cluster/<commit>/. Every result is labelled
PHYSICAL (measured on real hardware) or LOCAL (this node only).
"""
import argparse
import datetime as dt
import hashlib
import json
import os
import platform
import re
import shutil
import socket
import subprocess
import sys
import tempfile
import time
import urllib.request
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[1]
CONFIG = Path.home() / ".config" / "x3-cluster"
USER_UNITS = Path.home() / ".config" / "systemd" / "user"
ROLES = ("control", "gpu", "build", "sim", "data", "net", "ops")
ROLE_TOOLS = {
    "common": ["git", "tmux", "jq", "curl", "iperf3", "ethtool", "ssh"],
    "gpu": ["nvidia-smi", "ollama", "nvcc"],
    "build": ["cargo", "rustc", "sccache", "clang", "cmake", "pkg-config", "docker"],
    "sim": ["cargo", "cargo-fuzz"],
    "data": ["psql", "pg_dump"],
    "net": ["docker", "tc", "anvil", "solana-test-validator"],
    "ops": ["prometheus", "node_exporter"],
    "control": ["cargo", "code"],
}
OLLAMA_BASE_PORT = 11434
# Models a GPU worker advertises, in preference order. Tool-capable models are
# what Codex traffic needs; the coder model serves tool-free requests.
ADVERTISED_MODELS = {"qwen3:8b": {"supports_tools": True}, "qwen2.5-coder:7b": {"supports_tools": False}}


def run(cmd, timeout=30):
    try:
        out = subprocess.run(cmd, capture_output=True, text=True, timeout=timeout, shell=isinstance(cmd, str))
        return out.returncode, out.stdout.strip()
    except (OSError, subprocess.TimeoutExpired) as exc:
        return 127, str(exc)


def now():
    return dt.datetime.now(dt.timezone.utc).isoformat(timespec="seconds")


def git_identity():
    _, head = run(["git", "-C", str(REPO), "rev-parse", "HEAD"])
    _, branch = run(["git", "-C", str(REPO), "branch", "--show-current"])
    _, status = run(["git", "-C", str(REPO), "status", "--porcelain"])
    return {"commit": head, "branch": branch, "dirty": bool(status)}


# ---------------------------------------------------------------- inventory

def gpus():
    code, out = run(["nvidia-smi", "--query-gpu=index,uuid,name,pci.bus_id,memory.total,compute_cap,driver_version,"
                     "pcie.link.gen.current,pcie.link.width.current,power.limit,temperature.gpu",
                     "--format=csv,noheader,nounits"])
    if code != 0:
        return []
    keys = ["smi_index", "uuid", "name", "pci", "mem_mib", "compute_cap", "driver", "pcie_gen", "pcie_width",
            "power_limit_w", "temp_c"]
    result = []
    for line in out.splitlines():
        row = dict(zip(keys, (v.strip() for v in line.split(","))))
        row["mem_mib"] = int(float(row["mem_mib"]))
        result.append(row)
    return result


def nics():
    result = []
    for path in sorted(Path("/sys/class/net").iterdir()):
        name = path.name
        if name == "lo" or name.startswith(("veth", "docker", "br-")):
            continue
        read = lambda f: (path / f).read_text().strip() if (path / f).exists() else None  # noqa: E731
        try:
            speed = int(read("speed") or -1)
        except (OSError, ValueError):
            speed = -1
        _, addr = run(["ip", "-4", "-o", "addr", "show", name])
        ip = re.search(r"inet (\S+)", addr)
        result.append({"name": name, "mac": read("address"), "state": read("operstate"),
                       "speed_mbps": speed if speed > 0 else None, "ipv4": ip.group(1) if ip else None})
    return result


def disks():
    _, out = run(["lsblk", "-J", "-d", "-o", "NAME,SIZE,ROTA,MODEL,TYPE"])
    try:
        devices = [d for d in json.loads(out)["blockdevices"] if d.get("type") == "disk"]
    except (ValueError, KeyError):
        devices = []
    usage = shutil.disk_usage("/")
    return {"devices": [{"name": d["name"], "size": d["size"], "ssd": not d.get("rota"), "model": (d.get("model") or "").strip()}
                        for d in devices],
            "root_free_gb": round(usage.free / 1e9, 1), "root_total_gb": round(usage.total / 1e9, 1)}


def service_state(unit, user=False):
    base = ["systemctl", "--user"] if user else ["systemctl"]
    _, active = run(base + ["is-active", unit])
    _, enabled = run(base + ["is-enabled", unit])
    return {"active": active or "unknown", "enabled": enabled or "unknown"}


def listening_ports():
    _, out = run(["ss", "-H", "-ltn"])
    ports = {}
    for line in out.splitlines():
        parts = line.split()
        if len(parts) >= 4:
            host, _, port = parts[3].rpartition(":")
            ports.setdefault(int(port), set()).add(host)
    return ports


def inventory():
    mem_kb = int(re.search(r"MemTotal:\s+(\d+)", Path("/proc/meminfo").read_text()).group(1))
    cpu = re.search(r"model name\s*:\s*(.+)", Path("/proc/cpuinfo").read_text())
    _, ntp = run(["timedatectl", "show", "-p", "NTPSynchronized", "--value"])
    _, failed = run(["systemctl", "--failed", "--no-legend", "--plain"])
    _, user_failed = run(["systemctl", "--user", "--failed", "--no-legend", "--plain"])
    meta = read_env(CONFIG / "node.env")
    tools = {t: bool(shutil.which(t)) for t in sorted(set(ROLE_TOOLS["common"] + ROLE_TOOLS.get(meta.get("X3_NODE_ROLE"), [])))}
    return {
        "hostname": socket.gethostname(), "role": meta.get("X3_NODE_ROLE"), "collected": now(),
        "os": platform.freedesktop_os_release().get("PRETTY_NAME") if hasattr(platform, "freedesktop_os_release") else None,
        "kernel": platform.release(), "cpu": cpu.group(1).strip() if cpu else None, "threads": os.cpu_count(),
        "ram_gb": round(mem_kb / 1e6, 1), "load": os.getloadavg(), "disks": disks(), "nics": nics(), "gpus": gpus(),
        "clock_synced": ntp == "yes", "failed_units": [l.split()[0] for l in failed.splitlines() if l.strip()],
        "failed_user_units": [l.split()[0] for l in user_failed.splitlines() if l.strip()],
        "tools": tools, "repo": git_identity(),
    }


def read_env(path):
    values = {}
    if path.exists():
        for line in path.read_text().splitlines():
            if "=" in line and not line.lstrip().startswith("#"):
                key, _, value = line.partition("=")
                values[key.strip()] = value.strip().strip('"')
    return values


# ---------------------------------------------------------------- bootstrap

def ollama_process_on(port):
    """(pid, environ) of the Ollama process listening on `port`, environ None if unreadable."""
    _, out = run(["ss", "-H", "-ltnp", f"sport = :{port}"])
    match = re.search(r"pid=(\d+)", out)
    if not match:
        return (None, None) if not out else ("other", None)
    pid = int(match.group(1))
    try:
        raw = Path(f"/proc/{pid}/environ").read_bytes()
    except OSError:
        return pid, None
    return pid, dict(item.split("=", 1) for item in raw.decode(errors="replace").split("\0") if "=" in item)


def is_ollama(port):
    version = http_json(f"http://127.0.0.1:{port}/api/version")
    return isinstance(version, dict) and "version" in version


def wait_for_ollama(port, seconds=60):
    deadline = time.monotonic() + seconds
    while time.monotonic() < deadline:
        if is_ollama(port):
            return True
        time.sleep(1)
    return False


def ollama_models(port):
    try:
        with urllib.request.urlopen(f"http://127.0.0.1:{port}/api/tags", timeout=5) as response:
            return [m["name"] for m in json.load(response).get("models", [])]
    except Exception:  # noqa: BLE001 - absence is the answer
        return []


def worker_unit(rank, gpu, port, bind):
    return f"""[Unit]
Description=X3 Ollama worker gpu{rank} ({gpu['name']}, {gpu['uuid']})
After=network-online.target

[Service]
Type=simple
ExecStart={shutil.which('ollama') or '/usr/local/bin/ollama'} serve
# Generated by scripts/x3-cluster/x3cluster.py bootstrap --role gpu.
# Pinned by UUID: CUDA and nvidia-smi may enumerate GPUs in different orders.
Environment="CUDA_DEVICE_ORDER=PCI_BUS_ID"
Environment="CUDA_VISIBLE_DEVICES={gpu['uuid']}"
Environment="OLLAMA_VULKAN=0"
Environment="GGML_VK_VISIBLE_DEVICES="
Environment="OLLAMA_HOST={bind}:{port}"
Environment="OLLAMA_MODELS={Path.home()}/.ollama-gpu{rank}/models"
Environment="OLLAMA_SCHED_SPREAD=false"
Environment="OLLAMA_MAX_LOADED_MODELS=1"
Environment="OLLAMA_NUM_PARALLEL=1"
Environment="OLLAMA_CONTEXT_LENGTH=16384"
Environment="OLLAMA_FLASH_ATTENTION=1"
Environment="OLLAMA_KV_CACHE_TYPE=q8_0"
Environment="OLLAMA_KEEP_ALIVE=30m"
Restart=always
RestartSec=3

[Install]
WantedBy=default.target
"""


def plan_gpu_workers(node, node_ip, bind):
    """One worker per GPU. Strongest card (VRAM, then compute capability) gets
    the base port and the coding role; the rest get review/fast roles."""
    ranked = sorted(gpus(), key=lambda g: (-g["mem_mib"], -float(g["compute_cap"] or 0), g["pci"]))
    plan, staged = [], []
    for rank, gpu in enumerate(ranked):
        port = OLLAMA_BASE_PORT + 2 * rank
        pid, env = ollama_process_on(port)
        entry = {"rank": rank, "gpu": gpu, "port": port, "worker": f"{node}-gpu{rank}"}
        if pid is None:
            entry["state"] = "create"
            entry["unit"] = f"x3-ollama-gpu{rank}.service"
        elif env is None:
            # Owned by another user, so its environment (and pinning) cannot be read.
            # Treat it as the system ollama.service only when it answers as Ollama and
            # that unit is running; anything else on the port is a conflict, not a
            # service to overwrite and restart.
            if not is_ollama(port):
                entry["state"] = "conflict"
                entry["detail"] = f"port {port} is served by a process that does not answer as Ollama"
            elif run(["systemctl", "is-active", "ollama.service"])[1] != "active":
                entry["state"] = "conflict"
                entry["detail"] = f"port {port} is an Ollama this user cannot inspect, and ollama.service is not running"
            else:
                entry["state"] = "system-unverified"
                entry["unit"] = "ollama.service"
                staged.append(system_dropin(gpu, port, bind))
        elif env.get("CUDA_VISIBLE_DEVICES") == gpu["uuid"]:
            entry["state"] = "adopted"
            entry["bind"] = env.get("OLLAMA_HOST")
        else:
            entry["state"] = "conflict"
            entry["detail"] = f"port {port} served by pid {pid} pinned to {env.get('CUDA_VISIBLE_DEVICES')!r}"
        entry["models"] = [m for m in ollama_models(port) if m in ADVERTISED_MODELS]
        plan.append(entry)
    return plan, staged


def system_dropin(gpu, port, bind):
    return f"""# Pin the system ollama.service to {gpu['name']} ({gpu['uuid']}) on {bind}:{port}
sudo mkdir -p /etc/systemd/system/ollama.service.d
sudo tee /etc/systemd/system/ollama.service.d/override.conf >/dev/null <<'EOF'
[Service]
Environment="CUDA_DEVICE_ORDER=PCI_BUS_ID"
Environment="CUDA_VISIBLE_DEVICES={gpu['uuid']}"
Environment="OLLAMA_VULKAN=0"
Environment="GGML_VK_VISIBLE_DEVICES="
Environment="OLLAMA_HOST={bind}:{port}"
Environment="OLLAMA_SCHED_SPREAD=false"
Environment="OLLAMA_MAX_LOADED_MODELS=1"
Environment="OLLAMA_NUM_PARALLEL=1"
Environment="OLLAMA_CONTEXT_LENGTH=16384"
Environment="OLLAMA_FLASH_ATTENTION=1"
Environment="OLLAMA_KV_CACHE_TYPE=q8_0"
Environment="OLLAMA_KEEP_ALIVE=30m"
EOF
sudo systemctl daemon-reload && sudo systemctl restart ollama.service
"""


def registrable(entry):
    """Only workers known to be pinned to their GPU and running: adopted ones, and
    ones this run created and saw answer. A conflicting or unverified listener may
    be on another card, so advertising it would put two providers on one GPU."""
    return entry["state"] == "adopted" or (entry["state"] == "create" and entry.get("started", False))


def registration(node, host, plan, scope):
    """The providers.d drop-in a router loads to use this node's GPUs, or None when
    no worker is registrable with an advertised model (a router rejects an empty
    drop-in, and an empty one would only hide that the node cannot serve)."""
    providers, policies = {}, {}
    for entry in plan:
        if not registrable(entry):
            continue
        for model in entry["models"]:
            name = f"{node}_gpu{entry['rank']}_" + re.sub(r"[^a-z0-9]+", "_", model.lower()).strip("_")
            tools = ADVERTISED_MODELS[model]["supports_tools"]
            providers[name] = {"base_url": f"http://{host}:{entry['port']}/v1", "model": model,
                               "supports_tools": tools, "tool_probe": True, "probe_samples": 2,
                               "probe_timeout_seconds": 120, "critical_allowed": False,
                               "max_in_flight": 1, "worker": entry["worker"]}
            roles = ["x3-local", "x3-code", "x3-deep"] if entry["rank"] == 0 else ["x3-local", "x3-review", "x3-fast"]
            for policy in roles:
                policies.setdefault(policy, []).append(name)
    if not providers:
        return None
    return {"node": node, "generated": now(), "scope": scope, "providers": providers, "policies": policies}


def firewall_block(lan, allowed, ports):
    lines = ["# Firewall: LAN SSH, GPU workers only to the router/ops hosts. SSH is allowed before enabling.",
             f"sudo ufw allow from {lan} to any port 22 proto tcp"]
    for host in allowed:
        for port in ports:
            lines.append(f"sudo ufw allow from {host} to any port {port} proto tcp")
    lines.append("sudo ufw --force enable && sudo ufw status numbered")
    return "\n".join(lines) + "\n"


def bootstrap(args):
    node = socket.gethostname()
    inv = json.loads((HERE / "inventory.json").read_text())
    known = inv["nodes"].get(node, {})
    if known and known.get("role") != args.role:
        sys.exit(f"{node} is listed as role {known.get('role')!r} in inventory.json, not {args.role!r}")
    up = [n for n in nics() if n["ipv4"] and n["state"] == "up"]
    node_ip = up[0]["ipv4"].split("/")[0] if up else None
    CONFIG.mkdir(parents=True, exist_ok=True)
    env = {"X3_NODE_NAME": node, "X3_NODE_ROLE": args.role, "X3_NODE_IP": node_ip or "",
           "X3_NODE_BOOTSTRAPPED": now()}
    report = {"node": node, "role": args.role, "ip": node_ip, "apply": args.apply, "actions": [], "staged": []}

    staged = []
    missing = [t for t in ROLE_TOOLS["common"] + ROLE_TOOLS.get(args.role, []) if not shutil.which(t)]
    apt = {"tmux": "tmux", "iperf3": "iperf3", "ethtool": "ethtool", "jq": "jq", "curl": "curl", "git": "git",
           "ssh": "openssh-client openssh-server", "clang": "clang", "cmake": "cmake", "pkg-config": "pkg-config",
           "psql": "postgresql-client", "pg_dump": "postgresql-client", "tc": "iproute2",
           "nvcc": "cuda-toolkit-12-8  # from NVIDIA's apt repo; CUDA 13 dropped Pascal (sm_61)"}
    packages = sorted({apt[t] for t in missing if t in apt and t != "nvcc"})
    if packages:
        staged.append("sudo apt-get update && sudo apt-get install -y " + " ".join(packages) + "\n")
    if "nvcc" in missing:
        staged.append("# CUDA toolkit (needed to build the .cu kernels): install " + apt["nvcc"] + "\n")
    report["missing_tools"] = missing
    _, linger = run(["loginctl", "show-user", os.environ.get("USER", ""), "-p", "Linger", "--value"])
    if linger != "yes":
        staged.append(f"sudo loginctl enable-linger {os.environ.get('USER')}\n")

    if args.role == "gpu":
        # Every worker this run starts listens on loopback. With --lan the staged
        # script enables the firewall first and only then rebinds to 0.0.0.0, so an
        # unauthenticated Ollama API is never open to the LAN without it.
        lan_bind = "0.0.0.0" if args.lan else "127.0.0.1"
        plan, dropins = plan_gpu_workers(node, node_ip, lan_bind)
        rebinds = list(dropins)  # the system drop-in binds too: after the firewall
        for entry in plan:
            if entry["state"] == "create":
                path = USER_UNITS / entry["unit"]
                report["actions"].append(f"{'write' if args.apply else 'would write'} {path}")
                if args.apply:
                    USER_UNITS.mkdir(parents=True, exist_ok=True)
                    path.write_text(worker_unit(entry["rank"], entry["gpu"], entry["port"], "127.0.0.1"))
                    run(["systemctl", "--user", "daemon-reload"])
                    run(["systemctl", "--user", "enable", "--now", entry["unit"]])
                    entry["started"] = wait_for_ollama(entry["port"])
                    # Probed again now that it runs: the plan probed before it existed.
                    entry["models"] = [m for m in ollama_models(entry["port"]) if m in ADVERTISED_MODELS]
                    if entry["started"] and not entry["models"]:
                        staged.extend(f"OLLAMA_HOST=127.0.0.1:{entry['port']} ollama pull {model}\n"
                                      for model in ADVERTISED_MODELS)
                        report["actions"].append(f"{entry['worker']} has no advertised model yet: pull staged; "
                                                 "re-run bootstrap afterwards to register it")
                    if args.lan:
                        rebinds.append(f"sed -i 's|OLLAMA_HOST=127.0.0.1:{entry['port']}|OLLAMA_HOST=0.0.0.0:{entry['port']}|' "
                                       f"{path}\nsystemctl --user daemon-reload && systemctl --user restart {entry['unit']}\n")
        env["X3_GPU_WORKERS"] = ",".join(f"{e['worker']}:{e['port']}" for e in plan)
        # IP, not hostname: peers cannot resolve cluster names until DNS/DHCP reservations exist.
        # Without --lan the workers only listen on loopback, so only a router on this node
        # can use them, and the registration says so.
        if args.lan:
            reg = registration(node, node_ip or node, plan, "lan (reachable once staged-privileged.sh has run)")
        else:
            reg = registration(node, "127.0.0.1", plan, "local (workers bound to loopback; use --lan for remote routers)")
        reg_path = CONFIG / "registration" / f"{node}.json"
        (CONFIG / "registration").mkdir(exist_ok=True)
        if reg is None:
            if reg_path.exists():
                reg_path.unlink()
            report["registration"] = None
            report["registration_withheld"] = "no running, GPU-verified worker has an advertised model"
        else:
            reg_path.write_text(json.dumps(reg, indent=2) + "\n")
            report["registration"] = str(reg_path)
        report["gpu_workers"] = [{k: v for k, v in e.items() if k != "gpu"} | {"gpu": e["gpu"]["name"], "uuid": e["gpu"]["uuid"]}
                                 for e in plan]
        if args.lan:
            router_hosts = [n["ip"] for n in inv["nodes"].values() if n["role"] in ("control", "ops") and n.get("ip")]
            staged.append(firewall_block(inv["lan"], router_hosts, [e["port"] for e in plan]))
            staged.extend(rebinds)
            # Rebind adopted loopback user workers only after the firewall is on.
            for entry in plan:
                if entry["state"] == "adopted" and str(entry.get("bind", "")).startswith("127."):
                    pid, _ = ollama_process_on(entry["port"])
                    _, unit = run(["ps", "-o", "uunit=", "-p", str(pid)])
                    if not unit.endswith(".service") or not (USER_UNITS / unit).exists():
                        staged.append(f"# could not find the user unit for port {entry['port']}; rebind it by hand\n")
                        continue
                    staged.append(f"sed -i 's|OLLAMA_HOST={entry['bind']}|OLLAMA_HOST=0.0.0.0:{entry['port']}|' "
                                  f"{USER_UNITS / unit}\nsystemctl --user daemon-reload && systemctl --user restart {unit}\n")
        else:
            staged.extend(rebinds)  # without --lan these are loopback drop-ins only

    (CONFIG / "node.env").write_text("# Non-secret node metadata. Never put tokens or passwords here.\n"
                                     + "".join(f"{k}={v}\n" for k, v in env.items()))
    report["actions"].append(f"wrote {CONFIG / 'node.env'}")
    if staged:
        path = CONFIG / "staged-privileged.sh"
        path.write_text("#!/usr/bin/env bash\n# Review, then run: bash " + str(path) + "\nset -euo pipefail\n\n"
                        + "\n".join(staged))
        path.chmod(0o700)
        report["staged"] = str(path)
    print(json.dumps(report, indent=2))


# ---------------------------------------------------------------- bench

def bench(_args):
    result = {"node": socket.gethostname(), "label": "PHYSICAL", "collected": now(), "repo": git_identity()}
    # CPU: SHA-256 over 256 MiB, single process and one per thread.
    block = os.urandom(1 << 20)

    def hash_mib(n):
        h = hashlib.sha256()
        for _ in range(n):
            h.update(block)
        return h.hexdigest()

    started = time.perf_counter()
    hash_mib(256)
    single = 256 / (time.perf_counter() - started)
    threads = os.cpu_count() or 1
    code = ("import hashlib,os,time;b=os.urandom(1<<20);h=hashlib.sha256()\n"
            "for _ in range(128): h.update(b)")
    started = time.perf_counter()
    procs = [subprocess.Popen([sys.executable, "-c", code]) for _ in range(threads)]
    for p in procs:
        p.wait()
    multi = 128 * threads / (time.perf_counter() - started)
    result["cpu"] = {"sha256_mib_s_1thread": round(single, 1), "sha256_mib_s_all": round(multi, 1), "threads": threads}

    # Disk: 1 GiB sequential write (fsync'd) and read with the page cache dropped for the file.
    with tempfile.TemporaryDirectory(dir=str(Path.home())) as tmp:
        path = Path(tmp) / "bench.bin"
        code, out = run(f"dd if=/dev/zero of={path} bs=4M count=256 oflag=direct conv=fsync 2>&1 | tail -1", 120)
        write = re.search(r"([\d.,]+) ([GM])B/s", out)
        code, out = run(f"dd if={path} of=/dev/null bs=4M iflag=direct 2>&1 | tail -1", 120)
        read = re.search(r"([\d.,]+) ([GM])B/s", out)
    to_mb = lambda m: round(float(m.group(1).replace(",", ".")) * (1000 if m.group(2) == "G" else 1), 1) if m else None  # noqa: E731
    result["disk"] = {"seq_write_mb_s": to_mb(write), "seq_read_mb_s": to_mb(read), "method": "dd 1GiB O_DIRECT"}

    # Network: link speeds and latency to every known peer that answers.
    inv = json.loads((HERE / "inventory.json").read_text())
    latency = {}
    for name, node in inv["nodes"].items():
        if node.get("ip") and name != socket.gethostname():
            _, out = run(["ping", "-c", "10", "-i", "0.2", "-q", "-W", "1", node["ip"]], 15)
            match = re.search(r"= ([\d.]+)/([\d.]+)/([\d.]+)", out)
            latency[name] = {"avg_ms": float(match.group(2)), "max_ms": float(match.group(3))} if match else None
    result["network"] = {"nics": nics(), "latency": latency,
                         "iperf3": "not measured: iperf3 missing or no peer server" if not shutil.which("iperf3") else None}

    # Rust: clean release-less build of one small workspace crate, isolated target dir.
    if shutil.which("cargo"):
        with tempfile.TemporaryDirectory(dir=str(Path.home())) as target:
            started = time.perf_counter()
            # pipefail: without it the status is tail's, and a failed build reads as ok.
            code, out = run(["bash", "-o", "pipefail", "-c",
                             f"cd {REPO} && CARGO_TARGET_DIR={target} cargo build -q -p gpu-sig-verifier 2>&1 | tail -3"], 1800)
            result["rust_build"] = {"crate": "gpu-sig-verifier", "profile": "dev", "clean": True,
                                    "seconds": round(time.perf_counter() - started, 1), "ok": code == 0,
                                    "tail": out[-300:] if code else ""}
    print(json.dumps(result, indent=2))
    return result


# ---------------------------------------------------------------- health

def tcp_open(host, port, timeout=1.5):
    try:
        with socket.create_connection((host, port), timeout=timeout):
            return True
    except OSError:
        return False


def resolve(name, node):
    try:
        ip = socket.gethostbyname(name)
    except OSError:
        return node.get("ip")
    # Debian maps the own hostname to 127.0.1.1; that is not the LAN address.
    return node.get("ip") or ip if ip.startswith("127.") else ip


def ssh_target(name, ip):
    try:
        return name if socket.gethostbyname(name) == ip else ip
    except OSError:
        return ip


def http_json(url, timeout=3):
    try:
        with urllib.request.urlopen(url, timeout=timeout) as response:
            return json.load(response)
    except Exception:  # noqa: BLE001
        return None


def local_health(inv_node):
    inv = inventory()
    services = {}
    for unit in inv_node.get("services", []):
        if unit == "ssh":
            services[unit] = service_state("ssh")["active"]
            continue
        # A user unit wins when one exists; otherwise ask the system manager.
        user = service_state(unit, user=True)
        services[unit] = user["active"] if user["enabled"] not in ("unknown", "not-found") else service_state(unit)["active"]
    workers = {}
    for port in sorted(p for p in listening_ports() if OLLAMA_BASE_PORT <= p < OLLAMA_BASE_PORT + 16 and p != 11435):
        version = http_json(f"http://127.0.0.1:{port}/api/version")
        ps = http_json(f"http://127.0.0.1:{port}/api/ps") or {}
        if version:
            workers[port] = {"ok": True, "loaded": [m["name"] for m in ps.get("models", [])]}
    router = http_json("http://127.0.0.1:11435/health")
    return {"cpu": f"{inv['threads']}t load {inv['load'][0]:.1f}", "ram": f"{inv['ram_gb']}G",
            "gpu": ",".join(g["name"].replace("NVIDIA GeForce ", "") for g in inv["gpus"]) or "-",
            "services": services, "ollama_workers": workers, "router": router, "disk_free_gb": inv["disks"]["root_free_gb"],
            "clock_synced": inv["clock_synced"], "failed_units": inv["failed_units"] + inv["failed_user_units"],
            "repo": inv["repo"]}


def remote_health(host):
    # Identity comes back with the numbers: an inventory IP reassigned to another
    # machine would otherwise pass as the expected node.
    script = ("python3 - <<'PY'\nimport json,os,shutil,socket,pathlib\n"
              "env={}\np=pathlib.Path.home()/'.config'/'x3-cluster'/'node.env'\n"
              "if p.exists():\n"
              "    for l in p.read_text().splitlines():\n"
              "        k,_,v=l.partition('=')\n"
              "        if v and not l.lstrip().startswith('#'): env[k.strip()]=v.strip().strip('\"')\n"
              "print(json.dumps({'threads':os.cpu_count(),'load':os.getloadavg()[0],"
              "'free_gb':round(shutil.disk_usage('/').free/1e9,1),'hostname':socket.gethostname(),"
              "'node_name':env.get('X3_NODE_NAME'),'role':env.get('X3_NODE_ROLE')}))\nPY")
    code, out = run(["ssh", "-o", "BatchMode=yes", "-o", "ConnectTimeout=4", host, script], 15)
    if code != 0:
        return None
    try:
        return json.loads(out.splitlines()[-1])
    except (ValueError, IndexError):
        return None


def health(args, quiet=False):
    inv = json.loads((HERE / "inventory.json").read_text())
    me = socket.gethostname()
    rows = []
    for name, node in inv["nodes"].items():
        ip = resolve(name, node)
        row = {"node": name, "role": node["role"], "ip": ip, "required": node.get("required", False)}
        if name == me:
            row.update(reachable=True, ssh=tcp_open("127.0.0.1", 22), label="LOCAL", **local_health(node))
        elif not ip:
            row.update(reachable=False, ssh=False, label="UNKNOWN", detail="no IP: not in DNS/mDNS/inventory")
        else:
            reachable = run(["ping", "-c", "1", "-W", "1", ip])[0] == 0
            ssh_port = tcp_open(ip, 22)
            # SSH to the address that was actually probed; the name only when it
            # resolves to that address (so ~/.ssh/config aliases still apply).
            remote = remote_health(ssh_target(name, ip)) if ssh_port else None
            row.update(reachable=reachable, ssh=bool(remote), ssh_port=ssh_port, label="PHYSICAL", remote=remote)
            if node["role"] == "control":
                row["router_port"] = tcp_open(ip, 11435)
        rows.append(row)
    report = {"collected": now(), "from": me, "repo": git_identity(), "nodes": rows}
    if not quiet:
        if getattr(args, "json", False):
            print(json.dumps(report, indent=2, default=list))
        else:
            mark = lambda v: "PASS" if v else "FAIL"  # noqa: E731
            print(f"{'NODE':10} {'ROLE':8} {'IP':15} {'REACH':6} {'SSH':5} {'CPU':16} {'RAM':6} {'GPU':28} SERVICES")
            for r in rows:
                services = " ".join(f"{k}={v}" for k, v in (r.get("services") or {}).items())
                cpu = r.get("cpu") or (f"{r['remote']['threads']}t load {r['remote']['load']:.1f}" if r.get("remote") else "-")
                print(f"{r['node']:10} {r['role']:8} {str(r['ip'] or '-'):15} {mark(r['reachable']):6} {mark(r['ssh']):5} "
                      f"{cpu:16} {r.get('ram', '-'):6} {r.get('gpu', '-')[:28]:28} {services or r.get('detail', '')}")
            local = next(r for r in rows if r["node"] == me) if any(r["node"] == me for r in rows) else None
            if local:
                print(f"\nrouter@{me}:11435  {'ok ' + json.dumps(local['router'].get('in_flight')) if local['router'] else 'DOWN'}")
                for port, w in local["ollama_workers"].items():
                    print(f"ollama :{port}  ok  loaded={','.join(w['loaded']) or '-'}")
                print(f"disk free {local['disk_free_gb']} GB   clock synced {local['clock_synced']}   "
                      f"failed units {local['failed_units'] or 'none'}")
                print(f"repo {local['repo']['branch']}@{local['repo']['commit'][:12]} dirty={local['repo']['dirty']}")
    return report


# ---------------------------------------------------------------- gate

def gate(args):
    report = health(args, quiet=True)
    checks = []

    def check(name, ok, detail=""):
        checks.append({"check": name, "pass": bool(ok), "detail": detail})

    me = socket.gethostname()
    for row in report["nodes"]:
        check(f"{row['node']}.reachable", row["reachable"], row.get("detail", row.get("ip")))
        check(f"{row['node']}.ssh", row["ssh"], "key auth" if row["node"] != me else "local sshd")
        if row["node"] == me:
            meta = read_env(CONFIG / "node.env")
            check(f"{me}.role_identity", meta.get("X3_NODE_ROLE") == row["role"], meta.get("X3_NODE_ROLE"))
            check(f"{me}.clock_synced", row["clock_synced"])
            check(f"{me}.disk_free_10gb", row["disk_free_gb"] >= 10, row["disk_free_gb"])
            check(f"{me}.no_failed_units", not row["failed_units"], row["failed_units"])
            if row["role"] == "gpu":
                check(f"{me}.gpu_workers", len(row["ollama_workers"]) >= max(1, len(gpus())), sorted(row["ollama_workers"]))
            # Every service the inventory declares must be running: a cleanly stopped
            # unit is not in `systemctl --failed`, so that check alone misses it.
            for unit, state in (row.get("services") or {}).items():
                check(f"{me}.service.{unit}", state == "active", state)
            if "x3-ai-router" in (row.get("services") or {}):
                check(f"{me}.router_health", row.get("router") is not None, "http://127.0.0.1:11435/health")
        elif row.get("remote") is not None:
            remote = row["remote"]
            same_node = remote.get("node_name") == row["node"] or remote.get("hostname") == row["node"]
            check(f"{row['node']}.identity", same_node and remote.get("role") == row["role"],
                  f"hostname={remote.get('hostname')} node={remote.get('node_name')} role={remote.get('role')}")
    storage = Path("/x3-storage")
    check("storage.mounted", storage.is_mount() or any(p.is_mount() for p in storage.glob("*")) if storage.exists() else False,
          "/x3-storage")
    ident = report["repo"]
    out = REPO / "audit-artifacts" / "x3-cluster" / (ident["commit"] or "unknown")
    out.mkdir(parents=True, exist_ok=True)
    summary = {"collected": now(), "from": me, "repo": ident, "checks": checks,
               "passed": sum(c["pass"] for c in checks), "failed": sum(not c["pass"] for c in checks)}
    (out / f"health-{me}.json").write_text(json.dumps(report, indent=2, default=list) + "\n")
    (out / f"gate-{me}.json").write_text(json.dumps(summary, indent=2, default=list) + "\n")
    for c in checks:
        print(("PASS " if c["pass"] else "FAIL ") + c["check"] + (f"  ({c['detail']})" if c["detail"] not in ("", None) else ""))
    print(f"\n{summary['passed']} passed, {summary['failed']} failed -> {out}")
    return 0 if summary["failed"] == 0 else 1


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = parser.add_subparsers(dest="cmd", required=True)
    p = sub.add_parser("inventory")
    p.add_argument("--json", action="store_true")
    p = sub.add_parser("bootstrap")
    p.add_argument("--role", choices=ROLES, required=True)
    p.add_argument("--apply", action="store_true", help="create/enable user units (default: plan only)")
    p.add_argument("--lan", action="store_true", help="bind GPU workers to 0.0.0.0 and stage firewall rules")
    sub.add_parser("bench")
    p = sub.add_parser("health")
    p.add_argument("--json", action="store_true")
    sub.add_parser("gate")
    args = parser.parse_args()
    if args.cmd == "inventory":
        print(json.dumps(inventory(), indent=2))
    elif args.cmd == "bootstrap":
        bootstrap(args)
    elif args.cmd == "bench":
        bench(args)
    elif args.cmd == "health":
        health(args)
    elif args.cmd == "gate":
        sys.exit(gate(args))


if __name__ == "__main__":
    main()
