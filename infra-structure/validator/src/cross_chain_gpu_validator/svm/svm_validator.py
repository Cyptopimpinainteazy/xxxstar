"""SVM validation pipeline with GPU acceleration."""

from __future__ import annotations

from dataclasses import dataclass
from typing import Iterable, cast

from cross_chain_gpu_validator.chain_adapter import (
    ChainConfig,
    ChainTransaction,
    ChainValidator,
)
from cross_chain_gpu_validator.gpu import Ed25519BatchVerifier


@dataclass(frozen=True)
class SvmTransaction:
    signature: bytes
    pubkey: bytes
    payload: bytes


class SvmValidator(ChainValidator):
    """Validates SVM transactions using Ed25519 batch verification."""

    def __init__(self, config: ChainConfig, sig_verifier: Ed25519BatchVerifier) -> None:
        super().__init__(config)
        self._sig_verifier = sig_verifier

    def validate_transaction(self, tx: ChainTransaction) -> bool:
        """Validate a single transaction."""
        result = self.validate_transactions([tx])
        return result[0] if result else False

    def validate_transactions(
        self, transactions: Iterable[ChainTransaction] | Iterable[SvmTransaction]
    ) -> list[bool]:
        """Validate a batch of transactions using GPU acceleration."""
        txs = list(transactions)
        if not txs:
            return []

        # Support both ChainTransaction and legacy SvmTransaction
        svm_flags = [isinstance(tx, SvmTransaction) for tx in txs]
        if all(svm_flags):
            return self._validate_svm_transactions(cast(list[SvmTransaction], txs))
        if any(svm_flags):
            raise ValueError("cannot mix SVM and generic transactions in one batch")
        return self._validate_chain_transactions(cast(list[ChainTransaction], txs))

    def _validate_svm_transactions(
        self, transactions: Iterable[SvmTransaction]
    ) -> list[bool]:
        """Validate legacy SvmTransaction format."""
        return self.verify_signed_batch(
            transactions,
            self._sig_verifier,
            lambda payload: payload,
        )

    def _validate_chain_transactions(
        self, transactions: Iterable[ChainTransaction]
    ) -> list[bool]:
        """Validate generic ChainTransaction format."""
        return self.verify_signed_batch(
            transactions, self._sig_verifier, lambda payload: payload
        )
