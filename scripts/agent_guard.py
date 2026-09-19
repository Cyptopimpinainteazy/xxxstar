#!/usr/bin/env python3
import pathlib
import re
import signal
import subprocess

ROOT = pathlib.Path(__file__).resolve().parents[1]
MAX_FILE_BYTES = 1_000_000
# A single tracked file could otherwise stall the scan forever: reading a binary
# file line-by-line feeds multi-megabyte "lines" of NUL bytes to the patterns,
# which is exactly the input that makes an ambiguous regex backtrack
# exponentially. Large minified text is scanned, but always under a watchdog.
REGEX_BUDGET_SECONDS = 300
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
IGNORE_PREFIXES = (
    ".kilo/",
    "apps/x3-desktop/src-tauri/tauri-vendor/",
    "forge-std/",
    "packages/polkawallet-plugin/dist/",
    "proof/reports/",
    "tests/phase_core/security/lib/forge-std/",
)
SECRET_PATTERNS = [
    r"(?i)\b(private[_-]?key|mnemonic|api[_-]?key|rpc[_-]?key)\b\s*[:=]\s*['\"]?[^'\"\s]{8,}",
    r"AKIA[0-9A-Z]{16}",
    r"-----BEGIN (EC|RSA|OPENSSH) PRIVATE KEY-----",
]
ALLOW_LINE_PATTERNS = [
    r"(?i)\b(api[_-]?key|rpc[_-]?key)\b\s*[:=]\s*os\.environ\.",
    r"(?i)\b(api[_-]?key|rpc[_-]?key)\b\s*[:=]\s*std::env::",
    r"(?i)\b(api[_-]?key|rpc[_-]?key)\b\s*[:=]\s*env\.",
    r"(?i)\bapi[_-]?key\b\s*[:=]\s*(request\.headers\.get|extractApiKey\(|options\.|parsed\.|creds\.|localStorage\.|this\.|newApiKey|api_key\b)",
    r"(?i)\bapi[_-]?key\b\s*[:=]\s*data\.get\(",
    r"(?i)\b(private[_-]?key|mnemonic|api[_-]?key|rpc[_-]?key)\b\s*:\s*(pub\s+)?(Option<)?String",
    r"(?i)\b(private[_-]?key|mnemonic|api[_-]?key|rpc[_-]?key)\b\s*:\s*(string|Promise|PQPrivateKey|pkcs8::PrivateKeyInfo|generatedMnemonic|mnemonic_str|Some\(|\"test-key\"|\"0x1234567890)",
    r"(?i)\b(private[_-]?key|mnemonic|api[_-]?key|rpc[_-]?key)\b\s*=\s*(secrets\.|ec\.generate_|Mnemonic::|PQPrivateKey\(|request\.|creds\.|CREDS_STORE\.getItem\(|localStorage\.|bytes\(|jury_authority_private_key|uint256\(keccak256|vm\.deriveKey|format!\()",
    r"(?i)\bapiKey\b\s*:\s*('sk_key_[a-z]'|'sk_owner'|'tampered_key')",
    r"(?i)\bapiKey\b\s*=\s*`sk_x3_\$\{randomBytes",
    r"(?i)\bapiKey\b\s*=\s*apiKeyValidation\.error",
    r"(?i)X-API-Key:\\?\s*(\$|\$\{|<|\[|infra_xxxxx)",
    r"(?i)X-API-Key:\\?\s*\\\$INFRA_API_KEY",
    r"(?i)api-key=(\$|\$\{|<|\[|YOUR_)",
    r"(?i)apiKey=(\$|\$\{|<|\[|sk_x3_test_bootstrap)",
    r"(?i)API_KEY\s*=\s*(process\.env|os\.environ|\$INFRA_API_KEY|\"infra_x+\"|your-secret-api-key)",
    r'(?i)API_KEY\s*=\s*"\$\{[A-Z0-9_]+:-\}"',
    r"(?i)mnemonic\s*=\s*bip39::Mnemonic::from_phrase\(\s*seed_phrase\s*,",
    # The bip39 2.x replacement for the line above: `parse_in(Language, phrase)`
    # parses a caller-supplied phrase and hardcodes nothing. The old allow entry
    # stopped matching when the crate moved to 2.x, which turned a legitimate
    # constructor call into "secret-like material".
    r"(?i)mnemonic\s*=\s*bip39::Mnemonic::parse_in\(",
    r"(?i)apiKey\s*:\s*'your-api-key'",
    r"(?i)apiKey\s*:\s*config\.(apiKey|privateKey)",
    r"(?i)this\.config\.apiKey\s*=\s*(token|undefined)\b",
    r"(?i)hardcoded `PRIVATE_KEY=`/`MNEMONIC=`/AWS-key patterns",
    r"(?i)APIKey\s*=\s*\"infra_x+\"",
    r"(?i)PRIVATE_KEY=\d{16,}",
    r"(?i)--from-literal=api-key=sk-or-\.\.\.",
    r"(?i)\$\{API_KEY:0:30\}",
    r'''(?i)privateKey:\s*["']•+''',
    r"(?i)private[_-]?key\s*[:=]\s*(self\.private_key|\"//Charlie\"|private_key\.clone\(\))",
    r"(?i)mnemonic\s*=\s*self\.decrypt_data\(",
    r"(?i)key\.privateKey\b",
    r'''(?i)\bAPI_KEY\s*=\s*"\$\{[A-Z0-9_]+:-\}"''',
    r'''(?i)\bapiKey\s*:\s*[\'"]your-[^\'"]+[\'"]''',
    r"(?i)\bapiKey\s*:\s*config\.[A-Za-z_][A-Za-z0-9_]*",
    r"(?i)\bthis\.config\.apiKey\s*=\s*undefined",
    r"(?i)\bmnemonic\s*=\s*bip39::Mnemonic::from_phrase\(",
    r"(?i)hardcoded\s+`?PRIVATE_KEY=`?/`?MNEMONIC=`?/AWS-key",
]


