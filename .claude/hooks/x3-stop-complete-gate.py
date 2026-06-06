#!/usr/bin/env python3
import json
import os
import re
import sys

try:
    data = json.load(sys.stdin)
except Exception:
    sys.exit(0)

cwd = data.get("cwd") or os.getcwd()
last = data.get("last_assistant_message", "") or ""
already_active = data.get("stop_hook_active", False)

verify_stamp = os.path.join(cwd, ".claude/state/x3-last-verify.ok")

claims_complete = bool(re.search(
    r"(<promise>COMPLETE</promise>|\bCOMPLETE\b|mainnet[- ]ready|fully implemented|all tests pass)",
    last,
    re.IGNORECASE
))

if claims_complete and not os.path.exists(verify_stamp) and not already_active:
    print(json.dumps({
        "decision": "block",
        "reason": """
You claimed completion without an X3 verification stamp.

Before stopping:
1. Run ./scripts/x3-verify.sh
2. Fix every failure.
3. Only then output <promise>COMPLETE</promise>.
"""
    }))
