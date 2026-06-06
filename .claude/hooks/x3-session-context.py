#!/usr/bin/env python3
import json
import os
import subprocess
import sys

try:
    data = json.load(sys.stdin)
except Exception:
    data = {}

cwd = data.get("cwd") or os.getcwd()

def run(cmd):
    try:
        return subprocess.check_output(
            cmd,
            cwd=cwd,
            shell=True,
            text=True,
            stderr=subprocess.STDOUT,
            timeout=5
        ).strip()
    except Exception as e:
        return f"unavailable: {e}"

branch = run("git rev-parse --abbrev-ref HEAD")
status = run("git status --short | head -40")

context = f"""
X3 session context loaded.

Project focus:
- X3 Atomic Star / X3 Chain.
- x3-lang is the language layer for intents, cross-VM calls, atomic routes, settlement, rollback, replay protection, expiry, and adapter safety.
- Treat code as source of truth. Markdown plans are secondary.
- Never claim COMPLETE without running verification.

Git:
- branch: {branch}
- status:
{status if status else "clean"}

Required behavior:
- Fix the real implementation.
- Do not weaken tests.
- Do not hide unfinished code behind docs.
- Do not use stubs/mocks outside test-only paths.
"""

print(json.dumps({
    "hookSpecificOutput": {
        "hookEventName": "SessionStart",
        "additionalContext": context,
        "sessionTitle": "X3 build session"
    }
}))
