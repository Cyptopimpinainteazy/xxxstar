#!/usr/bin/env python3
"""rpc_crawler_daemon.py — Persistent RPC discovery daemon.

Continuously discovers, validates, and seeds public RPC endpoints into
the X3 Chain chain database. Survives restarts by saving crawl state
to a JSON checkpoint file.

Sources:
  - Hardcoded mega registry (500+ known public RPCs)
  - Pattern-based URL generation (9 provider templates × 60+ chain slugs)
  - Google dorks for undiscovered RPC endpoints
  - Exchange API endpoints (Binance, Coinbase, Kraken, etc.)
  - Gas station / fee oracle endpoints
  - DEX aggregator RPCs (1inch, Paraswap, 0x, etc.)
  - Block explorer API endpoints
  - Bridge / cross-chain RPCs
  - Testnet faucet-adjacent RPCs
  - GitHub awesome-list scraping
  - chainlist.org / DefiLlama chain registry

Features:
  - Graceful shutdown (SIGTERM / SIGINT) — saves state before exit
  - Checkpoint file: resumes exactly where it left off
  - Per-domain rate limiting (3 req/s default)
  - Async validation with configurable concurrency
  - Auto-ban dead endpoints, auto-recover on next cycle
  - Logs to file + stdout
  - Runs as background daemon from start-all.sh

Usage:
  python3 crawler_daemon.py                 # run forever (default 15min cycles)
  python3 crawler_daemon.py --once          # single crawl cycle then exit
  python3 crawler_daemon.py --interval 5    # crawl every 5 minutes
  python3 crawler_daemon.py --aggressive    # enable Google dorks + deep scraping
"""

from __future__ import annotations

import argparse
import asyncio
import json
import logging
import os
import re
import signal
import sqlite3
import sys
import time
import hashlib
from collections import defaultdict
from dataclasses import dataclass, field, asdict
from datetime import datetime, timezone
from pathlib import Path
from typing import Optional
from urllib.parse import urlparse, quote_plus

# ── Optional deps ─────────────────────────────────────────────────────
try:
    import aiohttp
    HAS_AIOHTTP = True
except ImportError:
    HAS_AIOHTTP = False

try:
    import urllib.request
    import urllib.error
    import ssl
except ImportError:
    pass

# ── Paths ─────────────────────────────────────────────────────────────
SCRIPT_DIR = Path(__file__).resolve().parent
DB_PATH = os.environ.get(
    "CHAIN_DB_PATH",
    str(SCRIPT_DIR.parent.parent / "db" / "chains.db"),
)
STATE_FILE = os.environ.get(
    "CRAWLER_STATE_FILE",
    str(SCRIPT_DIR / "crawler_state.json"),
)
LOG_FILE = os.environ.get(
    "CRAWLER_LOG_FILE",
    str(SCRIPT_DIR / "crawler.log"),
)

# ── Logging ───────────────────────────────────────────────────────────
def setup_logging():
    fmt = "%(asctime)s [%(levelname)s] %(message)s"
    handlers = [
        logging.StreamHandler(sys.stdout),
        logging.FileHandler(LOG_FILE, mode="a", encoding="utf-8"),
    ]
    logging.basicConfig(level=logging.INFO, format=fmt, handlers=handlers)
    return logging.getLogger("rpc-crawler")

log = setup_logging()


# ═════════════════════════════════════════════════════════════════════════
# DATA STRUCTURES
# ═════════════════════════════════════════════════════════════════════════

@dataclass
class RPCEndpoint:
    chain_id: str
    url: str
    protocol: str = "https"
    provider: str = "unknown"
    rate_limit_rps: int = 10
    tier: str = "public"
    latency_ms: Optional[float] = None
    is_healthy: bool = True
    chain_type: str = "mainnet"  # mainnet, testnet, devnet
    source: str = "registry"    # registry, pattern, google, exchange, gas, dex, explorer, bridge, github, chainlist
    ws_url: Optional[str] = None

    def __hash__(self):
        return hash((self.chain_id, self.url.rstrip("/")))

    def __eq__(self, other):
        return self.chain_id == other.chain_id and self.url.rstrip("/") == other.url.rstrip("/")

    def to_dict(self):
        return {
            "chain_id": self.chain_id,
            "url": self.url,
            "protocol": self.protocol,
            "provider": self.provider,
            "rate_limit_rps": self.rate_limit_rps,
            "tier": self.tier,
            "latency_ms": self.latency_ms,
            "is_healthy": self.is_healthy,
            "chain_type": self.chain_type,
            "source": self.source,
        }


@dataclass
class CrawlerState:
    """Persistent state saved to disk."""
    cycle_count: int = 0
    total_discovered: int = 0
    total_healthy: int = 0
    total_dead: int = 0
    last_cycle_time: str = ""
    last_google_dork_page: int = 0
    google_dork_index: int = 0
    discovered_urls: list = field(default_factory=list)  # URLs found this session
    banned_urls: list = field(default_factory=list)       # URLs that failed 3+ times
    fail_counts: dict = field(default_factory=dict)       # url -> consecutive fail count
    chains_crawled: list = field(default_factory=list)    # which chains we've pattern-scanned
    github_repos_scraped: list = field(default_factory=list)
    running: bool = True

    def save(self, path: str):
        with open(path, "w") as f:
            json.dump(asdict(self), f, indent=2, default=str)

    @classmethod
    def load(cls, path: str) -> "CrawlerState":
        if os.path.exists(path):
            try:
                with open(path) as f:
                    data = json.load(f)
                return cls(**{k: v for k, v in data.items() if k in cls.__dataclass_fields__})
            except Exception as e:
                log.warning(f"Could not load state from {path}: {e}")
        return cls()


# ═════════════════════════════════════════════════════════════════════════
# RATE LIMITER
# ═════════════════════════════════════════════════════════════════════════

class DomainRateLimiter:
    def __init__(self, max_per_second: float = 3.0):
        self.max_per_second = max_per_second
        self.min_interval = 1.0 / max_per_second
        self._last: dict[str, float] = {}
        self._lock = asyncio.Lock()

    def _domain(self, url: str) -> str:
        return urlparse(url).netloc

    async def wait(self, url: str):
        domain = self._domain(url)
        async with self._lock:
            now = time.monotonic()
            last = self._last.get(domain, 0)
            wait_time = self.min_interval - (now - last)
            if wait_time > 0:
                await asyncio.sleep(wait_time)
            self._last[domain] = time.monotonic()


# ═════════════════════════════════════════════════════════════════════════
# MEGA RPC REGISTRY — 500+ hardcoded known public endpoints
# ═════════════════════════════════════════════════════════════════════════

