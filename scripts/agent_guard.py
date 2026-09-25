#!/usr/bin/env python3
import pathlib
import re
import subprocess

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
IGNORE_PREFIXES = (
    ".kilo/",
    "apps/dashboard/dist/",
    "audit-artifacts/",
    "apps/x3-desktop/src-tauri/tauri-vendor/",
    "forge-std/",
    "packages/polkawallet-plugin/dist/",
    "proof/reports/",
    "tests/phase_core/security/lib/forge-std/",
)
SECRET_PATTERNS = [
    # The separator is `=` or a single `:`, and deliberately not `::`: Rust
    # path-qualified names are not assignments. `[:=]` matched the first colon of
    # `Mnemonic::from_phrase`, so any prose or code naming a type and a method
    # ("the `bip39::Mnemonic::from_phrase` spelling", `Mnemonic::parse_in(`) was
    # reported as secret-like material and reddened `make guard` for everyone —
    # the same class of false positive the bip39 allow-list entry below was added
    # for, one layer up.
    r"(?i)\b(private[_-]?key|mnemonic|api[_-]?key|rpc[_-]?key)\b\s*(?::(?![=:])|=)\s*['\"]?[^'\"\s]{8,}",
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
    # The bip39 2.x constructor. The broad rule above (line 32: an assignment to
    # `mnemonic` of eight or more characters) matches
    # `let mnemonic = bip39::Mnemonic::parse_in(...)`, which parses a
    # caller-supplied phrase and hardcodes nothing — it turned this crate's
    # legitimate constructor call into "secret-like material" and made
    # `make guard` fail on master. The 1.x entry below stopped matching when the
    # crate moved to 2.x, which is what left the gap.
    r"(?i)\bmnemonic\s*[:=]\s*['\"]?bip39::Mnemonic::(parse_in|from_phrase)\(",
    r"(?i)API_KEY\s*=\s*\"?\$\{[A-Z0-9_]+:-\}\"?",
    r"(?i)APIKey\s*=\s*\"infra_x+\"",
    r"(?i)PRIVATE_KEY=\d{16,}",
    r"(?i)--from-literal=api-key=sk-or-\.\.\.",
    r"(?i)\$\{API_KEY:0:30\}",
    r"(?i)privateKey:\s*'•+",
    r"(?i)private[_-]?key\s*[:=]\s*(self\.private_key|\"//Charlie\"|private_key\.clone\(\))",
    r"(?i)mnemonic\s*=\s*self\.decrypt_data\(",
    r"(?i)mnemonic\s*=\s*bip39::Mnemonic::from_phrase\(",
    r"(?i)apiKey:\s*'your-api-key'",
    r"(?i)apiKey:\s*config\.(apiKey|privateKey)",
    r"(?i)this\.config\.apiKey\s*=\s*undefined",
    r"(?i)key\.privateKey\b",
    # `os.getenv` is the other half of reading a key from the environment, which
    # the `os.environ.` entry above already excuses: `crates/gpu-swarm/src/
    # social_agents.py` names the Twitter keys it wants and gets them from the
    # process environment, hardcoding nothing.
    r"(?i)\b(api[_-]?key|rpc[_-]?key)\b\s*[:=]\s*os\.getenv\(",
    # The BIP39 *generator*. `admin.rs` mints a fresh mnemonic at request time
    # (`crate::bip39::generate_mnemonic_12()`); the broad rule above reads any
    # assignment to `mnemonic` as a hardcoded secret and flagged the call site.
    r"(?i)\bmnemonic\s*=\s*crate::bip39::generate_mnemonic_12\(\)",
    # A compose/service file passing an environment variable through. The
    # existing `${VAR:-}` entry only covered the default-value spelling, so the
    # plain `- API_KEY=${OPENAI_API_KEY}` line in
    # `x3-swarm-orchestra/docker-compose.yml` looked like a literal.
    r"(?i)\bAPI_KEY\b\s*=\s*\"?\$\{[A-Z0-9_]+\}\"?",
    # A detector naming the marker it looks for, e.g. the quoted PEM header in
    # `security/audit-sdk.sh`'s own SECRET_PATTERNS array. The line has to be the
    # bare quoted marker: a real key's header line is unquoted and is followed by
    # base64, so this cannot excuse one.
    r'^\s*"-----BEGIN (EC|RSA|OPENSSH) PRIVATE KEY-----"\s*$',
]


def is_ignored_path(path: pathlib.Path) -> bool:
    rel = path.relative_to(ROOT).as_posix()
    if rel == "scripts/agent_guard.py":
        return True
    if any(part in IGNORE_DIRS for part in path.parts) or any(
        rel.startswith(prefix) for prefix in IGNORE_PREFIXES
    ):
        return True
    # `Foo_files/` is what a browser writes when a page is saved for off-line
    # reading: minified third-party JS, not source. The imported Hashlock audit
    # reports under `docs/` ship exactly that, and `apiKey:` assignments inside a
    # webpack bundle tripped the patterns. Only `docs/` is excused, and only for a
    # `*_files/` directory, so a real secret committed anywhere else still fails.
    return rel.startswith("docs/") and any(part.endswith("_files") for part in path.parts)


def is_allowed_line(line: str) -> bool:
    return any(re.search(rx, line) for rx in ALLOW_LINE_PATTERNS)


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
    for p in iter_tracked_files():
        if not p.is_file() or is_ignored_path(p):
            continue
        try:
            if p.stat().st_size > MAX_FILE_BYTES:
                continue
        except OSError:
            continue
        txt = p.read_text(encoding="utf-8", errors="ignore")
        for i, line in enumerate(txt.splitlines(), 1):
            if is_allowed_line(line):
                continue
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
