"""Tests for jury token lifecycle system."""

from swarm.jury.tokens import TokenManager, TokenSpendReason, TokenStatus


def test_token_minting():
    """Test that agents mint exactly 3 tokens."""
    tm = TokenManager()
    ledger = tm.mint_agent("agent-001")

    assert len(ledger.tokens) == 3
    assert all(t.status == TokenStatus.ACTIVE for t in ledger.tokens)
    assert ledger.active_tokens() == 3
    assert not ledger.civic_death()


def test_token_spending():
    """Test token consumption and state transitions."""
    tm = TokenManager()
    tm.mint_agent("agent-001")

    # Spend first token
    assert tm.spend_token("agent-001", "case-1", TokenSpendReason.TRIAL_JURY_VOTE)
    assert tm.get_ledger("agent-001").active_tokens() == 2

    # Spend second token
    assert tm.spend_token("agent-001", "case-2", TokenSpendReason.APPEAL_JURY_VOTE)
    assert tm.get_ledger("agent-001").active_tokens() == 1

    # Spend third token
    assert tm.spend_token("agent-001", "case-3", TokenSpendReason.PRECEDENT_VOTING)
    assert tm.get_ledger("agent-001").active_tokens() == 0


def test_civic_death():
    """Test that agents become civically dead at 0 tokens."""
    tm = TokenManager()
    tm.mint_agent("agent-002")

    assert tm.is_civically_alive("agent-002") is True

    # Spend all 3 tokens
    for i in range(3):
        tm.spend_token("agent-002", f"case-{i}", TokenSpendReason.TRIAL_JURY_VOTE)

    assert tm.is_civically_alive("agent-002") is False
    assert tm.get_ledger("agent-002").civic_death() is True


def test_civic_death_logging():
    """Test that civic death is logged in CIVIC_SELF_TERMINATION_PATTERN."""
    tm = TokenManager()
    tm.mint_agent("agent-003")

    # Spend all tokens
    for i in range(3):
        tm.spend_token("agent-003", f"case-{i}", TokenSpendReason.TRIAL_JURY_VOTE)

    death_log = tm.get_civic_death_log()
    assert len(death_log) == 1
    assert death_log[0]["agent_id"] == "agent-003"
    assert death_log[0]["pattern"] == "CIVIC_SELF_TERMINATION"


def test_eligible_juror_filtering():
    """Test that civically dead agents are excluded from jury pools."""
    tm = TokenManager()

    # Create 5 agents
    agents = ["agent-a", "agent-b", "agent-c", "agent-d", "agent-e"]
    for agent in agents:
        tm.mint_agent(agent)

    # Kill agent-b and agent-d (spend all tokens)
    for agent in ["agent-b", "agent-d"]:
        for i in range(3):
            tm.spend_token(agent, f"case-{i}", TokenSpendReason.TRIAL_JURY_VOTE)

    # Get eligible jurors
    eligible = tm.get_eligible_jurors(agents)
    assert len(eligible) == 3
    assert "agent-b" not in eligible
    assert "agent-d" not in eligible
    assert set(eligible) == {"agent-a", "agent-c", "agent-e"}


def test_token_burn():
    """Test immutable token burning and cryptographic hashing."""
    tm = TokenManager()
    ledger = tm.mint_agent("agent-004")

    # Spend a token
    tm.spend_token("agent-004", "case-burn", TokenSpendReason.TRIAL_JURY_VOTE)

    # Burn spent tokens
    burn_hashes = tm.burn_spent_tokens("agent-004")

    assert len(burn_hashes) == 1
    assert burn_hashes[0] is not None
    assert len(burn_hashes[0]) == 64  # SHA256 hex length

    # Verify token status
    spent_token = next((t for t in ledger.tokens if t.spend_case_id == "case-burn"), None)
    assert spent_token is not None
    assert spent_token.status == TokenStatus.BURNED
    assert spent_token.burn_hash == burn_hashes[0]


def test_token_spend_log():
    """Test that all token spending is logged."""
    tm = TokenManager()
    tm.mint_agent("agent-005")

    # Perform various spend operations
    tm.spend_token("agent-005", "case-1", TokenSpendReason.TRIAL_JURY_VOTE)
    tm.spend_token("agent-005", "case-2", TokenSpendReason.APPEAL_JURY_VOTE)
    tm.spend_token("agent-005", "case-3", TokenSpendReason.PRECEDENT_CREATION)

    log = tm.get_token_spend_log()
    assert len(log) == 3
    assert all(entry["agent_id"] == "agent-005" for entry in log)
    assert log[0]["remaining_tokens"] == 2
    assert log[1]["remaining_tokens"] == 1
    assert log[2]["remaining_tokens"] == 0


def test_system_summary():
    """Test system-wide token statistics."""
    tm = TokenManager()

    # Create 10 agents
    for i in range(10):
        tm.mint_agent(f"agent-{i:02d}")

    # Kill 2 agents
    for i in range(2):
        agent = f"agent-{i:02d}"
        for j in range(3):
            tm.spend_token(agent, f"case-{i}-{j}", TokenSpendReason.TRIAL_JURY_VOTE)

    summary = tm.get_system_summary()
    assert summary["total_agents"] == 10
    assert summary["civically_alive"] == 8
    assert summary["civically_dead"] == 2
    assert summary["total_tokens_minted"] == 30
    assert summary["total_tokens_spent"] == 6
    assert summary["civic_death_events"] == 2
    assert abs(summary["civic_death_percentage"] - 0.2) < 0.01


def test_token_idempotence():
    """Test that repeated minting doesn't re-mint tokens."""
    tm = TokenManager()

    ledger1 = tm.mint_agent("agent-006")
    assert ledger1.active_tokens() == 3

    # Spend one token
    tm.spend_token("agent-006", "case-1", TokenSpendReason.TRIAL_JURY_VOTE)

    # Try to mint again (should return existing ledger)
    ledger2 = tm.mint_agent("agent-006")
    assert ledger2 is ledger1
    assert ledger2.active_tokens() == 2  # Still has 2 left


def test_high_stakes_appeal_token_consumption():
    """Test that high-stakes appeals consume extra tokens (documented for later)."""
    # This is a placeholder for future high-stakes appeal logic
    # High-stakes appeals require 2 tokens per agent instead of 1
    tm = TokenManager()
    tm.mint_agent("agent-007")

    # Regular appeal: 1 token
    tm.spend_token("agent-007", "appeal-normal", TokenSpendReason.APPEAL_JURY_VOTE)
    assert tm.get_ledger("agent-007").active_tokens() == 2

    # High-stakes appeal (simulated as 2 spend calls)
    tm.spend_token("agent-007", "appeal-high-stakes", TokenSpendReason.HIGH_STAKES_APPEAL)
    # In full implementation, this would consume 2 at once,
    # but for now it's documented for appeals layer to enforce
    assert tm.get_ledger("agent-007").active_tokens() == 1
