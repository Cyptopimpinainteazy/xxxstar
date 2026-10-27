"""Atomic swap registry backed by Redis."""

from __future__ import annotations

from dataclasses import dataclass
import json
import time
from typing import Iterable

import redis


@dataclass(frozen=True)
class AtomicSwapRecord:
    swap_id: str
    created_at: float
    timeout_at: float
    svm_validated: bool
    evm_validated: bool
    status: str


# Lua scripts for atomic read-modify-write operations.
# Using server-side scripts eliminates the GET→modify→SET race condition.
_LUA_UPDATE_STATUS = """
local raw = redis.call('GET', KEYS[1])
if not raw then return 0 end
local data = cjson.decode(raw)
data['status'] = ARGV[1]
local ttl = redis.call('TTL', KEYS[1])
if ttl > 0 then
    redis.call('SETEX', KEYS[1], ttl, cjson.encode(data))
else
    redis.call('SET', KEYS[1], cjson.encode(data))
end
return 1
"""

_LUA_UPDATE_VALIDATION = """
local raw = redis.call('GET', KEYS[1])
if not raw then return 0 end
local data = cjson.decode(raw)
data['svm_validated'] = ARGV[1] == '1'
data['evm_validated'] = ARGV[2] == '1'
local ttl = redis.call('TTL', KEYS[1])
if ttl > 0 then
    redis.call('SETEX', KEYS[1], ttl, cjson.encode(data))
else
    redis.call('SET', KEYS[1], cjson.encode(data))
end
return 1
"""


class AtomicSwapRegistry:
    """Redis-backed registry for atomic swaps.

    Optimizations over the original:
    - Lua scripts for atomic read-modify-write (no race conditions)
    - Connection pooling via redis-py's built-in pool
    - Pipeline-based pending scan to reduce round-trips
    """

    def __init__(self, redis_url: str, pool_size: int = 16) -> None:
        self._pool = redis.ConnectionPool.from_url(redis_url, max_connections=pool_size)
        self._client = redis.Redis(connection_pool=self._pool)
        if not self._client.ping():
            raise RuntimeError("Atomic registry unavailable (Redis ping failed).")
        # Pre-register Lua scripts (sent once, cached server-side by SHA)
        self._update_status_sha = self._client.script_load(_LUA_UPDATE_STATUS)
        self._update_validation_sha = self._client.script_load(_LUA_UPDATE_VALIDATION)

    def register_swap(self, swap_id: str, payload: dict) -> None:
        record = {
            "swap_id": swap_id,
            "created_at": time.time(),
            "timeout_at": time.time() + payload["timeout_seconds"],
            "svm_validated": False,
            "evm_validated": False,
            "status": "PENDING",
            "payload": payload,
        }
        self._client.set(
            f"swap:{swap_id}",
            json.dumps(record),
            ex=payload["timeout_seconds"] + 10,
        )

    def get_swap(self, swap_id: str) -> AtomicSwapRecord | None:
        raw = self._client.get(f"swap:{swap_id}")
        if raw is None:
            return None
        data = json.loads(raw)
        return AtomicSwapRecord(
            swap_id=data["swap_id"],
            created_at=data["created_at"],
            timeout_at=data["timeout_at"],
            svm_validated=data["svm_validated"],
            evm_validated=data["evm_validated"],
            status=data["status"],
        )

    def get_swaps_batch(self, swap_ids: list[str]) -> list[AtomicSwapRecord | None]:
        """Fetch multiple swap records in a single Redis pipeline round-trip."""
        if not swap_ids:
            return []
        pipe = self._client.pipeline(transaction=False)
        for sid in swap_ids:
            pipe.get(f"swap:{sid}")
        results = pipe.execute()
        records: list[AtomicSwapRecord | None] = []
        for raw in results:
            if raw is None:
                records.append(None)
                continue
            data = json.loads(raw)
            records.append(
                AtomicSwapRecord(
                    swap_id=data["swap_id"],
                    created_at=data["created_at"],
                    timeout_at=data["timeout_at"],
                    svm_validated=data["svm_validated"],
                    evm_validated=data["evm_validated"],
                    status=data["status"],
                )
            )
        return records

    def update_validation(self, swap_id: str, svm_valid: bool, evm_valid: bool) -> None:
        """Atomically update validation flags (Lua script, no race condition)."""
        self._client.evalsha(
            self._update_validation_sha,
            1,
            f"swap:{swap_id}",
            "1" if svm_valid else "0",
            "1" if evm_valid else "0",
        )

    def update_status(self, swap_id: str, status: str) -> None:
        """Atomically update swap status (Lua script, no race condition)."""
        self._client.evalsha(
            self._update_status_sha,
            1,
            f"swap:{swap_id}",
            status,
        )

    def pending_swaps(self) -> list[str]:
        """Scan for pending swaps using pipeline batch reads.

        Returns a list (not a generator) so callers can iterate safely while
        we mutate keys inside process_pending().
        """
        swap_ids: list[str] = []
        keys = list(self._client.scan_iter(match="swap:*", count=500))
        if not keys:
            return swap_ids

        # Pipeline GET for all keys in one round-trip
        pipe = self._client.pipeline(transaction=False)
        for key in keys:
            pipe.get(key)
        values = pipe.execute()

        for raw in values:
            if raw is None:
                continue
            try:
                data = json.loads(raw)
            except (json.JSONDecodeError, TypeError):
                continue
            if data.get("status") == "PENDING":
                swap_ids.append(data["swap_id"])
        return swap_ids
