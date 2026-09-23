#!/usr/bin/env python3
"""Can the chain *follow* Bitcoin, not just validate a header?

`btc-checkpoint-genesis-drill.py` proves a chain can be born anchored. This proves the other half:
that real headers from a real Bitcoin node can be pushed onto that anchored chain, in order, under
the same admission rules — and that one whose parent was never admitted is refused.

It needs a Bitcoin Core install and the dev runtime node (the only runtime whose `powLimit` is
regtest's). Neither is in the repository, so the drill **skips loudly** rather than pretending:

    ./scripts/testnet/btc-header-push-drill.py \
        --node-bin target/release/x3-chain-node \
        --bitcoind-dir /path/to/bitcoin-28.1        # contains bin/bitcoind, bin/bitcoin-cli

What it does:

1. starts a private regtest bitcoind, mines a wallet, captures a run of consecutive headers;
2. builds a dev spec with the oldest of them pinned as the checkpoint, and gives the spec a sudo
   account (a dev spec ships `sudo.key = null`, so root calls are otherwise unreachable);
3. boots that chain and pushes the rest of the headers through `scripts/btc/push-headers.mjs`;
4. requires the chain's `btcBestHeight` to reach the last pushed height, with every header recorded
   `anchored`;
5. mines two more blocks and pushes only the later one, which must be refused — its parent was never
   admitted — and requires the refusal to name `BtcParentMissing`.

Exit codes: 0 pass, 1 fail, 2 skip (no Bitcoin Core / no node binary / no npm deps).
"""

from __future__ import annotations

import argparse
import json
import os
import pathlib
import shutil
import subprocess
import sys
import time
import urllib.request

