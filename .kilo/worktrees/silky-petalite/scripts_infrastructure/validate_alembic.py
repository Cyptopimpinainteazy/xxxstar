#!/usr/bin/env python3
import re
import subprocess
import sys
from pathlib import Path

BASE_REF = "origin/main"

# fetch base branch to ensure we can diff against it
subprocess.run(["git", "fetch", "origin", "main"], check=False)

# list changed files against base
res = subprocess.run(["git", "diff", "--name-only", f"{BASE_REF}...HEAD"], capture_output=True, text=True)
changed = [s for s in res.stdout.splitlines() if s]

alembic_changes = [p for p in changed if p.startswith("alembic/versions/") and p.endswith('.py')]

if not alembic_changes:
    print("No Alembic migrations changed; nothing to check.")
    sys.exit(0)

pattern_rel = re.compile(r"relkind\s*=\s*'S'.*relname\s*=\s*'[^']+_id_seq'", re.DOTALL)
pattern_drop = re.compile(r"DROP\s+SEQUENCE", re.IGNORECASE)
pattern_marker = re.compile(r"#\s*sequence-guard", re.IGNORECASE)

failed = []
for path in alembic_changes:
    p = Path(path)
    if not p.exists():
        print(f"Warning: changed path {path} not found in workspace; skipping.")
        continue
    content = p.read_text()
    ok = False
    if pattern_rel.search(content):
        ok = True
    if pattern_drop.search(content):
        ok = True
    if pattern_marker.search(content):
        ok = True
    if not ok:
        failed.append(path)

if failed:
    print("The following Alembic migration files are missing the orphaned-sequence guard or marker comment (# sequence-guard):")
    for f in failed:
        print(" - ", f)
    print("\nPlease add a guard block like the following to avoid duplicate sequence errors during repeated test runs:")
    print(r'''
    op.execute("""
    DO $$
    BEGIN
        IF EXISTS (SELECT 1 FROM pg_class WHERE relkind='S' AND relname='reputation_events_id_seq')
           AND NOT EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name='reputation_events') THEN
            DROP SEQUENCE reputation_events_id_seq;
        END IF;
    END$$;
    """)
    # Or add a marker comment: # sequence-guard
    ''')
    sys.exit(1)

print("All changed Alembic migrations include an orphaned-sequence guard or marker.")
sys.exit(0)
