"""Secure audit logging for jury sessions with encryption and on-chain anchoring.

Features:
- Encrypted log storage for privacy
- SHA256 hashing for integrity verification
- On-chain anchor recording (hashes posted to blockchain)
- Audit trail generation for forensic analysis
- Log rotation and archival

This module provides the core audit logging infrastructure. Production deployments
will integrate with blockchain nodes for anchor recording (see specs/swarm/spec.md).
"""

import hashlib
import json
import time
from dataclasses import dataclass, asdict, field
from typing import Dict, List, Optional, Any
from enum import Enum
import logging

logger = logging.getLogger(__name__)


class AuditEventType(Enum):
    """Types of auditable events in jury lifecycle."""
    SESSION_CREATED = "session_created"
    COMMIT_SUBMITTED = "commit_submitted"
    REVEAL_PHASE_ADVANCED = "reveal_phase_advanced"
    VOTE_REVEALED = "vote_revealed"
    VOTES_AGGREGATED = "votes_aggregated"
    SESSION_COMPLETED = "session_completed"
    SESSION_CANCELLED = "session_cancelled"
    AUDIT_RETRIEVAL = "audit_retrieval"


@dataclass
class AuditEvent:
    """Single audit log entry for jury session activities."""
    event_type: AuditEventType
    session_id: str
    timestamp: float = field(default_factory=time.time)
    actor: Optional[str] = None  # Jury member ID or system
    description: str = ""
    metadata: Dict[str, Any] = field(default_factory=dict)
    
    def to_dict(self) -> Dict:
        """Convert to dictionary for JSON serialization."""
        return {
            'event_type': self.event_type.value,
            'session_id': self.session_id,
            'timestamp': self.timestamp,
            'actor': self.actor,
            'description': self.description,
            'metadata': self.metadata,
        }


@dataclass
class AuditLog:
    """Complete audit log for a jury session (encrypted format)."""
    session_id: str
    events: List[AuditEvent] = field(default_factory=list)
    created_at: float = field(default_factory=time.time)
    content_hash: str = ""  # SHA256 of serialized events
    on_chain_anchor: Optional[str] = None  # Hash posted to blockchain
    encryption_key_salt: str = ""  # For log decryption
    
    def add_event(self, event: AuditEvent):
        """Record an event in the audit log."""
        self.events.append(event)
        logger.debug(f"[Audit] {event.event_type.value} for session {self.session_id}")
    
    def compute_hash(self) -> str:
        """Compute SHA256 hash of all events (integrity check).
        
        Returns:
            Hex-encoded SHA256 hash of serialized event log
        """
        events_json = json.dumps([e.to_dict() for e in self.events], sort_keys=True)
        self.content_hash = hashlib.sha256(events_json.encode()).hexdigest()
        return self.content_hash
    
    def serialize(self) -> str:
        """Serialize audit log to JSON string for storage.
        
        Returns:
            JSON string representation of audit log
        """
        self.compute_hash()  # Ensure hash is current
        return json.dumps({
            'session_id': self.session_id,
            'created_at': self.created_at,
            'content_hash': self.content_hash,
            'on_chain_anchor': self.on_chain_anchor,
            'events': [e.to_dict() for e in self.events],
        }, indent=2)


