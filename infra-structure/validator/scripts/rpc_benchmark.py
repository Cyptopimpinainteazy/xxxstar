"""RPC health and latency benchmark across configured chains.

Reads `src/resources/chains.json` and for each chain attempts a single lightweight
JSON-RPC probe appropriate to the chain type (EVM -> eth_blockNumber, Solana -> getSlot).
Records success, status code, latency, and error if any, writes `benchmark_rpc_report.json`.
"""
from __future__ import annotations

import json
import re
import time
from pathlib import Path
from typing import Any

import requests

ROOT = Path(__file__).resolve().parents[1]
CHAINS = ROOT / "src" / "resources" / "chains.json"
OUT = ROOT / "benchmarks" / "rpc_benchmark_report.json"
OUT.parent.mkdir(exist_ok=True)

TEMPLATE_RE = re.compile(r"\$\{.+?\}|")


def is_templated(url: str) -> bool:
    return bool(re.search(r"\$\{.+?\}", url))


def probe_evm(url: str) -> dict[str, Any]:
    payload = {"jsonrpc": "2.0", "method": "eth_blockNumber", "params": [], "id": 1}
    headers = {"Content-Type": "application/json"}
    start = time.perf_counter()
    try:
        r = requests.post(url, json=payload, headers=headers, timeout=6)
        elapsed = time.perf_counter() - start
        ok = r.status_code == 200 and ("result" in r.json() if r.content else False)
        return {"ok": ok, "status_code": r.status_code, "latency_seconds": elapsed}
    except Exception as e:
        return {"ok": False, "error": str(e)}


def probe_solana(url: str) -> dict[str, Any]:
    payload = {"jsonrpc": "2.0", "id": 1, "method": "getSlot", "params": []}
    headers = {"Content-Type": "application/json"}
    start = time.perf_counter()
    try:
        r = requests.post(url, json=payload, headers=headers, timeout=6)
        elapsed = time.perf_counter() - start
        ok = r.status_code == 200 and ("result" in r.json() if r.content else False)
        return {"ok": ok, "status_code": r.status_code, "latency_seconds": elapsed}
    except Exception as e:
        return {"ok": False, "error": str(e)}


def main() -> None:
    chains = json.loads(CHAINS.read_text())
    report: dict[str, Any] = {"summary": {}, "results": []}
    total = len(chains)
    ok_count = 0
    skipped = 0
    for idx, item in enumerate(chains, 1):
        cid = item.get("chain_id")
        rpc = item.get("rpc_url")
        is_evm = item.get("is_evm")
        is_svm = item.get("is_svm")
        entry: dict[str, Any] = {"chain_id": cid, "rpc_url": rpc}
        if not rpc:
            entry["skipped"] = "no_rpc"
            skipped += 1
            report["results"].append(entry)
            continue
        if is_templated(rpc):
            entry["skipped"] = "templated_rpc"
            skipped += 1
            report["results"].append(entry)
            continue
        if is_evm:
            res = probe_evm(rpc)
        elif is_svm:
            res = probe_solana(rpc)
        else:
            # default to HTTP GET health check
            start = time.perf_counter()
            try:
                r = requests.get(rpc, timeout=5)
                elapsed = time.perf_counter() - start
                res = {"ok": r.status_code == 200, "status_code": r.status_code, "latency_seconds": elapsed}
            except Exception as e:
                res = {"ok": False, "error": str(e)}
        entry.update(res)
        if res.get("ok"):
            ok_count += 1
        report["results"].append(entry)
        # polite pause
        time.sleep(0.05)

    report["summary"] = {"total": total, "ok": ok_count, "skipped": skipped}
    OUT.write_text(json.dumps(report, indent=2))
    print(f"Wrote RPC benchmark report to {OUT}")


if __name__ == "__main__":
    main()
