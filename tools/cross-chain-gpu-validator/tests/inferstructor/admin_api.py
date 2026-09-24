#!/usr/bin/env python3
"""
Inferstructor Admin API — Comprehensive Admin Dashboard Backend

Features:
- Admin-only authentication (password + JWT, separate from validator auth)
- Real-time metrics aggregation from all services (5s polling)
- Historical metrics with ring buffer (1 hour retention)
- Service lifecycle management (start/stop/restart)
- Benchmark & stress test orchestration
- Admin action commands (throttle, pause, export)
- Cost & fee intelligence
- Audit logging
- Subscriber management (list, search, edit, enable/disable)
- Subscription tiers with rate limits
- Whitelist / Blacklist for IPs and validator IDs
- Accounting & usage analytics

Port: 7777
Auth: POST /admin/login with password → JWT token (8h expiry)
"""

import asyncio
import hashlib
import hmac
import json
import logging
import os
import signal
import sys
import time
from collections import deque
from dataclasses import dataclass, field
from typing import Dict, List, Optional

import jwt
from aiohttp import web
import aiohttp_cors
import aiohttp

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s [%(levelname)s] %(name)s: %(message)s",
)
logger = logging.getLogger("AdminAPI")

BASE_DIR = os.path.dirname(os.path.abspath(__file__))
VENV_PYTHON = os.path.join(BASE_DIR, "..", "..", ".venv", "bin", "python3")
if not os.path.exists(VENV_PYTHON):
    VENV_PYTHON = sys.executable

# ── Configuration ──────────────────────────────────────────
ADMIN_PASSWORD = os.environ.get("ADMIN_PASSWORD", "inferstructor-admin")
ADMIN_JWT_SECRET = os.environ.get(
    "ADMIN_JWT_SECRET", "inferstructor-admin-jwt-secret-2024"
)
ADMIN_JWT_EXPIRY = 8 * 3600  # 8 hours

GPU_POWER_WATTS = 150  # Per GTX 1070
GPU_COUNT = 3
ELECTRICITY_RATE = 0.12  # $/kWh
MAX_THEORETICAL_TPS = 960_000


# ═══════════════════════════════════════════════════════════
#  Admin Authentication
# ═══════════════════════════════════════════════════════════

class AdminAuth:
    """Password-based admin authentication with JWT tokens."""

    def __init__(self):
        self.audit_log: deque = deque(maxlen=500)

    def verify_password(self, password: str) -> bool:
        return hmac.compare_digest(password.encode(), ADMIN_PASSWORD.encode())

    def create_token(self) -> str:
        payload = {
            "role": "admin",
            "iat": int(time.time()),
            "exp": int(time.time()) + ADMIN_JWT_EXPIRY,
        }
        return jwt.encode(payload, ADMIN_JWT_SECRET, algorithm="HS256")

    def verify_request(self, request: web.Request) -> bool:
        auth = request.headers.get("Authorization", "")
        if not auth.startswith("Bearer "):
            return False
        token = auth[7:]
        try:
            payload = jwt.decode(token, ADMIN_JWT_SECRET, algorithms=["HS256"])
            return payload.get("role") == "admin"
        except (jwt.ExpiredSignatureError, jwt.InvalidTokenError):
            return False

    def log_event(self, event: str, details: str = ""):
        entry = {
            "timestamp": time.time(),
            "event": event,
            "details": details,
        }
        self.audit_log.appendleft(entry)
        logger.info(f"AUDIT: {event} {details}")

    def get_audit_log(self, limit: int = 50) -> list:
        return list(self.audit_log)[:limit]


# ═══════════════════════════════════════════════════════════
#  Metrics Collector
# ═══════════════════════════════════════════════════════════

