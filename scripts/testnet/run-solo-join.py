#!/usr/bin/env python3
"""Deterministic cold-start: node 1 bootnodes SOLO and builds a canonical head; nodes 2..N
then join and sync it before authoring (substrate syncs when far/anchor behind), eliminating
the cold-race fork seen when all 7 author from genesis concurrently. X3_DEV_SEED per node.
"""
import json, os, subprocess, sys, time, urllib.request

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(os.path.dirname(__file__))))
NODE = os.path.join(ROOT, "target", "release", "x3-chain-node")
FRESH = os.path.join(ROOT, "deployment", "chain-specs", "fresh")
SPEC = os.path.join(FRESH, "x3-testnet-plain.json")
KEYS = os.path.join(FRESH, "validator-keys")
COUNT = int(sys.argv[1]) if len(sys.argv) > 1 else 7
LEAD_BLOCKS = int(sys.argv[2]) if len(sys.argv) > 2 else 60
BASE = os.environ.get("TESTNET_BASE", "/tmp/x3-solo-join")
BP2P, BRPC = 30733, 9980
P2P_IP = "127.0.0.1"
os.makedirs(BASE + "/logs", exist_ok=True); os.makedirs(BASE + "/pids", exist_ok=True)

def p2p(i): return BP2P + i - 1
def rpc(i): return BRPC + i - 1
def seed(i):
    with open(os.path.join(KEYS, f"validator-{i}.suri")) as f:
        return next(l.split("=",1)[1].strip() for l in f if l.startswith("seed="))

def stop_prev():
    import glob
    for f in glob.glob(BASE + "/pids/*.pid"):
        try:
            with open(f) as pf: os.kill(int(pf.read()), 15)
        except Exception: pass
    time.sleep(2)

def start(i, boot=None):
    b = os.path.join(BASE, f"node-{i}"); os.makedirs(b, exist_ok=True)
    cmd = [NODE, "--chain", SPEC, "--base-path", b, "--name", f"sj{i}",
           "--rpc-port", str(rpc(i)), "--rpc-methods=Unsafe", "--no-prometheus",
           "--unsafe-force-node-key-generation", "--validator", "--force-authoring",
           "--allow-private-ip",
           "--listen-addr", f"/ip4/0.0.0.0/tcp/{p2p(i)}",
           "--public-addr", f"/ip4/{P2P_IP}/tcp/{p2p(i)}",
           "--no-mdns", "--no-telemetry", "--disable-log-color", "--execution=native-else-wasm"]
    if boot: cmd += ["--bootnodes", boot]
    env = dict(os.environ); env["X3_DEV_SEED"] = seed(i)
    logf = open(os.path.join(BASE, "logs", f"node-{i}.log"), "w")
    p = subprocess.Popen(cmd, env=env, stdout=logf, stderr=subprocess.STDOUT, start_new_session=True)
    open(os.path.join(BASE, "pids", f"node-{i}.pid"), "w").write(str(p.pid))
    print(f"[node] sj{i} pid={p.pid} rpc={rpc(i)} p2p={p2p(i)}")
    return p

def rpc_call(port, method, params=None):
    req = urllib.request.Request(f"http://127.0.0.1:{port}",
        data=json.dumps({"jsonrpc":"2.0","id":1,"method":method,"params":params or []}).encode(),
        headers={"Content-Type":"application/json"})
    with urllib.request.urlopen(req, timeout=3) as r: return json.load(r).get("result")

def head(port):
    try:
        h = rpc_call(port, "chain_getHeader")
        return int(h["number"],16) if h else 0
    except Exception: return 0
def pid_(port):
    for _ in range(90):
        try:
            v = rpc_call(port, "system_localPeerId")
            if v: return v
        except Exception: pass
        time.sleep(1)
    raise RuntimeError("no peer id")

stop_prev()
print(f"[ok] COUNT={COUNT} LEAD={LEAD_BLOCKS} spec={SPEC}")
start(1)
p1 = pid_(rpc(1))
# node1 authors SOLO to canonical lead
print(f"[net] node1 peer {p1}; letting it author ~{LEAD_BLOCKS} blocks solo...")
waited = 0
while head(rpc(1)) < LEAD_BLOCKS and waited < 240:
    time.sleep(4); waited += 1
print(f"[net] node1 head={head(rpc(1))} (solo canonical)")
boot = f"/ip4/{P2P_IP}/tcp/{p2p(1)}/p2p/{p1}"
for i in range(2, COUNT+1):
    start(i, boot=boot); time.sleep(1)
# wait for the joined set to converge & finalize: finalized head should advance past genesis
print("[ok] joiners started; waiting ~50s for GRANDPA finality convergence...")
time.sleep(50)
fin = {}
for i in range(1, COUNT+1):
    try:
        f = rpc_call(rpc(i), "chain_getFinalizedHead")
        fin[i] = f[:24] if f else None
    except Exception: fin[i] = None
heads = set(v for v in fin.values() if v)
print("[net] finalized-heads:", fin)
print("[net] unique finalized:", len(heads), "-> ", "CONVERGED" if len(heads) <= 1 else "FORKED (unique>1)")
for i in range(1, COUNT+1):
    try: print(f"  sj{i}: head={head(rpc(i))} final={fin[i]}")
    except Exception: pass
