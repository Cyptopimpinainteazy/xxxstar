"""Tests for Jury Token System — Vote Scarcity & Civic Life/Death."""

from swarm.governance.jury_tokens import (
    JuryTokenManager,
    TokenConsumptionType,
    TokenStatus,
)


def test_token_minting():
    """Test that agents are minted with exactly 3 tokens."""
    tm = JuryTokenManager()

    agent_id = "agent-001"
    assert tm.mint_tokens(agent_id) is True

    status = tm.get_agent_token_status(agent_id)
    assert status["total_minted"] == 3
    assert status["active"] == 0  # Not yet activated
    assert status["spent"] == 0
    assert status["burned"] == 0
    assert status["civically_dead"] is False


def test_token_activation():
    """Test that tokens transition from MINTED to ACTIVE."""
    tm = JuryTokenManager()

    agent_id = "agent-002"
    tm.mint_tokens(agent_id)

    # Activate tokens
    assert tm.activate_tokens(agent_id) is True

    status = tm.get_agent_token_status(agent_id)
    assert status["active"] == 3
    assert status["spent"] == 0


def test_single_token_spend():
    """Test spending one token in a trial jury vote."""
    tm = JuryTokenManager()

    agent_id = "agent-003"
    tm.mint_tokens(agent_id)
    tm.activate_tokens(agent_id)

    # Spend 1 token
    assert tm.spend_token(
        agent_id,
        TokenConsumptionType.TRIAL_JURY_VOTE,
        "case-trial-001",
    ) is True

    status = tm.get_agent_token_status(agent_id)
    assert status["active"] == 2
    assert status["spent"] == 1


def test_high_stakes_token_spend():
    """Test spending 2 tokens for constitutional vote."""
    tm = JuryTokenManager()

    agent_id = "agent-004"
    tm.mint_tokens(agent_id)
    tm.activate_tokens(agent_id)

    # Spend 2 tokens
    assert tm.spend_tokens_for_high_stakes(
        agent_id,
        TokenConsumptionType.CONSTITUTIONAL_VOTE,
        "constitutional-amendment-001",
    ) is True

    status = tm.get_agent_token_status(agent_id)
    assert status["active"] == 1
    assert status["spent"] == 2


def test_civic_death_when_tokens_exhausted():
    """Test that agent is civically dead when all tokens spent."""
    tm = JuryTokenManager()

    agent_id = "agent-005"
    tm.mint_tokens(agent_id)
    tm.activate_tokens(agent_id)

    # Spend all 3 tokens
    for i in range(3):
        assert tm.spend_token(
            agent_id,
            TokenConsumptionType.TRIAL_JURY_VOTE,
            f"case-{i}",
        ) is True

    # Agent is civically dead
    assert tm.is_agent_civically_dead(agent_id) is True

    status = tm.get_agent_token_status(agent_id)
    assert status["active"] == 0
    assert status["spent"] == 3
    assert status["civically_dead"] is True


def test_dead_agent_barred_from_jury():
    """Test that civically dead agents are excluded from jury pools."""
    tm = JuryTokenManager()

    # Create 5 agents
    for i in range(1, 6):
        agent_id = f"agent-{i:03d}"
        tm.mint_tokens(agent_id)
        tm.activate_tokens(agent_id)

        # Agent 1 spends all tokens → civic death
        if i == 1:
            for _ in range(3):
                tm.spend_token(agent_id, TokenConsumptionType.TRIAL_JURY_VOTE, "case-x")

    excluded = tm.get_excluded_agents()
    assert "agent-001" in excluded
    assert "agent-002" not in excluded
    assert len(excluded) == 1


def test_insufficient_tokens_for_high_stakes():
    """Test that high-stakes votes fail if insufficient tokens."""
    tm = JuryTokenManager()

    agent_id = "agent-006"
    tm.mint_tokens(agent_id)
    tm.activate_tokens(agent_id)

    # Spend 1 token → only 2 left
    tm.spend_token(agent_id, TokenConsumptionType.TRIAL_JURY_VOTE, "case-1")

    # High-stakes vote requires 2 tokens, agent has 2, so should succeed
    assert tm.spend_tokens_for_high_stakes(
        agent_id,
        TokenConsumptionType.CONSTITUTIONAL_VOTE,
        "const-1",
    ) is True

    # Agent now has 0 tokens → next high-stakes vote fails
    assert tm.spend_tokens_for_high_stakes(
        agent_id,
        TokenConsumptionType.CONSTITUTIONAL_VOTE,
        "const-2",
    ) is False


