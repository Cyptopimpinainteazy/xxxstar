"""Tests for x3_operator.identity"""

import tempfile
from pathlib import Path

from x3_operator.config import OperatorRole
from x3_operator.identity import (
    collect_hardware_attestation,
    generate_operator_identity,
    load_operator_identity,
)


def test_hardware_attestation():
    hw = collect_hardware_attestation()
    assert hw.hostname
    assert hw.os_name
    assert hw.cpu_count > 0
    assert hw.machine_id


def test_fingerprint_deterministic():
    hw = collect_hardware_attestation()
    fp1 = hw.fingerprint()
    fp2 = hw.fingerprint()
    assert fp1 == fp2
    assert len(fp1) == 64  # SHA-256 hex


def test_generate_and_load():
    with tempfile.TemporaryDirectory() as tmpdir:
        identity = generate_operator_identity(OperatorRole.VALIDATOR, tmpdir)
        assert identity.operator_id
        assert identity.pubkey
        assert identity.role == "validator"
        assert len(identity.operator_id) == 32  # blake2b digest_size=16 hex

        loaded = load_operator_identity(tmpdir)
        assert loaded is not None
        assert loaded.operator_id == identity.operator_id
        assert loaded.pubkey == identity.pubkey


def test_deterministic_operator_id():
    """Same machine + same role → same operator_id."""
    with tempfile.TemporaryDirectory() as dir1, tempfile.TemporaryDirectory() as dir2:
        id1 = generate_operator_identity("gpu", dir1)
        id2 = generate_operator_identity("gpu", dir2)
        # Same machine, same role → same operator_id
        assert id1.operator_id == id2.operator_id


def test_different_roles_different_ids():
    with tempfile.TemporaryDirectory() as dir1, tempfile.TemporaryDirectory() as dir2:
        id_val = generate_operator_identity("validator", dir1)
        id_gpu = generate_operator_identity("gpu", dir2)
        assert id_val.operator_id != id_gpu.operator_id


def test_registration_hash():
    with tempfile.TemporaryDirectory() as tmpdir:
        identity = generate_operator_identity("validator", tmpdir)
        h = identity.to_registration_hash()
        assert len(h) == 64  # blake2b digest_size=32 hex


def test_load_nonexistent():
    result = load_operator_identity("/tmp/nonexistent_x3_path_12345")
    assert result is None


def test_key_file_permissions():
    with tempfile.TemporaryDirectory() as tmpdir:
        generate_operator_identity("validator", tmpdir)
        key_path = Path(tmpdir) / "operator.key"
        assert key_path.exists()
        import stat
        mode = key_path.stat().st_mode
        assert not (mode & stat.S_IROTH)  # no world read
        assert not (mode & stat.S_IWOTH)  # no world write
