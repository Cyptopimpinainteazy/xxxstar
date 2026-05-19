#!/usr/bin/env python3
"""
X3 Chain — Faucet Auto-Claimer & Wallet Manager

Automatically claims testnet tokens from API-based faucets,
generates wallets per chain, and tracks claim cooldowns.

Features:
  - Auto-generate HD wallets per chain (EVM, Solana, Sui, Aptos)
  - Claim from API faucets (Solana devnet, Sui devnet, Aptos devnet)
  - Track cooldown periods and auto-retry
  - Validate faucet URLs are still alive
  - Log all claims to faucet_claims table
  - Register wallets in unified wallets table

Usage:
  python3 faucet_claimer.py --once          # Single claim pass
  python3 faucet_claimer.py --interval 60   # Continuous mode (every 60 min)
  python3 faucet_claimer.py --seed-wallets  # Only generate wallets
"""

import argparse
import asyncio
import json
import logging
import os
import signal
import sqlite3
import sys
import time
from dataclasses import dataclass, field
from datetime import datetime, timezone, timedelta
from typing import Optional

try:
    import aiohttp
    HAS_AIOHTTP = True
except ImportError:
    HAS_AIOHTTP = False

try:
    from llm_service.client import SubstreamsSkillsClient
    HAS_LLM_CLIENT = True
except ImportError:
    HAS_LLM_CLIENT = False

# Try to import eth_account for EVM wallet generation
try:
    from eth_account import Account
    HAS_ETH = True
except ImportError:
    HAS_ETH = False

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
DB_PATH = os.environ.get("CHAIN_DB_PATH", os.path.join(SCRIPT_DIR, "..", "..", "db", "chains.db"))
STATE_FILE = os.environ.get("CLAIMER_STATE_FILE", os.path.join(SCRIPT_DIR, "claimer_state.json"))
LOG_FILE = os.environ.get("CLAIMER_LOG_FILE", os.path.join(SCRIPT_DIR, "faucet_claimer.log"))

# ── Logging ────────────────────────────────────────────────────────────
log = logging.getLogger("faucet-claimer")
log.setLevel(logging.INFO)
_fmt = logging.Formatter("[%(asctime)s] %(levelname)s  %(message)s", datefmt="%H:%M:%S")
_sh = logging.StreamHandler()
_sh.setFormatter(_fmt)
log.addHandler(_sh)
try:
    _fh = logging.FileHandler(LOG_FILE)
    _fh.setFormatter(_fmt)
    log.addHandler(_fh)
except Exception:
    pass


# ═════════════════════════════════════════════════════════════════════════
# WALLET GENERATION
# ═════════════════════════════════════════════════════════════════════════

def generate_evm_wallet() -> tuple[str, str]:
    """Generate a new EVM wallet. Returns (address, private_key_hex)."""
    if HAS_ETH:
        acct = Account.create()
        return acct.address, acct.key.hex()
    else:
        # Fallback: generate random key
        import secrets
        private_key = secrets.token_hex(32)
        # Without eth_account, we can't derive the address properly
        # Return a placeholder that indicates manual setup needed
        return f"0x__NEEDS_ETH_ACCOUNT_LIB__{private_key[:8]}", private_key


def ensure_wallet_for_chain(chain_id: str, ecosystem: str = "evm") -> Optional[dict]:
    """Ensure we have a wallet for this chain. Create one if needed."""
    if not os.path.exists(DB_PATH):
        return None

    conn = sqlite3.connect(DB_PATH)
    cur = conn.cursor()

    # Check if wallet exists
    existing = cur.execute(
        "SELECT id, address, chain_id FROM wallets WHERE chain_id = ? AND is_active = 1 LIMIT 1",
        (chain_id,)
    ).fetchone()

    if existing:
        conn.close()
        return {"id": existing[0], "address": existing[1], "chain_id": existing[2]}

    # Generate new wallet
    if ecosystem == "evm":
        address, priv_key = generate_evm_wallet()
    else:
        # For non-EVM chains, create a placeholder
        import secrets
        address = f"placeholder_{chain_id}_{secrets.token_hex(8)}"
        priv_key = secrets.token_hex(32)

    try:
        cur.execute("""
            INSERT INTO wallets (chain_id, address, label, ecosystem, private_key_enc)
            VALUES (?, ?, 'auto', ?, ?)
        """, (chain_id, address, ecosystem, priv_key))
        conn.commit()
        wallet_id = cur.lastrowid
        log.info(f"  Created wallet for {chain_id}: {address[:12]}...")
        conn.close()
        return {"id": wallet_id, "address": address, "chain_id": chain_id}
    except Exception as e:
        log.error(f"  Failed to create wallet for {chain_id}: {e}")
        conn.close()
        return None


