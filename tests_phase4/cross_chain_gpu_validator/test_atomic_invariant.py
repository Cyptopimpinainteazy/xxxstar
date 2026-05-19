"""Invariant: INFRA-CCGV-002
Atomic swaps commit only when both sides validate.
"""

from __future__ import annotations

import time
from dataclasses import dataclass

from cross_chain_gpu_validator.metrics import MetricsStore
from cross_chain_gpu_validator.orchestrator.orchestrator import (
    AtomicSwapPayload,
    CrossChainOrchestrator,
)


class FakeRegistry:
    def __init__(self) -> None:
        self._records: dict[str, dict] = {}

    def register_swap(self, swap_id: str, payload: dict) -> None:
        self._records[swap_id] = {
            "swap_id": swap_id,
            "created_at": time.time(),
            "timeout_at": time.time() + payload["timeout_seconds"],
            "svm_validated": False,
            "evm_validated": False,
            "status": "PENDING",
        }

    def get_swap(self, swap_id: str):
        record = self._records.get(swap_id)
        if record is None:
            return None
        return type("Record", (), record)

    def get_swaps_batch(self, swap_ids: list[str]):
        return [self.get_swap(swap_id) for swap_id in swap_ids]

    def update_validation(self, swap_id: str, svm_valid: bool, evm_valid: bool) -> None:
        record = self._records[swap_id]
        record["svm_validated"] = svm_valid
        record["evm_validated"] = evm_valid

    def update_status(self, swap_id: str, status: str) -> None:
        self._records[swap_id]["status"] = status

    def pending_swaps(self):
        return [
            record["swap_id"]
            for record in self._records.values()
            if record["status"] == "PENDING"
        ]


@dataclass(frozen=True)
class FakeTx:
    signature: bytes
    pubkey: bytes
    payload: bytes


class FakeValidator:
    def __init__(self, valid: bool) -> None:
        self._valid = valid

    def validate_transactions(self, transactions):
        return [self._valid for _ in transactions]


class FakeChainRegistry:
    def __init__(self, svm_validator: FakeValidator, evm_validator: FakeValidator) -> None:
        self._validators = {
            "solana": svm_validator,
            "ethereum": evm_validator,
        }

    def validate_enabled_chains(self, chain_ids) -> bool:
        return all(chain_id in self._validators for chain_id in chain_ids)

    def get_validator(self, chain_id: str):
        return self._validators.get(chain_id)


def test_atomic_commit_requires_both_sides() -> None:
    registry = FakeRegistry()
    metrics = MetricsStore()

    svm_validator = FakeValidator(valid=True)
    evm_validator = FakeValidator(valid=True)
    chain_registry = FakeChainRegistry(svm_validator, evm_validator)
    orchestrator = CrossChainOrchestrator(registry, chain_registry, metrics)

    payload = AtomicSwapPayload(
        swap_id="swap-1",
        svm_transactions=[FakeTx(b"s", b"p", b"m")],
        evm_transactions=[FakeTx(b"s", b"p", b"m")],
        timeout_seconds=30,
    )

    orchestrator.submit_swap_legacy(payload)
    orchestrator.process_pending()

    record = registry.get_swap("swap-1")
    assert record.status == "APPROVED"


def test_atomic_rollback_on_failure() -> None:
    registry = FakeRegistry()
    metrics = MetricsStore()

    svm_validator = FakeValidator(valid=True)
    evm_validator = FakeValidator(valid=False)
    chain_registry = FakeChainRegistry(svm_validator, evm_validator)
    orchestrator = CrossChainOrchestrator(registry, chain_registry, metrics)

    payload = AtomicSwapPayload(
        swap_id="swap-2",
        svm_transactions=[FakeTx(b"s", b"p", b"m")],
        evm_transactions=[FakeTx(b"s", b"p", b"m")],
        timeout_seconds=30,
    )

    orchestrator.submit_swap_legacy(payload)
    orchestrator.process_pending()

    record = registry.get_swap("swap-2")
    assert record.status == "FAILED"
