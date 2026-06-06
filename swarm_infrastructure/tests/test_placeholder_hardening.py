import pytest

from swarm.self_model.ledger import SelfModelLedger
from swarm.storage.backend import PostgresStorage, SqliteStorage


def test_postgres_storage_fails_fast():
    with pytest.raises(NotImplementedError):
        PostgresStorage("postgres://example")


def test_anchor_to_chain_requires_backend():
    ledger = SelfModelLedger(agent_id="agent-1", storage=SqliteStorage(":memory:"))

    with pytest.raises(RuntimeError, match="No chain anchor backend configured"):
        ledger.anchor_to_chain()


def test_anchor_to_chain_uses_injected_backend():
    captured = {}

    def anchor_writer(agent_id: str, version: int, integrity_hash: str) -> str:
        captured["agent_id"] = agent_id
        captured["version"] = version
        captured["integrity_hash"] = integrity_hash
        return "tx-123"

    ledger = SelfModelLedger(
        agent_id="agent-2",
        storage=SqliteStorage(":memory:"),
        anchor_writer=anchor_writer,
    )

    tx_hash = ledger.anchor_to_chain()

    assert tx_hash == "tx-123"
    assert captured["agent_id"] == "agent-2"
    assert isinstance(captured["version"], int)
    assert captured["integrity_hash"]
