"""Simple SQLite persistence layer for agent registry and event history

This module provides a minimal, dependency-free persistence backend that
allows the AgentRegistry to persist births, deaths and agent snapshots.

Note: This implementation is intentionally lightweight for fast iteration
and local dev. For production, swap out with Postgres or another durable store.
"""

import sqlite3
import json
import os
from typing import Dict, Any, List, Optional

DEFAULT_DB_PATH = os.environ.get('AGENT_DB_PATH', './data/agent_registry.db')

def _get_conn(db_path: str = DEFAULT_DB_PATH):
    os.makedirs(os.path.dirname(db_path), exist_ok=True)
    conn = sqlite3.connect(db_path, timeout=5)
    conn.row_factory = sqlite3.Row
    return conn

def init_db(db_path: str = DEFAULT_DB_PATH):
    conn = _get_conn(db_path)
    cur = conn.cursor()

    # Agents table: stores serialized agent record JSON
    cur.execute('''
    CREATE TABLE IF NOT EXISTS agents (
        agent_id TEXT PRIMARY KEY,
        serial_number TEXT,
        specialization TEXT,
        data_json TEXT,
        last_updated INTEGER
    )
    ''')

    # Births and deaths (append-only event logs)
    cur.execute('''
    CREATE TABLE IF NOT EXISTS births (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        agent_id TEXT,
        payload_json TEXT,
        timestamp INTEGER
    )
    ''')

    cur.execute('''
    CREATE TABLE IF NOT EXISTS deaths (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        agent_id TEXT,
        payload_json TEXT,
        timestamp INTEGER
    )
    ''')

    # Performance history
    cur.execute('''
    CREATE TABLE IF NOT EXISTS performance_history (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        agent_id TEXT,
        ts INTEGER,
        score REAL
    )
    ''')

    conn.commit()
    conn.close()


def init_social_tables(db_path: str = DEFAULT_DB_PATH):
    conn = _get_conn(db_path)
    cur = conn.cursor()

    cur.execute('''
    CREATE TABLE IF NOT EXISTS social_drafts (
        draft_id TEXT PRIMARY KEY,
        payload_json TEXT,
        created_at INTEGER
    )
    ''')

    cur.execute('''
    CREATE TABLE IF NOT EXISTS social_audit (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        event_type TEXT,
        payload_json TEXT,
        timestamp INTEGER
    )
    ''')

    conn.commit()
    conn.close()


def save_social_draft(draft_id: str, payload: Dict[str, Any], db_path: str = DEFAULT_DB_PATH):
    conn = _get_conn(db_path)
    cur = conn.cursor()
    cur.execute('''
    INSERT INTO social_drafts(draft_id, payload_json, created_at)
    VALUES (?, ?, strftime('%s','now'))
    ON CONFLICT(draft_id) DO UPDATE SET
      payload_json=excluded.payload_json,
      created_at=strftime('%s','now')
    ''', (draft_id, json.dumps(payload)))
    conn.commit()
    conn.close()


def load_social_draft(draft_id: str, db_path: str = DEFAULT_DB_PATH) -> Optional[Dict[str, Any]]:
    conn = _get_conn(db_path)
    cur = conn.cursor()
    cur.execute('SELECT payload_json FROM social_drafts WHERE draft_id = ?', (draft_id,))
    row = cur.fetchone()
    conn.close()
    if not row:
        return None
    return json.loads(row['payload_json']) if row['payload_json'] else {}


def list_social_drafts(limit: int = 50, db_path: str = DEFAULT_DB_PATH) -> List[Dict[str, Any]]:
    conn = _get_conn(db_path)
    cur = conn.cursor()
    cur.execute('SELECT payload_json FROM social_drafts ORDER BY created_at DESC LIMIT ?', (limit,))
    rows = cur.fetchall()
    conn.close()
    return [json.loads(r['payload_json']) if r['payload_json'] else {} for r in rows]


