"""
X3 Agent Supervisor
~~~~~~~~~~~~~~~~~~~

Process-level agent supervision with kill-switch,
rate limiting, policy hash enforcement, and resource budgets.
"""

import hashlib
import logging
import os
import signal
import time
from dataclasses import dataclass, field
from enum import Enum
from typing import Optional

from .config import X3Config

logger = logging.getLogger(__name__)


class AgentState(str, Enum):
    REGISTERED = "registered"
    RUNNING = "running"
    PAUSED = "paused"
    KILLED = "killed"
    EXITED = "exited"


@dataclass
class PolicyManifest:
    """Immutable policy hash for agent behavior contract."""
    allowed_endpoints: list = field(default_factory=list)
    max_memory_mb: int = 512
    max_cpu_percent: float = 25.0
    max_network_bytes_sec: int = 10 * 1024 * 1024  # 10 MB/s
    can_spawn_children: bool = False
    sandbox_mode: str = "strict"

    def policy_hash(self) -> str:
        payload = (
            f"{sorted(self.allowed_endpoints)}:{self.max_memory_mb}:"
            f"{self.max_cpu_percent}:{self.max_network_bytes_sec}:"
            f"{self.can_spawn_children}:{self.sandbox_mode}"
        )
        return hashlib.sha256(payload.encode()).hexdigest()


@dataclass
class AgentRecord:
    agent_id: str
    operator_id: str
    pid: Optional[int] = None
    state: AgentState = AgentState.REGISTERED
    policy: Optional[PolicyManifest] = None
    policy_hash: str = ""
    started_at: float = 0.0
    last_heartbeat: float = 0.0
    call_count: int = 0
    call_window_start: float = 0.0
    violations: int = 0
    kill_reason: str = ""


class RateLimiter:
    """Token bucket rate limiter for agent API calls."""

    def __init__(self, max_calls: int, window_seconds: float):
        self.max_calls = max_calls
        self.window_seconds = window_seconds
        self._buckets: dict[str, list[float]] = {}

    def check(self, agent_id: str) -> bool:
        """Returns True if the call is allowed."""
        now = time.time()
        calls = self._buckets.setdefault(agent_id, [])
        # Expire old calls
        cutoff = now - self.window_seconds
        self._buckets[agent_id] = [t for t in calls if t > cutoff]
        calls = self._buckets[agent_id]

        if len(calls) >= self.max_calls:
            return False

        calls.append(now)
        return True

    def reset(self, agent_id: str):
        self._buckets.pop(agent_id, None)


