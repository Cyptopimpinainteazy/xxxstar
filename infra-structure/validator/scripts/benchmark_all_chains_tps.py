#!/usr/bin/env python3
"""Benchmark configured chains against running GPU lane endpoints."""

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
    txns = [
        {"tx_hash": f"{chain_id}-{start_id + index}", "tx_data": tx_data_hex}
        for index in range(count)
    ]
    return json.dumps({"transactions": txns, "chain": chain_id}).encode("utf-8")


async def check_lane_health(
    session: aiohttp.ClientSession, lane_url: str
) -> tuple[bool, str]:
    base = lane_url.rsplit("/", 2)[0]
    try:
        async with session.get(
            f"{base}/health", timeout=aiohttp.ClientTimeout(total=3)
        ) as response:
            if response.status != 200:
                return False, f"HTTP {response.status}"
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
        ) as response:
            data = await response.json()
            successful = int(data.get("successful", 0))
            return LaneResult(
                lane=lane_name,
                requested=requested,
                successful=successful,
                failed=max(requested - successful, 0),
                duration_s=time.time() - start,
                status=response.status,
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
    per_lane, remainder = divmod(level, len(lanes))
    tasks = []
    tx_offset = 0
    for index, (lane_name, lane_url) in enumerate(lanes):
        count = per_lane + (1 if index < remainder else 0)
        payload = build_payload(chain_id, tx_offset, count, tx_data_hex)
        tx_offset += count
        tasks.append(
            send_lane_batch(session, lane_name, lane_url, payload, count, timeout_s)
        )

    start = time.time()
    lane_results = await asyncio.gather(*tasks)
    duration = time.time() - start
    successful = sum(result.successful for result in lane_results)
    failed = sum(result.failed for result in lane_results)

    return {
        "level": level,
        "successful": successful,
        "failed": failed,
        "success_rate": successful / level,
        "duration_s": duration,
        "tps": successful / duration if duration > 0 else 0.0,
        "lanes": [result.__dict__ for result in lane_results],
    }


async def benchmark_all(args: argparse.Namespace) -> dict[str, Any]:
    chain_configs = load_default_chain_configs()
    chain_items = sorted(chain_configs.items(), key=lambda item: item[0])
    our_chain_id = args.our_chain_id

    if not any(chain_id == our_chain_id for chain_id, _ in chain_items):
        chain_items.append(
            (our_chain_id, SimpleNamespace(chain_name=our_chain_id.upper()))
        )

    if args.max_chains:
        chain_items = chain_items[: args.max_chains]
        if not any(chain_id == our_chain_id for chain_id, _ in chain_items):
            target = (our_chain_id, SimpleNamespace(chain_name=our_chain_id.upper()))
            chain_items = [*chain_items[:-1], target] if chain_items else [target]

    levels = parse_levels(args.levels)
    lanes = list(DEFAULT_LANES)
    results: list[dict[str, Any]] = []
    started_at = time.time()

    connector = aiohttp.TCPConnector(limit=64, limit_per_host=32)
    async with aiohttp.ClientSession(connector=connector) as session:
        for lane_name, lane_url in lanes:
            healthy, message = await check_lane_health(session, lane_url)
            if not healthy:
                raise RuntimeError(f"lane {lane_name} unavailable: {message}")

        for index, (chain_id, config) in enumerate(chain_items, start=1):
            print(f"[{index}/{len(chain_items)}] benchmarking {chain_id} ...")
            level_results = []
            for level in levels:
                level_results.append(
                    await run_chain_level(
                        session,
                        chain_id,
                        level,
                        lanes,
                        args.tx_data_hex,
                        args.timeout_s,
                    )
                )

            best = max(level_results, key=lambda item: item["tps"])
            results.append(
                {
                    "chain_id": chain_id,
                    "chain_name": config.chain_name,
                    "max_tps": best["tps"],
                    "best_level": best["level"],
                    "levels": level_results,
                }
            )

    results.sort(key=lambda item: item["max_tps"], reverse=True)
    our_chain = next(
        (item for item in results if item["chain_id"] == our_chain_id), None
    )
    return {
        "generated_at": time.time(),
        "duration_s": time.time() - started_at,
        "levels": levels,
        "tested_chains": len(results),
        "our_chain_id": our_chain_id,
        "our_chain_max_tps": (our_chain or {}).get("max_tps", 0.0),
        "global_max_tps": results[0]["max_tps"] if results else 0.0,
        "lanes": [{"name": name, "url": url} for name, url in lanes],
        "chains": results,
    }


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", default="benchmarks/all_chains_tps.json")
    parser.add_argument("--levels", default="25000,100000")
    parser.add_argument("--tx-data-hex", default="48656c6c6f")
    parser.add_argument("--timeout-s", type=float, default=120.0)
    parser.add_argument("--max-chains", type=int, default=0)
    parser.add_argument("--our-chain-id", default="solana")
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    report = asyncio.run(benchmark_all(args))
    output_path = Path(args.output)
    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(f"saved report: {output_path}")
    print(f"tested chains: {report['tested_chains']}")
    print(f"global max TPS: {report['global_max_tps']:.0f}")


if __name__ == "__main__":
    main()
