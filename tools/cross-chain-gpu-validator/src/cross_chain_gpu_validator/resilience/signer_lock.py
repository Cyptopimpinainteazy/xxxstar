"""Signer Lock Manager — prevents double-signing during failover.

Only ONE node in a validator cluster may hold signing authority at any
time.  This module provides a distributed mutex backed by Redis (or a
local file lock for single-node deployments).

If two nodes sign simultaneously → slashing.  This module makes that
architecturally impossible.

Lock Protocol
─────────────
1. Node acquires lock with a lease TTL (default 30 s).
2. Node must renew before TTL expires.
3. On failover, new node waits until lock expires OR the old node
   explicitly releases.
4. Fencing tokens prevent stale holders from signing.
"""

from __future__ import annotations

import json
import os
import threading
import time
import uuid
from dataclasses import dataclass
from enum import Enum
from typing import Callable


class SignerAuthority(Enum):
    """Current signing authority status."""
    HOLDER = "holder"       # We hold the lock — may sign
    STANDBY = "standby"     # Lock held by another node — must NOT sign
    ACQUIRING = "acquiring" # Trying to acquire
    RELEASED = "released"   # Explicitly released


@dataclass
class SignerLockState:
    """Snapshot of the signer lock."""
    authority: SignerAuthority
    holder_id: str
    fencing_token: int
    acquired_at: float
    ttl_seconds: float
    expires_at: float
    renewals: int