def is_ignored_path(path: pathlib.Path) -> bool:
    rel = path.relative_to(ROOT).as_posix()
    if rel == "scripts/agent_guard.py":
        return True
    return any(part in IGNORE_DIRS for part in path.parts) or any(
        rel.startswith(prefix) for prefix in IGNORE_PREFIXES
    )


def is_allowed_line(line: str) -> bool:
    return any(rx.search(line) for rx in _ALLOW_LINE_RE)


# Compiled once: `re.search` recompiles via a cache hit per call, which is the
# dominant cost when scanning tens of thousands of lines.
_SECRET_RE = [re.compile(rx) for rx in SECRET_PATTERNS]
_ALLOW_LINE_RE = [re.compile(rx) for rx in ALLOW_LINE_PATTERNS]


def looks_binary(raw: bytes) -> bool:
    """NUL bytes in the first block mark generated/binary data, not source."""
    return b"\x00" in raw[:8192]


class RegexBudgetExceeded(Exception):
    pass


def _on_budget_exceeded(signum, frame):  # pragma: no cover - signal path
    raise RegexBudgetExceeded()


def iter_tracked_files():
    result = subprocess.run(
        ["git", "ls-files"],
        cwd=ROOT,
        check=True,
        text=True,
        capture_output=True,
    )
    for rel in result.stdout.splitlines():
        yield ROOT / rel

def main() -> int:
    issues = []
    signal.signal(signal.SIGALRM, _on_budget_exceeded)
    signal.alarm(REGEX_BUDGET_SECONDS)
    current = "<startup>"
    try:
        for p in iter_tracked_files():
            if not p.is_file() or is_ignored_path(p):
                continue
            try:
                if p.stat().st_size > MAX_FILE_BYTES:
                    continue
            except OSError:
                continue
            current = p.relative_to(ROOT).as_posix()
            try:
                with p.open("rb") as handle:
                    head = handle.read(8192)
                    if looks_binary(head):
                        # Generated chain data, archives, images: a line-based
                        # secret scan has nothing to say about these, and
                        # regex-matching them can hang. Checking the head first
                        # also avoids reading 200 MB+ of ParityDB tables to skip them.
                        continue
                    raw = head + handle.read()
            except OSError:
                continue
            txt = raw.decode("utf-8", errors="ignore")
            for i, line in enumerate(txt.splitlines(), 1):
                # Cheapest order: only the three secret patterns run per line, and
                # the forty allow-list patterns only run on a hit.
                if not any(rx.search(line) for rx in _SECRET_RE):
                    continue
                if is_allowed_line(line):
                    continue
                issues.append(f"{p.relative_to(ROOT)}:{i}: {line.strip()}")
    except RegexBudgetExceeded:
        signal.alarm(0)
        # Fail loudly instead of hanging a 90-minute CI job: say which file ate the
        # budget so the pattern or the input can be fixed.
        print(
            f"[agent_guard] blocked: regex budget of {REGEX_BUDGET_SECONDS}s "
            f"exceeded while scanning {current}"
        )
        return 1
    signal.alarm(0)
    if issues:
        print("[agent_guard] blocked: secret-like material detected")
        print("\n".join(issues[:200]))
        return 1
    print("[agent_guard] ok")
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
