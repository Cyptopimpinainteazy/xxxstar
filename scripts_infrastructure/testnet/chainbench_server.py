#!/usr/bin/env python3
"""
Chainbench API + static host for X3 Chain local testnet operations.

Serves:
  - chainbench-ultimate(1).html
  - API endpoints for mode control, onboarding, rpc sweep/bench, harness hooks, and network status.
"""
from __future__ import annotations

import argparse
import concurrent.futures
import contextlib
import json
import os
import random
import re
import subprocess
import sys
import threading
import time
import urllib.parse
import urllib.request
from http import HTTPStatus
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from typing import Any

REPO_ROOT = Path(__file__).resolve().parents[2]
DASHBOARD_FILE = REPO_ROOT / "chainbench-ultimate(1).html"
STATE_DIR_DEFAULT = Path.home() / ".local/share/x3/testnet-local/chainbench"


def rpc_call(url: str, method: str, params: list[Any]) -> dict[str, Any]:
    payload = json.dumps(
        {
            "jsonrpc": "2.0",
            "id": 1,
            "method": method,
            "params": params,
        }
    ).encode("utf-8")
    req = urllib.request.Request(
        url,
        data=payload,
        headers={"Content-Type": "application/json"},
    )
    with urllib.request.urlopen(req, timeout=2.5) as resp:
        body = resp.read().decode("utf-8")
    return json.loads(body)


def timed_rpc_call(url: str, method: str, params: list[Any], timeout_s: float = 2.5) -> dict[str, Any]:
    started = time.time()
    try:
        payload = json.dumps(
            {
                "jsonrpc": "2.0",
                "id": 1,
                "method": method,
                "params": params,
            }
        ).encode("utf-8")
        req = urllib.request.Request(
            url,
            data=payload,
            headers={"Content-Type": "application/json"},
        )
        with urllib.request.urlopen(req, timeout=timeout_s) as resp:
            body = json.loads(resp.read().decode("utf-8"))
        return {
            "ok": "result" in body and "error" not in body,
            "latency_ms": int((time.time() - started) * 1000),
            "result": body.get("result"),
            "error": body.get("error"),
        }
    except Exception as exc:
        return {
            "ok": False,
            "latency_ms": int((time.time() - started) * 1000),
            "error": str(exc),
        }


def http_get_json(url: str, timeout_s: float = 2.5) -> dict[str, Any]:
    req = urllib.request.Request(url, headers={"Accept": "application/json"})
    with urllib.request.urlopen(req, timeout=timeout_s) as resp:
        body = resp.read().decode("utf-8")
    return json.loads(body)


def http_post_json(url: str, payload: dict[str, Any], timeout_s: float = 3.0, headers: dict[str, str] | None = None) -> dict[str, Any]:
    data = json.dumps(payload).encode("utf-8")
    hdrs = {"Content-Type": "application/json"}
    if headers:
        hdrs.update(headers)
    req = urllib.request.Request(url, data=data, headers=hdrs)
    with urllib.request.urlopen(req, timeout=timeout_s) as resp:
        body = resp.read().decode("utf-8")
    return json.loads(body)