class AgentSupervisor:
    """Manages agent lifecycle with kill-switch capability.

    Enforces:
    - Policy hash verification (no runtime policy drift)
    - Rate limiting per agent
    - Resource budget tracking
    - Kill-switch with configurable delay
    - Heartbeat-based liveness detection
    """

    def __init__(self, config: X3Config):
        self.config = config
        self.agents: dict[str, AgentRecord] = {}
        self.rate_limiter = RateLimiter(
            max_calls=config.agent.max_calls_per_minute,
            window_seconds=60.0,
        )
        self._kill_switch_armed = False
        self._global_pause = False

    def register_agent(self, agent_id: str, operator_id: str, policy: PolicyManifest) -> AgentRecord:
        """Register a new agent with its policy manifest."""
        if len(self.agents) >= self.config.agent.max_agents_per_operator:
            raise ValueError(
                f"Agent limit reached ({self.config.agent.max_agents_per_operator})"
            )

        record = AgentRecord(
            agent_id=agent_id,
            operator_id=operator_id,
            policy=policy,
            policy_hash=policy.policy_hash(),
        )
        self.agents[agent_id] = record
        logger.info("agent registered: id=%s operator=%s policy=%s",
                     agent_id, operator_id, record.policy_hash[:16])
        return record

    def start_agent(self, agent_id: str, pid: int) -> AgentRecord:
        """Mark agent as running with its OS process ID."""
        record = self._get(agent_id)
        if self._global_pause:
            raise RuntimeError("Global pause active - no new agents can start")
        if record.state not in (AgentState.REGISTERED, AgentState.PAUSED, AgentState.EXITED):
            raise ValueError(f"Cannot start from state: {record.state.value}")

        record.pid = pid
        record.state = AgentState.RUNNING
        record.started_at = time.time()
        record.last_heartbeat = time.time()
        logger.info("agent started: id=%s pid=%d", agent_id, pid)
        return record

    def heartbeat(self, agent_id: str, current_policy_hash: str) -> bool:
        """Process heartbeat from agent. Returns False if policy drifted."""
        record = self._get(agent_id)
        record.last_heartbeat = time.time()

        if current_policy_hash != record.policy_hash:
            record.violations += 1
            logger.error(
                "policy drift detected: agent=%s expected=%s got=%s violations=%d",
                agent_id, record.policy_hash[:16], current_policy_hash[:16], record.violations,
            )
            if record.violations >= 3:
                self.kill_agent(agent_id, "policy drift exceeded threshold")
            return False
        return True

    def check_rate_limit(self, agent_id: str) -> bool:
        """Check if agent is within rate limits."""
        if not self.rate_limiter.check(agent_id):
            record = self._get(agent_id)
            record.violations += 1
            logger.warning("rate limit exceeded: agent=%s violations=%d",
                           agent_id, record.violations)
            return False
        record = self._get(agent_id)
        record.call_count += 1
        return True

    def kill_agent(self, agent_id: str, reason: str):
        """Send SIGTERM (then SIGKILL) to agent process."""
        record = self._get(agent_id)
        record.state = AgentState.KILLED
        record.kill_reason = reason
        logger.warning("killing agent: id=%s pid=%s reason=%s", agent_id, record.pid, reason)

        if record.pid:
            try:
                os.kill(record.pid, signal.SIGTERM)
                # Give agent grace period
                time.sleep(min(self.config.agent.kill_switch_delay_seconds, 2.0))
                # Check if still alive
                try:
                    os.kill(record.pid, 0)  # signal 0 = check existence
                    os.kill(record.pid, signal.SIGKILL)
                    logger.warning("force killed agent: id=%s pid=%d", agent_id, record.pid)
                except ProcessLookupError:
                    pass  # already dead
            except ProcessLookupError:
                pass  # already dead
            except PermissionError:
                logger.error("no permission to kill agent pid=%d", record.pid)

    def pause_agent(self, agent_id: str):
        """Pause agent (SIGSTOP)."""
        record = self._get(agent_id)
        if record.state != AgentState.RUNNING:
            return
        record.state = AgentState.PAUSED
        if record.pid:
            try:
                os.kill(record.pid, signal.SIGSTOP)
            except (ProcessLookupError, PermissionError):
                pass

    def resume_agent(self, agent_id: str):
        """Resume paused agent (SIGCONT)."""
        record = self._get(agent_id)
        if record.state != AgentState.PAUSED:
            return
        record.state = AgentState.RUNNING
        record.last_heartbeat = time.time()
        if record.pid:
            try:
                os.kill(record.pid, signal.SIGCONT)
            except (ProcessLookupError, PermissionError):
                pass

    def arm_kill_switch(self):
        """Arm global kill switch - kills all agents."""
        self._kill_switch_armed = True
        self._global_pause = True
        logger.critical("KILL SWITCH ARMED - terminating all agents")
        for agent_id, record in self.agents.items():
            if record.state == AgentState.RUNNING:
                self.kill_agent(agent_id, "global kill switch")

    def check_liveness(self, timeout_seconds: float = 60.0) -> list[str]:
        """Check for agents that missed heartbeat. Returns list of dead agent IDs."""
        now = time.time()
        dead = []
        for agent_id, record in self.agents.items():
            if record.state != AgentState.RUNNING:
                continue
            if now - record.last_heartbeat > timeout_seconds:
                dead.append(agent_id)
                self.kill_agent(agent_id, f"missed heartbeat ({timeout_seconds}s)")
        return dead

    def status_summary(self) -> dict:
        states = {}
        for record in self.agents.values():
            states[record.state.value] = states.get(record.state.value, 0) + 1
        return {
            "total_agents": len(self.agents),
            "states": states,
            "global_pause": self._global_pause,
            "kill_switch_armed": self._kill_switch_armed,
        }

    def _get(self, agent_id: str) -> AgentRecord:
        record = self.agents.get(agent_id)
        if record is None:
            raise KeyError(f"Unknown agent: {agent_id}")
        return record
