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
    x3cluster.py job --class C [--priority P] [--node N] [--ref R] -- CMD...
                                                run CMD at an exact commit on the node
                                                that owns class C; evidence per job
    x3cluster.py pipeline [--spec FILE]         staged distributed pipeline (pipeline.json)
    x3cluster.py ssh-config [--write]           Host aliases for every node with a known IP
    x3cluster.py discover [--onboard]           find inventory nodes on the LAN; onboard new ones
    x3cluster.py onboard NODE [--ip IP]         tooling, checkout, bootstrap, workers, models,
                                                router registration, bench, first job (no sudo)

Anything that needs root is never run: it is written to
~/.config/x3-cluster/staged-privileged.sh for an operator to review and run.
Evidence goes to audit-artifacts/x3-cluster/<commit>/. Every result is labelled
PHYSICAL (measured on real hardware) or LOCAL (this node only).
"""
import argparse
import datetime as dt
import hashlib
import io
import json
import os
import platform
import re
import shutil
import socket
import subprocess
import sys
import tarfile
import tempfile
import time
import urllib.request
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import x3jobs  # noqa: E402

HERE = Path(__file__).resolve().parent
# X3_REPO lets the tooling run from a copy outside the checkout (as onboarded nodes do).
REPO = Path(os.environ["X3_REPO"]).expanduser() if os.environ.get("X3_REPO") else HERE.parents[1]
DISCOVERED = Path.home() / ".config" / "x3-cluster" / "discovered.json"
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


def load_inventory():
    """inventory.json plus IPs this control node discovered for nodes the file has no IP for."""
    inv = json.loads((HERE / "inventory.json").read_text())
    try:
        found = json.loads(DISCOVERED.read_text())
    except (OSError, ValueError):
        found = {}
    for name, entry in found.items():
        node = inv["nodes"].get(name)
        if node is not None and not node.get("ip") and entry.get("ip"):
            node["ip"] = entry["ip"]
            node["discovered"] = entry.get("seen", True)
    return inv


def firewall_enabled():
    try:
        return "ENABLED=yes" in Path("/etc/ufw/ufw.conf").read_text()
    except OSError:
        return False


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


def system_unit_env_for_port(port):
    """(unit, environment) of an active system Ollama unit configured for `port`.

    A root-owned process's /proc environ is unreadable, but the unit's configured
    Environment= (drop-ins included) is visible to any user through systemctl show."""
    import shlex
    _, units = run(["systemctl", "list-units", "--type=service", "--state=active", "--no-legend", "--plain",
                    "ollama*", "x3-ollama*"])
    for line in units.splitlines():
        unit = line.split()[0] if line.split() else ""
        _, raw = run(["systemctl", "show", "-p", "Environment", "--value", unit])
        env = dict(item.split("=", 1) for item in shlex.split(raw) if "=" in item)
        if env.get("OLLAMA_HOST", "").rsplit(":", 1)[-1] == str(port):
            return unit, env
    return None, None


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
            # A root/ollama-owned service: read its configured pinning from systemd instead.
            unit, unit_env = system_unit_env_for_port(port)
            if unit_env and (unit_env.get("CUDA_VISIBLE_DEVICES") == gpu["uuid"]
                             or (len(ranked) == 1 and not unit_env.get("CUDA_VISIBLE_DEVICES"))):
                # Unpinned is only unambiguous while there is one GPU; a second card makes it a conflict.
                entry["state"] = "adopted"
                entry["unit"] = unit
                entry["bind"] = unit_env.get("OLLAMA_HOST")
                if not unit_env.get("CUDA_VISIBLE_DEVICES"):
                    entry["detail"] = "unpinned system worker, adopted because this node has one GPU"
            else:
                entry["state"] = "system-unverified"
                entry["unit"] = unit or "ollama.service"
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


def registration(node, host, plan):
    """The providers.d drop-in a router loads to use this node's GPUs."""
    providers, policies = {}, {}
    for entry in plan:
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
    return {"node": node, "generated": now(), "providers": providers, "policies": policies}


def firewall_block(lan, allowed, ports):
    lines = ["# Firewall: LAN SSH, GPU workers only to the router/ops hosts. SSH is allowed before enabling.",
             f"sudo ufw allow from {lan} to any port 22 proto tcp"]
    for host in allowed:
        for port in ports:
            lines.append(f"sudo ufw allow from {host} to any port {port} proto tcp")
    lines.append("sudo ufw --force enable && sudo ufw status numbered")
    return "\n".join(lines) + "\n"


def hosts_block(inv, me):
    """/etc/hosts fallback for peers with a known IP that do not resolve yet (no DHCP-reservation DNS)."""
    lines = []
    for name, node in inv["nodes"].items():
        if name == me or not node.get("ip"):
            continue
        try:
            if socket.gethostbyname(name) == node["ip"]:
                continue
        except OSError:
            pass
        lines.append(f"{node['ip']} {name}")
    if not lines:
        return ""
    body = "\n".join(lines)
    return ("# Stable-name fallback until DHCP reservations/DNS exist. Skips names already present.\n"
            + "".join(f"grep -qw '{l.split()[1]}' /etc/hosts || echo '{l}  # x3-cluster' | sudo tee -a /etc/hosts\n"
                      for l in body.splitlines()))


