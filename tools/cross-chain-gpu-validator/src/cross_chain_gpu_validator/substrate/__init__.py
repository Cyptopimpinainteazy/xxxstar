"""Substrate chain validator."""

from __future__ import annotations

from typing import Iterable

from cross_chain_gpu_validator.chain_adapter import ChainValidator, ChainConfig, ChainTransaction
from cross_chain_gpu_validator.gpu import Ed25519BatchVerifier


class SubstrateValidator(ChainValidator):
    """Validates Substrate transactions using ED25519 batch verification.
    
    Substrate chains (Polkadot, Kusama, etc.) use SR25519 signatures (variant of ED25519)
    with Blake2b hashing. This validator uses GPU-accelerated ED25519 batch verification.
    """

    def __init__(self, config: ChainConfig, sig_verifier: Ed25519BatchVerifier) -> None:
        super().__init__(config)
        self._sig_verifier = sig_verifier

    def validate_transaction(self, tx: ChainTransaction) -> bool:
        """Validate a single transaction."""
        result = self.validate_transactions([tx])
        return result[0] if result else False

    def validate_transactions(self, transactions: Iterable[ChainTransaction]) -> list[bool]:
        """Validate a batch of Substrate transactions."""
        txs = list(transactions)
        if not txs:
            return []

        signatures = [tx.signature for tx in txs]
        messages = [self.prepare_message(tx.payload) for tx in txs]
        pubkeys = [tx.pubkey for tx in txs]

        return self._sig_verifier.verify_batch(signatures, messages, pubkeys)
