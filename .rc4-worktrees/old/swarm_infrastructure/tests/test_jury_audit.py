"""Tests for jury audit logging system"""
import json
import time
from swarm.jury.audit import AuditLogger, AuditEventType, AuditEvent


def test_create_log():
    """Test creating a new audit log."""
    logger = AuditLogger()
    log = logger.create_log("session-1")
    
    assert log.session_id == "session-1"
    assert len(log.events) == 1  # Creation event
    assert log.events[0].event_type == AuditEventType.SESSION_CREATED


def test_record_events():
    """Test recording multiple events."""
    logger = AuditLogger()
    log = logger.create_log("session-1")
    
    # Record some events
    logger.record_event(
        "session-1",
        AuditEventType.COMMIT_SUBMITTED,
        actor="juror-1",
        description="Submitted vote commitment",
        metadata={"commitment_hash": "abc123"}
    )
    
    logger.record_event(
        "session-1",
        AuditEventType.REVEAL_PHASE_ADVANCED,
        actor="system",
        description="Transitioned to reveal phase",
    )
    
    logger.record_event(
        "session-1",
        AuditEventType.VOTE_REVEALED,
        actor="juror-1",
        description="Revealed vote",
        metadata={"vote": True}
    )
    
    assert len(log.events) == 4  # Creation + 3 events


def test_compute_hash():
    """Test hash computation for integrity verification."""
    logger = AuditLogger()
    log = logger.create_log("session-1")
    
    # Record events
    logger.record_event("session-1", AuditEventType.COMMIT_SUBMITTED, actor="juror-1")
    logger.record_event("session-1", AuditEventType.COMMIT_SUBMITTED, actor="juror-2")
    
    # Compute hash
    hash1 = log.compute_hash()
    assert hash1 is not None
    assert len(hash1) == 64  # SHA256 hex digest
    
    # Hash should be deterministic
    hash2 = log.compute_hash()
    assert hash1 == hash2
    
    # Hash should change if events change
    logger.record_event("session-1", AuditEventType.COMMIT_SUBMITTED, actor="juror-3")
    hash3 = log.compute_hash()
    assert hash3 != hash1


def test_complete_session_and_anchor():
    """Test completing a session and anchoring on-chain."""
    logger = AuditLogger()
    log = logger.create_log("session-1")
    
    # Record voting events
    logger.record_event("session-1", AuditEventType.VOTE_REVEALED, actor="juror-1")
    logger.record_event("session-1", AuditEventType.VOTE_REVEALED, actor="juror-2")
    logger.record_event("session-1", AuditEventType.VOTES_AGGREGATED, actor="system")
    
    # Complete session
    content_hash = logger.complete_session("session-1")
    assert content_hash is not None
    assert content_hash in logger.pending_anchors.values()
    
    # Simulate on-chain anchoring
    tx_hash = "0xabcd1234"
    ok = logger.anchor_on_chain("session-1", tx_hash)
    assert ok is True
    assert log.on_chain_anchor == tx_hash
    assert content_hash not in logger.pending_anchors.values()


def test_audit_trail_retrieval():
    """Test retrieving audit log as JSON."""
    logger = AuditLogger()
    log = logger.create_log("session-1")
    
    logger.record_event("session-1", AuditEventType.COMMIT_SUBMITTED, actor="juror-1")
    logger.record_event("session-1", AuditEventType.COMMIT_SUBMITTED, actor="juror-2")
    logger.complete_session("session-1")
    
    # Retrieve audit trail
    trail_json = logger.get_audit_trail("session-1")
    assert trail_json is not None
    
    # Parse and verify
    trail = json.loads(trail_json)
    assert trail['session_id'] == "session-1"
    assert len(trail['events']) >= 3  # Creation + 2 commits + completion
    assert trail['content_hash'] is not None


def test_integrity_verification():
    """Test that log integrity can be verified."""
    logger = AuditLogger()
    log = logger.create_log("session-1")
    
    logger.record_event("session-1", AuditEventType.VOTE_REVEALED, actor="juror-1")
    log.compute_hash()
    
    # Integrity should pass
    assert logger.verify_log_integrity("session-1") is True
    
    # Tamper with events (simulate tampering)
    task_copy = log.events[0]
    log.events.insert(0, AuditEvent(
        event_type=AuditEventType.AUDIT_RETRIEVAL,
        session_id="session-1",
        actor="attacker",
        description="Forged event",
    ))
    
    # Integrity should fail
    assert logger.verify_log_integrity("session-1") is False


def test_log_statistics():
    """Test log statistics reporting."""
    logger = AuditLogger()
    log = logger.create_log("session-1")
    
    logger.record_event("session-1", AuditEventType.COMMIT_SUBMITTED, actor="juror-1")
    logger.record_event("session-1", AuditEventType.COMMIT_SUBMITTED, actor="juror-2")
    logger.record_event("session-1", AuditEventType.REVEAL_PHASE_ADVANCED, actor="system")
    logger.record_event("session-1", AuditEventType.VOTE_REVEALED, actor="juror-1")
    logger.complete_session("session-1")
    
    stats = logger.get_log_stats("session-1")
    assert stats is not None
    assert stats['session_id'] == "session-1"
    assert stats['event_count'] >= 5
    assert 'event_types' in stats
    assert stats['event_types']['commit_submitted'] == 2
    assert stats['event_types']['reveal_phase_advanced'] == 1
    assert stats['integrity_verified'] is True


def test_non_existent_session():
    """Test handling of non-existent sessions."""
    logger = AuditLogger()
    
    # Attempt to record event for non-existent session
    ok = logger.record_event("nonexistent", AuditEventType.COMMIT_SUBMITTED)
    assert ok is False
    
    # Attempt to retrieve non-existent session
    trail = logger.get_audit_trail("nonexistent")
    assert trail is None
    
    # Attempt to get stats for non-existent session
    stats = logger.get_log_stats("nonexistent")
    assert stats is None
    
    # Attempt to verify integrity of non-existent session
    ok = logger.verify_log_integrity("nonexistent")
    assert ok is False


if __name__ == "__main__":
    import pytest
    pytest.main([__file__, "-v"])