class MetricsCollector:
    """Background service that polls all infrastructure and aggregates metrics."""

    def __init__(self):
        self.history: deque = deque(maxlen=720)  # 1 hour at 5s intervals
        self.tps_history: deque = deque(maxlen=3600)
        self.peak_tps: float = 0
        self.latest: Optional[dict] = None
        self._running = False

    async def start(self):
        self._running = True
        logger.info("Metrics collector started (5s interval)")
        while self._running:
            try:
                await self._collect()
            except Exception as e:
                logger.warning(f"Metrics collection error: {e}")
            await asyncio.sleep(5)

    def stop(self):
        self._running = False

    async def _collect(self):
        snapshot = {
            "timestamp": time.time(),
            "services": {},
            "gpu_lanes": [],
            "bridge": None,
            "rpc_proxy": None,
            "gpu_verifier": None,
            "chain": None,
            "upstreams": [],
        }

        timeout = aiohttp.ClientTimeout(total=3)
        async with aiohttp.ClientSession(timeout=timeout) as session:
            # GPU Lanes
            for port in [9001, 9002, 9003]:
                try:
                    async with session.get(f"http://localhost:{port}/health") as resp:
                        if resp.status == 200:
                            data = await resp.json()
                            snapshot["gpu_lanes"].append(data)
                            snapshot["services"][f"gpu-{port}"] = "up"
                        else:
                            snapshot["services"][f"gpu-{port}"] = "down"
                except Exception:
                    snapshot["services"][f"gpu-{port}"] = "down"

            # Bridge
            try:
                async with session.get("http://localhost:9999/stats") as resp:
                    if resp.status == 200:
                        snapshot["bridge"] = await resp.json()
                        snapshot["services"]["bridge-9999"] = "up"
                    else:
                        snapshot["services"]["bridge-9999"] = "down"
            except Exception:
                snapshot["services"]["bridge-9999"] = "down"

            # RPC Proxy chain-stats
            try:
                async with session.get("http://localhost:8899/chain-stats") as resp:
                    if resp.status == 200:
                        data = await resp.json()
                        snapshot["rpc_proxy"] = data.get("proxy", {})
                        snapshot["chain"] = data.get("chain", {})
                        snapshot["upstreams"] = data.get("upstreams", [])
                        snapshot["gpu_verifier"] = data.get("gpu_verifier", {})
                        snapshot["services"]["rpc-8899"] = "up"
                    else:
                        snapshot["services"]["rpc-8899"] = "down"
            except Exception:
                snapshot["services"]["rpc-8899"] = "down"

            # Registry
            try:
                async with session.get("http://localhost:7001/health") as resp:
                    snapshot["services"]["registry-7001"] = (
                        "up" if resp.status == 200 else "down"
                    )
            except Exception:
                snapshot["services"]["registry-7001"] = "down"

        # ── Compute aggregated metrics ──
        bridge = snapshot["bridge"]
        current_tps = bridge.get("current_tps", 0) if bridge else 0
        if current_tps > self.peak_tps:
            self.peak_tps = current_tps

        lanes = snapshot["gpu_lanes"]
        total_gpu_txns = sum(
            l.get("stats", {}).get("total_txns", 0) for l in lanes
        )
        total_gpu_success = sum(
            l.get("stats", {}).get("total_success", 0) for l in lanes
        )
        total_gpu_failed = sum(
            l.get("stats", {}).get("total_failed", 0) for l in lanes
        )

        n_lanes = max(len(lanes), 1)
        avg_utilization = sum(
            l.get("gpu", {}).get("utilization", 0) for l in lanes
        ) / n_lanes
        avg_memory = sum(
            l.get("gpu", {}).get("memory_used_mb", 0) for l in lanes
        ) / n_lanes
        avg_temp = sum(
            l.get("gpu", {}).get("temperature_c", 0) for l in lanes
        ) / n_lanes

        uptime = bridge.get("uptime_seconds", 0) if bridge else 0
        bridge_received = bridge.get("total_received", 0) if bridge else 0
        bridge_forwarded = bridge.get("total_forwarded", 0) if bridge else 0
        bridge_failed = bridge.get("total_failed", 0) if bridge else 0

        rpc = snapshot.get("rpc_proxy") or {}
        verifier = snapshot.get("gpu_verifier") or {}

        # Cost
        power_kw = GPU_POWER_WATTS * GPU_COUNT / 1000
        cost_per_hour = power_kw * ELECTRICITY_RATE
        cost_per_tx = (
            cost_per_hour / (current_tps * 3600) if current_tps > 0 else 0
        )

        services_up = sum(1 for s in snapshot["services"].values() if s == "up")

        snapshot["aggregated"] = {
            "current_tps": round(current_tps, 1),
            "peak_tps": round(self.peak_tps, 1),
            "services_up": services_up,
            "services_total": len(snapshot["services"]),
            "total_gpu_txns": total_gpu_txns,
            "total_gpu_success": total_gpu_success,
            "total_gpu_failed": total_gpu_failed,
            "success_rate": round(
                total_gpu_success / max(total_gpu_txns, 1) * 100, 2
            ),
            "avg_gpu_utilization": round(avg_utilization, 1),
            "avg_gpu_memory_mb": round(avg_memory, 1),
            "avg_gpu_temp_c": round(avg_temp, 1),
            "bridge_received": bridge_received,
            "bridge_forwarded": bridge_forwarded,
            "bridge_failed": bridge_failed,
            "dropped_tx_pct": round(
                bridge_failed / max(bridge_received, 1) * 100, 3
            ),
            "throughput_utilization": round(
                current_tps / MAX_THEORETICAL_TPS * 100, 2
            ),
            "rpc_total_requests": rpc.get("total_requests", 0),
            "rpc_cache_hit_rate": rpc.get("cache_hit_rate", "0.0%"),
            "rpc_cached_responses": rpc.get("cached_responses", 0),
            "rpc_gpu_verified": verifier.get("total_verified", 0),
            "rpc_errors": rpc.get("errors", 0),
            "uptime_seconds": round(uptime, 1),
            "cost_per_tx_usd": cost_per_tx,
            "cost_per_million_tx_usd": round(cost_per_tx * 1_000_000, 6),
            "gpu_power_watts": GPU_POWER_WATTS * GPU_COUNT,
            "gpu_cost_per_hour_usd": round(cost_per_hour, 4),
        }

        self.tps_history.append(
            {"timestamp": snapshot["timestamp"], "tps": round(current_tps, 1)}
        )
        self.history.append(snapshot)
        self.latest = snapshot

    def get_latest(self) -> Optional[dict]:
        return self.latest

    def get_history(self, seconds: int = 3600) -> list:
        cutoff = time.time() - seconds
        return [s for s in self.history if s["timestamp"] >= cutoff]

    def get_tps_history(self, seconds: int = 3600) -> list:
        cutoff = time.time() - seconds
        return [p for p in self.tps_history if p["timestamp"] >= cutoff]


# ═══════════════════════════════════════════════════════════
#  Subscriber & Access-List Manager
# ═══════════════════════════════════════════════════════════

TIER_CONFIG = {
    "basic":      {"max_tps": 100_000,     "rate_limit_rpm": 600,   "price_monthly": 0},
    "pro":        {"max_tps": 1_000_000,   "rate_limit_rpm": 6000,  "price_monthly": 49},
    "enterprise": {"max_tps": 999_999_999, "rate_limit_rpm": 60000, "price_monthly": 299},
}


