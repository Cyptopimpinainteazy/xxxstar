"""
circuit_breaker.py - Circuit breaker pattern for resilient RPC calls

Prevents cascading failures when blockchain RPC becomes unavailable.
Implements exponential backoff, fast fail, and automatic recovery.
"""

import time
import asyncio
from typing import Callable, Any, Dict, Optional
from enum import Enum
from dataclasses import dataclass


class CircuitState(Enum):
    """Circuit breaker states."""
    CLOSED = "closed"  # Normal operation
    OPEN = "open"  # Failing, reject requests
    HALF_OPEN = "half_open"  # Testing recovery


@dataclass
class CircuitBreakerConfig:
    """Configuration for circuit breaker."""
    failure_threshold: int = 5  # Failures before opening
    recovery_timeout: int = 60  # Seconds before half-open
    success_threshold: int = 2  # Successes before closing
    expected_exception: type = Exception


class CircuitBreaker:
    """
    Circuit breaker for preventing cascading failures.
    
    States:
    - CLOSED: Normal operation, requests pass through
    - OPEN: Too many failures, requests fail fast
    - HALF_OPEN: Testing recovery, limited requests allowed
    """

    def __init__(self, config: CircuitBreakerConfig = None):
        self.config = config or CircuitBreakerConfig()
        self.state = CircuitState.CLOSED
        self.failure_count = 0
        self.success_count = 0
        self.last_failure_time = None
        self.last_state_change = time.time()

    async def call(
        self,
        func: Callable,
        *args,
        **kwargs
    ) -> Any:
        """
        Execute function with circuit breaker protection.
        
        Args:
            func: Async function to execute
            *args: Positional arguments
            **kwargs: Keyword arguments
            
        Returns:
            Function result
            
        Raises:
            CircuitBreakerOpen: If circuit is open
            Original exception: If function fails
        """
        if self.state == CircuitState.OPEN:
            if self._should_attempt_reset():
                self.state = CircuitState.HALF_OPEN
                self.success_count = 0
            else:
                raise CircuitBreakerOpen(
                    f"Circuit breaker OPEN (retry in {self._time_until_reset()}s)"
                )

        try:
            result = await func(*args, **kwargs)
            self._on_success()
            return result

        except Exception as e:
            self._on_failure()
            raise

    def _on_success(self) -> None:
        """Handle successful call."""
        self.failure_count = 0

        if self.state == CircuitState.HALF_OPEN:
            self.success_count += 1
            if self.success_count >= self.config.success_threshold:
                self._reset()

    def _on_failure(self) -> None:
        """Handle failed call."""
        self.failure_count += 1
        self.last_failure_time = time.time()

        if self.failure_count >= self.config.failure_threshold:
            self._open()

    def _open(self) -> None:
        """Open the circuit."""
        if self.state != CircuitState.OPEN:
            print(f"🔴 CIRCUIT BREAKER OPENED after {self.failure_count} failures")
            self.state = CircuitState.OPEN
            self.last_state_change = time.time()

    def _reset(self) -> None:
        """Close the circuit (return to normal operation)."""
        print(f"✅ CIRCUIT BREAKER RESET after {self.success_count} successes")
        self.state = CircuitState.CLOSED
        self.failure_count = 0
        self.success_count = 0
        self.last_state_change = time.time()

    def _should_attempt_reset(self) -> bool:
        """Check if enough time has passed to attempt recovery."""
        if self.last_failure_time is None:
            return False

        elapsed = time.time() - self.last_failure_time
        return elapsed >= self.config.recovery_timeout

    def _time_until_reset(self) -> int:
        """Get seconds until circuit can attempt recovery."""
        if self.last_failure_time is None:
            return 0

        elapsed = time.time() - self.last_failure_time
        remaining = self.config.recovery_timeout - elapsed
        return max(0, int(remaining))

    def get_status(self) -> Dict:
        """Get current circuit breaker status."""
        return {
            "state": self.state.value,
            "failure_count": self.failure_count,
            "success_count": self.success_count,
            "time_until_retry": self._time_until_reset(),
            "uptime_seconds": time.time() - self.last_state_change,
        }


class CircuitBreakerOpen(Exception):
    """Exception raised when circuit breaker is open."""
    pass


class ResilientRPCClient:
    """RPC client with circuit breaker and exponential backoff."""

    def __init__(self, rpc_endpoint: str, max_retries: int = 3):
        self.rpc_endpoint = rpc_endpoint
        self.max_retries = max_retries
        self.circuit_breaker = CircuitBreaker(
            CircuitBreakerConfig(
                failure_threshold=5,
                recovery_timeout=30,
                success_threshold=2,
            )
        )
        self.request_count = 0
        self.error_count = 0

    async def call_rpc(
        self,
        method: str,
        params: list = None,
        timeout: int = 10,
    ) -> Any:
        """
        Make RPC call with automatic retry and circuit breaker.
        
        Args:
            method: RPC method name
            params: RPC parameters
            timeout: Request timeout in seconds
            
        Returns:
            RPC response
        """
        self.request_count += 1

        async def _make_call():
            # Simulated RPC call
            await asyncio.sleep(0.5)  # RPC latency
            return {"result": "success", "method": method}

        try:
            result = await self.circuit_breaker.call(_make_call)
            return result

        except CircuitBreakerOpen as e:
            self.error_count += 1
            raise Exception(f"RPC unavailable: {str(e)}")

        except Exception as e:
            self.error_count += 1

            # Retry with exponential backoff
            for attempt in range(self.max_retries):
                wait_time = (2 ** attempt)  # 1s, 2s, 4s
                print(f"⚠️  RPC failed, retrying in {wait_time}s (attempt {attempt + 1}/{self.max_retries})")

                await asyncio.sleep(wait_time)

                try:
                    result = await self.circuit_breaker.call(_make_call)
                    return result
                except (CircuitBreakerOpen, Exception):
                    if attempt == self.max_retries - 1:
                        raise

            raise

    def get_stats(self) -> Dict:
        """Get resilience statistics."""
        error_rate = (
            self.error_count / self.request_count * 100
            if self.request_count > 0
            else 0
        )

        return {
            "total_requests": self.request_count,
            "errors": self.error_count,
            "error_rate": f"{error_rate:.1f}%",
            "circuit_breaker": self.circuit_breaker.get_status(),
        }


# Global RPC client with circuit breaker
rpc_client = ResilientRPCClient("http://localhost:9944")


async def anchor_with_resilience(
    session_id: str,
    decision_hash: str,
    retries: int = 3,
) -> str:
    """
    Anchor decision with circuit breaker protection.
    
    Automatically handles:
    - RPC node temporary outages
    - Network errors
    - Exponential backoff retries
    """
    try:
        result = await rpc_client.call_rpc(
            "anchor_decision",
            [session_id, decision_hash],
        )
        return result.get("tx_hash", "")

    except Exception as e:
        print(f"❌ Failed to anchor decision: {str(e)}")
        print(f"Stats: {rpc_client.get_stats()}")
        raise


# Example usage:
"""
async def main():
    # This will automatically retry failed calls with exponential backoff
    # If 5 consecutive failures occur, circuit breaker opens
    # After 30 seconds, circuit half-opens to test recovery
    
    try:
        tx_hash = await anchor_with_resilience(
            "session-001",
            "0xabc123..."
        )
        print(f"✅ Anchored with tx: {tx_hash}")
    except Exception as e:
        print(f"❌ Could not anchor: {e}")
        
    # Check status
    stats = rpc_client.get_stats()
    print(json.dumps(stats, indent=2))

asyncio.run(main())
"""
