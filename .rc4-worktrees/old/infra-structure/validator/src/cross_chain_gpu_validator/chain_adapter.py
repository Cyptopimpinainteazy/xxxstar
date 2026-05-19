"""Abstract chain validator interface for multi-chain support."""

from __future__ import annotations

from abc import ABC, abstractmethod
from dataclasses import dataclass
from enum import Enum
from typing import Any, Iterable


class SignatureAlgorithm(Enum):
    """Supported signature algorithms."""

    SECP256K1 = "secp256k1"
    ED25519 = "ed25519"
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
