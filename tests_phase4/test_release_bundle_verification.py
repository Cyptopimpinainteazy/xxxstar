import subprocess
import tarfile
import tempfile
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
OUTER_CHECKSUMS = ROOT / "CHECKSUMS.sha256"
RELEASE_TARBALL = ROOT / "x3-chain-v1.1-release.tar.gz"


def test_outer_checksums_track_only_distributable_tarball():
    lines = [line.strip() for line in OUTER_CHECKSUMS.read_text(encoding="utf-8").splitlines() if line.strip()]

    assert len(lines) == 1
    assert lines[0].endswith("  x3-chain-v1.1-release.tar.gz")
    assert "target/" not in lines[0]


def test_extracted_bundle_contains_self_verifying_checksum_manifest():
    with tempfile.TemporaryDirectory() as temp_dir:
        with tarfile.open(RELEASE_TARBALL, "r:gz") as tar:
            tar.extractall(temp_dir)

        extracted_root = Path(temp_dir)
        bundle_checksum = next(extracted_root.rglob("CHECKSUMS.bundle.sha256"), None)

        assert bundle_checksum is not None

        result = subprocess.run(
            ["sha256sum", "--check", bundle_checksum.name],
            cwd=bundle_checksum.parent,
            capture_output=True,
            text=True,
            check=False,
        )

        assert result.returncode == 0, result.stdout + result.stderr