"""Tests for x3_operator.cli"""

import tempfile
from pathlib import Path

import pytest

from x3_operator.cli import main


@pytest.fixture
def data_dir():
    with tempfile.TemporaryDirectory() as tmpdir:
        yield tmpdir


def test_doctor(data_dir):
    code = main(["--data-dir", data_dir, "doctor"])
    assert code == 0


def test_init(data_dir):
    code = main(["--data-dir", data_dir, "init", "--role", "validator", "--network", "devnet"])
    assert code == 0
    assert (Path(data_dir) / "config.json").exists()
    assert (Path(data_dir) / "identity.json").exists()


def test_bond(data_dir):
    main(["--data-dir", data_dir, "init", "--role", "gpu"])
    code = main(["--data-dir", data_dir, "bond", "5000"])
    assert code == 0
    assert (Path(data_dir) / "bonds.json").exists()


def test_status_after_init(data_dir):
    main(["--data-dir", data_dir, "init", "--role", "validator"])
    code = main(["--data-dir", data_dir, "status"])
    assert code == 0


def test_status_not_initialized(data_dir):
    code = main(["--data-dir", data_dir, "status"])
    assert code == 1


def test_simulate(data_dir):
    code = main(["--data-dir", data_dir, "simulate"])
    assert code == 0


def test_simulate_whale(data_dir):
    code = main(["--data-dir", data_dir, "simulate", "--attack", "whale"])
    assert code == 0


def test_genesis(data_dir):
    main(["--data-dir", data_dir, "init", "--role", "validator"])
    code = main(["--data-dir", data_dir, "genesis"])
    assert code == 0
    assert (Path(data_dir) / "chain-spec.json").exists()


def test_exit_no_bond(data_dir):
    code = main(["--data-dir", data_dir, "exit-op"])
    assert code == 1


def test_full_lifecycle(data_dir):
    """init → bond → status → genesis → exit"""
    assert main(["--data-dir", data_dir, "init", "--role", "gpu"]) == 0
    assert main(["--data-dir", data_dir, "bond", "5000"]) == 0
    assert main(["--data-dir", data_dir, "status"]) == 0
    assert main(["--data-dir", data_dir, "genesis"]) == 0
    assert main(["--data-dir", data_dir, "exit-op"]) == 0
