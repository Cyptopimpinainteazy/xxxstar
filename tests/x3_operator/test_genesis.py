"""Tests for x3_operator.genesis"""

import json
import tempfile
from pathlib import Path

import pytest

from x3_operator.config import X3Config
from x3_operator.genesis import (
    GenesisCeremony,
    GenesisConfig,
    GenesisParticipant,
)

VALIDATORS = [
    "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY",
    "5FHneW46xGXgs5mUiveU4sbTyGBzmstUspZC92UhjJM694ty",
    "5FLSigC9HGRKVhB9FiEo4Y3koPsNmBmLJbpXg2mp1hXcS59Y",
]


@pytest.fixture
def ceremony():
    return GenesisCeremony(X3Config())


def test_configure_genesis(ceremony):
    genesis = ceremony.configure_genesis(
        chain_id="test-chain",
        chain_name="Test Chain",
        initial_validators=VALIDATORS,
        initial_balances=dict.fromkeys(VALIDATORS, 1000000),
    )
    assert genesis.chain_id == "test-chain"
    assert len(genesis.initial_validators) == 3


def test_genesis_hash_deterministic():
    g1 = GenesisConfig(chain_id="test", chain_name="Test",
                       initial_validators=VALIDATORS,
                       initial_balances=dict.fromkeys(VALIDATORS, 1000))
    g2 = GenesisConfig(chain_id="test", chain_name="Test",
                       initial_validators=VALIDATORS,
                       initial_balances=dict.fromkeys(VALIDATORS, 1000))
    assert g1.compute_hash() == g2.compute_hash()


def test_freeze():
    g = GenesisConfig(chain_id="test", chain_name="Test",
                      initial_validators=VALIDATORS,
                      initial_balances=dict.fromkeys(VALIDATORS, 1000))
    h = g.freeze()
    assert g.frozen is True
    assert h == g.frozen_hash


def test_double_freeze():
    g = GenesisConfig(chain_id="test", chain_name="Test",
                      initial_validators=VALIDATORS)
    g.freeze()
    with pytest.raises(RuntimeError, match="already frozen"):
        g.freeze()


def test_verify_frozen():
    g = GenesisConfig(chain_id="test", chain_name="Test",
                      initial_validators=VALIDATORS,
                      initial_balances=dict.fromkeys(VALIDATORS, 1000))
    g.freeze()
    assert g.verify() is True


def test_participant_sign():
    p = GenesisParticipant(operator_id="op-001", pubkey="pk", role="validator", stake=1000)
    h = p.sign_genesis("genesis_hash_123")
    assert h
    assert p.signed_at > 0


def test_full_ceremony(ceremony):
    balances = dict.fromkeys(VALIDATORS, 1000000)
    ceremony.configure_genesis("test", "Test", VALIDATORS, balances, VALIDATORS[0])

    for i, v in enumerate(VALIDATORS):
        ceremony.add_participant(GenesisParticipant(
            operator_id=f"op-{i}", pubkey=v, role="validator", stake=balances[v],
        ))

    n = ceremony.collect_attestations()
    assert n == 3

    frozen = ceremony.freeze_genesis()
    assert frozen

    passed, errors = ceremony.verify_genesis()
    assert passed
    assert not errors


def test_generate_chain_spec(ceremony):
    balances = dict.fromkeys(VALIDATORS, 1000000)
    ceremony.configure_genesis("test", "Test", VALIDATORS, balances, VALIDATORS[0])

    for i, v in enumerate(VALIDATORS):
        ceremony.add_participant(GenesisParticipant(
            operator_id=f"op-{i}", pubkey=v, role="validator", stake=balances[v],
        ))

    ceremony.collect_attestations()
    ceremony.freeze_genesis()

    with tempfile.TemporaryDirectory() as tmpdir:
        spec_path = Path(tmpdir) / "chain-spec.json"
        ceremony.generate_chain_spec(spec_path)
        assert spec_path.exists()

        spec = json.loads(spec_path.read_text())
        assert spec["id"] == "test"
        assert spec["properties"]["tokenSymbol"] == "X3"
        assert len(spec["genesis"]["runtime"]["aura"]["authorities"]) == 3


def test_dry_run_passes(ceremony):
    balances = dict.fromkeys(VALIDATORS, 1000000)
    ceremony.configure_genesis("test", "Test", VALIDATORS, balances, VALIDATORS[0])
    passed, _issues = ceremony.dry_run()
    assert passed


def test_dry_run_missing_validator_balance(ceremony):
    # Validator without balance
    ceremony.configure_genesis("test", "Test", VALIDATORS, {}, VALIDATORS[0])
    passed, issues = ceremony.dry_run()
    assert not passed
    assert len(issues) >= 3  # 3 validators without balances


def test_status(ceremony):
    status = ceremony.status()
    assert "steps" in status
    assert len(status["steps"]) == 7
    assert status["genesis_frozen"] is False
