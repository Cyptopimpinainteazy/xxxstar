"""Toll Booth — access control, SLA enforcement, and rate limiting.

Every external validator that rides the X3 superhighway passes through the
toll booth.  The booth checks:

1. **Access Tier** — Base / Pro / Enterprise (determines throughput cap)
2. **Rate Limit** — Sliding-window token bucket per validator
3. **SLA Contract** — Uptime / latency guarantees per tier
4. **Metering** — Usage tracking for billing hooks

Design philosophy: the booth never blocks emergency CPU fallback.  If
the toll booth itself fails, validators degrade to BASE tier (lower
throughput cap) rather than being denied service.  Safety > revenue.
"""

from __future__ import annotations

import logging
import threading
import time
from dataclasses import dataclass, field
from enum import Enum
from typing import Any

logger = logging.getLogger("x3.tollbooth")


# ─── Access Tiers ────────────────────────────────────────────


class AccessTier(Enum):
    """Validator subscription tier — determines throughput and SLA."""
    BASE = "base"             # Free tier, basic acceleration
    PRO = "pro"               # Higher throughput, better SLA
    ENTERPRISE = "enterprise" # Maximum throughput, full SLA


# Tier limits: requests per second
# UNCAPPED — validators run at full GPU speed, no artificial throttle
_TIER_RPS: dict[AccessTier, float] = {
    AccessTier.BASE: float('inf'),
    AccessTier.PRO: float('inf'),
    AccessTier.ENTERPRISE: float('inf'),
}

# Tier limits: max batch size
_TIER_BATCH: dict[AccessTier, int] = {
    AccessTier.BASE: 1024,
    AccessTier.PRO: 4096,
    AccessTier.ENTERPRISE: 16384,
}

# SLA: max latency (ms) per tier
_TIER_LATENCY_SLA: dict[AccessTier, float] = {
    AccessTier.BASE: 100.0,
    AccessTier.PRO: 50.0,
    AccessTier.ENTERPRISE: 10.0,
}


# ─── Validator Ticket ────────────────────────────────────────


@dataclass
class ValidatorTicket:
    """Issued to an authenticated validator for a single session."""

    validator_id: str
    chain_id: str
    tier: AccessTier
    issued_at: float = field(default_factory=time.time)
    expires_at: float = 0.0
    requests_used: int = 0
    bytes_processed: int = 0
    active: bool = True

    def __post_init__(self) -> None:
        if self.expires_at == 0.0:
            # Default: 1 hour session
            self.expires_at = self.issued_at + 3600.0

    @property
    def expired(self) -> bool:
        return time.time() > self.expires_at

    @property
    def rps_limit(self) -> float:
        return _TIER_RPS[self.tier]

    @property
    def batch_limit(self) -> int:
        return _TIER_BATCH[self.tier]

    @property
    def latency_sla_ms(self) -> float:
        return _TIER_LATENCY_SLA[self.tier]

    def record_usage(self, request_count: int = 1, bytes_count: int = 0) -> None:
        self.requests_used += request_count
        self.bytes_processed += bytes_count

    def to_dict(self) -> dict[str, Any]:
        return {
            "validator_id": self.validator_id,
            "chain_id": self.chain_id,
            "tier": self.tier.value,
            "issued_at": self.issued_at,
            "expires_at": self.expires_at,
            "requests_used": self.requests_used,
            "bytes_processed": self.bytes_processed,
            "expired": self.expired,
            "active": self.active,
        }


# ─── Token Bucket Rate Limiter ──────────────────────────────


class _TokenBucket:
    """Sliding-window token bucket for per-validator rate limiting."""

    def __init__(self, rate: float, capacity: float) -> None:
        self._rate = rate            # Tokens per second
        self._capacity = capacity    # Max burst
        self._tokens = capacity
        self._last_refill = time.monotonic()

    def try_consume(self, tokens: int = 1) -> bool:
        now = time.monotonic()
        elapsed = now - self._last_refill
        self._tokens = min(self._capacity, self._tokens + elapsed * self._rate)
        self._last_refill = now

        if self._tokens >= tokens:
            self._tokens -= tokens
            return True
        return False

    @property
    def available(self) -> float:
        return self._tokens


# ─── Toll Booth ──────────────────────────────────────────────


