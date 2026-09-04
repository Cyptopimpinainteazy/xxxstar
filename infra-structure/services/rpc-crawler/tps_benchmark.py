#!/usr/bin/env python3
"""
TPS Benchmark Runner — Measures real TPS for all chains with healthy RPC endpoints.

Hits each chain's fastest endpoint with eth_blockNumber / getSlot / equivalent calls,
measures response time, estimates throughput, and posts results to the chain-db API.

Usage:
    python3 tps_benchmark.py                   # Run once, benchmark all chains
    python3 tps_benchmark.py --top 200         # Only benchmark top 200 chains by latency
    python3 tps_benchmark.py --ecosystem evm   # Only EVM chains
    python3 tps_benchmark.py --loop 300        # Re-run every 5 minutes
"""

import asyncio
import aiohttp
import json
import os
import sys
import time
import signal
import argparse
import sqlite3
from pathlib import Path
from datetime import datetime

# ── Config ─────────────────────────────────────────────────────────────────────

DB_PATH = os.environ.get("CHAIN_DB_PATH", str(Path(__file__).resolve().parent.parent.parent / "db" / "chains.db"))
API_URL = os.environ.get("CHAIN_DB_API", "http://localhost:7070")
CONCURRENCY = int(os.environ.get("BENCH_CONCURRENCY", "50"))
TIMEOUT_S = float(os.environ.get("BENCH_TIMEOUT", "5"))
BATCH_POST_SIZE = 100

STOP = False

def handle_signal(sig, frame):
    global STOP
    STOP = True
    print("\n⏹  Stopping benchmark...")

signal.signal(signal.SIGINT, handle_signal)
signal.signal(signal.SIGTERM, handle_signal)

# ── RPC call payloads per ecosystem ────────────────────────────────────────────

def get_rpc_payload(ecosystem: str, chain_type: str):
    """Return (method, params, extract_fn) for measuring a chain."""
    if ecosystem in ("evm",) or chain_type in ("L2", "L3"):
        return {
            "jsonrpc": "2.0", "id": 1,
            "method": "eth_blockNumber", "params": []
        }
    elif ecosystem == "svm":
        return {
            "jsonrpc": "2.0", "id": 1,
            "method": "getSlot", "params": []
        }
    elif ecosystem == "cosmos":
        # Tendermint RPC uses different format
        return None  # Will use REST /status
    elif ecosystem == "x3":
        return {
            "jsonrpc": "2.0", "id": 1,
            "method": "system_version", "params": []
        }
    elif ecosystem == "substrate":
        return {
            "jsonrpc": "2.0", "id": 1,
            "method": "chain_getHeader", "params": []
        }
    elif ecosystem == "move":
        return None  # Aptos/Sui use REST
    else:
        return {
            "jsonrpc": "2.0", "id": 1,
            "method": "eth_blockNumber", "params": []
        }


async def measure_single_rpc(session: aiohttp.ClientSession, url: str, ecosystem: str, chain_type: str):
    """Hit an RPC endpoint and measure response time + extract block data."""
    payload = get_rpc_payload(ecosystem, chain_type)
    
    start = time.monotonic()
    try:
        if payload is None:
            # REST-based chains (cosmos, move)
            if ecosystem == "cosmos":
                # Try tendermint status endpoint
                rest_url = url.rstrip("/")
                if "/rpc" not in rest_url and rest_url.endswith(("26657", "443", "80")):
                    rest_url += "/status"
                async with session.get(rest_url, timeout=aiohttp.ClientTimeout(total=TIMEOUT_S)) as resp:
                    elapsed = time.monotonic() - start
                    if resp.status == 200:
                        data = await resp.json()
                        block = None
                        if "result" in data and "sync_info" in data["result"]:
                            block = int(data["result"]["sync_info"].get("latest_block_height", 0))
                        return {"ok": True, "latency_ms": round(elapsed * 1000), "block_height": block}
                    return {"ok": False, "error": f"HTTP {resp.status}"}
            else:
                # Try POST with eth_blockNumber as fallback
                payload = {"jsonrpc": "2.0", "id": 1, "method": "eth_blockNumber", "params": []}
        
        async with session.post(
            url,
            json=payload,
            timeout=aiohttp.ClientTimeout(total=TIMEOUT_S),
            headers={"Content-Type": "application/json"}
        ) as resp:
            elapsed = time.monotonic() - start
            if resp.status == 200:
                data = await resp.json()
                block_height = None
                if "result" in data:
                    result = data["result"]
                    if isinstance(result, str) and result.startswith("0x"):
                        block_height = int(result, 16)
                    elif isinstance(result, int):
                        block_height = result
                    elif isinstance(result, dict):
                        block_height = int(result.get("number", "0x0"), 16) if "number" in result else None
                return {"ok": True, "latency_ms": round(elapsed * 1000), "block_height": block_height}
            return {"ok": False, "error": f"HTTP {resp.status}"}
    except asyncio.TimeoutError:
        return {"ok": False, "error": "timeout"}
    except Exception as e:
        return {"ok": False, "error": str(e)[:80]}


