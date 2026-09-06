import json

import pytest
from swarm.db import SessionLocal
from tools.json_to_db import import_json_to_db


@pytest.fixture
def sample_json_files(tmp_path):
    """Create sample JSON files for testing."""
    # Create out directory
    out_dir = tmp_path / "out"
    out_dir.mkdir()

    # Sample data
    admins = [{"address": "0x123", "added_at": "2025-01-01T00:00:00"}]
    consents = [{"contributor_id": "user1", "wallet": "0x456", "kyc": True}]
    claims = [{"contributor_id": "user1", "wallet": "0x456", "amount": 1000, "status": "queued"}]

    # Write files
    (out_dir / "admins.json").write_text(json.dumps(admins))
    (out_dir / "consents.json").write_text(json.dumps(consents))
    (out_dir / "claims.json").write_text(json.dumps(claims))

    return str(out_dir)

def test_import_json_to_db(sample_json_files):
    """Test importing JSON files to DB."""
    # Import
    import_json_to_db(sample_json_files)

    # Check data
    with SessionLocal() as session:
        # Check admins
        admins = session.execute("SELECT address FROM admins").fetchall()
        assert len(admins) == 1
        assert admins[0][0] == "0x123"

        # Check consents
        consents = session.execute("SELECT contributor_id, wallet, kyc FROM consents").fetchall()
        assert len(consents) == 1
        assert consents[0] == ("user1", "0x456", True)

        # Check claims
        claims = session.execute("SELECT contributor_id, wallet, amount, status FROM claims").fetchall()
        assert len(claims) == 1
        assert claims[0] == ("user1", "0x456", 1000, "queued")
