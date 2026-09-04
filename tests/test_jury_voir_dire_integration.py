"""
Integration test: Complete voir dire jury creation workflow.

Demonstrates the constitutional model:
1. Anonymize candidates
2. Dual-counsel voir dire with symmetric strikes
3. Randomized empanelment
4. Jury creation from selected pool
5. Voting proceeds with full audit trail
"""

import hashlib

from swarm.jury import JuryManager
from swarm.jury.manager import JuryMember


def test_full_voir_dire_to_jury_workflow():
    """Test complete workflow: anonymization → voir dire → empanelment → voting."""
    jm = JuryManager()

    # Candidate pool
    case_id = "case-2026-001"
    candidate_ids = [f"agent-{i}" for i in range(1, 13)]  # 12 candidates

    candidate_data = {
        candidate_ids[i]: {
            "reputation": 0.6 + (i % 10) * 0.03,  # Varied reputations
            "section": ["governance", "economic", "security"][i % 3],
            "recent_jury_count": i % 2,  # Some have recent jury service
            "domain_expertise": ["constitutional", "tokenomics", "security"][(i // 3) % 3],
        }
        for i in range(len(candidate_ids))
    }

    # Create jury via voir dire with dual counsel
    success, session, audit = jm.create_session_via_voir_dire(
        case_id=case_id,
        task_ids=["deliberate-on-amendment-x"],
        candidate_ids=candidate_ids,
        candidate_data=candidate_data,
        prosecution_counsel_id="counsel-da-alice",
        defense_counsel_id="counsel-defense-bob",
        jury_size=6,
    )

    assert success is True
    assert session is not None
    assert len(session.members) == 6
    assert session.metadata["prosecution_counsel"] == "counsel-da-alice"
    assert session.metadata["defense_counsel"] == "counsel-defense-bob"

    # Verify voir dire audit trail is recorded
    voir_dire_audit = session.metadata["voir_dire_audit"]
    assert voir_dire_audit["total_candidates"] == 12
    assert voir_dire_audit["final_jury_size"] == 6
    assert voir_dire_audit["prosecution_strikes"] > 0  # Both sides struck candidates
    assert voir_dire_audit["defense_strikes"] > 0

    # Verify jury composition constraints
    section_counts = {}
    for member in session.members:
        section_counts[member.section] = section_counts.get(member.section, 0) + 1

    # Check diversity: no section > 75%
    for section, count in section_counts.items():
        ratio = count / len(session.members)
        assert ratio <= 0.75, f"Section {section} has {ratio:.0%} > 75% max"

    # Now verify voting works with this jury
    for i, member in enumerate(session.members):
        vote = i % 2 == 0  # Some yes, some no
        nonce = f"nonce-{i}"
        commitment = hashlib.sha256((str(int(vote)) + "|" + nonce).encode()).hexdigest()
        assert jm.submit_commit(session.session_id, member.agent_id, commitment) is True

    assert jm.advance_to_reveal(session.session_id) is True

    for i, member in enumerate(session.members):
        vote = i % 2 == 0
        nonce = f"nonce-{i}"
        assert jm.submit_reveal(session.session_id, member.agent_id, vote, nonce) is True

    res = jm.aggregate(session.session_id)
    assert res is not None
    assert res["total"] == 6
    # Should have 3 yes, 3 no (alternating)
    assert res["yes"] == 3 and res["no"] == 3

    print("✅ Full voir dire workflow successful:")
    print(f"   - {audit['seated_count']} jurors seated after voir dire")
    print(f"   - {audit['prosecution_strikes']} prosecution strikes")
    print(f"   - {audit['defense_strikes']} defense strikes")
    print(f"   - Vote result: {res['yes']} yes, {res['no']} no")


def test_voir_dire_insufficient_candidates():
    """Test that voir dire fails gracefully if not enough candidates remain."""
    jm = JuryManager()

    # Only 3 candidates, but both lawyers will strike all of them
    case_id = "case-2026-002"
    candidate_ids = [f"agent-{i}" for i in range(1, 4)]  # Only 3 candidates

    candidate_data = {
        cid: {
            "reputation": 0.7,
            "section": "general",
            "recent_jury_count": 0,
        }
        for cid in candidate_ids
    }

    success, session, audit = jm.create_session_via_voir_dire(
        case_id=case_id,
        task_ids=["test-task"],
        candidate_ids=candidate_ids,
        candidate_data=candidate_data,
        prosecution_counsel_id="counsel-da",
        defense_counsel_id="counsel-defense",
        jury_size=3,  # Need all 3, but they'll likely be struck
    )

    # With proper random strike patterns, this might succeed or fail
    # What matters is the system handles both gracefully
    if not success:
        assert "error" in audit or audit.get("error") == "Insufficient remaining candidates after voir dire"
    else:
        assert session is not None
        assert len(session.members) >= 3


def test_backward_compatibility_simple_jury():
    """Test that old-style simple jury creation still works (backward compat)."""
    jm = JuryManager()

    # Create a simple 3-person jury (no voir dire)
    members = [
        JuryMember(agent_id='juror-old-1', section='governance', is_on_chain=False),
        JuryMember(agent_id='juror-old-2', section='economic', is_on_chain=False),
        JuryMember(agent_id='juror-old-3', section='security', is_on_chain=False),
    ]

    session = jm.create_session(task_ids=["legacy-task"], members=members)

    assert session is not None
    assert len(session.members) == 3
    assert session.metadata.get("voir_dire_id") is None  # Simple method doesn't use voir dire


if __name__ == "__main__":
    test_full_voir_dire_to_jury_workflow()
    test_voir_dire_insufficient_candidates()
    test_backward_compatibility_simple_jury()
    print("\n✅ All integration tests passed!")
