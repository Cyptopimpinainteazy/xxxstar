"""Abstract chain validator interface for multi-chain support."""

from __future__ import annotations

from abc import ABC, abstractmethod
from dataclasses import dataclass
from enum import Enum
from typing import Any, Callable, Iterable, Protocol


class SignatureAlgorithm(Enum):
    """Supported signature algorithms."""

    SECP256K1 = "secp256k1"
    ED25519 = "ed25519"
    # Schnorrkel over Ristretto25519: not Ed25519, and not verifiable by the
    # Ed25519 kernel. No verifier for it exists here yet.
    SR25519 = "sr25519"
    BLS = "bls"


class HashAlgorithm(Enum):
    """Supported hash algorithms."""

    SHA256 = "sha256"
    SHA3_256 = "sha3_256"
    KECCAK256 = "keccak256"


@dataclass(frozen=True)
class ChainConfig:
    """Configuration for a blockchain."""

    chain_id: str
    chain_name: str
    rpc_url: str
    sig_algorithm: SignatureAlgorithm
    hash_algorithm: HashAlgorithm
    sig_pubkey_size: int  # bytes
    sig_signature_size: int  # bytes
    hash_output_size: int  # bytes
    supports_gpu: bool = True


@dataclass(frozen=True)
class ChainTransaction:
    """Generic transaction for any chain."""

    chain_id: str
    signature: bytes
    pubkey: bytes
    payload: bytes
    metadata: dict[str, Any] | None = None


class SignedTransaction(Protocol):
    """Transaction fields required for signature verification."""

    @property
    def signature(self) -> bytes: ...

    @property
    def pubkey(self) -> bytes: ...

    @property
    def payload(self) -> bytes: ...


class BatchSignatureVerifier(Protocol):
    """Batch verifier interface shared by Secp256k1 and Ed25519 backends."""

    def verify_batch(
        self,
        signatures: Iterable[bytes],
        messages: Iterable[bytes],
        pubkeys: Iterable[bytes],
    ) -> list[bool]:
        """Verify aligned batches and return one result per signature."""


class ChainValidator(ABC):
    """Abstract validator for chain-specific transaction validation."""

    def __init__(self, config: ChainConfig) -> None:
        self.config = config

    @abstractmethod
    def validate_transaction(self, tx: ChainTransaction) -> bool:
        """Validate a single transaction."""
        pass

    @abstractmethod
    def validate_transactions(self, txs: Iterable[ChainTransaction]) -> list[bool]:
        """Validate a batch of transactions, returning a list of bools."""
        pass

    def prepare_message(self, payload: bytes) -> bytes:
        """Prepare payload for signature verification (hash if needed)."""
        if self.config.hash_algorithm == HashAlgorithm.SHA256:
            import hashlib

            return hashlib.sha256(payload).digest()
        elif self.config.hash_algorithm == HashAlgorithm.SHA3_256:
            import hashlib

            return hashlib.sha3_256(payload).digest()
        elif self.config.hash_algorithm == HashAlgorithm.KECCAK256:
            from Crypto.Hash import keccak

            k = keccak.new(digest_bits=256)
            k.update(payload)
            return k.digest()
        raise ValueError(f"Unsupported hash algorithm: {self.config.hash_algorithm}")

    def verify_signed_batch(
        self,
        transactions: Iterable[SignedTransaction],
        verifier: BatchSignatureVerifier,
        prepare_message: Callable[[bytes], bytes],
    ) -> list[bool]:
        """Verify a batch while rejecting malformed keys without dropping peers."""
        txs = list(transactions)
        shape_valid = [
            len(tx.signature) == self.config.sig_signature_size
            and len(tx.pubkey) == self.config.sig_pubkey_size
            for tx in txs
        ]

        signatures: list[bytes] = []
        messages: list[bytes] = []
        pubkeys: list[bytes] = []
        for tx, is_valid in zip(txs, shape_valid):
            if is_valid:
                signatures.append(tx.signature)
                messages.append(prepare_message(tx.payload))
                pubkeys.append(tx.pubkey)

        verified = iter(verifier.verify_batch(signatures, messages, pubkeys))
        results: list[bool] = []
        for is_valid in shape_valid:
            if not is_valid:
                results.append(False)
                continue
            try:
                results.append(next(verified))
            except StopIteration as exc:
                raise RuntimeError(
                    "signature verifier returned an incomplete result batch"
                ) from exc
        try:
            next(verified)
        except StopIteration:
            return results
        raise RuntimeError("signature verifier returned excess result entries")
