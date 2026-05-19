#!/usr/bin/env python3
"""
Cross-VM test harness (RPC smoke + deterministic simulation).

Phase-0 harness to validate:
  - Node RPC responds
  - Atomic trade engine RPCs are wired
  - X3 asset metadata resolves
"""
from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
import urllib.request


def rpc_call(url: str, method: str, params: list) -> dict:
    payload = json.dumps({
        "jsonrpc": "2.0",
        "id": 1,
        "method": method,
        "params": params,
    }).encode("utf-8")
    req = urllib.request.Request(url, data=payload, headers={"Content-Type": "application/json"})
    with urllib.request.urlopen(req, timeout=2) as resp:
        body = resp.read().decode("utf-8")
    return json.loads(body)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--rpc-url", default="http://127.0.0.1:9944")
    parser.add_argument("--ws-url", default="ws://127.0.0.1:9944")
    parser.add_argument("--suri", default="//Alice")
    parser.add_argument("--remark", default=None)
    parser.add_argument("--submit-remark", action="store_true")
    parser.add_argument("--no-submit", action="store_true")
    args = parser.parse_args()

    url = args.rpc_url

    print(f"RPC: {url}")

    health = rpc_call(url, "system_health", [])
    print("system_health:", health)

    asset_meta = rpc_call(url, "atlasKernel_getAssetMetadata", [1000, None])
    print("atlasKernel_getAssetMetadata:", asset_meta)

    h256_zero = "0x" + ("00" * 32)
    simulate = rpc_call(
        url,
        "atomicTrade_simulate",
        [h256_zero, h256_zero, 1_000_000_000_000, 50, None],
    )
    print("atomicTrade_simulate:", simulate)

    estimate = rpc_call(
        url,
        "atomicTrade_estimateCost",
        [2, [0, 1], None],
    )
    print("atomicTrade_estimateCost:", estimate)

    price = rpc_call(
        url,
        "atomicTrade_getPriceData",
        [h256_zero, h256_zero, None],
    )
    print("atomicTrade_getPriceData:", price)

    if args.submit_remark and not args.no_submit:
        remark = args.remark or f"x3-harness-{int(__import__('time').time())}"
        env = os.environ.copy()
        env.setdefault("NODE_PATH", "apps/wallet/node_modules")
        env["RPC_WS"] = args.ws_url
        env["SURI"] = args.suri
        env["REMARK"] = remark

        print("submit_remark:", remark)
        subprocess.run(["node", "scripts/testnet/submit-remark.js"], check=True, env=env)

    return 0


if __name__ == "__main__":
    sys.exit(main())
