#!/usr/bin/env python3
"""Benchmark TPS for all configured chains against GPU lanes.

This script sends batch acceleration requests directly to the GPU lane
`/accelerate/batch` endpoints for each configured chain and records the
best observed TPS per chain.
"""

from __future__ import annotations

import argparse
import asyncio
import json
import time
from dataclasses import dataclass
from pathlib import Path
from types import SimpleNamespace
from typing import Any

import aiohttp

from cross_chain_gpu_validator.chain_registry import load_default_chain_configs


DEFAULT_LANES = (
    ("primary", "http://localhost:9001/accelerate/batch"),
    ("shadow", "http://localhost:9002/accelerate/batch"),
    ("tertiary", "http://localhost:9003/accelerate/batch"),
)


@dataclass
class LaneResult:
    lane: str
    requested: int
    successful: int
    failed: int
    duration_s: float
    status: int
    error: str | None = None


def parse_levels(raw: str) -> list[int]:
    levels = []
    for part in raw.split(","):
        part = part.strip()
        if not part:
            continue
        value = int(part)
        if value <= 0:
            raise ValueError(f"invalid level: {value}")
        levels.append(value)
    if not levels:
        raise ValueError("no load levels provided")
    return levels


def build_payload(chain_id: str, start_id: int, count: int, tx_data_hex: str) -> bytes:
    txns = [{"tx_hash": f"{chain_id}-{start_id + i}", "tx_data": tx_data_hex} for i in range(count)]
    return json.dumps({"transactions": txns, "chain": chain_id}).encode("utf-8")


async def check_lane_health(session: aiohttp.ClientSession, lane_url: str) -> tuple[bool, str]:
    base = lane_url.rsplit("/", 2)[0]
    health = f"{base}/health"
    try:
        async with session.get(health, timeout=aiohttp.ClientTimeout(total=3)) as resp:
            if resp.status != 200:
                return False, f"HTTP {resp.status}"
            return True, ""
    except Exception as exc:  # pragma: no cover - network/runtime errors
        return False, str(exc)


async def send_lane_batch(
    session: aiohttp.ClientSession,
    lane_name: str,
    lane_url: str,
    payload: bytes,
    requested: int,
    timeout_s: float,
) -> LaneResult:
    start = time.time()
    try:
        async with session.post(
            lane_url,
            data=payload,
            headers={"Content-Type": "application/json"},
            timeout=aiohttp.ClientTimeout(total=timeout_s),
        ) as resp:
            data = await resp.json()
            successful = int(data.get("successful", 0))
            failed = max(requested - successful, 0)
            return LaneResult(
                lane=lane_name,
                requested=requested,
                successful=successful,
                failed=failed,
                duration_s=time.time() - start,
                status=resp.status,
            )
    except Exception as exc:  # pragma: no cover - network/runtime errors
        return LaneResult(
            lane=lane_name,
            requested=requested,
            successful=0,
            failed=requested,
            duration_s=time.time() - start,
            status=0,
            error=str(exc),
        )


async def run_chain_level(
    session: aiohttp.ClientSession,
    chain_id: str,
    level: int,
    lanes: list[tuple[str, str]],
    tx_data_hex: str,
    timeout_s: float,
) -> dict[str, Any]:
    lane_count = len(lanes)
    per_lane = level // lane_count
    remainder = level % lane_count

    tasks = []
    tx_offset = 0
    for idx, (lane_name, lane_url) in enumerate(lanes):
        count = per_lane + (1 if idx < remainder else 0)
        payload = build_payload(chain_id, tx_offset, count, tx_data_hex)
        tx_offset += count
        tasks.append(send_lane_batch(session, lane_name, lane_url, payload, count, timeout_s))

    start = time.time()
    lane_results = await asyncio.gather(*tasks)
    total_duration = time.time() - start

    successful = sum(result.successful for result in lane_results)
    failed = sum(result.failed for result in lane_results)
    tps = successful / total_duration if total_duration > 0 else 0.0
    success_rate = successful / level if level > 0 else 0.0

    return {
        "level": level,
        "successful": successful,
        "failed": failed,
        "success_rate": success_rate,
        "duration_s": total_duration,
        "tps": tps,
        "lanes": [result.__dict__ for result in lane_results],
    }