def build_mega_registry() -> list[RPCEndpoint]:
    """All known public RPC endpoints from prior scraping."""
    rpc = RPCEndpoint
    eps: list[RPCEndpoint] = []

    # ── ETHEREUM MAINNET ──────────────────────────────────────────────
    eps.extend([
        rpc("eth", "https://eth.llamarpc.com", "https", "llamarpc", 40, "public", chain_type="mainnet"),
        rpc("eth", "https://rpc.ankr.com/eth", "https", "ankr", 30, "public", chain_type="mainnet"),
        rpc("eth", "https://ethereum-rpc.publicnode.com", "https", "publicnode", 25, "public", chain_type="mainnet"),
        rpc("eth", "https://ethereum.publicnode.com", "https", "publicnode", 25, "public", chain_type="mainnet"),
        rpc("eth", "https://1rpc.io/eth", "https", "1rpc", 15, "public", chain_type="mainnet"),
        rpc("eth", "https://rpc.mevblocker.io", "https", "mevblocker", 20, "public", chain_type="mainnet"),
        rpc("eth", "https://eth.drpc.org", "https", "drpc", 25, "public", chain_type="mainnet"),
        rpc("eth", "https://rpc.flashbots.net", "https", "flashbots", 15, "public", chain_type="mainnet"),
        rpc("eth", "https://api.securerpc.com/v1", "https", "securerpc", 10, "public", chain_type="mainnet"),
        rpc("eth", "https://eth.merkle.io", "https", "merkle", 20, "public", chain_type="mainnet"),
        rpc("eth", "https://eth-mainnet.public.blastapi.io", "https", "blast", 20, "public", chain_type="mainnet"),
        rpc("eth", "https://cloudflare-eth.com", "https", "cloudflare", 40, "public", chain_type="mainnet"),
        rpc("eth", "https://ethereum.blockpi.network/v1/rpc/public", "https", "blockpi", 10, "public", chain_type="mainnet"),
        rpc("eth", "https://eth-mainnet.rpcfast.com", "https", "rpcfast", 15, "public", chain_type="mainnet"),
        rpc("eth", "https://rpc.builder0x69.io", "https", "builder0x69", 10, "public", chain_type="mainnet"),
        rpc("eth", "https://rpc.eth.gateway.fm", "https", "gateway", 15, "public", chain_type="mainnet"),
        rpc("eth", "https://virginia.rpc.blxrbdn.com", "https", "bloxroute", 15, "public", chain_type="mainnet"),
        rpc("eth", "https://uk.rpc.blxrbdn.com", "https", "bloxroute-uk", 15, "public", chain_type="mainnet"),
        rpc("eth", "https://singapore.rpc.blxrbdn.com", "https", "bloxroute-sg", 15, "public", chain_type="mainnet"),
        rpc("eth", "https://eth.meowrpc.com", "https", "meowrpc", 15, "public", chain_type="mainnet"),
    ])

    # ── BSC ────────────────────────────────────────────────────────────
    eps.extend([
        rpc("bsc", "https://bsc-dataseed.binance.org", "https", "binance", 50, "public", chain_type="mainnet"),
        rpc("bsc", "https://bsc-dataseed1.binance.org", "https", "binance", 50, "public", chain_type="mainnet"),
        rpc("bsc", "https://bsc-dataseed2.binance.org", "https", "binance", 50, "public", chain_type="mainnet"),
        rpc("bsc", "https://bsc-dataseed3.binance.org", "https", "binance", 50, "public", chain_type="mainnet"),
        rpc("bsc", "https://bsc-dataseed4.binance.org", "https", "binance", 50, "public", chain_type="mainnet"),
        rpc("bsc", "https://bsc-dataseed1.defibit.io", "https", "defibit", 30, "public", chain_type="mainnet"),
        rpc("bsc", "https://bsc-dataseed1.ninicoin.io", "https", "ninicoin", 30, "public", chain_type="mainnet"),
        rpc("bsc", "https://rpc.ankr.com/bsc", "https", "ankr", 30, "public", chain_type="mainnet"),
        rpc("bsc", "https://bsc.publicnode.com", "https", "publicnode", 25, "public", chain_type="mainnet"),
        rpc("bsc", "https://bsc-rpc.publicnode.com", "https", "publicnode", 25, "public", chain_type="mainnet"),
        rpc("bsc", "https://bsc.drpc.org", "https", "drpc", 25, "public", chain_type="mainnet"),
        rpc("bsc", "https://bsc-mainnet.public.blastapi.io", "https", "blast", 20, "public", chain_type="mainnet"),
        rpc("bsc", "https://bsc.meowrpc.com", "https", "meowrpc", 15, "public", chain_type="mainnet"),
        rpc("bsc", "https://1rpc.io/bnb", "https", "1rpc", 15, "public", chain_type="mainnet"),
        rpc("bsc", "https://bsc.blockpi.network/v1/rpc/public", "https", "blockpi", 10, "public", chain_type="mainnet"),
        rpc("bsc", "https://bsc.llamarpc.com", "https", "llamarpc", 30, "public", chain_type="mainnet"),
    ])

    # ── POLYGON ────────────────────────────────────────────────────────
    eps.extend([
        rpc("polygon", "https://polygon-rpc.com", "https", "polygon", 50, "public", chain_type="mainnet"),
        rpc("polygon", "https://rpc-mainnet.matic.network", "https", "matic", 30, "public", chain_type="mainnet"),
        rpc("polygon", "https://rpc-mainnet.maticvigil.com", "https", "maticvigil", 20, "public", chain_type="mainnet"),
        rpc("polygon", "https://rpc.ankr.com/polygon", "https", "ankr", 30, "public", chain_type="mainnet"),
        rpc("polygon", "https://polygon.publicnode.com", "https", "publicnode", 25, "public", chain_type="mainnet"),
        rpc("polygon", "https://polygon.drpc.org", "https", "drpc", 25, "public", chain_type="mainnet"),
        rpc("polygon", "https://polygon-mainnet.public.blastapi.io", "https", "blast", 20, "public", chain_type="mainnet"),
        rpc("polygon", "https://polygon.meowrpc.com", "https", "meowrpc", 15, "public", chain_type="mainnet"),
        rpc("polygon", "https://1rpc.io/matic", "https", "1rpc", 15, "public", chain_type="mainnet"),
        rpc("polygon", "https://polygon.blockpi.network/v1/rpc/public", "https", "blockpi", 10, "public", chain_type="mainnet"),
        rpc("polygon", "https://polygon.llamarpc.com", "https", "llamarpc", 30, "public", chain_type="mainnet"),
        rpc("polygon", "https://polygon.api.onfinality.io/public", "https", "onfinality", 10, "public", chain_type="mainnet"),
    ])

    # ── ARBITRUM ───────────────────────────────────────────────────────
    eps.extend([
        rpc("arb-one", "https://arb1.arbitrum.io/rpc", "https", "offchain-labs", 40, "public", chain_type="mainnet"),
        rpc("arb-one", "https://rpc.ankr.com/arbitrum", "https", "ankr", 30, "public", chain_type="mainnet"),
        rpc("arb-one", "https://arbitrum.publicnode.com", "https", "publicnode", 25, "public", chain_type="mainnet"),
        rpc("arb-one", "https://arbitrum.drpc.org", "https", "drpc", 25, "public", chain_type="mainnet"),
        rpc("arb-one", "https://arb-mainnet.public.blastapi.io", "https", "blast", 20, "public", chain_type="mainnet"),
        rpc("arb-one", "https://arbitrum.meowrpc.com", "https", "meowrpc", 15, "public", chain_type="mainnet"),
        rpc("arb-one", "https://1rpc.io/arb", "https", "1rpc", 15, "public", chain_type="mainnet"),
        rpc("arb-one", "https://arbitrum.blockpi.network/v1/rpc/public", "https", "blockpi", 10, "public", chain_type="mainnet"),
        rpc("arb-one", "https://arbitrum.llamarpc.com", "https", "llamarpc", 30, "public", chain_type="mainnet"),
        rpc("arb-one", "https://arbitrum.api.onfinality.io/public", "https", "onfinality", 10, "public", chain_type="mainnet"),
    ])

    # ── OPTIMISM ───────────────────────────────────────────────────────
    eps.extend([
        rpc("optimism", "https://mainnet.optimism.io", "https", "optimism", 40, "public", chain_type="mainnet"),
        rpc("optimism", "https://rpc.ankr.com/optimism", "https", "ankr", 30, "public", chain_type="mainnet"),
        rpc("optimism", "https://optimism.publicnode.com", "https", "publicnode", 25, "public", chain_type="mainnet"),
        rpc("optimism", "https://optimism.drpc.org", "https", "drpc", 25, "public", chain_type="mainnet"),
        rpc("optimism", "https://optimism-mainnet.public.blastapi.io", "https", "blast", 20, "public", chain_type="mainnet"),
        rpc("optimism", "https://optimism.meowrpc.com", "https", "meowrpc", 15, "public", chain_type="mainnet"),
        rpc("optimism", "https://1rpc.io/op", "https", "1rpc", 15, "public", chain_type="mainnet"),
        rpc("optimism", "https://optimism.blockpi.network/v1/rpc/public", "https", "blockpi", 10, "public", chain_type="mainnet"),
        rpc("optimism", "https://optimism.llamarpc.com", "https", "llamarpc", 30, "public", chain_type="mainnet"),
    ])

    # ── AVALANCHE ──────────────────────────────────────────────────────
    eps.extend([
        rpc("avax", "https://api.avax.network/ext/bc/C/rpc", "https", "avalabs", 40, "public", chain_type="mainnet"),
        rpc("avax", "https://rpc.ankr.com/avalanche", "https", "ankr", 30, "public", chain_type="mainnet"),
        rpc("avax", "https://avalanche.publicnode.com", "https", "publicnode", 25, "public", chain_type="mainnet"),
        rpc("avax", "https://avalanche.drpc.org", "https", "drpc", 25, "public", chain_type="mainnet"),
        rpc("avax", "https://avax-mainnet.public.blastapi.io", "https", "blast", 20, "public", chain_type="mainnet"),
        rpc("avax", "https://1rpc.io/avax/c", "https", "1rpc", 15, "public", chain_type="mainnet"),
        rpc("avax", "https://avax.meowrpc.com", "https", "meowrpc", 15, "public", chain_type="mainnet"),
        rpc("avax", "https://avalanche.blockpi.network/v1/rpc/public", "https", "blockpi", 10, "public", chain_type="mainnet"),
    ])

    # ── BASE ───────────────────────────────────────────────────────────
    eps.extend([
        rpc("base", "https://mainnet.base.org", "https", "coinbase", 40, "public", chain_type="mainnet"),
        rpc("base", "https://base.publicnode.com", "https", "publicnode", 25, "public", chain_type="mainnet"),
        rpc("base", "https://rpc.ankr.com/base", "https", "ankr", 30, "public", chain_type="mainnet"),
        rpc("base", "https://base.drpc.org", "https", "drpc", 25, "public", chain_type="mainnet"),
        rpc("base", "https://base.meowrpc.com", "https", "meowrpc", 15, "public", chain_type="mainnet"),
        rpc("base", "https://1rpc.io/base", "https", "1rpc", 15, "public", chain_type="mainnet"),
        rpc("base", "https://base-mainnet.public.blastapi.io", "https", "blast", 20, "public", chain_type="mainnet"),
        rpc("base", "https://base.blockpi.network/v1/rpc/public", "https", "blockpi", 10, "public", chain_type="mainnet"),
        rpc("base", "https://base.llamarpc.com", "https", "llamarpc", 40, "public", chain_type="mainnet"),
    ])

    # ── FANTOM ─────────────────────────────────────────────────────────
    eps.extend([
        rpc("ftm", "https://rpc.ftm.tools", "https", "fantom", 40, "public", chain_type="mainnet"),
        rpc("ftm", "https://rpc.ankr.com/fantom", "https", "ankr", 30, "public", chain_type="mainnet"),
        rpc("ftm", "https://fantom.publicnode.com", "https", "publicnode", 25, "public", chain_type="mainnet"),
        rpc("ftm", "https://fantom.drpc.org", "https", "drpc", 25, "public", chain_type="mainnet"),
        rpc("ftm", "https://1rpc.io/ftm", "https", "1rpc", 15, "public", chain_type="mainnet"),
        rpc("ftm", "https://fantom.blockpi.network/v1/rpc/public", "https", "blockpi", 10, "public", chain_type="mainnet"),
    ])

    # ── GNOSIS ─────────────────────────────────────────────────────────
    eps.extend([
        rpc("gnosis", "https://rpc.gnosischain.com", "https", "gnosis", 40, "public", chain_type="mainnet"),
        rpc("gnosis", "https://rpc.ankr.com/gnosis", "https", "ankr", 30, "public", chain_type="mainnet"),
        rpc("gnosis", "https://gnosis.publicnode.com", "https", "publicnode", 25, "public", chain_type="mainnet"),
        rpc("gnosis", "https://gnosis.drpc.org", "https", "drpc", 25, "public", chain_type="mainnet"),
        rpc("gnosis", "https://1rpc.io/gnosis", "https", "1rpc", 15, "public", chain_type="mainnet"),
        rpc("gnosis", "https://gnosis.blockpi.network/v1/rpc/public", "https", "blockpi", 10, "public", chain_type="mainnet"),
    ])

    # ── SOLANA ─────────────────────────────────────────────────────────
    eps.extend([
        rpc("sol", "https://api.mainnet-beta.solana.com", "https", "solana-labs", 10, "public", chain_type="mainnet"),
        rpc("sol", "https://rpc.ankr.com/solana", "https", "ankr", 20, "public", chain_type="mainnet"),
        rpc("sol", "https://solana.publicnode.com", "https", "publicnode", 15, "public", chain_type="mainnet"),
        rpc("sol", "https://solana.drpc.org", "https", "drpc", 15, "public", chain_type="mainnet"),
        rpc("sol", "https://solana.blockpi.network/v1/rpc/public", "https", "blockpi", 10, "public", chain_type="mainnet"),
        rpc("sol", "https://solana-mainnet.public.blastapi.io", "https", "blast", 15, "public", chain_type="mainnet"),
    ])

    # ── L2s ────────────────────────────────────────────────────────────
    l2_chains = {
        "linea": [("https://rpc.linea.build", "consensys", 30), ("https://linea.drpc.org", "drpc", 25),
                  ("https://linea.publicnode.com", "publicnode", 20), ("https://rpc.ankr.com/linea", "ankr", 20),
                  ("https://1rpc.io/linea", "1rpc", 15), ("https://linea.blockpi.network/v1/rpc/public", "blockpi", 10)],
        "scroll": [("https://rpc.scroll.io", "scroll", 30), ("https://scroll.drpc.org", "drpc", 25),
                   ("https://scroll.publicnode.com", "publicnode", 20), ("https://rpc.ankr.com/scroll", "ankr", 20),
                   ("https://1rpc.io/scroll", "1rpc", 15)],
        "zksync": [("https://mainnet.era.zksync.io", "zksync", 30), ("https://zksync.drpc.org", "drpc", 25),
                   ("https://rpc.ankr.com/zksync_era", "ankr", 20), ("https://1rpc.io/zksync2-era", "1rpc", 15),
                   ("https://zksync.meowrpc.com", "meowrpc", 15)],
        "mantle": [("https://rpc.mantle.xyz", "mantle", 30), ("https://rpc.ankr.com/mantle", "ankr", 20),
                   ("https://mantle.publicnode.com", "publicnode", 20), ("https://mantle.drpc.org", "drpc", 25),
                   ("https://1rpc.io/mantle", "1rpc", 15)],
        "blast": [("https://rpc.blast.io", "blast-l2", 30), ("https://blast.publicnode.com", "publicnode", 20),
                  ("https://rpc.ankr.com/blast", "ankr", 20), ("https://blast.drpc.org", "drpc", 25)],
        "mode": [("https://mainnet.mode.network", "mode", 30), ("https://mode.drpc.org", "drpc", 25),
                 ("https://1rpc.io/mode", "1rpc", 15)],
        "manta": [("https://pacific-rpc.manta.network/http", "manta", 30),
                  ("https://manta-pacific.drpc.org", "drpc", 25), ("https://1rpc.io/manta", "1rpc", 15)],
    }
    for chain_id, chain_rpcs in l2_chains.items():
        for url, provider, rps in chain_rpcs:
            eps.append(rpc(chain_id, url, "https", provider, rps, "public", chain_type="mainnet"))

    # ── ALT L1s ────────────────────────────────────────────────────────
    alt_l1s = {
        "celo": [("https://forno.celo.org", "clabs", 30), ("https://rpc.ankr.com/celo", "ankr", 20),
                 ("https://celo.drpc.org", "drpc", 25), ("https://1rpc.io/celo", "1rpc", 15)],
        "moonbeam": [("https://rpc.api.moonbeam.network", "moonbeam", 30), ("https://rpc.ankr.com/moonbeam", "ankr", 20),
                     ("https://moonbeam.drpc.org", "drpc", 25), ("https://moonbeam.publicnode.com", "publicnode", 20)],
        "moonriver": [("https://rpc.api.moonriver.moonbeam.network", "moonbeam", 30),
                      ("https://moonriver.publicnode.com", "publicnode", 20), ("https://moonriver.drpc.org", "drpc", 25)],
        "cronos": [("https://evm.cronos.org", "cronos", 30), ("https://cronos.drpc.org", "drpc", 25),
                   ("https://rpc.ankr.com/cronos", "ankr", 20), ("https://1rpc.io/cro", "1rpc", 15)],
        "aurora": [("https://mainnet.aurora.dev", "aurora", 30), ("https://aurora.drpc.org", "drpc", 25),
                   ("https://1rpc.io/aurora", "1rpc", 15)],
        "harmony": [("https://api.harmony.one", "harmony", 20), ("https://harmony.drpc.org", "drpc", 25),
                    ("https://rpc.ankr.com/harmony", "ankr", 20)],
        "metis": [("https://andromeda.metis.io/?owner=1088", "metis", 30), ("https://metis.drpc.org", "drpc", 25),
                  ("https://rpc.ankr.com/metis", "ankr", 20)],
        "kava": [("https://evm.kava.io", "kava", 30), ("https://kava.drpc.org", "drpc", 25),
                 ("https://rpc.ankr.com/kava_evm", "ankr", 20)],
        "polygon-zkevm": [("https://zkevm-rpc.com", "polygon", 30), ("https://polygon-zkevm.drpc.org", "drpc", 25),
                          ("https://rpc.ankr.com/polygon_zkevm", "ankr", 20), ("https://1rpc.io/polygon/zkevm", "1rpc", 15)],
    }
    for chain_id, chain_rpcs in alt_l1s.items():
        for url, provider, rps in chain_rpcs:
            eps.append(rpc(chain_id, url, "https", provider, rps, "public", chain_type="mainnet"))

    # ── COSMOS ECOSYSTEM ───────────────────────────────────────────────
    cosmos_chains = {
        "cosmos": [("https://cosmos-rpc.publicnode.com", "publicnode", 20), ("https://rpc.ankr.com/cosmos", "ankr", 20)],
        "osmosis": [("https://osmosis-rpc.publicnode.com", "publicnode", 20), ("https://rpc.ankr.com/osmosis", "ankr", 20)],
        "injective": [("https://injective-rpc.publicnode.com", "publicnode", 20)],
        "sei": [("https://sei-evm-rpc.publicnode.com", "publicnode", 20), ("https://sei.drpc.org", "drpc", 25)],
        "celestia": [("https://celestia-rpc.publicnode.com", "publicnode", 20), ("https://celestia.drpc.org", "drpc", 25)],
        "dydx": [("https://dydx-rpc.publicnode.com", "publicnode", 20)],
        "terra": [("https://terra-rpc.publicnode.com", "publicnode", 20)],
    }
    for chain_id, chain_rpcs in cosmos_chains.items():
        for url, provider, rps in chain_rpcs:
            eps.append(rpc(chain_id, url, "https", provider, rps, "public", chain_type="mainnet"))

    # ── MOVE CHAINS ────────────────────────────────────────────────────
    eps.extend([
        rpc("aptos", "https://fullnode.mainnet.aptoslabs.com/v1", "https", "aptoslabs", 30, "public", chain_type="mainnet"),
        rpc("aptos", "https://aptos.publicnode.com", "https", "publicnode", 20, "public", chain_type="mainnet"),
        rpc("aptos", "https://rpc.ankr.com/aptos", "https", "ankr", 20, "public", chain_type="mainnet"),
        rpc("sui", "https://fullnode.mainnet.sui.io", "https", "mysten-labs", 20, "public", chain_type="mainnet"),
        rpc("sui", "https://sui.publicnode.com", "https", "publicnode", 20, "public", chain_type="mainnet"),
        rpc("sui", "https://rpc.ankr.com/sui", "https", "ankr", 20, "public", chain_type="mainnet"),
        rpc("sui", "https://sui.drpc.org", "https", "drpc", 25, "public", chain_type="mainnet"),
    ])

    # ── SUBSTRATE ──────────────────────────────────────────────────────
    eps.extend([
        rpc("polkadot", "https://polkadot.api.onfinality.io/public", "https", "onfinality", 10, "public", chain_type="mainnet"),
        rpc("polkadot", "https://polkadot.publicnode.com", "https", "publicnode", 20, "public", chain_type="mainnet"),
        rpc("polkadot", "https://rpc.ankr.com/polkadot", "https", "ankr", 20, "public", chain_type="mainnet"),
        rpc("polkadot", "https://polkadot.drpc.org", "https", "drpc", 25, "public", chain_type="mainnet"),
        rpc("kusama", "https://kusama.api.onfinality.io/public", "https", "onfinality", 10, "public", chain_type="mainnet"),
        rpc("kusama", "https://kusama.publicnode.com", "https", "publicnode", 20, "public", chain_type="mainnet"),
        rpc("kusama", "https://rpc.ankr.com/kusama", "https", "ankr", 20, "public", chain_type="mainnet"),
    ])

    # ── OTHER ──────────────────────────────────────────────────────────
    eps.extend([
        rpc("near", "https://rpc.mainnet.near.org", "https", "near", 30, "public", chain_type="mainnet"),
        rpc("near", "https://rpc.ankr.com/near", "https", "ankr", 20, "public", chain_type="mainnet"),
        rpc("near", "https://near.drpc.org", "https", "drpc", 25, "public", chain_type="mainnet"),
        rpc("tron", "https://api.trongrid.io", "https", "trongrid", 30, "public", chain_type="mainnet"),
        rpc("tron", "https://tron.publicnode.com", "https", "publicnode", 15, "public", chain_type="mainnet"),
        rpc("tron", "https://rpc.ankr.com/tron", "https", "ankr", 20, "public", chain_type="mainnet"),
        rpc("ton", "https://toncenter.com/api/v2/jsonRPC", "https", "toncenter", 10, "public", chain_type="mainnet"),
        rpc("ton", "https://ton.publicnode.com", "https", "publicnode", 15, "public", chain_type="mainnet"),
        rpc("filecoin", "https://api.node.glif.io", "https", "glif", 20, "public", chain_type="mainnet"),
        rpc("filecoin", "https://rpc.ankr.com/filecoin", "https", "ankr", 20, "public", chain_type="mainnet"),
        rpc("filecoin", "https://filecoin.drpc.org", "https", "drpc", 25, "public", chain_type="mainnet"),
    ])

    # ── NEWER CHAINS ───────────────────────────────────────────────────
    newer = {
        "taiko": [("https://rpc.mainnet.taiko.xyz", "taiko", 30), ("https://taiko.publicnode.com", "publicnode", 20),
                  ("https://rpc.ankr.com/taiko", "ankr", 20)],
        "sonic": [("https://sonic.publicnode.com", "publicnode", 20), ("https://rpc.ankr.com/sonic", "ankr", 20)],
        "unichain": [("https://unichain.publicnode.com", "publicnode", 20), ("https://rpc.ankr.com/unichain", "ankr", 20)],
        "fraxtal": [("https://rpc.frax.com", "frax", 30), ("https://fraxtal.publicnode.com", "publicnode", 20)],
        "opbnb": [("https://opbnb.publicnode.com", "publicnode", 25), ("https://opbnb-mainnet-rpc.bnbchain.org", "bnbchain", 30)],
        "berachain": [("https://berachain-rpc.publicnode.com", "publicnode", 20), ("https://berachain.publicnode.com", "publicnode", 20)],
        "chiliz": [("https://rpc.ankr.com/chiliz", "ankr", 20), ("https://chiliz.publicnode.com", "publicnode", 20)],
        "starknet": [("https://starknet.publicnode.com", "publicnode", 20)],
        "pulsechain": [("https://rpc.pulsechain.com", "pulsechain", 30), ("https://pulsechain.publicnode.com", "publicnode", 20)],
        "xlayer": [("https://rpc.xlayer.tech", "xlayer", 25), ("https://xlayer.drpc.org", "drpc", 25)],
        "core": [("https://rpc.coredao.org", "core", 30), ("https://core.drpc.org", "drpc", 25)],
        "flare": [("https://flare-api.flare.network/ext/C/rpc", "flare", 25)],
        "telos": [("https://mainnet.telos.net/evm", "telos", 25)],
        "gravity": [("https://rpc.gravity.xyz", "gravity", 25)],
        "story": [("https://story.drpc.org", "drpc", 25)],
    }
    for chain_id, chain_rpcs in newer.items():
        for url, provider, rps in chain_rpcs:
            eps.append(rpc(chain_id, url, "https", provider, rps, "public", chain_type="mainnet"))

    # ═══════════════════════════════════════════════════════════════════
    # TESTNETS
    # ═══════════════════════════════════════════════════════════════════
    testnets = {
        "sepolia": [("https://rpc.sepolia.org", "ethereum", 20), ("https://ethereum-sepolia-rpc.publicnode.com", "publicnode", 25),
                    ("https://sepolia.drpc.org", "drpc", 25), ("https://1rpc.io/sepolia", "1rpc", 15),
                    ("https://rpc.ankr.com/eth_sepolia", "ankr", 25), ("https://sepolia.gateway.tenderly.co", "tenderly", 20)],
        "holesky": [("https://holesky.drpc.org", "drpc", 25), ("https://rpc.ankr.com/eth_holesky", "ankr", 25),
                    ("https://holesky.publicnode.com", "publicnode", 20)],
        "bsc-testnet": [("https://data-seed-prebsc-1-s1.binance.org:8545", "binance", 20),
                        ("https://bsc-testnet-rpc.publicnode.com", "publicnode", 20),
                        ("https://bsc-testnet.drpc.org", "drpc", 20)],
        "polygon-amoy": [("https://rpc-amoy.polygon.technology", "polygon", 25),
                         ("https://polygon-amoy.drpc.org", "drpc", 25),
                         ("https://rpc.ankr.com/polygon_amoy", "ankr", 20)],
        "arb-sepolia": [("https://sepolia-rollup.arbitrum.io/rpc", "offchain-labs", 25),
                        ("https://arbitrum-sepolia.drpc.org", "drpc", 25)],
        "op-sepolia": [("https://sepolia.optimism.io", "optimism", 25),
                       ("https://optimism-sepolia.drpc.org", "drpc", 25)],
        "base-sepolia": [("https://sepolia.base.org", "coinbase", 25),
                         ("https://base-sepolia.drpc.org", "drpc", 25)],
        "avax-fuji": [("https://api.avax-test.network/ext/bc/C/rpc", "avalabs", 25),
                      ("https://avalanche-fuji.drpc.org", "drpc", 25)],
        "scroll-sepolia": [("https://sepolia-rpc.scroll.io", "scroll", 25)],
        "blast-sepolia": [("https://sepolia.blast.io", "blast-l2", 25)],
        "linea-sepolia": [("https://rpc.sepolia.linea.build", "consensys", 20)],
        "zksync-sepolia": [("https://sepolia.era.zksync.dev", "zksync", 20)],
        "mantle-sepolia": [("https://rpc.sepolia.mantle.xyz", "mantle", 20)],
        "sol-devnet": [("https://api.devnet.solana.com", "solana-labs", 10)],
        "sol-testnet": [("https://api.testnet.solana.com", "solana-labs", 10)],
        "near-testnet": [("https://rpc.testnet.near.org", "near", 20)],
        "aptos-testnet": [("https://fullnode.testnet.aptoslabs.com/v1", "aptoslabs", 20)],
        "sui-testnet": [("https://fullnode.testnet.sui.io", "mysten-labs", 20)],
    }
    for chain_id, chain_rpcs in testnets.items():
        for url, provider, rps in chain_rpcs:
            eps.append(rpc(chain_id, url, "https", provider, rps, "public", chain_type="testnet"))

    # ═══════════════════════════════════════════════════════════════════
    # EXCHANGE ENDPOINTS (public RPC/API endpoints from exchanges)
    # ═══════════════════════════════════════════════════════════════════
    exchange_eps = [
        # Binance Smart Chain — run by Binance
        rpc("bsc", "https://bsc-dataseed.bnbchain.org", "https", "bnbchain-official", 50, "exchange", chain_type="mainnet"),
        rpc("bsc", "https://bsc-dataseed1.bnbchain.org", "https", "bnbchain-official", 50, "exchange", chain_type="mainnet"),
        rpc("bsc", "https://bsc-dataseed2.bnbchain.org", "https", "bnbchain-official", 50, "exchange", chain_type="mainnet"),
        # Coinbase — Base chain
        rpc("base", "https://developer-access-mainnet.base.org", "https", "coinbase-dev", 20, "exchange", chain_type="mainnet"),
        # OKX — X Layer
        rpc("xlayer", "https://rpc.xlayer.tech", "https", "okx", 25, "exchange", chain_type="mainnet"),
        rpc("xlayer", "https://xlayerrpc.okx.com", "https", "okx", 25, "exchange", chain_type="mainnet"),
        # Cronos — from Crypto.com
        rpc("cronos", "https://evm.cronos.org", "https", "crypto-com", 30, "exchange", chain_type="mainnet"),
        rpc("cronos", "https://cronos-evm-rpc.publicnode.com", "https", "publicnode", 20, "exchange", chain_type="mainnet"),
    ]
    eps.extend(exchange_eps)

    # ═══════════════════════════════════════════════════════════════════
    # GAS STATION / FEE ORACLE ENDPOINTS
    # ═══════════════════════════════════════════════════════════════════
    gas_eps = [
        rpc("eth-gas", "https://gas.api.infura.io/v3/public", "https", "infura-gas", 5, "gas", chain_type="mainnet", source="gas"),
        rpc("eth-gas", "https://api.etherscan.io/api?module=gastracker&action=gasoracle", "https", "etherscan-gas", 5, "gas", chain_type="mainnet", source="gas"),
        rpc("polygon-gas", "https://gasstation.polygon.technology/v2", "https", "polygon-gas", 10, "gas", chain_type="mainnet", source="gas"),
        rpc("bsc-gas", "https://api.bscscan.com/api?module=gastracker&action=gasoracle", "https", "bscscan-gas", 5, "gas", chain_type="mainnet", source="gas"),
        rpc("eth-gas", "https://api.blocknative.com/gasprices/blockprices", "https", "blocknative", 5, "gas", chain_type="mainnet", source="gas"),
        rpc("avax-gas", "https://api.snowtrace.io/api?module=gastracker&action=gasoracle", "https", "snowtrace-gas", 5, "gas", chain_type="mainnet", source="gas"),
    ]
    eps.extend(gas_eps)

    # ═══════════════════════════════════════════════════════════════════
    # DEX AGGREGATOR / DeFi RPCs
    # ═══════════════════════════════════════════════════════════════════
    defi_eps = [
        # 1inch — public RPCs
        rpc("eth", "https://web3.1inch.io/v1.0/1", "https", "1inch", 10, "dex", chain_type="mainnet", source="dex"),
        rpc("bsc", "https://web3.1inch.io/v1.0/56", "https", "1inch", 10, "dex", chain_type="mainnet", source="dex"),
        rpc("polygon", "https://web3.1inch.io/v1.0/137", "https", "1inch", 10, "dex", chain_type="mainnet", source="dex"),
        rpc("arb-one", "https://web3.1inch.io/v1.0/42161", "https", "1inch", 10, "dex", chain_type="mainnet", source="dex"),
        rpc("optimism", "https://web3.1inch.io/v1.0/10", "https", "1inch", 10, "dex", chain_type="mainnet", source="dex"),
        rpc("avax", "https://web3.1inch.io/v1.0/43114", "https", "1inch", 10, "dex", chain_type="mainnet", source="dex"),
        rpc("base", "https://web3.1inch.io/v1.0/8453", "https", "1inch", 10, "dex", chain_type="mainnet", source="dex"),
        rpc("gnosis", "https://web3.1inch.io/v1.0/100", "https", "1inch", 10, "dex", chain_type="mainnet", source="dex"),
        rpc("ftm", "https://web3.1inch.io/v1.0/250", "https", "1inch", 10, "dex", chain_type="mainnet", source="dex"),
        # MEV RPCs (Flashbots, MEV Blocker, etc.)
        rpc("eth", "https://rpc.flashbots.net/fast", "https", "flashbots-fast", 15, "mev", chain_type="mainnet", source="dex"),
        rpc("eth", "https://rpc.mevblocker.io/fast", "https", "mevblocker-fast", 15, "mev", chain_type="mainnet", source="dex"),
        rpc("eth", "https://rpc.mevblocker.io/noreverts", "https", "mevblocker-norev", 15, "mev", chain_type="mainnet", source="dex"),
        # Lido
        rpc("eth", "https://rpc.ankr.com/eth/lido", "https", "ankr-lido", 15, "defi", chain_type="mainnet", source="dex"),
    ]
    eps.extend(defi_eps)

    # ═══════════════════════════════════════════════════════════════════
    # BLOCK EXPLORER API RPCs
    # ═══════════════════════════════════════════════════════════════════
    explorer_eps = [
        rpc("eth-explorer", "https://api.etherscan.io/api", "https", "etherscan", 5, "explorer", chain_type="mainnet", source="explorer"),
        rpc("bsc-explorer", "https://api.bscscan.com/api", "https", "bscscan", 5, "explorer", chain_type="mainnet", source="explorer"),
        rpc("polygon-explorer", "https://api.polygonscan.com/api", "https", "polygonscan", 5, "explorer", chain_type="mainnet", source="explorer"),
        rpc("arb-explorer", "https://api.arbiscan.io/api", "https", "arbiscan", 5, "explorer", chain_type="mainnet", source="explorer"),
        rpc("op-explorer", "https://api-optimistic.etherscan.io/api", "https", "op-etherscan", 5, "explorer", chain_type="mainnet", source="explorer"),
        rpc("avax-explorer", "https://api.snowtrace.io/api", "https", "snowtrace", 5, "explorer", chain_type="mainnet", source="explorer"),
        rpc("ftm-explorer", "https://api.ftmscan.com/api", "https", "ftmscan", 5, "explorer", chain_type="mainnet", source="explorer"),
        rpc("base-explorer", "https://api.basescan.org/api", "https", "basescan", 5, "explorer", chain_type="mainnet", source="explorer"),
        rpc("linea-explorer", "https://api.lineascan.build/api", "https", "lineascan", 5, "explorer", chain_type="mainnet", source="explorer"),
        rpc("scroll-explorer", "https://api.scrollscan.com/api", "https", "scrollscan", 5, "explorer", chain_type="mainnet", source="explorer"),
    ]
    eps.extend(explorer_eps)

    # ═══════════════════════════════════════════════════════════════════
    # BRIDGE / CROSS-CHAIN RPCs
    # ═══════════════════════════════════════════════════════════════════
    bridge_eps = [
        rpc("arb-nova", "https://nova.arbitrum.io/rpc", "https", "arb-bridge", 30, "bridge", chain_type="mainnet", source="bridge"),
        rpc("polygon", "https://polygon-rpc.com", "https", "polygon-bridge", 50, "bridge", chain_type="mainnet", source="bridge"),
        rpc("gnosis", "https://rpc.gnosis.gateway.fm", "https", "gateway-bridge", 20, "bridge", chain_type="mainnet", source="bridge"),
    ]
    eps.extend(bridge_eps)

    # Deduplicate
    seen = set()
    deduped = []
    for ep in eps:
        key = (ep.chain_id, ep.url.rstrip("/"))
        if key not in seen:
            seen.add(key)
            deduped.append(ep)
    return deduped


