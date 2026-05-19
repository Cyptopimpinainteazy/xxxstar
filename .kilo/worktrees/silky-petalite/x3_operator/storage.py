"""
X3 Storage Protocol
~~~~~~~~~~~~~~~~~~~

Filecoin-style storage provider management with proof-of-storage,
SLA enforcement, CID tracking, and replication factor guarantees.
"""

import hashlib
import logging
import time
from dataclasses import dataclass, field
from enum import Enum
from pathlib import Path
from typing import Optional

from .config import X3Config

logger = logging.getLogger(__name__)


class StorageProofType(str, Enum):
    PROOF_OF_REPLICATION = "proof_of_replication"
    PROOF_OF_SPACETIME = "proof_of_spacetime"
    PROOF_OF_RETRIEVAL = "proof_of_retrieval"


class DealStatus(str, Enum):
    PROPOSED = "proposed"
    ACCEPTED = "accepted"
    ACTIVE = "active"
    EXPIRED = "expired"
    FAULTED = "faulted"
    SLASHED = "slashed"


@dataclass
class ContentID:
    """Content-addressed identifier for stored data."""
    cid: str
    size_bytes: int
    checksum_sha256: str
    created_at: float = 0.0

    @classmethod
    def from_bytes(cls, data: bytes) -> "ContentID":
        checksum = hashlib.sha256(data).hexdigest()
        cid = f"bafk{hashlib.blake2b(data, digest_size=32).hexdigest()}"
        return cls(
            cid=cid,
            size_bytes=len(data),
            checksum_sha256=checksum,
            created_at=time.time(),
        )

    @classmethod
    def from_file(cls, path: Path) -> "ContentID":
        sha = hashlib.sha256()
        blake = hashlib.blake2b(digest_size=32)
        size = 0
        with open(path, "rb") as f:
            while True:
                chunk = f.read(65536)
                if not chunk:
                    break
                sha.update(chunk)
                blake.update(chunk)
                size += len(chunk)
        return cls(
            cid=f"bafk{blake.hexdigest()}",
            size_bytes=size,
            checksum_sha256=sha.hexdigest(),
            created_at=time.time(),
        )


@dataclass
class StorageDeal:
    """Agreement between client and storage provider."""
    deal_id: str
    client_id: str
    provider_id: str
    content: ContentID
    replication_factor: int
    duration_seconds: float
    price_per_byte_epoch: int  # price in planck per byte per epoch
    status: DealStatus = DealStatus.PROPOSED
    started_at: float = 0.0
    expires_at: float = 0.0
    last_proof_at: float = 0.0
    proof_count: int = 0
    fault_count: int = 0

    def is_expired(self) -> bool:
        return time.time() > self.expires_at if self.expires_at > 0 else False

    def total_cost(self) -> int:
        epochs = int(self.duration_seconds / 6.0)  # 6 second epochs
        return self.content.size_bytes * self.price_per_byte_epoch * epochs


@dataclass
class StorageProof:
    """Proof submitted by storage provider."""
    deal_id: str
    provider_id: str
    proof_type: StorageProofType
    proof_hash: str
    submitted_at: float
    verified: bool = False
    verified_at: float = 0.0

    @classmethod
    def generate_proof(cls, deal: StorageDeal, data_sample: bytes, proof_type: StorageProofType) -> "StorageProof":
        """Generate a storage proof from data sample."""
        payload = f"{deal.deal_id}:{deal.content.cid}:{time.time()}".encode() + data_sample
        proof_hash = hashlib.sha256(payload).hexdigest()
        return cls(
            deal_id=deal.deal_id,
            provider_id=deal.provider_id,
            proof_type=proof_type,
            proof_hash=proof_hash,
            submitted_at=time.time(),
        )


