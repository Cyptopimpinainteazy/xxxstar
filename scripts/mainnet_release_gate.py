#!/usr/bin/env python3
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]

REQUIRED_DOCS = [
    "MAINNET_READINESS.md",
    "INVARIANTS.md",
    "RELEASE_GATES.md",
    "SECURITY.md",
    "TESTING.md",
    "AUDIT_SPEC.md",
]

def has_forbidden_secrets() -> bool:
    assignment_re = re.compile(r"(?m)^\s*(?:export\s+)?(?:PRIVATE_KEY|MNEMONIC)\s*=\s*([^\s#]+)")
    aws_key_re = re.compile(r"AKIA[0-9A-Z]{16}")
    example_value_prefixes = ("replace_", "your_", "<", "$")
    ignored_dirs = {".git", "target", "node_modules", ".venv", ".cocoindex_code"}
    for p in ROOT.rglob("*"):
        if not p.is_file() or any(x in p.parts for x in ignored_dirs):
            continue
        txt = p.read_text(encoding="utf-8", errors="ignore")
        has_secret_assignment = any(
            not match.group(1).strip("\"'").lower().startswith(example_value_prefixes)
            for match in assignment_re.finditer(txt)
        )
        if has_secret_assignment or aws_key_re.search(txt):
            print(f"[mainnet_release_gate] secret-like token found: {p.relative_to(ROOT)}")
            return True
    return False

def main() -> int:
    missing = [d for d in REQUIRED_DOCS if not (ROOT / d).exists()]
    if missing:
        print("[mainnet_release_gate] missing required docs:")
        for m in missing:
            print(f" - {m}")
        return 1
    if has_forbidden_secrets():
        return 1
    print("[mainnet_release_gate] ok")
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