# ═════════════════════════════════════════════════════════════════════════
# PATTERN-BASED DISCOVERY ENGINE
# ═════════════════════════════════════════════════════════════════════════

PROVIDER_PATTERNS = {
    "publicnode":  "https://{chain}.publicnode.com",
    "publicnode2": "https://{chain}-rpc.publicnode.com",
    "ankr":        "https://rpc.ankr.com/{chain}",
    "drpc":        "https://{chain}.drpc.org",
    "1rpc":        "https://1rpc.io/{chain}",
    "blockpi":     "https://{chain}.blockpi.network/v1/rpc/public",
    "meowrpc":     "https://{chain}.meowrpc.com",
    "blast":       "https://{chain}-mainnet.public.blastapi.io",
    "onfinality":  "https://{chain}.api.onfinality.io/public",
    "llamarpc":    "https://{chain}.llamarpc.com",
    "pokt":        "https://{chain}-mainnet.gateway.pokt.network/v1/lb/public",
    "lava":        "https://{chain}.lava.build",
    "nodies":      "https://{chain}.nodies.app",
}

CHAIN_SLUGS_MAINNET = [
    "ethereum", "bsc", "polygon", "arbitrum", "optimism", "avalanche", "base",
    "fantom", "gnosis", "solana", "linea", "scroll", "zksync", "celo", "mantle",
    "blast", "moonbeam", "moonriver", "cronos", "aurora", "harmony", "metis",
    "kava", "near", "sui", "aptos", "cosmos", "osmosis", "injective", "sei",
    "celestia", "polkadot", "kusama", "tron", "filecoin", "chiliz", "sonic",
    "taiko", "unichain", "fraxtal", "opbnb", "pulsechain", "starknet", "evmos",
    "astar", "mode", "manta", "berachain", "xlayer", "core", "flare", "telos",
    "gravity", "story", "polygon-zkevm", "iotex", "xdc", "ton", "fuse", "boba",
    "oasis", "canto", "klaytn", "wemix", "zetachain", "hedera", "algorand",
    "cardano", "flow", "iota", "syscoin", "rootstock", "thundercore", "velas",
    "conflux", "neon", "okx", "zkfair", "merlin", "bob", "cyber", "redstone",
    "worldchain", "zora", "pgn", "lyra", "orderly", "lisk", "mint",
]