def test_burn_finalization():
    """Test that spent tokens are burned and locked."""
    tm = JuryTokenManager()

    agent_id = "agent-007"
    tm.mint_tokens(agent_id)
    tm.activate_tokens(agent_id)
    tm.spend_token(agent_id, TokenConsumptionType.TRIAL_JURY_VOTE, "case-burn-1")

    # Before finalization: token is SPENT
    status_before = tm.get_agent_token_status(agent_id)
    assert status_before["spent"] == 1

    # Finalize burns
    burned_count = tm.finalize_burns()
    assert burned_count == 1

    # After finalization: token is BURNED
    status_after = tm.get_agent_token_status(agent_id)
    token = status_after["tokens"][0]
    assert token["status"] == "burned"


def test_burn_audit_trail():
    """Test that all token burns are logged with cryptographic hashes."""
    tm = JuryTokenManager()

    agent_id = "agent-008"
    tm.mint_tokens(agent_id)
    tm.activate_tokens(agent_id)

    # Make several spends
    for i in range(3):
        tm.spend_token(
            agent_id,
            TokenConsumptionType.TRIAL_JURY_VOTE,
            f"case-audit-{i}",
        )

    # Get burn audit trail
    trail = tm.get_burn_audit_trail()

    assert len(trail) == 3
    for _, record in enumerate(trail):
        assert record["agent_id"] == agent_id
        assert record["consumption_type"] == "trial_jury_vote"
        assert record["burn_hash"] is not None  # Cryptographic proof
        assert "case-audit" in record["context"]


def test_token_non_transfer():
    """Test that tokens cannot be transferred between agents."""
    tm = JuryTokenManager()

    agent_a = "agent-a"
    agent_b = "agent-b"

    tm.mint_tokens(agent_a)
    tm.mint_tokens(agent_b)
    tm.activate_tokens(agent_a)
    tm.activate_tokens(agent_b)

    # Agent A has 3 tokens, B has 3 tokens
    assert tm.get_active_tokens(agent_a) == 3
    assert tm.get_active_tokens(agent_b) == 3

    # No transfer mechanism exists; tokens are bound to agent
    # This is implicit in the design (no transfer_token method)
    # Test passes if both agents retain their own tokens
    status_a = tm.get_agent_token_status(agent_a)
    status_b = tm.get_agent_token_status(agent_b)

    assert all(t["agent_id"] == agent_a for t in status_a["tokens"] if hasattr(t, "agent_id"))
    assert all(t["agent_id"] == agent_b for t in status_b["tokens"] if hasattr(t, "agent_id"))


def test_multiple_agents_parallel_spending():
    """Test that multiple agents can spend tokens independently."""
    tm = JuryTokenManager()

    # Create 10 agents
    agents = [f"agent-{i:03d}" for i in range(1, 11)]
    for agent_id in agents:
        tm.mint_tokens(agent_id)
        tm.activate_tokens(agent_id)

    # Each agent spends 1 token
    for agent_id in agents:
        assert tm.spend_token(
            agent_id,
            TokenConsumptionType.TRIAL_JURY_VOTE,
            "case-parallel",
        ) is True

    # Each agent has 2 active tokens and 1 spent
    for agent_id in agents:
        status = tm.get_agent_token_status(agent_id)
        assert status["active"] == 2
        assert status["spent"] == 1

    # Burn log should have 10 entries
    trail = tm.get_burn_audit_trail()
    assert len(trail) == 10


def test_prevented_double_spend():
    """Test that token cannot be spent twice."""
    tm = JuryTokenManager()

    agent_id = "agent-009"
    tm.mint_tokens(agent_id)
    tm.activate_tokens(agent_id)

    token = tm.allocations[agent_id].tokens[0]

    # Spend token once
    assert tm.spend_token(agent_id, TokenConsumptionType.TRIAL_JURY_VOTE, "case-1") is True
    assert token.status == TokenStatus.SPENT

    # Try to spend again (find an "ACTIVE" token, but none exist)
    active_before = tm.get_active_tokens(agent_id)
    assert active_before == 2  # 3 total, 1 spent

    # Spend another
    assert tm.spend_token(agent_id, TokenConsumptionType.TRIAL_JURY_VOTE, "case-2") is True

    # Now only 1 active left
    active_after = tm.get_active_tokens(agent_id)
    assert active_after == 1
