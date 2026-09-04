#!/usr/bin/env python3
"""Boot a local X3 testnet with REDUNDANT bootnodes and advertised addresses.

Why redundant: a star (single bootnode) forks when its center dies. Here bootnodes 1..B are
each also connected to each other (each later bootnode lists prior bootnodes), and nodes
B+1..N list ALL bootnodes. Advertised --public-addr lets substrate DHT gossip addresses so
connected peers can also discover each other. Killing one bootnode leaves the rest as centers.
"""
import json, os, re, subprocess, sys, time, urllib.request

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(os.path.dirname(__file__))))
NODE = os.path.join(ROOT, "target", "release", "x3-chain-node")
FRESH = os.path.join(ROOT, "deployment", "chain-specs", "fresh")
SPEC = os.path.join(FRESH, "x3-testnet-plain.json")
KEYS = os.path.join(FRESH, "validator-keys")
COUNT = int(sys.argv[1]) if len(sys.argv) > 1 else 7
BOOTN = min(3, COUNT)          # number of redundant bootnodes
BASE = os.environ.get("TESTNET_BASE", "/tmp/x3-fresh-multi")
P2P_IP = os.environ.get("P2P_IP", "127.0.0.1")
BP2P, BRPC = 30633, 9970       # port bases
os.makedirs(os.path.join(BASE, "logs"), exist_ok=True)
os.makedirs(os.path.join(BASE, "pids"), exist_ok=True)

def p2p(i): return BP2P + i - 1
def rpc(i): return BRPC + i - 1
def seed(i):
    with open(os.path.join(KEYS, f"validator-{i}.suri")) as f:
        return next(l.split("=", 1)[1].strip() for l in f if l.startswith("seed="))

def stop_prev():
    import glob
    for f in glob.glob(os.path.join(BASE, "pids", "*.pid")):
        try:
            with open(f) as pf: os.kill(int(pf.read()), 15)
        except Exception: pass
    time.sleep(2)

# deterministic node-key per index so PeerIds are stable across reloads (helps human ops)
def nodekey(i): return f"00{i:02d}" + ("00" * 30)

procs = {}
def start(i, bootnodes=None):
    b = os.path.join(BASE, f"node-{i}"); os.makedirs(b, exist_ok=True)
    cmd = [NODE, "--chain", SPEC, "--base-path", b, "--name", f"mv{i}",
           "--rpc-port", str(rpc(i)), "--rpc-methods=Unsafe", "--no-prometheus",
           "--unsafe-force-node-key-generation", "--node-key", nodekey(i),
           "--validator", "--force-authoring", "--allow-private-ip",
           "--listen-addr", f"/ip4/0.0.0.0/tcp/{p2p(i)}",
           "--public-addr", f"/ip4/{P2P_IP}/tcp/{p2p(i)}",
           "--no-mdns", "--no-telemetry", "--disable-log-color",
           "--execution=native-else-wasm"]
    if bootnodes:
        for bn in bootnodes:
            cmd += ["--bootnodes", bn]
    env = dict(os.environ); env["X3_DEV_SEED"] = seed(i)
    logf = open(os.path.join(BASE, "logs", f"node-{i}.log"), "w")
    p = subprocess.Popen(cmd, env=env, stdout=logf, stderr=subprocess.STDOUT, start_new_session=True)
    procs[i] = p
    with open(os.path.join(BASE, "pids", f"node-{i}.pid"), "w") as pf: pf.write(str(p.pid))
    print(f"[node] mv{i} pid={p.pid} p2p={p2p(i)} rpc={rpc(i)} seed=*")

def rpc_call(port, method, params=None):
    req = urllib.request.Request(
        f"http://127.0.0.1:{port}",
        data=json.dumps({"jsonrpc": "2.0", "id": 1, "method": method, "params": params or []}).encode(),
        headers={"Content-Type": "application/json"})
    with urllib.request.urlopen(req, timeout=3) as r:
        return json.load(r).get("result")

def wait_peer(port, tries=90):
    for _ in range(tries):
        try:
            pid = rpc_call(port, "system_localPeerId"); 
            if pid: return pid
        except Exception: pass
        time.sleep(1)
    return None

def wait_head(port, gt=1, tries=80):
    for _ in range(tries):
        try:
            h = rpc_call(port, "chain_getHeader")
            if h and int(h["number"], 16) >= gt: return True
        except Exception: pass
        time.sleep(1)
    return False

def main():
    stop_prev()
    # 1) boot the BOOTNODE set: each new bootnode peers with prior bootnodes (connects them)
    print(f"[spec] {SPEC}\n[ok] COUNT={COUNT} REDUNDANT_BOOTNODES={BOOTN}")
    start(1)
    pid1 = wait_peer(rpc(1))
    if not pid1: print("[err] node1 peer id"); sys.exit(1)
    peers1 = [f"/ip4/{P2P_IP}/tcp/{p2p(1)}/p2p/{pid1}"]
    for i in range(2, BOOTN + 1):
        start(i, bootnodes=peers1)   # i lists all earlier bootnodes
        time.sleep(2)
        pidi = wait_peer(rpc(i))
        if pidi: peers1.append(f"/ip4/{P2P_IP}/tcp/{p2p(i)}/p2p/{pidi}")
        print(f"[net] bootnode{i} add {pidi}")
    # wait nodes 1..BOOTN authoring
    for i in range(1, BOOTN + 1):
        wait_head(rpc(i))
    print(f"[net] bootnodes ready: {len(peers1)} centers")
    # 2) boot the rest peering to ALL bootnodes
    for i in range(BOOTN + 1, COUNT + 1):
        start(i, bootnodes=peers1)
        time.sleep(1)
    # brief report
    time.sleep(6)
    for i in range(1, COUNT + 1):
        try:
            h = rpc_call(rpc(i), "system_health")
            print(f"  mv{i}: peers={h['peers']} syncing={h['isSyncing']}")
        except Exception as e:
            print(f"  mv{i}: rpc error")
    print(f"[ok] started; pid files {BASE}/pids; logs {BASE}/logs")

main()