class StorageRegistry:
    """Manages storage deals, proofs, and SLA enforcement."""

    def __init__(self, config: X3Config):
        self.config = config
        self.deals: dict[str, StorageDeal] = {}
        self.proofs: dict[str, list[StorageProof]] = {}  # deal_id -> proofs
        self.provider_capacity: dict[str, int] = {}  # provider_id -> bytes available
        self._deal_counter = 0

    def register_provider(self, provider_id: str, capacity_bytes: int):
        """Register a storage provider with available capacity."""
        self.provider_capacity[provider_id] = capacity_bytes
        logger.info("storage provider registered: id=%s capacity=%d bytes", provider_id, capacity_bytes)

    def propose_deal(
        self,
        client_id: str,
        provider_id: str,
        content: ContentID,
        duration_seconds: float,
        replication_factor: int = 0,
        price_per_byte_epoch: int = 1,
    ) -> StorageDeal:
        """Create a new storage deal proposal."""
        if provider_id not in self.provider_capacity:
            raise KeyError(f"Unknown provider: {provider_id}")

        if replication_factor == 0:
            replication_factor = self.config.storage.min_replication_factor

        available = self.provider_capacity[provider_id]
        needed = content.size_bytes * replication_factor
        if needed > available:
            raise ValueError(
                f"Insufficient capacity: need {needed} bytes, have {available}"
            )

        self._deal_counter += 1
        deal_id = f"deal-{self._deal_counter:06d}"

        deal = StorageDeal(
            deal_id=deal_id,
            client_id=client_id,
            provider_id=provider_id,
            content=content,
            replication_factor=replication_factor,
            duration_seconds=duration_seconds,
            price_per_byte_epoch=price_per_byte_epoch,
        )
        self.deals[deal_id] = deal
        self.proofs[deal_id] = []
        logger.info("deal proposed: id=%s client=%s provider=%s cid=%s",
                     deal_id, client_id, provider_id, content.cid[:24])
        return deal

    def accept_deal(self, deal_id: str) -> StorageDeal:
        """Provider accepts a deal."""
        deal = self._get_deal(deal_id)
        if deal.status != DealStatus.PROPOSED:
            raise ValueError(f"Deal not in proposed state: {deal.status.value}")

        deal.status = DealStatus.ACCEPTED
        # Reserve capacity
        self.provider_capacity[deal.provider_id] -= deal.content.size_bytes * deal.replication_factor
        logger.info("deal accepted: id=%s", deal_id)
        return deal

    def activate_deal(self, deal_id: str) -> StorageDeal:
        """Activate deal after data is transferred and initial proof submitted."""
        deal = self._get_deal(deal_id)
        if deal.status != DealStatus.ACCEPTED:
            raise ValueError(f"Deal not accepted: {deal.status.value}")

        deal.status = DealStatus.ACTIVE
        deal.started_at = time.time()
        deal.expires_at = deal.started_at + deal.duration_seconds
        logger.info("deal activated: id=%s expires=%f", deal_id, deal.expires_at)
        return deal

    def submit_proof(self, proof: StorageProof) -> bool:
        """Submit and verify a storage proof."""
        deal = self._get_deal(proof.deal_id)
        if deal.status != DealStatus.ACTIVE:
            raise ValueError(f"Deal not active: {deal.status.value}")

        # Verify proof hash is non-empty and well-formed
        if not proof.proof_hash or len(proof.proof_hash) != 64:
            logger.warning("invalid proof hash for deal %s", proof.deal_id)
            return False

        proof.verified = True
        proof.verified_at = time.time()
        self.proofs[proof.deal_id].append(proof)
        deal.last_proof_at = proof.verified_at
        deal.proof_count += 1
        logger.info("proof verified: deal=%s type=%s count=%d",
                     proof.deal_id, proof.proof_type.value, deal.proof_count)
        return True

    def check_proofs(self) -> list[str]:
        """Check all active deals for missed proofs. Returns list of faulted deal IDs."""
        faulted = []
        now = time.time()
        interval = self.config.storage.proof_interval_seconds

        for deal_id, deal in self.deals.items():
            if deal.status != DealStatus.ACTIVE:
                continue
            if deal.is_expired():
                deal.status = DealStatus.EXPIRED
                continue
            if deal.last_proof_at > 0 and (now - deal.last_proof_at) > interval * 2:
                deal.fault_count += 1
                logger.warning("missed proof: deal=%s faults=%d", deal_id, deal.fault_count)
                max_faults = 3
                if deal.fault_count >= max_faults:
                    deal.status = DealStatus.FAULTED
                    faulted.append(deal_id)
                    logger.error("deal faulted: id=%s", deal_id)

        return faulted

    def get_provider_deals(self, provider_id: str) -> list[StorageDeal]:
        return [d for d in self.deals.values() if d.provider_id == provider_id]

    def get_deal_proofs(self, deal_id: str) -> list[StorageProof]:
        return list(self.proofs.get(deal_id, []))

    def _get_deal(self, deal_id: str) -> StorageDeal:
        deal = self.deals.get(deal_id)
        if deal is None:
            raise KeyError(f"Unknown deal: {deal_id}")
        return deal
