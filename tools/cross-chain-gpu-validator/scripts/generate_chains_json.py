"""Fetch chainlist and generate a canonical EVM+SVM chains JSON resource.

This script fetches https://chainid.network/chains.json, filters for chains
with EVM features (EIP155/EIP1559) or Solana, selects the first N chains
(sorted by chainId), and writes `src/resources/chains.json`.
"""
from __future__ import annotations

import json
from pathlib import Path
from typing import Any

import requests

URL = "https://chainid.network/chains.json"
OUT = Path(__file__).resolve().parents[1] / "src" / "resources" / "chains.json"
# No cap — take all discovered chains

EVM_FEATURES = {"EIP155", "EIP1559"}


def is_evm(chain: dict[str, Any]) -> bool:
    features = {f["name"] for f in chain.get("features", []) if isinstance(f, dict) and "name" in f}
    # treat presence of chainId + EIP155/EIP1559 or presence of RPC endpoints as EVM-compatible fallback
    return bool(chain.get("chainId")) and (bool(features & EVM_FEATURES) or bool(chain.get("rpc")))


def is_solana(chain: dict[str, Any]) -> bool:
    return chain.get("chain", "").lower() == "solana"


def canonical_chain_id(chain: dict[str, Any]) -> str:
    if chain.get("shortName"):
        return str(chain["shortName"]).lower()
    name = chain.get("name", "").lower().replace(" ", "-")
    return name


def canonical_rpc(chain: dict[str, Any]) -> str | None:
    rpcs = chain.get("rpc") or []
    if not rpcs:
        return None
    # pick first non-empty RPC
    for r in rpcs:
        if r and isinstance(r, str):
            return r
    return None


def main() -> None:
    print(f"Fetching {URL} ...")
    resp = requests.get(URL, timeout=30)
    resp.raise_for_status()
    data = resp.json()

    # filter for EVM-compatible and Solana
    selected = [c for c in data if is_evm(c) or is_solana(c)]

    # sort by chainId numeric if available else name
    def sort_key(c: dict[str, Any]):
        return (c.get("chainId") or 10**12, c.get("name"))

    selected.sort(key=sort_key)

    out = []
    for c in selected:
        rpc = canonical_rpc(c)
        if not rpc:
            continue
        item = {
            "chain_id": canonical_chain_id(c),
            "chain_name": c.get("name"),
            "rpc_url": rpc,
            "chain_numeric_id": c.get("chainId"),
            "is_evm": is_evm(c),
            "is_svm": is_solana(c),
            "supports_gpu": True,
        }
        out.append(item)

    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(json.dumps(out, indent=2))
    print(f"Wrote {len(out)} chains to {OUT}")


if __name__ == "__main__":
    main()
