"""Tests for x3_operator.bonding"""

import tempfile
from pathlib import Path

import pytest

from x3_operator.bonding import BondLedger, BondStatus
from x3_operator.config import OperatorRole, X3Config


@pytest.fixture
def config():
    return X3Config()


@pytest.fixture
def ledger():
    return BondLedger()


def test_create_bond(config, ledger):
    record = ledger.create_bond("op-001", OperatorRole.VALIDATOR, 10_000, config)
    assert record.status == BondStatus.PENDING
    assert record.amount == 10_000
    assert record.tx_hash


def test_confirm_bond(config, ledger):
    ledger.create_bond("op-001", OperatorRole.VALIDATOR, 10_000, config)
    record = ledger.confirm_bond("op-001")
    assert record.status == BondStatus.BONDED
    assert record.bonded_at > 0


def test_bond_below_minimum(config, ledger):
    with pytest.raises(ValueError, match="below minimum"):
        ledger.create_bond("op-001", OperatorRole.VALIDATOR, 1, config)


def test_duplicate_bond(config, ledger):
    ledger.create_bond("op-001", OperatorRole.VALIDATOR, 10_000, config)
    with pytest.raises(ValueError, match="already has an active bond"):
        ledger.create_bond("op-001", OperatorRole.VALIDATOR, 10_000, config)


def test_unbonding(config, ledger):
    ledger.create_bond("op-001", OperatorRole.VALIDATOR, 10_000, config)
    ledger.confirm_bond("op-001")
    record = ledger.start_unbonding("op-001", config)
    assert record.status == BondStatus.UNBONDING
    assert record.unbonding_completes > record.unbonding_started


def test_unbonding_not_complete(config, ledger):
    ledger.create_bond("op-001", OperatorRole.VALIDATOR, 10_000, config)
    ledger.confirm_bond("op-001")
    ledger.start_unbonding("op-001", config)
    with pytest.raises(ValueError, match="not complete"):
        ledger.complete_unbonding("op-001")


def test_apply_slash(config, ledger):
    ledger.create_bond("op-001", OperatorRole.GPU, 5_000, config)
    ledger.confirm_bond("op-001")
    record = ledger.apply_slash("op-001", 1_000, "downtime")
    assert record.slash_total == 1_000
    assert record.effective_stake() == 4_000


def test_full_slash(config, ledger):
    ledger.create_bond("op-001", OperatorRole.GPU, 5_000, config)
    ledger.confirm_bond("op-001")
    record = ledger.apply_slash("op-001", 5_000, "equivocation")
    assert record.status == BondStatus.SLASHED
    assert record.effective_stake() == 0


def test_top_up(config, ledger):
    ledger.create_bond("op-001", OperatorRole.GPU, 5_000, config)
    ledger.confirm_bond("op-001")
    record = ledger.top_up("op-001", 3_000)
    assert record.amount == 8_000


def test_save_and_load(config, ledger):
    ledger.create_bond("op-001", OperatorRole.VALIDATOR, 10_000, config)
    ledger.confirm_bond("op-001")

    with tempfile.NamedTemporaryFile(suffix=".json", delete=False) as f:
        path = Path(f.name)

    ledger.save(path)
    loaded = BondLedger.load(path)

    bond = loaded.get_bond("op-001")
    assert bond is not None
    assert bond.status == BondStatus.BONDED
    assert bond.amount == 10_000
    path.unlink()


def test_history(config, ledger):
    ledger.create_bond("op-001", OperatorRole.GPU, 5_000, config)
    ledger.confirm_bond("op-001")
    ledger.apply_slash("op-001", 500, "test")
    assert len(ledger.history) == 3  # create + confirm + slash


def test_is_below_minimum(config, ledger):
    ledger.create_bond("op-001", OperatorRole.GPU, 1_500, config)
    ledger.confirm_bond("op-001")
    bond = ledger.get_bond("op-001")
    assert not bond.is_below_minimum(config)  # 1500 >= 1000
    ledger.apply_slash("op-001", 600, "test")
    assert bond.is_below_minimum(config)  # 900 < 1000 → below minimum