class SubscriberManager:
    """Manages subscribers (validators), whitelist, blacklist, and accounting."""

    def __init__(self, auth: "AdminAuth"):
        self.auth = auth
        self.validators_path = os.path.join(BASE_DIR, "validators.json")
        self.acl_path = os.path.join(BASE_DIR, "access_lists.json")
        self._whitelist: List[str] = []  # IPs or validator IDs
        self._blacklist: List[str] = []
        self._load_acl()

    # ── Persistence helpers ──

    def _load_validators(self) -> dict:
        try:
            with open(self.validators_path) as f:
                return json.load(f)
        except (FileNotFoundError, json.JSONDecodeError):
            return {}

    def _save_validators(self, data: dict):
        with open(self.validators_path, "w") as f:
            json.dump(data, f, indent=2)

    def _load_acl(self):
        try:
            with open(self.acl_path) as f:
                d = json.load(f)
                self._whitelist = d.get("whitelist", [])
                self._blacklist = d.get("blacklist", [])
        except (FileNotFoundError, json.JSONDecodeError):
            self._whitelist = []
            self._blacklist = []

    def _save_acl(self):
        with open(self.acl_path, "w") as f:
            json.dump(
                {"whitelist": self._whitelist, "blacklist": self._blacklist},
                f,
                indent=2,
            )

    # ── Subscriber operations ──

    def list_subscribers(self, search: str = "", tier: str = "") -> list:
        data = self._load_validators()
        results = []
        for vid, v in data.items():
            if search and search.lower() not in json.dumps(v).lower():
                continue
            if tier and v.get("sla_tier") != tier:
                continue
            results.append(v)
        return results

    def get_subscriber(self, validator_id: str) -> Optional[dict]:
        data = self._load_validators()
        return data.get(validator_id)

    def update_subscriber(self, validator_id: str, updates: dict) -> Optional[dict]:
        data = self._load_validators()
        if validator_id not in data:
            return None
        allowed_fields = {"sla_tier", "enabled", "max_tps", "email"}
        for k, v in updates.items():
            if k in allowed_fields:
                data[validator_id][k] = v
        # Auto-set max_tps from tier when tier changes
        if "sla_tier" in updates and updates["sla_tier"] in TIER_CONFIG:
            data[validator_id]["max_tps"] = TIER_CONFIG[updates["sla_tier"]]["max_tps"]
        self._save_validators(data)
        self.auth.log_event("subscriber_update", f"{validator_id}: {updates}")
        return data[validator_id]

    def disable_subscriber(self, validator_id: str) -> bool:
        data = self._load_validators()
        if validator_id not in data:
            return False
        data[validator_id]["enabled"] = False
        self._save_validators(data)
        self.auth.log_event("subscriber_disabled", validator_id)
        return True

    def enable_subscriber(self, validator_id: str) -> bool:
        data = self._load_validators()
        if validator_id not in data:
            return False
        data[validator_id]["enabled"] = True
        self._save_validators(data)
        self.auth.log_event("subscriber_enabled", validator_id)
        return True

    def delete_subscriber(self, validator_id: str) -> bool:
        data = self._load_validators()
        if validator_id not in data:
            return False
        del data[validator_id]
        self._save_validators(data)
        self.auth.log_event("subscriber_deleted", validator_id)
        return True

    # ── Whitelist / Blacklist ──

    def get_whitelist(self) -> list:
        return list(self._whitelist)

    def add_to_whitelist(self, entry: str, reason: str = "") -> bool:
        entry = entry.strip()
        if not entry or entry in self._whitelist:
            return False
        # Remove from blacklist if present
        if entry in self._blacklist:
            self._blacklist.remove(entry)
        self._whitelist.append(entry)
        self._save_acl()
        self.auth.log_event("whitelist_add", f"{entry} — {reason}")
        return True

    def remove_from_whitelist(self, entry: str) -> bool:
        if entry not in self._whitelist:
            return False
        self._whitelist.remove(entry)
        self._save_acl()
        self.auth.log_event("whitelist_remove", entry)
        return True

    def get_blacklist(self) -> list:
        return list(self._blacklist)

    def add_to_blacklist(self, entry: str, reason: str = "") -> bool:
        entry = entry.strip()
        if not entry or entry in self._blacklist:
            return False
        # Remove from whitelist if present
        if entry in self._whitelist:
            self._whitelist.remove(entry)
        self._blacklist.append(entry)
        self._save_acl()
        self.auth.log_event("blacklist_add", f"{entry} — {reason}")
        return True

    def remove_from_blacklist(self, entry: str) -> bool:
        if entry not in self._blacklist:
            return False
        self._blacklist.remove(entry)
        self._save_acl()
        self.auth.log_event("blacklist_remove", entry)
        return True

    def is_allowed(self, identifier: str) -> bool:
        """Check if an IP/validator is allowed (not blacklisted)."""
        if identifier in self._blacklist:
            return False
        # If whitelist is non-empty, only whitelisted entries are allowed
        if self._whitelist and identifier not in self._whitelist:
            return False
        return True

    # ── Accounting ──

    def get_accounting(self) -> dict:
        data = self._load_validators()
        tier_counts = {"basic": 0, "pro": 0, "enterprise": 0}
        total_requests = 0
        total_tx = 0
        active_count = 0
        inactive_count = 0

        subscribers = []
        for vid, v in data.items():
            tier = v.get("sla_tier", "basic")
            tier_counts[tier] = tier_counts.get(tier, 0) + 1
            total_requests += v.get("total_requests", 0)
            total_tx += v.get("total_tx", 0)
            if v.get("enabled"):
                active_count += 1
            else:
                inactive_count += 1
            subscribers.append(v)

        # Revenue projection (monthly)
        monthly_revenue = sum(
            TIER_CONFIG.get(t, {}).get("price_monthly", 0) * c
            for t, c in tier_counts.items()
        )

        return {
            "total_subscribers": len(data),
            "active": active_count,
            "inactive": inactive_count,
            "tier_breakdown": tier_counts,
            "total_requests": total_requests,
            "total_tx_processed": total_tx,
            "monthly_revenue_usd": monthly_revenue,
            "annual_revenue_usd": monthly_revenue * 12,
            "tier_config": TIER_CONFIG,
            "whitelist_count": len(self._whitelist),
            "blacklist_count": len(self._blacklist),
        }


# ═══════════════════════════════════════════════════════════
#  Job Records & Command Runner
# ═══════════════════════════════════════════════════════════

@dataclass
class JobRecord:
    job_id: str
    command_id: str
    label: str
    status: str  # running | completed | failed | killed
    pid: Optional[int] = None
    started_at: float = 0.0
    finished_at: float = 0.0
    exit_code: Optional[int] = None
    output_lines: deque = field(default_factory=lambda: deque(maxlen=500))