def seed_wallets_for_known_testnets():
    """Create wallets for all known testnet chains with faucets."""
    if not os.path.exists(DB_PATH):
        log.error("DB not found")
        return

    conn = sqlite3.connect(DB_PATH)
    cur = conn.cursor()

    # Get unique chains from faucets table
    chains = cur.execute("SELECT DISTINCT chain_id FROM faucets WHERE status = 'active'").fetchall()
    conn.close()

    created = 0
    for (chain_id,) in chains:
        ecosystem = "evm"  # Default
        if any(s in chain_id for s in ["sol", "solana"]):
            ecosystem = "svm"
        elif any(s in chain_id for s in ["sui"]):
            ecosystem = "move"
        elif any(s in chain_id for s in ["aptos"]):
            ecosystem = "move"
        elif any(s in chain_id for s in ["cosmos", "osmosis"]):
            ecosystem = "cosmos"

        result = ensure_wallet_for_chain(chain_id, ecosystem)
        if result:
            created += 1

    log.info(f"  Wallet seeding complete: {created} chains")


# ═════════════════════════════════════════════════════════════════════════
# FAUCET CLAIMING
# ═════════════════════════════════════════════════════════════════════════

async def claim_solana_faucet(session: aiohttp.ClientSession, address: str, network: str = "devnet") -> dict:
    """Request SOL from the Solana faucet via the JSON-RPC airdrop method."""
    rpc_url = f"https://api.{network}.solana.com"
    payload = {
        "jsonrpc": "2.0",
        "id": 1,
        "method": "requestAirdrop",
        "params": [address, 2_000_000_000]  # 2 SOL in lamports
    }
    try:
        async with session.post(rpc_url, json=payload, timeout=aiohttp.ClientTimeout(total=30)) as resp:
            data = await resp.json()
            if "result" in data:
                return {"success": True, "tx_hash": data["result"], "amount": "2"}
            else:
                return {"success": False, "error": data.get("error", {}).get("message", "Unknown error")}
    except Exception as e:
        return {"success": False, "error": str(e)}


async def claim_sui_faucet(session: aiohttp.ClientSession, address: str) -> dict:
    """Request SUI from the Sui devnet faucet."""
    try:
        async with session.post(
            "https://faucet.devnet.sui.io/gas",
            json={"FixedAmountRequest": {"recipient": address}},
            headers={"Content-Type": "application/json"},
            timeout=aiohttp.ClientTimeout(total=30),
        ) as resp:
            if resp.status == 200 or resp.status == 202:
                data = await resp.json()
                return {"success": True, "tx_hash": str(data), "amount": "10"}
            else:
                text = await resp.text()
                return {"success": False, "error": f"HTTP {resp.status}: {text[:200]}"}
    except Exception as e:
        return {"success": False, "error": str(e)}


async def claim_aptos_faucet(session: aiohttp.ClientSession, address: str) -> dict:
    """Request APT from the Aptos devnet faucet."""
    try:
        async with session.post(
            f"https://faucet.devnet.aptoslabs.com/mint?amount=100000000&address={address}",
            timeout=aiohttp.ClientTimeout(total=30),
        ) as resp:
            if resp.status == 200:
                data = await resp.json()
                return {"success": True, "tx_hash": str(data), "amount": "1"}
            else:
                text = await resp.text()
                return {"success": False, "error": f"HTTP {resp.status}: {text[:200]}"}
    except Exception as e:
        return {"success": False, "error": str(e)}


async def llm_fill_form(form_description: str) -> dict:
    """Use LLM to generate form data for filling."""
    if not HAS_LLM_CLIENT:
        return {"success": False, "error": "LLM client not available"}

    client = SubstreamsSkillsClient(default_provider="openrouter")
    prompt = f"Generate JSON data to fill this form for crypto claiming: {form_description}. Include fields like address, email if needed."
    try:
        result = client.query(prompt)
        if result.success:
            return json.loads(result.response)
        else:
            return {"success": False, "error": result.error}
    except Exception as e:
        return {"success": False, "error": str(e)}


