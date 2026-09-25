"""
X3 Operator Slashing Engine
~~~~~~~~~~~~~~~~~~~~~~~~~~~~

Deterministic slashing with severity table, repetition multiplier,
confidence factor, and max-slash cap.

Slashing formula:
    slash_amount = base_severity * repetition_multiplier * confidence * bond_amount
    capped at max_slash_fraction * bond_amount
"""

import hashlib
import logging
import time
from dataclasses import dataclass, field
from enum import Enum
from typing import Optional

from .config import X3Config

logger = logging.getLogger(__name__)


class FaultType(str, Enum):
    DOWNTIME = "downtime"
    EQUIVOCATION = "equivocation"
    INVALID_PROOF = "invalid_proof"
    MISSED_HEARTBEAT = "missed_heartbeat"
    SLA_VIOLATION = "sla_violation"
    DATA_CORRUPTION = "data_corruption"
    GOVERNANCE_ABUSE = "governance_abuse"
    AGENT_VIOLATION = "agent_violation"


@dataclass
class SlashEvidence:
    """Immutable evidence record for a slashing event."""
    fault_type: FaultType
    operator_id: str
    block_number: int
    timestamp: float
    description: str
    reporter_id: str
    evidence_hash: str = ""

    def __post_init__(self):
        if not self.evidence_hash:
            payload = (
                f"{self.fault_type.value}:{self.operator_id}:"
                f"{self.block_number}:{self.timestamp}:{self.description}"
            )
            self.evidence_hash = hashlib.sha256(payload.encode()).hexdigest()


@dataclass
class SlashVerdict:
    """Result of evaluating a slash event."""
    evidence: SlashEvidence
    severity: float
    repetition_count: int
    repetition_multiplier: float
    confidence: float
    slash_fraction: float  # fraction of bond to slash
    slash_amount: int  # absolute amount in planck
    capped: bool  # whether max_slash_fraction was applied
    verdict_hash: str = ""

    def to_dict(self) -> dict:
        return {
            "fault_type": self.evidence.fault_type.value,
            "operator_id": self.evidence.operator_id,
            "block_number": self.evidence.block_number,
            "severity": self.severity,
            "repetition_count": self.repetition_count,
            "repetition_multiplier": self.repetition_multiplier,
            "confidence": self.confidence,
            "slash_fraction": self.slash_fraction,
            "slash_amount": self.slash_amount,
            "capped": self.capped,
            "evidence_hash": self.evidence.evidence_hash,
            "verdict_hash": self.verdict_hash,
        }


class SlashingEngine:
    """Deterministic, auditable slashing computation.

    No randomness. Same inputs always produce same outputs.
    """

    def __init__(self, config: X3Config):
        self.config = config
        self.fault_history: dict[str, list[SlashEvidence]] = {}  # operator_id -> list
        self.verdicts: list[SlashVerdict] = []

    def evaluate(self, evidence: SlashEvidence, bond_amount: int, confidence: float = 1.0) -> SlashVerdict:
        """Evaluate evidence and compute slash amount.

        Args:
            evidence: The fault evidence.
            bond_amount: Current effective bond of the operator.
            confidence: Confidence factor [0.0, 1.0] from jury/detection system.

        Returns:
            SlashVerdict with computed amounts.
        """
        if confidence < 0.0 or confidence > 1.0:
            raise ValueError(f"Confidence must be [0.0, 1.0], got {confidence}")

        # Base severity from config table
        severity = self.config.slashing.severity_table.get(evidence.fault_type.value, 0.05)

        # Track history for repetition multiplier
        history = self.fault_history.setdefault(evidence.operator_id, [])
        same_type_count = sum(
            1 for h in history if h.fault_type == evidence.fault_type
        )
        repetition_count = same_type_count + 1

        # Repetition multiplier: base^(count-1), e.g. 1.5^0=1, 1.5^1=1.5, 1.5^2=2.25
        rep_base = self.config.slashing.repetition_base
        rep_multiplier = rep_base ** (repetition_count - 1)

        # Raw slash fraction
        raw_fraction = severity * rep_multiplier * confidence
        max_fraction = self.config.slashing.max_slash_fraction
        capped = raw_fraction > max_fraction
        slash_fraction = min(raw_fraction, max_fraction)

        # Absolute amount
        slash_amount = int(bond_amount * slash_fraction)

        # Build verdict hash for auditability
        verdict_data = (
            f"{evidence.evidence_hash}:{severity}:{repetition_count}:"
            f"{rep_multiplier}:{confidence}:{slash_fraction}:{slash_amount}"
        )
        verdict_hash = hashlib.sha256(verdict_data.encode()).hexdigest()

        verdict = SlashVerdict(
            evidence=evidence,
            severity=severity,
            repetition_count=repetition_count,
            repetition_multiplier=rep_multiplier,
            confidence=confidence,
            slash_fraction=slash_fraction,
            slash_amount=slash_amount,
            capped=capped,
            verdict_hash=verdict_hash,
        )

        # Record
        history.append(evidence)
        self.verdicts.append(verdict)

        logger.warning(
            "slash verdict: operator=%s fault=%s severity=%.3f rep=%d rep_mul=%.2f "
            "conf=%.2f fraction=%.4f amount=%d capped=%s",
            evidence.operator_id, evidence.fault_type.value,
            severity, repetition_count, rep_multiplier,
            confidence, slash_fraction, slash_amount, capped,
        )

        return verdict

    def get_operator_history(self, operator_id: str) -> list[SlashEvidence]:
        return list(self.fault_history.get(operator_id, []))

    def get_operator_verdicts(self, operator_id: str) -> list[SlashVerdict]:
        return [v for v in self.verdicts if v.evidence.operator_id == operator_id]

    def cumulative_slash(self, operator_id: str) -> int:
        return sum(v.slash_amount for v in self.get_operator_verdicts(operator_id))

    def reset_operator_history(self, operator_id: str):
        """Reset fault history (e.g., after successful appeal)."""
        self.fault_history.pop(operator_id, None)
        logger.info("slash history reset for operator=%s", operator_id)