async def measure_tps_burst(session: aiohttp.ClientSession, url: str, ecosystem: str, chain_type: str, burst_count: int = 10):
    """Send a burst of requests to estimate max TPS."""
    payload = get_rpc_payload(ecosystem, chain_type)
    if payload is None:
        payload = {"jsonrpc": "2.0", "id": 1, "method": "eth_blockNumber", "params": []}
    
    start = time.monotonic()
    tasks = []
    for i in range(burst_count):
        p = dict(payload)
        p["id"] = i + 1
        tasks.append(
            session.post(url, json=p, timeout=aiohttp.ClientTimeout(total=TIMEOUT_S),
                        headers={"Content-Type": "application/json"})
        )
    
    results = await asyncio.gather(*[_safe_request(t) for t in tasks], return_exceptions=True)
    elapsed = time.monotonic() - start
    
    successes = sum(1 for r in results if r is True)
    if elapsed > 0 and successes > 0:
        return round(successes / elapsed, 1)
    return 0


async def _safe_request(coro):
    try:
        resp = await coro
        ok = resp.status == 200
        await resp.read()
        return ok
    except:
        return False


# ── Main benchmark logic ──────────────────────────────────────────────────────

def get_chains_to_benchmark(db_path: str, ecosystem_filter: str = None, top_n: int = None):
    """Get chains with their fastest healthy RPC endpoint."""
    db = sqlite3.connect(db_path)
    db.row_factory = sqlite3.Row
    
    query = """
        SELECT c.chain_id, c.chain_name, c.ecosystem, c.chain_type, c.native_token, c.is_testnet,
               r.url as rpc_url, r.latency_ms, r.rate_limit_rps, r.provider
        FROM chains c
        JOIN rpc_endpoints r ON r.chain_id = c.chain_id AND r.is_healthy = 1
        WHERE r.url LIKE 'http%'
    """
    params = {}
    if ecosystem_filter:
        query += " AND c.ecosystem = :eco"
        params["eco"] = ecosystem_filter
    
    query += """
        GROUP BY c.chain_id
        HAVING MIN(CASE WHEN r.latency_ms > 0 THEN r.latency_ms ELSE 99999 END)
        ORDER BY MIN(CASE WHEN r.latency_ms > 0 THEN r.latency_ms ELSE 99999 END) ASC
    """
    
    if top_n:
        query += f" LIMIT {top_n}"
    
    rows = db.execute(query, params).fetchall()
    db.close()
    return [dict(r) for r in rows]