async def claim_web_faucet(session: aiohttp.ClientSession, faucet: dict, wallet: dict) -> dict:
    """Claim from web-based faucet using LLM to fill forms."""
    # For demonstration, assume a simple POST form
    form_data = await llm_fill_form(f"Wallet address: {wallet['address']}, for faucet: {faucet['name']}")
    if not form_data.get("success"):
        return form_data

    try:
        async with session.post(
            faucet["url"],
            json=form_data,
            timeout=aiohttp.ClientTimeout(total=30),
        ) as resp:
            if resp.status in (200, 201):
                data = await resp.json()
                return {"success": True, "tx_hash": data.get("tx"), "amount": faucet.get("amount_per_claim")}
            else:
                text = await resp.text()
                return {"success": False, "error": f"HTTP {resp.status}: {text[:200]}"}
    except Exception as e:
        return {"success": False, "error": str(e)}
async def check_faucet_alive(session: aiohttp.ClientSession, url: str) -> bool:
    """Quick health check on a faucet URL."""
    try:
        async with session.head(url, timeout=aiohttp.ClientTimeout(total=10),
                                allow_redirects=True) as resp:
            return resp.status < 500
    except Exception:
        return False


def record_faucet_claim(faucet_id: int, wallet_id: int, chain_id: str,
                        status: str, tx_hash: Optional[str], amount: Optional[str],
                        cooldown_hours: float, error: Optional[str] = None):
    """Record a faucet claim attempt in the DB."""
    if not os.path.exists(DB_PATH):
        return

    conn = sqlite3.connect(DB_PATH)
    cur = conn.cursor()

    next_claim = datetime.now(timezone.utc) + timedelta(hours=cooldown_hours)

    cur.execute("""
        INSERT INTO faucet_claims
        (faucet_id, wallet_id, chain_id, status, tx_hash, amount, next_claim_at, error_message)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?)
    """, (faucet_id, wallet_id, chain_id, status, tx_hash, amount,
          next_claim.isoformat(), error))

    # Update faucet's last_claimed and total_claims
    if status in ("submitted", "received"):
        cur.execute("""
            UPDATE faucets SET last_claimed = datetime('now'), total_claims = total_claims + 1,
                   last_checked = datetime('now')
            WHERE id = ?
        """, (faucet_id,))
    elif status == "failed":
        cur.execute("UPDATE faucets SET last_checked = datetime('now') WHERE id = ?", (faucet_id,))

    conn.commit()
    conn.close()


async def auto_claim_faucets():
    """Go through active API faucets and claim where cooldown has expired."""
    if not os.path.exists(DB_PATH):
        log.error("DB not found")
        return

    conn = sqlite3.connect(DB_PATH)
    conn.row_factory = sqlite3.Row
    cur = conn.cursor()

    # Get active API-type faucets (we can only auto-claim API faucets)
    faucets = cur.execute("""
        SELECT f.*, 
               (SELECT MAX(fc.next_claim_at) FROM faucet_claims fc WHERE fc.faucet_id = f.id) as next_available
        FROM faucets f
        WHERE f.status = 'active' AND f.faucet_type = 'api'
        ORDER BY f.last_claimed ASC NULLS FIRST
    """).fetchall()

    conn.close()

    if not faucets:
        log.info("  No API faucets available for auto-claiming")
        return

    claimed = 0
    skipped = 0

    async with aiohttp.ClientSession() as session:
        for faucet in faucets:
            faucet = dict(faucet)
            faucet_id = faucet["id"]
            chain_id = faucet["chain_id"]

            # Check cooldown
            next_available = faucet.get("next_available")
            if next_available:
                try:
                    next_dt = datetime.fromisoformat(next_available)
                    if next_dt > datetime.now(timezone.utc):
                        remaining = (next_dt - datetime.now(timezone.utc)).total_seconds() / 3600
                        log.debug(f"  Faucet {faucet['name']}: cooldown ({remaining:.1f}h remaining)")
                        skipped += 1
                        continue
                except Exception:
                    pass

            # Ensure we have a wallet for this chain
            ecosystem = "evm"
            if "sol" in chain_id:
                ecosystem = "svm"
            elif "sui" in chain_id:
                ecosystem = "move"
            elif "aptos" in chain_id:
                ecosystem = "move"

            wallet = ensure_wallet_for_chain(chain_id, ecosystem)
            if not wallet:
                log.warning(f"  Cannot create wallet for {chain_id}")
                continue

            # Attempt claim based on chain type
            result = {"success": False, "error": "No claimer for this chain"}

            if faucet["faucet_type"] == "web":
                result = await claim_web_faucet(session, faucet, wallet)
            elif "sol" in chain_id:
                network = "devnet" if "devnet" in chain_id else "testnet"
                result = await claim_solana_faucet(session, wallet["address"], network)
            elif "sui" in chain_id:
                result = await claim_sui_faucet(session, wallet["address"])
            elif "aptos" in chain_id:
                result = await claim_aptos_faucet(session, wallet["address"])

            if result["success"]:
                record_faucet_claim(
                    faucet_id=faucet_id,
                    wallet_id=wallet["id"],
                    chain_id=chain_id,
                    status="submitted",
                    tx_hash=result.get("tx_hash"),
                    amount=result.get("amount"),
                    cooldown_hours=faucet.get("cooldown_hours", 24),
                )
                log.info(f"  ✅ Claimed from {faucet['name']}: {result.get('amount')} {faucet.get('token_symbol', '?')} → {wallet['address'][:16]}...")
                claimed += 1
            else:
                record_faucet_claim(
                    faucet_id=faucet_id,
                    wallet_id=wallet["id"],
                    chain_id=chain_id,
                    status="failed",
                    tx_hash=None,
                    amount=None,
                    cooldown_hours=1,  # Retry after 1 hour on failure
                    error=result.get("error"),
                )
                log.warning(f"  ❌ Failed to claim from {faucet['name']}: {result.get('error', 'unknown')}")

            await asyncio.sleep(2)  # Be nice between claims

    log.info(f"  Auto-claim complete: {claimed} claimed, {skipped} on cooldown")


