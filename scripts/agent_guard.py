#!/usr/bin/env python3
import pathlib
import re

ROOT = pathlib.Path(__file__).resolve().parents[1]
MAX_FILE_BYTES = 1_000_000
IGNORE_DIRS = {
    ".git",
    "target",
    "node_modules",
    ".venv",
    "vendor",
    "vendov",
    "3d",
    "ChatGPT_files",
    "protoc_bin",
    "logs",
    ".toolchain",
}
SECRET_PATTERNS = [
    r"(?i)\b(private[_-]?key|mnemonic|api[_-]?key|rpc[_-]?key)\b\s*[:=]\s*['\"]?[^'\"\s]{8,}",
    r"AKIA[0-9A-Z]{16}",
    r"-----BEGIN (EC|RSA|OPENSSH) PRIVATE KEY-----",
]

def main() -> int:
    issues = []
    for p in ROOT.rglob("*"):
        if not p.is_file() or any(part in IGNORE_DIRS for part in p.parts):
            continue
        try:
            if p.stat().st_size > MAX_FILE_BYTES:
                continue
        except OSError:
            continue
        txt = p.read_text(encoding="utf-8", errors="ignore")
        for i, line in enumerate(txt.splitlines(), 1):
            if any(re.search(rx, line) for rx in SECRET_PATTERNS):
                issues.append(f"{p.relative_to(ROOT)}:{i}: {line.strip()}")
    if issues:
        print("[agent_guard] blocked: secret-like material detected")
        print("\n".join(issues[:200]))
        return 1
    print("[agent_guard] ok")
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