def control_firewall_block(lan):
    """Default-deny inbound, keeping every service that already listens beyond loopback
    reachable from the LAN and over Tailscale, so enabling ufw breaks nothing in use."""
    lines = ["# Firewall for the control node. Existing LAN listeners stay LAN-reachable; nothing is public.",
             "sudo ufw default deny incoming", "sudo ufw default allow outgoing",
             f"sudo ufw allow from {lan} to any port 22 proto tcp"]
    for port in sorted(p for p, hosts in listening_ports().items()
                       if p != 22 and any(not h.startswith(("127.", "[::1]", "[::ffff:127.", "100.", "[fd7a:")) for h in hosts)):
        lines.append(f"sudo ufw allow from {lan} to any port {port} proto tcp")
    if Path("/sys/class/net/tailscale0").exists():
        lines.append("sudo ufw allow in on tailscale0")
    lines.append("sudo ufw --force enable && sudo ufw status numbered")
    return "\n".join(lines) + "\n"


def bootstrap(args):
    node = socket.gethostname()
    inv = load_inventory()
    known = inv["nodes"].get(node, {})
    if known and known.get("role") != args.role:
        sys.exit(f"{node} is listed as role {known.get('role')!r} in inventory.json, not {args.role!r}")
    up = [n for n in nics() if n["ipv4"] and n["state"] == "up"]
    node_ip = up[0]["ipv4"].split("/")[0] if up else None
    CONFIG.mkdir(parents=True, exist_ok=True)
    # Keys an operator added (X3_REPO, X3_CARGO_TARGET_DIR, ...) survive a re-bootstrap.
    env = read_env(CONFIG / "node.env")
    env.update({"X3_NODE_NAME": node, "X3_NODE_ROLE": args.role, "X3_NODE_IP": node_ip or "",
                "X3_NODE_BOOTSTRAPPED": now()})
    report = {"node": node, "role": args.role, "ip": node_ip, "apply": args.apply, "actions": [], "staged": []}

    staged = []
    missing = [t for t in ROLE_TOOLS["common"] + ROLE_TOOLS.get(args.role, []) if not shutil.which(t)]
    apt = {"tmux": "tmux", "iperf3": "iperf3", "ethtool": "ethtool", "jq": "jq", "curl": "curl", "git": "git",
           "ssh": "openssh-client openssh-server", "clang": "clang", "cmake": "cmake", "pkg-config": "pkg-config",
           "psql": "postgresql-client", "pg_dump": "postgresql-client", "tc": "iproute2",
           "nvcc": "cuda-toolkit-12-8  # from NVIDIA's apt repo; CUDA 13 dropped Pascal (sm_61)"}
    if not Path("/usr/sbin/sshd").exists():
        # `ssh` on PATH is only the client; peers, x3ops1 and VS Code Remote need the server.
        missing.append("sshd")
        apt["sshd"] = "openssh-server"
    packages = sorted({apt[t] for t in missing if t in apt and t != "nvcc"})
    if packages:
        # packagekitd (desktop updater) holds the apt lock for long stretches; it is
        # D-Bus activated and comes back on demand. apt then waits rather than failing.
        staged.append("sudo systemctl stop packagekit 2>/dev/null || true\n"
                      "sudo apt-get -o DPkg::Lock::Timeout=600 update\n"
                      "sudo apt-get -o DPkg::Lock::Timeout=600 install -y " + " ".join(packages) + "\n"
                      + ("sudo systemctl enable --now ssh\n" if "openssh-server" in packages else ""))
    if "nvcc" in missing:
        staged.append("# CUDA toolkit (needed to build the .cu kernels): install " + apt["nvcc"] + "\n")
    report["missing_tools"] = missing
    _, linger = run(["loginctl", "show-user", os.environ.get("USER", ""), "-p", "Linger", "--value"])
    if linger != "yes":
        staged.append(f"sudo loginctl enable-linger {os.environ.get('USER')}\n")

    hosts = hosts_block(inv, node)
    if hosts:
        staged.append(hosts)
    if args.role == "control" and args.lan:
        staged.append(control_firewall_block(inv["lan"]))

    if args.role == "gpu":
        if not shutil.which("nvidia-smi"):
            staged.append("# No NVIDIA driver: install the recommended one, then reboot and re-run bootstrap\n"
                          "sudo ubuntu-drivers install\n")
        if not shutil.which("ollama"):
            staged.append("# Ollama (official installer); per-GPU workers replace its default service\n"
                          "curl -fsSL https://ollama.com/install.sh | sh\n"
                          "sudo systemctl disable --now ollama.service\n")
        # A worker is only bound to the LAN once the firewall is on; until then it stays
        # on loopback and the staged block rebinds it after enabling ufw.
        lan_now = args.lan and firewall_enabled()
        bind = "0.0.0.0" if lan_now else "127.0.0.1"
        plan, dropins = plan_gpu_workers(node, node_ip, "0.0.0.0" if args.lan else "127.0.0.1")
        staged.extend(dropins)
        for entry in plan:
            if entry["state"] == "create":
                path = USER_UNITS / entry["unit"]
                report["actions"].append(f"{'write' if args.apply else 'would write'} {path}")
                if args.apply:
                    USER_UNITS.mkdir(parents=True, exist_ok=True)
                    path.write_text(worker_unit(entry["rank"], entry["gpu"], entry["port"], bind))
                    run(["systemctl", "--user", "daemon-reload"])
                    run(["systemctl", "--user", "enable", "--now", entry["unit"]])
        env["X3_GPU_WORKERS"] = ",".join(f"{e['worker']}:{e['port']}" for e in plan)
        # IP, not hostname: peers cannot resolve cluster names until DNS/DHCP reservations exist.
        reg = registration(node, node_ip or node, plan)
        (CONFIG / "registration").mkdir(exist_ok=True)
        (CONFIG / "registration" / f"{node}.json").write_text(json.dumps(reg, indent=2) + "\n")
        report["gpu_workers"] = [{k: v for k, v in e.items() if k != "gpu"} | {"gpu": e["gpu"]["name"], "uuid": e["gpu"]["uuid"]}
                                 for e in plan]
        report["registration"] = str(CONFIG / "registration" / f"{node}.json")
        if args.lan:
            router_hosts = [n["ip"] for n in inv["nodes"].values() if n["role"] in ("control", "ops") and n.get("ip")]
            staged.append(firewall_block(inv["lan"], router_hosts, [e["port"] for e in plan]))
            # Rebind loopback user workers only after the firewall is on.
            for entry in plan:
                if entry["state"] == "create" and not lan_now:
                    path = USER_UNITS / entry["unit"]
                    staged.append(f"sed -i 's|OLLAMA_HOST=127.0.0.1:{entry['port']}|OLLAMA_HOST=0.0.0.0:{entry['port']}|' {path}\n"
                                  f"systemctl --user daemon-reload && systemctl --user restart {entry['unit']}\n")
                if entry["state"] == "adopted" and str(entry.get("bind", "")).startswith("127."):
                    pid, _ = ollama_process_on(entry["port"])
                    _, unit = run(["ps", "-o", "uunit=", "-p", str(pid)])
                    if not unit.endswith(".service") or not (USER_UNITS / unit).exists():
                        staged.append(f"# could not find the user unit for port {entry['port']}; rebind it by hand\n")
                        continue
                    staged.append(f"sed -i 's|OLLAMA_HOST={entry['bind']}|OLLAMA_HOST=0.0.0.0:{entry['port']}|' "
                                  f"{USER_UNITS / unit}\nsystemctl --user daemon-reload && systemctl --user restart {unit}\n")

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

