# Mainnet Readiness

**Canonical source: `FEATURE_REGISTRY.toml`** — all readiness scores and blockers derive from it. Run `scripts/check-readiness-consistency.sh` to validate.

**Overall readiness: ~36%** (average across 23 features). Mainnet is blocked until all required gates pass.

## Required gates

- `make guard` — agent/stub/test-cheat guards
- `make test` — focused Python + Rust compiler tests
- `make audit` — invariant guard + mainnet release gate
- `make mainnet-check` — validates builds, chain-spec, tests, reproducible-build, secrets
- `make fresh-machine-check` — bootstrap validation on fresh machine

## Mandatory controls (status)

| Control | Status | Detail |
|---|---|---|
| No critical/high unresolved security findings | ✅ | P0 key hygiene remediated |
| Replay protection and nonce uniqueness verified | ✅ | Dual-layer in place |
| Cross-VM atomic commit/rollback | ⚠️ Partial | VM opcodes execute; no multi-validator test |
| Canonical supply invariants verified | ✅ | King invariant + proofs + fuzzing |
| Secrets externalized | ✅ | Policy + gitignore + rotation docs |
| External bridge audit gate | ✅ | Fail-closed at genesis |
| Genesis ceremony | ⬜ | Requires tagged release + srtool |
| CI release artifacts | ✅ | SBOM + attestations pipeline live |

## Top blockers to mainnet

1. Atomic kernel economic halt never triggered in multi-validator (75%)
2. External bridge disabled at genesis — governance gate untested (45%)
3. No CI hard gate for wallet pallet, DEX, token factory, LP locker, sentinel
4. BTC mainnet SIM_TESTNET only — no real signer quorum (25%)
5. No automated migration dry-run across runtime variants (50%)
6. All 6 launch phases require infrastructure deployment