CHAIN_SLUGS_TESTNET = [
    "sepolia", "holesky", "goerli", "mumbai", "amoy", "fuji", "bsc-testnet",
    "arbitrum-sepolia", "optimism-sepolia", "base-sepolia", "scroll-sepolia",
    "blast-sepolia", "linea-sepolia", "zksync-sepolia", "mantle-sepolia",
    "polygon-zkevm-testnet", "fantom-testnet", "moonbase-alpha", "gnosis-chiado",
    "celo-alfajores",
]

def discover_pattern_rpcs(state: CrawlerState) -> list[RPCEndpoint]:
    """Generate candidate RPCs from URL templates."""
    candidates = []
    all_slugs = CHAIN_SLUGS_MAINNET + CHAIN_SLUGS_TESTNET

    for slug in all_slugs:
        chain_type = "testnet" if slug in CHAIN_SLUGS_TESTNET else "mainnet"
        for provider, pattern in PROVIDER_PATTERNS.items():
            url = pattern.format(chain=slug)
            candidates.append(RPCEndpoint(
                chain_id=slug, url=url, protocol="https", provider=provider.replace("2", ""),
                rate_limit_rps=15, chain_type=chain_type, source="pattern",
            ))
    return candidates


# ═════════════════════════════════════════════════════════════════════════
# GOOGLE DORKS DISCOVERY
# ═════════════════════════════════════════════════════════════════════════

GOOGLE_DORKS = [
    # RPC endpoint discovery
    'inurl:"/rpc" intext:"eth_chainId" -github.com -stackoverflow.com',
    'inurl:"rpc" intext:"jsonrpc" site:*.io OR site:*.com OR site:*.xyz',
    'intitle:"RPC" intext:"public" intext:"endpoint" blockchain',
    'inurl:"mainnet" inurl:"rpc" -docs -tutorial -blog',
    '"public rpc" "rate limit" ethereum OR polygon OR arbitrum OR optimism',
    'site:*.network inurl:rpc -docs -blog',
    'site:*.xyz inurl:rpc intext:chainId',
    '"free rpc" ethereum OR bsc OR polygon OR avalanche',
    'inurl:"ext/bc/C/rpc" OR inurl:"evm" site:*.network',
    'intext:"wss://" intext:"rpc" blockchain websocket public',
    # Exchange / CEX endpoints
    '"binance" "rpc" "public" endpoint',
    '"coinbase" "rpc" "public" base OR ethereum',
    '"kraken" OR "okx" OR "bybit" intext:"rpc" public blockchain',
    # DeFi / DEX discovery
    '"1inch" OR "paraswap" OR "0x" rpc endpoint public',
    '"uniswap" OR "sushiswap" OR "curve" rpc node public',
    '"aave" OR "compound" OR "lido" rpc endpoint list',
    # Gas station
    '"gas" "oracle" "api" ethereum OR polygon free public',
    '"gas price" "api" "endpoint" blockchain',
    # Bridge endpoints
    '"bridge" "rpc" public endpoint blockchain cross-chain',
    '"wormhole" OR "layerzero" OR "axelar" rpc endpoint',
    # Chainlist-like aggregators
    'site:chainlist.org OR site:chainid.network rpc endpoint',
    '"chain" "rpc" "endpoints" list json',
    # Testnet faucets (often expose RPCs)
    '"faucet" "rpc" testnet sepolia OR holesky OR fuji OR amoy',
    '"testnet" "rpc" "public" endpoint free',
    # Node-as-a-service (sometimes expose free tiers)
    '"alchemy" OR "quicknode" OR "infura" "free" "rpc" "public"',
    '"getblock" OR "nodereal" OR "chainstack" free public rpc',
    # GitHub awesome lists
    'site:github.com "awesome" "rpc" "public" blockchain list',
    'site:github.com "public-rpc" OR "free-rpc" blockchain',
]

# Regex to extract RPC-like URLs from search results / web pages
RPC_URL_PATTERN = re.compile(
    r'(https?://[a-zA-Z0-9\-._~:/?#\[\]@!$&\'()*+,;=%]+)'
    r'(?:.*?(?:rpc|jsonrpc|eth_|chain|mainnet|testnet|node|api|evm))',
    re.IGNORECASE,
)

# More precise: match URLs that look like RPC endpoints
RPC_ENDPOINT_RE = re.compile(
    r'(https?://[\w\-.]+'
    r'(?:\.io|\.com|\.org|\.network|\.xyz|\.build|\.dev|\.one|\.tools|\.tech)'
    r'(?:/[\w\-./]*)?'
    r'(?:rpc|jsonrpc|v1|api|evm|ext|mainnet|public)?'
    r'[\w\-./]*)',
    re.IGNORECASE,
)

def is_likely_rpc_url(url: str) -> bool:
    """Heuristic: does this URL look like an RPC endpoint?"""
    parsed = urlparse(url)
    if not parsed.scheme.startswith("http"):
        return False
    if not parsed.netloc:
        return False
    # Skip known non-RPC domains
    skip_domains = [
        "github.com", "stackoverflow.com", "reddit.com", "twitter.com",
        "medium.com", "youtube.com", "google.com", "docs.", "blog.",
        "forum.", "discord", "telegram", "t.me", "wikipedia",
    ]
    for d in skip_domains:
        if d in parsed.netloc:
            return False
    # Positive signals
    rpc_signals = ["rpc", "jsonrpc", "api", "node", "mainnet", "testnet",
                   "evm", "ext/bc", "public", "endpoint", "chain"]
    url_lower = url.lower()
    return any(s in url_lower for s in rpc_signals) or parsed.port is not None


async def google_dork_search(
    session: "aiohttp.ClientSession",
    limiter: DomainRateLimiter,
    dork: str,
    num_results: int = 20,
) -> list[str]:
    """Search using a Google dork query and extract URLs from results.

    Uses DuckDuckGo HTML search (no API key needed, less aggressive
    rate-limiting than Google).
    """
    urls_found = []
    try:
        # DuckDuckGo HTML search (no JS required)
        search_url = f"https://html.duckduckgo.com/html/?q={quote_plus(dork)}"
        await limiter.wait(search_url)

        async with session.get(
            search_url,
            headers={
                "User-Agent": "Mozilla/5.0 (X11; Linux x86_64; rv:120.0) Gecko/20100101 Firefox/120.0",
                "Accept": "text/html,application/xhtml+xml",
                "Accept-Language": "en-US,en;q=0.9",
            },
            timeout=aiohttp.ClientTimeout(total=15),
            ssl=False,
        ) as resp:
            if resp.status == 200:
                html = await resp.text()
                # Extract URLs from DuckDuckGo results
                # DDG wraps links in uddg= parameter
                uddg_links = re.findall(r'uddg=([^&"]+)', html)
                for encoded in uddg_links:
                    from urllib.parse import unquote
                    decoded = unquote(encoded)
                    urls_found.append(decoded)

                # Also extract any RPC-like URLs from the page content
                for match in RPC_ENDPOINT_RE.finditer(html):
                    url = match.group(1)
                    if is_likely_rpc_url(url):
                        urls_found.append(url)
    except Exception as e:
        log.debug(f"Dork search failed for [{dork[:50]}...]: {e}")

    return list(set(urls_found))


async def scrape_page_for_rpcs(
    session: "aiohttp.ClientSession",
    limiter: DomainRateLimiter,
    page_url: str,
) -> list[str]:
    """Fetch a web page and extract any RPC-like URLs from it."""
    rpcs = []
    try:
        await limiter.wait(page_url)
        async with session.get(
            page_url,
            headers={"User-Agent": "Mozilla/5.0 (X11; Linux x86_64; rv:120.0) Gecko/20100101 Firefox/120.0"},
            timeout=aiohttp.ClientTimeout(total=10),
            ssl=False,
        ) as resp:
            if resp.status == 200:
                text = await resp.text()
                for match in RPC_ENDPOINT_RE.finditer(text):
                    url = match.group(1)
                    if is_likely_rpc_url(url):
                        rpcs.append(url)
    except Exception:
        pass
    return list(set(rpcs))


# ═════════════════════════════════════════════════════════════════════════
# KNOWN WEB SOURCES TO SCRAPE
# ═════════════════════════════════════════════════════════════════════════

SCRAPE_SOURCES = [
    # Chain registries
    "https://chainid.network/chains.json",
    "https://raw.githubusercontent.com/DefiLlama/chainlist/main/constants/extraRpcs.js",
    # GitHub awesome lists
    "https://raw.githubusercontent.com/arddluma/awesome-list-rpc-nodes-providers/main/docs/root/README.md",
    "https://raw.githubusercontent.com/sambacha/ethereum-rpc-archive/main/docs/root/README.md",
    # PublicNode directory
    "https://www.publicnode.com/",
    # Polkadot / Substrate
    "https://raw.githubusercontent.com/nickytonline/polkadot-rpc-list/main/docs/root/README.md",
]


async def scrape_chainid_network(
    session: "aiohttp.ClientSession",
    limiter: DomainRateLimiter,
) -> list[RPCEndpoint]:
    """Fetch the chainid.network/chains.json and extract all RPCs."""
    endpoints = []
    try:
        url = "https://chainid.network/chains.json"
        await limiter.wait(url)
        async with session.get(url, timeout=aiohttp.ClientTimeout(total=30), ssl=False) as resp:
            if resp.status == 200:
                chains = await resp.json(content_type=None)
                for chain in chains:
                    chain_id_num = chain.get("chainId", 0)
                    name = chain.get("name", "").lower().replace(" ", "-")
                    rpcs = chain.get("rpc", [])
                    is_testnet = any(t in name for t in ["test", "devnet", "sepolia", "goerli", "mumbai"])
                    chain_type = "testnet" if is_testnet else "mainnet"

                    # Use short name or network name
                    short_name = chain.get("shortName", name)[:20]

                    for rpc_url in rpcs:
                        if isinstance(rpc_url, dict):
                            rpc_url = rpc_url.get("url", "")
                        if not rpc_url or not rpc_url.startswith("http"):
                            continue
                        # Skip URLs with API key placeholders
                        if "${" in rpc_url or "API_KEY" in rpc_url or "YOUR_" in rpc_url:
                            continue
                        # Skip wss:// for now (we focus on HTTPS)
                        if rpc_url.startswith("wss://"):
                            continue

                        provider = urlparse(rpc_url).netloc.split(".")[0]
                        endpoints.append(RPCEndpoint(
                            chain_id=short_name,
                            url=rpc_url,
                            protocol="https",
                            provider=provider,
                            rate_limit_rps=10,  # conservative default
                            chain_type=chain_type,
                            source="chainlist",
                        ))

                log.info(f"  chainid.network: extracted {len(endpoints)} RPC endpoints from {len(chains)} chains")
    except Exception as e:
        log.warning(f"  chainid.network scrape failed: {e}")

    return endpoints


# ═════════════════════════════════════════════════════════════════════════
# ASYNC RPC VALIDATOR
# ═════════════════════════════════════════════════════════════════════════

EVM_CHAIN_ID = json.dumps({"jsonrpc": "2.0", "method": "eth_chainId", "params": [], "id": 1}).encode()
SOL_HEALTH = json.dumps({"jsonrpc": "2.0", "method": "getHealth", "params": [], "id": 1}).encode()
SUBSTRATE_HEALTH = json.dumps({"jsonrpc": "2.0", "method": "system_health", "params": [], "id": 1}).encode()
NET_VERSION = json.dumps({"jsonrpc": "2.0", "method": "net_version", "params": [], "id": 1}).encode()

