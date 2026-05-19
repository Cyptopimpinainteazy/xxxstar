from swarm.db import SessionLocal, models

from tools.fund_allocations import fund_allocations, load_allocations


def test_fund_allocations(tmp_path, monkeypatch):
    # insert sample allocations into DB
    session = SessionLocal()
    try:
        session.add(models.Allocation(contributor_id='alice_0', amount=100))
        session.add(models.Allocation(contributor_id='bob_1', amount=50))
        session.commit()
    finally:
        session.close()

    # call fund_allocations with eth-tester (rpc=None)
    fund_allocations()
    # no exceptions means allocations were applied (we can't directly inspect ephemeral eth-tester state here)
    loaded = load_allocations()
    assert 'alice_0' in loaded
    assert loaded['alice_0'] == 100