def bench(args):
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
    inv = load_inventory()
    latency = {}
    for name, node in inv["nodes"].items():
        if node.get("ip") and name != socket.gethostname():
            _, out = run(["ping", "-c", "10", "-i", "0.2", "-q", "-W", "1", node["ip"]], 15)
            match = re.search(r"= ([\d.]+)/([\d.]+)/([\d.]+)", out)
            latency[name] = {"avg_ms": float(match.group(2)), "max_ms": float(match.group(3))} if match else None
    result["network"] = {"nics": nics(), "latency": latency,
                         "iperf3": "not measured: iperf3 missing or no peer server" if not shutil.which("iperf3") else None}

    # Rust: clean release-less build of one small workspace crate, isolated target dir.
    free_gb = shutil.disk_usage(str(Path.home())).free / 1e9
    if getattr(args, "no_rust", False) or free_gb < 50:
        result["rust_build"] = {"skipped": "--no-rust" if getattr(args, "no_rust", False) else f"{free_gb:.0f} GB free < 50 GB"}
    elif shutil.which("cargo"):
        with tempfile.TemporaryDirectory(dir=str(Path.home())) as target:
            started = time.perf_counter()
            code, out = run(f"cd {REPO} && CARGO_TARGET_DIR={target} cargo build -q -p gpu-sig-verifier 2>&1 | tail -3", 1800)
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


# Runs on the peer over SSH. Workers often listen on loopback only, so they are probed
# from the peer itself rather than across the LAN.
REMOTE_HEALTH = r"""python3 - <<'PY'
import json, os, re, shutil, subprocess, urllib.request
from pathlib import Path
def sh(c):
    try:
        return subprocess.run(c, shell=True, capture_output=True, text=True, timeout=10).stdout.strip()
    except Exception:
        return ""
def get(url):
    try:
        with urllib.request.urlopen(url, timeout=3) as r:
            return json.load(r)
    except Exception:
        return None
env = {}
p = Path.home() / ".config/x3-cluster/node.env"
if p.exists():
    for line in p.read_text().splitlines():
        if "=" in line and not line.lstrip().startswith("#"):
            k, _, v = line.partition("=")
            env[k.strip()] = v.strip()
mem = int(re.search(r"MemTotal:\s+(\d+)", Path("/proc/meminfo").read_text()).group(1))
workers = {}
for line in sh("ss -H -ltn").splitlines():
    parts = line.split()
    if len(parts) < 4:
        continue
    host, _, port = parts[3].rpartition(":")
    port = int(port)
    if 11434 <= port < 11450 and port != 11435 and port not in workers and get(f"http://127.0.0.1:{port}/api/version"):
        ps = get(f"http://127.0.0.1:{port}/api/ps") or {}
        workers[port] = {"ok": True, "bind": host, "loaded": [m["name"] for m in ps.get("models", [])]}
failed = [l.split()[0] for c in ("systemctl --failed --no-legend --plain", "systemctl --user --failed --no-legend --plain")
          for l in sh(c).splitlines() if l.strip()]
repo = env.get("X3_REPO") or str(Path.home() / "Desktop/xxxstar-main")
print(json.dumps({"hostname": os.uname().nodename, "threads": os.cpu_count(), "load": os.getloadavg()[0],
    "ram_gb": round(mem / 1e6, 1), "free_gb": round(shutil.disk_usage(str(Path.home())).free / 1e9, 1),
    "gpus": [g for g in sh("nvidia-smi --query-gpu=name --format=csv,noheader").splitlines() if g],
    "ollama_workers": workers, "router": get("http://127.0.0.1:11435/health"),
    "clock_synced": sh("timedatectl show -p NTPSynchronized --value") == "yes", "failed_units": failed,
    "role": env.get("X3_NODE_ROLE"), "repo_head": sh(f"git -C '{repo}' rev-parse HEAD 2>/dev/null")}))
PY"""


