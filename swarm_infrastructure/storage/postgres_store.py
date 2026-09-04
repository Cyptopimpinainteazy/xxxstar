"""Postgres persistence layer for social drafts and audits.

Uses psycopg2 connection. Controlled by environment variables:
 - POSTGRES_URL (preferred) or PGHOST/PGPORT/PGUSER/PGPASSWORD/PGDATABASE

API mirrors the sqlite_store minimal interface used by the swarm server.
"""
import os
import json
import time
import logging
from typing import Dict, Any, List, Optional

import psycopg2
from psycopg2.extras import Json, DictCursor

logger = logging.getLogger(__name__)

DEFAULT_DB_URL = os.environ.get('POSTGRES_URL')


def _get_conn():
    url = DEFAULT_DB_URL
    if not url:
        host = os.environ.get('PGHOST', 'localhost')
        port = os.environ.get('PGPORT', '5432')
        user = os.environ.get('PGUSER', 'x3')
        password = os.environ.get('PGPASSWORD', 'x3')
        db = os.environ.get('PGDATABASE', 'swarm')
        url = f"postgresql://{user}:{password}@{host}:{port}/{db}"
    return psycopg2.connect(url)


def init_db():
    conn = _get_conn()
    cur = conn.cursor()

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


def save_social_draft(draft_id: str, payload: Dict[str, Any]):
    conn = _get_conn()
    cur = conn.cursor()
    cur.execute('''
    INSERT INTO social_drafts(draft_id, payload_json, created_at)
    VALUES (%s, %s, now())
    ON CONFLICT (draft_id) DO UPDATE SET payload_json = EXCLUDED.payload_json, created_at = now()
    ''', (draft_id, Json(payload)))
    conn.commit()
    cur.close()
    conn.close()


def load_social_draft(draft_id: str) -> Optional[Dict[str, Any]]:
    conn = _get_conn()
    cur = conn.cursor(cursor_factory=DictCursor)
    cur.execute('SELECT payload_json FROM social_drafts WHERE draft_id = %s', (draft_id,))
    row = cur.fetchone()
    cur.close()
    conn.close()
    if not row:
        return None
    return row['payload_json']


def list_social_drafts(limit: int = 50) -> List[Dict[str, Any]]:
    conn = _get_conn()
    cur = conn.cursor(cursor_factory=DictCursor)
    cur.execute('SELECT payload_json FROM social_drafts ORDER BY created_at DESC LIMIT %s', (limit,))
    rows = cur.fetchall()
    cur.close()
    conn.close()
    return [r['payload_json'] for r in rows]


def append_social_audit(event_type: str, payload: Dict[str, Any]):
    conn = _get_conn()
    cur = conn.cursor()
    cur.execute('INSERT INTO social_audit(event_type, payload_json, timestamp) VALUES (%s, %s, now())', (event_type, Json(payload)))
    conn.commit()
    cur.close()
    conn.close()


def migrate_from_fallback(path: str = '/tmp/swarm_social_drafts.json') -> int:
    """Migrate drafts from fallback JSON into Postgres. Returns number migrated."""
    if not os.path.exists(path):
        return 0
    try:
        with open(path) as f:
            data = json.load(f)
    except Exception as e:
        logger.warning(f"Failed to read fallback for migration: {e}")
        return 0

    count = 0
    for draft_id, payload in data.items():
        try:
            save_social_draft(draft_id, payload)
            count += 1
        except Exception as e:
            logger.warning(f"Failed to migrate draft {draft_id}: {e}")
    # On success, remove fallback
    try:
        os.remove(path)
    except Exception:
        pass
    return count