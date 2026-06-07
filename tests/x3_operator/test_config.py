"""Tests for x3_operator.config"""

import tempfile
from pathlib import Path

from x3_operator.config import (
    NetworkPhase,
    OperatorRole,
    X3Config,
)


def test_default_config():
    cfg = X3Config()
    assert cfg.chain.rpc_url == "ws://127.0.0.1:9944"
    assert cfg.chain.network_phase == NetworkPhase.DEVNET
    assert cfg.bonding.min_bond_validator == 10_000
    assert cfg.bonding.min_bond_gpu == 1_000


def test_validate_clean():
    cfg = X3Config()
    errors = cfg.validate()
    assert errors == []


def test_validate_bad_slash_fraction():
    cfg = X3Config()
    cfg.slashing.max_slash_fraction = 1.5
    errors = cfg.validate()
    assert any("max_slash_fraction" in e for e in errors)


def test_validate_bad_heartbeat():
    cfg = X3Config()
    cfg.health.heartbeat_timeout_seconds = 10
    cfg.health.heartbeat_interval_seconds = 30
    errors = cfg.validate()
    assert any("heartbeat" in e for e in errors)


def test_min_bond_for_role():
    cfg = X3Config()
    assert cfg.min_bond_for_role(OperatorRole.VALIDATOR) == 10_000
    assert cfg.min_bond_for_role(OperatorRole.GPU) == 1_000
    assert cfg.min_bond_for_role(OperatorRole.STORAGE) == 2_000
    assert cfg.min_bond_for_role(OperatorRole.RELAYER) == 5_000


def test_save_and_load():
    cfg = X3Config()
    cfg.chain.network_phase = NetworkPhase.TESTNET
    cfg.chain.rpc_url = "ws://testnet.example.com:9944"

    with tempfile.NamedTemporaryFile(suffix=".json", delete=False) as f:
        path = Path(f.name)

    cfg.save(path)
    loaded = X3Config.load(path)

    assert loaded.chain.network_phase == NetworkPhase.TESTNET
    assert loaded.chain.rpc_url == "ws://testnet.example.com:9944"
    assert loaded.bonding.min_bond_validator == 10_000
    path.unlink()


def test_from_env(monkeypatch):
    monkeypatch.setenv("X3_RPC_URL", "ws://env-node:9944")
    monkeypatch.setenv("X3_NETWORK_PHASE", "mainnet")
    monkeypatch.setenv("X3_DEBUG", "true")

    cfg = X3Config.from_env()
    assert cfg.chain.rpc_url == "ws://env-node:9944"
    assert cfg.chain.network_phase == NetworkPhase.MAINNET
    assert cfg.telemetry.debug_mode is True


def test_to_dict():
    cfg = X3Config()
    d = cfg.to_dict()
    assert d["chain"]["rpc_url"] == "ws://127.0.0.1:9944"
    assert "bonding" in d
    assert "slashing" in d


def test_operator_role_enum():
    assert OperatorRole("validator") == OperatorRole.VALIDATOR
    assert OperatorRole("gpu") == OperatorRole.GPU


def test_severity_table_defaults():
    cfg = X3Config()
    table = cfg.slashing.severity_table
    assert "downtime" in table
    assert "equivocation" in table
    assert table["equivocation"] == 0.50
    assert table["data_corruption"] == 1.00
