# Proof Mode

Proof Mode is the core mechanism that prevents fake completion claims.

## How It Works

1. Any command run through `runProofCommand()` captures:
   - Full command string
   - Working directory
   - Start/end timestamps
   - Duration
   - Exit code
   - stdout + stderr
   - Changed files (via `git diff`)
2. Artifacts are written to:
   - `x3-proof/PROOF_REPORT.json` — machine-readable
   - `x3-proof/PROOF_REPORT.md` — human-readable

## Proof Actions

| Action | Command | Status |
|--------|---------|--------|
| Cargo Check | `cargo check 2>&1` | ✅ |
| Cargo Test | `cargo test 2>&1 \| tail -20` | ✅ |
| Forge Build | `forge build 2>&1` | ✅ |
| Forge Test | `forge test 2>&1 \| tail -20` | ✅ |
| Custom Command | Any shell command | ✅ |
| Testnet Gate | `cargo check + cargo test` | ✅ |
| Mainnet Gate | Full verification pipeline | ✅ |

## Integrity

Every proof record includes the exact exit code. No claim of success is accepted unless:
- `exitCode === 0` → PASS
- `exitCode !== 0` → FAIL
- `exitCode === null` → BLOCKED
