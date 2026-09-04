# Security Scanner

## Status: ✅ FULL (pattern-based)

### What It Scans For
- **Exposed Private Keys** — PRIVATE_KEY, private_key, SECRET, secret_key patterns
- **Seed Phrases** — seed phrase, mnemonic patterns
- **API Keys** — api_key, API_KEY patterns
- **Hardcoded Passwords** — password=, PASSWORD patterns
- **.env files** — tracked environment files
- **Insecure eval** — arbitrary code execution risk
- **Unsafe file access** — unrestricted read/write
- **Shell injection risk** — child_process.exec/spawn
- **Hardcoded hex keys** — 64-char hex strings (potential private keys)

### How It Works
1. Scans workspace with glob `**/*.{ts,tsx,js,jsx,rs,sol,py,x3,go,json,yaml,toml}`
2. Ignores node_modules, dist, target, .git, out/
3. Each match is classified with severity: INFO, WARNING, HIGH, CRITICAL
4. Results displayed in Security panel and Problems panel

### Limitations
- Pattern-based: will have false positives
- No semantic analysis (Slither, Semgrep not integrated)
- No dynamic analysis (Echidna, Medusa not integrated)