class ChainbenchState:
    def __init__(
        self,
        state_dir: Path,
        testnet_rpc: str,
        live_rpc: str,
        admin_key: str,
        nodecore_query_url: str,
        dshackle_proxy_url: str,
        chain_db_url: str,
        tps_url: str,
        default_chain_id: str,
        chain_db_admin_key: str,
    ) -> None:
        self.state_dir = state_dir
        self.state_dir.mkdir(parents=True, exist_ok=True)
        self.state_file = self.state_dir / "state.json"
        self.chains_file = self.state_dir / "onboarded_chains.json"
        self.bench_file = self.state_dir / "rpc_bench_last.json"
        self.mode_map = {
            "testnet": {
                "name": "Testnet",
                "rpc": testnet_rpc,
                "hint": "Local X3 Chain testnet RPC",
            },
            "live": {
                "name": "Live Net",
                "rpc": live_rpc,
                "hint": "Public high-TPS RPC",
            },
        }
        self.admin_key = admin_key
        self.nodecore_query_url = nodecore_query_url
        self.dshackle_proxy_url = dshackle_proxy_url
        self.chain_db_url = chain_db_url.rstrip("/")
        self.tps_url = tps_url.rstrip("/")
        self.default_chain_id = default_chain_id
        self.chain_db_admin_key = chain_db_admin_key
        self.evm_rpc_override = os.getenv("CHAINBENCH_EVM_RPC", "").strip()
        self.svm_rpc_override = os.getenv("CHAINBENCH_SVM_RPC", "").strip()
        self.btc_rpc_override = os.getenv("CHAINBENCH_BTC_RPC", "").strip()
        self.btc_mempool_api = os.getenv("CHAINBENCH_BTC_MEMPOOL_API", "https://mempool.space/api").rstrip("/")
        self.liquidity_pair = os.getenv("CHAINBENCH_LIQ_PAIR", "0xB4e16d0168e52d35CaCD2c6185b44281Ec28C9Dc")
        self.liq_token0_symbol = os.getenv("CHAINBENCH_LIQ_TOKEN0", "USDC")
        self.liq_token1_symbol = os.getenv("CHAINBENCH_LIQ_TOKEN1", "WETH")
        self.liq_token0_dec = int(os.getenv("CHAINBENCH_LIQ_TOKEN0_DEC", "6"))
        self.liq_token1_dec = int(os.getenv("CHAINBENCH_LIQ_TOKEN1_DEC", "18"))
        self.fuzzer_target = os.getenv("CHAINBENCH_FUZZER_TARGET", "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2")
        self.connectors_health_file = self.state_dir / "connectors_health.json"
        self._connectors_health_cache = self._load_json(self.connectors_health_file, {"items": [], "updated_at": 0})
        self._connectors_lock = threading.Lock()
        self.state = self._load_json(self.state_file, {"mode": "testnet", "admin_enabled": False})

    @staticmethod
    def _load_json(path: Path, fallback: Any) -> Any:
        if not path.exists():
            return fallback
        try:
            return json.loads(path.read_text())
        except Exception:
            return fallback

    def _save_json(self, path: Path, data: Any) -> None:
        path.write_text(json.dumps(data, indent=2, sort_keys=True))

    def save_state(self) -> None:
        self._save_json(self.state_file, self.state)

    def _pick_rpc_from_chain_db(self, chain_id: str) -> str | None:
        if not self.chain_db_url:
            return None
        try:
            res = self.chain_db_get(f"/api/rpc/{chain_id}")
        except Exception:
            return None
        if not isinstance(res, dict):
            return None
        endpoints = res.get("endpoints", []) or []
        healthy = [e for e in endpoints if e.get("is_healthy") == 1 or e.get("is_healthy") is True]
        if healthy:
            healthy.sort(key=lambda e: e.get("latency_ms") or 999999)
            return healthy[0].get("url")
        if endpoints:
            return endpoints[0].get("url")
        return None

    def _pick_rpc_from_connectors(self, ecosystem: str) -> str | None:
        if not self.chain_db_url:
            return None
        eco = ecosystem.lower()
        try:
            health = self.connectors_health()
            items = health.get("items", []) if isinstance(health, dict) else []
            ok_items = [row for row in items if str(row.get("ecosystem", "")).lower() == eco and row.get("ok") and row.get("rpc")]
            if ok_items:
                ok_items.sort(key=lambda r: r.get("latency_ms") or 999999)
                return ok_items[0].get("rpc")
        except Exception:
            pass
        try:
            res = self.chain_db_get("/api/connectors")
        except Exception:
            return None
        if not isinstance(res, dict):
            return None
        items = res.get("items", []) or []
        for row in items:
            if str(row.get("ecosystem", "")).lower() == eco:
                return row.get("rpc_url") or row.get("rpc")
        return None

    def _pick_evm_rpc(self) -> str | None:
        if self.evm_rpc_override:
            return self.evm_rpc_override
        return (
            self._pick_rpc_from_connectors("evm")
            or self._pick_rpc_from_chain_db("eth")
            or self._pick_rpc_from_chain_db("ethereum")
            or self.dshackle_proxy_url
            or self.nodecore_query_url
        )

    def _pick_svm_rpc(self) -> str | None:
        if self.svm_rpc_override:
            return self.svm_rpc_override
        return (
            self._pick_rpc_from_connectors("svm")
            or self._pick_rpc_from_chain_db("sol")
            or self._pick_rpc_from_chain_db("solana")
            or "https://api.mainnet-beta.solana.com"
        )

    def _pick_btc_rpc(self) -> str | None:
        if self.btc_rpc_override:
            return self.btc_rpc_override
        return (
            self._pick_rpc_from_connectors("btc")
            or self._pick_rpc_from_chain_db("btc")
            or self._pick_rpc_from_chain_db("bitcoin")
        )

    def chain_db_get(self, path: str) -> dict[str, Any] | None:
        if not self.chain_db_url:
            return None
        try:
            return http_get_json(f"{self.chain_db_url}{path}")
        except Exception:
            return None

    def chain_db_post(self, path: str, payload: dict[str, Any]) -> dict[str, Any] | None:
        if not self.chain_db_url:
            return None
        headers = {}
        if self.chain_db_admin_key:
            headers["X-Admin-Key"] = self.chain_db_admin_key
        try:
            return http_post_json(f"{self.chain_db_url}{path}", payload, headers=headers)
        except Exception:
            return None

    def tps_get(self, path: str) -> dict[str, Any] | None:
        if not self.tps_url:
            return None
        try:
            return http_get_json(f"{self.tps_url}{path}")
        except Exception:
            return None

    def resolve_chain_id(self, chain_label: str) -> str:
        label = chain_label.strip()
        if not label:
            return self.default_chain_id
        # slugify for fallback
        slug = re.sub(r"[^a-zA-Z0-9-]+", "-", label.lower()).strip("-")
        if not self.chain_db_url:
            return slug or self.default_chain_id
        try:
            res = self.chain_db_get(f"/api/chains/search?q={urllib.parse.quote(label)}")
            items = res.get("results") or res.get("chains") or []
            if items:
                return str(items[0].get("chain_id") or slug or self.default_chain_id)
        except Exception:
            pass
        return slug or self.default_chain_id

    def get_mode_payload(self) -> dict[str, Any]:
        mode = self.state.get("mode", "testnet")
        return {
            "mode": mode,
            "config": self.mode_map[mode],
            "modes": self.mode_map,
            "admin_enabled": bool(self.state.get("admin_enabled", False)),
        }

    def set_mode(self, mode: str) -> dict[str, Any]:
        if mode not in self.mode_map:
            raise ValueError(f"Unsupported mode: {mode}")
        self.state["mode"] = mode
        self.save_state()
        return self.get_mode_payload()

    def set_admin_enabled(self, enabled: bool) -> dict[str, Any]:
        self.state["admin_enabled"] = enabled
        self.save_state()
        return {
            "admin_enabled": enabled,
            "updated_at": int(time.time()),
        }

    def list_onboarded(self) -> list[dict[str, Any]]:
        if self.chain_db_url:
            res = self.chain_db_get("/api/connectors")
            if isinstance(res, dict) and isinstance(res.get("items"), list):
                out = []
                for rec in res["items"]:
                    ts = int(time.time())
                    created_at = rec.get("created_at")
                    if created_at:
                        try:
                            ts = int(time.mktime(time.strptime(created_at, "%Y-%m-%d %H:%M:%S")))
                        except Exception:
                            ts = int(time.time())
                    out.append(
                        {
                            "id": rec.get("id"),
                            "chain_id": rec.get("chain_id"),
                            "chain": rec.get("chain_name") or rec.get("chain_id"),
                            "rpc": rec.get("rpc_url"),
                            "rpc_url": rec.get("rpc_url"),
                            "credential_mask": rec.get("credential_mask"),
                            "notes": rec.get("notes"),
                            "status": rec.get("status"),
                            "ecosystem": rec.get("ecosystem"),
                            "registered_at": ts,
                        }
                    )
                return out
        data = self._load_json(self.chains_file, [])
        if not isinstance(data, list):
            return []
        return data

    def add_onboarded(self, payload: dict[str, Any]) -> dict[str, Any]:
        chain_label = str(payload.get("chain", "")).strip()
        rpc = str(payload.get("rpc", "")).strip()
        cred = str(payload.get("cred", "")).strip()
        notes = str(payload.get("notes", "")).strip()
        if not chain_label or not rpc:
            raise ValueError("chain and rpc are required")

        chain_id = self.resolve_chain_id(chain_label)
        if self.chain_db_url:
            auth_type = "api_key" if cred else "none"
            res = self.chain_db_post(
                "/api/connectors",
                {
                    "chain_id": chain_id,
                    "rpc_url": rpc,
                    "auth_type": auth_type,
                    "credential": cred,
                    "notes": notes,
                },
            )
            if isinstance(res, dict) and res.get("ok"):
                connector = res.get("connector") or {}
                return {
                    "id": connector.get("id"),
                    "chain_id": connector.get("chain_id"),
                    "chain": chain_label,
                    "rpc": connector.get("rpc_url") or rpc,
                    "rpc_url": connector.get("rpc_url") or rpc,
                    "credential_mask": connector.get("credential_mask"),
                    "notes": connector.get("notes") or notes,
                    "status": connector.get("status") or "active",
                    "registered_at": int(time.time()),
                }

        masked = cred[:4] + "..." + cred[-2:] if len(cred) > 8 else ("****" if cred else "")
        entry = {
            "id": f"cb-{int(time.time() * 1000)}-{random.randint(1000, 9999)}",
            "chain_id": chain_id,
            "chain": chain_label,
            "rpc_url": rpc,
            "credential_mask": masked,
            "notes": notes,
            "registered_at": int(time.time()),
            "status": "active",
        }

        existing = self._load_json(self.chains_file, [])
        existing.insert(0, entry)
        self._save_json(self.chains_file, existing)
        return entry

    def rpc_sweep(self, chain_id: str | None = None, rpc_override: str | None = None) -> dict[str, Any]:
        mode = self.state.get("mode", "testnet")
        cfg = self.mode_map[mode]
        rpc = rpc_override or cfg["rpc"]
        if chain_id and self.chain_db_url:
            res = self.chain_db_get(f"/api/rpc/{chain_id}")
            if isinstance(res, dict):
                endpoints = [r.get("url") for r in res.get("endpoints", []) if r.get("url")]
                if endpoints:
                    rpc = endpoints[0]
        methods = [
            ("system_health", []),
            ("system_name", []),
            ("chain_getHeader", []),
            ("chain_getBlockHash", []),
            ("atlasKernel_getAssetMetadata", [1000, None]),
            (
                "atomicTrade_simulate",
                [
                    "0x" + ("00" * 32),
                    "0x" + ("00" * 32),
                    1_000_000_000_000,
                    50,
                    None,
                ],
            ),
        ]

        start = time.time()
        out: dict[str, Any] = {"mode": mode, "rpc": rpc, "results": {}, "ok": True}
        for method, params in methods:
            try:
                out["results"][method] = rpc_call(rpc, method, params)
            except Exception as exc:
                out["ok"] = False
                out["results"][method] = {"error": str(exc)}

        elapsed_ms = int((time.time() - start) * 1000)
        header = out["results"].get("chain_getHeader", {}).get("result", {})
        health = out["results"].get("system_health", {}).get("result", {})
        out["summary"] = {
            "latency_ms": elapsed_ms,
            "block": header.get("number", "0x0"),
            "peers": health.get("peers", 0),
            "syncing": health.get("isSyncing", True),
        }
        return out

    def rpc_bench(
        self,
        endpoints: list[str] | None = None,
        methods: list[str] | None = None,
        chain_id: str | None = None,
    ) -> dict[str, Any]:
        bench_methods = methods or ["system_health", "chain_getHeader", "state_getRuntimeVersion"]
        default_eps = []
        if self.chain_db_url:
            cid = chain_id or self.default_chain_id
            res = self.chain_db_get(f"/api/rpc/{cid}")
            if isinstance(res, dict):
                default_eps = [r.get("url") for r in res.get("endpoints", []) if r.get("url")]

        if not default_eps:
            default_eps = [
                self.mode_map["testnet"]["rpc"],
                "http://127.0.0.1:9945",
                "http://127.0.0.1:9946",
                "http://127.0.0.1:9947",
                "http://127.0.0.1:9948",
                "http://127.0.0.1:9949",
                "http://127.0.0.1:9950",
            ]
        deduped = []
        for ep in (endpoints or default_eps):
            if ep and ep not in deduped:
                deduped.append(ep)

        rows = []
        for ep in deduped:
            method_latencies = {}
            success = 0
            for method in bench_methods:
                started = time.time()
                try:
                    rpc_call(ep, method, [])
                    ms = int((time.time() - started) * 1000)
                    method_latencies[method] = ms
                    success += 1
                except Exception:
                    method_latencies[method] = None

            vals = [v for v in method_latencies.values() if isinstance(v, int)]
            avg_ms = int(sum(vals) / len(vals)) if vals else 9999
            reliability = round((success / len(bench_methods)) * 100, 2)
            score = max(0, int((10000 / max(avg_ms, 1)) + reliability * 10))
            rows.append(
                {
                    "endpoint": ep,
                    "latencies": method_latencies,
                    "avg_latency_ms": avg_ms,
                    "reliability": reliability,
                    "score": score,
                }
            )

        rows.sort(key=lambda x: (x["avg_latency_ms"], -x["reliability"]))
        fastest = rows[0] if rows else None
        slowest = rows[-1] if rows else None
        bench = {
            "generated_at": int(time.time()),
            "rows": rows,
            "summary": {
                "nodes_tested": len(rows),
                "fastest": fastest,
                "slowest": slowest,
                "avg_reliability": round(sum(r["reliability"] for r in rows) / len(rows), 2) if rows else 0,
                "recommendation": fastest["endpoint"] if fastest else None,
            },
            "methods": bench_methods,
        }
        self._save_json(self.bench_file, bench)
        return bench

    def rpc_bench_last(self) -> dict[str, Any]:
        return self._load_json(self.bench_file, {"rows": [], "summary": {}})

    def rpc_default_endpoints(self, chain_id: str | None = None) -> list[str]:
        if self.chain_db_url:
            cid = chain_id or self.default_chain_id
            res = self.chain_db_get(f"/api/rpc/{cid}")
            if isinstance(res, dict):
                endpoints = [r.get("url") for r in res.get("endpoints", []) if r.get("url")]
                if endpoints:
                    return endpoints
        return [
            self.mode_map["testnet"]["rpc"],
            "http://127.0.0.1:9945",
            "http://127.0.0.1:9946",
            "http://127.0.0.1:9947",
            "http://127.0.0.1:9948",
            "http://127.0.0.1:9949",
            "http://127.0.0.1:9950",
        ]

    def _probe_connector(self, url: str, ecosystem: str | None = None) -> dict[str, Any]:
        eco = (ecosystem or "").lower()
        if eco in {"svm", "solana"}:
            return timed_rpc_call(url, "getHealth", [])
        if eco in {"substrate", "polkadot"}:
            return timed_rpc_call(url, "system_health", [])
        if eco in {"evm", "ethereum"}:
            return timed_rpc_call(url, "eth_blockNumber", [])

        # Best-effort fallback (try JSON-RPC then plain HTTP GET)
        res = timed_rpc_call(url, "web3_clientVersion", [])
        if res.get("ok"):
            return res
        try:
            started = time.time()
            _ = http_get_json(url)
            return {"ok": True, "latency_ms": int((time.time() - started) * 1000), "result": "http_ok"}
        except Exception as exc:
            return {"ok": False, "latency_ms": int((time.time() - started) * 1000), "error": str(exc)}

    def refresh_connectors_health(self) -> dict[str, Any]:
        items = self.list_onboarded()
        results: list[dict[str, Any]] = []
        updated_at = int(time.time())

        def run_probe(item: dict[str, Any]) -> dict[str, Any]:
            url = str(item.get("rpc") or item.get("rpc_url") or "").strip()
            eco = str(item.get("ecosystem") or "").lower()
            if not url:
                return {
                    "chain_id": item.get("chain_id"),
                    "chain": item.get("chain"),
                    "rpc": url,
                    "ok": False,
                    "latency_ms": None,
                    "error": "missing rpc url",
                    "updated_at": updated_at,
                }
            out = self._probe_connector(url, eco)
            return {
                "chain_id": item.get("chain_id"),
                "chain": item.get("chain"),
                "rpc": url,
                "ok": bool(out.get("ok")),
                "latency_ms": out.get("latency_ms"),
                "error": out.get("error"),
                "updated_at": updated_at,
            }

        with concurrent.futures.ThreadPoolExecutor(max_workers=12) as ex:
            for res in ex.map(run_probe, items):
                results.append(res)

        payload = {"items": results, "updated_at": updated_at}
        with self._connectors_lock:
            self._connectors_health_cache = payload
            self._save_json(self.connectors_health_file, payload)
        return payload

    def connectors_health(self, max_age_s: int = 60) -> dict[str, Any]:
        with self._connectors_lock:
            cached = self._connectors_health_cache
        if not cached or int(time.time()) - int(cached.get("updated_at", 0)) > max_age_s:
            return self.refresh_connectors_health()
        return cached

    def network_status(self, count: int, base_port: int) -> dict[str, Any]:
        nodes = []
        for i in range(count):
            port = base_port + i
            url = f"http://127.0.0.1:{port}"
            node = {"node": i + 1, "rpc_port": port, "up": False}
            try:
                health = rpc_call(url, "system_health", [])
                header = rpc_call(url, "chain_getHeader", [])
                block_hash = rpc_call(url, "chain_getBlockHash", [])
                node.update(
                    {
                        "up": True,
                        "health": health.get("result", {}),
                        "header": header.get("result", {}),
                        "block_hash": block_hash.get("result"),
                    }
                )
            except Exception as exc:
                node["error"] = str(exc)
            nodes.append(node)

        up = sum(1 for n in nodes if n["up"])
        best_block = 0
        for n in nodes:
            number = n.get("header", {}).get("number")
            if isinstance(number, str) and number.startswith("0x"):
                with contextlib.suppress(Exception):
                    best_block = max(best_block, int(number, 16))

        finalized_block = None
        for n in nodes:
            if not n.get("up"):
                continue
            try:
                url = f"http://127.0.0.1:{n['rpc_port']}"
                fin_hash = rpc_call(url, "chain_getFinalizedHead", [])
                fin_header = rpc_call(url, "chain_getHeader", [fin_hash.get("result")])
                fin_num = fin_header.get("result", {}).get("number")
                if isinstance(fin_num, str) and fin_num.startswith("0x"):
                    finalized_block = int(fin_num, 16)
                break
            except Exception:
                continue

        forks = 0
        hash_set = set()
        for n in nodes:
            num = n.get("header", {}).get("number")
            if isinstance(num, str) and num.startswith("0x"):
                try:
                    num_i = int(num, 16)
                except Exception:
                    continue
                if num_i == best_block and n.get("block_hash"):
                    hash_set.add(n.get("block_hash"))
        if len(hash_set) > 1:
            forks = len(hash_set) - 1

        finality_delay = None
        if best_block and finalized_block:
            finality_delay = max(0, best_block - finalized_block)

        orphan_blocks = forks
        up_ratio = up / count if count else 0
        score = int(max(0, min(100, (up_ratio * 100) - (forks * 10) - (finality_delay or 0))))

        out = {
            "count": count,
            "up": up,
            "down": count - up,
            "best_block": best_block,
            "finalized_block": finalized_block,
            "forks": forks,
            "orphan_blocks": orphan_blocks,
            "finality_delay": finality_delay,
            "score": score,
            "nodes": nodes,
            "updated_at": int(time.time()),
        }
        out["infra"] = self.infra_status()
        return out

    def adapter_status(self) -> dict[str, Any]:
        live = self.state.get("mode", "testnet") == "live"
        connectors = self.list_onboarded()
        evm = 0
        svm = 0
        btc = 0
        for c in connectors:
            eco = str(c.get("ecosystem") or "").lower()
            chain_id = str(c.get("chain_id") or c.get("chain") or "").lower()
            if eco == "evm" or chain_id in {"eth", "ethereum", "bsc", "polygon", "arb-one", "op"}:
                evm += 1
            elif eco == "svm" or chain_id in {"sol", "solana"}:
                svm += 1
            elif chain_id in {"btc", "bitcoin"}:
                btc += 1
        return {
            "mode": self.state.get("mode", "testnet"),
            "adapters": {
                "evm": {
                    "enabled": True,
                    "state": "online" if evm > 0 and live else ("ready" if evm > 0 else "offline"),
                    "note": f"{evm} connector(s) registered",
                },
                "svm": {
                    "enabled": True,
                    "state": "online" if svm > 0 and live else ("ready" if svm > 0 else "offline"),
                    "note": f"{svm} connector(s) registered",
                },
                "btc": {
                    "enabled": True,
                    "state": "online" if btc > 0 and live else ("ready" if btc > 0 else "offline"),
                    "note": f"{btc} connector(s) registered",
                },
            },
        }

    def infra_status(self) -> dict[str, Any]:
        out: dict[str, Any] = {"chain_db": {"ok": False}, "blockchain_tps": {"ok": False}}

        if self.chain_db_url:
            try:
                started = time.time()
                health = self.chain_db_get("/health")
                stats = self.chain_db_get("/api/rpc/stats") or {}
                out["chain_db"] = {
                    "ok": bool(health),
                    "health": health,
                    "rpc_stats": stats,
                    "latency_ms": int((time.time() - started) * 1000),
                }
            except Exception as exc:
                out["chain_db"] = {"ok": False, "error": str(exc)}

        if self.tps_url:
            try:
                started = time.time()
                health = self.tps_get("/health")
                out["blockchain_tps"] = {
                    "ok": bool(health),
                    "health": health,
                    "latency_ms": int((time.time() - started) * 1000),
                }
            except Exception as exc:
                out["blockchain_tps"] = {"ok": False, "error": str(exc)}

        out["checked_at"] = int(time.time())
        return out

    def mempool_status(self) -> dict[str, Any]:
        out: dict[str, Any] = {"evm": {"ok": False}, "svm": {"ok": False}, "btc": {"ok": False}, "x3vm": {"ok": False}}

        # X3VM (Substrate)
        try:
            health = rpc_call(self.mode_map[self.state.get("mode", "testnet")]["rpc"], "system_health", [])
            header = rpc_call(self.mode_map[self.state.get("mode", "testnet")]["rpc"], "chain_getHeader", [])
            out["x3vm"] = {
                "ok": True,
                "rpc": self.mode_map[self.state.get("mode", "testnet")]["rpc"],
                "peers": health.get("result", {}).get("peers", 0),
                "best_block": header.get("result", {}).get("number", "0x0"),
            }
        except Exception as exc:
            out["x3vm"] = {"ok": False, "error": str(exc)}

        # EVM mempool
        evm_rpc = self._pick_evm_rpc()
        if evm_rpc:
            try:
                pending_block = rpc_call(evm_rpc, "eth_getBlockByNumber", ["pending", False])
                pending_count = len(pending_block.get("result", {}).get("transactions", []) or [])
                txpool = None
                try:
                    txpool = rpc_call(evm_rpc, "txpool_status", [])
                except Exception:
                    txpool = None
                gas = None
                priority = None
                try:
                    gas = rpc_call(evm_rpc, "eth_gasPrice", [])
                except Exception:
                    gas = None
                try:
                    priority = rpc_call(evm_rpc, "eth_maxPriorityFeePerGas", [])
                except Exception:
                    priority = None
                gas_gwei = None
                priority_gwei = None
                if isinstance(gas, dict) and isinstance(gas.get("result"), str):
                    gas_gwei = int(gas["result"], 16) / 1_000_000_000
                if isinstance(priority, dict) and isinstance(priority.get("result"), str):
                    priority_gwei = int(priority["result"], 16) / 1_000_000_000
                gas_tiers = None
                if gas_gwei is not None:
                    slow = max(1, gas_gwei * 0.9)
                    std = max(1, gas_gwei)
                    fast = max(1, gas_gwei * 1.2)
                    mev = max(1, gas_gwei * 1.5 + (priority_gwei or 0))
                    gas_tiers = {
                        "base_gwei": round(gas_gwei, 2),
                        "priority_gwei": round(priority_gwei or 0, 2),
                        "slow_gwei": round(slow, 2),
                        "standard_gwei": round(std, 2),
                        "fast_gwei": round(fast, 2),
                        "mev_gwei": round(mev, 2),
                    }
                out["evm"] = {
                    "ok": True,
                    "rpc": evm_rpc,
                    "pending": pending_count,
                    "txpool": txpool.get("result") if isinstance(txpool, dict) else None,
                    "gas": gas_tiers,
                }
            except Exception as exc:
                out["evm"] = {"ok": False, "rpc": evm_rpc, "error": str(exc)}
        else:
            out["evm"] = {"ok": False, "error": "no evm rpc configured"}

        # SVM mempool (no direct mempool; report health + slot)
        svm_rpc = self._pick_svm_rpc()
        if svm_rpc:
            try:
                health = rpc_call(svm_rpc, "getHealth", [])
                slot = rpc_call(svm_rpc, "getSlot", [])
                priority_fee = None
                try:
                    fees = rpc_call(svm_rpc, "getRecentPrioritizationFees", [[]])
                    if isinstance(fees.get("result"), list) and fees["result"]:
                        priority_fee = fees["result"][0].get("prioritizationFee", 0)
                except Exception:
                    priority_fee = None
                out["svm"] = {
                    "ok": True,
                    "rpc": svm_rpc,
                    "health": health.get("result"),
                    "slot": slot.get("result"),
                    "base_fee": 0,
                    "priority_fee": priority_fee,
                }
            except Exception as exc:
                out["svm"] = {"ok": False, "rpc": svm_rpc, "error": str(exc)}
        else:
            out["svm"] = {"ok": False, "error": "no svm rpc configured"}

        # BTC mempool
        btc_rpc = self._pick_btc_rpc()
        if btc_rpc:
            try:
                info = rpc_call(btc_rpc, "getmempoolinfo", [])
                chain = rpc_call(btc_rpc, "getblockchaininfo", [])
                out["btc"] = {
                    "ok": True,
                    "rpc": btc_rpc,
                    "mempool": info.get("result"),
                    "chain": chain.get("result"),
                }
            except Exception as exc:
                out["btc"] = {"ok": False, "rpc": btc_rpc, "error": str(exc)}
        else:
            # fallback to public mempool API
            try:
                info = http_get_json(f"{self.btc_mempool_api}/mempool")
                tip = http_get_json(f"{self.btc_mempool_api}/blocks/tip/height")
                fees = http_get_json(f"{self.btc_mempool_api}/v1/fees/recommended")
                out["btc"] = {
                    "ok": True,
                    "rpc": self.btc_mempool_api,
                    "mempool": info,
                    "tip_height": tip,
                    "fees": fees if isinstance(fees, dict) else None,
                }
            except Exception as exc:
                out["btc"] = {"ok": False, "error": str(exc)}

        return out

    def mev_status(self) -> dict[str, Any]:
        stats = self.chain_db_get("/api/rpc/stats") or {}
        by_tier = stats.get("by_tier", []) if isinstance(stats, dict) else []
        mev_count = 0
        for row in by_tier:
            if str(row.get("tier", "")).lower() == "mev":
                mev_count += row.get("count", 0)
        mev_items: list[dict[str, Any]] = []
        evm_rpc = self._pick_rpc_from_chain_db("eth") or self._pick_rpc_from_chain_db("ethereum")
        if evm_rpc:
            mev_items.append({"name": "EVM RPC", "url": evm_rpc, "tier": "primary"})
        try:
            res = self.chain_db_get("/api/rpc/eth")
            if isinstance(res, dict):
                for row in res.get("endpoints", []) or []:
                    if str(row.get("tier", "")).lower() == "mev" or "mev" in str(row.get("provider", "")).lower():
                        mev_items.append(
                            {
                                "name": row.get("provider") or "mev",
                                "url": row.get("url"),
                                "tier": row.get("tier") or "mev",
                                "latency_ms": row.get("latency_ms"),
                            }
                        )
        except Exception:
            pass
        return {
            "ok": True,
            "mev_endpoints": mev_count,
            "total_endpoints": stats.get("total_endpoints", 0),
            "healthy_endpoints": stats.get("healthy_endpoints", 0),
            "items": mev_items,
            "updated_at": int(time.time()),
        }

    def liquidity_status(self) -> dict[str, Any]:
        evm_rpc = self._pick_evm_rpc()
        out: dict[str, Any] = {"evm": {"ok": False}}
        if not evm_rpc:
            out["evm"] = {"ok": False, "error": "no evm rpc configured"}
            return out
        try:
            call = {"to": self.liquidity_pair, "data": "0x0902f1ac"}  # getReserves()
            resp = rpc_call(evm_rpc, "eth_call", [call, "latest"])
            data = resp.get("result", "0x")
            raw = data[2:].rjust(64 * 3, "0")
            r0 = int(raw[0:64], 16)
            r1 = int(raw[64:128], 16)
            price = 0.0
            if r0 and r1:
                price = (r0 / (10**self.liq_token0_dec)) / (r1 / (10**self.liq_token1_dec))
            r0_adj = r0 / (10**self.liq_token0_dec) if r0 else 0.0
            r1_adj = r1 / (10**self.liq_token1_dec) if r1 else 0.0
            tvl_usd = r0_adj + (r1_adj * price if price else 0.0)
            out["evm"] = {
                "ok": True,
                "rpc": evm_rpc,
                "pair": self.liquidity_pair,
                "token0": self.liq_token0_symbol,
                "token1": self.liq_token1_symbol,
                "reserve0": r0,
                "reserve1": r1,
                "reserve0_adj": r0_adj,
                "reserve1_adj": r1_adj,
                "price_token1": price,
                "tvl_usd": tvl_usd,
            }
        except Exception as exc:
            out["evm"] = {"ok": False, "rpc": evm_rpc, "error": str(exc)}
        return out

    def fuzzer_status(self) -> dict[str, Any]:
        evm_rpc = self._pick_evm_rpc()
        if not evm_rpc:
            return {"ok": False, "error": "no evm rpc configured"}
        try:
            code = rpc_call(evm_rpc, "eth_getCode", [self.fuzzer_target, "latest"])
            has_code = code.get("result", "0x") not in {"0x", "0x0"}
            return {"ok": True, "rpc": evm_rpc, "target": self.fuzzer_target, "has_code": has_code}
        except Exception as exc:
            return {"ok": False, "rpc": evm_rpc, "error": str(exc)}

    def fuzzer_run(self) -> dict[str, Any]:
        evm_rpc = self._pick_evm_rpc()
        if not evm_rpc:
            return {"ok": False, "error": "no evm rpc configured"}
        selectors = {
            "name": "0x06fdde03",
            "symbol": "0x95d89b41",
            "decimals": "0x313ce567",
            "totalSupply": "0x18160ddd",
        }
        results: dict[str, Any] = {}
        for label, sig in selectors.items():
            try:
                resp = rpc_call(evm_rpc, "eth_call", [{"to": self.fuzzer_target, "data": sig}, "latest"])
                results[label] = resp.get("result")
            except Exception as exc:
                results[label] = {"error": str(exc)}
        return {"ok": True, "rpc": evm_rpc, "target": self.fuzzer_target, "results": results}

    def crossvm_status(self) -> dict[str, Any]:
        out: dict[str, Any] = {"x3vm": {}, "evm": {}, "svm": {}, "btc": {}}
        try:
            health = rpc_call(self.mode_map[self.state.get("mode", "testnet")]["rpc"], "system_health", [])
            header = rpc_call(self.mode_map[self.state.get("mode", "testnet")]["rpc"], "chain_getHeader", [])
            out["x3vm"] = {"ok": True, "peers": health.get("result", {}).get("peers", 0), "block": header.get("result", {}).get("number", "0x0")}
        except Exception as exc:
            out["x3vm"] = {"ok": False, "error": str(exc)}

        evm_rpc = self._pick_evm_rpc()
        if evm_rpc:
            try:
                block = rpc_call(evm_rpc, "eth_blockNumber", [])
                out["evm"] = {"ok": True, "rpc": evm_rpc, "block": block.get("result")}
            except Exception as exc:
                out["evm"] = {"ok": False, "rpc": evm_rpc, "error": str(exc)}
        else:
            out["evm"] = {"ok": False, "error": "no evm rpc configured"}

        svm_rpc = self._pick_svm_rpc()
        if svm_rpc:
            try:
                slot = rpc_call(svm_rpc, "getSlot", [])
                out["svm"] = {"ok": True, "rpc": svm_rpc, "slot": slot.get("result")}
            except Exception as exc:
                out["svm"] = {"ok": False, "rpc": svm_rpc, "error": str(exc)}
        else:
            out["svm"] = {"ok": False, "error": "no svm rpc configured"}

        btc_rpc = self._pick_btc_rpc()
        if btc_rpc:
            try:
                tip = rpc_call(btc_rpc, "getblockcount", [])
                out["btc"] = {"ok": True, "rpc": btc_rpc, "height": tip.get("result")}
            except Exception as exc:
                out["btc"] = {"ok": False, "rpc": btc_rpc, "error": str(exc)}
        else:
            try:
                tip = http_get_json(f"{self.btc_mempool_api}/blocks/tip/height")
                out["btc"] = {"ok": True, "rpc": self.btc_mempool_api, "height": tip}
            except Exception as exc:
                out["btc"] = {"ok": False, "error": str(exc)}
        return out

    def crossvm_sim(self, payload: dict[str, Any]) -> dict[str, Any]:
        source = str(payload.get("source", "X3VM")).upper()
        dest = str(payload.get("dest", payload.get("destination", "EVM"))).upper()
        protocol = str(payload.get("protocol", "X3 PROOF"))
        iterations = int(payload.get("iterations", 100))
        trade_size = float(payload.get("trade_size", 0) or 0)

        connectors = (self.connectors_health().get("items") or [])

        def group_for(item: dict[str, Any]) -> str | None:
            eco = str(item.get("ecosystem") or "").lower()
            cid = str(item.get("chain_id") or item.get("chain") or "").lower()
            if eco == "evm" or any(x in cid for x in ["eth", "bsc", "polygon", "arb", "op"]):
                return "EVM"
            if eco == "svm" or "sol" in cid:
                return "SVM"
            if "btc" in cid or "bitcoin" in cid:
                return "BTC"
            if "x3" in cid:
                return "X3VM"
            return None

        def best_latency(label: str) -> int | None:
            hits = [c for c in connectors if group_for(c) == label and c.get("latency_ms") is not None]
            if not hits:
                return None
            return int(min(h["latency_ms"] for h in hits))

        def is_ok(label: str) -> bool:
            if label == "X3VM":
                return True
            hits = [c for c in connectors if group_for(c) == label]
            if not hits:
                return False
            return any(bool(h.get("ok")) for h in hits)

        src_lat = best_latency(source)
        dst_lat = best_latency(dest)
        src_ok = is_ok(source)
        dst_ok = is_ok(dest)

        rtt = None
        if source == dest and src_lat is not None:
            rtt = src_lat
        elif src_lat is not None and dst_lat is not None:
            rtt = src_lat + dst_lat

        p99 = int(rtt * 1.5) if rtt is not None else None
        fail_rate = 0 if (src_ok and dst_ok) else 100
        sla_pass = 100 if (rtt is not None and rtt < 800 and fail_rate == 0) else 0

        fee_est = "n/a"
        if source == "EVM" or dest == "EVM":
            try:
                evm_rpc = self._pick_evm_rpc()
                if evm_rpc:
                    gas = rpc_call(evm_rpc, "eth_gasPrice", [])
                    if isinstance(gas.get("result"), str):
                        gwei = int(gas["result"], 16) / 1_000_000_000
                        fee_est = f"{gwei:.2f} gwei"
            except Exception:
                pass
        if source == "BTC" or dest == "BTC":
            try:
                fees = http_get_json(f"{self.btc_mempool_api}/v1/fees/recommended")
                if isinstance(fees, dict) and "fastestFee" in fees:
                    fee_est = f"{fees['fastestFee']} sat/vB"
            except Exception:
                pass
        if source == "SVM" or dest == "SVM":
            try:
                svm_rpc = self._pick_svm_rpc()
                if svm_rpc:
                    fees = rpc_call(svm_rpc, "getRecentPrioritizationFees", [[]])
                    if isinstance(fees.get("result"), list) and fees["result"]:
                        fee_est = f"{fees['result'][0].get('prioritizationFee', 0)} lamports"
            except Exception:
                pass

        return {
            "ok": rtt is not None,
            "source": source,
            "dest": dest,
            "protocol": protocol,
            "iterations": iterations,
            "trade_size": trade_size,
            "rtt_ms": rtt,
            "p99_ms": p99,
            "fail_rate_pct": fail_rate,
            "sla_pass_pct": sla_pass,
            "fee_est": fee_est,
        }

    def crossvm_protocols(self, source: str, dest: str) -> dict[str, Any]:
        src = source.upper()
        dst = dest.upper()
        protocols = ["ATTESTED"]
        if "BTC" in (src, dst):
            protocols.insert(0, "HTLC")
        else:
            protocols.insert(0, "X3 PROOF")
            protocols.insert(1, "THRESHOLD SIGNED")
        return {
            "source": src,
            "dest": dst,
            "protocols": protocols,
        }

    def drpc_status(self) -> dict[str, Any]:
        out: dict[str, Any] = {
            "nodecore": {"query_url": self.nodecore_query_url, "ok": False},
            "dshackle": {"proxy_url": self.dshackle_proxy_url, "ok": False},
            "checked_at": int(time.time()),
        }

        # Nodecore JSON-RPC path (default /queries/ethereum)
        try:
            started = time.time()
            req = urllib.request.Request(
                self.nodecore_query_url,
                data=json.dumps(
                    {
                        "jsonrpc": "2.0",
                        "id": 1,
                        "method": "eth_blockNumber",
                        "params": [],
                    }
                ).encode("utf-8"),
                headers={"Content-Type": "application/json"},
            )
            with urllib.request.urlopen(req, timeout=2.5) as resp:
                body = json.loads(resp.read().decode("utf-8"))
            out["nodecore"] = {
                "query_url": self.nodecore_query_url,
                "ok": "result" in body and "error" not in body,
                "latency_ms": int((time.time() - started) * 1000),
                "result": body.get("result"),
                "error": body.get("error"),
            }
        except Exception as exc:
            out["nodecore"]["error"] = str(exc)

        # Dshackle proxy path (default /eth)
        try:
            started = time.time()
            req = urllib.request.Request(
                self.dshackle_proxy_url,
                data=json.dumps(
                    {
                        "jsonrpc": "2.0",
                        "id": 1,
                        "method": "eth_blockNumber",
                        "params": [],
                    }
                ).encode("utf-8"),
                headers={"Content-Type": "application/json"},
            )
            with urllib.request.urlopen(req, timeout=2.5) as resp:
                body = json.loads(resp.read().decode("utf-8"))
            ok = "result" in body and "error" not in body
            warning = None
            if not ok and isinstance(body.get("error"), dict):
                err = body.get("error", {})
                if err.get("code") == -32003 and "Unsupported blockchain" in str(err.get("message", "")):
                    ok = True
                    warning = "proxy reachable; upstream chain not enabled in dshackle"

            out["dshackle"] = {
                "proxy_url": self.dshackle_proxy_url,
                "ok": ok,
                "latency_ms": int((time.time() - started) * 1000),
                "result": body.get("result"),
                "error": body.get("error"),
                "warning": warning,
            }
        except Exception as exc:
            out["dshackle"]["error"] = str(exc)

        return out

    def run_harness(self, submit_remark: bool = False, rpc_url: str | None = None, ws_url: str | None = None, suri: str | None = None) -> dict[str, Any]:
        cmd = [
            sys.executable,
            str(REPO_ROOT / "scripts/testnet/cross-vm-harness.py"),
            "--rpc-url",
            rpc_url or self.mode_map[self.state.get("mode", "testnet")]["rpc"],
            "--ws-url",
            ws_url or "ws://127.0.0.1:9944",
        ]
        if suri:
            cmd.extend(["--suri", suri])
        if submit_remark:
            cmd.append("--submit-remark")

        proc = subprocess.run(cmd, cwd=REPO_ROOT, capture_output=True, text=True, timeout=60, check=False)
        return {
            "cmd": cmd,
            "returncode": proc.returncode,
            "stdout": proc.stdout,
            "stderr": proc.stderr,
        }