def parse_exporter(text):
    """Facts from a Prometheus node_exporter page (plus the x3_gpu_* textfile metrics)."""
    facts = {"threads": 0, "ram_gb": None, "free_gb": None, "gpus": [], "units": {}, "clock_synced": None, "os": None}
    gpus = {}
    for line in text.splitlines():
        if line.startswith("#") or " " not in line:
            continue
        key, _, value = line.rpartition(" ")
        try:
            num = float(value)
        except ValueError:
            continue
        name, _, labels = key.partition("{")
        lab = dict(re.findall(r'(\w+)="([^"]*)"', labels))
        if name == "node_cpu_seconds_total" and lab.get("mode") == "idle":
            facts["threads"] += 1
        elif name == "node_memory_MemTotal_bytes":
            facts["ram_gb"] = round(num / 1e9, 1)
        elif name == "node_filesystem_avail_bytes" and lab.get("mountpoint") == "/":
            facts["free_gb"] = round(num / 1e9, 1)
        elif name == "node_timex_sync_status":
            facts["clock_synced"] = num == 1
        elif name == "node_os_info":
            facts["os"] = lab.get("pretty_name")
        elif name == "node_uname_info":
            facts["hostname"], facts["kernel"] = lab.get("nodename"), lab.get("release")
        elif name == "node_systemd_unit_state" and num == 1 and lab.get("state") in ("active", "failed"):
            facts["units"][lab["name"]] = lab["state"]
        elif name.startswith("x3_gpu_") and "uuid" in lab:
            gpus.setdefault(lab["uuid"], {"uuid": lab["uuid"]})[name[len("x3_gpu_"):]] = num
    facts["gpus"] = list(gpus.values())
    facts["failed_units"] = sorted(u for u, st in facts["units"].items() if st == "failed")
    return facts


def metrics_health(ip):
    """What a node reveals without SSH: node_exporter on :9100 and Ollama on its worker ports."""
    try:
        with urllib.request.urlopen(f"http://{ip}:9100/metrics", timeout=4) as response:
            facts = parse_exporter(response.read().decode(errors="replace"))
    except Exception:  # noqa: BLE001
        return None
    facts["ollama_workers"] = {}
    for port in range(OLLAMA_BASE_PORT, OLLAMA_BASE_PORT + 16):
        if port != 11435 and tcp_open(ip, port, 0.5) and http_json(f"http://{ip}:{port}/api/version"):
            tags = http_json(f"http://{ip}:{port}/api/tags") or {}
            facts["ollama_workers"][port] = {"ok": True, "bind": "LAN", "models": [m["name"] for m in tags.get("models", [])]}
    return facts


def remote_health(host):
    code, out = run(["ssh", "-o", "BatchMode=yes", "-o", "ConnectTimeout=4", host, REMOTE_HEALTH], 25)
    if code != 0:
        return None
    try:
        return json.loads(out.splitlines()[-1])
    except (ValueError, IndexError):
        return None


def health(args, quiet=False):
    inv = load_inventory()
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
            remote = remote_health(name if x3jobs.name_resolves(name) else ip) if ssh_port else None
            row.update(reachable=reachable, ssh=bool(remote), ssh_port=ssh_port, label="PHYSICAL", remote=remote)
            if remote:
                row.update(cpu=f"{remote['threads']}t load {remote['load']:.1f}", ram=f"{remote['ram_gb']}G",
                           gpu=",".join(g.replace("NVIDIA GeForce ", "") for g in remote["gpus"]) or "-",
                           ollama_workers=remote["ollama_workers"], router=remote["router"],
                           disk_free_gb=remote["free_gb"], clock_synced=remote["clock_synced"],
                           failed_units=remote["failed_units"], node_role=remote["role"], repo_head=remote["repo_head"])
            elif ssh_port:
                row["detail"] = "sshd answers but refuses this node's key: run x3-join.sh on it"
                seen = metrics_health(ip)
                if seen and seen.get("hostname") == name:
                    row.update(label="PHYSICAL-METRICS", metrics=seen, cpu=f"{seen['threads']}t",
                               ram=f"{seen['ram_gb']}G", gpu=f"{len(seen['gpus'])}x{int(seen['gpus'][0]['memory_total_mib']) // 1024}GB"
                               if seen["gpus"] else "-", ollama_workers=seen["ollama_workers"])
            elif reachable:
                row["detail"] = "pings but no sshd on :22"
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
            for r in rows:
                if r["node"] != me and r.get("ollama_workers") is not None:
                    for port, w in r["ollama_workers"].items():
                        lan = tcp_open(r["ip"], int(port))
                        print(f"ollama {r['node']}:{port}  ok bind={w['bind']} reachable-from-{me}={'yes' if lan else 'NO'}"
                              f"  loaded={','.join(w['loaded']) or '-'}")
                    print(f"{r['node']}: disk free {r['disk_free_gb']} GB  clock synced {r['clock_synced']}  "
                          f"failed units {r['failed_units'] or 'none'}  repo {str(r.get('repo_head'))[:12]}")
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
            gpu_count = len(gpus())
        elif row.get("remote"):
            check(f"{row['node']}.role_identity", row["node_role"] == row["role"], row["node_role"])
            gpu_count = len(row["remote"]["gpus"])
        else:
            continue
        n = row["node"]
        check(f"{n}.clock_synced", row["clock_synced"])
        check(f"{n}.disk_free_10gb", row["disk_free_gb"] >= 10, row["disk_free_gb"])
        check(f"{n}.no_failed_units", not row["failed_units"], row["failed_units"])
        if row["role"] == "gpu":
            check(f"{n}.gpu_workers", len(row["ollama_workers"]) >= max(1, gpu_count), sorted(row["ollama_workers"]))
            if n != me:
                # The router on the control node has to reach every worker it routes to.
                closed = [p for p in row["ollama_workers"] if not tcp_open(row["ip"], int(p))]
                check(f"{n}.workers_reachable_from_{me}", not closed, f"closed: {closed}" if closed else "all")
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