def append_social_audit(event_type: str, payload: Dict[str, Any], db_path: str = DEFAULT_DB_PATH):
    conn = _get_conn(db_path)
    cur = conn.cursor()
    cur.execute(
        'INSERT INTO social_audit(event_type, payload_json, timestamp) VALUES (?, ?, strftime(\"%s\",\"now\"))',
        (event_type, json.dumps(payload)),
    )
    conn.commit()
    conn.close()

def save_agent_snapshot(agent_id: str, serial_number: str, specialization: str, data: Dict[str, Any], db_path: str = DEFAULT_DB_PATH):
    conn = _get_conn(db_path)
    cur = conn.cursor()
    cur.execute('''
    INSERT INTO agents(agent_id, serial_number, specialization, data_json, last_updated)
    VALUES (?, ?, ?, ?, strftime('%s','now'))
    ON CONFLICT(agent_id) DO UPDATE SET
      serial_number=excluded.serial_number,
      specialization=excluded.specialization,
      data_json=excluded.data_json,
      last_updated=strftime('%s','now')
    ''', (agent_id, serial_number, specialization, json.dumps(data)))
    conn.commit()
    conn.close()

def delete_agent(agent_id: str, db_path: str = DEFAULT_DB_PATH):
    conn = _get_conn(db_path)
    cur = conn.cursor()
    cur.execute('DELETE FROM agents WHERE agent_id = ?', (agent_id,))
    conn.commit()
    conn.close()

def load_all_agents(db_path: str = DEFAULT_DB_PATH) -> List[Dict[str, Any]]:
    conn = _get_conn(db_path)
    cur = conn.cursor()
    cur.execute('SELECT agent_id, serial_number, specialization, data_json FROM agents')
    rows = cur.fetchall()
    result = []
    for r in rows:
        payload = json.loads(r['data_json']) if r['data_json'] else {}
        payload.update({'agent_id': r['agent_id'], 'serial_number': r['serial_number'], 'specialization': r['specialization']})
        result.append(payload)
    conn.close()
    return result

def append_birth(agent_id: str, payload: Dict[str, Any], db_path: str = DEFAULT_DB_PATH):
    conn = _get_conn(db_path)
    cur = conn.cursor()
    cur.execute('INSERT INTO births(agent_id, payload_json, timestamp) VALUES (?, ?, strftime("%s","now"))', (agent_id, json.dumps(payload)))
    conn.commit()
    conn.close()

def append_death(agent_id: str, payload: Dict[str, Any], db_path: str = DEFAULT_DB_PATH):
    conn = _get_conn(db_path)
    cur = conn.cursor()
    cur.execute('INSERT INTO deaths(agent_id, payload_json, timestamp) VALUES (?, ?, strftime("%s","now"))', (agent_id, json.dumps(payload)))
    conn.commit()
    conn.close()

def load_birth_history(limit: int = 100, db_path: str = DEFAULT_DB_PATH) -> List[Dict[str, Any]]:
    conn = _get_conn(db_path)
    cur = conn.cursor()
    cur.execute('SELECT agent_id, payload_json, timestamp FROM births ORDER BY id DESC LIMIT ?', (limit,))
    rows = cur.fetchall()
    res = []
    for r in rows:
        res.append({'agent_id': r['agent_id'], 'payload': json.loads(r['payload_json']), 'timestamp': r['timestamp']})
    conn.close()
    return res

def load_death_history(limit: int = 100, db_path: str = DEFAULT_DB_PATH) -> List[Dict[str, Any]]:
    conn = _get_conn(db_path)
    cur = conn.cursor()
    cur.execute('SELECT agent_id, payload_json, timestamp FROM deaths ORDER BY id DESC LIMIT ?', (limit,))
    rows = cur.fetchall()
    res = []
    for r in rows:
        res.append({'agent_id': r['agent_id'], 'payload': json.loads(r['payload_json']), 'timestamp': r['timestamp']})
    conn.close()
    return res