class Handler(BaseHTTPRequestHandler):
    state: ChainbenchState
    count: int
    base_port: int

    server_version = "ChainbenchServer/1.0"

    def _send_json(self, payload: dict[str, Any], status: int = 200) -> None:
        data = json.dumps(payload).encode("utf-8")
        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(data)))
        self.send_header("Access-Control-Allow-Origin", "*")
        self.end_headers()
        self.wfile.write(data)

    def _read_json(self) -> dict[str, Any]:
        length = int(self.headers.get("Content-Length", "0"))
        raw = self.rfile.read(length) if length else b"{}"
        return json.loads(raw.decode("utf-8") or "{}")

    def _serve_dashboard(self) -> None:
        content = DASHBOARD_FILE.read_bytes()
        self.send_response(HTTPStatus.OK)
        self.send_header("Content-Type", "text/html; charset=utf-8")
        self.send_header("Content-Length", str(len(content)))
        self.end_headers()
        self.wfile.write(content)

    def _gpu_route_benchmark(self, body: dict[str, Any]) -> dict[str, Any]:
        direct_rpc = str(body.get("direct_rpc", "http://127.0.0.1:9944")).strip()
        method = str(body.get("method", "system_health")).strip()
        params = body.get("params", [])
        if not isinstance(params, list):
            raise ValueError("params must be a list")

        iterations = int(body.get("iterations", 12))
        iterations = max(3, min(200, iterations))
        timeout_ms = int(body.get("timeout_ms", 2500))
        timeout_s = max(0.2, min(15.0, timeout_ms / 1000.0))

        route_pool = []
        for i in range(self.count):
            route_pool.append(f"http://127.0.0.1:{self.base_port + i}")

        def best_routed() -> dict[str, Any]:
            with concurrent.futures.ThreadPoolExecutor(max_workers=len(route_pool)) as ex:
                fut_map = {
                    ex.submit(timed_rpc_call, url, method, params, timeout_s): url
                    for url in route_pool
                }
                best: dict[str, Any] | None = None
                for fut in concurrent.futures.as_completed(fut_map):
                    url = fut_map[fut]
                    res = fut.result()
                    if not res.get("ok"):
                        continue
                    candidate = {
                        "url": url,
                        "latency_ms": res.get("latency_ms", 9999),
                        "result": res.get("result"),
                    }
                    if best is None or candidate["latency_ms"] < best["latency_ms"]:
                        best = candidate
                if best is None:
                    return {
                        "ok": False,
                        "url": None,
                        "latency_ms": None,
                        "error": "all routed validators failed",
                    }
                return {"ok": True, **best}

        direct_samples: list[int] = []
        routed_samples: list[int] = []
        routed_winners: dict[str, int] = {}
        failures = {"direct": 0, "routed": 0}

        for _ in range(iterations):
            d = timed_rpc_call(direct_rpc, method, params, timeout_s)
            if d.get("ok"):
                direct_samples.append(int(d.get("latency_ms", 0)))
            else:
                failures["direct"] += 1

            r = best_routed()
            if r.get("ok"):
                lat = int(r.get("latency_ms", 0))
                routed_samples.append(lat)
                winner = str(r.get("url"))
                routed_winners[winner] = routed_winners.get(winner, 0) + 1
            else:
                failures["routed"] += 1

        direct_avg = int(sum(direct_samples) / len(direct_samples)) if direct_samples else None
        routed_avg = int(sum(routed_samples) / len(routed_samples)) if routed_samples else None
        speedup_pct = None
        if direct_avg and routed_avg and direct_avg > 0:
            speedup_pct = round(((direct_avg - routed_avg) / direct_avg) * 100, 2)

        top_winner = None
        if routed_winners:
            top_winner = max(routed_winners.items(), key=lambda x: x[1])[0]

        return {
            "ok": direct_avg is not None and routed_avg is not None,
            "method": method,
            "iterations": iterations,
            "direct": {
                "rpc": direct_rpc,
                "avg_latency_ms": direct_avg,
                "samples": direct_samples,
                "failures": failures["direct"],
            },
            "routed": {
                "pool_size": len(route_pool),
                "avg_latency_ms": routed_avg,
                "samples": routed_samples,
                "failures": failures["routed"],
                "winner_counts": routed_winners,
                "top_winner": top_winner,
            },
            "speedup_pct": speedup_pct,
            "updated_at": int(time.time()),
        }

    def do_OPTIONS(self) -> None:
        self.send_response(204)
        self.send_header("Access-Control-Allow-Origin", "*")
        self.send_header("Access-Control-Allow-Headers", "Content-Type,X-Admin-Key")
        self.send_header("Access-Control-Allow-Methods", "GET,POST,OPTIONS")
        self.end_headers()

    def do_GET(self) -> None:
        if self.path in {"/", "/chainbench", "/chainbench/"}:
            return self._serve_dashboard()

        if self.path == "/health":
            return self._send_json({"ok": True, "service": "chainbench-server"})

        if self.path == "/api/mode":
            return self._send_json(self.state.get_mode_payload())

        if self.path == "/api/onboarding/chains":
            return self._send_json({"items": self.state.list_onboarded()})

        if self.path == "/api/rpc/bench":
            return self._send_json(self.state.rpc_bench_last())

        if self.path == "/api/rpc/defaults":
            return self._send_json({"endpoints": self.state.rpc_default_endpoints()})

        if self.path == "/api/connectors/health":
            return self._send_json(self.state.connectors_health())

        if self.path == "/api/network/status":
            return self._send_json(self.state.network_status(self.count, self.base_port))

        if self.path == "/api/adapters/status":
            return self._send_json(self.state.adapter_status())

        if self.path == "/api/drpc/status":
            return self._send_json(self.state.drpc_status())

        if self.path == "/api/infra/status":
            return self._send_json(self.state.infra_status())

        if self.path == "/api/admin/state":
            return self._send_json({"admin_enabled": bool(self.state.state.get("admin_enabled", False))})

        if self.path == "/api/mempool/status":
            return self._send_json(self.state.mempool_status())

        if self.path == "/api/mev/status":
            return self._send_json(self.state.mev_status())

        if self.path == "/api/liquidity/status":
            return self._send_json(self.state.liquidity_status())

        if self.path == "/api/fuzzer/status":
            return self._send_json(self.state.fuzzer_status())

        if self.path.startswith("/api/crossvm/protocols"):
            try:
                from urllib.parse import parse_qs, urlparse

                parsed = urlparse(self.path)
                params = parse_qs(parsed.query)
                source = (params.get("source") or params.get("src") or ["X3VM"])[0]
                dest = (params.get("dest") or params.get("dst") or ["EVM"])[0]
                return self._send_json(self.state.crossvm_protocols(source, dest))
            except Exception as exc:
                return self._send_json({"error": str(exc)}, status=400)

        if self.path == "/api/crossvm/status":
            return self._send_json(self.state.crossvm_status())

        if self.path == "/api/gpu-route/defaults":
            return self._send_json(
                {
                    "direct_rpc": "http://127.0.0.1:9944",
                    "method": "system_health",
                    "iterations": 12,
                    "timeout_ms": 2500,
                    "route_pool": [f"http://127.0.0.1:{self.base_port + i}" for i in range(self.count)],
                }
            )

        return self._send_json({"error": "not found", "path": self.path}, status=404)

    def do_POST(self) -> None:
        try:
            body = self._read_json()
        except Exception as exc:
            return self._send_json({"error": f"invalid json: {exc}"}, status=400)

        try:
            if self.path == "/api/mode":
                mode = str(body.get("mode", "")).strip().lower()
                return self._send_json(self.state.set_mode(mode))

            if self.path == "/api/rpc/sweep":
                chain_id = body.get("chain_id")
                rpc_override = body.get("rpc")
                return self._send_json(self.state.rpc_sweep(chain_id=chain_id, rpc_override=rpc_override))

            if self.path == "/api/onboarding/chains":
                key = self.headers.get("X-Admin-Key", "")
                if self.state.admin_key and key != self.state.admin_key:
                    return self._send_json({"error": "unauthorized"}, status=401)
                entry = self.state.add_onboarded(body)
                return self._send_json({"ok": True, "entry": entry})

            if self.path == "/api/rpc/bench":
                endpoints = body.get("endpoints")
                methods = body.get("methods")
                chain_id = body.get("chain_id")
                if endpoints is not None and not isinstance(endpoints, list):
                    return self._send_json({"error": "endpoints must be list"}, status=400)
                if methods is not None and not isinstance(methods, list):
                    return self._send_json({"error": "methods must be list"}, status=400)
                return self._send_json(self.state.rpc_bench(endpoints=endpoints, methods=methods, chain_id=chain_id))

            if self.path == "/api/harness/smoke":
                out = self.state.run_harness(submit_remark=False)
                return self._send_json(out, status=200 if out["returncode"] == 0 else 500)

            if self.path == "/api/harness/remark":
                out = self.state.run_harness(
                    submit_remark=True,
                    rpc_url=body.get("rpc_url"),
                    ws_url=body.get("ws_url"),
                    suri=body.get("suri"),
                )
                return self._send_json(out, status=200 if out["returncode"] == 0 else 500)

            if self.path == "/api/gpu-route/benchmark":
                out = self._gpu_route_benchmark(body)
                return self._send_json(out, status=200 if out.get("ok") else 500)

            if self.path == "/api/fuzzer/run":
                out = self.state.fuzzer_run()
                return self._send_json(out, status=200 if out.get("ok") else 500)

            if self.path == "/api/crossvm/sim":
                out = self.state.crossvm_sim(body)
                return self._send_json(out, status=200 if out.get("ok") else 500)

            if self.path == "/api/admin/toggle":
                key = self.headers.get("X-Admin-Key", "")
                if not self.state.admin_key or key != self.state.admin_key:
                    return self._send_json({"error": "unauthorized"}, status=401)
                enabled = bool(body.get("enabled", False))
                return self._send_json(self.state.set_admin_enabled(enabled))

            return self._send_json({"error": "not found", "path": self.path}, status=404)

        except ValueError as exc:
            return self._send_json({"error": str(exc)}, status=400)
        except Exception as exc:
            return self._send_json({"error": str(exc)}, status=500)


