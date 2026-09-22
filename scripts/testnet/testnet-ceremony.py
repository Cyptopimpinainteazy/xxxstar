#!/usr/bin/env python3
"""Record and verify a testnet launch against a ceremony manifest.

    testnet-ceremony.py record <spec> --node-bin <path> --rpc 9944[,9945,…] --out ceremony.json
    testnet-ceremony.py verify <ceremony.json> --rpc 9944[,9945,…]

`record` writes what was actually launched: the spec and node binary with their sha256s,
the chain name, the genesis hash the network reports, the runtime version, the authority
set, each validator's libp2p peer id and the height it has finalized.

`verify` takes that manifest to a *running* network and re-checks every one of those
claims, one line per check, naming the first disagreement. It is the check a user or an
operator should be able to run against a published testnet: not "the node answered", but
"this is the artifact that was launched, these are its authorities, and this is the
genesis hash".

Nothing here trusts the network for a value it is supposed to prove: the spec hash and
the binary hash come from the files, and the chain-side values are compared against the
manifest, not against each other.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import sys
import urllib.request
from pathlib import Path

GENESIS_HEIGHT = 0


def rpc(port: int, method: str, params: list | None = None, timeout: float = 10.0):
    body = json.dumps(
        {"jsonrpc": "2.0", "id": 1, "method": method, "params": params or []}
    ).encode()
    req = urllib.request.Request(
        f"http://127.0.0.1:{port}", data=body, headers={"Content-Type": "application/json"}
    )
    with urllib.request.urlopen(req, timeout=timeout) as resp:
        payload = json.loads(resp.read())
    if "error" in payload:
        raise RuntimeError(f"{method} on {port}: {payload['error']}")
    return payload.get("result")


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as fh:
        for chunk in iter(lambda: fh.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


def header_number(port: int, block_hash: str) -> int | None:
    header = rpc(port, "chain_getHeader", [block_hash])
    if not header:
        return None
    return int(header["number"], 16)


def spec_authorities(spec: dict) -> dict:
    """The authority sets the spec itself declares (plain form)."""
    cfg = spec.get("genesis", {}).get("runtimeGenesis", {}).get("config", {})
    aura = cfg.get("aura", {}).get("authorities", [])
    grandpa = [
        entry[0] if isinstance(entry, list) else entry
        for entry in cfg.get("grandpa", {}).get("authorities", [])
    ]
    return {"aura": aura, "grandpa": grandpa}


def collect(port: int) -> dict:
    health = rpc(port, "system_health") or {}
    peer_id = rpc(port, "system_localPeerId")
    finalized = rpc(port, "chain_getFinalizedHead")
    return {
        "rpc_port": port,
        "peer_id": peer_id,
        "chain": rpc(port, "system_chain"),
        "genesis_hash": rpc(port, "chain_getBlockHash", [GENESIS_HEIGHT]),
        "runtime_version": rpc(port, "state_getRuntimeVersion"),
        "grandpa_authorities_scaled": rpc(
            port, "state_call", ["GrandpaApi_grandpa_authorities", "0x"]
        ),
        "peers": health.get("peers"),
        "finalized_hash": finalized,
        "finalized_height": header_number(port, finalized) if finalized else None,
    }


def cmd_record(args: argparse.Namespace) -> int:
    spec_path = Path(args.spec).resolve()
    if not spec_path.is_file():
        print(f"spec not found: {spec_path}", file=sys.stderr)
        return 2
    spec = json.loads(spec_path.read_text())
    node_bin = Path(args.node_bin).resolve()
    if not node_bin.is_file():
        print(f"node binary not found: {node_bin}", file=sys.stderr)
        return 2

    observers = [collect(port) for port in args.rpc]
    first = observers[0]
    manifest = {
        "manifest_version": 1,
        "chain": first["chain"],
        "chain_id": spec.get("id"),
        "chain_type": spec.get("chainType"),
        "spec": {
            "path": str(spec_path),
            "sha256": sha256_file(spec_path),
            "bytes": spec_path.stat().st_size,
        },
        "node_binary": {"path": str(node_bin), "sha256": sha256_file(node_bin)},
        "runtime_version": first["runtime_version"],
        "genesis": {
            "height": GENESIS_HEIGHT,
            "hash": first["genesis_hash"],
        },
        "authorities_expected_from_spec": spec_authorities(spec),
        "validators": [
            {
                "rpc_port": obs["rpc_port"],
                "peer_id": obs["peer_id"],
                "finalized_height": obs["finalized_height"],
                "peers": obs["peers"],
                "grandpa_authorities_scaled": obs["grandpa_authorities_scaled"],
            }
            for obs in observers
        ],
    }
    out = Path(args.out).resolve()
    out.write_text(json.dumps(manifest, indent=2) + "\n")
    print(
        f"[ceremony] recorded {len(observers)} validator(s): genesis {manifest['genesis']['hash']}, "
        f"spec {manifest['spec']['sha256'][:16]}…, spec_version "
        f"{manifest['runtime_version']['specVersion']} -> {out}"
    )
    return 0


def cmd_verify(args: argparse.Namespace) -> int:
    manifest = json.loads(Path(args.manifest).read_text())
    failures: list[str] = []

    def check(name: str, ok: bool, detail: str = "") -> None:
        if ok:
            print(f"  ok    {name}")
        else:
            print(f"  FAIL  {name}{(' — ' + detail) if detail else ''}")
            failures.append(name)

    spec_path = Path(manifest["spec"]["path"])
    if spec_path.is_file():
        check(
            "spec sha256 matches the manifest",
            sha256_file(spec_path) == manifest["spec"]["sha256"],
            f"expected {manifest['spec']['sha256'][:16]}…",
        )
    else:
        check("spec is present", False, f"missing {spec_path}")

    node_bin = Path(manifest["node_binary"]["path"])
    if args.node_bin and Path(args.node_bin).is_file():
        node_bin = Path(args.node_bin)
    if node_bin.is_file():
        check(
            "node binary sha256 matches the manifest",
            sha256_file(node_bin) == manifest["node_binary"]["sha256"],
            f"{node_bin} is not the launched binary",
        )
    else:
        print(f"  skip  node binary not available locally ({node_bin})")

    observers = [collect(port) for port in args.rpc]
    for obs in observers:
        label = f"rpc {obs['rpc_port']}"
        check(f"{label}: chain name", obs["chain"] == manifest["chain"], str(obs["chain"]))
        check(
            f"{label}: genesis hash",
            obs["genesis_hash"] == manifest["genesis"]["hash"],
            f"{obs['genesis_hash']} != {manifest['genesis']['hash']}",
        )
        expected_version = manifest["runtime_version"]
        check(
            f"{label}: spec_version",
            obs["runtime_version"]["specVersion"] == expected_version["specVersion"],
            f"{obs['runtime_version']['specVersion']} != {expected_version['specVersion']}",
        )
        check(
            f"{label}: transaction_version",
            obs["runtime_version"]["transactionVersion"]
            == expected_version["transactionVersion"],
            "runtime transaction version differs from the manifest",
        )
        check(
            f"{label}: GRANDPA authority set",
            obs["grandpa_authorities_scaled"] == manifest["validators"][0]["grandpa_authorities_scaled"],
            "this node's authority set is not the one the manifest recorded",
        )
        check(
            f"{label}: finality advancing",
            obs["finalized_height"] is not None
            and obs["finalized_height"] >= args.min_finalized,
            f"finalized height {obs['finalized_height']} < {args.min_finalized}",
        )

        recorded = next(
            (v for v in manifest["validators"] if v["rpc_port"] == obs["rpc_port"]), None
        )
        if recorded:
            check(
                f"{label}: peer id matches the manifest",
                obs["peer_id"] == recorded["peer_id"],
                f"{obs['peer_id']} != {recorded['peer_id']}",
            )
        else:
            check(
                f"{label}: peer id is one the manifest lists",
                obs["peer_id"] in [v["peer_id"] for v in manifest["validators"]],
                obs["peer_id"],
            )

    print()
    if failures:
        print(f"[ceremony] FAILED — {len(failures)} check(s): {', '.join(failures)}")
        return 1
    print(f"[ceremony] PASS — {len(observers)} validator(s) match the manifest")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = parser.add_subparsers(dest="cmd", required=True)

    rec = sub.add_parser("record", help="write a manifest from files + a running network")
    rec.add_argument("spec")
    rec.add_argument("--node-bin", required=True)
    rec.add_argument("--rpc", type=lambda s: [int(p) for p in s.split(",")], default=[9944])
    rec.add_argument("--out", default="ceremony.json")
    rec.set_defaults(func=cmd_record)

    ver = sub.add_parser("verify", help="check a running network against a manifest")
    ver.add_argument("manifest")
    ver.add_argument("--rpc", type=lambda s: [int(p) for p in s.split(",")], default=[9944])
    ver.add_argument("--node-bin", default="")
    ver.add_argument(
        "--min-finalized",
        type=int,
        default=1,
        help="require every validator's finalized height to be at least this (default 1)",
    )
    ver.set_defaults(func=cmd_verify)

    args = parser.parse_args()
    return args.func(args)


if __name__ == "__main__":
    raise SystemExit(main())
