"""SVM validation pipeline with GPU acceleration."""

from __future__ import annotations

from dataclasses import dataclass
import hashlib
from typing import Iterable

from cross_chain_gpu_validator.gpu import Ed25519BatchVerifier
from cross_chain_gpu_validator.chain_adapter import ChainValidator, ChainConfig, ChainTransaction


@dataclass(frozen=True)
class SvmTransaction:
    signature: bytes
    pubkey: bytes
    payload: bytes


class SvmValidator(ChainValidator):
    """Validates SVM transactions using GPU-accelerated batch verification."""

    def __init__(self, config: ChainConfig, sig_verifier: Ed25519BatchVerifier) -> None:
        super().__init__(config)
        self._sig_verifier = sig_verifier

    def validate_transaction(self, tx: ChainTransaction) -> bool:
        """Validate a single transaction."""
        result = self.validate_transactions([tx])
        return result[0] if result else False

    def validate_transactions(self, transactions: Iterable[ChainTransaction] | Iterable[SvmTransaction]) -> list[bool]:
        """Validate a batch of transactions using GPU acceleration."""
        txs = list(transactions)
        if not txs:
            return []
        
        # Support both ChainTransaction and legacy SvmTransaction
        if isinstance(txs[0], SvmTransaction):
            return self._validate_svm_transactions(txs)
        else:
            return self._validate_chain_transactions(txs)

    def _validate_svm_transactions(self, transactions: Iterable[SvmTransaction]) -> list[bool]:
        """Validate legacy SvmTransaction format."""
        signatures = [tx.signature for tx in transactions]
        messages = [hashlib.sha256(tx.payload).digest() for tx in transactions]
        pubkeys = [tx.pubkey for tx in transactions]
        return self._sig_verifier.verify_batch(signatures, messages, pubkeys)

    def _validate_chain_transactions(self, transactions: Iterable[ChainTransaction]) -> list[bool]:
        """Validate generic ChainTransaction format."""
        signatures = [tx.signature for tx in transactions]
        messages = [self.prepare_message(tx.payload) for tx in transactions]
        pubkeys = [tx.pubkey for tx in transactions]
        return self._sig_verifier.verify_batch(signatures, messages, pubkeys)
