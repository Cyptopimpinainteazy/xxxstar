# Mainnet Readiness

Mainnet is blocked unless all required release gates pass.

## Required gates

- `make guard` — agent/stub/test-cheat guards
- `make test` — focused Python + Rust compiler tests
- `make audit` — invariant guard + mainnet release gate
- `make mainnet-check` — **mainnet release gate**: validates builds, chain-spec artifacts,
  critical runtime/pallet test suites, reproducible-build prerequisites, and secret hygiene
- `make fresh-machine-check` — bootstrap validation on a fresh machine

## What the mainnet release gate (`scripts/mainnet_release_gate.py`) validates

1. Required documentation exists (`MAINNET_READINESS.md`, `INVARIANTS.md`, `RELEASE_GATES.md`,
   `SECURITY.md`, `TESTING.md`, `AUDIT_SPEC.md`)
2. `x3-chain-node` and `x3-chain-runtime` WASM build successfully
3. Chain-spec artifacts are valid JSON genesis specs (parseable, contain `genesis` key)
4. Critical runtime and pallet test suites pass (`x3-chain-runtime`, `x3-supply-ledger`,
   `x3-packet-standard`, `x3-bridge`, `x3-fees`, `x3-slash`)
5. Reproducible-build prerequisites are met (`srtool` installed, `docker` available,
   no `SKIP_WASM_BUILD` override)
6. No hardcoded secrets (`PRIVATE_KEY`, `MNEMONIC`, `AKIA...`) are present in the repository

## Mandatory controls

- No critical/high unresolved security findings
- Replay protection and nonce uniqueness verified
- Cross-VM atomic commit/rollback verified
- Canonical supply invariants verified
- Secrets externalized (or no hardcoded secrets detected)
- Rollback and migration procedures documented
- Genesis ceremony requires tagged release commit + `srtool` (no cargo fallback)
- Mainnet chain spec is generated from `--chain production` preset (no testnet fallback)
- CI publishes build artifacts (node binary, WASM, chain specs, hashes) as release candidates