# Chain IDs that need special validation
SOLANA_CHAINS = {"sol", "sol-devnet", "sol-testnet", "solana"}
COSMOS_CHAINS = {"cosmos", "osmosis", "injective", "sei", "celestia", "dydx", "terra"}
MOVE_CHAINS = {"aptos", "sui", "aptos-testnet", "sui-testnet"}
SUBSTRATE_CHAINS = {"polkadot", "kusama"}
GAS_CHAINS = {"eth-gas", "polygon-gas", "bsc-gas", "avax-gas"}
EXPLORER_CHAINS = {"eth-explorer", "bsc-explorer", "polygon-explorer", "arb-explorer",
                   "op-explorer", "avax-explorer", "ftm-explorer", "base-explorer",
                   "linea-explorer", "scroll-explorer"}


async def validate_rpc(
    ep: RPCEndpoint,
    limiter: DomainRateLimiter,
    session: "aiohttp.ClientSession",
    timeout_s: float = 6.0,
) -> tuple[RPCEndpoint, bool, Optional[float]]:
    """Validate a single RPC endpoint."""
    await limiter.wait(ep.url)
    try:
        t0 = time.monotonic()

        # Gas / Explorer endpoints — just check HTTP status
        if ep.chain_id in GAS_CHAINS or ep.chain_id in EXPLORER_CHAINS:
            async with session.get(ep.url, timeout=aiohttp.ClientTimeout(total=timeout_s), ssl=False) as resp:
                lat = (time.monotonic() - t0) * 1000
                ok = resp.status in (200, 403)  # 403 = API key needed but endpoint exists
                ep.latency_ms = lat
                ep.is_healthy = ok
                return (ep, ok, lat)

        # Solana
        if ep.chain_id in SOLANA_CHAINS:
            payload = SOL_HEALTH
        # Cosmos — try GET /status
        elif ep.chain_id in COSMOS_CHAINS:
            async with session.get(
                ep.url.rstrip("/") + "/status",
                timeout=aiohttp.ClientTimeout(total=timeout_s), ssl=False,
            ) as resp:
                lat = (time.monotonic() - t0) * 1000
                ok = resp.status == 200
                ep.latency_ms = lat
                ep.is_healthy = ok
                return (ep, ok, lat)
        # Move chains — REST GET
        elif ep.chain_id in MOVE_CHAINS:
            async with session.get(
                ep.url, timeout=aiohttp.ClientTimeout(total=timeout_s), ssl=False,
            ) as resp:
                lat = (time.monotonic() - t0) * 1000
                ok = resp.status == 200
                ep.latency_ms = lat
                ep.is_healthy = ok
                return (ep, ok, lat)
        # Substrate
        elif ep.chain_id in SUBSTRATE_CHAINS:
            payload = SUBSTRATE_HEALTH
        # Default: EVM
        else:
            payload = EVM_CHAIN_ID

        async with session.post(
            ep.url, data=payload,
            headers={"Content-Type": "application/json"},
            timeout=aiohttp.ClientTimeout(total=timeout_s), ssl=False,
        ) as resp:
            lat = (time.monotonic() - t0) * 1000
            try:
                data = await resp.json(content_type=None)
                ok = "result" in data and resp.status == 200
            except Exception:
                # Some RPCs return non-JSON on success
                ok = resp.status == 200
            ep.latency_ms = lat
            ep.is_healthy = ok
            return (ep, ok, lat)

    except Exception:
        ep.is_healthy = False
        return (ep, False, None)


async def validate_batch(
    endpoints: list[RPCEndpoint],
    concurrency: int = 30,
    timeout_s: float = 6.0,
) -> list[tuple[RPCEndpoint, bool, Optional[float]]]:
    """Validate many endpoints concurrently."""
    limiter = DomainRateLimiter(max_per_second=3.0)
    results = []
    sem = asyncio.Semaphore(concurrency)

    async with aiohttp.ClientSession() as session:
        async def worker(ep):
            async with sem:
                return await validate_rpc(ep, limiter, session, timeout_s)

        tasks = [worker(ep) for ep in endpoints]
        done = 0
        total = len(tasks)
        for coro in asyncio.as_completed(tasks):
            result = await coro
            results.append(result)
            done += 1
            if done % 50 == 0 or done == total:
                log.info(f"    [{done}/{total}] validated...")

    return results


# ═════════════════════════════════════════════════════════════════════════
# DATABASE SEEDER
# ═════════════════════════════════════════════════════════════════════════

def ensure_chain_exists(cur: sqlite3.Cursor, chain_id: str, chain_type: str = "mainnet"):
    """Create chain entry if missing."""
    cur.execute("SELECT 1 FROM chains WHERE chain_id = ?", (chain_id,))
    if not cur.fetchone():
        ecosystem = "evm"
        ct = chain_type.upper() if chain_type != "mainnet" else "L1"
        if chain_id in SOLANA_CHAINS:
            ecosystem = "svm"
        elif chain_id in COSMOS_CHAINS:
            ecosystem = "cosmos"
        elif chain_id in SUBSTRATE_CHAINS:
            ecosystem = "substrate"
        elif chain_id in MOVE_CHAINS:
            ecosystem = "move"

        l2_indicators = ["arb", "optimism", "base", "mantle", "blast", "linea", "scroll",
                         "zksync", "mode", "manta", "taiko", "fraxtal", "opbnb", "starknet"]
        if any(ind in chain_id for ind in l2_indicators):
            ct = "L2"
        if "test" in chain_id or "sepolia" in chain_id or "devnet" in chain_id:
            ct = "testnet"
        if chain_id.endswith("-gas") or chain_id.endswith("-explorer"):
            ct = "utility"

        name = chain_id.replace("-", " ").title()
        try:
            cur.execute("""
                INSERT OR IGNORE INTO chains (chain_id, chain_name, ecosystem, chain_type, status)
                VALUES (?, ?, ?, ?, 'active')
            """, (chain_id, name, ecosystem, ct))
        except Exception:
            pass


def seed_to_db(endpoints: list[RPCEndpoint], state: CrawlerState, replace_first: bool = False):
    """Seed validated endpoints into the chain database."""
    if not os.path.exists(DB_PATH):
        log.error(f"Database not found at {DB_PATH}")
        return

    conn = sqlite3.connect(DB_PATH)
    cur = conn.cursor()

    if replace_first:
        cur.execute("""
            DELETE FROM rpc_endpoints
            WHERE url LIKE '%${%' OR url LIKE '%API_KEY%'
               OR (rate_limit_rps IS NULL AND provider IS NULL)
        """)
        deleted = cur.rowcount
        if deleted > 0:
            log.info(f"  Cleaned {deleted} placeholder RPCs")

    inserted = 0
    updated = 0

    for ep in endpoints:
        if not ep.is_healthy:
            continue
        if ep.url in state.banned_urls:
            continue

        ensure_chain_exists(cur, ep.chain_id, ep.chain_type)

        cur.execute(
            "SELECT id, is_healthy FROM rpc_endpoints WHERE chain_id = ? AND url = ?",
            (ep.chain_id, ep.url),
        )
        existing = cur.fetchone()

        if existing:
            cur.execute("""
                UPDATE rpc_endpoints SET
                    provider = ?, rate_limit_rps = ?, tier = ?,
                    is_healthy = 1, latency_ms = ?, avg_latency_ms = ?,
                    last_checked = CURRENT_TIMESTAMP,
                    last_success_at = CURRENT_TIMESTAMP,
                    weight = CASE WHEN ? < 100 THEN 1.0 WHEN ? < 300 THEN 0.8 ELSE 0.5 END
                WHERE id = ?
            """, (
                ep.provider, ep.rate_limit_rps, ep.tier,
                int(ep.latency_ms) if ep.latency_ms else None,
                ep.latency_ms,
                ep.latency_ms or 999, ep.latency_ms or 999,
                existing[0],
            ))
            updated += 1
        else:
            weight = 1.0 if (ep.latency_ms or 999) < 100 else (0.8 if (ep.latency_ms or 999) < 300 else 0.5)
            try:
                cur.execute("""
                    INSERT INTO rpc_endpoints (
                        chain_id, url, protocol, provider, tier,
                        is_primary, is_healthy, latency_ms, rate_limit_rps,
                        avg_latency_ms, weight, last_checked, last_success_at
                    ) VALUES (?, ?, ?, ?, ?, 0, 1, ?, ?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
                """, (
                    ep.chain_id, ep.url, ep.protocol, ep.provider, ep.tier,
                    int(ep.latency_ms) if ep.latency_ms else None,
                    ep.rate_limit_rps, ep.latency_ms, weight,
                ))
                inserted += 1
            except sqlite3.IntegrityError:
                pass

    # Set primaries
    cur.execute("UPDATE rpc_endpoints SET is_primary = 0")
    cur.execute("""
        UPDATE rpc_endpoints SET is_primary = 1
        WHERE id IN (
            SELECT MIN(id) FROM rpc_endpoints
            WHERE is_healthy = 1 AND protocol = 'https'
            GROUP BY chain_id
        )
    """)

    conn.commit()

    # Stats
    cur.execute("SELECT COUNT(DISTINCT chain_id) FROM rpc_endpoints WHERE is_healthy = 1")
    chains = cur.fetchone()[0]
    cur.execute("SELECT COUNT(*) FROM rpc_endpoints WHERE is_healthy = 1")
    healthy = cur.fetchone()[0]
    cur.execute("SELECT SUM(rate_limit_rps) FROM rpc_endpoints WHERE is_healthy = 1")
    total_rps = cur.fetchone()[0] or 0

    conn.close()

    state.total_healthy = healthy
    log.info(f"  DB: +{inserted} new, ~{updated} updated | {healthy} healthy across {chains} chains | {total_rps} combined rps")


def mark_dead_in_db(dead_urls: list[str], state: CrawlerState):
    """Increment fail counts and auto-ban after 3 consecutive failures."""
    if not os.path.exists(DB_PATH):
        return

    conn = sqlite3.connect(DB_PATH)
    cur = conn.cursor()

    newly_banned = 0
    for url in dead_urls:
        state.fail_counts[url] = state.fail_counts.get(url, 0) + 1
        if state.fail_counts[url] >= 3:
            # Auto-ban
            if url not in state.banned_urls:
                state.banned_urls.append(url)
                newly_banned += 1
            # Mark unhealthy in DB
            cur.execute("UPDATE rpc_endpoints SET is_healthy = 0, weight = 0 WHERE url = ?", (url,))

    conn.commit()
    conn.close()

    if newly_banned > 0:
        log.info(f"  Auto-banned {newly_banned} endpoints (3+ consecutive failures)")


# ═════════════════════════════════════════════════════════════════════════
# AIRDROP / FAUCET / CHAIN DISCOVERY
# ═════════════════════════════════════════════════════════════════════════

# Known airdrop aggregator URLs to scrape
AIRDROP_SOURCES = [
    "https://airdrops.io/latest-airdrops/",
    "https://www.airdropking.io/",
    "https://cosmosairdrops.io/",
    "https://earni.fi/",
]