class TollBooth:
    """Access control and SLA enforcement for external validators.

    Parameters
    ----------
    default_tier : AccessTier
        Tier assigned to unknown validators (default BASE).
    session_ttl : float
        Default session duration in seconds (default 3600).
    on_denied : callable
        ``fn(validator_id, reason)`` invoked when access is denied.
    on_sla_breach : callable
        ``fn(validator_id, metric, actual, limit)`` on SLA violation.
    """

    def __init__(
        self,
        default_tier: AccessTier = AccessTier.BASE,
        session_ttl: float = 3600.0,
        on_denied: Any = None,
        on_sla_breach: Any = None,
    ) -> None:
        self._default_tier = default_tier
        self._session_ttl = session_ttl
        self._on_denied = on_denied
        self._on_sla_breach = on_sla_breach

        self._lock = threading.Lock()
        self._tickets: dict[str, ValidatorTicket] = {}
        self._buckets: dict[str, _TokenBucket] = {}
        self._tier_overrides: dict[str, AccessTier] = {}
        self._total_denied: int = 0
        self._total_admitted: int = 0

    # ── Registration ─────────────────────────────────────────

    def register_validator(self, validator_id: str, tier: AccessTier) -> None:
        """Register a validator with a specific tier (e.g. from on-chain data)."""
        with self._lock:
            self._tier_overrides[validator_id] = tier

    def get_tier(self, validator_id: str) -> AccessTier:
        """Get the access tier for a validator."""
        with self._lock:
            return self._tier_overrides.get(validator_id, self._default_tier)

    # ── Admission ────────────────────────────────────────────

    def admit(self, validator_id: str, chain_id: str) -> ValidatorTicket | None:
        """Admit a validator and issue a session ticket.

        Returns None if rate limit exceeded or validator is blocked.
        """
        tier = self.get_tier(validator_id)

        with self._lock:
            # Check existing ticket
            existing = self._tickets.get(validator_id)
            if existing and not existing.expired and existing.active:
                # Reuse existing session
                return existing

            # Rate limit check — skip when tier is uncapped (inf)
            rps = _TIER_RPS[tier]
            if rps != float('inf'):
                bucket = self._buckets.get(validator_id)
                if bucket is None:
                    bucket = _TokenBucket(rate=rps, capacity=rps * 2)
                    self._buckets[validator_id] = bucket

                if not bucket.try_consume():
                    self._total_denied += 1
                    logger.warning(
                        "Toll booth: rate limit exceeded for %s (tier=%s)",
                        validator_id, tier.value,
                    )
                    if self._on_denied:
                        try:
                            self._on_denied(validator_id, "rate_limit_exceeded")
                        except Exception:
                            pass
                    return None

            # Issue ticket
            now = time.time()
            ticket = ValidatorTicket(
                validator_id=validator_id,
                chain_id=chain_id,
                tier=tier,
                issued_at=now,
                expires_at=now + self._session_ttl,
            )
            self._tickets[validator_id] = ticket
            self._total_admitted += 1
            return ticket

    def check_ticket(self, validator_id: str) -> bool:
        """Check if a validator has a valid, active ticket."""
        with self._lock:
            ticket = self._tickets.get(validator_id)
            if ticket is None:
                return False
            if ticket.expired:
                ticket.active = False
                return False
            return ticket.active

    def revoke(self, validator_id: str) -> None:
        """Revoke a validator's session."""
        with self._lock:
            ticket = self._tickets.get(validator_id)
            if ticket:
                ticket.active = False

    # ── SLA Enforcement ──────────────────────────────────────

    def check_sla(self, validator_id: str, latency_ms: float) -> bool:
        """Check if a response met the validator's SLA.

        Returns True if within SLA, False if breached.
        Fires ``on_sla_breach`` callback on violation.
        """
        with self._lock:
            ticket = self._tickets.get(validator_id)
            if ticket is None:
                return True  # No ticket = no SLA to enforce

            sla_limit = ticket.latency_sla_ms
            if latency_ms > sla_limit:
                if self._on_sla_breach:
                    try:
                        self._on_sla_breach(
                            validator_id, "latency_ms", latency_ms, sla_limit
                        )
                    except Exception:
                        pass
                return False
            return True

    def check_batch_size(self, validator_id: str, batch_size: int) -> bool:
        """Check if a batch size is within the validator's tier limit."""
        tier = self.get_tier(validator_id)
        return batch_size <= _TIER_BATCH[tier]

    # ── Metering ─────────────────────────────────────────────

    def record_usage(
        self, validator_id: str, requests: int = 1, bytes_count: int = 0
    ) -> None:
        """Record usage for a validator's session."""
        with self._lock:
            ticket = self._tickets.get(validator_id)
            if ticket:
                ticket.record_usage(requests, bytes_count)

    # ── Cleanup ──────────────────────────────────────────────

    def cleanup_expired(self) -> int:
        """Remove expired tickets.  Returns count of cleaned tickets."""
        with self._lock:
            expired_ids = [
                vid for vid, t in self._tickets.items() if t.expired
            ]
            for vid in expired_ids:
                del self._tickets[vid]
                self._buckets.pop(vid, None)
            return len(expired_ids)

    # ── Status ───────────────────────────────────────────────

    def status(self) -> dict[str, Any]:
        with self._lock:
            active_tickets = sum(
                1 for t in self._tickets.values() if t.active and not t.expired
            )
            return {
                "active_tickets": active_tickets,
                "total_admitted": self._total_admitted,
                "total_denied": self._total_denied,
                "registered_validators": len(self._tier_overrides),
                "tier_distribution": {
                    tier.value: sum(
                        1
                        for v, t in self._tier_overrides.items()
                        if t == tier
                    )
                    for tier in AccessTier
                },
            }