# ---------------------------------------------------------------- jobs

def resolve_commit(ref):
    code, sha = run(["git", "-C", str(REPO), "rev-parse", "--verify", f"{ref}^{{commit}}"])
    if code != 0:
        sys.exit(f"cannot resolve {ref!r} to a commit in {REPO}")
    return sha


def stream_line(line):
    sys.stdout.write(line)
    sys.stdout.flush()


def job(args):
    inv = load_inventory()
    cmd = " ".join(args.command[1:] if args.command[:1] == ["--"] else args.command)
    if not cmd:
        sys.exit("job: no command given (use: job --class TEST -- cargo test -p crate)")
    j = x3jobs.new_job(args.cls, cmd, resolve_commit(args.ref), args.ref, args.priority, args.timeout, args.node,
                       args.min_free_gb, args.name)
    record = x3jobs.run_job(j, inv, socket.gethostname(), REPO / "audit-artifacts" / "x3-cluster",
                            stream=None if args.quiet else stream_line)
    summary = {k: record.get(k) for k in ("id", "class", "worker", "label", "result", "exit_code", "seconds", "detail")}
    print(json.dumps(summary, indent=2))
    print(f"evidence: {REPO / 'audit-artifacts' / 'x3-cluster' / j['commit'] / 'jobs' / j['id']}")
    return 0 if record["result"] == "PASS" else 1


def pipeline(args):
    inv = load_inventory()
    spec = json.loads(Path(args.spec).read_text())
    commit = resolve_commit(args.ref)
    root = REPO / "audit-artifacts" / "x3-cluster"
    result = x3jobs.run_pipeline(spec, inv, socket.gethostname(), commit, args.ref, root,
                                 stream=None if args.quiet else stream_line)
    out = root / commit / f"pipeline-{dt.datetime.now(dt.timezone.utc).strftime('%Y%m%dT%H%M%SZ')}.json"
    out.write_text(json.dumps(result, indent=2) + "\n")
    for stage in result["stages"]:
        print(f"== {stage['stage']}: {stage['result']}")
        for j in stage["jobs"]:
            print(f"   {j['result']:8} {j['class']:11} {j.get('name', ''):24} {j.get('worker') or '-':10} "
                  f"{j.get('label') or '-':15} {j.get('seconds') or '':>8}  {j.get('detail') or ''}")
    print(f"\npipeline {result['result']} @ {commit[:12]} -> {out}")
    return 0 if result["result"] == "PASS" else 1


def ssh_config(args):
    """Host blocks for inventory nodes with an IP that ~/.ssh/config does not already define."""
    inv = load_inventory()
    existing = Path.home() / ".ssh" / "config"
    defined = set(re.findall(r"^\s*Host\s+(.+)$", existing.read_text(), re.M)) if existing.exists() else set()
    defined = {h for line in defined for h in line.split()}
    blocks = []
    for name, node in inv["nodes"].items():
        if name == socket.gethostname() or not node.get("ip") or name in defined:
            continue
        blocks.append(f"Host {name}\n    HostName {node['ip']}\n    User {os.environ.get('USER', 'lojak')}\n"
                      "    IdentityFile ~/.ssh/id_ed25519\n    IdentitiesOnly yes\n    StrictHostKeyChecking accept-new\n"
                      "    ConnectTimeout 8\n    ServerAliveInterval 30\n    ServerAliveCountMax 6\n    TCPKeepAlive yes\n")
    text = "# Generated by scripts/x3-cluster/x3cluster.py ssh-config from inventory.json\n" + "\n".join(blocks)
    if args.write:
        target = Path.home() / ".ssh" / "x3-cluster.conf"
        target.write_text(text)
        target.chmod(0o600)
        if existing.exists() and "Include ~/.ssh/x3-cluster.conf" not in existing.read_text():
            shutil.copy2(existing, existing.with_name(f"config.bak-{dt.datetime.now().strftime('%Y%m%dT%H%M%S')}"))
            # Include must come before any Host block to apply globally.
            existing.write_text("Include ~/.ssh/x3-cluster.conf\n\n" + existing.read_text())
        print(f"wrote {target} ({len(blocks)} hosts)")
    else:
        print(text)


# ---------------------------------------------------------------- discover / onboard

ONBOARDED = CONFIG / "onboarded"
EVENTS = Path.home() / ".local" / "state" / "x3-cluster" / "events.log"
TOOL_FILES = ("x3cluster.py", "x3jobs.py", "inventory.json", "x3-cluster-health", "x3-node-bootstrap", "x3-join.sh")
RETRY_SECONDS = 1800


def event(message):
    """One line per state change, for the operator to read later."""
    EVENTS.parent.mkdir(parents=True, exist_ok=True)
    with open(EVENTS, "a") as f:
        f.write(f"{now()} {message}\n")
    print(message, flush=True)