# Known faucet registries
FAUCET_REGISTRY: list[dict] = [
    # Ethereum testnets
    {"chain_id": "sepolia", "name": "Alchemy Sepolia Faucet", "provider": "alchemy", "url": "https://sepoliafaucet.com", "token_symbol": "ETH", "amount_per_claim": "0.5", "cooldown_hours": 24, "faucet_type": "web", "requires_auth": True, "auth_type": "alchemy"},
    {"chain_id": "sepolia", "name": "Infura Sepolia Faucet", "provider": "infura", "url": "https://www.infura.io/faucet/sepolia", "token_symbol": "ETH", "amount_per_claim": "0.5", "cooldown_hours": 24, "faucet_type": "web", "requires_auth": True, "auth_type": "infura"},
    {"chain_id": "sepolia", "name": "Google Cloud Sepolia Faucet", "provider": "google", "url": "https://cloud.google.com/application/web3/faucet/ethereum/sepolia", "token_symbol": "ETH", "amount_per_claim": "0.05", "cooldown_hours": 24, "faucet_type": "web", "requires_auth": True, "auth_type": "google"},
    {"chain_id": "sepolia", "name": "QuickNode Sepolia Faucet", "provider": "quicknode", "url": "https://faucet.quicknode.com/ethereum/sepolia", "token_symbol": "ETH", "amount_per_claim": "0.1", "cooldown_hours": 12, "faucet_type": "web", "requires_auth": True, "auth_type": "quicknode"},
    {"chain_id": "holesky", "name": "Holesky PoW Faucet", "provider": "community", "url": "https://holesky-faucet.pk910.de/", "token_symbol": "ETH", "amount_per_claim": "0.5", "cooldown_hours": 0, "faucet_type": "pow"},
    {"chain_id": "holesky", "name": "Holesky Faucet", "provider": "stakely", "url": "https://stakely.io/en/faucet/ethereum-holesky-testnet-eth", "token_symbol": "ETH", "amount_per_claim": "0.025", "cooldown_hours": 24, "faucet_type": "web"},
    # BNB testnet
    {"chain_id": "bsc-testnet", "name": "BNB Testnet Faucet", "provider": "bnb", "url": "https://www.bnbchain.org/en/testnet-faucet", "token_symbol": "tBNB", "amount_per_claim": "0.5", "cooldown_hours": 24, "faucet_type": "web"},
    # Polygon
    {"chain_id": "polygon-amoy", "name": "Polygon Amoy Faucet", "provider": "polygon", "url": "https://faucet.polygon.technology/", "token_symbol": "MATIC", "amount_per_claim": "0.5", "cooldown_hours": 24, "faucet_type": "web"},
    {"chain_id": "polygon-amoy", "name": "Alchemy Amoy Faucet", "provider": "alchemy", "url": "https://www.alchemy.com/faucets/polygon-amoy", "token_symbol": "MATIC", "amount_per_claim": "0.5", "cooldown_hours": 24, "faucet_type": "web", "requires_auth": True, "auth_type": "alchemy"},
    # Avalanche
    {"chain_id": "avax-fuji", "name": "Avalanche Fuji Faucet", "provider": "avalanche", "url": "https://core.app/tools/testnet-faucet/?subnet=c&token=c", "token_symbol": "AVAX", "amount_per_claim": "2", "cooldown_hours": 24, "faucet_type": "web"},
    # Arbitrum
    {"chain_id": "arb-sepolia", "name": "Arbitrum Sepolia Faucet", "provider": "alchemy", "url": "https://www.alchemy.com/faucets/arbitrum-sepolia", "token_symbol": "ETH", "amount_per_claim": "0.1", "cooldown_hours": 24, "faucet_type": "web", "requires_auth": True, "auth_type": "alchemy"},
    # Optimism
    {"chain_id": "op-sepolia", "name": "Optimism Sepolia Faucet", "provider": "alchemy", "url": "https://www.alchemy.com/faucets/optimism-sepolia", "token_symbol": "ETH", "amount_per_claim": "0.1", "cooldown_hours": 24, "faucet_type": "web", "requires_auth": True, "auth_type": "alchemy"},
    # Base
    {"chain_id": "base-sepolia", "name": "Base Sepolia Faucet", "provider": "alchemy", "url": "https://www.alchemy.com/faucets/base-sepolia", "token_symbol": "ETH", "amount_per_claim": "0.1", "cooldown_hours": 24, "faucet_type": "web", "requires_auth": True, "auth_type": "alchemy"},
    # Solana
    {"chain_id": "sol-devnet", "name": "Solana Devnet Faucet", "provider": "solana", "url": "https://faucet.solana.com/", "token_symbol": "SOL", "amount_per_claim": "2", "cooldown_hours": 0, "faucet_type": "api"},
    {"chain_id": "sol-testnet", "name": "Solana Testnet Faucet", "provider": "solana", "url": "https://faucet.solana.com/", "token_symbol": "SOL", "amount_per_claim": "2", "cooldown_hours": 0, "faucet_type": "api"},
    # Sui
    {"chain_id": "sui-devnet", "name": "Sui Devnet Faucet", "provider": "sui", "url": "https://faucet.devnet.sui.io/", "token_symbol": "SUI", "amount_per_claim": "10", "cooldown_hours": 0, "faucet_type": "api"},
    # Aptos
    {"chain_id": "aptos-devnet", "name": "Aptos Devnet Faucet", "provider": "aptos", "url": "https://faucet.devnet.aptoslabs.com/", "token_symbol": "APT", "amount_per_claim": "1", "cooldown_hours": 0, "faucet_type": "api"},
    # Scroll
    {"chain_id": "scroll-sepolia", "name": "Scroll Sepolia Faucet", "provider": "scroll", "url": "https://sepolia.scroll.io/bridge", "token_symbol": "ETH", "amount_per_claim": "0.01", "cooldown_hours": 24, "faucet_type": "web"},
    # zkSync
    {"chain_id": "zksync-sepolia", "name": "zkSync Sepolia Faucet", "provider": "chainlink", "url": "https://faucets.chain.link/zksync-sepolia", "token_symbol": "ETH", "amount_per_claim": "0.1", "cooldown_hours": 24, "faucet_type": "web", "requires_auth": True, "auth_type": "github"},
    # Linea
    {"chain_id": "linea-sepolia", "name": "Linea Sepolia Faucet", "provider": "infura", "url": "https://www.infura.io/faucet/linea", "token_symbol": "ETH", "amount_per_claim": "0.5", "cooldown_hours": 24, "faucet_type": "web", "requires_auth": True, "auth_type": "infura"},
    # Mantle
    {"chain_id": "mantle-sepolia", "name": "Mantle Sepolia Faucet", "provider": "mantle", "url": "https://faucet.sepolia.mantle.xyz/", "token_symbol": "MNT", "amount_per_claim": "1", "cooldown_hours": 24, "faucet_type": "web"},
    # Chainlink universal
    {"chain_id": "sepolia", "name": "Chainlink Sepolia Faucet", "provider": "chainlink", "url": "https://faucets.chain.link/sepolia", "token_symbol": "ETH+LINK", "amount_per_claim": "0.1+20", "cooldown_hours": 24, "faucet_type": "web", "requires_auth": True, "auth_type": "github"},
    # Fantom
    {"chain_id": "ftm-testnet", "name": "Fantom Testnet Faucet", "provider": "fantom", "url": "https://faucet.fantom.network/", "token_symbol": "FTM", "amount_per_claim": "10", "cooldown_hours": 24, "faucet_type": "web"},
    # Moonbeam/Moonriver
    {"chain_id": "moonbase-alpha", "name": "Moonbase Alpha Faucet", "provider": "moonbeam", "url": "https://faucet.moonbeam.network/", "token_symbol": "DEV", "amount_per_claim": "1", "cooldown_hours": 24, "faucet_type": "web"},
]

# Google dorks specifically for airdrops & faucets  
AIRDROP_DORKS = [
    '"airdrop" "claim" crypto 2025 eligible token',
    '"retroactive" "airdrop" "announced" OR "live" 2025',
    '"testnet airdrop" eligible snapshot "claim now"',
    '"token claim" "airdrop" "deadline" 2025 crypto',
    'site:twitter.com "airdrop" "live" "claim" crypto 2025',
    '"layer 2" OR "L2" airdrop token claim 2025',
    '"cosmos" OR "solana" OR "ethereum" airdrop "claim" 2025',
    '"faucet" "testnet" "free" tokens sepolia OR holesky OR amoy',
    '"new blockchain" testnet launch faucet 2025',
    '"airdrop" "snapshot" "eligible" wallet claim',
]


async def scrape_airdrops(
    session: "aiohttp.ClientSession",
    limiter: DomainRateLimiter,
) -> list[dict]:
    """Scrape airdrop aggregator sites for new airdrops."""
    airdrops_found = []

    for src_url in AIRDROP_SOURCES:
        try:
            await limiter.wait(src_url)
            async with session.get(src_url, timeout=aiohttp.ClientTimeout(total=15),
                                   headers={"User-Agent": "Mozilla/5.0 (compatible; AtlasCrawler/1.0)"}) as resp:
                if resp.status != 200:
                    continue
                html = await resp.text()

                # Extract airdrop-like entries from HTML
                # Look for patterns: project names + chains + claim links
                # This is heuristic — HTML parsing for each aggregator would give better results
                import re as _re

                # Find potential airdrop names and links
                links = _re.findall(r'<a[^>]+href="([^"]*)"[^>]*>([^<]+)</a>', html)
                for href, text in links:
                    text_lower = text.lower().strip()
                    # Filter for airdrop-related content
                    if any(kw in text_lower for kw in ['airdrop', 'claim', 'token', 'drop', 'reward']):
                        if len(text.strip()) < 100 and len(text.strip()) > 3:
                            airdrop_entry = {
                                "name": text.strip()[:100],
                                "source_url": href if href.startswith("http") else f"{src_url.rstrip('/')}/{href.lstrip('/')}",
                                "source": "crawler",
                                "chain_id": _infer_chain_from_text(text),
                                "airdrop_type": "unknown",
                            }
                            airdrops_found.append(airdrop_entry)
        except Exception as e:
            log.debug(f"  Failed to scrape airdrops from {src_url}: {e}")

    return airdrops_found


async def dork_for_airdrops(
    session: "aiohttp.ClientSession",
    limiter: DomainRateLimiter,
    state: CrawlerState,
) -> list[dict]:
    """Use Google dorks to discover new airdrops."""
    airdrops = []
    dork_idx = getattr(state, 'airdrop_dork_index', 0) % len(AIRDROP_DORKS)
    dorks = AIRDROP_DORKS[dork_idx:dork_idx + 2]
    state.airdrop_dork_index = (dork_idx + 2) % len(AIRDROP_DORKS)  # type: ignore

    for dork in dorks:
        try:
            urls = await google_dork_search(session, limiter, dork, num_results=10)
            for url in urls:
                parsed = urlparse(url)
                if any(skip in parsed.netloc for skip in ["google.com", "youtube.com", "wikipedia"]):
                    continue
                airdrops.append({
                    "name": f"Discovered from: {parsed.netloc}",
                    "source_url": url,
                    "source": "crawler",
                    "chain_id": "unknown",
                    "airdrop_type": "unknown",
                })
            await asyncio.sleep(2)
        except Exception as e:
            log.debug(f"  Airdrop dork failed: {e}")

    return airdrops


def _infer_chain_from_text(text: str) -> str:
    """Infer chain from airdrop/faucet text."""
    text_lower = text.lower()
    hints = [
        ("ethereum", "eth"), ("eth", "eth"), ("bsc", "bsc"), ("bnb", "bsc"),
        ("polygon", "polygon"), ("matic", "polygon"), ("arbitrum", "arb-one"),
        ("optimism", "optimism"), ("avalanche", "avax"), ("base", "base"),
        ("solana", "sol"), ("sui", "sui"), ("aptos", "aptos"), ("cosmos", "cosmos"),
        ("fantom", "ftm"), ("near", "near"), ("zksync", "zksync"),
        ("scroll", "scroll"), ("linea", "linea"), ("mantle", "mantle"),
        ("blast", "blast"), ("sei", "sei"), ("celestia", "celestia"),
        ("injective", "injective"), ("starknet", "starknet"), ("ton", "ton"),
    ]
    for hint, chain_id in hints:
        if hint in text_lower:
            return chain_id
    return "unknown"


def seed_faucets_to_db(state: CrawlerState):
    """Seed known faucets into the faucets table."""
    if not os.path.exists(DB_PATH):
        return

    conn = sqlite3.connect(DB_PATH)
    cur = conn.cursor()
    inserted = 0

    for f in FAUCET_REGISTRY:
        try:
            cur.execute("""
                INSERT OR IGNORE INTO faucets
                (chain_id, name, provider, url, faucet_type, token_symbol,
                 amount_per_claim, cooldown_hours, requires_auth, auth_type, source)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 'crawler')
            """, (
                f["chain_id"], f["name"], f.get("provider"), f["url"],
                f.get("faucet_type", "web"), f.get("token_symbol"),
                f.get("amount_per_claim"), f.get("cooldown_hours", 24),
                1 if f.get("requires_auth") else 0, f.get("auth_type"),
            ))
            if cur.rowcount > 0:
                inserted += 1
        except Exception as e:
            log.debug(f"  Faucet insert failed for {f['name']}: {e}")

    conn.commit()
    conn.close()
    if inserted > 0:
        log.info(f"  Seeded {inserted} new faucets to DB")


def seed_airdrops_to_db(airdrops: list[dict]):
    """Insert discovered airdrops into the airdrops table."""
    if not os.path.exists(DB_PATH) or not airdrops:
        return

    conn = sqlite3.connect(DB_PATH)
    cur = conn.cursor()
    inserted = 0

    for ad in airdrops:
        try:
            # Check if we already have this airdrop (by name + source_url)
            existing = cur.execute(
                "SELECT id FROM airdrops WHERE name = ? AND source_url = ?",
                (ad["name"], ad.get("source_url"))
            ).fetchone()
            if existing:
                continue

            cur.execute("""
                INSERT INTO airdrops
                (chain_id, name, airdrop_type, source, source_url, status)
                VALUES (?, ?, ?, ?, ?, 'discovered')
            """, (
                ad.get("chain_id", "unknown"),
                ad["name"],
                ad.get("airdrop_type", "unknown"),
                ad.get("source", "crawler"),
                ad.get("source_url"),
            ))
            inserted += 1
        except Exception as e:
            log.debug(f"  Airdrop insert failed: {e}")

    conn.commit()
    conn.close()
    if inserted > 0:
        log.info(f"  Seeded {inserted} new airdrops to DB")


def log_chain_discovery(chain_name: str, chain_id: Optional[str], chain_numeric_id: Optional[int],
                        ecosystem: str, chain_type: str, is_testnet: bool,
                        source: str, source_url: Optional[str] = None, rpc_url: Optional[str] = None):
    """Log a newly discovered chain to the chain_discoveries table."""
    if not os.path.exists(DB_PATH):
        return

    conn = sqlite3.connect(DB_PATH)
    cur = conn.cursor()

    try:
        # Check if already discovered
        if chain_numeric_id:
            existing = cur.execute(
                "SELECT id FROM chain_discoveries WHERE chain_numeric_id = ?",
                (chain_numeric_id,)
            ).fetchone()
        elif chain_id:
            existing = cur.execute(
                "SELECT id FROM chain_discoveries WHERE chain_id = ?",
                (chain_id,)
            ).fetchone()
        else:
            existing = cur.execute(
                "SELECT id FROM chain_discoveries WHERE chain_name = ?",
                (chain_name,)
            ).fetchone()

        if not existing:
            cur.execute("""
                INSERT INTO chain_discoveries
                (chain_id, chain_name, chain_numeric_id, ecosystem, chain_type,
                 is_testnet, source, source_url, rpc_url, status)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, 'new')
            """, (chain_id, chain_name, chain_numeric_id, ecosystem, chain_type,
                  1 if is_testnet else 0, source, source_url, rpc_url))
            if cur.rowcount > 0:
                log.info(f"  NEW CHAIN DISCOVERED: {chain_name} ({chain_id or chain_numeric_id}) [{chain_type}]")
    except Exception as e:
        log.debug(f"  Chain discovery log failed: {e}")

    conn.commit()
    conn.close()


async def detect_new_chains_from_chainlist(
    session: "aiohttp.ClientSession",
    limiter: DomainRateLimiter,
):
    """Scrape chainid.network for chains not in our DB and log them."""
    try:
        await limiter.wait("https://chainid.network")
        async with session.get(
            "https://chainid.network/chains.json",
            timeout=aiohttp.ClientTimeout(total=30),
            headers={"User-Agent": "Mozilla/5.0 (compatible; AtlasCrawler/1.0)"}
        ) as resp:
            if resp.status != 200:
                return
            chains_json = await resp.json()

        if not isinstance(chains_json, list):
            return

        conn = sqlite3.connect(DB_PATH)
        cur = conn.cursor()

        new_count = 0
        for c in chains_json:
            numeric_id = c.get("chainId")
            name = c.get("name", "")
            if not numeric_id or not name:
                continue

            # Check if we already have it in chains table
            existing = cur.execute(
                "SELECT chain_id FROM chains WHERE chain_numeric_id = ?",
                (numeric_id,)
            ).fetchone()

            if not existing:
                # Determine if testnet
                name_lower = name.lower()
                is_testnet = any(kw in name_lower for kw in ["testnet", "devnet", "sepolia", "goerli", "holesky", "fuji", "amoy", "mumbai"])

                # Get first RPC
                rpcs = c.get("rpc", [])
                first_rpc = None
                for r in rpcs:
                    if isinstance(r, str) and r.startswith("https://") and "${" not in r:
                        first_rpc = r
                        break

                ecosystem = "evm"
                chain_type = "testnet" if is_testnet else "L1"

                log_chain_discovery(
                    chain_name=name,
                    chain_id=c.get("shortName"),
                    chain_numeric_id=numeric_id,
                    ecosystem=ecosystem,
                    chain_type=chain_type,
                    is_testnet=is_testnet,
                    source="chainlist",
                    source_url=f"https://chainid.network/chains.json",
                    rpc_url=first_rpc,
                )
                new_count += 1

        conn.close()
        if new_count > 0:
            log.info(f"  Detected {new_count} new chains from chainid.network")

    except Exception as e:
        log.debug(f"  Chain detection from chainlist failed: {e}")


