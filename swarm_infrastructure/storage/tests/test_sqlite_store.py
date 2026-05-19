import os
import tempfile
from swarm.storage import sqlite_store


def test_init_and_save_and_load(tmp_path):
    db = str(tmp_path / "agents.db")
    sqlite_store.init_db(db)

    agent_id = 'agent-test-1'
    payload = {'agent_id': agent_id, 'serial_number': 'serial-1', 'specialization': 'test', 'status': 'active'}

    sqlite_store.save_agent_snapshot(agent_id, payload['serial_number'], payload['specialization'], payload, db_path=db)
    agents = sqlite_store.load_all_agents(db_path=db)

    assert any(a['agent_id'] == agent_id for a in agents)

    # birth/death logging
    sqlite_store.append_birth(agent_id, {'reason': 'test_birth'}, db_path=db)
    sqlite_store.append_death(agent_id, {'reason': 'test_death'}, db_path=db)

    births = sqlite_store.load_birth_history(limit=10, db_path=db)
    deaths = sqlite_store.load_death_history(limit=10, db_path=db)

    assert len(births) >= 1
    assert len(deaths) >= 1
