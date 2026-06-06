#!/usr/bin/env python3
import json
import os
import subprocess
import sys

try:
    data = json.load(sys.stdin)
except Exception:
    sys.exit(0)

cwd = data.get("cwd") or os.getcwd()
tool_input = data.get("tool_input", {}) or {}
path = tool_input.get("file_path") or tool_input.get("path") or ""

def run(cmd):
    try:
        out = subprocess.check_output(
            cmd,
            cwd=cwd,
            shell=True,
            text=True,
            stderr=subprocess.STDOUT,
            timeout=60
        )
        return True, out[-2000:]
    except Exception as e:
        return False, str(e)

messages = []

if path.endswith(".rs") and os.path.exists(os.path.join(cwd, "Cargo.toml")):
    ok, out = run("cargo fmt --all")
    messages.append(f"cargo fmt --all => {'ok' if ok else 'failed'}\n{out}")

if messages:
    print(json.dumps({
        "additionalContext": "X3 post-edit hook result:\n" + "\n".join(messages)
    }))