async def benchmark_all(args: argparse.Namespace) -> dict[str, Any]:
    chain_configs = load_default_chain_configs()
    chain_items = sorted(chain_configs.items(), key=lambda item: item[0])

    our_chain_id = args.our_chain_id
    has_our_chain = any(chain_id == our_chain_id for chain_id, _ in chain_items)
    if not has_our_chain:
        # Ensure the requested "our chain" is always benchmarked, even when
        # the loaded chain registry source (e.g., chains.json) omits it.
        chain_items.append((our_chain_id, SimpleNamespace(chain_name=our_chain_id.upper())))

    if args.max_chains:
        chain_items = chain_items[: args.max_chains]
        if not any(chain_id == our_chain_id for chain_id, _ in chain_items):
            if chain_items:
                chain_items[-1] = (our_chain_id, SimpleNamespace(chain_name=our_chain_id.upper()))
            else:
                chain_items = [(our_chain_id, SimpleNamespace(chain_name=our_chain_id.upper()))]

    levels = parse_levels(args.levels)
    lanes = list(DEFAULT_LANES)
    results: list[dict[str, Any]] = []
    started_at = time.time()

    connector = aiohttp.TCPConnector(limit=64, limit_per_host=32)
    async with aiohttp.ClientSession(connector=connector) as session:
        for lane_name, lane_url in lanes:
            ok, message = await check_lane_health(session, lane_url)
            if not ok:
                raise RuntimeError(f"lane {lane_name} unavailable: {message}")

        total = len(chain_items)
        for index, (chain_id, config) in enumerate(chain_items, start=1):
            print(f"[{index}/{total}] benchmarking {chain_id} ...")
            level_results = []
            for level in levels:
                level_result = await run_chain_level(
                    session=session,
                    chain_id=chain_id,
                    level=level,
                    lanes=lanes,
                    tx_data_hex=args.tx_data_hex,
                    timeout_s=args.timeout_s,
                )
                level_results.append(level_result)

            best = max(level_results, key=lambda item: item["tps"]) if level_results else None
            results.append(
                {
                    "chain_id": chain_id,
                    "chain_name": config.chain_name,
                    "max_tps": best["tps"] if best else 0.0,
                    "best_level": best["level"] if best else 0,
                    "levels": level_results,
                }
            )

    results.sort(key=lambda item: item["max_tps"], reverse=True)
    global_max = results[0]["max_tps"] if results else 0.0

    our_chain = next((item for item in results if item["chain_id"] == our_chain_id), None)
    if our_chain is None:
        # Allow close matches, e.g. requested "solana" and measured
        # "solana-mainnet" style IDs.
        our_chain = next(
            (
                item
                for item in results
                if item["chain_id"].startswith(our_chain_id)
                or our_chain_id.startswith(item["chain_id"])
            ),
            None,
        )
    if our_chain is None:
        fallback = next((item for item in results if item["chain_id"] == "solana"), None)
        if fallback:
            our_chain_id = "solana"
            our_chain = fallback

    return {
        "generated_at": time.time(),
        "duration_s": time.time() - started_at,
        "levels": levels,
        "tested_chains": len(results),
        "our_chain_id": our_chain_id,
        "our_chain_max_tps": (our_chain or {}).get("max_tps", 0.0),
        "global_max_tps": global_max,
        "lanes": [{"name": name, "url": url} for name, url in lanes],
        "chains": results,
    }


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Benchmark max TPS for all configured chains")
    parser.add_argument(
        "--output",
        default="benchmarks/all_chains_tps.json",
        help="output JSON path, relative to cross-chain-gpu-validator root",
    )
    parser.add_argument(
        "--levels",
        default="25000,100000",
        help="comma-separated tx load levels per chain",
    )
    parser.add_argument(
        "--tx-data-hex",
        default="48656c6c6f",
        help="hex payload for each synthetic transaction",
    )
    parser.add_argument(
        "--timeout-s",
        type=float,
        default=120.0,
        help="per-lane request timeout seconds",
    )
    parser.add_argument(
        "--max-chains",
        type=int,
        default=0,
        help="optional limit for quick runs (0 means all chains)",
    )
    parser.add_argument(
        "--our-chain-id",
        default="solana",
        help="chain id highlighted as 'our chain' in dashboard",
    )
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    report = asyncio.run(benchmark_all(args))

    output_path = Path(args.output)
    output_path.parent.mkdir(parents=True, exist_ok=True)
    with output_path.open("w", encoding="utf-8") as handle:
        json.dump(report, handle, indent=2)

    print("")
    print(f"saved report: {output_path}")
    print(f"tested chains: {report['tested_chains']}")
    print(f"global max TPS: {report['global_max_tps']:.0f}")
    print(f"{report['our_chain_id']} max TPS: {report['our_chain_max_tps']:.0f}")


if __name__ == "__main__":
    main()
