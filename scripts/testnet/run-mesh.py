#!/usr/bin/env python3
"""RESERVED FULL-MESH launcher for the X3 local testnet.

Closes GAP-CONSENSUS-REPRO-1 / GAP-P2P-1 / GAP-MESH-1 (see TESTNET_GAP_LEDGER.md).

Root cause of intermittent cold-start forks on this host: default libp2p bring-up on a
single loopback interface forms a sparse/star-ish graph; when that graph partitions (cold
race or a member loss) subgroups of GRANDPA authorities can finalize different branches.
Start-ordering mitigations are NOT enough (an identical solo-lead+join run forked later —
GAP-CONSENSUS-REPRO-1).

Fix: give each validator a DETERMINISTIC `--node-key` (fixed ed25519 secret per index) and
wire a RESERVED FULL MESH via `--reserved-nodes /ip4/127.0.0.1/tcp/<p2p-port>/p2p/<PeerId>`
so every node has a hard-reserved connection to every other node before authoring starts.
A full mesh has no leaf and no single point of failure: killing any one member leaves a
fully-connected (>=5/7) majority that keeps finalizing ONE chain.

The substrate `--reserved-nodes` contract REQUIRES the `/p2p/<PeerId>` suffix, and PeerId is
not derivable purely from the key inside this CLI — so PeerIds are DERIVED from the node
itself (ground truth): node n is booted once (sequentially, throwaway state) with its fixed
`--node-key`, `system_localPeerId` is read, and it is shut down. Because the key is fixed,
that PeerId is deterministic and reproducible for every later mesh boot. Results are cached
under <base>/peerids.json (keyed by node-key) so repeated cycles reuse them.

Usage:
  run-mesh.py derive  --count N            # print deterministic PeerIds for nodes 1..N
  run-mesh.py cycles  --count N --cycles K # K fresh cold-starts; assert every cycle converges
  run-mesh.py kills   --count N            # converge once, kill each node -> survival

Environment:
  TESTNET_BASE   override base dir (default /tmp/x3-mesh)
  P2P_IP         dial IP (default 127.0.0.1)
"""
import argparse, json, os, subprocess, sys, time, urllib.request, glob, shutil

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(os.path.dirname(__file__))))
NODE = os.path.join(ROOT, "target", "release", "x3-chain-node")
FRESH = os.path.join(ROOT, "deployment", "chain-specs", "fresh")
SPEC = os.path.join(FRESH, "x3-testnet-plain.json")
KEYS = os.path.join(FRESH, "validator-keys")

BASE = os.environ.get("TESTNET_BASE", "/tmp/x3-mesh")
P2P_IP = os.environ.get("P2P_IP", "127.0.0.1")
BP2P, BRPC = 30633, 9970
CACHE = os.path.join(BASE, "peerids.json")
CONVERGE_HARD = 120           # max seconds to converge a full mesh


def p2p(i): return BP2P + i - 1
def rpc(i): return BRPC + i - 1


def seed(i):
    with open(os.path.join(KEYS, f"validator-{i}.suri")) as f:
        return next(l.split("=", 1)[1].strip() for l in f if l.startswith("seed="))


def nodekey(i):
    """Deterministic ed25519 libp2p secret per validator index: 32 bytes as 64 hex chars."""
    return "0x" + f"{i:064x}"


def rpc_json(port, method, params=None, timeout=4):
    req = urllib.request.Request(
        f"http://127.0.0.1:{port}",
        data=json.dumps({"jsonrpc": "2.0", "id": 1, "method": method, "params": params or []}).encode(),
        headers={"Content-Type": "application/json"})
    with urllib.request.urlopen(req, timeout=timeout) as r:
        return json.load(r).get("result")


def local_peerid(port, tries=180, delay=1):
    for _ in range(tries):
        try:
            v = rpc_json(port, "system_localPeerId")
            if v:
                return v
        except Exception:
            pass
        time.sleep(delay)
    return None


def final_hash_gt0(port, tries=1, delay=0):
    """Return the finalized head hash with block number>0, or None when dead/na/not yet."""
    for _ in range(tries):
        try:
            fh = rpc_json(port, "chain_getFinalizedHead", timeout=3)
            if not fh:
                return None
            h = rpc_json(port, "chain_getHeader", [fh], timeout=3)
            if h and int(h.get("number", "0x0"), 16) > 0:
                return fh
        except Exception:
            return None
        if delay:
            time.sleep(delay)
    return None


def live_finalized(count):
    """{i: finalizedhash(>genesis)} for nodes reachable and GRANDPA-active."""
    out = {}
    for i in range(1, count + 1):
        f = final_hash_gt0(rpc(i))
        if f:
            out[i] = f
    return out


def base_cmd(i, base):
    return [NODE, "--chain", SPEC, "--base-path", os.path.join(base, f"node-{i}"),
            "--name", f"mesh{i}", "--rpc-port", str(rpc(i)), "--rpc-methods=Unsafe",
            "--no-prometheus", "--node-key", nodekey(i), "--validator", "--force-authoring",
            "--allow-private-ip", "--listen-addr", f"/ip4/0.0.0.0/tcp/{p2p(i)}",
            "--public-addr", f"/ip4/{P2P_IP}/tcp/{p2p(i)}", "--no-mdns", "--no-telemetry",
            "--disable-log-color", "--execution=native-else-wasm"]


