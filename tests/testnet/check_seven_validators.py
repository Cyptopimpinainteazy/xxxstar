#!/usr/bin/env python3
"""Read-only X3 seven-validator RPC smoke and consensus checks."""
import argparse
import json
import sys
import time
import urllib.error
import urllib.request

def rpc(url, method, timeout):
    payload = json.dumps({"jsonrpc": "2.0", "id": 1, "method": method, "params": []}).encode()
    req = urllib.request.Request(url, data=payload, headers={"Content-Type": "application/json"})
    with urllib.request.urlopen(req, timeout=timeout) as response:
        result = json.load(response)
    if "error" in result or "result" not in result:
        raise RuntimeError(f"{method}: {result.get('error', 'missing result')}")
    return result["result"]

def snapshot(url, timeout):
    health = rpc(url, "system_health", timeout)
    head = rpc(url, "chain_getHeader", timeout)
    finalized_hash = rpc(url, "chain_getFinalizedHead", timeout)
    finalized_header = rpc(url, "chain_getHeader", timeout) if False else None
    # chain_getHeader without params reports the best head; finalized number is
    # resolved separately using chain_getBlockHash-independent header lookup.
    req = json.dumps({"jsonrpc": "2.0", "id": 1, "method": "chain_getHeader", "params": [finalized_hash]}).encode()
    with urllib.request.urlopen(urllib.request.Request(url, data=req, headers={"Content-Type": "application/json"}), timeout=timeout) as response:
        body = json.load(response)
    if "error" in body or not isinstance(body.get("result"), dict):
        raise RuntimeError("finalized header unavailable")
    return {
        "url": url,
        "peers": health.get("peers"),
        "isSyncing": health.get("isSyncing"),
        "best_number": int(head["number"], 16),
        "finalized_number": int(body["result"]["number"], 16),
        "finalized_hash": finalized_hash,
    }

def main():
    p = argparse.ArgumentParser()
    p.add_argument("--nodes", type=int, default=7)
    p.add_argument("--host", default="127.0.0.1")
    p.add_argument("--base-port", type=int, default=9950)
    p.add_argument("--timeout", type=float, default=3)
    p.add_argument("--settle-seconds", type=float, default=4)
    p.add_argument("--min-peers", type=int, default=1)
    p.add_argument("--output", default="")
    a = p.parse_args()
    if a.nodes < 2 or not 0 <= a.min_peers < a.nodes or a.settle_seconds < 0:
        p.error("invalid node count, peer threshold, or settle duration")
    urls = [f"http://{a.host}:{a.base_port + i}/" for i in range(a.nodes)]
    errors = []
    def sample(label):
        records = []
        for url in urls:
            try:
                records.append(snapshot(url, a.timeout))
            except (OSError, ValueError, KeyError, TypeError, RuntimeError) as exc:
                errors.append(f"{label} {url}: {exc}")
        return records
    before = sample("before")
    time.sleep(a.settle_seconds)
    after = sample("after")
    if len(before) != a.nodes or len(after) != a.nodes:
        errors.append("not all RPC endpoints responded")
    if len(after) == a.nodes:
        for node in after:
            if node["peers"] is None or node["peers"] < a.min_peers:
                errors.append(f"{node['url']}: insufficient peers")
            if node["isSyncing"]:
                errors.append(f"{node['url']}: syncing")
        # Different finalized heights are normal in a live network. Compare
        # hashes at the lowest finalized height rather than latest heads.
        common_height = min(n["finalized_number"] for n in after)
        hashes = []
        for node in after:
            try:
                req = json.dumps({"jsonrpc": "2.0", "id": 1, "method": "chain_getBlockHash", "params": [common_height]}).encode()
                with urllib.request.urlopen(urllib.request.Request(node["url"], data=req, headers={"Content-Type": "application/json"}), timeout=a.timeout) as response:
                    result = json.load(response)
                if not result.get("result") or "error" in result:
                    raise RuntimeError("missing common-height hash")
                hashes.append(result["result"])
            except (OSError, ValueError, RuntimeError) as exc:
                errors.append(f"{node['url']}: {exc}")
        if len(hashes) == a.nodes and len(set(hashes)) != 1:
            errors.append(f"finalized divergence at height {common_height}")
    if len(before) == a.nodes and len(after) == a.nodes:
        if max(n["finalized_number"] for n in after) <= max(n["finalized_number"] for n in before):
            errors.append("no finality progress during sampling window")
    report = {"passed": not errors, "before": before, "after": after, "errors": errors}
    rendered = json.dumps(report, indent=2)
    if a.output:
        with open(a.output, "w", encoding="utf-8") as f:
            f.write(rendered + "\n")
    print(rendered)
    return 0 if not errors else 1

if __name__ == "__main__":
    sys.exit(main())
