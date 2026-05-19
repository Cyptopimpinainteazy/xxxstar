"""Tests for Appeals v0 with token integration."""

from swarm.jury.appeals import AppealGround, AppealsManager, AppealState
from swarm.jury.tokens import TokenManager, TokenSpendReason


def test_appeal_filing_requires_civic_status():
    """Test that only civically alive agents can file appeals."""
    tm = TokenManager()
    am = AppealsManager(tm)

    # Agent with tokens
    tm.mint_agent("agent-alive")
    success, appeal, msg = am.file_appeal(
        filer_id="agent-alive",
        original_case_id="case-001",
        grounds=[AppealGround.SCORE_VIOLATION],
        brief="Score was violated in jury decision"
    )
    assert success is True
    assert appeal is not None

    # Agent without tokens (civically dead)
    tm.mint_agent("agent-dead")
    for i in range(3):
        tm.spend_token("agent-dead", f"case-{i}", TokenSpendReason.TRIAL_JURY_VOTE)

    success, appeal, msg = am.file_appeal(
        filer_id="agent-dead",
        original_case_id="case-001",
        grounds=[AppealGround.SCORE_VIOLATION],
        brief="This should fail"
    )
    assert success is False
    assert "CIVIC_DEATH" in msg


def test_appeal_state_transitions():
    """Test appeal lifecycle: FILED → ACCEPTED → HEARD → DECIDED → RECORDED."""
    tm = TokenManager()
    am = AppealsManager(tm)
    tm.mint_agent("filer")

    # File appeal
    _success, appeal, _ = am.file_appeal(
        filer_id="filer",
        original_case_id="case-001",
        grounds=[AppealGround.PROCEDURAL_VIOLATION],
        brief="Jury selection was biased"
    )
    assert appeal.state == AppealState.FILED

    # Accept appeal
    am.accept_appeal(appeal.appeal_id)
    appeal_obj = am.appeals[appeal.appeal_id]
    assert appeal_obj.state == AppealState.ACCEPTED


def test_appellate_jury_empanelment_filters_dead_agents():
    """Test that civically dead agents are excluded from appellate juries."""
    tm = TokenManager()
    am = AppealsManager(tm)

    # Create candidate pool
    candidates = [f"juror-{i}" for i in range(1, 11)]
    for c in candidates:
        tm.mint_agent(c)

    # Kill jurors 1-3
    for i in range(1, 4):
        agent = f"juror-{i}"
        for j in range(3):
            tm.spend_token(agent, f"case-{j}", TokenSpendReason.TRIAL_JURY_VOTE)

    # File and accept appeal
    tm.mint_agent("filer")
    success, appeal, _ = am.file_appeal(
        filer_id="filer",
        original_case_id="case-001",
        grounds=[AppealGround.SCORE_VIOLATION],
        brief="Test appeal"
    )
    am.accept_appeal(appeal.appeal_id)

    # Try to empanel jury
    success, seated = am.empanel_appellate_jury(
        appeal_id=appeal.appeal_id,
        candidate_ids=candidates,
        prosecution_counsel_id="counsel-da",
        defense_counsel_id="counsel-defense",
        jury_size=5
    )

    assert success is True
    assert len(seated) == 5
    assert all(juror not in ["juror-1", "juror-2", "juror-3"] for juror in seated)


def test_appellate_jury_empanelment_fails_if_insufficient_tokens():
    """Test that jury empanelment fails if not enough token-eligible candidates."""
    tm = TokenManager()
    am = AppealsManager(tm)

    # Create only 3 candidates, all alive
    for i in range(3):
        tm.mint_agent(f"juror-{i}")

    # File and accept appeal
    tm.mint_agent("filer")
    success, appeal, _ = am.file_appeal(
        filer_id="filer",
        original_case_id="case-001",
        grounds=[AppealGround.PROCEDURAL_VIOLATION],
        brief="Test"
    )
    am.accept_appeal(appeal.appeal_id)

    # Try to empanel 5-member jury (only 3 alive)
    success, seated = am.empanel_appellate_jury(
        appeal_id=appeal.appeal_id,
        candidate_ids=[f"juror-{i}" for i in range(3)],
        prosecution_counsel_id="counsel-da",
        defense_counsel_id="counsel-defense",
        jury_size=5
    )

    assert success is False
    assert len(seated) == 0


