#!/usr/bin/env python3
import json
import sys

try:
    data = json.load(sys.stdin)
except Exception:
    sys.exit(0)

prompt = data.get("prompt", "").lower()

terms = [
    "x3", "x3-lang", "x3vm", "evm", "svm", "btc", "utxo",
    "atomic", "router", "kernel", "adapter", "rollback",
    "settlement", "mainnet", "validator", "runtime", "pallet"
]

if any(term in prompt for term in terms):
    print(json.dumps({
        "additionalContext": """
X3 prompt context:
- Inspect actual code before planning.
- Mainnet-ready means build, lint, tests, coverage, and docs.
- Atomic cross-VM behavior must preserve rollback, replay protection, expiry checks, adapter compatibility, and canonical asset invariants.
- Do not mark a feature complete until tests prove it.
"""
    }))
