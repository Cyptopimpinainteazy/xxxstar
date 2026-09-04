#!/usr/bin/env python3
"""
X3 Chain — LLM Endpoints Seeder
Seeds discovered LLM endpoints into the infrastructure database from ollama_recon_results.json.
"""

import json
import sqlite3
from datetime import datetime
from pathlib import Path

SCRIPT_DIR = Path(__file__).parent
DB_DIR = SCRIPT_DIR.parent
DEFAULT_DB = DB_DIR / "chains.db"
RESULTS_JSON = Path("/home/lojak/Desktop/x3-chain-master/llm_recon_results.json")

def seed_llm_endpoints(db_path: str = str(DEFAULT_DB)):
    conn = sqlite3.connect(db_path)
    cursor = conn.cursor()

    # Apply schema if not present
    with open(SCRIPT_DIR.parent / "schema.sql") as f:
        conn.executescript(f.read())

    with open(RESULTS_JSON, 'r') as f:
        endpoints = json.load(f)

    inserted = 0
    for ep in endpoints:
        url = f"http://{ep['ip']}:{ep['port']}"
        models_json = json.dumps(ep['models'])
        last_checked = datetime.now().isoformat()

        cursor.execute("""
            INSERT OR IGNORE INTO llm_endpoints
            (url, provider, is_healthy, latency_ms, last_checked, models, version, source)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        """, (
            url, ep['platform'].lower(), 1, ep['response_time_ms'], last_checked,
            models_json, ep['version'], ep['source']
        ))
        if cursor.rowcount > 0:
            inserted += 1

    conn.commit()
    conn.close()

    print(f"Inserted {inserted} LLM endpoints into {db_path}")

if __name__ == "__main__":
    seed_llm_endpoints()