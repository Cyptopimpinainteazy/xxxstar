#!/usr/bin/env python3
import json
import os
import re
import subprocess
import sys

try:
    data = json.load(sys.stdin)
except Exception:
    data = {}

cwd = data.get("cwd") or os.getcwd()

try:
    diff = subprocess.check_output(
        "git diff -- '*.rs' '*.ts' '*.tsx' '*.js' '*.jsx' ':!target/**' ':!node_modules/**'",
        cwd=cwd,
        shell=True,
        text=True,
        stderr=subprocess.DEVNULL,
        timeout=10
    )
except Exception:
    sys.exit(0)

bad = [
    r"^\+\s*#\[ignore\]",
    r"^\+\s*\.skip\(",
    r"^\+\s*it\.skip",
    r"^\+\s*test\.skip",
    r"^\+\s*assert!\(true\)",
    r"^\+\s*expect\(true\)\.toBe\(true\)"
]

if any(re.search(pattern, diff, re.MULTILINE) for pattern in bad):
    print(json.dumps({
        "decision": "block",
        "reason": "X3 anti-cheat guard triggered. Do not weaken tests. Fix the implementation."
    }))
