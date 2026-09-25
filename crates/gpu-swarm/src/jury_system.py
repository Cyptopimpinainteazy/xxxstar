"""
GPU Swarm Jury System
Encrypted audit logging, agent rotation, consensus verification
"""

import os
import json
import hashlib
import hmac
import time
import logging
from typing import List, Dict, Any, Optional, Tuple
from dataclasses import dataclass, asdict, field
from enum import Enum
from datetime import datetime, timedelta
from cryptography.fernet import Fernet
from cryptography.hazmat.primitives import hashes
from cryptography.hazmat.primitives.kdf.pbkdf2 import PBKDF2

logger = logging.getLogger(__name__)


class VerificationResult(Enum):
    """Task verification outcomes"""
    VERIFIED = "verified"
    FAILED = "failed"
    DISPUTED = "disputed"
    CORRUPTED = "corrupted"


@dataclass
class AuditLogEntry:
    """Encrypted audit log entry"""
    timestamp: float
    agent_id: str
    task_id: str
    action: str
    result: VerificationResult
    evidence_hash: str
    signature: str
    metadata: Dict[str, Any] = field(default_factory=dict)
    encrypted_details: Optional[str] = None


class EncryptedAuditLogger:
    """Manages encrypted audit logs for compliance and forensics"""

    def __init__(self, encryption_key: Optional[str] = None):
        """Initialize encrypted logger with key derivation"""
        if encryption_key is None:
            encryption_key = os.getenv(
                "AUDIT_ENCRYPTION_KEY",
                "default-insecure-key-change-in-production"
            )
        
        # Derive encryption key
        salt = b"gpu_swarm_audit_salt_v1"
        kdf = PBKDF2(
            algorithm=hashes.SHA256(),
            length=32,
            salt=salt,
            iterations=100000,
        )
        key_bytes = kdf.derive(encryption_key.encode())
        self.cipher = Fernet(Fernet.generate_key() if not key_bytes else
                            Fernet(base64.b64encode(key_bytes)))
        
        self.audit_logs: List[AuditLogEntry] = []
        self.hmac_key = hashlib.sha256(encryption_key.encode()).digest()

    def log_action(
        self,
        agent_id: str,
        task_id: str,
        action: str,
        result: VerificationResult,
        evidence_hash: str,
        metadata: Dict[str, Any] = None,
        sensitive_details: str = None
    ) -> AuditLogEntry:
        """Log auditable action with encryption"""
        # Create signature for integrity
        signature_data = f"{agent_id}:{task_id}:{action}:{result.value}:{evidence_hash}"
        signature = hmac.new(
            self.hmac_key,
            signature_data.encode(),
            hashlib.sha256
        ).hexdigest()

        # Encrypt sensitive details
        encrypted_details = None
        if sensitive_details:
            encrypted_details = self.cipher.encrypt(
                sensitive_details.encode()
            ).decode()

        entry = AuditLogEntry(
            timestamp=time.time(),
            agent_id=agent_id,
            task_id=task_id,
            action=action,
            result=result,
            evidence_hash=evidence_hash,
            signature=signature,
            metadata=metadata or {},
            encrypted_details=encrypted_details
        )

        self.audit_logs.append(entry)
        logger.info(f"Audit: Agent {agent_id} {action} Task {task_id} -> {result.value}")
        
        return entry

    def verify_log_integrity(self, entry: AuditLogEntry) -> bool:
        """Verify audit log entry integrity"""
        signature_data = f"{entry.agent_id}:{entry.task_id}:{entry.action}:{entry.result.value}:{entry.evidence_hash}"
        expected_signature = hmac.new(
            self.hmac_key,
            signature_data.encode(),
            hashlib.sha256
        ).hexdigest()

        return hmac.compare_digest(entry.signature, expected_signature)

    def decrypt_details(self, entry: AuditLogEntry) -> Optional[str]:
        """Decrypt sensitive details from audit entry"""
        if not entry.encrypted_details:
            return None
        
        try:
            return self.cipher.decrypt(entry.encrypted_details.encode()).decode()
        except Exception as e:
            logger.error(f"Failed to decrypt audit entry: {e}")
            return None

    def export_logs(self, start_time: float = None, end_time: float = None) -> List[Dict[str, Any]]:
        """Export audit logs for reporting"""
        logs = []
        for entry in self.audit_logs:
            if start_time and entry.timestamp < start_time:
                continue
            if end_time and entry.timestamp > end_time:
                continue
            
            log_dict = asdict(entry)
            log_dict["timestamp_iso"] = datetime.fromtimestamp(entry.timestamp).isoformat()
            log_dict["result"] = entry.result.value
            logs.append(log_dict)
        
        return logs