def start_node(i, base, reserved, logdir):
    cmd = base_cmd(i, base)
    for ra in sorted(reserved):
        cmd += ["--reserved-nodes", ra]
    env = dict(os.environ)
    env["X3_DEV_SEED"] = seed(i)
    os.makedirs(logdir, exist_ok=True)
    logf = open(os.path.join(logdir, f"node-{i}.log"), "w")
    p = subprocess.Popen(cmd, env=env, stdout=logf, stderr=subprocess.STDOUT,
                         start_new_session=True)
    os.makedirs(os.path.join(base, "pids"), exist_ok=True)
    with open(os.path.join(base, "pids", f"node-{i}.pid"), "w") as pf:
        pf.write(str(p.pid))
    return p


def kill_pids(base):
    for f in glob.glob(os.path.join(base, "pids", "*.pid")):
        try:
            with open(f) as pf:
                pid = int(pf.read().strip())
                try:
                    os.kill(pid, 15)
                except ProcessLookupError:
                    pass
        except Exception:
            pass


def wipe_datadir(base, i):
    """Remove one node's chain datadir so the next boot is a cold start from genesis."""
    shutil.rmtree(os.path.join(base, f"node-{i}"), ignore_errors=True)


def peercount(port):
    try:
        h = rpc_json(port, "system_health", timeout=3)
        return h.get("peers") if h else None
    except Exception:
        return None


# --------------------------------------------------------------------------
# PeerId derivation (ground truth, from the node itself)
# --------------------------------------------------------------------------
def derive_peer_ids(count):
    dbase = os.path.join(BASE, "derive")
    kill_pids(dbase)
    os.makedirs(os.path.join(dbase, "logs"), exist_ok=True)
    out = {}
    for i in range(1, count + 1):
        wipe_datadir(dbase, i)
        cmd = base_cmd(i, dbase)
        logf = open(os.path.join(dbase, "logs", f"node-{i}.log"), "w")
        env = dict(os.environ, X3_DEV_SEED=seed(i))
        p = subprocess.Popen(cmd, env=env, stdout=logf, stderr=subprocess.STDOUT,
                             start_new_session=True)
        pid = local_peerid(rpc(i))
        try:
            os.kill(p.pid, 15)
        except ProcessLookupError:
            pass
        if not pid:
            raise RuntimeError(f"derive: node {i} never reported system_localPeerId")
        out[i] = pid
        print(f"   derived node-{i}: {pid}")
        time.sleep(3)  # allow p2p/rpc port reuse before next derivation
    return out


def cached_peer_ids(count):
    os.makedirs(os.path.dirname(CACHE), exist_ok=True)
    if os.path.exists(CACHE):
        try:
            data = json.load(open(CACHE))
        except Exception:
            data = {}
        keys = [nodekey(i) for i in range(1, count + 1)]
        have = all(str(i) in data and data.get(f"key{i}") == keys[i - 1]
                   for i in range(1, count + 1))
        if have:
            return {i: data[str(i)] for i in range(1, count + 1)}
    ids = derive_peer_ids(count)
    store = {"schema": 1}
    for i in range(1, count + 1):
        store[str(i)] = ids[i]
        store[f"key{i}"] = nodekey(i)
    json.dump(store, open(CACHE, "w"), indent=2)
    return ids


def reserved_addrs(peer_ids):
    """Full mesh: each node reserves every OTHER node loopback+p2p+PeerId."""
    return {i: [f"/ip4/{P2P_IP}/tcp/{p2p(j)}/p2p/{peer_ids[j]}" for j in peer_ids if j != i]
            for i in peer_ids}


# --------------------------------------------------------------------------
# Mesh convergence / survival
# --------------------------------------------------------------------------
def wait_single_final_chain(count, alive, hard=CONVERGE_HARD):
    """Wait until every alive node shares ONE finalized head past genesis."""
    t0 = time.time()
    guard = 0
    while time.time() - t0 < hard:
        fh = live_finalized(count)
        live = [i for i in alive if i in fh]
        if live and len(set(fh.values())) == 1 and len(live) == len(alive):
            return {i: fh[i] for i in alive}, time.time() - t0
        guard += 1
        if guard % 15 == 0:
            print(f"   ...waiting convergence ({guard}*2s): {len(live)}/{len(alive)} alive "
                  f"finalizing={len(set(fh.values())) if fh else 'none'}")
        time.sleep(2)
    return None, time.time() - t0


def status_line(count):
    parts = []
    for i in range(1, count + 1):
        try:
            fh = rpc_json(rpc(i), "chain_getFinalizedHead", timeout=2)
            hh = fh[:16] if fh else "-"
            parts.append(f"mesh{i}:final={hh},peers={peercount(rpc(i))}")
        except Exception:
            parts.append(f"mesh{i}:DOWN")
    return " | ".join(parts)