def sweep_ssh(lan, skip, port=22, timeout=0.7):
    """IPs in `lan` with something listening on :22 (ICMP is often filtered; SSH is what we need)."""
    import concurrent.futures
    import ipaddress
    hosts = [str(h) for h in ipaddress.ip_network(lan, strict=False).hosts() if str(h) not in skip]
    with concurrent.futures.ThreadPoolExecutor(64) as pool:
        return sorted((ip for ip, ok in zip(hosts, pool.map(lambda h: tcp_open(h, port, timeout), hosts)) if ok),
                      key=lambda ip: tuple(int(o) for o in ip.split(".")))


def ssh_run(host, command, timeout=60, stdin=None):
    try:
        out = subprocess.run(["ssh", "-o", "BatchMode=yes", "-o", "ConnectTimeout=6", "-o", "StrictHostKeyChecking=accept-new",
                              host, command], input=stdin, capture_output=True, timeout=timeout)
        return out.returncode, out.stdout.decode(errors="replace").strip(), out.stderr.decode(errors="replace").strip()
    except (OSError, subprocess.TimeoutExpired) as exc:
        return 124, "", str(exc)


def classify_hosts(inv, answers):
    """Map {ip: hostname-or-None} from a sweep onto the inventory.

    Returns (found {node: ip}, pending [ip], conflicts [str]). A host only claims a
    node if its own hostname says so; an IP is never guessed from position."""
    found, pending, conflicts = {}, [], []
    for ip, hostname in answers.items():
        if hostname is None:
            pending.append(ip)
            continue
        node = inv["nodes"].get(hostname)
        if node is None:
            continue
        if node.get("ip") and node["ip"] != ip and not node.get("discovered"):
            conflicts.append(f"{hostname} answers at {ip} but inventory.json says {node['ip']}")
            continue
        found[hostname] = ip
    return found, pending, conflicts


def discover(args):
    inv = load_inventory()
    me = socket.gethostname()
    own = {n["ipv4"].split("/")[0] for n in nics() if n["ipv4"]}
    # Every sshd is asked who it is: a node with a fixed inventory IP still needs onboarding
    # the first time it accepts this node's key (x3gpu2 had an IP before it had our key).
    open_ssh = sweep_ssh(inv["lan"], own)
    answers = {}
    for ip in open_ssh:
        code, out, _ = ssh_run(ip, "hostname", 15)
        answers[ip] = out.splitlines()[-1].strip() if code == 0 and out else None
    found, pending, conflicts = classify_hosts(inv, answers)
    try:
        state = json.loads(DISCOVERED.read_text())
    except (OSError, ValueError):
        state = {}
    for name, ip in found.items():
        if state.get(name, {}).get("ip") != ip:
            event(f"discovered {name} ({inv['nodes'][name]['role']}) at {ip}")
        _, neigh = run(["ip", "neigh", "show", ip])
        mac = re.search(r"lladdr (\S+)", neigh)
        state[name] = {"ip": ip, "seen": now(), "mac": mac.group(1) if mac else None}
    DISCOVERED.parent.mkdir(parents=True, exist_ok=True)
    DISCOVERED.write_text(json.dumps(state, indent=2) + "\n")
    for ip in pending:
        event(f"sshd at {ip} refuses {me}'s key; run x3-join.sh on it to join the cluster")
    for line in conflicts:
        event(f"CONFLICT {line}")
    print(json.dumps({"from": me, "swept": inv["lan"], "ssh_hosts": open_ssh, "found": found,
                      "pending_key": pending, "conflicts": conflicts}, indent=2))
    if args.onboard:
        for name, ip in found.items():
            try:
                last = json.loads((ONBOARDED / f"{name}.json").read_text())
            except (OSError, ValueError):
                last = {}
            if last.get("status") == "ok" and last.get("ip") == ip:
                continue
            if last and time.time() - last.get("attempted_epoch", 0) < RETRY_SECONDS and not args.force:
                continue
            onboard_node(name, ip, load_inventory())
    return 0


def router_static_endpoints():
    """base_urls the router's own config.json already routes to (the live one and this checkout's)."""
    urls = set()
    for path in (Path.home() / "Desktop/xxxstar-main/services/x3-ai-router/config.json",
                 REPO / "services/x3-ai-router/config.json"):
        try:
            urls |= {p.get("base_url", "").rstrip("/") for p in json.loads(path.read_text()).get("providers", {}).values()}
        except (OSError, ValueError):
            pass
    return urls


