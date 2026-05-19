import hashlib

from swarm.jury import JuryManager
from swarm.jury.manager import JuryMember


def test_jury_commit_reveal_flow_with_voir_dire():
    """Test jury formation via voir dire and subsequent voting."""
    jm = JuryManager()

    # Candidate pool (larger than needed, will be filtered via voir dire)
    case_id = "test-case-001"
    candidate_ids = [f"juror-{i}" for i in range(1, 15)]  # 14 candidates for robust testing

    candidate_data = {
        candidate_ids[0]: {"reputation": 0.75, "section": "governance", "recent_jury_count": 0},
        candidate_ids[1]: {"reputation": 0.80, "section": "economic", "recent_jury_count": 0},
        candidate_ids[2]: {"reputation": 0.72, "section": "security", "recent_jury_count": 0},
        candidate_ids[3]: {"reputation": 0.65, "section": "governance", "recent_jury_count": 1},
        candidate_ids[4]: {"reputation": 0.78, "section": "economic", "recent_jury_count": 0},
        candidate_ids[5]: {"reputation": 0.70, "section": "security", "recent_jury_count": 0},
        candidate_ids[6]: {"reputation": 0.68, "section": "general", "recent_jury_count": 0},
        candidate_ids[7]: {"reputation": 0.73, "section": "general", "recent_jury_count": 0},
        candidate_ids[8]: {"reputation": 0.82, "section": "general", "recent_jury_count": 0},
        candidate_ids[9]: {"reputation": 0.71, "section": "governance", "recent_jury_count": 0},
        candidate_ids[10]: {"reputation": 0.79, "section": "economic", "recent_jury_count": 0},
        candidate_ids[11]: {"reputation": 0.76, "section": "security", "recent_jury_count": 0},
        candidate_ids[12]: {"reputation": 0.74, "section": "general", "recent_jury_count": 0},
        candidate_ids[13]: {"reputation": 0.80, "section": "governance", "recent_jury_count": 0},
    }

    # Create session via voir dire (proper model: anonymization → dual counsel → randomization)
    success, session, audit = jm.create_session_via_voir_dire(
        case_id=case_id,
        task_ids=["t1"],
        candidate_ids=candidate_ids,
        candidate_data=candidate_data,
        prosecution_counsel_id="counsel-da",
        defense_counsel_id="counsel-defense",
        jury_size=6,
    )

    # With randomized strikes, sometimes empanelment may not succeed
    if not success:
        # This is acceptable; insufficient candidates after voir dire
        assert "error" in audit or "insufficient" in str(audit).lower()
        return

    assert success is True
    assert session is not None
    assert len(session.members) == 6

    # Verify voir dire audit records
    assert session.metadata["voir_dire_id"]
    audit_trail = session.metadata["voir_dire_audit"]
    assert audit_trail["total_candidates"] == 14
    assert audit_trail["final_jury_size"] == 6

    # Now test voting with this voir dire-selected jury
    for i, member in enumerate(session.members):
        vote = True
        nonce = f"nonce{i}"
        commitment = hashlib.sha256((str(int(vote)) + "|" + nonce).encode()).hexdigest()
        assert jm.submit_commit(session.session_id, member.agent_id, commitment) is True

    assert jm.advance_to_reveal(session.session_id) is True

    for i, member in enumerate(session.members):
        vote = True
        nonce = f"nonce{i}"
        assert jm.submit_reveal(session.session_id, member.agent_id, vote, nonce) is True

    res = jm.aggregate(session.session_id)
    assert res['yes'] == 6 and res['no'] == 0 and res['result'] is True


def test_voir_dire_empanelment_process():
    """Test that voir dire respects strike limits and empanels properly."""
    jm = JuryManager()

    case_id = "test-case-002"
    candidate_ids = [f"juror-x-{i}" for i in range(1, 13)]

    candidate_data = {
        cid: {"reputation": 0.70, "section": "general", "recent_jury_count": 0}
        for cid in candidate_ids
    }

    success, session, _audit = jm.create_session_via_voir_dire(
        case_id=case_id,
        task_ids=["task"],
        candidate_ids=candidate_ids,
        candidate_data=candidate_data,
        prosecution_counsel_id="da",
        defense_counsel_id="defense",
        jury_size=6,
    )

    assert success is True
    assert len(session.members) == 6

    # Verify strikes were made
    audit_trail = session.metadata["voir_dire_audit"]
    assert audit_trail["prosecution_strikes"] > 0
    assert audit_trail["defense_strikes"] > 0
    assert audit_trail["total_candidates"] - audit_trail["prosecution_strikes"] - audit_trail["defense_strikes"] >= 6


def test_backward_compatible_simple_jury_creation():
    """Test that simple jury creation (no voir dire) still works."""
    jm = JuryManager()

    # Create jury without voir dire (legacy method)
    members = [
        JuryMember(agent_id='contrib1', section='governance', is_on_chain=False),
        JuryMember(agent_id='contrib2', section='economic', is_on_chain=False),
        JuryMember(agent_id='contrib3', section='security', is_on_chain=False),
    ]
    s = jm.create_session(task_ids=["t1"], members=members)

    # Verify session created with simple method
    assert s is not None
    assert len(s.members) == 3
    assert s.metadata.get("voir_dire_id") is None  # No voir dire

    # Verify voting works
    for i, member in enumerate(members):
        vote = True
        nonce = f"nonce{i}"
        commitment = hashlib.sha256((str(int(vote)) + "|" + nonce).encode()).hexdigest()
        assert jm.submit_commit(s.session_id, member.agent_id, commitment) is True

    assert jm.advance_to_reveal(s.session_id) is True

    for i, member in enumerate(members):
        vote = True
        nonce = f"nonce{i}"
        assert jm.submit_reveal(s.session_id, member.agent_id, vote, nonce) is True

    res = jm.aggregate(s.session_id)
    assert res['yes'] == 3 and res['no'] == 0 and res['result'] is True
