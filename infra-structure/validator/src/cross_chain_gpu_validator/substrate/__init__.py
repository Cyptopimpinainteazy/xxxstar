"""Substrate chain validator."""

from __future__ import annotations

from typing import Iterable

from cross_chain_gpu_validator.chain_adapter import (
    ChainConfig,
    ChainTransaction,
    ChainValidator,
)
from cross_chain_gpu_validator.gpu import Ed25519BatchVerifier


class SubstrateValidator(ChainValidator):
    """Validates ED25519-configured Substrate transactions.

    SR25519 is a distinct signature scheme and must use native Substrate
    verification rather than the Ed25519 kernel.
    """

    def __init__(self, config: ChainConfig, sig_verifier: Ed25519BatchVerifier) -> None:
        super().__init__(config)
        self._sig_verifier = sig_verifier

    def validate_transaction(self, tx: ChainTransaction) -> bool:
        """Validate a single transaction."""
        result = self.validate_transactions([tx])
        return result[0] if result else False

    def validate_transactions(
        self, transactions: Iterable[ChainTransaction]
    ) -> list[bool]:
        """Validate a batch of Substrate transactions."""
        txs = list(transactions)
        if not txs:
            return []

        return self.verify_signed_batch(
            txs, self._sig_verifier, lambda payload: payload
        )