class AdminCommandRunner:
    """Manages admin commands, service lifecycle, and benchmarks."""

    COMMANDS = {
        # ── Service Lifecycle ──
        "start-gpu-lanes": {
            "label": "Start All GPU Lanes (9001-9003)",
            "category": "services",
            "description": "Launches 3 GPU lane services on ports 9001, 9002, 9003",
            "script": "gpu_lane_service.py",
            "multi": [
                {"args": ["primary", "0", "9001"], "log": "/tmp/primary_lane.log"},
                {"args": ["shadow", "1", "9002"], "log": "/tmp/shadow_lane.log"},
                {"args": ["tertiary", "2", "9003"], "log": "/tmp/tertiary_lane.log"},
            ],
        },
        "start-bridge": {
            "label": "Start TPS Bridge (9999)",
            "category": "services",
            "description": "Launches TPS transaction bridge on port 9999",
            "script": "tps_bridge.py",
            "args": [],
            "log": "/tmp/tps_bridge.log",
        },
        "start-registry": {
            "label": "Start Validator Registry (7001)",
            "category": "services",
            "description": "Launches validator registration/auth on port 7001",
            "script": "validator_registry.py",
            "args": [],
            "log": "/tmp/validator_registry.log",
        },
        "start-rpc-proxy": {
            "label": "Start Solana RPC Proxy (8899)",
            "category": "services",
            "description": "Launches GPU-accelerated Solana RPC proxy on port 8899",
            "script": "solana_rpc_proxy.py",
            "args": ["0", "8899"],
            "log": "/tmp/solana_rpc_proxy.log",
        },
        "stop-gpu-lanes": {
            "label": "Stop All GPU Lanes",
            "category": "services",
            "description": "Kills all gpu_lane_service processes",
            "kill_pattern": "gpu_lane_service",
        },
        "stop-bridge": {
            "label": "Stop TPS Bridge",
            "category": "services",
            "description": "Kills the TPS bridge process",
            "kill_pattern": "tps_bridge",
        },
        "stop-registry": {
            "label": "Stop Validator Registry",
            "category": "services",
            "description": "Kills the validator registry",
            "kill_pattern": "validator_registry",
        },
        "stop-rpc-proxy": {
            "label": "Stop RPC Proxy",
            "category": "services",
            "description": "Kills the Solana RPC proxy",
            "kill_pattern": "solana_rpc_proxy",
        },
        "stop-all": {
            "label": "Stop All Services",
            "category": "services",
            "description": "Kills all Inferstructor services",
            "kill_pattern": "gpu_lane_service|tps_bridge|validator_registry|solana_rpc_proxy",
        },
        # ── Benchmarks & Stress Tests ──
        "bench-rpc-latency": {
            "label": "RPC Latency Benchmark",
            "category": "benchmarks",
            "description": "Proxy vs direct Solana RPC latency + throughput",
            "inline_script": "/tmp/rpc_bench.py",
        },
        "load-test-bridge": {
            "label": "Load Test (via Bridge)",
            "category": "benchmarks",
            "description": "Batch load test through TPS bridge — targets 500K+ TPS",
            "script": "load_test.py",
            "args": [],
            "log": "/tmp/load_test_bridge.log",
        },
        "load-test-direct": {
            "label": "Load Test (Direct GPU)",
            "category": "benchmarks",
            "description": "Direct-to-GPU load test — targets 960K+ TPS peak",
            "script": "load_test_direct.py",
            "args": [],
            "log": "/tmp/load_test_direct.log",
        },
        # ── Health Checks ──
        "health-gpu-lanes": {
            "label": "Check GPU Lanes Health",
            "category": "health",
            "description": "Curl health endpoints on all 3 GPU lanes",
            "shell": (
                'echo "=== GPU Lane 9001 ===" && curl -s http://localhost:9001/health 2>/dev/null | python3 -m json.tool || echo "DOWN"; '
                'echo "=== GPU Lane 9002 ===" && curl -s http://localhost:9002/health 2>/dev/null | python3 -m json.tool || echo "DOWN"; '
                'echo "=== GPU Lane 9003 ===" && curl -s http://localhost:9003/health 2>/dev/null | python3 -m json.tool || echo "DOWN"'
            ),
        },
        "health-bridge": {
            "label": "Check Bridge Health",
            "category": "health",
            "description": "Curl the TPS bridge health endpoint",
            "shell": 'curl -s http://localhost:9999/health 2>/dev/null | python3 -m json.tool || echo "DOWN"',
        },
        "health-registry": {
            "label": "Check Registry Health",
            "category": "health",
            "description": "Curl the validator registry health",
            "shell": 'curl -s http://localhost:7001/health 2>/dev/null | python3 -m json.tool || echo "DOWN"',
        },
        "health-rpc-proxy": {
            "label": "Check RPC Proxy Health",
            "category": "health",
            "description": "Curl the Solana RPC proxy health + chain stats",
            "shell": (
                'echo "=== Health ===" && curl -s http://localhost:8899/health 2>/dev/null | python3 -m json.tool || echo "DOWN"; '
                'echo "=== Chain Stats ===" && curl -s http://localhost:8899/chain-stats 2>/dev/null | python3 -m json.tool || echo "No stats"'
            ),
        },
        "health-all": {
            "label": "Full Health Check",
            "category": "health",
            "description": "Check all services at once",
            "shell": (
                "for port in 9001 9002 9003 9999 7001 8899; do "
                '  status=$(curl -s -o /dev/null -w "%{http_code}" http://localhost:$port/health 2>/dev/null); '
                '  if [ "$status" = "200" ]; then echo "  ✅ Port $port: UP"; '
                '  else echo "  ❌ Port $port: DOWN (HTTP $status)"; fi; '
                "done"
            ),
        },
        # ── System ──
        "gpu-status": {
            "label": "GPU Status (nvidia-smi)",
            "category": "system",
            "description": "GPU utilization, memory, temperature",
            "shell": "nvidia-smi",
        },
        "gpu-processes": {
            "label": "GPU Processes",
            "category": "system",
            "description": "Show processes using GPUs",
            "shell": "nvidia-smi pmon -c 1 2>/dev/null || nvidia-smi",
        },
        # ── Logs ──
        "logs-gpu-primary": {
            "label": "Logs: GPU Primary",
            "category": "logs",
            "description": "Last 50 lines of primary GPU lane log",
            "shell": "tail -50 /tmp/primary_lane.log 2>/dev/null || echo 'No log file'",
        },
        "logs-gpu-shadow": {
            "label": "Logs: GPU Shadow",
            "category": "logs",
            "description": "Last 50 lines of shadow GPU lane log",
            "shell": "tail -50 /tmp/shadow_lane.log 2>/dev/null || echo 'No log file'",
        },
        "logs-gpu-tertiary": {
            "label": "Logs: GPU Tertiary",
            "category": "logs",
            "description": "Last 50 lines of tertiary GPU lane log",
            "shell": "tail -50 /tmp/tertiary_lane.log 2>/dev/null || echo 'No log file'",
        },
        "logs-bridge": {
            "label": "Logs: TPS Bridge",
            "category": "logs",
            "description": "Last 50 lines of TPS bridge log",
            "shell": "tail -50 /tmp/tps_bridge.log 2>/dev/null || echo 'No log file'",
        },
        "logs-rpc-proxy": {
            "label": "Logs: RPC Proxy",
            "category": "logs",
            "description": "Last 50 lines of RPC proxy log",
            "shell": "tail -50 /tmp/solana_rpc_proxy.log 2>/dev/null || echo 'No log file'",
        },
        "logs-registry": {
            "label": "Logs: Registry",
            "category": "logs",
            "description": "Last 50 lines of registry log",
            "shell": "tail -50 /tmp/validator_registry.log 2>/dev/null || echo 'No log file'",
        },
    }

    def __init__(self, auth: AdminAuth, metrics: MetricsCollector, subscribers: "SubscriberManager"):
        self.auth = auth
        self.metrics = metrics
        self.subscribers = subscribers
        self.jobs: Dict[str, JobRecord] = {}
        self._job_counter = 0
        self.start_time = time.time()
        self._throttle_tps: Optional[int] = None
        self._paused = False

    def _next_job_id(self) -> str:
        self._job_counter += 1
        return f"job-{self._job_counter:04d}"

    def _require_auth(self, request: web.Request) -> Optional[web.Response]:
        if not self.auth.verify_request(request):
            return web.json_response({"error": "Unauthorized"}, status=401)
        return None

    # ── Auth endpoints ──

    async def handle_login(self, request: web.Request) -> web.Response:
        try:
            body = await request.json()
        except Exception:
            return web.json_response({"error": "Invalid JSON"}, status=400)

        password = body.get("password", "")
        if not self.auth.verify_password(password):
            self.auth.log_event("login_failed", "Invalid password attempt")
            return web.json_response({"error": "Invalid password"}, status=401)

        token = self.auth.create_token()
        self.auth.log_event("login_success", "Admin session started")
        return web.json_response({
            "success": True,
            "token": token,
            "expires_in": ADMIN_JWT_EXPIRY,
        })

    async def handle_verify(self, request: web.Request) -> web.Response:
        if err := self._require_auth(request):
            return err
        return web.json_response({"valid": True})

    # ── Metrics endpoints ──

    async def handle_metrics(self, request: web.Request) -> web.Response:
        if err := self._require_auth(request):
            return err

        latest = self.metrics.get_latest()
        if not latest:
            return web.json_response({"error": "No metrics yet"}, status=503)
        return web.json_response(latest)

    async def handle_metrics_history(self, request: web.Request) -> web.Response:
        if err := self._require_auth(request):
            return err

        seconds = int(request.query.get("seconds", "3600"))
        seconds = min(seconds, 3600)

        history = self.metrics.get_history(seconds)
        # Return only aggregated data to keep response small
        return web.json_response({
            "points": [
                {
                    "timestamp": s["timestamp"],
                    **s.get("aggregated", {}),
                }
                for s in history
            ],
            "count": len(history),
        })

    async def handle_tps_history(self, request: web.Request) -> web.Response:
        if err := self._require_auth(request):
            return err

        seconds = int(request.query.get("seconds", "3600"))
        return web.json_response({
            "points": list(self.metrics.get_tps_history(seconds)),
        })

    # ── Command endpoints ──

    async def handle_list_commands(self, request: web.Request) -> web.Response:
        if err := self._require_auth(request):
            return err

        by_category: Dict[str, list] = {}
        for cmd_id, cmd in self.COMMANDS.items():
            cat = cmd.get("category", "other")
            if cat not in by_category:
                by_category[cat] = []
            by_category[cat].append({
                "id": cmd_id,
                "label": cmd["label"],
                "description": cmd.get("description", ""),
                "category": cat,
            })

        return web.json_response({
            "commands": by_category,
            "categories": list(by_category.keys()),
        })

    async def handle_run_command(self, request: web.Request) -> web.Response:
        if err := self._require_auth(request):
            return err

        command_id = request.match_info["command_id"]
        cmd_def = self.COMMANDS.get(command_id)
        if not cmd_def:
            return web.json_response(
                {"error": f"Unknown command: {command_id}"}, status=404
            )

        job_id = self._next_job_id()
        job = JobRecord(
            job_id=job_id,
            command_id=command_id,
            label=cmd_def["label"],
            status="running",
            started_at=time.time(),
        )
        self.jobs[job_id] = job
        self.auth.log_event("command_run", f"{command_id} -> {job_id}")

        if "kill_pattern" in cmd_def:
            await self._run_kill(job, cmd_def["kill_pattern"])
        elif "shell" in cmd_def:
            asyncio.create_task(self._run_shell(job, cmd_def["shell"]))
        elif "multi" in cmd_def:
            await self._run_multi_service(job, cmd_def)
        elif "script" in cmd_def:
            asyncio.create_task(self._run_script(job, cmd_def))
        elif "inline_script" in cmd_def:
            asyncio.create_task(
                self._run_inline_script(job, cmd_def["inline_script"])
            )

        return web.json_response({
            "job_id": job_id,
            "command": command_id,
            "label": cmd_def["label"],
            "status": job.status,
        })

    async def _run_kill(self, job: JobRecord, pattern: str):
        for pat in pattern.split("|"):
            proc = await asyncio.create_subprocess_exec(
                "pkill", "-f", pat,
                stdout=asyncio.subprocess.PIPE,
                stderr=asyncio.subprocess.PIPE,
            )
            await proc.wait()
            if proc.returncode == 0:
                job.output_lines.append(f"Killed processes matching: {pat}")
            else:
                job.output_lines.append(f"No processes found for: {pat}")
        job.status = "completed"
        job.finished_at = time.time()
        job.exit_code = 0

    async def _run_shell(self, job: JobRecord, shell_cmd: str):
        try:
            proc = await asyncio.create_subprocess_shell(
                shell_cmd,
                stdout=asyncio.subprocess.PIPE,
                stderr=asyncio.subprocess.STDOUT,
                cwd=BASE_DIR,
            )
            job.pid = proc.pid
            async for line in proc.stdout:
                job.output_lines.append(
                    line.decode("utf-8", errors="replace").rstrip()
                )
            await proc.wait()
            job.exit_code = proc.returncode
            job.status = "completed" if proc.returncode == 0 else "failed"
        except Exception as e:
            job.output_lines.append(f"Error: {e}")
            job.status = "failed"
        finally:
            job.finished_at = time.time()

    async def _run_script(self, job: JobRecord, cmd_def: dict):
        script = os.path.join(BASE_DIR, cmd_def["script"])
        args = cmd_def.get("args", [])
        log_file = cmd_def.get("log")

        if not os.path.exists(script):
            job.output_lines.append(f"Script not found: {script}")
            job.status = "failed"
            job.finished_at = time.time()
            return

        cmd = [VENV_PYTHON, script] + [str(a) for a in args]
        try:
            if log_file:
                with open(log_file, "a") as lf:
                    proc = await asyncio.create_subprocess_exec(
                        *cmd,
                        stdout=lf,
                        stderr=asyncio.subprocess.STDOUT,
                        cwd=BASE_DIR,
                    )
                job.pid = proc.pid
                job.output_lines.append(
                    f"Started PID {proc.pid}, logging to {log_file}"
                )
                await asyncio.sleep(2)
                if proc.returncode is not None:
                    job.status = "failed"
                    job.exit_code = proc.returncode
                    job.output_lines.append(
                        f"Process exited immediately with code {proc.returncode}"
                    )
                else:
                    job.status = "completed"
                    job.output_lines.append("Service started successfully")
            else:
                proc = await asyncio.create_subprocess_exec(
                    *cmd,
                    stdout=asyncio.subprocess.PIPE,
                    stderr=asyncio.subprocess.STDOUT,
                    cwd=BASE_DIR,
                )
                job.pid = proc.pid
                async for line in proc.stdout:
                    job.output_lines.append(
                        line.decode("utf-8", errors="replace").rstrip()
                    )
                await proc.wait()
                job.exit_code = proc.returncode
                job.status = "completed" if proc.returncode == 0 else "failed"
        except Exception as e:
            job.output_lines.append(f"Error: {e}")
            job.status = "failed"
        finally:
            job.finished_at = time.time()

    async def _run_multi_service(self, job: JobRecord, cmd_def: dict):
        script = os.path.join(BASE_DIR, cmd_def["script"])
        pids = []
        for instance in cmd_def["multi"]:
            args = [str(a) for a in instance["args"]]
            log_file = instance["log"]
            cmd = [VENV_PYTHON, script] + args
            try:
                with open(log_file, "a") as lf:
                    proc = await asyncio.create_subprocess_exec(
                        *cmd,
                        stdout=lf,
                        stderr=asyncio.subprocess.STDOUT,
                        cwd=BASE_DIR,
                    )
                pids.append(proc.pid)
                job.output_lines.append(
                    f"Started {args[0]} (PID {proc.pid}) -> {log_file}"
                )
            except Exception as e:
                job.output_lines.append(f"Failed to start {args[0]}: {e}")
        await asyncio.sleep(3)
        job.output_lines.append(f"Launched {len(pids)} instances: PIDs {pids}")
        job.status = "completed"
        job.finished_at = time.time()

    async def _run_inline_script(self, job: JobRecord, script_path: str):
        if not os.path.exists(script_path):
            job.output_lines.append(f"Script not found: {script_path}")
            job.status = "failed"
            job.finished_at = time.time()
            return
        await self._run_shell(job, f"{VENV_PYTHON} {script_path}")

    # ── Job management ──

    async def handle_job_status(self, request: web.Request) -> web.Response:
        if err := self._require_auth(request):
            return err
        job_id = request.match_info["job_id"]
        job = self.jobs.get(job_id)
        if not job:
            return web.json_response({"error": "Job not found"}, status=404)
        return web.json_response({
            "job_id": job.job_id,
            "command": job.command_id,
            "label": job.label,
            "status": job.status,
            "pid": job.pid,
            "started_at": job.started_at,
            "finished_at": job.finished_at,
            "exit_code": job.exit_code,
            "output": list(job.output_lines),
            "duration_seconds": round(
                (job.finished_at or time.time()) - job.started_at, 1
            ),
        })

    async def handle_jobs_list(self, request: web.Request) -> web.Response:
        if err := self._require_auth(request):
            return err
        jobs = sorted(
            self.jobs.values(), key=lambda j: j.started_at, reverse=True
        )[:50]
        return web.json_response({
            "jobs": [
                {
                    "job_id": j.job_id,
                    "command": j.command_id,
                    "label": j.label,
                    "status": j.status,
                    "started_at": j.started_at,
                    "duration_seconds": round(
                        (j.finished_at or time.time()) - j.started_at, 1
                    ),
                }
                for j in jobs
            ]
        })

    async def handle_kill_job(self, request: web.Request) -> web.Response:
        if err := self._require_auth(request):
            return err
        job_id = request.match_info["job_id"]
        job = self.jobs.get(job_id)
        if not job:
            return web.json_response({"error": "Job not found"}, status=404)
        if job.pid and job.status == "running":
            try:
                os.kill(job.pid, signal.SIGTERM)
                job.status = "killed"
                job.finished_at = time.time()
                job.output_lines.append("Killed by admin (SIGTERM)")
                self.auth.log_event("job_killed", job_id)
            except ProcessLookupError:
                job.status = "completed"
                job.output_lines.append("Process already exited")
        return web.json_response({"job_id": job_id, "status": job.status})

    # ── Service status ──

    async def handle_service_status(self, request: web.Request) -> web.Response:
        if err := self._require_auth(request):
            return err

        services = [
            {"name": "GPU Lane Primary", "port": 9001},
            {"name": "GPU Lane Shadow", "port": 9002},
            {"name": "GPU Lane Tertiary", "port": 9003},
            {"name": "TPS Bridge", "port": 9999},
            {"name": "Validator Registry", "port": 7001},
            {"name": "Solana RPC Proxy", "port": 8899},
        ]

        results = []
        timeout = aiohttp.ClientTimeout(total=3)
        async with aiohttp.ClientSession(timeout=timeout) as session:
            for svc in services:
                url = f"http://localhost:{svc['port']}/health"
                try:
                    async with session.get(url) as resp:
                        if resp.status == 200:
                            data = await resp.json()
                            results.append({**svc, "status": "up", "details": data})
                        else:
                            results.append(
                                {**svc, "status": "error", "http_code": resp.status}
                            )
                except Exception:
                    results.append({**svc, "status": "down"})

        return web.json_response({"services": results})

    # ── Admin actions ──

    async def handle_admin_action(self, request: web.Request) -> web.Response:
        if err := self._require_auth(request):
            return err

        action = request.match_info["action"]
        try:
            body = await request.json()
        except Exception:
            body = {}

        if action == "throttle":
            limit = body.get("tps_limit")
            self._throttle_tps = int(limit) if limit else None
            self.auth.log_event(
                "throttle", f"TPS limit set to {self._throttle_tps}"
            )
            return web.json_response({
                "action": "throttle",
                "tps_limit": self._throttle_tps,
            })

        elif action == "pause":
            self._paused = True
            self.auth.log_event("pause", "Services paused")
            return web.json_response({"action": "pause", "paused": True})

        elif action == "resume":
            self._paused = False
            self.auth.log_event("resume", "Services resumed")
            return web.json_response({"action": "resume", "paused": False})

        elif action == "export-forensics":
            bundle = {
                "exported_at": time.time(),
                "metrics_history": [
                    {"timestamp": s["timestamp"], **s.get("aggregated", {})}
                    for s in self.metrics.get_history(3600)
                ],
                "jobs": [
                    {
                        "job_id": j.job_id,
                        "command": j.command_id,
                        "status": j.status,
                        "started_at": j.started_at,
                        "output": list(j.output_lines),
                    }
                    for j in self.jobs.values()
                ],
                "audit_log": self.auth.get_audit_log(200),
            }
            self.auth.log_event("export_forensics", "Forensic bundle exported")
            return web.json_response(bundle)

        else:
            return web.json_response(
                {"error": f"Unknown action: {action}"}, status=404
            )

    # ── Audit log ──

    async def handle_audit_log(self, request: web.Request) -> web.Response:
        if err := self._require_auth(request):
            return err
        limit = int(request.query.get("limit", "50"))
        return web.json_response({
            "events": self.auth.get_audit_log(limit),
        })

    # ── Subscribers ──

    async def handle_list_subscribers(self, request: web.Request) -> web.Response:
        if err := self._require_auth(request):
            return err
        search = request.query.get("search", "")
        tier = request.query.get("tier", "")
        subs = self.subscribers.list_subscribers(search, tier)
        return web.json_response({"subscribers": subs, "total": len(subs)})

    async def handle_get_subscriber(self, request: web.Request) -> web.Response:
        if err := self._require_auth(request):
            return err
        vid = request.match_info["validator_id"]
        sub = self.subscribers.get_subscriber(vid)
        if not sub:
            return web.json_response({"error": "Subscriber not found"}, status=404)
        return web.json_response(sub)

    async def handle_update_subscriber(self, request: web.Request) -> web.Response:
        if err := self._require_auth(request):
            return err
        vid = request.match_info["validator_id"]
        try:
            body = await request.json()
        except Exception:
            return web.json_response({"error": "Invalid JSON"}, status=400)
        result = self.subscribers.update_subscriber(vid, body)
        if not result:
            return web.json_response({"error": "Subscriber not found"}, status=404)
        return web.json_response({"success": True, "subscriber": result})

    async def handle_disable_subscriber(self, request: web.Request) -> web.Response:
        if err := self._require_auth(request):
            return err
        vid = request.match_info["validator_id"]
        ok = self.subscribers.disable_subscriber(vid)
        if not ok:
            return web.json_response({"error": "Subscriber not found"}, status=404)
        return web.json_response({"success": True, "validator_id": vid, "enabled": False})

    async def handle_enable_subscriber(self, request: web.Request) -> web.Response:
        if err := self._require_auth(request):
            return err
        vid = request.match_info["validator_id"]
        ok = self.subscribers.enable_subscriber(vid)
        if not ok:
            return web.json_response({"error": "Subscriber not found"}, status=404)
        return web.json_response({"success": True, "validator_id": vid, "enabled": True})

    async def handle_delete_subscriber(self, request: web.Request) -> web.Response:
        if err := self._require_auth(request):
            return err
        vid = request.match_info["validator_id"]
        ok = self.subscribers.delete_subscriber(vid)
        if not ok:
            return web.json_response({"error": "Subscriber not found"}, status=404)
        return web.json_response({"success": True, "deleted": vid})

    # ── Whitelist / Blacklist ──

    async def handle_get_whitelist(self, request: web.Request) -> web.Response:
        if err := self._require_auth(request):
            return err
        return web.json_response({"whitelist": self.subscribers.get_whitelist()})

    async def handle_add_whitelist(self, request: web.Request) -> web.Response:
        if err := self._require_auth(request):
            return err
        try:
            body = await request.json()
        except Exception:
            return web.json_response({"error": "Invalid JSON"}, status=400)
        entry = body.get("entry", "").strip()
        reason = body.get("reason", "")
        if not entry:
            return web.json_response({"error": "Missing entry"}, status=400)
        ok = self.subscribers.add_to_whitelist(entry, reason)
        return web.json_response({"success": ok, "whitelist": self.subscribers.get_whitelist()})

    async def handle_remove_whitelist(self, request: web.Request) -> web.Response:
        if err := self._require_auth(request):
            return err
        entry = request.match_info["entry"]
        ok = self.subscribers.remove_from_whitelist(entry)
        return web.json_response({"success": ok, "whitelist": self.subscribers.get_whitelist()})

    async def handle_get_blacklist(self, request: web.Request) -> web.Response:
        if err := self._require_auth(request):
            return err
        return web.json_response({"blacklist": self.subscribers.get_blacklist()})

    async def handle_add_blacklist(self, request: web.Request) -> web.Response:
        if err := self._require_auth(request):
            return err
        try:
            body = await request.json()
        except Exception:
            return web.json_response({"error": "Invalid JSON"}, status=400)
        entry = body.get("entry", "").strip()
        reason = body.get("reason", "")
        if not entry:
            return web.json_response({"error": "Missing entry"}, status=400)
        ok = self.subscribers.add_to_blacklist(entry, reason)
        return web.json_response({"success": ok, "blacklist": self.subscribers.get_blacklist()})

    async def handle_remove_blacklist(self, request: web.Request) -> web.Response:
        if err := self._require_auth(request):
            return err
        entry = request.match_info["entry"]
        ok = self.subscribers.remove_from_blacklist(entry)
        return web.json_response({"success": ok, "blacklist": self.subscribers.get_blacklist()})

    # ── Accounting ──

    async def handle_accounting(self, request: web.Request) -> web.Response:
        if err := self._require_auth(request):
            return err
        return web.json_response(self.subscribers.get_accounting())

    # ── System status ──

    async def handle_system_status(self, request: web.Request) -> web.Response:
        if err := self._require_auth(request):
            return err
        return web.json_response({
            "paused": self._paused,
            "throttle_tps": self._throttle_tps,
            "uptime_seconds": round(time.time() - self.start_time, 1),
            "total_jobs": len(self.jobs),
            "running_jobs": sum(
                1 for j in self.jobs.values() if j.status == "running"
            ),
        })

    # ── Health (unauthenticated) ──

    async def handle_health(self, request: web.Request) -> web.Response:
        return web.json_response({
            "status": "healthy",
            "service": "inferstructor-admin",
            "port": 7777,
            "uptime_seconds": round(time.time() - self.start_time, 1),
            "total_jobs": len(self.jobs),
        })