def test_jury_decision_consumes_tokens():
    """Test that jury members consume tokens when voting."""
    tm = TokenManager()
    am = AppealsManager(tm)

    # Create jury
    jurors = ["juror-a", "juror-b", "juror-c"]
    for j in jurors:
        tm.mint_agent(j)

    # File appeal
    tm.mint_agent("filer")
    _success, appeal, _ = am.file_appeal(
        filer_id="filer",
        original_case_id="case-001",
        grounds=[AppealGround.SCORE_VIOLATION],
        brief="Appeal"
    )

    # Accept appeal (FILED → ACCEPTED)
    am.accept_appeal(appeal.appeal_id)

    # Record jury decision
    votes = dict.fromkeys(jurors, True)  # All vote to affirm
    am.record_jury_decision(
        appeal_id=appeal.appeal_id,
        jury_session_id="jury-001",
        affirm=True,
        votes=votes,
        reasoning="Original decision was sound"
    )

    # Verify tokens were consumed
    for juror in jurors:
        assert tm.get_ledger(juror).active_tokens() == 2


def test_reversal_creates_precedent():
    """Test that reversals create precedent records."""
    tm = TokenManager()
    am = AppealsManager(tm)

    # Create jury
    jurors = ["juror-x", "juror-y", "juror-z"]
    for j in jurors:
        tm.mint_agent(j)

    # File appeal
    tm.mint_agent("filer")
    _success, appeal, _ = am.file_appeal(
        filer_id="filer",
        original_case_id="case-001",
        grounds=[AppealGround.SCORE_VIOLATION],
        brief="Original decision violated constitutional rule"
    )

    # Accept appeal (FILED → ACCEPTED)
    am.accept_appeal(appeal.appeal_id)

    # Record reversal
    votes = dict.fromkeys(jurors, False)  # All vote to reverse
    am.record_jury_decision(
        appeal_id=appeal.appeal_id,
        jury_session_id="jury-001",
        affirm=False,  # REVERSAL
        votes=votes,
        reasoning="Original violated Score clause"
    )

    # Verify precedent created
    appeal_obj = am.appeals[appeal.appeal_id]
    assert appeal_obj.creates_precedent is True
    assert appeal_obj.precedent_id is not None


def test_appeal_audit_trail():
    """Test that all appeals are logged to audit trail."""
    tm = TokenManager()
    am = AppealsManager(tm)

    # File multiple appeals
    for i in range(3):
        tm.mint_agent(f"filer-{i}")
        am.file_appeal(
            filer_id=f"filer-{i}",
            original_case_id=f"case-{i}",
            grounds=[AppealGround.PROCEDURAL_VIOLATION],
            brief=f"Appeal {i}"
        )

    # Kill one agent and try to file
    agent = "filer-dead"
    tm.mint_agent(agent)
    for j in range(3):
        tm.spend_token(agent, f"case-{j}", TokenSpendReason.TRIAL_JURY_VOTE)

    am.file_appeal(
        filer_id=agent,
        original_case_id="case-dead",
        grounds=[AppealGround.SCORE_VIOLATION],
        brief="This should be rejected"
    )

    # Check audit log
    audit = am.get_appeal_audit_trail()
    filed_count = sum(1 for entry in audit if entry["type"] == "APPEAL_FILED")
    rejected_count = sum(1 for entry in audit if entry["type"] == "APPEAL_REJECTED_CIVIC_DEATH")

    assert filed_count == 3
    assert rejected_count == 1