# ═════════════════════════════════════════════════════════════════════════
# MAIN CRAWL CYCLE
# ═════════════════════════════════════════════════════════════════════════

async def run_cycle(state: CrawlerState, aggressive: bool = False):
    """Execute one full crawl cycle."""
    state.cycle_count += 1
    state.last_cycle_time = datetime.now(timezone.utc).isoformat()
    cycle = state.cycle_count

    log.info(f"{'━'*60}")
    log.info(f"  CRAWL CYCLE #{cycle} — {time.strftime('%Y-%m-%d %H:%M:%S')}")
    log.info(f"{'━'*60}")

    all_endpoints: list[RPCEndpoint] = []

    # ── Step 1: Hardcoded mega registry ────────────────────────────────
    registry = build_mega_registry()
    log.info(f"  [1/5] Registry: {len(registry)} endpoints across {len(set(e.chain_id for e in registry))} chains")
    all_endpoints.extend(registry)

    # ── Step 2: Pattern-based discovery ────────────────────────────────
    patterns = discover_pattern_rpcs(state)
    log.info(f"  [2/5] Pattern discovery: {len(patterns)} candidates")
    all_endpoints.extend(patterns)

    # ── Step 3: chainid.network / chainlist ────────────────────────────
    if HAS_AIOHTTP:
        async with aiohttp.ClientSession() as session:
            limiter = DomainRateLimiter(2.0)
            chainlist_eps = await scrape_chainid_network(session, limiter)
            log.info(f"  [3/5] chainid.network: {len(chainlist_eps)} endpoints")
            all_endpoints.extend(chainlist_eps)

            # ── Step 4: Google dorks (if aggressive) ───────────────────
            if aggressive and cycle <= 50:  # Don't dork forever
                dork_idx = state.google_dork_index % len(GOOGLE_DORKS)
                # Do 3 dorks per cycle to pace ourselves
                dorks_this_cycle = GOOGLE_DORKS[dork_idx:dork_idx + 3]
                state.google_dork_index = (dork_idx + 3) % len(GOOGLE_DORKS)

                dork_urls = []
                for dork in dorks_this_cycle:
                    log.info(f"  [4/5] Dorking: {dork[:60]}...")
                    found = await google_dork_search(session, limiter, dork)
                    dork_urls.extend(found)
                    await asyncio.sleep(2)  # Be nice to search engines

                # Scrape found pages for RPC URLs
                rpc_urls_from_dorks = set()
                for page_url in dork_urls[:20]:  # Limit pages per cycle
                    if is_likely_rpc_url(page_url):
                        rpc_urls_from_dorks.add(page_url)
                    else:
                        # Scrape the page for RPC URLs
                        found_rpcs = await scrape_page_for_rpcs(session, limiter, page_url)
                        rpc_urls_from_dorks.update(found_rpcs)

                log.info(f"  [4/5] Dorks found {len(rpc_urls_from_dorks)} candidate RPC URLs")

                # Convert to RPCEndpoint objects
                for url in rpc_urls_from_dorks:
                    if url not in state.banned_urls:
                        provider = urlparse(url).netloc.split(".")[0]
                        # Try to infer chain from URL
                        chain_id = infer_chain_from_url(url)
                        all_endpoints.append(RPCEndpoint(
                            chain_id=chain_id, url=url, protocol="https",
                            provider=provider, rate_limit_rps=5,
                            source="google", chain_type="mainnet",
                        ))
            else:
                log.info(f"  [4/5] Dorks: {'skipped (not aggressive)' if not aggressive else 'completed all dork queries'}")

            # ── Step 5: Scrape known sources ───────────────────────────
            for src_url in SCRAPE_SOURCES[:3]:  # Rotate through sources
                found = await scrape_page_for_rpcs(session, limiter, src_url)
                for url in found:
                    if url not in state.banned_urls:
                        chain_id = infer_chain_from_url(url)
                        all_endpoints.append(RPCEndpoint(
                            chain_id=chain_id, url=url, protocol="https",
                            provider=urlparse(url).netloc.split(".")[0],
                            rate_limit_rps=5, source="scrape", chain_type="mainnet",
                        ))
    else:
        log.warning("  [3-5] aiohttp not installed — skipping web scraping")

    # ── Deduplicate ────────────────────────────────────────────────────
    seen = set()
    deduped = []
    for ep in all_endpoints:
        key = (ep.chain_id, ep.url.rstrip("/"))
        if key not in seen and ep.url not in state.banned_urls:
            seen.add(key)
            deduped.append(ep)

    log.info(f"  Total unique candidates: {len(deduped)}")

    # ── Validate ───────────────────────────────────────────────────────
    if HAS_AIOHTTP:
        log.info(f"  Validating {len(deduped)} endpoints...")
        results = await validate_batch(deduped, concurrency=40, timeout_s=8)
    else:
        log.warning("  Sync validation (no aiohttp)")
        results = []

    healthy = [r for r in results if r[1]]
    dead = [r for r in results if not r[1]]
    log.info(f"  Results: {len(healthy)} healthy / {len(dead)} dead")

    # ── Seed healthy ones ──────────────────────────────────────────────
    if healthy:
        healthy_eps = [r[0] for r in healthy]
        seed_to_db(healthy_eps, state, replace_first=(cycle == 1))

    # ── Track dead ones ────────────────────────────────────────────────
    if dead:
        dead_urls = [r[0].url for r in dead]
        mark_dead_in_db(dead_urls, state)

    # Clear fail counts for healthy endpoints
    for ep, ok, _ in healthy:
        if ep.url in state.fail_counts:
            del state.fail_counts[ep.url]

    # ── Top performers ─────────────────────────────────────────────────
    if healthy:
        sorted_h = sorted(healthy, key=lambda r: r[2] or 9999)[:15]
        log.info(f"  TOP 15 FASTEST:")
        for ep, _, lat in sorted_h:
            log.info(f"    {lat:.0f}ms  {ep.provider:>14}  {ep.chain_id:<18} {ep.url}")

    # ── Provider stats ─────────────────────────────────────────────────
    provs: dict[str, dict] = defaultdict(lambda: {"ok": 0, "fail": 0, "lats": []})
    for ep, ok, lat in results:
        p = provs[ep.provider]
        if ok:
            p["ok"] += 1
            if lat:
                p["lats"].append(lat)
        else:
            p["fail"] += 1

    log.info(f"  BY PROVIDER:")
    for prov in sorted(provs.keys(), key=lambda p: provs[p]["ok"], reverse=True)[:12]:
        s = provs[prov]
        avg = sum(s["lats"]) / len(s["lats"]) if s["lats"] else 0
        log.info(f"    {prov:>14}: {s['ok']:>3} ok / {s['fail']:<3} fail  avg {avg:.0f}ms")

    # ── Source breakdown ───────────────────────────────────────────────
    by_source = defaultdict(int)
    for ep, ok, _ in healthy:
        by_source[ep.source] += 1
    if by_source:
        log.info(f"  BY SOURCE: {dict(by_source)}")

    state.total_discovered = len(deduped)

    # ── Step 6: Airdrop, Faucet, Grant, and Free Coin Discovery ────────
    log.info(f"  [6/7] Airdrop, Faucet, Grant, and Free Coin Discovery...")

    # Seed known faucets (idempotent — uses INSERT OR IGNORE)
    seed_faucets_to_db(state)

    if HAS_AIOHTTP:
        async with aiohttp.ClientSession() as session2:
            limiter2 = DomainRateLimiter(1.5)

            # Scrape airdrop aggregators
            try:
                found_airdrops = await scrape_airdrops(session2, limiter2)
                if found_airdrops:
                    seed_airdrops_to_db(found_airdrops)
                    log.info(f"  [6/7] Found {len(found_airdrops)} airdrop candidates from aggregators")
            except Exception as e:
                log.debug(f"  Airdrop scraping failed: {e}")

            # Dork for airdrops, grants, and free coins (if aggressive)
            if aggressive:
                try:
                    dork_results = await dork_for_airdrops(session2, limiter2, state)
                    grant_results = await dork_for_grants(session2, limiter2, state)
                    free_coin_results = await dork_for_free_coins(session2, limiter2, state)
                    all_dorked = dork_results + grant_results + free_coin_results
                    if all_dorked:
                        seed_airdrops_to_db(all_dorked)  # Using airdrops table for simplicity; extend to grants if needed
                        log.info(f"  [6/7] Found {len(all_dorked)} candidates from dorks (airdrops, grants, free coins)")
                except Exception as e:
                    log.debug(f"  Dorking failed: {e}")

            # Scrape social networks for opportunities including referrals
            try:
                social_results = await scrape_social_for_opportunities(session2, limiter2)
                if social_results:
                    seed_airdrops_to_db(social_results)
                    log.info(f"  [6/7] Found {len(social_results)} opportunities from social networks")
            except Exception as e:
                log.debug(f"  Social scraping failed: {e}")

            # Dork for referral programs
            try:
                referral_results = await dork_for_referrals(session2, limiter2, state)
                if referral_results:
                    seed_referrals_to_db(referral_results)
                    log.info(f"  [6/7] Found {len(referral_results)} referral programs from dorks")
            except Exception as e:
                log.debug(f"  Referral dorking failed: {e}")

            # ── Step 7: Chain Discovery ────────────────────────────────
            log.info(f"  [7/7] Chain Discovery (new chains, testnets)...")
            try:
                await detect_new_chains_from_chainlist(session2, limiter2)
            except Exception as e:
                log.debug(f"  Chain discovery failed: {e}")

            # Optimize dorks using OpenRouter LLM
            try:
                optimized_dorks = await optimize_dorks_with_llm(session2, limiter2, GOOGLE_DORKS[:5])
                log.info(f"  [7/7] Optimized {len(optimized_dorks)} dorks using LLM")
                # Could update GOOGLE_DORKS or use them in next cycle
            except Exception as e:
                log.debug(f"  LLM dork optimization failed: {e}")

            # ── Step 8: LLM Endpoint Validation ────────────────────────────────
            log.info(f"  [8/8] LLM Endpoint Validation...")
            await validate_llm_endpoints()

    else:
        log.info(f"  [6-7] Skipped (no aiohttp)")

    # Save state
    state.save(STATE_FILE)
    log.info(f"  State saved to {STATE_FILE}")


async def dork_for_grants(
    session: "aiohttp.ClientSession",
    limiter: DomainRateLimiter,
    state: CrawlerState,
) -> list[dict]:
    """Use Google dorks to discover crypto-related grants."""
    grants = []
    grant_dorks = [
        '"crypto grant" "application" "open" 2025',
        '"blockchain grant" "funding" "apply now" crypto',
        '"web3 grant" "program" "eligible" 2025',
        'site:gitcoin.co "grant" active',
        '"foundation grant" "crypto" "proposal" submit',
    ]
    for dork in grant_dorks:
        try:
            urls = await google_dork_search(session, limiter, dork, num_results=10)
            for url in urls:
                grants.append({
                    "name": f"Grant from: {urlparse(url).netloc}",
                    "source_url": url,
                    "source": "crawler",
                    "chain_id": "unknown",
                    "airdrop_type": "grant",
                })
            await asyncio.sleep(2)
        except Exception as e:
            log.debug(f"  Grant dork failed: {e}")
    return grants


async def dork_for_free_coins(
    session: "aiohttp.ClientSession",
    limiter: DomainRateLimiter,
    state: CrawlerState,
) -> list[dict]:
    """Use Google dorks to discover free coin opportunities."""
    free_coins = []
    free_dorks = [
        '"free crypto" "claim" "tokens" 2025',
        '"free coins" "giveaway" "airdrop" OR "faucet" crypto',
        '"testnet tokens" "free" "claim" blockchain',
        '"promo" "free tokens" "new users" crypto',
    ]
    for dork in free_dorks:
        try:
            urls = await google_dork_search(session, limiter, dork, num_results=10)
            for url in urls:
                free_coins.append({
                    "name": f"Free coins from: {urlparse(url).netloc}",
                    "source_url": url,
                    "source": "crawler",
                    "chain_id": "unknown",
                    "airdrop_type": "free_coin",
                })
            await asyncio.sleep(2)
        except Exception as e:
            log.debug(f"  Free coin dork failed: {e}")
    return free_coins


async def scrape_social_for_opportunities(
    session: "aiohttp.ClientSession",
    limiter: DomainRateLimiter,
) -> list[dict]:
    """Scrape social networks (Twitter, Reddit) for airdrop/grant mentions using public APIs."""
    opportunities = []

    # Twitter search (using public RSS or nitter-like, no auth)
    twitter_search = "https://nitter.net/search/rss?f=tweets&q=airdrop+claim+crypto+OR+grant+funding+blockchain"
    try:
        await limiter.wait(twitter_search)
        async with session.get(twitter_search, timeout=aiohttp.ClientTimeout(total=15)) as resp:
            if resp.status == 200:
                xml = await resp.text()
                import xml.etree.ElementTree as ET
                root = ET.fromstring(xml)
                for item in root.findall(".//item"):
                    title = item.find("title").text
                    link = item.find("link").text
                    if "airdrop" in title.lower() or "grant" in title.lower():
                        opportunities.append({
                            "name": title[:100],
                            "source_url": link,
                            "source": "twitter",
                            "chain_id": _infer_chain_from_text(title),
                            "airdrop_type": "social",
                        })
    except Exception as e:
        log.debug(f"  Twitter scrape failed: {e}")

    # Reddit search
    reddit_search = "https://www.reddit.com/r/cryptocurrency/search.json?q=airdrop+OR+grant+OR+free+coins&restrict_sr=on&sort=new&limit=10"
    try:
        await limiter.wait(reddit_search)
        async with session.get(reddit_search, headers={"User-Agent": "AtlasCrawler/1.0"}, timeout=aiohttp.ClientTimeout(total=15)) as resp:
            if resp.status == 200:
                data = await resp.json()
                posts = data.get("data", {}).get("children", [])
                for post in posts:
                    title = post["data"]["title"]
                    url = "https://reddit.com" + post["data"]["permalink"]
                    if any(kw in title.lower() for kw in ["airdrop", "grant", "free coins", "faucet"]):
                        opportunities.append({
                            "name": title[:100],
                            "source_url": url,
                            "source": "reddit",
                            "chain_id": _infer_chain_from_text(title),
                            "airdrop_type": "social",
                        })
    except Exception as e:
        log.debug(f"  Reddit scrape failed: {e}")

    return opportunities


