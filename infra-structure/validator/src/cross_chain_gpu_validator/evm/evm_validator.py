"""EVM validation pipeline using GPU accelerators."""

from __future__ import annotations

from dataclasses import dataclass
from typing import Iterable

from cross_chain_gpu_validator.gpu import KeccakBatchHasher, Secp256k1BatchVerifier
from cross_chain_gpu_validator.gpu.keccak_gpu import _keccak256
from cross_chain_gpu_validator.chain_adapter import ChainValidator, ChainConfig, ChainTransaction
from .state_root import merkle_root


@dataclass(frozen=True)
class EvmTransaction:
    signature: bytes
    pubkey: bytes
    payload: bytes


class EvmValidator(ChainValidator):
    """Validates EVM transactions and state roots."""

    def __init__(
        self, 
        config: ChainConfig, 
        sig_verifier: Secp256k1BatchVerifier, 
        hasher: KeccakBatchHasher
    ) -> None:
        super().__init__(config)
        self._sig_verifier = sig_verifier
        self._hasher = hasher

    def validate_transaction(self, tx: ChainTransaction) -> bool:
        """Validate a single transaction."""
        result = self.validate_transactions([tx])
        return result[0] if result else False

    def validate_transactions(self, transactions: Iterable[ChainTransaction] | Iterable[EvmTransaction]) -> list[bool]:
        """Validate a batch of transactions using GPU accelerators."""
        txs = list(transactions)
        if not txs:
            return []
        
        # Support both ChainTransaction and legacy EvmTransaction
        if isinstance(txs[0], EvmTransaction):
            return self._validate_evm_transactions(txs)
        else:
            return self._validate_chain_transactions(txs)

    def _validate_evm_transactions(self, transactions: Iterable[EvmTransaction]) -> list[bool]:
        """Validate legacy EvmTransaction format using real Keccak-256."""
        signatures = [tx.signature for tx in transactions]
        messages = [_keccak256(tx.payload) for tx in transactions]
        pubkeys = [tx.pubkey for tx in transactions]
        return self._sig_verifier.verify_batch(signatures, messages, pubkeys)

    def _validate_chain_transactions(self, transactions: Iterable[ChainTransaction]) -> list[bool]:
        """Validate generic ChainTransaction format."""
        signatures = [tx.signature for tx in transactions]
        messages = [self.prepare_message(tx.payload) for tx in transactions]
        pubkeys = [tx.pubkey for tx in transactions]
        return self._sig_verifier.verify_batch(signatures, messages, pubkeys)

    def validate_state_root(self, state_nodes: Iterable[bytes], expected_root: bytes) -> bool:
        computed_root = merkle_root(state_nodes)
        return computed_root == expected_root

    def hash_state_nodes(self, state_nodes: Iterable[bytes]) -> list[bytes]:
        return self._hasher.hash_batch(state_nodes)
