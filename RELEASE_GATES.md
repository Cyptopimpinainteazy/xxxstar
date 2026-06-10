# Release Gates

Release candidates must pass all of the following gates:

## Gate commands

- `make guard` — agent/stub/test-cheat guards
- `make test` — focused Python + Rust compiler tests
- `make audit` — invariant guard + mainnet release gate
- `make mainnet-check` — mainnet release gate (detailed below)
- `make fresh-machine-check` — bootstrap validation on a fresh machine

## Mainnet release gate (`make mainnet-check` → `scripts/mainnet_release_gate.py`)

Exit 0 = gate PASSES. Exit 1 = gate FAILS — do NOT cut a release.

The gate validates:

1. **Required documentation** — `MAINNET_READINESS.md`, `INVARIANTS.md`, `RELEASE_GATES.md`,
   `SECURITY.md`, `TESTING.md`, `AUDIT_SPEC.md` must all exist
2. **Build validation** — `x3-chain-node` and `x3-chain-runtime` WASM must build successfully
3. **Chain-spec/artifacts** — genesis spec JSON artifacts must be valid and contain a `genesis` key;
   `production_config()` must exist in `node/src/chain_spec.rs`
4. **Critical test suites** — unit tests for `x3-chain-runtime`, `x3-supply-ledger`,
   `x3-packet-standard`, `x3-bridge`, `x3-fees`, `x3-slash` must pass
5. **Reproducible builds** — `srtool` and `docker` must be available; no `SKIP_WASM_BUILD` override
6. **Secret hygiene** — no hardcoded `PRIVATE_KEY`, `MNEMONIC`, or `AKIA...` tokens in the repo

## CI enforcement

The mainnet gate is enforced in `.github/workflows/mainnet-readiness.yml`:
- Runs on every push/PR to `main`/`master`
- Installs Python + Rust toolchain
- Runs `make mainnet-check` and `make fresh-machine-check`
- Computes release hashes and uploads build artifacts (node binary, WASM, chain specs, hashes)

## Mainnet-ready claims

Mainnet-ready claims are forbidden unless all gates pass.