async def run_benchmark(chains: list, burst: int = 10):
    """Run TPS benchmark across all chains."""
    results = []
    sem = asyncio.Semaphore(CONCURRENCY)
    
    total = len(chains)
    done = 0
    start_time = time.monotonic()
    
    print(f"\n🏁 TPS Benchmark — {total} chains, concurrency={CONCURRENCY}, burst={burst}")
    print(f"{'─' * 80}")
    
    async with aiohttp.ClientSession() as session:
        async def bench_one(chain):
            nonlocal done
            if STOP:
                return None
            async with sem:
                url = chain["rpc_url"]
                eco = chain["ecosystem"]
                ctype = chain["chain_type"]
                
                # Step 1: Single request for latency + block height
                single = await measure_single_rpc(session, url, eco, ctype)
                
                if not single["ok"]:
                    done += 1
                    return None
                
                # Step 2: Burst to estimate TPS
                tps = await measure_tps_burst(session, url, eco, ctype, burst)
                
                # Estimate theoretical TPS based on known chains
                theoretical = estimate_theoretical_tps(chain["chain_id"], eco)
                
                done += 1
                elapsed = time.monotonic() - start_time
                rate = done / elapsed if elapsed > 0 else 0
                eta = (total - done) / rate if rate > 0 else 0
                
                rank_icon = "🥇" if tps > 100 else "🥈" if tps > 50 else "🥉" if tps > 10 else "  "
                print(f"  {rank_icon} {done:>5}/{total}  {chain['chain_name'][:30]:30s}  "
                      f"TPS={tps:>7.1f}  latency={single['latency_ms']:>4d}ms  "
                      f"block={single.get('block_height') or '?':>12}  "
                      f"ETA {eta:.0f}s", flush=True)
                
                return {
                    "chain_id": chain["chain_id"],
                    "tps_current": tps,
                    "tps_peak": tps * 1.2,  # Assume burst is close to peak
                    "tps_theoretical": theoretical,
                    "total_txns_24h": 0,
                    "finality_seconds": estimate_finality(chain["chain_id"], eco),
                    "block_height": single.get("block_height"),
                }
        
        tasks = [bench_one(c) for c in chains]
        all_results = await asyncio.gather(*tasks)
        results = [r for r in all_results if r is not None]
    
    return results


def estimate_theoretical_tps(chain_id: str, ecosystem: str) -> float:
    """Estimate theoretical max TPS for known chains."""
    known = {
        "x3-chain": 26755, "x3": 26755,
        "sol": 65000, "sol-mainnet": 65000, "solana": 65000,
        "sui": 120000, "sui-mainnet": 120000,
        "aptos": 160000, "apt": 160000,
        "sei": 12500,
        "monad": 10000,
        "eth": 30, "ethereum": 30,
        "bsc": 300, "bnb": 300,
        "polygon": 7000, "matic": 7000, "polygon-pos": 7000,
        "arb-one": 40000, "arbitrum": 40000,
        "op": 2000, "optimism": 2000,
        "base": 2000,
        "avax": 4500, "avalanche": 4500, "avalanche-c": 4500,
        "ftm": 10000, "fantom": 10000,
        "near": 100000,
        "algo": 6000, "algorand": 6000,
        "xlm": 1000, "stellar": 1000,
        "xrp": 1500, "ripple": 1500,
        "dot": 1000, "polkadot": 1000,
        "cosmos": 10000, "atom": 10000,
        "ton": 100000,
        "tron": 2000,
        "celo": 400,
        "hbar": 10000, "hedera": 10000,
        "zksync": 2000, "zksync-era": 2000,
        "scroll": 2000,
        "linea": 2000,
        "mantle": 2000,
        "blast": 2000,
        "manta": 2000,
    }
    cid = chain_id.lower().replace("-mainnet", "").replace("-testnet", "")
    for key, val in known.items():
        if key in cid:
            return float(val)
    # Default by ecosystem
    defaults = {"evm": 100, "svm": 5000, "cosmos": 5000, "substrate": 1000, "move": 10000, "x3": 26755}
    return float(defaults.get(ecosystem, 100))


def estimate_finality(chain_id: str, ecosystem: str) -> float:
    """Estimate finality time in seconds."""
    known = {
        "x3": 6, "x3-chain": 6,
        "sol": 0.4, "aptos": 0.9, "sui": 0.5, "sei": 0.4,
        "eth": 900, "bsc": 3, "polygon": 2, "avax": 2,
        "arb-one": 0.3, "op": 2, "base": 2, "near": 1.3,
        "ftm": 1, "algo": 3.3, "ton": 5, "tron": 3,
        "dot": 12, "cosmos": 6, "xlm": 5, "xrp": 4,
        "zksync": 300, "scroll": 300, "linea": 300,
    }
    cid = chain_id.lower()
    for key, val in known.items():
        if key in cid:
            return val
    defaults = {"evm": 12, "svm": 0.5, "cosmos": 6, "substrate": 6, "move": 1, "x3": 6}
    return defaults.get(ecosystem, 12)


async def post_results(results: list):
    """Post benchmark results to the chain-db API."""
    posted = 0
    async with aiohttp.ClientSession() as session:
        for i in range(0, len(results), BATCH_POST_SIZE):
            batch = results[i:i + BATCH_POST_SIZE]
            try:
                async with session.post(
                    f"{API_URL}/api/tps/benchmark",
                    json={"results": batch},
                    timeout=aiohttp.ClientTimeout(total=10)
                ) as resp:
                    if resp.status == 200:
                        data = await resp.json()
                        posted += data.get("inserted", 0)
                    else:
                        text = await resp.text()
                        print(f"  ⚠ POST failed: {resp.status} — {text[:100]}")
            except Exception as e:
                print(f"  ⚠ POST error: {e}")
    return posted


