"""Cosmos/Tendermint chain validator."""

from __future__ import annotations

from typing import Iterable

from cross_chain_gpu_validator.chain_adapter import ChainValidator, ChainConfig, ChainTransaction
from cross_chain_gpu_validator.gpu import Secp256k1BatchVerifier


class CosmosValidator(ChainValidator):
    """Validates Cosmos/Tendermint transactions using secp256k1 batch verification.
    
    Cosmos chains use secp256k1 signatures with SHA-256 hashing of the transaction payload.
    Public keys are stored as compressed 33-byte values.
    """

    def __init__(self, config: ChainConfig, sig_verifier: Secp256k1BatchVerifier) -> None:
        super().__init__(config)
        self._sig_verifier = sig_verifier

    def validate_transaction(self, tx: ChainTransaction) -> bool:
        """Validate a single transaction."""
        result = self.validate_transactions([tx])
        return result[0] if result else False

    def validate_transactions(self, transactions: Iterable[ChainTransaction]) -> list[bool]:
        """Validate a batch of Cosmos transactions."""
        txs = list(transactions)
        if not txs:
            return []

        signatures = [tx.signature for tx in txs]
        messages = [self.prepare_message(tx.payload) for tx in txs]
        pubkeys = [tx.pubkey for tx in txs]

        return self._sig_verifier.verify_batch(signatures, messages, pubkeys)
