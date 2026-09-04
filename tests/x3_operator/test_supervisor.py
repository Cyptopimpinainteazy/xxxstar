"""Tests for x3_operator.supervisor"""

import pytest

from x3_operator.config import X3Config
from x3_operator.supervisor import (
    AgentState,
    AgentSupervisor,
    PolicyManifest,
    RateLimiter,
)


@pytest.fixture
def supervisor():
    return AgentSupervisor(X3Config())


@pytest.fixture
def policy():
    return PolicyManifest(
        allowed_endpoints=["/api/task", "/api/result"],
        max_memory_mb=256,
    )


def test_register_agent(supervisor, policy):
    record = supervisor.register_agent("agent-1", "op-001", policy)
    assert record.state == AgentState.REGISTERED
    assert record.policy_hash == policy.policy_hash()


def test_agent_limit(supervisor, policy):
    cfg = X3Config()
    cfg.agent.max_agents_per_operator = 2
    sup = AgentSupervisor(cfg)
    sup.register_agent("a1", "op-001", policy)
    sup.register_agent("a2", "op-001", policy)
    with pytest.raises(ValueError, match="limit reached"):
        sup.register_agent("a3", "op-001", policy)


def test_start_agent(supervisor, policy):
    supervisor.register_agent("agent-1", "op-001", policy)
    record = supervisor.start_agent("agent-1", pid=12345)
    assert record.state == AgentState.RUNNING
    assert record.pid == 12345


def test_heartbeat_ok(supervisor, policy):
    supervisor.register_agent("agent-1", "op-001", policy)
    supervisor.start_agent("agent-1", pid=12345)
    ok = supervisor.heartbeat("agent-1", policy.policy_hash())
    assert ok is True


def test_heartbeat_policy_drift(supervisor, policy):
    supervisor.register_agent("agent-1", "op-001", policy)
    supervisor.start_agent("agent-1", pid=12345)
    ok = supervisor.heartbeat("agent-1", "wrong_hash")
    assert ok is False
    record = supervisor.agents["agent-1"]
    assert record.violations == 1


def test_rate_limiter():
    rl = RateLimiter(max_calls=3, window_seconds=60.0)
    assert rl.check("a1") is True
    assert rl.check("a1") is True
    assert rl.check("a1") is True
    assert rl.check("a1") is False  # exceeded


def test_rate_limiter_different_agents():
    rl = RateLimiter(max_calls=2, window_seconds=60.0)
    assert rl.check("a1") is True
    assert rl.check("a1") is True
    assert rl.check("a1") is False
    assert rl.check("a2") is True  # different agent, fresh bucket


def test_kill_switch(supervisor, policy):
    supervisor.register_agent("a1", "op-001", policy)
    supervisor.register_agent("a2", "op-001", policy)
    supervisor.start_agent("a1", pid=99998)
    supervisor.start_agent("a2", pid=99999)
    supervisor.arm_kill_switch()

    assert supervisor.agents["a1"].state == AgentState.KILLED
    assert supervisor.agents["a2"].state == AgentState.KILLED
    summary = supervisor.status_summary()
    assert summary["kill_switch_armed"] is True
    assert summary["global_pause"] is True


def test_status_summary(supervisor, policy):
    supervisor.register_agent("a1", "op-001", policy)
    supervisor.start_agent("a1", pid=12345)
    summary = supervisor.status_summary()
    assert summary["total_agents"] == 1
    assert summary["states"]["running"] == 1


def test_policy_hash_deterministic():
    p1 = PolicyManifest(allowed_endpoints=["/a", "/b"], max_memory_mb=512)
    p2 = PolicyManifest(allowed_endpoints=["/a", "/b"], max_memory_mb=512)
    assert p1.policy_hash() == p2.policy_hash()


def test_policy_hash_changes_with_params():
    p1 = PolicyManifest(max_memory_mb=512)
    p2 = PolicyManifest(max_memory_mb=1024)
    assert p1.policy_hash() != p2.policy_hash()


def test_check_rate_limit(supervisor, policy):
    supervisor.register_agent("a1", "op-001", policy)
    supervisor.start_agent("a1", pid=12345)

    for _ in range(60):
        assert supervisor.check_rate_limit("a1") is True

    assert supervisor.check_rate_limit("a1") is False