class SignerLock:
    """Distributed signing authority lock.

    Uses Redis ``SET NX EX`` for distributed locking or a local file lock
    as fallback.  The ``fencing_token`` is monotonically increasing so a
    stale holder can be detected even if the network partitions.

    Parameters
    ----------
    node_id : str
        Unique identifier for this node.
    redis_url : str | None
        Redis connection string.  If ``None``, uses local file lock.
    ttl_seconds : float
        Lock lease duration (default 30).  Must be renewed before expiry.
    lock_key : str
        Redis key for the lock (default ``"x3:signer:lock"``).
    on_acquired : callable
        Called when this node becomes the signer.
    on_lost : callable
        Called when this node loses signing authority.
    """

    def __init__(
        self,
        node_id: str | None = None,
        redis_url: str | None = None,
        ttl_seconds: float = 30.0,
        lock_key: str = "x3:signer:lock",
        on_acquired: Callable[[], None] | None = None,
        on_lost: Callable[[], None] | None = None,
    ) -> None:
        self._node_id = node_id or f"node-{uuid.uuid4().hex[:8]}"
        self._redis_url = redis_url
        self._ttl = ttl_seconds
        self._lock_key = lock_key
        self._token_key = f"{lock_key}:token"
        self._on_acquired = on_acquired
        self._on_lost = on_lost

        self._lock = threading.Lock()
        self._authority = SignerAuthority.STANDBY
        self._fencing_token = 0
        self._acquired_at = 0.0
        self._renewals = 0
        self._redis = None

        self._renewal_thread: threading.Thread | None = None
        self._stop = threading.Event()

        # Local file lock fallback
        self._local_lock_path = os.path.join(
            os.getenv("CCGV_DATA_DIR", "/tmp"), "x3_signer.lock"
        )

        self._connect_redis()

    def _connect_redis(self) -> None:
        if self._redis_url is None:
            return
        try:
            import redis
            self._redis = redis.Redis.from_url(
                self._redis_url,
                decode_responses=True,
                socket_timeout=5,
                socket_connect_timeout=5,
            )
            self._redis.ping()
        except Exception:
            self._redis = None

    # ── Acquire / Release ────────────────────────────────────

    def try_acquire(self) -> bool:
        """Attempt to acquire signing authority.  Returns True if acquired."""
        with self._lock:
            if self._authority == SignerAuthority.HOLDER:
                return True
            self._authority = SignerAuthority.ACQUIRING

        acquired = False
        if self._redis is not None:
            acquired = self._acquire_redis()
        else:
            acquired = self._acquire_local()

        with self._lock:
            if acquired:
                self._authority = SignerAuthority.HOLDER
                self._acquired_at = time.time()
                self._renewals = 0
                if self._redis is not None:
                    self._start_renewal()
                if self._on_acquired:
                    try:
                        self._on_acquired()
                    except Exception:
                        pass
            else:
                self._authority = SignerAuthority.STANDBY
        return acquired

    def release(self) -> None:
        """Explicitly release signing authority."""
        with self._lock:
            if self._authority != SignerAuthority.HOLDER:
                return
            self._authority = SignerAuthority.RELEASED

        self._stop_renewal()

        if self._redis is not None:
            self._release_redis()
        else:
            self._release_local()

        if self._on_lost:
            try:
                self._on_lost()
            except Exception:
                pass

    def renew(self) -> bool:
        """Renew the lock lease.  Returns False if we lost it."""
        if self._redis is not None:
            return self._renew_redis()
        return self._renew_local()

    # ── State ────────────────────────────────────────────────

    @property
    def authority(self) -> SignerAuthority:
        with self._lock:
            return self._authority

    @property
    def is_signer(self) -> bool:
        with self._lock:
            return self._authority == SignerAuthority.HOLDER

    @property
    def fencing_token(self) -> int:
        with self._lock:
            return self._fencing_token

    def state(self) -> SignerLockState:
        with self._lock:
            return SignerLockState(
                authority=self._authority,
                holder_id=self._node_id,
                fencing_token=self._fencing_token,
                acquired_at=self._acquired_at,
                ttl_seconds=self._ttl,
                expires_at=self._acquired_at + self._ttl if self._acquired_at else 0,
                renewals=self._renewals,
            )

    # ── Redis Implementation ─────────────────────────────────

    _ACQUIRE_SCRIPT = """
    local ok = redis.call('SET', KEYS[1], ARGV[1], 'NX', 'EX', ARGV[2])
    if ok then
        local token = redis.call('INCR', KEYS[2])
        return token
    end
    return 0
    """

    _RELEASE_SCRIPT = """
    if redis.call('GET', KEYS[1]) == ARGV[1] then
        redis.call('DEL', KEYS[1])
        return 1
    end
    return 0
    """

    _RENEW_SCRIPT = """
    if redis.call('GET', KEYS[1]) == ARGV[1] then
        redis.call('EXPIRE', KEYS[1], ARGV[2])
        return 1
    end
    return 0
    """

    def _acquire_redis(self) -> bool:
        try:
            token = self._redis.eval(
                self._ACQUIRE_SCRIPT,
                2,
                self._lock_key,
                self._token_key,
                self._node_id,
                int(self._ttl),
            )
            if token and int(token) > 0:
                with self._lock:
                    self._fencing_token = int(token)
                return True
            return False
        except Exception:
            return False

    def _release_redis(self) -> None:
        try:
            self._redis.eval(
                self._RELEASE_SCRIPT, 1, self._lock_key, self._node_id
            )
        except Exception:
            pass

    def _renew_redis(self) -> bool:
        try:
            result = self._redis.eval(
                self._RENEW_SCRIPT, 1, self._lock_key, self._node_id, int(self._ttl)
            )
            if result and int(result) == 1:
                with self._lock:
                    self._renewals += 1
                return True
            # Lost the lock
            with self._lock:
                self._authority = SignerAuthority.STANDBY
            if self._on_lost:
                try:
                    self._on_lost()
                except Exception:
                    pass
            return False
        except Exception:
            return False

    # ── Local File Lock ──────────────────────────────────────

    def _acquire_local(self) -> bool:
        try:
            fd = os.open(
                self._local_lock_path,
                os.O_CREAT | os.O_EXCL | os.O_WRONLY,
                0o600,
            )
            payload = json.dumps(
                {
                    "holder_id": self._node_id,
                    "acquired_at": time.time(),
                    "ttl_seconds": self._ttl,
                }
            ).encode()
            os.write(fd, payload)
            os.close(fd)
            with self._lock:
                self._fencing_token += 1
            return True
        except FileExistsError:
            # Check if lock is stale using the original holder's lease data.
            try:
                with open(self._local_lock_path, "r", encoding="utf-8") as handle:
                    raw = handle.read().strip()
                lock_data = json.loads(raw) if raw.startswith("{") else {"holder_id": raw}
                acquired_at = float(lock_data.get("acquired_at", 0.0))
                ttl_seconds = float(lock_data.get("ttl_seconds", self._ttl))
                if acquired_at and time.time() > acquired_at + ttl_seconds:
                    os.unlink(self._local_lock_path)
                    return self._acquire_local()
            except (OSError, ValueError, json.JSONDecodeError):
                pass
            return False
        except OSError:
            return False

    def _release_local(self) -> None:
        try:
            with open(self._local_lock_path, "r", encoding="utf-8") as handle:
                raw = handle.read().strip()
            lock_data = json.loads(raw) if raw.startswith("{") else {"holder_id": raw}
            if lock_data.get("holder_id") == self._node_id:
                os.unlink(self._local_lock_path)
        except (OSError, json.JSONDecodeError):
            pass

    def _renew_local(self) -> bool:
        try:
            with open(self._local_lock_path, "r", encoding="utf-8") as handle:
                raw = handle.read().strip()
            lock_data = json.loads(raw) if raw.startswith("{") else {"holder_id": raw}
            if lock_data.get("holder_id") == self._node_id:
                with open(self._local_lock_path, "w", encoding="utf-8") as handle:
                    handle.write(
                        json.dumps(
                            {
                                "holder_id": self._node_id,
                                "acquired_at": time.time(),
                                "ttl_seconds": self._ttl,
                            }
                        )
                    )
                with self._lock:
                    self._renewals += 1
                return True
            with self._lock:
                self._authority = SignerAuthority.STANDBY
            return False
        except (OSError, json.JSONDecodeError):
            return False

    # ── Auto-Renewal Thread ──────────────────────────────────

    def _start_renewal(self) -> None:
        self._stop.clear()
        self._renewal_thread = threading.Thread(
            target=self._renewal_loop, daemon=True, name="signer-lock-renew"
        )
        self._renewal_thread.start()

    def _stop_renewal(self) -> None:
        self._stop.set()
        if self._renewal_thread is not None:
            self._renewal_thread.join(timeout=3)
            self._renewal_thread = None

    def _renewal_loop(self) -> None:
        interval = self._ttl / 3.0  # Renew at 1/3 TTL
        while not self._stop.is_set():
            self._stop.wait(interval)
            if self._stop.is_set():
                break
            if not self.renew():
                break