def onboard_node(name, ip, inv):
    """Bring a discovered node into the cluster with no sudo: tooling, checkout, role bootstrap,
    (gpu) per-GPU workers + models + router registration, benchmark, and a real job on it."""
    role = inv["nodes"][name]["role"]
    inv["nodes"][name]["ip"] = ip
    me = socket.gethostname()
    steps = []
    record = {"node": name, "ip": ip, "role": role, "controller": me, "started": now(), "label": "PHYSICAL",
              "tool_commit": git_identity()["commit"], "attempted_epoch": time.time(), "steps": steps}

    def step(title, ok, detail=None):
        steps.append({"step": title, "pass": bool(ok), "detail": detail})
        return bool(ok)

    def finish(status):
        record.update(status=status, finished=now())
        ONBOARDED.mkdir(parents=True, exist_ok=True)
        (ONBOARDED / f"{name}.json").write_text(json.dumps(record, indent=2, default=str) + "\n")
        out = REPO / "audit-artifacts" / "x3-cluster" / (record["tool_commit"] or "unknown")
        out.mkdir(parents=True, exist_ok=True)
        (out / f"onboard-{name}.json").write_text(json.dumps(record, indent=2, default=str) + "\n")
        failed = [s["step"] for s in steps if not s["pass"]]
        event(f"onboard {name}: {status}" + (f" (failed: {', '.join(failed)})" if failed else ""))
        return record

    event(f"onboarding {name} ({role}) at {ip}")
    code, out, err = ssh_run(ip, "hostname", 15)
    if not step("identity", code == 0 and out.strip() == name, out or err):
        return finish("failed")

    # 1. tooling, copied from this control node so every node runs the same version
    buf = io.BytesIO()
    with tarfile.open(fileobj=buf, mode="w") as tar:
        for f in TOOL_FILES:
            if (HERE / f).exists():
                tar.add(HERE / f, arcname=f)
    code, _, err = ssh_run(ip, "mkdir -p ~/.local/share/x3-cluster ~/.local/bin && tar -xf - -C ~/.local/share/x3-cluster && "
                               "for t in x3-cluster-health x3-node-bootstrap; do ln -sf ~/.local/share/x3-cluster/$t ~/.local/bin/$t; done",
                           60, stdin=buf.getvalue())
    if not step("install_tooling", code == 0, err or "~/.local/share/x3-cluster"):
        return finish("failed")

    # 2. a checkout to run jobs in (partial clone: commits and trees now, blobs on checkout)
    remote = inv.get("canonical_repo", x3jobs.CANONICAL_REMOTE)
    code, out, err = ssh_run(ip, f"""set -e
for r in ~/Desktop/xxxstar-main ~/Desktop/xxxstar-master ~/xxxstar; do [ -e "$r/.git" ] && {{ echo "$r"; exit 0; }}; done
command -v git >/dev/null || {{ echo "git missing: run x3-join.sh" >&2; exit 3; }}
git clone -q --filter=blob:none --no-checkout {remote} ~/xxxstar && echo ~/xxxstar""", 1800)
    repo = out.splitlines()[-1] if code == 0 and out else None
    if not step("checkout", repo, repo or err):
        return finish("failed")
    ssh_run(ip, f"mkdir -p ~/.config/x3-cluster && f=~/.config/x3-cluster/node.env && touch $f && "
                f"(grep -q ^X3_REPO= $f || echo X3_REPO={repo} >> $f)", 15)

    # 3. role bootstrap (creates one Ollama worker per GPU for role=gpu)
    tool = f"X3_REPO={repo} python3 ~/.local/share/x3-cluster/x3cluster.py"
    code, out, err = ssh_run(ip, f"{tool} bootstrap --role {role} --apply --lan", 300)
    try:
        boot = json.loads(out[out.index("{"):])
    except ValueError:
        boot = {}
    if not step("bootstrap", code == 0 and boot, boot.get("missing_tools") if boot else (err or out)[-500:]):
        return finish("failed")
    record["bootstrap"] = boot
    _, staged, _ = ssh_run(ip, "cat ~/.config/x3-cluster/staged-privileged.sh 2>/dev/null", 15)
    record["staged_privileged"] = staged or None

    if role == "gpu":
        workers = boot.get("gpu_workers", [])
        if not step("gpus_detected", workers, [w["gpu"] for w in workers] or "no NVIDIA GPU visible: driver missing?"):
            return finish("blocked")
        bad = [w for w in workers if w["state"] not in ("create", "adopted")]
        step("workers_planned", not bad, [{k: w.get(k) for k in ("worker", "port", "state", "gpu")} for w in workers])
        time.sleep(5)
        code, out, _ = ssh_run(ip, "for p in " + " ".join(str(w["port"]) for w in workers) +
                               "; do curl -fsS -m 5 127.0.0.1:$p/api/version >/dev/null && echo $p; done", 60)
        up = [int(p) for p in out.split()] if out else []
        if not step("workers_up", len(up) == len(workers), {"up": up, "expected": [w["port"] for w in workers]}):
            return finish("blocked")
        # Models: the coding model on every worker; the tool-capable model too if disk allows.
        _, free, _ = ssh_run(ip, "df -P --block-size=1G ~ | awk 'NR==2{print $4}'", 15)
        free_gb = int(free) if free.isdigit() else 0
        models = ["qwen2.5-coder:7b"] + (["qwen3:8b"] if free_gb >= 12 * len(workers) + 20 else [])
        pulled = {}
        for w in workers:
            for m in models:
                code, _, _ = ssh_run(ip, f"OLLAMA_HOST=127.0.0.1:{w['port']} ollama pull {m} >/dev/null 2>&1", 3600)
                pulled[f"{w['port']}:{m}"] = code == 0
        step("models_pulled", all(pulled.values()), {"free_gb_before": free_gb, "pulled": pulled})
        ssh_run(ip, f"{tool} bootstrap --role gpu --apply --lan", 300)  # re-register with the models now present
        _, reg, _ = ssh_run(ip, f"cat ~/.config/x3-cluster/registration/{name}.json", 15)
        try:
            reg = json.loads(reg)
        except ValueError:
            reg = {}
        hosts_ok = all(p["base_url"].startswith(f"http://{ip}:") for p in reg.get("providers", {}).values())
        static = router_static_endpoints()
        already = sorted(n for n, p in reg.get("providers", {}).items() if p["base_url"].rstrip("/") in static)
        if already and len(already) == len(reg.get("providers", {})):
            step("registration", hosts_ok, f"skipped drop-in: router config.json already routes to {sorted(static & {p['base_url'].rstrip('/') for p in reg['providers'].values()})}")
        elif step("registration", reg.get("providers") and hosts_ok, sorted(reg.get("providers", {}))):
            drop = Path.home() / ".config" / "x3-router" / "providers.d"
            drop.mkdir(parents=True, exist_ok=True)
            (drop / f"{name}.json").write_text(json.dumps(reg, indent=2) + "\n")
            record["router_dropin"] = str(drop / f"{name}.json")
        reachable = {w["port"]: tcp_open(ip, w["port"]) for w in workers}
        step(f"workers_reachable_from_{me}", all(reachable.values()),
             reachable if all(reachable.values()) else f"{reachable}: run the staged firewall block on {name}")

    # 4. baseline benchmark (the clean Rust build is skipped automatically below 50 GB free)
    code, out, err = ssh_run(ip, f"{tool} bench", 2400)
    try:
        record["bench"] = json.loads(out[out.index("{"):])
    except ValueError:
        record["bench"] = None
    step("bench", record["bench"], None if record["bench"] else (err or out)[-300:])

    # 5. a real job routed to it by name, at the control node's (pushed) commit
    if role == "gpu":
        cls = "INFERENCE"
        cmd = ("for p in " + " ".join(str(w["port"]) for w in boot.get("gpu_workers", [])) + "; do "
               "curl -fsS -m 600 http://127.0.0.1:$p/api/generate -d '{\"model\":\"qwen2.5-coder:7b\","
               "\"prompt\":\"Reply with the single word: ok\",\"stream\":false,\"options\":{\"num_predict\":8}}' "
               "-o \"$X3_JOB_OUT/worker-$p.json\" || exit 1; done")
    else:
        cls = {"build": "BUILD", "sim": "SIMULATION", "data": "DATABASE", "net": "NETWORK", "ops": "NETWORK"}.get(role, "TEST")
        cmd = "git rev-parse HEAD && uname -a && nproc && free -g"
    job = x3jobs.new_job(cls, cmd, git_identity()["commit"], "HEAD", "NORMAL", 3600, name, 1, f"onboard-{name}")
    result = x3jobs.run_job(job, inv, me, REPO / "audit-artifacts" / "x3-cluster")
    step(f"first_job_{cls.lower()}", result["result"] == "PASS",
         {k: result.get(k) for k in ("id", "worker", "label", "result", "exit_code", "detail")})
    return finish("ok" if all(s["pass"] for s in steps) else "partial")


