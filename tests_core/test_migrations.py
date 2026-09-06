import os

import pytest
from alembic import command
from alembic.config import Config
from swarm.db import SessionLocal, init_db


@pytest.fixture(scope="module")
def test_db():
    """Set up test database."""
    test_db_url = os.environ.get('DATABASE_URL', 'postgresql://postgres:postgres@localhost:5432/test_swarm')
    os.environ['DATABASE_URL'] = test_db_url
    # Initialize DB (create tables if needed, but migrations will handle)
    engine = init_db()
    yield engine
    # Cleanup if needed

def test_migrations(test_db):
    """Test that migrations can be applied and rolled back."""
    # Create alembic config
    alembic_cfg = Config("alembic.ini")
    alembic_cfg.set_main_option("sqlalchemy.url", os.environ['DATABASE_URL'])

    # Upgrade to head
    command.upgrade(alembic_cfg, "head")

    # Check that tables exist
    with SessionLocal():
        # Use SQLAlchemy inspector to check tables
        from sqlalchemy import inspect
        inspector = inspect(test_db)
        table_names = inspector.get_table_names()
        expected_tables = ['admins', 'allocations', 'audit_log', 'claims', 'consents', 'events', 'kyc_entries', 'pending_actions', 'payouts_finalized']
        for table in expected_tables:
            assert table in table_names, f"Table {table} not found"

    # Downgrade to base
    command.downgrade(alembic_cfg, "base")

    # Check that tables are gone
    with SessionLocal():
        inspector = inspect(test_db)
        table_names = inspector.get_table_names()
        for table in expected_tables:
            assert table not in table_names, f"Table {table} still exists after downgrade"
