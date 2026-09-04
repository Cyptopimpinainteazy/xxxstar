#!/usr/bin/env python3
"""
x3_router_test.py — Test script for the X3 Router

Verifies that the router correctly classifies prompts and routes
to the right specialist model.

Usage:
  python3 x3_router_test.py                          # Test classifier only
  python3 x3_router_test.py --live                    # Test against running router
  python3 x3_router_test.py --live --port 11436       # Custom port
"""

import json
import sys
import argparse
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from classifier import classify


def test_classifier():
    """Test the classifier without the router running."""
    test_cases = [
        # (prompt, expected_model_key)
        ("Audit this Solidity contract for reentrancy", "solidity_guard"),
        ("Fix the failing tests in the Rust pallet", "rust_runtime"),
        ("Design an arbitrage strategy for cross-chain DEX", "arb_king"),
        ("Build a flashloan executor for Aave V3", "flashloan_executor"),
        ("Score this route: Uniswap to Raydium via Wormhole", "route_oracle"),
        ("Review the mainnet readiness of X3 Chain", "auditor"),
        ("Write a Solana program for token swap", "svm_guard"),
        ("Set up Docker and systemd for the validator node", "devops_commander"),
        ("Kill all TODOs and fix broken imports", "cline_finisher"),
        ("Hello, how are you today?", "cryptomaster"),
        ("Implement the BTC to X3 bridge custody proof", "btc_guard"),
        ("Write Foundry tests for the vault contract", "solidity_guard"),
        ("Check MEV exposure on this trading route", "mev_defense"),
        ("Build an indexer for X3 events", "data_engineer"),
        ("Calculate the risk model for this arb strategy", "quant_risk"),
        ("Set up CI/CD pipeline for deployment", "devops_commander"),
        ("Write the grant application for X3", "compliance_ops"),
        ("Write a CosmWasm IBC transfer handler", "cosmwasm_guard"),
        ("Design the PnL dashboard for trading ops", "quant_risk"),
        ("Build a Taproot script for the bridge", "btc_guard"),
        ("Score model outputs for regression", "eval_judge"),
        ("Update the README and model card", "docsmith"),
        ("Design the X3 cross-VM architecture", "cryptomaster"),
        ("Audit the mainnet readiness checklist", "auditor"),
        # Edge cases
        ("How do I stake my tokens?", "cryptomaster"),  # generic → fallback
        ("Solidity reentrancy attack vector analysis", "solidity_guard"),
        ("Solana Anchor CPI safety check", "svm_guard"),
        ("Cross-chain arbitrage profit calculator", "arb_king"),
        ("Flashloan callback validation and repayment", "flashloan_executor"),
        ("MEV sandwich defense and private routing", "mev_defense"),
        ("BTC Taproot key path spend script", "btc_guard"),
        ("CosmWasm IBC channel handshake verification", "cosmwasm_guard"),
    ]

    print("X3 Router Classifier Tests")
    print("=" * 80)

    passed = 0
    failed = 0
    failures = []

    for prompt, expected in test_cases:
        key, model, reviewer, score = classify(prompt)
        status = "✓" if key == expected else "✗"
        if key == expected:
            passed += 1
        else:
            failed += 1
            failures.append((prompt, expected, key, score))
        print(f"  {status}  Score={score:3d}  Got={key:25s}  Expected={expected:25s}  Model={model}")

    print()
    print(f"Results: {passed}/{len(test_cases)} passed, {failed}/{len(test_cases)} failed")

    if failures:
        print()
        print("Failures:")
        for prompt, expected, got, score in failures:
            print(f"  ✗  '{prompt[:70]}' → Got {got}, Expected {expected} (score={score})")

    return failed == 0


def test_live_router(port: int):
    """Test against a running router instance."""
    from urllib.request import Request, urlopen

    print(f"X3 Router Live Tests (port {port})")
    print("=" * 80)

    test_cases = [
        ("Audit this Solidity contract", "lojak/x3-solidity-guard"),
        ("Fix the Rust pallet tests", "lojak/x3-rust-runtime"),
        ("Arbitrage strategy for DEX", "lojak/x3-arb-king"),
        ("Hello world", "lojak/cryptomaster"),
    ]

    passed = 0
    failed = 0

    for prompt, expected_model in test_cases:
        try:
            data = json.dumps({
                "model": "lojak/cryptomaster",
                "messages": [{"role": "user", "content": prompt}],
                "stream": False,
            }).encode()

            req = Request(
                f"http://localhost:{port}/v1/chat/completions",
                data=data,
                headers={"Content-Type": "application/json"},
            )

            with urlopen(req, timeout=30) as resp:
                routed = resp.headers.get("X-X3-Model-Routed-To", "unknown")
                status = "✓" if routed == expected_model else "✗"
                if routed == expected_model:
                    passed += 1
                else:
                    failed += 1
                print(f"  {status}  Prompt='{prompt[:50]}'  Routed={routed}  Expected={expected_model}")
        except Exception as e:
            print(f"  ✗  Error: {e}")
            failed += 1

    print(f"\nResults: {passed}/{len(test_cases)} passed, {failed}/{len(test_cases)} failed")
    return failed == 0


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Test X3 Router")
    parser.add_argument("--live", action="store_true", help="Test against a running router")
    parser.add_argument("--port", type=int, default=11435, help="Router port for live tests")
    args = parser.parse_args()

    if args.live:
        success = test_live_router(args.port)
    else:
        success = test_classifier()

    sys.exit(0 if success else 1)