async def optimize_dorks_with_llm(
    session: "aiohttp.ClientSession",
    limiter: DomainRateLimiter,
    dorks: list[str],
) -> list[str]:
    """Use OpenRouter LLM to optimize search dorks."""
    optimized = []
    openrouter_url = "https://openrouter.ai/api/v1/chat/completions"  # Assuming configured API key in env
    api_key = os.environ.get("OPENROUTER_API_KEY")
    if not api_key:
        log.warning("  No OPENROUTER_API_KEY — skipping LLM optimization")
        return dorks

    for dork in dorks:
        try:
            prompt = f"Optimize this Google dork for finding crypto airdrops and grants: {dork}. Make it more effective and specific."
            payload = {
                "model": "gpt-3.5-turbo",
                "messages": [{"role": "user", "content": prompt}],
            }
            headers = {
                "Authorization": f"Bearer {api_key}",
                "Content-Type": "application/json",
            }
            await limiter.wait(openrouter_url)
            async with session.post(openrouter_url, json=payload, headers=headers, timeout=aiohttp.ClientTimeout(total=30)) as resp:
                if resp.status == 200:
                    data = await resp.json()
                    optimized_dork = data["choices"][0]["message"]["content"].strip()
                    optimized.append(optimized_dork)
                else:
                    optimized.append(dork)
        except Exception as e:
            log.debug(f"  LLM optimization failed for dork: {e}")
            optimized.append(dork)
    return optimized


async def dork_for_referrals(
    session: "aiohttp.ClientSession",
    limiter: DomainRateLimiter,
    state: CrawlerState,
) -> list[dict]:
    """Use Google dorks to discover crypto referral programs."""
    referrals = []
    referral_dorks = [
        '"crypto referral" "program" "earn" "rewards" 2025',
        '"blockchain referral" "bonus" "sign up" crypto',
        '"web3 referral" "code" "invite" 2025',
        'site:binance.com "referral" program',
        '"exchange referral" "crypto" "earn" commission',
    ]
    for dork in referral_dorks:
        try:
            urls = await google_dork_search(session, limiter, dork, num_results=10)
            for url in urls:
                referrals.append({
                    "name": f"Referral from: {urlparse(url).netloc}",
                    "url": url,
                    "provider": urlparse(url).netloc.split(".")[0],
                    "referral_number": hashlib.md5(url.encode()).hexdigest()[:8],  # Generate simple referral number
                    "plan": "Standard referral plan - earn commissions on invites",
                    "rules": "Invite friends, earn percentage of fees/trades",
                })
            await asyncio.sleep(2)
        except Exception as e:
            log.debug(f"  Referral dork failed: {e}")
    return referrals


def seed_referrals_to_db(referrals: list[dict]):
    """Insert discovered referrals into the referrals table."""
    if not os.path.exists(DB_PATH) or not referrals:
        return

    conn = sqlite3.connect(DB_PATH)
    cur = conn.cursor()
    inserted = 0

    for ref in referrals:
        try:
            # Check if already exists by url
            existing = cur.execute(
                "SELECT id FROM referrals WHERE url = ?",
                (ref["url"],)
            ).fetchone()
            if existing:
                continue

            cur.execute("""
                INSERT INTO referrals
                (name, provider, url, referral_number, plan, rules, status)
                VALUES (?, ?, ?, ?, ?, ?, 'discovered')
            """, (
                ref["name"],
                ref.get("provider"),
                ref["url"],
                ref.get("referral_number"),
                ref.get("plan"),
                ref.get("rules"),
            ))
            inserted += 1
        except Exception as e:
            log.debug(f"  Referral insert failed: {e}")

    conn.commit()
    conn.close()
    if inserted > 0:
        log.info(f"  Seeded {inserted} new referrals to DB")


def infer_chain_from_url(url: str) -> str:
    """Try to figure out which chain an RPC URL belongs to."""
    url_lower = url.lower()
    chain_hints = [
        ("ethereum", "eth"), ("bsc", "bsc"), ("bnb", "bsc"), ("binance", "bsc"),
        ("polygon", "polygon"), ("matic", "polygon"), ("arbitrum", "arb-one"),
        ("optimism", "optimism"), ("avalanche", "avax"), ("avax", "avax"),
        ("base", "base"), ("fantom", "ftm"), ("ftm", "ftm"),
        ("gnosis", "gnosis"), ("xdai", "gnosis"), ("solana", "sol"),
        ("linea", "linea"), ("scroll", "scroll"), ("zksync", "zksync"),
        ("mantle", "mantle"), ("blast", "blast"), ("moonbeam", "moonbeam"),
        ("moonriver", "moonriver"), ("cronos", "cronos"), ("aurora", "aurora"),
        ("harmony", "harmony"), ("metis", "metis"), ("kava", "kava"),
        ("celo", "celo"), ("near", "near"), ("tron", "tron"), ("ton", "ton"),
        ("sui", "sui"), ("aptos", "aptos"), ("polkadot", "polkadot"),
        ("kusama", "kusama"), ("cosmos", "cosmos"), ("osmosis", "osmosis"),
        ("filecoin", "filecoin"), ("mode", "mode"), ("manta", "manta"),
        ("zkevm", "polygon-zkevm"), ("taiko", "taiko"), ("sonic", "sonic"),
        ("berachain", "berachain"), ("opbnb", "opbnb"), ("sei", "sei"),
        ("celestia", "celestia"), ("injective", "injective"),
        ("sepolia", "sepolia"), ("holesky", "holesky"), ("goerli", "goerli"),
        ("testnet", "unknown-testnet"), ("devnet", "unknown-devnet"),
    ]
    for hint, chain_id in chain_hints:
        if hint in url_lower:
            return chain_id
    return "unknown"


def infer_chain_from_url(url: str) -> str:
    """Try to figure out which chain an RPC URL belongs to."""
    url_lower = url.lower()
    chain_hints = [
        ("ethereum", "eth"), ("bsc", "bsc"), ("bnb", "bsc"), ("binance", "bsc"),
        ("polygon", "polygon"), ("matic", "polygon"), ("arbitrum", "arb-one"),
        ("optimism", "optimism"), ("avalanche", "avax"), ("avax", "avax"),
        ("base", "base"), ("fantom", "ftm"), ("ftm", "ftm"),
        ("gnosis", "gnosis"), ("xdai", "gnosis"), ("solana", "sol"),
        ("linea", "linea"), ("scroll", "scroll"), ("zksync", "zksync"),
        ("mantle", "mantle"), ("blast", "blast"), ("moonbeam", "moonbeam"),
        ("moonriver", "moonriver"), ("cronos", "cronos"), ("aurora", "aurora"),
        ("harmony", "harmony"), ("metis", "metis"), ("kava", "kava"),
        ("celo", "celo"), ("near", "near"), ("tron", "tron"), ("ton", "ton"),
        ("sui", "sui"), ("aptos", "aptos"), ("polkadot", "polkadot"),
        ("kusama", "kusama"), ("cosmos", "cosmos"), ("osmosis", "osmosis"),
        ("filecoin", "filecoin"), ("mode", "mode"), ("manta", "manta"),
        ("zkevm", "polygon-zkevm"), ("taiko", "taiko"), ("sonic", "sonic"),
        ("berachain", "berachain"), ("opbnb", "opbnb"), ("sei", "sei"),
        ("celestia", "celestia"), ("injective", "injective"),
        ("sepolia", "sepolia"), ("holesky", "holesky"), ("goerli", "goerli"),
        ("testnet", "unknown-testnet"), ("devnet", "unknown-devnet"),
    ]
    for hint, chain_id in chain_hints:
        if hint in url_lower:
            return chain_id
    return "unknown"


# ═════════════════════════════════════════════════════════════════════════
# GRACEFUL SHUTDOWN
# ═════════════════════════════════════════════════════════════════════════

_shutdown_event = asyncio.Event()


def _handle_signal(signum, frame):
    sig_name = signal.Signals(signum).name
    log.info(f"\n  Received {sig_name} — saving state and shutting down...")
    _shutdown_event.set()


# ═════════════════════════════════════════════════════════════════════════
# MAIN LOOP
# ═════════════════════════════════════════════════════════════════════════

async def main_loop(interval_minutes: int = 15, once: bool = False, aggressive: bool = False):
    """Main daemon loop."""
    # Register signal handlers
    signal.signal(signal.SIGTERM, _handle_signal)
    signal.signal(signal.SIGINT, _handle_signal)

    # Load state
    state = CrawlerState.load(STATE_FILE)
    log.info(f"═══════════════════════════════════════════════════════════")
    log.info(f"  RPC CRAWLER DAEMON")
    log.info(f"  DB: {DB_PATH}")
    log.info(f"  State: {STATE_FILE}")
    log.info(f"  Interval: {interval_minutes}min {'(single run)' if once else '(continuous)'}")
    log.info(f"  Aggressive: {aggressive}")
    log.info(f"  Resuming from cycle #{state.cycle_count}")
    log.info(f"  Banned URLs: {len(state.banned_urls)}")
    log.info(f"  Google dork index: {state.google_dork_index}/{len(GOOGLE_DORKS)}")
    log.info(f"═══════════════════════════════════════════════════════════")

    while not _shutdown_event.is_set():
        try:
            await run_cycle(state, aggressive=aggressive)
        except Exception as e:
            log.error(f"  Cycle error: {e}", exc_info=True)
            state.save(STATE_FILE)

        if once:
            break

        # Wait for next cycle or shutdown
        log.info(f"  Next cycle in {interval_minutes} minutes (Ctrl+C to stop)...")
        try:
            await asyncio.wait_for(_shutdown_event.wait(), timeout=interval_minutes * 60)
        except asyncio.TimeoutError:
            pass  # Normal — timeout means it's time for next cycle

        if _shutdown_event.is_set():
            break

    # Final save
    state.running = False
    state.save(STATE_FILE)
    log.info(f"  Final state saved. Total cycles: {state.cycle_count}")
    log.info(f"  Total discovered: {state.total_discovered} | Healthy: {state.total_healthy}")
    log.info(f"  Banned: {len(state.banned_urls)}")
    log.info(f"  Goodbye!")


def main():
    parser = argparse.ArgumentParser(description="RPC Crawler Daemon — Persistent RPC discovery")
    parser.add_argument("--interval", type=int, default=15, help="Minutes between cycles (default: 15)")
    parser.add_argument("--once", action="store_true", help="Run one cycle then exit")
    parser.add_argument("--aggressive", action="store_true", help="Enable Google dorks + deep scraping")
    parser.add_argument("--reset", action="store_true", help="Reset saved state before starting")
    args = parser.parse_args()

    if args.reset and os.path.exists(STATE_FILE):
        os.remove(STATE_FILE)
        log.info("State file reset")

    if not HAS_AIOHTTP:
        log.error("aiohttp is required: pip install aiohttp")
        sys.exit(1)

    asyncio.run(main_loop(
        interval_minutes=args.interval,
        once=args.once,
        aggressive=args.aggressive,
    ))


async def validate_llm_endpoints():
    """Validate all LLM endpoints in the database and update their status."""
    if not os.path.exists(DB_PATH):
        log.warning("  DB not found — skipping LLM validation")
        return

    conn = sqlite3.connect(DB_PATH)
    cur = conn.cursor()

    # Fetch all endpoints
    cur.execute("SELECT id, url, provider FROM llm_endpoints")
    endpoints = cur.fetchall()

    if not endpoints:
        log.info("  No LLM endpoints to validate")
        conn.close()
        return

    log.info(f"  Validating {len(endpoints)} LLM endpoints...")

    limiter = DomainRateLimiter(max_per_second=2.0)
    async with aiohttp.ClientSession() as session:
        for ep_id, url, provider in endpoints:
            try:
                await limiter.wait(url)
                t0 = time.monotonic()

                # Validation endpoint depends on provider
                if provider == 'ollama':
                    validate_url = f"{url.rstrip('/')}/api/tags"
                else:
                    validate_url = f"{url.rstrip('/')}/v1/models"

                async with session.get(validate_url, timeout=aiohttp.ClientTimeout(total=5), ssl=False) as resp:
                    lat = (time.monotonic() - t0) * 1000
                    is_healthy = resp.status == 200
                    models = []
                    version = None
                    if is_healthy:
                        data = await resp.json()
                        if provider == 'ollama':
                            models = [m['name'] for m in data.get('models', [])]
                            version = data.get('version')
                        else:
                            models = [m['id'] for m in data.get('data', [])]
                            version = data.get('version')

                    # Update DB
                    cur.execute("""
                        UPDATE llm_endpoints SET
                            is_healthy = ?,
                            latency_ms = ?,
                            last_checked = CURRENT_TIMESTAMP,
                            models = ?,
                            version = ?
                        WHERE id = ?
                    """, (1 if is_healthy else 0, lat, json.dumps(models), version, ep_id))

                    if not is_healthy:
                        # If failed 3 times, delete
                        cur.execute("SELECT fail_count FROM llm_endpoints WHERE id = ?", (ep_id,))
                        fail_count = cur.fetchone()[0] or 0
                        fail_count += 1
                        cur.execute("UPDATE llm_endpoints SET fail_count = ? WHERE id = ?", (fail_count, ep_id))
                        if fail_count >= 3:
                            cur.execute("DELETE FROM llm_endpoints WHERE id = ?", (ep_id,))
                            log.info(f"  Deleted unhealthy LLM endpoint: {url}")

            except Exception as e:
                log.debug(f"  Validation failed for {url}: {e}")
                cur.execute("""
                    UPDATE llm_endpoints SET
                        is_healthy = 0,
                        last_checked = CURRENT_TIMESTAMP
                    WHERE id = ?
                """, (ep_id,))

    conn.commit()
    conn.close()
    log.info("  LLM validation complete")

if __name__ == "__main__":
    main()