@dataclass
class JuryMember:
    """Represents a jury member (verification agent)"""
    agent_id: str
    last_active: float = field(default_factory=time.time)
    tasks_verified: int = 0
    accuracy: float = 1.0
    reputation_score: float = 100.0
    slashing_count: int = 0
    rotation_epoch: int = 0


class JuryCensusManager:
    """Manages jury member rotation and consensus"""

    def __init__(
        self,
        min_jury_size: int = 7,
        rotation_interval_epochs: int = 100,
        accuracy_threshold: float = 0.95
    ):
        self.min_jury_size = min_jury_size
        self.rotation_interval = rotation_interval_epochs
        self.accuracy_threshold = accuracy_threshold
        self.jury_pool: Dict[str, JuryMember] = {}
        self.consensus_threshold = (min_jury_size // 2) + 1  # >50% consensus
        self.epoch = 0
        self.lock = __import__('threading').Lock()

    def register_jury_member(self, agent_id: str) -> JuryMember:
        """Register agent as potential jury member"""
        with self.lock:
            if agent_id not in self.jury_pool:
                member = JuryMember(agent_id=agent_id, rotation_epoch=self.epoch)
                self.jury_pool[agent_id] = member
                logger.info(f"Registered jury member: {agent_id}")
                return member
            return self.jury_pool[agent_id]

    def select_jury(self, task_id: str) -> List[JuryMember]:
        """Select jury for verification task based on reputation"""
        with self.lock:
            # Filter active members with good accuracy
            candidates = [
                m for m in self.jury_pool.values()
                if m.accuracy >= self.accuracy_threshold and
                   time.time() - m.last_active < 3600  # Active in last hour
            ]

            if len(candidates) < self.min_jury_size:
                logger.warning(f"Insufficient qualified jury members for task {task_id}")
                return []

            # Sort by reputation score (higher is better)
            candidates.sort(key=lambda m: m.reputation_score, reverse=True)
            
            # Select top members
            selected = candidates[:self.min_jury_size]
            
            logger.debug(f"Selected jury for {task_id}: {[m.agent_id for m in selected]}")
            return selected

    def record_verification(
        self,
        jury_member_id: str,
        task_id: str,
        correct: bool
    ):
        """Record jury member verification and update reputation"""
        with self.lock:
            if jury_member_id not in self.jury_pool:
                logger.warning(f"Jury member {jury_member_id} not found")
                return
            
            member = self.jury_pool[jury_member_id]
            member.last_active = time.time()
            member.tasks_verified += 1
            
            # Update accuracy (exponential moving average)
            alpha = 0.1
            member.accuracy = alpha * (1.0 if correct else 0.0) + (1 - alpha) * member.accuracy
            
            # Update reputation based on accuracy
            member.reputation_score = 100 * member.accuracy

    def slash_jury_member(
        self,
        jury_member_id: str,
        reason: str,
        tokens_slashed: float
    ):
        """Penalize jury member for misbehavior"""
        with self.lock:
            if jury_member_id not in self.jury_pool:
                return
            
            member = self.jury_pool[jury_member_id]
            member.slashing_count += 1
            member.reputation_score = max(0, member.reputation_score - (tokens_slashed / 1000))
            
            logger.warning(f"Slashed jury member {jury_member_id}: {reason} ({tokens_slashed} tokens)")

    def rotate_epoch(self):
        """Advance rotation epoch and refresh jury pool"""
        with self.lock:
            self.epoch += 1
            
            # Remove inactive or low-reputation members
            to_remove = [
                agent_id for agent_id, member in self.jury_pool.items()
                if member.reputation_score < 20 or  # Too low reputation
                   time.time() - member.last_active > 86400 * 7  # Inactive >7 days
            ]
            
            for agent_id in to_remove:
                logger.info(f"Removing jury member from pool: {agent_id}")
                del self.jury_pool[agent_id]
            
            logger.info(f"Epoch {self.epoch}: {'Removed' if to_remove else 'No removals'} in jury rotation")

    def get_jury_stats(self) -> Dict[str, Any]:
        """Get jury statistics"""
        with self.lock:
            return {
                "total_members": len(self.jury_pool),
                "active_members": sum(
                    1 for m in self.jury_pool.values()
                    if time.time() - m.last_active < 3600
                ),
                "avg_accuracy": sum(m.accuracy for m in self.jury_pool.values()) / len(self.jury_pool) if self.jury_pool else 0,
                "avg_reputation": sum(m.reputation_score for m in self.jury_pool.values()) / len(self.jury_pool) if self.jury_pool else 0,
                "current_epoch": self.epoch,
            }


class VerificationConsensus:
    """Determine consensus from jury verifications"""

    def __init__(self, consensus_threshold: int = 4):
        self.consensus_threshold = consensus_threshold
        self.verifications: Dict[str, List[Tuple[str, bool]]] = {}

    def add_verification(self, task_id: str, agent_id: str, result: bool):
        """Add jury member verification"""
        if task_id not in self.verifications:
            self.verifications[task_id] = []
        
        self.verifications[task_id].append((agent_id, result))

    def get_consensus(self, task_id: str) -> Tuple[bool, float]:
        """
        Determine consensus result.
        Returns: (consensus_result, confidence_score)
        """
        if task_id not in self.verifications or not self.verifications[task_id]:
            return None, 0.0
        
        verifications = self.verifications[task_id]
        approved = sum(1 for _, result in verifications if result)
        total = len(verifications)
        
        # Check if consensus reached
        if approved >= self.consensus_threshold:
            consensus = True
        elif (total - approved) >= self.consensus_threshold:
            consensus = False
        else:
            # No consensus yet
            return None, approved / total

        confidence = max(approved, total - approved) / total
        return consensus, confidence

    def is_disputed(self, task_id: str) -> bool:
        """Check if verification is disputed (no consensus)"""
        consensus, _ = self.get_consensus(task_id)
        return consensus is None


class JurySystem:
    """Main Jury System coordinating all components"""

    def __init__(self):
        self.audit_logger = EncryptedAuditLogger()
        self.jury_manager = JuryCensusManager()
        self.consensus_engine = VerificationConsensus()

    def initiate_verification(self, task_id: str) -> List[JuryMember]:
        """Start verification process for task"""
        jury = self.jury_manager.select_jury(task_id)
        
        if not jury:
            logger.error(f"Could not assemble jury for task {task_id}")
            return []
        
        logger.info(f"Initiating verification for task {task_id} with {len(jury)} jury members")
        return jury

    def submit_verification(
        self,
        task_id: str,
        agent_id: str,
        result: bool,
        evidence_hash: str,
        evidence_details: str = None
    ):
        """Submit jury member verification"""
        # Log to audit trail
        self.audit_logger.log_action(
            agent_id=agent_id,
            task_id=task_id,
            action="verify_task",
            result=VerificationResult.VERIFIED if result else VerificationResult.FAILED,
            evidence_hash=evidence_hash,
            sensitive_details=evidence_details
        )

        # Add to consensus engine
        self.consensus_engine.add_verification(task_id, agent_id, result)

        # Update jury member stats
        self.jury_manager.record_verification(agent_id, task_id, result)

    def finalize_verification(self, task_id: str) -> Tuple[bool, float]:
        """Finalize verification and return consensus"""
        consensus, confidence = self.consensus_engine.get_consensus(task_id)
        
        if consensus is None:
            logger.warning(f"No consensus reached for task {task_id}")
            return False, confidence
        
        logger.info(f"Task {task_id} verification: {'APPROVED' if consensus else 'REJECTED'} (confidence: {confidence:.1%})")
        return consensus, confidence

    def handle_dispute(self, task_id: str):
        """Handle disputed verification"""
        logger.warning(f"Dispute initiated for task {task_id}")
        self.audit_logger.log_action(
            agent_id="system",
            task_id=task_id,
            action="dispute_initiated",
            result=VerificationResult.DISPUTED,
            evidence_hash="",
        )

    def get_audit_report(
        self,
        agent_id: str = None,
        start_time: float = None,
        end_time: float = None
    ) -> Dict[str, Any]:
        """Generate audit report"""
        logs = self.audit_logger.export_logs(start_time, end_time)
        
        if agent_id:
            logs = [l for l in logs if l["agent_id"] == agent_id]
        
        return {
            "total_actions": len(logs),
            "timestamp": datetime.utcnow().isoformat(),
            "logs": logs,
            "jury_stats": self.jury_manager.get_jury_stats(),
        }

    def rotate_jury(self):
        """Trigger jury rotation epoch"""
        self.jury_manager.rotate_epoch()


# Singleton instance
_jury_system = None

def get_jury_system() -> JurySystem:
    """Get or create jury system"""
    global _jury_system
    if _jury_system is None:
        _jury_system = JurySystem()
    return _jury_system


__all__ = [
    'JurySystem',
    'EncryptedAuditLogger',
    'JuryCensusManager',
    'VerificationConsensus',
    'AuditLogEntry',
    'JuryMember',
    'VerificationResult',
    'get_jury_system',
]
