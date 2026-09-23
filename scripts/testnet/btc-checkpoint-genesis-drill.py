#!/usr/bin/env python3
"""Prove that a chain spec can pin Bitcoin's checkpoint and the chain comes up anchored.

The settlement engine's BTC path fails closed until a checkpoint is anchored, and
`anchor_btc_checkpoint` is a root call. On a testnet that is a manual step someone has to
remember, which is why genesis carries the checkpoints too. This drill is the live half of
that claim: it builds two specs from the same node binary — one plain, one with a real
Bitcoin regtest header pinned — and reads the resulting genesis state and the running
chain's storage back out.

What it checks, in order:

1. the key math is verified against Substrate's known `twox_128("System")` prefix, so a
   wrong key cannot be mistaken for an empty value;
2. the anchored spec's genesis state contains `BtcCheckpoints(height) = hash`,
   `BtcHeaderMetaStore(hash) = {height, anchored: true}` and `BtcBestHeight = height`;
3. the plain spec's genesis state contains none of them — the difference is the spec;
4. a node started from the anchored spec serves the same values over RPC and still
   produces blocks, so an anchored genesis is not a chain that cannot author;
5. a spec whose checkpoint is not a mined Bitcoin header makes the node refuse to start,
   with the pallet's own message.

The default checkpoint is a real one: block 121 of a Bitcoin Core v28.1.0 regtest chain,
captured by `scripts/btc/capture-regtest-spv.py`. Pass `--capture` to use another.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import pathlib
import shutil
import subprocess
import sys
import tempfile
import time
import urllib.error
import urllib.request

ROOT = pathlib.Path(__file__).resolve().parents[2]

# ── Substrate storage keys ───────────────────────────────────────────────────────────────
# Twox = xxHash64 with a seed; Substrate's `twox_128` is seeds 0 and 1 concatenated, and
# `twox_64_concat` is the seed-0 hash followed by the key itself. Reimplemented here
# because the storage values this drill reads are the whole point, and a wrong key looks
# exactly like a missing one — so it is checked against a known vector before use.
MASK = (1 << 64) - 1
P1, P2, P3, P4, P5 = (11400714785074694791, 14029467366897019727, 1609587929392839161,
                      9650029242287828579, 2870177450012600261)
KNOWN_TWOX_128_SYSTEM = "26aa394eea5630e07c48ae0c9558cef7"


def _rotl(x: int, r: int) -> int:
    return ((x << r) | (x >> (64 - r))) & MASK


def _round(acc: int, inp: int) -> int:
    return (_rotl((acc + inp * P2) & MASK, 31) * P1) & MASK


def xxh64(data: bytes, seed: int = 0) -> int:
    n, i = len(data), 0
    if n >= 32:
        v1, v2, v3, v4 = ((seed + P1 + P2) & MASK, (seed + P2) & MASK,
                          seed & MASK, (seed - P1) & MASK)
        while i + 32 <= n:
            v1 = _round(v1, int.from_bytes(data[i:i + 8], "little")); i += 8
            v2 = _round(v2, int.from_bytes(data[i:i + 8], "little")); i += 8
            v3 = _round(v3, int.from_bytes(data[i:i + 8], "little")); i += 8
            v4 = _round(v4, int.from_bytes(data[i:i + 8], "little")); i += 8
        h = (_rotl(v1, 1) + _rotl(v2, 7) + _rotl(v3, 12) + _rotl(v4, 18)) & MASK
        for v in (v1, v2, v3, v4):
            h = ((h ^ _round(0, v)) * P1 + P4) & MASK
    else:
        h = (seed + P5) & MASK
    h = (h + n) & MASK
    while i + 8 <= n:
        h ^= _round(0, int.from_bytes(data[i:i + 8], "little")); i += 8
        h = (_rotl(h, 27) * P1 + P4) & MASK
    if i + 4 <= n:
        h ^= (int.from_bytes(data[i:i + 4], "little") * P1) & MASK; i += 4
        h = (_rotl(h, 23) * P2 + P3) & MASK
    while i < n:
        h ^= (data[i] * P5) & MASK; i += 1
        h = (_rotl(h, 11) * P1) & MASK
    h ^= h >> 33; h = (h * P2) & MASK; h ^= h >> 29; h = (h * P3) & MASK; h ^= h >> 32
    return h


def twox_128(name: str) -> bytes:
    raw = name.encode()
    return xxh64(raw, 0).to_bytes(8, "little") + xxh64(raw, 1).to_bytes(8, "little")


def twox_64_concat(key: bytes) -> bytes:
    return xxh64(key, 0).to_bytes(8, "little") + key


def blake2_128_concat(key: bytes) -> bytes:
    return hashlib.blake2b(key, digest_size=16).digest() + key


PALLET = "X3SettlementEngine"


def key_best_height() -> str:
    return ("0x" + (twox_128(PALLET) + twox_128("BtcBestHeight")).hex())


def key_checkpoint(height: int) -> str:
    encoded = height.to_bytes(8, "little")
    return ("0x" + (twox_128(PALLET) + twox_128("BtcCheckpoints")
                    + twox_64_concat(encoded)).hex())


def key_header_meta(block_hash_internal: bytes) -> str:
    return ("0x" + (twox_128(PALLET) + twox_128("BtcHeaderMetaStore")
                    + blake2_128_concat(block_hash_internal)).hex())


# ── helpers ─────────────────────────────────────────────────────────────────────────────

class DrillFailure(Exception):
    pass


def run(cmd, **kw):
    return subprocess.run(cmd, capture_output=True, text=True, **kw)


def rpc(port: int, method: str, params=None, timeout: float = 10.0):
    body = json.dumps({"jsonrpc": "2.0", "id": 1, "method": method,
                       "params": params or []}).encode()
    req = urllib.request.Request(f"http://127.0.0.1:{port}", data=body,
                                 headers={"Content-Type": "application/json"})
    with urllib.request.urlopen(req, timeout=timeout) as resp:
        payload = json.loads(resp.read())
    if "error" in payload:
        raise DrillFailure(f"{method} returned {payload['error']}")
    return payload.get("result")


def wait_for_rpc(port: int, proc: subprocess.Popen, seconds: int = 120) -> str:
    deadline = time.time() + seconds
    while time.time() < deadline:
        if proc.poll() is not None:
            raise DrillFailure(f"the node exited with status {proc.returncode} before its "
                               f"RPC came up")
        try:
            return rpc(port, "system_chain")
        except (urllib.error.URLError, OSError, DrillFailure):
            time.sleep(1)
    raise DrillFailure(f"RPC on {port} did not come up within {seconds}s")


def build_spec(node_bin: str, env_checkpoints: str | None, out: pathlib.Path) -> None:
    env = dict(os.environ)
    if env_checkpoints is None:
        env.pop("X3_BTC_CHECKPOINTS", None)
    else:
        env["X3_BTC_CHECKPOINTS"] = env_checkpoints
    with open(out, "w") as fh:
        r = subprocess.run([node_bin, "build-spec", "--chain", "dev"],
                           stdout=fh, stderr=subprocess.PIPE, text=True, env=env)
    if r.returncode != 0:
        raise DrillFailure(f"build-spec failed: {r.stderr.strip()[:400]}")


def raw_genesis_top(node_bin: str, spec: pathlib.Path) -> dict:
    r = run([node_bin, "build-spec", "--chain", str(spec), "--raw"])
    if r.returncode != 0:
        raise DrillFailure(f"build-spec --raw failed: {r.stderr.strip()[:400]}")
    return json.loads(r.stdout)["genesis"]["raw"]["top"]


def start_node(node_bin: str, spec: pathlib.Path, base: pathlib.Path, rpc_port: int,
               p2p_port: int, prom_port: int, log: pathlib.Path) -> subprocess.Popen:
    # A libp2p identity is required: this build exits with `NetworkKeyNotFound` rather than
    # inventing one, so a throwaway key is passed explicitly. It is per-run and discarded.
    node_key = os.urandom(32).hex()
    handle = open(log, "w")
    proc = subprocess.Popen(
        [node_bin, "--chain", str(spec), "--base-path", str(base),
         "--rpc-port", str(rpc_port), "--port", str(p2p_port),
         "--prometheus-port", str(prom_port), "--alice", "--validator",
         "--node-key", node_key, "--rpc-cors", "all", "--name", "btc-anchor-drill"],
        stdout=handle, stderr=subprocess.STDOUT, text=True)
    proc._log_handle = handle  # keep the file object alive for the process's lifetime
    return proc


def stop_node(proc: subprocess.Popen) -> None:
    proc.terminate()
    try:
        proc.wait(timeout=30)
    except subprocess.TimeoutExpired:
        proc.kill()
        proc.wait(timeout=15)
    handle = getattr(proc, "_log_handle", None)
    if handle:
        handle.close()


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--node-bin", default=str(ROOT / "target/debug/x3-chain-node"))
    ap.add_argument("--capture",
                    default=str(ROOT / ".ai/reports/btc-regtest-capture-20260922.json"))
    ap.add_argument("--rpc-port", type=int, default=11044)
    ap.add_argument("--p2p-port", type=int, default=31400)
    ap.add_argument("--prom-port", type=int, default=19615)
    ap.add_argument("--keep", action="store_true", help="leave the temp dirs behind")
    args = ap.parse_args()

    checks: list[tuple[str, bool, str]] = []

    def check(name: str, ok: bool, detail: str = "") -> None:
        checks.append((name, ok, detail))
        print(f"  {'PASS' if ok else 'FAIL'}  {name}{'' if ok else ' — ' + detail}")

    print("[drill] key math")
    got = twox_128("System").hex()
    if got != KNOWN_TWOX_128_SYSTEM:
        print(f"FAIL: twox_128(\"System\") = {got}, expected {KNOWN_TWOX_128_SYSTEM}. "
              f"Every storage key this drill computes would be wrong; refusing to run.",
              file=sys.stderr)
        return 2
    print(f"  PASS  twox_128(\"System\") = {got} (Substrate's known prefix)")

    node_bin = args.node_bin
    if not pathlib.Path(node_bin).exists():
        print(f"node binary not found: {node_bin}", file=sys.stderr)
        return 2

    capture = json.loads(pathlib.Path(args.capture).read_text())
    header_hex = capture["header_hex"]
    height = int(capture["height"])
    header_bytes = bytes.fromhex(header_hex)
    # Substrate stores Bitcoin's hash in the order the pallet computes it: double SHA-256
    # over the 80 wire bytes. A block explorer shows that value reversed.
    internal_hash = hashlib.sha256(hashlib.sha256(header_bytes).digest()).digest()
    block_hash_display = capture["block_hash"]
    if "next_prev_block_hash_hex" in capture:
        # The next block's parent link is the same hash, written down by the node.
        assert capture["next_prev_block_hash_hex"] == internal_hash.hex(), \
            "the capture's own parent link disagrees with the header's hash"

    print(f"[drill] capture: {capture['chain']} block {height} {block_hash_display}")

    tmp = pathlib.Path(tempfile.mkdtemp(prefix="x3-btc-anchor-drill-"))
    procs: list[subprocess.Popen] = []
    try:
        anchored_spec = tmp / "spec-anchored.json"
        plain_spec = tmp / "spec-plain.json"
        build_spec(node_bin, f"{header_hex}@{height}", anchored_spec)
        build_spec(node_bin, None, plain_spec)

        pinned = json.loads(anchored_spec.read_text())["genesis"]["runtimeGenesis"]["config"]
        pinned_headers = pinned["x3SettlementEngine"].get("btcCheckpoints", [])
        print("[drill] the spec carries the checkpoint")
        check("anchored spec lists the header",
              len(pinned_headers) == 1 and pinned_headers[0]["height"] == height,
              f"btcCheckpoints = {pinned_headers}")
        plain_cfg = json.loads(plain_spec.read_text())["genesis"]["runtimeGenesis"]["config"]
        check("plain spec lists none",
              not plain_cfg["x3SettlementEngine"].get("btcCheckpoints"),
              f"btcCheckpoints = {plain_cfg['x3SettlementEngine'].get('btcCheckpoints')}")

        # Genesis state, as the node builds it.
        print("[drill] genesis storage (build-spec --raw)")
        top = raw_genesis_top(node_bin, anchored_spec)
        plain_top = raw_genesis_top(node_bin, plain_spec)

        expected_meta = height.to_bytes(8, "little") + b"\x01"
        check("BtcCheckpoints(height) pins the block hash",
              top.get(key_checkpoint(height)) == "0x" + internal_hash.hex(),
              f"{top.get(key_checkpoint(height))}")
        check("BtcHeaderMetaStore(hash) says anchored at that height",
              top.get(key_header_meta(internal_hash)) == "0x" + expected_meta.hex(),
              f"{top.get(key_header_meta(internal_hash))}")
        check("BtcBestHeight is the checkpoint height",
              top.get(key_best_height()) == "0x" + height.to_bytes(8, "little").hex(),
              f"{top.get(key_best_height())}")
        check("plain spec's genesis has no checkpoint",
              key_checkpoint(height) not in plain_top
              and key_header_meta(internal_hash) not in plain_top,
              "found the anchored spec's entries in the plain spec")

        # A running node, from the anchored spec.
        print("[drill] a node started from the anchored spec")
        base = tmp / "node"
        base.mkdir()
        log = tmp / "node.log"
        proc = start_node(node_bin, anchored_spec, base, args.rpc_port, args.p2p_port,
                          args.prom_port, log)
        procs.append(proc)
        chain = wait_for_rpc(args.rpc_port, proc)
        check("the node starts and serves RPC", bool(chain), str(chain))

        live_checkpoint = rpc(args.rpc_port, "state_getStorage", [key_checkpoint(height)])
        live_meta = rpc(args.rpc_port, "state_getStorage", [key_header_meta(internal_hash)])
        live_best = rpc(args.rpc_port, "state_getStorage", [key_best_height()])
        check("live storage: BtcCheckpoints(height)",
              live_checkpoint == "0x" + internal_hash.hex(), str(live_checkpoint))
        check("live storage: BtcHeaderMetaStore(hash)",
              live_meta == "0x" + expected_meta.hex(), str(live_meta))
        check("live storage: BtcBestHeight",
              live_best == "0x" + height.to_bytes(8, "little").hex(), str(live_best))

        # An anchored genesis must not be a chain that cannot author.
        first = rpc(args.rpc_port, "chain_getHeader")["number"]
        deadline = time.time() + 60
        advanced = False
        while time.time() < deadline:
            now = rpc(args.rpc_port, "chain_getHeader")["number"]
            if int(now, 16) >= int(first, 16) + 2:
                advanced = True
                break
            time.sleep(2)
        check("the anchored chain still produces blocks", advanced,
              f"height stayed at {first}")
        stop_node(proc)
        procs.remove(proc)

        # A spec that lies about Bitcoin must not launch.
        print("[drill] a spec whose checkpoint is not a mined Bitcoin header")
        bad_header = "00" * 4 + "11" * 32 + "22" * 32 + "00000000" + "ffff001d" + "00000000"
        bad_spec = tmp / "spec-bad.json"
        build_spec(node_bin, f"{bad_header}@{height}", bad_spec)
        bad_base = tmp / "node-bad"
        bad_base.mkdir()
        bad_log = tmp / "node-bad.log"
        bad = start_node(node_bin, bad_spec, bad_base, args.rpc_port + 1, args.p2p_port + 1,
                         args.prom_port + 1, bad_log)
        procs.append(bad)
        try:
            bad.wait(timeout=90)
        except subprocess.TimeoutExpired:
            stop_node(bad)
            raise DrillFailure("the node accepted a checkpoint that is not a mined header")
        text = bad_log.read_text(errors="replace")
        procs.remove(bad)
        refused = bad.returncode != 0 and "proof-of-work target" in text
        check("the node refuses it, with the pallet's reason", refused,
              f"exit {bad.returncode}; log tail: {text[-300:]}")

    except DrillFailure as e:
        print(f"\n[drill] FAIL: {e}", file=sys.stderr)
        for p in procs:
            stop_node(p)
        if not args.keep:
            shutil.rmtree(tmp, ignore_errors=True)
        return 1
    finally:
        for p in procs:
            stop_node(p)

    failed = [c for c in checks if not c[1]]
    print(f"\n[drill] {len(checks) - len(failed)}/{len(checks)} checks passed")
    if args.keep:
        print(f"[drill] artifacts kept in {tmp}")
    else:
        shutil.rmtree(tmp, ignore_errors=True)
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