def onboard(args):
    inv = load_inventory()
    if args.node not in inv["nodes"]:
        sys.exit(f"{args.node} is not in inventory.json")
    ip = args.ip or inv["nodes"][args.node].get("ip")
    if not ip:
        sys.exit(f"no IP for {args.node}: run `discover` or pass --ip")
    record = onboard_node(args.node, ip, inv)
    print(json.dumps({k: record[k] for k in ("node", "status", "steps")}, indent=2, default=str))
    return 0 if record["status"] == "ok" else 1


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = parser.add_subparsers(dest="cmd", required=True)
    p = sub.add_parser("inventory")
    p.add_argument("--json", action="store_true")
    p = sub.add_parser("bootstrap")
    p.add_argument("--role", choices=ROLES, required=True)
    p.add_argument("--apply", action="store_true", help="create/enable user units (default: plan only)")
    p.add_argument("--lan", action="store_true", help="bind GPU workers to 0.0.0.0 and stage firewall rules")
    p = sub.add_parser("bench")
    p.add_argument("--no-rust", action="store_true", help="skip the clean Rust build")
    p = sub.add_parser("health")
    p.add_argument("--json", action="store_true")
    sub.add_parser("gate")
    p = sub.add_parser("job")
    p.add_argument("--class", dest="cls", choices=sorted(x3jobs.CLASSES), required=True)
    p.add_argument("--priority", choices=list(x3jobs.PRIORITIES), default="NORMAL")
    p.add_argument("--timeout", type=int, default=3600, help="seconds")
    p.add_argument("--node", help="pin to this node instead of routing by class")
    p.add_argument("--ref", default="HEAD", help="commit-ish; must be fetchable from GitHub for remote workers")
    p.add_argument("--min-free-gb", type=int)
    p.add_argument("--name")
    p.add_argument("--quiet", action="store_true")
    p.add_argument("command", nargs=argparse.REMAINDER)
    p = sub.add_parser("pipeline")
    p.add_argument("--spec", default=str(HERE / "pipeline.json"))
    p.add_argument("--ref", default="HEAD")
    p.add_argument("--quiet", action="store_true")
    p = sub.add_parser("discover", help="sweep the LAN for inventory nodes answering SSH with our key")
    p.add_argument("--onboard", action="store_true", help="onboard newly found nodes")
    p.add_argument("--force", action="store_true", help="retry onboarding now, ignoring the 30 min back-off")
    p = sub.add_parser("onboard", help="bring one node into the cluster (no sudo)")
    p.add_argument("node")
    p.add_argument("--ip")
    p = sub.add_parser("ssh-config")
    p.add_argument("--write", action="store_true", help="write ~/.ssh/x3-cluster.conf and Include it")
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
    elif args.cmd == "job":
        sys.exit(job(args))
    elif args.cmd == "pipeline":
        sys.exit(pipeline(args))
    elif args.cmd == "discover":
        sys.exit(discover(args))
    elif args.cmd == "onboard":
        sys.exit(onboard(args))
    elif args.cmd == "ssh-config":
        ssh_config(args)


if __name__ == "__main__":
    main()
