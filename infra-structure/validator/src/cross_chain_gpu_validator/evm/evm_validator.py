"""EVM validation pipeline using GPU accelerators."""

from __future__ import annotations

from dataclasses import dataclass
from typing import Iterable, cast

from cross_chain_gpu_validator.chain_adapter import (
    ChainConfig,
    ChainTransaction,
    ChainValidator,
)
from cross_chain_gpu_validator.gpu import KeccakBatchHasher, Secp256k1BatchVerifier
from cross_chain_gpu_validator.gpu.keccak_gpu import _keccak256

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
        hasher: KeccakBatchHasher,
    ) -> None:
        super().__init__(config)
        self._sig_verifier = sig_verifier
        self._hasher = hasher

    def validate_transaction(self, tx: ChainTransaction) -> bool:
        """Validate a single transaction."""
        result = self.validate_transactions([tx])
        return result[0] if result else False

    def validate_transactions(
        self, transactions: Iterable[ChainTransaction] | Iterable[EvmTransaction]
    ) -> list[bool]:
        """Validate a batch of transactions using GPU accelerators."""
        txs = list(transactions)
        if not txs:
            return []

        # Support both ChainTransaction and legacy EvmTransaction
        evm_flags = [isinstance(tx, EvmTransaction) for tx in txs]
        if all(evm_flags):
            return self._validate_evm_transactions(cast(list[EvmTransaction], txs))
        if any(evm_flags):
            raise ValueError("cannot mix EVM and generic transactions in one batch")
        return self._validate_chain_transactions(cast(list[ChainTransaction], txs))

    def _validate_evm_transactions(
        self, transactions: Iterable[EvmTransaction]
    ) -> list[bool]:
        """Validate legacy EvmTransaction format using real Keccak-256."""
        return self.verify_signed_batch(transactions, self._sig_verifier, _keccak256)

    def _validate_chain_transactions(
        self, transactions: Iterable[ChainTransaction]
    ) -> list[bool]:
        """Validate generic ChainTransaction format."""
        return self.verify_signed_batch(
            transactions, self._sig_verifier, self.prepare_message
        )

    def validate_state_root(
        self, state_nodes: Iterable[bytes], expected_root: bytes
    ) -> bool:
        computed_root = merkle_root(state_nodes)
        return computed_root == expected_root

    def hash_state_nodes(self, state_nodes: Iterable[bytes]) -> list[bytes]:
        return self._hasher.hash_batch(state_nodes)