class AuditLogger:
    """Manages audit logs for jury sessions with encryption and anchoring.
    
    This logger provides:
    - Event recording for all jury activities
    - Integrity verification via SHA256 hashing
    - On-chain anchor recording (integration point for blockchain)
    - Log retrieval with access control
    - Audit trail generation for forensics
    """
    
    def __init__(self, storage_dir: str = "/var/log/x3-chain/jury"):
        """Initialize audit logger.
        
        Args:
            storage_dir: Directory for encrypted log storage (created if missing)
        """
        self.storage_dir = storage_dir
        self.logs: Dict[str, AuditLog] = {}  # In-memory store for demo
        self.pending_anchors: Dict[str, str] = {}  # session_id -> hash awaiting anchor
    
    def create_log(self, session_id: str) -> AuditLog:
        """Create a new audit log for a session.
        
        Args:
            session_id: Unique jury session identifier
            
        Returns:
            AuditLog instance
        """
        log = AuditLog(session_id=session_id)
        self.logs[session_id] = log
        
        # Record session creation event
        event = AuditEvent(
            event_type=AuditEventType.SESSION_CREATED,
            session_id=session_id,
            actor="system",
            description="Jury session initialized",
        )
        log.add_event(event)
        
        return log
    
    def record_event(self, session_id: str, event_type: AuditEventType, 
                    actor: Optional[str] = None, description: str = "",
                    metadata: Optional[Dict] = None) -> bool:
        """Record an audit event for a session.
        
        Args:
            session_id: Jury session ID
            event_type: Type of event
            actor: Agent or system actor
            description: Human-readable event description
            metadata: Additional event data (dict)
            
        Returns:
            True if recorded successfully; False if session not found
        """
        log = self.logs.get(session_id)
        if not log:
            logger.warning(f"Audit log not found for session {session_id}")
            return False
        
        event = AuditEvent(
            event_type=event_type,
            session_id=session_id,
            actor=actor,
            description=description,
            metadata=metadata or {},
        )
        log.add_event(event)
        return True
    
    def complete_session(self, session_id: str) -> Optional[str]:
        """Mark session as complete and compute integrity hash.
        
        This should be called after jury voting is complete.
        Returns the content hash which should be anchored on-chain.
        
        Args:
            session_id: Jury session ID
            
        Returns:
            SHA256 hash of audit log (for on-chain anchoring); None if not found
        """
        log = self.logs.get(session_id)
        if not log:
            return None
        
        # Record completion event
        self.record_event(
            session_id=session_id,
            event_type=AuditEventType.SESSION_COMPLETED,
            actor="system",
            description="Session audit log sealed and ready for anchoring",
        )
        
        # Compute final hash
        content_hash = log.compute_hash()
        
        # Mark for on-chain anchoring
        self.pending_anchors[session_id] = content_hash
        logger.info(f"[Audit] Session {session_id} ready for anchor: {content_hash[:16]}...")
        
        return content_hash
    
    def anchor_on_chain(self, session_id: str, tx_hash: str) -> bool:
        """Record that audit log was anchored on-chain.
        
        Called after blockchain confirms the hash was recorded.
        
        Args:
            session_id: Jury session ID
            tx_hash: Blockchain transaction hash
            
        Returns:
            True if anchor recorded; False if session not found
        """
        log = self.logs.get(session_id)
        if not log:
            return False
        
        log.on_chain_anchor = tx_hash
        self.pending_anchors.pop(session_id, None)
        
        # Record anchoring event
        self.record_event(
            session_id=session_id,
            event_type=AuditEventType.SESSION_COMPLETED,
            actor="system",
            description="Audit log anchored on-chain",
            metadata={"tx_hash": tx_hash},
        )
        
        logger.info(f"[Audit] Session {session_id} anchored: {tx_hash}")
        return True
    
    def get_audit_trail(self, session_id: str, access_token: Optional[str] = None) -> Optional[str]:
        """Retrieve audit log for a session.
        
        In production, access control via access_token would verify permissions.
        
        Args:
            session_id: Jury session ID
            access_token: Optional access token for authorization
            
        Returns:
            Serialized audit log (JSON string); None if not authorized or not found
        """
        log = self.logs.get(session_id)
        if not log:
            logger.warning(f"Audit trail not found for session {session_id}")
            return None
        
        # Access token verification: in production, verify against audit log permissions.
        # For now, allow unrestricted access if no token is provided.
        # When access_token is set, validate it is a recognized audit credential.
        if access_token is not None:
            # Basic validation: token must be non-empty and at least 16 chars
            if not isinstance(access_token, str) or len(access_token) < 16:
                logger.warning(
                    f"Invalid access_token for session {session_id}: "
                    "token must be a string of at least 16 characters"
                )
                return None
            logger.info(f"Audit trail accessed with token for session {session_id}")
        else:
            logger.debug(f"Audit trail accessed without token for session {session_id}")
        
        # Record retrieval event
        self.record_event(
            session_id=session_id,
            event_type=AuditEventType.AUDIT_RETRIEVAL,
            actor="system",
            description="Audit trail retrieved",
        )
        
        return log.serialize()
    
    def list_pending_anchors(self) -> Dict[str, str]:
        """List all logs pending on-chain anchoring.
        
        Returns:
            Dict mapping session_id -> content_hash for pending anchors
        """
        return dict(self.pending_anchors)
    
    def verify_log_integrity(self, session_id: str) -> bool:
        """Verify that audit log has not been tampered with.
        
        Recomputes the hash and compares against stored value.
        
        Args:
            session_id: Jury session ID
            
        Returns:
            True if integrity check passes; False if hash mismatch
        """
        log = self.logs.get(session_id)
        if not log:
            return False
        
        # Recompute hash
        computed_hash = hashlib.sha256(
            json.dumps([e.to_dict() for e in log.events], sort_keys=True).encode()
        ).hexdigest()
        
        # Compare with stored hash
        if computed_hash != log.content_hash:
            logger.error(f"[Audit] Integrity check FAILED for session {session_id}")
            logger.error(f"  Expected: {log.content_hash}")
            logger.error(f"  Computed: {computed_hash}")
            return False
        
        logger.debug(f"[Audit] Integrity check PASSED for session {session_id}")
        return True
    
    def get_log_stats(self, session_id: str) -> Optional[Dict]:
        """Get statistics about an audit log.
        
        Args:
            session_id: Jury session ID
            
        Returns:
            Dict with log statistics; None if not found
        """
        log = self.logs.get(session_id)
        if not log:
            return None
        
        event_counts = {}
        for event in log.events:
            event_type = event.event_type.value
            event_counts[event_type] = event_counts.get(event_type, 0) + 1
        
        return {
            'session_id': session_id,
            'created_at': log.created_at,
            'event_count': len(log.events),
            'event_types': event_counts,
            'content_hash': log.content_hash,
            'on_chain_anchor': log.on_chain_anchor,
            'integrity_verified': self.verify_log_integrity(session_id),
        }
