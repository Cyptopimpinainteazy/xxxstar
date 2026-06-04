#!/usr/bin/env python3
import subprocess
import sys

COMMANDS = [
    ["cargo", "test", "--workspace", "--", "--nocapture"],
]

def main() -> int:
    for cmd in COMMANDS:
        print(f"[invariant_guard] running: {' '.join(cmd)}")
        rc = subprocess.call(cmd)
        if rc != 0:
            print(f"[invariant_guard] failed: {' '.join(cmd)}")
            return rc
    print("[invariant_guard] ok")
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