def boot_mesh(count, peer_ids, base):
    addrs = reserved_addrs(peer_ids)
    kill_pids(base)
    for i in range(1, count + 1):
        wipe_datadir(base, i)          # ensure a true cold start from empty genesis
    os.makedirs(base, exist_ok=True)
    procs = {}
    for i in range(1, count + 1):
        procs[i] = start_node(i, base, addrs[i], os.path.join(base, "logs"))
    return procs


# --------------------------------------------------------------------------
def cmd_derive(args):
    ids = cached_peer_ids(args.count)
    print("Deterministic reserved-mesh identities:")
    for i in sorted(ids):
        print(f"  node-{i}: key={nodekey(i)} peerid={ids[i]}  "
              f"addr=/ip4/{P2P_IP}/tcp/{p2p(i)}/p2p/{ids[i]}")


def cmd_cycles(args):
    peer_ids = cached_peer_ids(args.count)
    print(f"[mesh] count={args.count} deterministic keys -> reserved full mesh")
    ok = []
    for cyc in range(1, args.cycles + 1):
        base = os.path.join(BASE, "cycles", f"cycle-{cyc}")
        print(f"\n=== cycle {cyc}/{args.cycles} (fresh genesis, full mesh) ===")
        boot_mesh(args.count, peer_ids, base)
        conv, dt = wait_single_final_chain(args.count, list(range(1, args.count + 1)))
        uniq = len(set(conv.values())) if conv else None
        good = bool(conv) and uniq == 1
        ok.append(good)
        print(f"[cycle {cyc}] converged in {dt:.0f}s unique_finalized_heads={uniq} -> "
              + ("CONVERGED" if good else "NOT-CONVERGED"))
        print("   ", status_line(args.count))
        kill_pids(base)
        time.sleep(3)
    good = sum(1 for x in ok if x)
    print(f"\n[result] cold-start cycles converged: {good}/{args.cycles}")
    if good != args.cycles:
        print("[FAIL] not every cold start produced one finalized chain.")
        sys.exit(1)
    print("[OK] RESERVED FULL MESH: every cold-start converged to ONE finalized head.")


def cmd_kills(args):
    peer_ids = cached_peer_ids(args.count)
    base = os.path.join(BASE, "kills")
    print(f"[mesh] k-count={args.count}; each victim taken from a fresh converged mesh")
    survivor_ok = {}
    for victim in range(1, args.count + 1):
        alive = [i for i in range(1, args.count + 1) if i != victim]
        # fresh converged 7-node mesh before EACH kill test (single-loss semantics)
        sub_base = os.path.join(base, f"victim-{victim}")
        boot_mesh(args.count, peer_ids, sub_base)
        conv, dt = wait_single_final_chain(args.count, list(range(1, args.count + 1)))
        if not conv or len(set(conv.values())) != 1 or len(conv) != args.count:
            kill_pids(sub_base)
            print(f"[FAIL] baseline for victim {victim} did not converge; aborting.")
            sys.exit(1)
        print(f"[ok] victim={victim}: baseline converged in {dt:.0f}s; killing mesh{victim}...")
        with open(os.path.join(sub_base, "pids", f"node-{victim}.pid")) as f:
            os.kill(int(f.read().strip()), 15)
        # require ALL survivors to keep finalizing ONE common chain within 60s
        snap, t0 = None, time.time()
        conv2, dt2 = wait_single_final_chain(args.count, alive, hard=60)
        uniq = len(set(conv2.values())) if conv2 else None
        down = len(alive) - (len(conv2) if conv2 else 0)
        good = bool(conv2) and uniq == 1 and down == 0
        survivor_ok[victim] = good
        print(f"[kill mesh{victim}] {down} survivors lost RPC; {uniq} finalized head(s) among "
              f"{len(conv2) if conv2 else 0}/{len(alive)} in {dt2:.0f}s -> "
              + ("SINGLE CHAIN" if good else "DIVERGED/PARTITION"))
        print("   ", status_line(args.count))
        kill_pids(sub_base)
        time.sleep(3)
    allok = len(survivor_ok) == args.count and all(survivor_ok.values())
    print(f"\n[result] kill-survival (one loss at a time from full mesh): "
          f"{sum(survivor_ok.values())}/{args.count} victims -> survivors kept ONE chain")
    kill_pids(base)
    print("")
    if not allok:
        print("[FAIL] some single-member loss partitioned the full mesh.")
        sys.exit(1)
    print("[OK] RESERVED FULL MESH: loss of ANY single member left >=5 finalizing ONE chain.")


def main():
    ap = argparse.ArgumentParser(description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = ap.add_subparsers(dest="mode", required=True)
    d = sub.add_parser("derive"); d.add_argument("--count", type=int, default=7)
    c = sub.add_parser("cycles"); c.add_argument("--count", type=int, default=7)
    c.add_argument("--cycles", type=int, default=8)
    k = sub.add_parser("kills"); k.add_argument("--count", type=int, default=7)
    a = ap.parse_args()
    {"derive": cmd_derive, "cycles": cmd_cycles, "kills": cmd_kills}[a.mode](a)


if __name__ == "__main__":
    main()
