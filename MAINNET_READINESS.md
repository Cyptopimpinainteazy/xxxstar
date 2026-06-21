# Mainnet Readiness

**Canonical source: `FEATURE_REGISTRY.toml`** — all readiness scores and blockers derive from it. Run `scripts/check-readiness-consistency.sh` to validate.

**Overall readiness: ~52%** (average across 23 features). Mainnet is blocked until all required gates pass and blockers are resolved.

## Required gates

- `make guard` — agent/stub/test-cheat guards
- `make test` — focused Python + Rust compiler tests  
- `make audit` — invariant guard + mainnet release gate
- `make mainnet-check` — validates builds, chain-spec, tests, reproducible-build, secrets, node build
- `make fresh-machine-check` — bootstrap validation on fresh machine

## Mandatory controls (status)

| Control | Status | Detail |
|---|---|---|
| No critical/high unresolved security findings | 🟡 Partial | P0 key hygiene remediated; biometric + swarm audits pending |
| Replay protection and nonce uniqueness verified | ✅ | Dual-layer in place |
| Cross-VM atomic commit/rollback | ✅ | VM opcodes execute; multi-validator not yet validated |
| Canonical supply invariants verified | ✅ | King invariant + proofs + fuzzing |
| Secrets externalized | ✅ | Policy + gitignore + rotation docs |
| External bridge audit gate | 🟡 Partial | Fail-closed at genesis; multi-validator context untested |
| Genesis ceremony | ❌ | Not yet run |
| CI release artifacts | 🟡 Partial | SBOM pipeline live; mainnet-rc1 build broken |
| Multi-validator network testing | ❌ | Never triggered |
| BTC signer quorum | ❌ | Not established |
| Biometric security review | ❌ | Pending |
| Node build verification | 🟡 Partial | Default build passes; mainnet-rc1 feature fails (44 errors) |

## Blockers (open)

1. **mainnet-rc1 feature gate broken** — The runtime declares `mainnet-rc1 = []` but doesn't gate any pallets in `construct_runtime!`. Build fails with 44 E0277 errors.
2. **Multi-validator testing never run** — EconomicHalt never triggered in 4-validator Zombienet.
3. **BTC mainnet path untested** — Signer quorum, withdrawal broadcast, all untested.
4. **Biometric security review pending** — Wallet biometric templates + recovery logic not audited.
5. **Swarm agents not production-ready** — 6 agents experimental, not in CI, findings not triaged.
6. **Genesis ceremony not performed** — srtool verified release not tagged.

## Resolved blockers

1. ✅ CI gate wired for wallet pallet, DEX, token factory, LP locker, sentinel
2. ✅ Pallet tests fixed to use `--features std` (inline unit tests, not integration)
3. ✅ DEX/forge use EconomicHalt runtime gates
4. ✅ Score adjustments honest across FEATURE_REGISTRY.toml
5. ✅ 6 review comments implemented on status docs
6. ✅ Readiness consistency check script wired

## Feature Readiness Summary

See `FEATURE_REGISTRY.toml` for per-feature scores. Average ~52%:
- 2 features ≥85% (atomic kernel, atomic router)
- 5 features 65-75% (DEX, forge, atomic lock, gateway, wallet)
- 16 features 10-55% (everything else — experimental, untested, or missing gates)