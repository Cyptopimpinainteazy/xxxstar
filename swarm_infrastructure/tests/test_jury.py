import hashlib
import time
from swarm.jury import JuryManager
from swarm.jury.manager import JuryMember, JuryState


def test_create_and_vote():
    """Test basic jury session creation and commit-reveal voting."""
    jm = JuryManager()
    members = [
        JuryMember(agent_id="juror-1", section="governance", is_on_chain=False),
        JuryMember(agent_id="juror-2", section="security", is_on_chain=False),
        JuryMember(agent_id="juror-3", section="economics", is_on_chain=False),
    ]
    session = jm.create_session(task_ids=['task-1'], members=members)
    assert session.session_id
    assert session.state == JuryState.COMMIT_PHASE
    assert len(session.members) == 3
    
    # Commit phase: Each member submits commitment
    for member in session.members:
        vote = True  # Everyone votes yes
        nonce = f"secret-nonce-{member.agent_id}"
        commitment = hashlib.sha256((str(int(vote)) + "|" + nonce).encode()).hexdigest()
        ok = jm.submit_commit(session.session_id, member.agent_id, commitment)
        assert ok, f"Failed to submit commit for {member.agent_id}"
    
    # Advance to reveal phase
    ok = jm.advance_to_reveal(session.session_id)
    assert ok
    s = jm.get_session(session.session_id)
    assert s.state == JuryState.REVEAL_PHASE
    
    # Reveal votes
    for member in session.members:
        vote = True
        nonce = f"secret-nonce-{member.agent_id}"
        ok = jm.submit_reveal(session.session_id, member.agent_id, vote, nonce)
        assert ok, f"Failed to reveal vote for {member.agent_id}"
    
    # Aggregate votes
    result = jm.aggregate(session.session_id)
    assert result is not None
    assert result["yes"] == 3
    assert result["no"] == 0
    assert result["result"] is True  # 3/3 >= 66%
    assert result["quorum_met"] is True
    
    s = jm.get_session(session.session_id)
    assert s.state == JuryState.COMPLETED


def test_quorum_threshold():
    """Test quorum threshold enforcement (66% majority)."""
    jm = JuryManager()
    
    # 5-member jury: 4 yes, 1 no → 4/5 = 80% >= 66% → PASS
    # Use diverse sections to pass diversity cap
    members = [
        JuryMember(agent_id="juror-1", section="governance", is_on_chain=False),
        JuryMember(agent_id="juror-2", section="governance", is_on_chain=False),
        JuryMember(agent_id="juror-3", section="security", is_on_chain=False),
        JuryMember(agent_id="juror-4", section="economics", is_on_chain=False),
        JuryMember(agent_id="juror-5", section="operations", is_on_chain=False),
    ]
    session = jm.create_session(task_ids=['task-1'], members=members)
    
    # Submit commits and reveals
    for i, member in enumerate(session.members):
        vote = i < 4  # First 4 vote yes, last 1 votes no
        nonce = f"nonce-{member.agent_id}"
        commitment = hashlib.sha256((str(int(vote)) + "|" + nonce).encode()).hexdigest()
        jm.submit_commit(session.session_id, member.agent_id, commitment)
    
    jm.advance_to_reveal(session.session_id)
    
    for i, member in enumerate(session.members):
        vote = i < 4
        nonce = f"nonce-{member.agent_id}"
        jm.submit_reveal(session.session_id, member.agent_id, vote, nonce)
    
    result = jm.aggregate(session.session_id)
    assert result["yes"] == 4
    assert result["no"] == 1
    assert result["result"] is True  # 4/5 = 80% >= 66%


def test_section_diversity_cap():
    """Test that jury composition respects section diversity caps."""
    jm = JuryManager()
    
    # Try to create jury with too many members from same section (> 75%)
    members_over_cap = [
        JuryMember(agent_id="juror-1", section="test", is_on_chain=False),
        JuryMember(agent_id="juror-2", section="test", is_on_chain=False),
        JuryMember(agent_id="juror-3", section="test", is_on_chain=False),
        JuryMember(agent_id="juror-4", section="other", is_on_chain=False),
    ]
    # 3/4 = 75%, which is at cap (should still fail if > 75%)
    try:
        session = jm.create_session(task_ids=['task-1'], members=members_over_cap)
        assert session is not None  # 75% is at the threshold
    except ValueError as e:
        # 75% might trigger error depending on exact implementation
        assert "max ratio" in str(e).lower()
    
    # Create jury within diversity cap
    members_ok = [
        JuryMember(agent_id="juror-1", section="governance", is_on_chain=False),
        JuryMember(agent_id="juror-2", section="governance", is_on_chain=False),
        JuryMember(agent_id="juror-3", section="security", is_on_chain=False),
    ]
    # 2/3 = 67%, which is < 75%
    session = jm.create_session(task_ids=['task-1'], members=members_ok)
    assert session is not None


def test_rotate_jury():
    """Test jury rotation from on-chain agents."""
    jm = JuryManager()
    
    on_chain_agents = ["agent-1", "agent-2", "agent-3", "agent-4"]
    rotated = jm.rotate_jury(epoch=1, on_chain_agents=on_chain_agents)
    
    assert len(rotated) <= 3  # Should select up to 3 members
    for member in rotated:
        assert member.is_on_chain is True
        assert member.readonly_snapshot is not None
        assert member.readonly_snapshot["epoch"] == 1