def build_handler(state: ChainbenchState, count: int, base_port: int):
    class BoundHandler(Handler):
        pass

    BoundHandler.state = state
    BoundHandler.count = count
    BoundHandler.base_port = base_port
    return BoundHandler


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--host", default="127.0.0.1")
    parser.add_argument("--port", type=int, default=7788)
    parser.add_argument("--state-dir", default=str(STATE_DIR_DEFAULT))
    parser.add_argument("--testnet-rpc", default=os.getenv("CHAINBENCH_TESTNET_RPC", "http://127.0.0.1:9944"))
    parser.add_argument("--live-rpc", default=os.getenv("CHAINBENCH_LIVE_RPC", "https://rpc.x3star.net"))
    parser.add_argument("--admin-key", default=os.getenv("CHAINBENCH_ADMIN_KEY", "x3-admin-local"))
    parser.add_argument(
        "--nodecore-query-url",
        default=os.getenv("CHAINBENCH_NODECORE_QUERY_URL", "http://127.0.0.1:9090/queries/ethereum"),
    )
    parser.add_argument(
        "--dshackle-proxy-url",
        default=os.getenv("CHAINBENCH_DSHACKLE_PROXY_URL", "http://127.0.0.1:8545/eth"),
    )
    parser.add_argument(
        "--chain-db-url",
        default=os.getenv("CHAINBENCH_CHAIN_DB_URL", "http://127.0.0.1:7070"),
    )
    parser.add_argument(
        "--tps-url",
        default=os.getenv("CHAINBENCH_TPS_URL", "http://127.0.0.1:3010"),
    )
    parser.add_argument(
        "--default-chain-id",
        default=os.getenv("CHAINBENCH_DEFAULT_CHAIN_ID", "eth"),
    )
    parser.add_argument(
        "--chain-db-admin-key",
        default=os.getenv("CHAINBENCH_CHAIN_DB_ADMIN_KEY", os.getenv("CHAIN_DB_ADMIN_KEY", "")),
    )
    parser.add_argument("--count", type=int, default=int(os.getenv("CHAINBENCH_VALIDATOR_COUNT", "7")))
    parser.add_argument("--base-port", type=int, default=int(os.getenv("CHAINBENCH_BASE_RPC_PORT", "9944")))
    args = parser.parse_args()

    state = ChainbenchState(
        state_dir=Path(args.state_dir),
        testnet_rpc=args.testnet_rpc,
        live_rpc=args.live_rpc,
        admin_key=args.admin_key,
        nodecore_query_url=args.nodecore_query_url,
        dshackle_proxy_url=args.dshackle_proxy_url,
        chain_db_url=args.chain_db_url,
        tps_url=args.tps_url,
        default_chain_id=args.default_chain_id,
        chain_db_admin_key=args.chain_db_admin_key,
    )

    scan_interval = int(os.getenv("CHAINBENCH_CONNECTOR_SCAN_SEC", "120"))

    def connector_loop() -> None:
        while True:
            with contextlib.suppress(Exception):
                state.refresh_connectors_health()
            time.sleep(max(30, scan_interval))

    threading.Thread(target=connector_loop, daemon=True).start()

    server = ThreadingHTTPServer((args.host, args.port), build_handler(state, args.count, args.base_port))
    print(f"chainbench server listening on http://{args.host}:{args.port}")
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        pass
    finally:
        server.server_close()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