# ═══════════════════════════════════════════════════════════
#  Main — Wire everything together
# ═══════════════════════════════════════════════════════════

def main():
    auth = AdminAuth()
    metrics = MetricsCollector()
    subscribers = SubscriberManager(auth)
    runner = AdminCommandRunner(auth, metrics, subscribers)

    app = web.Application()
    cors = aiohttp_cors.setup(
        app,
        defaults={
            "*": aiohttp_cors.ResourceOptions(
                allow_credentials=True,
                expose_headers="*",
                allow_headers="*",
                allow_methods="*",
            )
        },
    )

    routes = [
        # Health (no auth)
        app.router.add_get("/health", runner.handle_health),
        # Auth
        app.router.add_post("/admin/login", runner.handle_login),
        app.router.add_get("/admin/verify", runner.handle_verify),
        # Metrics
        app.router.add_get("/admin/metrics", runner.handle_metrics),
        app.router.add_get("/admin/metrics/history", runner.handle_metrics_history),
        app.router.add_get("/admin/metrics/tps", runner.handle_tps_history),
        # Commands & Jobs
        app.router.add_get("/admin/commands", runner.handle_list_commands),
        app.router.add_post("/admin/run/{command_id}", runner.handle_run_command),
        app.router.add_get("/admin/jobs", runner.handle_jobs_list),
        app.router.add_get("/admin/jobs/{job_id}", runner.handle_job_status),
        app.router.add_delete("/admin/jobs/{job_id}", runner.handle_kill_job),
        # Services
        app.router.add_get("/admin/services", runner.handle_service_status),
        # Admin actions
        app.router.add_post("/admin/actions/{action}", runner.handle_admin_action),
        # Audit
        app.router.add_get("/admin/audit", runner.handle_audit_log),
        # System
        app.router.add_get("/admin/system", runner.handle_system_status),
        # Subscribers
        app.router.add_get("/admin/subscribers", runner.handle_list_subscribers),
        app.router.add_get("/admin/subscribers/{validator_id}", runner.handle_get_subscriber),
        app.router.add_post("/admin/subscribers/{validator_id}", runner.handle_update_subscriber),
        app.router.add_post("/admin/subscribers/{validator_id}/disable", runner.handle_disable_subscriber),
        app.router.add_post("/admin/subscribers/{validator_id}/enable", runner.handle_enable_subscriber),
        app.router.add_delete("/admin/subscribers/{validator_id}", runner.handle_delete_subscriber),
        # Whitelist / Blacklist
        app.router.add_get("/admin/whitelist", runner.handle_get_whitelist),
        app.router.add_post("/admin/whitelist", runner.handle_add_whitelist),
        app.router.add_delete("/admin/whitelist/{entry}", runner.handle_remove_whitelist),
        app.router.add_get("/admin/blacklist", runner.handle_get_blacklist),
        app.router.add_post("/admin/blacklist", runner.handle_add_blacklist),
        app.router.add_delete("/admin/blacklist/{entry}", runner.handle_remove_blacklist),
        # Accounting
        app.router.add_get("/admin/accounting", runner.handle_accounting),
    ]

    for route in routes:
        cors.add(route)

    async def on_startup(app_):
        app_["metrics_task"] = asyncio.create_task(metrics.start())
        logger.info("Background metrics collector launched")

    async def on_cleanup(app_):
        metrics.stop()
        app_["metrics_task"].cancel()

    app.on_startup.append(on_startup)
    app.on_cleanup.append(on_cleanup)

    logger.info("=" * 60)
    logger.info("  Inferstructor Admin API")
    logger.info(f"  Port: 7777")
    logger.info(f"  Admin Password: {'*' * len(ADMIN_PASSWORD)}")
    logger.info(f"  JWT Expiry: {ADMIN_JWT_EXPIRY // 3600}h")
    logger.info(f"  Metrics Interval: 5s")
    logger.info("=" * 60)

    web.run_app(app, host="0.0.0.0", port=7777, print=None)


if __name__ == "__main__":
    main()
