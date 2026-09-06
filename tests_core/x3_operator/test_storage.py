"""Tests for x3_operator.storage"""

import pytest

from x3_operator.config import X3Config
from x3_operator.storage import (
    ContentID,
    DealStatus,
    StorageDeal,
    StorageProof,
    StorageProofType,
    StorageRegistry,
)


@pytest.fixture
def registry():
    return StorageRegistry(X3Config())


@pytest.fixture
def content():
    return ContentID.from_bytes(b"test data payload for storage")


def test_content_id_from_bytes():
    cid = ContentID.from_bytes(b"hello world")
    assert cid.cid.startswith("bafk")
    assert cid.size_bytes == 11
    assert cid.checksum_sha256


def test_content_id_deterministic():
    c1 = ContentID.from_bytes(b"same data")
    c2 = ContentID.from_bytes(b"same data")
    assert c1.cid == c2.cid
    assert c1.checksum_sha256 == c2.checksum_sha256


def test_register_provider(registry):
    registry.register_provider("prov-1", 1_000_000)
    assert registry.provider_capacity["prov-1"] == 1_000_000


def test_propose_deal(registry, content):
    registry.register_provider("prov-1", 1_000_000)
    deal = registry.propose_deal("client-1", "prov-1", content, 86400)
    assert deal.status == DealStatus.PROPOSED
    assert deal.deal_id.startswith("deal-")


def test_propose_deal_unknown_provider(registry, content):
    with pytest.raises(KeyError, match="Unknown provider"):
        registry.propose_deal("client-1", "nonexistent", content, 86400)


def test_propose_deal_insufficient_capacity(registry):
    registry.register_provider("prov-1", 10)  # Only 10 bytes
    big_content = ContentID(cid="bafk123", size_bytes=1000, checksum_sha256="abc")
    with pytest.raises(ValueError, match="Insufficient capacity"):
        registry.propose_deal("client-1", "prov-1", big_content, 86400)


def test_accept_deal(registry, content):
    registry.register_provider("prov-1", 1_000_000)
    deal = registry.propose_deal("client-1", "prov-1", content, 86400)
    accepted = registry.accept_deal(deal.deal_id)
    assert accepted.status == DealStatus.ACCEPTED


def test_activate_deal(registry, content):
    registry.register_provider("prov-1", 1_000_000)
    deal = registry.propose_deal("client-1", "prov-1", content, 86400)
    registry.accept_deal(deal.deal_id)
    active = registry.activate_deal(deal.deal_id)
    assert active.status == DealStatus.ACTIVE
    assert active.started_at > 0
    assert active.expires_at > active.started_at


def test_submit_proof(registry, content):
    registry.register_provider("prov-1", 1_000_000)
    deal = registry.propose_deal("client-1", "prov-1", content, 86400)
    registry.accept_deal(deal.deal_id)
    registry.activate_deal(deal.deal_id)

    proof = StorageProof.generate_proof(deal, b"sample", StorageProofType.PROOF_OF_REPLICATION)
    ok = registry.submit_proof(proof)
    assert ok is True
    assert deal.proof_count == 1


def test_get_provider_deals(registry, content):
    registry.register_provider("prov-1", 1_000_000)
    registry.propose_deal("client-1", "prov-1", content, 86400)
    registry.propose_deal("client-2", "prov-1", content, 86400)

    deals = registry.get_provider_deals("prov-1")
    assert len(deals) == 2


def test_deal_total_cost(content):
    deal = StorageDeal(
        deal_id="d1", client_id="c1", provider_id="p1",
        content=content, replication_factor=3,
        duration_seconds=600, price_per_byte_epoch=1,
    )
    cost = deal.total_cost()
    # 600s / 6s = 100 epochs * size * 1
    expected = content.size_bytes * 1 * 100
    assert cost == expected
