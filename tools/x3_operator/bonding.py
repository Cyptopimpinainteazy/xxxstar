"""
X3 Operator Bonding
~~~~~~~~~~~~~~~~~~~

On-chain bond management for operator stake.
Deterministic escrow logic with role-based minimums.
"""

import hashlib
import json
import logging
import time
from dataclasses import dataclass, field
from enum import Enum
from pathlib import Path
from typing import Optional

from .config import OperatorRole, X3Config

logger = logging.getLogger(__name__)


class BondStatus(str, Enum):
    UNBONDED = "unbonded"
    PENDING = "pending"
    BONDED = "bonded"
    UNBONDING = "unbonding"
    SLASHED = "slashed"


@dataclass
class BondRecord:
    operator_id: str
    role: OperatorRole
    amount: int  # in smallest units (planck/lamports)
    status: BondStatus
    bonded_at: float = 0.0
    unbonding_started: float = 0.0
    unbonding_completes: float = 0.0
    slash_total: int = 0
    tx_hash: str = ""
    nonce: int = 0

    def effective_stake(self) -> int:
        """Stake minus cumulative slashes."""
        return max(0, self.amount - self.slash_total)

    def is_below_minimum(self, config: X3Config) -> bool:
        min_bond = config.min_bond_for_role(self.role)
        return self.effective_stake() < min_bond

    def to_dict(self) -> dict:
        return {
            "operator_id": self.operator_id,
            "role": self.role.value,
            "amount": self.amount,
            "status": self.status.value,
            "bonded_at": self.bonded_at,
            "unbonding_started": self.unbonding_started,
            "unbonding_completes": self.unbonding_completes,
            "slash_total": self.slash_total,
            "tx_hash": self.tx_hash,
            "nonce": self.nonce,
        }

    @classmethod
    def from_dict(cls, data: dict) -> "BondRecord":
        return cls(
            operator_id=data["operator_id"],
            role=OperatorRole(data["role"]),
            amount=data["amount"],
            status=BondStatus(data["status"]),
            bonded_at=data.get("bonded_at", 0.0),
            unbonding_started=data.get("unbonding_started", 0.0),
            unbonding_completes=data.get("unbonding_completes", 0.0),
            slash_total=data.get("slash_total", 0),
            tx_hash=data.get("tx_hash", ""),
            nonce=data.get("nonce", 0),
        )


@dataclass
class BondLedger:
    """Tracks all bond operations for an operator."""
    records: dict = field(default_factory=dict)  # operator_id -> BondRecord
    history: list = field(default_factory=list)

    def _log_event(self, event_type: str, operator_id: str, amount: int = 0, detail: str = ""):
        entry = {
            "ts": time.time(),
            "type": event_type,
            "operator_id": operator_id,
            "amount": amount,
            "detail": detail,
        }
        self.history.append(entry)
        logger.info("bond event: %s operator=%s amount=%d %s", event_type, operator_id, amount, detail)

    def create_bond(self, operator_id: str, role: OperatorRole, amount: int, config: X3Config) -> BondRecord:
        """Create a new bond. Validates minimum requirements."""
        min_bond = config.min_bond_for_role(role)
        if amount < min_bond:
            raise ValueError(
                f"Bond amount {amount} below minimum {min_bond} for role {role.value}"
            )

        if operator_id in self.records:
            existing = self.records[operator_id]
            if existing.status in (BondStatus.BONDED, BondStatus.PENDING):
                raise ValueError(f"Operator {operator_id} already has an active bond")

        nonce = len(self.history)
        tx_data = f"{operator_id}:{role.value}:{amount}:{nonce}:{time.time()}"
        tx_hash = hashlib.sha256(tx_data.encode()).hexdigest()

        record = BondRecord(
            operator_id=operator_id,
            role=role,
            amount=amount,
            status=BondStatus.PENDING,
            nonce=nonce,
            tx_hash=tx_hash,
        )
        self.records[operator_id] = record
        self._log_event("bond_created", operator_id, amount, f"role={role.value} tx={tx_hash[:16]}")
        return record

    def confirm_bond(self, operator_id: str) -> BondRecord:
        """Mark a pending bond as confirmed (on-chain finality)."""
        record = self._get_record(operator_id)
        if record.status != BondStatus.PENDING:
            raise ValueError(f"Bond not in pending state: {record.status.value}")
        record.status = BondStatus.BONDED
        record.bonded_at = time.time()
        self._log_event("bond_confirmed", operator_id, record.amount)
        return record

    def start_unbonding(self, operator_id: str, config: X3Config) -> BondRecord:
        """Begin unbonding period. Operator remains active until period completes."""
        record = self._get_record(operator_id)
        if record.status != BondStatus.BONDED:
            raise ValueError(f"Cannot unbond from state: {record.status.value}")
        record.status = BondStatus.UNBONDING
        record.unbonding_started = time.time()
        record.unbonding_completes = record.unbonding_started + config.bonding.unbonding_delay_seconds
        self._log_event(
            "unbonding_started", operator_id, record.amount,
            f"completes_at={record.unbonding_completes}",
        )
        return record

    def complete_unbonding(self, operator_id: str) -> BondRecord:
        """Finalize unbonding after delay period."""
        record = self._get_record(operator_id)
        if record.status != BondStatus.UNBONDING:
            raise ValueError(f"Not unbonding: {record.status.value}")
        if time.time() < record.unbonding_completes:
            remaining = record.unbonding_completes - time.time()
            raise ValueError(f"Unbonding not complete. {remaining:.0f}s remaining.")
        record.status = BondStatus.UNBONDED
        self._log_event("unbonding_complete", operator_id, record.effective_stake())
        return record

    def apply_slash(self, operator_id: str, amount: int, reason: str) -> BondRecord:
        """Apply a slash to an operator's bond."""
        record = self._get_record(operator_id)
        if record.status not in (BondStatus.BONDED, BondStatus.UNBONDING):
            raise ValueError(f"Cannot slash in state: {record.status.value}")
        actual = min(amount, record.effective_stake())
        record.slash_total += actual
        self._log_event("slash_applied", operator_id, actual, f"reason={reason}")
        if record.effective_stake() == 0:
            record.status = BondStatus.SLASHED
            self._log_event("bond_fully_slashed", operator_id, 0)
        return record

    def top_up(self, operator_id: str, amount: int) -> BondRecord:
        """Add additional stake to an existing bond."""
        record = self._get_record(operator_id)
        if record.status != BondStatus.BONDED:
            raise ValueError(f"Can only top up bonded operators: {record.status.value}")
        record.amount += amount
        self._log_event("bond_topped_up", operator_id, amount)
        return record

    def get_bond(self, operator_id: str) -> Optional[BondRecord]:
        return self.records.get(operator_id)

    def _get_record(self, operator_id: str) -> BondRecord:
        record = self.records.get(operator_id)
        if record is None:
            raise KeyError(f"No bond record for operator {operator_id}")
        return record

    def save(self, path: Path):
        data = {
            "records": {k: v.to_dict() for k, v in self.records.items()},
            "history": self.history,
        }
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(json.dumps(data, indent=2))

    @classmethod
    def load(cls, path: Path) -> "BondLedger":
        data = json.loads(path.read_text())
        ledger = cls()
        for k, v in data.get("records", {}).items():
            ledger.records[k] = BondRecord.from_dict(v)
        ledger.history = data.get("history", [])
        return ledger