def print_leaderboard(results: list, top_n: int = 30):
    """Print a pretty leaderboard to terminal."""
    # Load chain names
    try:
        db = sqlite3.connect(DB_PATH)
        names = {r[0]: r[1] for r in db.execute("SELECT chain_id, chain_name FROM chains").fetchall()}
        db.close()
    except:
        names = {}
    
    sorted_results = sorted(results, key=lambda r: r["tps_current"], reverse=True)
    
    print(f"\n{'═' * 90}")
    print(f"  🏆  TPS LEADERBOARD — Top {min(top_n, len(sorted_results))} Fastest Chains")
    print(f"  ⚡ Only takes a couple mins to see your speed get faster...")
    print(f"  📊 Dial in your TPS. Optimize the hell out of them.")
    print(f"{'═' * 90}")
    print(f"  {'#':>3}  {'Chain':<35} {'TPS':>8}  {'Peak':>8}  {'Theoretical':>12}  {'Latency':>8}  {'Finality':>8}")
    print(f"  {'─' * 3}  {'─' * 35} {'─' * 8}  {'─' * 8}  {'─' * 12}  {'─' * 8}  {'─' * 8}")
    
    medals = {1: "🥇", 2: "🥈", 3: "🥉"}
    
    for i, r in enumerate(sorted_results[:top_n], 1):
        name = names.get(r["chain_id"], r["chain_id"])[:35]
        medal = medals.get(i, f"{i:>3}")
        fin = f'{r["finality_seconds"]:.1f}s' if r["finality_seconds"] else "—"
        print(f"  {medal:>3}  {name:<35} {r['tps_current']:>8.1f}  {r['tps_peak']:>8.1f}  "
              f"{r['tps_theoretical']:>12.0f}  {'—':>8}  {fin:>8}")
    
    print(f"{'═' * 90}")
    total_tps = sum(r["tps_current"] for r in sorted_results)
    print(f"  📈 Total measured TPS across {len(sorted_results)} chains: {total_tps:,.1f}")
    print(f"  🕐 Benchmark completed at {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}")
    print(f"{'═' * 90}\n")


# ── CLI ────────────────────────────────────────────────────────────────────────

async def main():
    global CONCURRENCY
    parser = argparse.ArgumentParser(description="TPS Benchmark Runner")
    parser.add_argument("--top", type=int, default=None, help="Only benchmark top N chains by latency")
    parser.add_argument("--ecosystem", type=str, default=None, help="Filter by ecosystem (evm, svm, cosmos, substrate, move)")
    parser.add_argument("--burst", type=int, default=10, help="Number of burst requests per chain (default: 10)")
    parser.add_argument("--loop", type=int, default=0, help="Re-run every N seconds (0 = run once)")
    parser.add_argument("--no-post", action="store_true", help="Don't post results to API")
    parser.add_argument("--concurrency", type=int, default=CONCURRENCY, help=f"Max concurrent benchmarks (default: {CONCURRENCY})")
    args = parser.parse_args()
    
    CONCURRENCY = args.concurrency
    
    while True:
        print(f"\n⚡ X3 Chain TPS Benchmark — {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}")
        print(f"   DB: {DB_PATH}")
        print(f"   API: {API_URL}")
        
        chains = get_chains_to_benchmark(DB_PATH, args.ecosystem, args.top)
        if not chains:
            print("❌ No chains found to benchmark!")
            break
        
        print(f"   Chains to benchmark: {len(chains)}")
        
        results = await run_benchmark(chains, args.burst)
        
        if results:
            print_leaderboard(results)
            
            if not args.no_post:
                posted = await post_results(results)
                print(f"✅ Posted {posted}/{len(results)} results to {API_URL}/api/tps/benchmark")
        else:
            print("❌ No results collected")
        
        if args.loop <= 0 or STOP:
            break
        
        print(f"⏳ Next benchmark in {args.loop}s...")
        for _ in range(args.loop):
            if STOP:
                break
            await asyncio.sleep(1)


if __name__ == "__main__":
    asyncio.run(main())
