#!/usr/bin/env python3
import json
import re
import sys

def deny(reason):
    print(json.dumps({
        "hookSpecificOutput": {
            "hookEventName": "PreToolUse",
            "permissionDecision": "deny",
            "permissionDecisionReason": reason
        }
    }))
    sys.exit(0)

try:
    data = json.load(sys.stdin)
except Exception:
    sys.exit(0)

tool = data.get("tool_name", "")
tool_input = data.get("tool_input", {}) or {}

if tool == "Bash":
    cmd = tool_input.get("command", "")

    blocked = [
        (r"\brm\s+-rf\s+/", "Blocked destructive root delete."),
        (r"\bsudo\s+rm\b", "Blocked sudo rm."),
        (r"\bmkfs\b", "Blocked filesystem format command."),
        (r"\bdd\s+if=", "Blocked raw disk command."),
        (r"\bgit\s+push\s+--force\b", "Blocked force push."),
        (r"\bgit\s+reset\s+--hard\b", "Blocked hard reset."),
        (r"\bgit\s+clean\s+-fdx\b", "Blocked destructive git clean."),
        (r"\bnpm\s+publish\b", "Blocked npm publish."),
        (r"\bcargo\s+publish\b", "Blocked cargo publish."),
    ]

    for pattern, reason in blocked:
        if re.search(pattern, cmd):
            deny(reason)

    cheat_patterns = [
        r"#\[ignore\]",
        r"\.skip\(",
        r"it\.skip",
        r"test\.skip",
        r"assert!\(true\)",
        r"expect\(true\)\.toBe\(true\)"
    ]

    if any(re.search(pattern, cmd) for pattern in cheat_patterns):
        deny("Blocked suspicious test-weakening command. Fix code, not tests.")

sys.exit(0)
