from swarm.db import SessionLocal, models

from tools.process_finalized_payouts import process


def test_process_finalized(tmp_path, monkeypatch):
    # insert finalized payout into DB
    session = SessionLocal()
    try:
        session.add(models.PayoutFinalized(action_id="claim-alice-1", contributor_id="alice_0", wallet="0xabc", amount=100))
        session.commit()
    finally:
        session.close()

    # run process (eth-tester)
    process()
    # check that event was added to DB
    session = SessionLocal()
    try:
        events = session.query(models.Event).filter_by(type='payouts_processed').all()
        assert len(events) == 1
        assert events[0].payload['count'] == 1
    finally:
        session.close()
