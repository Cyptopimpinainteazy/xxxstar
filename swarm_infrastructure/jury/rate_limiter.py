"""
rate_limiter.py - Rate limiting middleware for jury service

Protects against DoS attacks, abuse, and resource exhaustion.
Implements token bucket algorithm with per-IP tracking.
"""

import time
from typing import Dict, Tuple
from collections import defaultdict
import asyncio


class RateLimiter:
    """Token bucket rate limiter for production APIs."""

    def __init__(
        self,
        requests_per_minute: int = 100,
        burst_size: int = 20,
        cleanup_interval: int = 300,
    ):
        """
        Initialize rate limiter.

        Args:
            requests_per_minute: Sustained rate per IP
            burst_size: Tokens available for burst
            cleanup_interval: Seconds between token bucket cleanup
        """
        self.requests_per_minute = requests_per_minute
        self.burst_size = burst_size
        self.tokens_per_second = requests_per_minute / 60.0

        # Track tokens per IP: {ip: (tokens, last_refill_time)}
        self.buckets: Dict[str, Tuple[float, float]] = defaultdict(
            lambda: (burst_size, time.time())
        )

        # Start cleanup task
        self.cleanup_interval = cleanup_interval
        self._cleanup_task = None

    async def start_cleanup(self) -> None:
        """Start background cleanup task."""
        self._cleanup_task = asyncio.create_task(self._periodic_cleanup())

    async def stop_cleanup(self) -> None:
        """Stop background cleanup task."""
        if self._cleanup_task:
            self._cleanup_task.cancel()

    async def _periodic_cleanup(self) -> None:
        """Periodically clean up old entries."""
        while True:
            try:
                await asyncio.sleep(self.cleanup_interval)
                current_time = time.time()
                # Remove entries older than 1 hour
                self.buckets = {
                    ip: (tokens, last_time)
                    for ip, (tokens, last_time) in self.buckets.items()
                    if current_time - last_time < 3600
                }
            except asyncio.CancelledError:
                break

    def _refill_tokens(self, ip: str, current_time: float) -> Tuple[float, float]:
        """Refill tokens based on elapsed time."""
        tokens, last_time = self.buckets[ip]
        elapsed = current_time - last_time

        # Add tokens for elapsed time
        new_tokens = min(
            self.burst_size,
            tokens + elapsed * self.tokens_per_second,
        )

        return new_tokens, current_time

    def check_rate_limit(self, ip: str, cost: int = 1) -> Tuple[bool, Dict]:
        """
        Check if request from IP is within rate limit.

        Args:
            ip: Client IP address
            cost: Token cost for this request (default 1)

        Returns:
            (allowed: bool, info: dict with remaining tokens and reset time)
        """
        current_time = time.time()
        tokens, last_time = self.buckets.get(ip, (self.burst_size, current_time))

        # Refill tokens
        tokens, last_time = self._refill_tokens(ip, current_time)

        # Check if request allowed
        allowed = tokens >= cost

        if allowed:
            tokens -= cost
            self.buckets[ip] = (tokens, current_time)
            remaining = int(tokens)
        else:
            remaining = 0

        # Calculate time until next token available
        if not allowed:
            time_until_reset = (cost - tokens) / self.tokens_per_second
        else:
            time_until_reset = 0

        return allowed, {
            "remaining": remaining,
            "limit": self.requests_per_minute,
            "retry_after": int(time_until_reset) + 1,
            "reset_at": current_time + time_until_reset,
        }

    def get_status(self, ip: str) -> Dict:
        """Get current rate limit status for IP."""
        current_time = time.time()
        if ip not in self.buckets:
            return {
                "ip": ip,
                "tokens": self.burst_size,
                "limit": self.requests_per_minute,
                "reset_in": 60,
                "status": "new",
            }

        tokens, last_time = self.buckets[ip]
        tokens, _ = self._refill_tokens(ip, current_time)

        return {
            "ip": ip,
            "tokens": int(tokens),
            "limit": self.requests_per_minute,
            "reset_in": int((self.burst_size - tokens) / self.tokens_per_second),
            "status": "active",
        }


class AdaptiveRateLimiter(RateLimiter):
    """Rate limiter that adapts limits based on system load."""

    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self.system_load = 0.0  # 0.0 - 1.0
        self.adaptive_enabled = True

    def update_system_load(self, load: float) -> None:
        """Update current system load (0.0 = idle, 1.0 = overloaded)."""
        self.system_load = max(0.0, min(1.0, load))

    def get_adaptive_limit(self) -> int:
        """Get rate limit adjusted for current system load."""
        if not self.adaptive_enabled:
            return self.requests_per_minute

        # When system is under load, reduce limits
        # At 100% load, reduce to 20% of normal
        reduction_factor = 1.0 - (self.system_load * 0.8)
        return int(self.requests_per_minute * reduction_factor)

    def check_rate_limit(self, ip: str, cost: int = 1) -> Tuple[bool, Dict]:
        """Check rate limit with adaptive adjustment."""
        allowed, info = super().check_rate_limit(ip, cost)

        # On high system load, be stricter
        if self.system_load > 0.8 and self.requests_per_minute > 20:
            # Only allow burst traffic, queue others
            strict_mode = False
            if info["remaining"] < 2:
                strict_mode = True

            if strict_mode:
                allowed = False

        info["system_load"] = self.system_load
        info["adaptive_limit"] = self.get_adaptive_limit()

        return allowed, info


# Global rate limiter instance
global_rate_limiter = RateLimiter(
    requests_per_minute=100,
    burst_size=20,
)


async def rate_limit_middleware(ip: str, path: str = "") -> Tuple[bool, Dict]:
    """Middleware function for FastAPI/similar."""
    allowed, info = global_rate_limiter.check_rate_limit(ip)

    if not allowed:
        return False, {
            "error": "Rate limit exceeded",
            "retry_after": info["retry_after"],
            "limit_reset": info["reset_at"],
        }

    return True, {"remaining": info["remaining"]}


# Example FastAPI integration:
"""
from fastapi import FastAPI, Request
from fastapi.responses import JSONResponse

app = FastAPI()

@app.middleware("http")
async def rate_limit_middleware_fastapi(request: Request, call_next):
    client_ip = request.client.host
    allowed, info = await rate_limit_middleware(client_ip, request.url.path)

    if not allowed:
        return JSONResponse(
            status_code=429,
            content=info,
            headers={"Retry-After": str(info["retry_after"])},
        )

    response = await call_next(request)
    response.headers["X-RateLimit-Remaining"] = str(info["remaining"])
    return response

@app.on_event("startup")
async def startup():
    await global_rate_limiter.start_cleanup()

@app.on_event("shutdown")
async def shutdown():
    await global_rate_limiter.stop_cleanup()
"""
