"""Postgres-backed store for Swarm persistence
Provides a minimal compatible API with swarm.storage.sqlite_store for social drafts,
agent snapshots and event audit tables used by local dev and CI when POSTGRES_URL is set.
"""
import os
import json
import time
from typing import Dict, Any, List, Optional

import psycopg2
import psycopg2.extras

DSN = os.getenv('POSTGRES_URL', os.getenv('DATABASE_URL', ''))


def _get_conn():
    if not DSN:
        raise RuntimeError('POSTGRES_URL not configured')
    conn = psycopg2.connect(DSN)
    return conn


def init_db():
    conn = _get_conn()
    cur = conn.cursor()
    # Agents table
    cur.execute('''
    CREATE TABLE IF NOT EXISTS agents (
        agent_id TEXT PRIMARY KEY,
        serial_number TEXT,
        specialization TEXT,
        data_json JSONB,
        last_updated TIMESTAMP WITH TIME ZONE
    )
    ''')

    cur.execute('''
    CREATE TABLE IF NOT EXISTS births (
        id SERIAL PRIMARY KEY,
        agent_id TEXT,
        payload_json JSONB,
        timestamp TIMESTAMP WITH TIME ZONE DEFAULT now()
    )
    ''')

    cur.execute('''
    CREATE TABLE IF NOT EXISTS deaths (
        id SERIAL PRIMARY KEY,
        agent_id TEXT,
        payload_json JSONB,
        timestamp TIMESTAMP WITH TIME ZONE DEFAULT now()
    )
    ''')

    cur.execute('''
    CREATE TABLE IF NOT EXISTS social_drafts (
        draft_id TEXT PRIMARY KEY,
        payload_json JSONB,
        created_at TIMESTAMP WITH TIME ZONE DEFAULT now()
    )
    ''')

    cur.execute('''
    CREATE TABLE IF NOT EXISTS social_audit (
        id SERIAL PRIMARY KEY,
        event_type TEXT,
        payload_json JSONB,
        timestamp TIMESTAMP WITH TIME ZONE DEFAULT now()
    )
    ''')

    conn.commit()
    cur.close()
    conn.close()

    # If fallback file exists, try to import it once
    _try_import_fallback()


def _try_import_fallback():
    path = '/tmp/swarm_social_drafts.json'
    if not os.path.exists(path):
        return
    try:
        conn = _get_conn()
        cur = conn.cursor()
        with open(path) as f:
            data = json.load(f)
        for draft_id, payload in data.items():
            cur.execute("INSERT INTO social_drafts(draft_id, payload_json) VALUES (%s, %s) ON CONFLICT (draft_id) DO UPDATE SET payload_json = EXCLUDED.payload_json, created_at = now()", (draft_id, json.dumps(payload)))
        conn.commit()
        cur.close()
        conn.close()
        # remove fallback
        os.remove(path)
    except Exception:
        # don't fail on import issues
        pass


# Social drafts API

def init_social_tables():
    init_db()


def save_social_draft(draft_id: str, payload: Dict[str, Any]):
    conn = _get_conn()
    cur = conn.cursor()
    cur.execute("INSERT INTO social_drafts(draft_id, payload_json) VALUES (%s, %s) ON CONFLICT (draft_id) DO UPDATE SET payload_json = EXCLUDED.payload_json, created_at = now()", (draft_id, json.dumps(payload)))
    conn.commit()
    cur.close()
    conn.close()


def load_social_draft(draft_id: str) -> Optional[Dict[str, Any]]:
    conn = _get_conn()
    cur = conn.cursor(cursor_factory=psycopg2.extras.RealDictCursor)
    cur.execute('SELECT payload_json FROM social_drafts WHERE draft_id = %s', (draft_id,))
    row = cur.fetchone()
    cur.close()
    conn.close()
    if not row:
        return None
    return row['payload_json']


def list_social_drafts(limit: int = 50) -> List[Dict[str, Any]]:
    conn = _get_conn()
    cur = conn.cursor(cursor_factory=psycopg2.extras.RealDictCursor)
    cur.execute('SELECT payload_json FROM social_drafts ORDER BY created_at DESC LIMIT %s', (limit,))
    rows = cur.fetchall()
    cur.close()
    conn.close()
    return [r['payload_json'] for r in rows]


def append_social_audit(event_type: str, payload: Dict[str, Any]):
    conn = _get_conn()
    cur = conn.cursor()
    cur.execute('INSERT INTO social_audit(event_type, payload_json) VALUES (%s, %s)', (event_type, json.dumps(payload)))
    conn.commit()
    cur.close()
    conn.close()


# Agent persistence

def save_agent_snapshot(agent_id: str, serial_number: str, specialization: str, data: Dict[str, Any]):
    conn = _get_conn()
    cur = conn.cursor()
    cur.execute("INSERT INTO agents(agent_id, serial_number, specialization, data_json, last_updated) VALUES (%s,%s,%s,%s,now()) ON CONFLICT (agent_id) DO UPDATE SET serial_number=EXCLUDED.serial_number, specialization=EXCLUDED.specialization, data_json=EXCLUDED.data_json, last_updated=EXCLUDED.last_updated", (agent_id, serial_number, specialization, json.dumps(data)))
    conn.commit()
    cur.close()
    conn.close()


def load_all_agents() -> List[Dict[str, Any]]:
    conn = _get_conn()
    cur = conn.cursor(cursor_factory=psycopg2.extras.RealDictCursor)
    cur.execute('SELECT agent_id, serial_number, specialization, data_json FROM agents')
    rows = cur.fetchall()
    cur.close()
    conn.close()
    res = []
    for r in rows:
        payload = r['data_json'] or {}
        payload.update({'agent_id': r['agent_id'], 'serial_number': r['serial_number'], 'specialization': r['specialization']})
        res.append(payload)
    return res


def append_birth(agent_id: str, payload: Dict[str, Any]):
    conn = _get_conn()
    cur = conn.cursor()
    cur.execute('INSERT INTO births(agent_id, payload_json) VALUES (%s, %s)', (agent_id, json.dumps(payload)))
    conn.commit()
    cur.close()
    conn.close()


def append_death(agent_id: str, payload: Dict[str, Any]):
    conn = _get_conn()
    cur = conn.cursor()
    cur.execute('INSERT INTO deaths(agent_id, payload_json) VALUES (%s, %s)', (agent_id, json.dumps(payload)))
    conn.commit()
    cur.close()
    conn.close()


def load_birth_history(limit: int = 100) -> List[Dict[str, Any]]:
    conn = _get_conn()
    cur = conn.cursor(cursor_factory=psycopg2.extras.RealDictCursor)
    cur.execute('SELECT agent_id, payload_json, extract(epoch from timestamp) as ts FROM births ORDER BY id DESC LIMIT %s', (limit,))
    rows = cur.fetchall()
    cur.close()
    conn.close()
    res = []
    for r in rows:
        res.append({'agent_id': r['agent_id'], 'payload': r['payload_json'], 'timestamp': int(r['ts'])})
    return res


def load_death_history(limit: int = 100) -> List[Dict[str, Any]]:
    conn = _get_conn()
    cur = conn.cursor(cursor_factory=psycopg2.extras.RealDictCursor)
    cur.execute('SELECT agent_id, payload_json, extract(epoch from timestamp) as ts FROM deaths ORDER BY id DESC LIMIT %s', (limit,))
    rows = cur.fetchall()
    cur.close()
    conn.close()
    res = []
    for r in rows:
        res.append({'agent_id': r['agent_id'], 'payload': r['payload_json'], 'timestamp': int(r['ts'])})
    return res
