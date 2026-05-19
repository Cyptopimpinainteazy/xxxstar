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
    req = urllib.request.Request(
        url,
        data=payload,
        headers={"Content-Type": "application/json"},
    )
    with urllib.request.urlopen(req, timeout=2) as resp:
        body = resp.read().decode("utf-8")
    return json.loads(body)


def pick_method(methods: set[str], candidates: list[str]) -> str | None:
    for method in candidates:
        if method in methods:
            return method
    return None


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

    method_result = rpc_call(url, "rpc_methods", [])
    available = set(method_result.get("result", {}).get("methods", []))
    print(f"rpc_methods: discovered {len(available)} methods")

    asset_method = pick_method(
        available,
        ["atlasKernel_getAssetMetadata", "x3_getAssetMetadata"],
    )
    if asset_method is None:
        print(
            "asset metadata method: not found "
            "(atlasKernel_getAssetMetadata/x3_getAssetMetadata)"
        )
    else:
        asset_meta = rpc_call(url, asset_method, [1000])
        print(f"{asset_method}:", asset_meta)

    h256_zero = "0x" + ("00" * 32)
    simulate_method = pick_method(
        available,
        ["atomicTrade_simulate", "atomicTrade_getSwapQuote"],
    )
    if simulate_method is None:
        print(
            "simulation method: not found "
            "(atomicTrade_simulate/atomicTrade_getSwapQuote)"
        )
    else:
        simulate_params = {
            "atomicTrade_simulate": [
                h256_zero,
                h256_zero,
                1_000_000_000_000,
                50,
                None,
            ],
            # Current RPC expects a single JSON object.
            "atomicTrade_getSwapQuote": [
                {
                    "token_in": "X3",
                    "token_out": "USDC",
                    "amount_in": str(1_000_000_000_000),
                }
            ],
        }
        simulate = rpc_call(url, simulate_method, simulate_params[simulate_method])
        print(f"{simulate_method}:", simulate)

    estimate_method = pick_method(
        available,
        ["atomicTrade_estimateCost", "atomicTrade_estimateSlippage"],
    )
    if estimate_method is None:
        print(
            "estimate method: not found "
            "(atomicTrade_estimateCost/atomicTrade_estimateSlippage)"
        )
    else:
        estimate_params = {
            "atomicTrade_estimateCost": [2, [0, 1], None],
            # Current RPC expects a single JSON object.
            "atomicTrade_estimateSlippage": [
                {
                    "token_in": "X3",
                    "token_out": "USDC",
                    "amount_in": str(1_000_000_000_000),
                }
            ],
        }
        estimate = rpc_call(url, estimate_method, estimate_params[estimate_method])
        print(f"{estimate_method}:", estimate)

    price_method = pick_method(
        available,
        ["atomicTrade_getPriceData", "atomicTrade_getSwapStatus"],
    )
    if price_method is None:
        print(
            "price/status method: not found "
            "(atomicTrade_getPriceData/atomicTrade_getSwapStatus)"
        )
    else:
        price_params = {
            "atomicTrade_getPriceData": [h256_zero, h256_zero, None],
            # Use a dummy swap id to validate method wiring.
            "atomicTrade_getSwapStatus": ["0x" + ("00" * 32)],
        }
        price = rpc_call(url, price_method, price_params[price_method])
        print(f"{price_method}:", price)

    cross_vm_method = pick_method(available, ["x3_submitCrossVmTransaction"])
    if cross_vm_method is None:
        print(
            "cross-VM submission method: not found "
            "(x3_submitCrossVmTransaction)"
        )
    else:
        cross_vm_probe = rpc_call(
            url,
            cross_vm_method,
            [{"evm_payload": "0x01", "svm_payload": "0x02", "atomic": True}],
        )
        print(f"{cross_vm_method}:", cross_vm_probe)

    if args.submit_remark and not args.no_submit:
        remark = args.remark or f"x3-harness-{int(__import__('time').time())}"
        env = os.environ.copy()
        env.setdefault("NODE_PATH", "apps/wallet/node_modules")
        env["RPC_WS"] = args.ws_url
        env["SURI"] = args.suri
        env["REMARK"] = remark

        print("submit_remark:", remark)
        subprocess.run(
            ["node", "scripts/testnet/submit-remark.js"],
            check=True,
            env=env,
        )

    return 0


if __name__ == "__main__":
    sys.exit(main())