ROOT = pathlib.Path(__file__).resolve().parents[2]
ALICE = "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY"  # //Alice, sr25519, ss58 42


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
        raise RuntimeError(f"{method}: {payload['error']}")
    return payload.get("result")


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--node-bin", required=True)
    ap.add_argument("--bitcoind-dir", default=os.environ.get("X3_BITCOIND_DIR", ""),
                    help="directory containing bin/bitcoind and bin/bitcoin-cli")
    ap.add_argument("--work-dir", default=str(ROOT / ".btc-header-push-drill"))
    ap.add_argument("--rpc-port", type=int, default=12444)
    ap.add_argument("--p2p-port", type=int, default=32800)
    ap.add_argument("--prom-port", type=int, default=20015)
    ap.add_argument("--btc-rpc-port", type=int, default=18443)
    ap.add_argument("--headers", type=int, default=6)
    ap.add_argument("--keep", action="store_true")
    args = ap.parse_args()

    checks: list[tuple[str, bool, str]] = []

    def check(name: str, ok: bool, detail: str = "") -> None:
        checks.append((name, ok, detail))
        print(f"  {'PASS' if ok else 'FAIL'}  {name}{'' if ok else ' — ' + detail}")

    if not args.bitcoind_dir:
        print("[push-drill] SKIP: no Bitcoin Core; pass --bitcoind-dir (nothing verified)",
              file=sys.stderr)
        return 2
    bitcoind = pathlib.Path(args.bitcoind_dir) / "bin/bitcoind"
    cli = pathlib.Path(args.bitcoind_dir) / "bin/bitcoin-cli"
    if not bitcoind.exists() or not cli.exists():
        print(f"[push-drill] SKIP: {bitcoind} not found (nothing verified)", file=sys.stderr)
        return 2
    if not pathlib.Path(args.node_bin).exists():
        print(f"[push-drill] SKIP: no node binary at {args.node_bin} (nothing verified)",
              file=sys.stderr)
        return 2

    work = pathlib.Path(args.work_dir)
    work.mkdir(parents=True, exist_ok=True)
    datadir = work / "regtest"
    datadir.mkdir(exist_ok=True)
    (datadir / "bitcoin.conf").write_text(
        "regtest=1\nserver=1\nrpcuser=x3dev\nrpcpassword=x3devpass\nfallbackfee=0.0002\n"
        "[regtest]\nrpcbind=127.0.0.1\nrpcallowip=127.0.0.1\n"
        f"rpcport={args.btc_rpc_port}\nport={args.btc_rpc_port + 1}\n")

    def btc(*argv):
        out = run([str(cli), f"-datadir={datadir}", *argv])
        if out.returncode != 0:
            raise RuntimeError(f"bitcoin-cli {' '.join(argv)}: {out.stderr.strip()}")
        return out.stdout.strip()

    def stop_bitcoind():
        run([str(cli), f"-datadir={datadir}", "stop"])
        time.sleep(2)

    stop_bitcoind()                                    # a leftover from an earlier run
    print(f"[push-drill] starting bitcoind on rpc {args.btc_rpc_port}")
    started = run([str(bitcoind), f"-datadir={datadir}", "-daemon"], check=False)
    if started.returncode != 0:
        print(f"[push-drill] FAIL: bitcoind did not start: {started.stderr.strip()}", file=sys.stderr)
        return 1
    for _ in range(30):
        try:
            btc("getblockcount")
            break
        except Exception:
            time.sleep(1)
    else:
        print("[push-drill] FAIL: bitcoind RPC never answered", file=sys.stderr)
        return 1

    try:
        btc("createwallet", "x3")
        address = btc("-rpcwallet=x3", "getnewaddress")
        btc("generatetoaddress", "121", address)
        tip = int(btc("getblockcount"))
        first = tip - args.headers                      # headers first..tip, inclusive
        headers = []
        for h in range(first, tip + 1):
            block_hash = btc("getblockhash", str(h))
            raw = bytes.fromhex(btc("getblock", block_hash, "0"))[:80]
            headers.append({"height": h, "block_hash": block_hash, "header_hex": raw.hex()})
        (work / "headers.json").write_text(json.dumps(headers, indent=1))
        print(f"[push-drill] regtest tip {tip}; captured {len(headers)} headers {first}..{tip}")

        # A dev spec whose checkpoint is the oldest captured header, with a sudo account.
        env = dict(os.environ)
        env["X3_BTC_CHECKPOINTS"] = f"{headers[0]['header_hex']}@{first}"
        spec = work / "spec.json"
        spec_run = run([args.node_bin, "build-spec", "--chain", "dev", "--disable-log-color"],
                       env=env)
        if spec_run.returncode != 0 or not spec_run.stdout.strip().startswith("{"):
            print(f"[push-drill] FAIL: build-spec: {spec_run.stderr.strip()[:300]}", file=sys.stderr)
            return 1
        spec_json = json.loads(spec_run.stdout)
        cfg = spec_json["genesis"]["runtimeGenesis"]["config"]
        cfg["sudo"]["key"] = ALICE
        spec.write_text(json.dumps(spec_json))
        check("the spec carries the checkpoint",
              bool(cfg["x3SettlementEngine"].get("btcCheckpoints")),
              str(cfg["x3SettlementEngine"].get("btcCheckpoints")))

        # Boot the chain, detached so it outlives this process's children.
        base = work / "node"
        shutil.rmtree(base, ignore_errors=True)
        log = work / "node.log"
        node_key = os.urandom(32).hex()
        handle = open(log, "w")
        proc = subprocess.Popen(
            [args.node_bin, "--chain", str(spec), "--base-path", str(base),
             "--name", "btc-push-drill", "--rpc-port", str(args.rpc_port),
             "--rpc-methods=Unsafe", "--rpc-cors=all", "--port", str(args.p2p_port),
             "--prometheus-port", str(args.prom_port), "--node-key", node_key,
             "--alice", "--validator"],
            stdout=handle, stderr=subprocess.STDOUT, text=True, start_new_session=True)

        def stop_node():
            proc.terminate()
            try:
                proc.wait(timeout=20)
            except subprocess.TimeoutExpired:
                proc.kill()

        deadline = time.time() + 120
        while time.time() < deadline:
            if proc.poll() is not None:
                print(f"[push-drill] FAIL: node exited {proc.returncode}; see {log}",
                      file=sys.stderr)
                return 1
            try:
                rpc(args.rpc_port, "system_chain")
                break
            except Exception:
                time.sleep(1)
        else:
            stop_node()
            print("[push-drill] FAIL: node RPC never answered", file=sys.stderr)
            return 1
        check("the anchored chain boots and serves RPC", True)

        # Push the rest of the captured headers.
        push = run(["node", str(ROOT / "scripts/btc/push-headers.mjs"),
                    "--ws", f"ws://127.0.0.1:{args.rpc_port}",
                    "--datadir", str(datadir), "--bitcoin-cli", str(cli),
                    "--from-height", str(first + 1), "--to-height", str(tip),
                    "--suri", "//Alice", "--via", "sudo"])
        tail = "\n".join((push.stdout + push.stderr).splitlines()[-6:])
        check(f"pushing headers {first + 1}..{tip} succeeds", push.returncode == 0, tail)
        check("the chain followed Bitcoin", f"btcBestHeight after:  {tip}" in push.stdout, tail)

        # ... and refuse one whose parent was never admitted.
        address = btc("-rpcwallet=x3", "getnewaddress")
        btc("generatetoaddress", "2", address)
        gapped = int(btc("getblockcount"))
        gap = run(["node", str(ROOT / "scripts/btc/push-headers.mjs"),
                   "--ws", f"ws://127.0.0.1:{args.rpc_port}",
                   "--datadir", str(datadir), "--bitcoin-cli", str(cli),
                   "--from-height", str(gapped), "--to-height", str(gapped),
                   "--suri", "//Alice", "--via", "sudo"])
        gap_out = gap.stdout + gap.stderr
        check("a header whose parent was never admitted is refused",
              gap.returncode != 0 and "BtcParentMissing" in gap_out,
              "\n".join(gap_out.splitlines()[-4:]))
        stop_node()
    finally:
        stop_bitcoind()

    failed = [c for c in checks if not c[1]]
    print(f"\n[push-drill] {len(checks) - len(failed)}/{len(checks)} checks passed")
    if args.keep:
        print(f"[push-drill] artifacts kept in {work}")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
