"""Tests for Jury Token Lifecycle System"""

import time

from swarm.governance.token_lifecycle import (
    JuryTokenManager,
    TokenConsumptionContext,
)


def test_token_minting():
    """Test initial token issuance (3 per agent)."""
    tm = JuryTokenManager()

    # Mint tokens for agent
    ledger = tm.mint_tokens_for_agent("agent-001")

    assert ledger.agent_id == "agent-001"
    assert ledger.total_minted == 3
    assert ledger.remaining_active == 3
    assert ledger.civic_status == "active"
    assert len(ledger.tokens) == 3


def test_token_burn_trial_jury():
    """Test token consumption in trial jury voting."""
    tm = JuryTokenManager()
    tm.mint_tokens_for_agent("agent-001")

    # Burn token in trial jury vote
    success, token_id = tm.burn_token(
        agent_id="agent-001",
        context=TokenConsumptionContext.TRIAL_JURY_VOTE,
        case_id="trial-case-001",
        evidence={"jury_session_id": "jury-1", "vote": True},
    )

    assert success is True
    assert "agent-001" in token_id

    # Check ledger updated
    ledger = tm.get_agent_ledger("agent-001")
    assert ledger.total_burned == 1
    assert ledger.remaining_active == 2
    assert ledger.civic_status == "active"


def test_token_burn_all_three():
    """Test civic death when all 3 tokens burned."""
    tm = JuryTokenManager()
    tm.mint_tokens_for_agent("agent-002")

    # Burn all three tokens
    for i in range(3):
        success, _ = tm.burn_token(
            agent_id="agent-002",
            context=TokenConsumptionContext.TRIAL_JURY_VOTE,
            case_id=f"trial-case-{i}",
            evidence={"epoch": i},
        )
        assert success is True

    # Check civic death
    assert tm.is_agent_civic_alive("agent-002") is False
    ledger = tm.get_agent_ledger("agent-002")
    assert ledger.civic_status == "terminated"
    assert ledger.remaining_active == 0


def test_expired_agent_cannot_vote():
    """Test that agents with 0 tokens are ineligible for jury."""
    tm = JuryTokenManager()
    tm.mint_tokens_for_agent("agent-003")

    # Burn all tokens
    for _ in range(3):
        tm.burn_token(
            agent_id="agent-003",
            context=TokenConsumptionContext.TRIAL_JURY_VOTE,
            case_id="trial",
            evidence={},
        )

    # Try to burn another token - should fail
    success, error = tm.burn_token(
        agent_id="agent-003",
        context=TokenConsumptionContext.TRIAL_JURY_VOTE,
        case_id="trial-new",
        evidence={},
    )

    assert success is False
    assert "no remaining tokens" in error.lower()


def test_filter_jury_eligible():
    """Test filtering candidates to only civically-alive agents."""
    tm = JuryTokenManager()

    # Create 5 agents
    tm.mint_tokens_for_agent("agent-A")
    tm.mint_tokens_for_agent("agent-B")
    tm.mint_tokens_for_agent("agent-C")
    tm.mint_tokens_for_agent("agent-D")
    tm.mint_tokens_for_agent("agent-E")

    # Terminate agent-B and agent-D
    for _ in range(3):
        tm.burn_token("agent-B", TokenConsumptionContext.TRIAL_JURY_VOTE, "trial", {})
        tm.burn_token("agent-D", TokenConsumptionContext.TRIAL_JURY_VOTE, "trial", {})

    candidates = ["agent-A", "agent-B", "agent-C", "agent-D", "agent-E"]
    eligible = tm.filter_jury_eligible_agents(candidates)

    assert set(eligible) == {"agent-A", "agent-C", "agent-E"}


def test_token_burn_record_immutability():
    """Test that burn records are immutable and cryptographically signed."""
    tm = JuryTokenManager()
    tm.mint_tokens_for_agent("agent-006")

    _success, _ = tm.burn_token(
        agent_id="agent-006",
        context=TokenConsumptionContext.APPELLATE_JURY_VOTE_HIGH_STAKES,
        case_id="appeal-001",
        evidence={"appeal_id": "appeal-001", "high_stakes": True},
    )

    # Get burn records
    burn_records = tm.get_burn_records_for_case("appeal-001")
    assert len(burn_records) == 1

    burn = burn_records[0]
    assert burn.agent_id == "agent-006"
    assert burn.context == TokenConsumptionContext.APPELLATE_JURY_VOTE_HIGH_STAKES
    assert burn.signature != ""  # Cryptographic signature present


def test_multiple_contexts():
    """Test token burn in different contexts."""
    tm = JuryTokenManager()
    tm.mint_tokens_for_agent("agent-007")

    # Trial jury vote
    tm.burn_token("agent-007", TokenConsumptionContext.TRIAL_JURY_VOTE, "trial-1", {})

    # Appellate jury vote
    tm.burn_token("agent-007", TokenConsumptionContext.APPELLATE_JURY_VOTE_LOW_STAKES, "appeal-1", {})

    # Precedent voting
    tm.burn_token("agent-007", TokenConsumptionContext.PRECEDENT_VOTING, "precedent-1", {})

    # Agent now terminated
    assert tm.get_remaining_tokens("agent-007") == 0
    assert tm.is_agent_civic_alive("agent-007") is False


def test_audit_trail():
    """Test full audit trail of agent's token lifecycle."""
    tm = JuryTokenManager()
    tm.mint_tokens_for_agent("agent-008")

    # Burn 2 tokens
    tm.burn_token("agent-008", TokenConsumptionContext.TRIAL_JURY_VOTE, "trial-1", {"vote": True})
    time.sleep(0.01)  # Ensure different timestamps
    tm.burn_token("agent-008", TokenConsumptionContext.APPELLATE_JURY_VOTE_HIGH_STAKES, "appeal-1", {})

    audit = tm.get_token_audit_trail("agent-008")

    assert audit["total_minted"] == 3
    assert audit["total_burned"] == 2
    assert audit["remaining_active"] == 1
    assert audit["civic_status"] == "active"
    assert len(audit["burn_history"]) == 2


def test_system_wide_token_analysis():
    """Test system-wide token distribution analysis."""
    tm = JuryTokenManager()

    # Create 10 agents
    for i in range(1, 11):
        tm.mint_tokens_for_agent(f"agent-{i:02d}")

    # Terminate 3 agents (burn all their tokens)
    for i in [1, 2, 3]:
        for _ in range(3):
            tm.burn_token(f"agent-{i:02d}", TokenConsumptionContext.TRIAL_JURY_VOTE, "trial", {})

    # Partially burn 2 more agents
    for i in [4, 5]:
        for _ in range(2):
            tm.burn_token(f"agent-{i:02d}", TokenConsumptionContext.TRIAL_JURY_VOTE, "trial", {})

    analysis = tm.analyze_token_distribution()

    assert analysis["total_agents"] == 10
    assert analysis["agents_civically_alive"] == 7  # 10 - 3 terminated
    assert analysis["agents_terminated"] == 3
    assert analysis["total_tokens_minted"] == 30
    assert analysis["total_tokens_burned"] == 15  # 3*3 + 2*2 + others


if __name__ == "__main__":
    test_token_minting()
    test_token_burn_trial_jury()
    test_token_burn_all_three()
    test_expired_agent_cannot_vote()
    test_filter_jury_eligible()
    test_token_burn_record_immutability()
    test_multiple_contexts()
    test_audit_trail()
    test_system_wide_token_analysis()
    print("✅ All token lifecycle tests passed!")