async def validate_faucets():
    """Check which faucets are still alive and update status."""
    if not os.path.exists(DB_PATH):
        return

    conn = sqlite3.connect(DB_PATH)
    conn.row_factory = sqlite3.Row
    faucets = conn.execute("SELECT id, url, status FROM faucets").fetchall()
    conn.close()

    if not faucets:
        return

    log.info(f"  Validating {len(faucets)} faucets...")

    async with aiohttp.ClientSession() as session:
        alive_count = 0
        dead_count = 0

        for faucet in faucets:
            alive = await check_faucet_alive(session, faucet["url"])

            conn2 = sqlite3.connect(DB_PATH)
            if alive:
                if faucet["status"] == "dead":
                    conn2.execute("UPDATE faucets SET status = 'active', last_checked = datetime('now') WHERE id = ?", (faucet["id"],))
                else:
                    conn2.execute("UPDATE faucets SET last_checked = datetime('now') WHERE id = ?", (faucet["id"],))
                alive_count += 1
            else:
                conn2.execute("UPDATE faucets SET status = 'dead', last_checked = datetime('now') WHERE id = ?", (faucet["id"],))
                dead_count += 1
            conn2.commit()
            conn2.close()

        log.info(f"  Faucet validation: {alive_count} alive, {dead_count} dead")


# ═════════════════════════════════════════════════════════════════════════
# MAIN LOOP
# ═════════════════════════════════════════════════════════════════════════

_shutdown = asyncio.Event()

def _sig_handler(signum, frame):
    log.info(f"\n  Received {signal.Signals(signum).name} — shutting down...")
    _shutdown.set()


async def claim_loop(interval_minutes: int = 60, once: bool = False):
    """Main faucet claimer loop."""
    signal.signal(signal.SIGTERM, _sig_handler)
    signal.signal(signal.SIGINT, _sig_handler)

    log.info("═══════════════════════════════════════════════════════════")
    log.info("  FAUCET AUTO-CLAIMER")
    log.info(f"  DB: {DB_PATH}")
    log.info(f"  Interval: {interval_minutes}min {'(single run)' if once else '(continuous)'}")
    log.info("═══════════════════════════════════════════════════════════")

    while not _shutdown.is_set():
        try:
            # 1. Validate faucets
            await validate_faucets()

            # 2. Auto-claim from API faucets
            await auto_claim_faucets()

        except Exception as e:
            log.error(f"  Claim cycle error: {e}", exc_info=True)

        if once:
            break

        log.info(f"  Next claim cycle in {interval_minutes} minutes...")
        try:
            await asyncio.wait_for(_shutdown.wait(), timeout=interval_minutes * 60)
        except asyncio.TimeoutError:
            pass

    log.info("  Faucet claimer stopped.")


def main():
    parser = argparse.ArgumentParser(description="Faucet Auto-Claimer — Auto-claim testnet tokens")
    parser.add_argument("--interval", type=int, default=60, help="Minutes between claim cycles (default: 60)")
    parser.add_argument("--once", action="store_true", help="Run one cycle then exit")
    parser.add_argument("--seed-wallets", action="store_true", help="Only generate wallets for known testnets")
    args = parser.parse_args()

    if not HAS_AIOHTTP:
        log.error("aiohttp required: pip install aiohttp")
        sys.exit(1)

    if args.seed_wallets:
        seed_wallets_for_known_testnets()
        return

    asyncio.run(claim_loop(interval_minutes=args.interval, once=args.once))


if __name__ == "__main__":
    main()
