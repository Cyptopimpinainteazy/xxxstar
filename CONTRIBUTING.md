# Repository Guidelines

## Project Structure & Module Organization

X3 Atomic Star is a Substrate chain with cross-VM execution across X3Native, X3Evm, and X3Svm; root `Cargo.toml` defines the Rust workspace (default member: `node`).

- `runtime/` (`x3-chain-runtime`), `node/` (`x3-chain-node`), `pallets/` (`x3-cross-vm-router`, `x3-supply-ledger`, `x3-settlement-engine`, `x3-atomic-kernel`), `crates/` (`x3-vm`, `x3-atomic-swap`).
- `x3-lang/` — language workspace: the Python pipeline is the authoritative MVP surface; `compiler/` and `vm/` hold the Rust IR and opcode work. Trading Core v1: `x3-lang/examples/trading_core_v1.x3`.
- `X3-contracts/` — separate Foundry (`evm/`) and Anchor (`svm/`) workspace with shared test vectors.
- `apps/`, `packages/`, `tests/`, `tests_phase4/`, `tests/e2e/`, `proof/` — TypeScript apps/SDKs, test suites, and ProofForge receipts; `.github/workflows/` holds CI gates.

## Build, Test, and Development Commands

- `cargo fmt --all -- --check` — formatting gate.
- `cargo check --workspace` — type-check all members.
- `cargo test --workspace` — Rust tests.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` — lint gate; warnings are errors.
- `make guard` — stub and test-cheat guards.
- `make audit` — invariants and readiness consistency.
- `make mainnet-check` — release gate; failure blocks release.
- `python -m pytest` (Python), `pnpm test` / `pnpm build` (JS/TS).
- `X3-contracts/`: `forge test` (EVM), `anchor test` (SVM).

## Coding Style & Naming Conventions

- Rust is `rustfmt`-formatted (`x3-lang/rustfmt.toml`: `max_width = 120`) and must pass Clippy.
- Use checked numeric conversions, explicit error handling, and no silent fallbacks in security code; consensus and runtime paths must be deterministic.
- Crates use `x3-*`; pallets use `x3-*` or `pallet-*`; tests use descriptive `snake_case` names.

## Testing Guidelines

Cover unit (`#[cfg(test)]`), integration (`tests/`, `tests/e2e/`, `tests_phase4/`), and E2E lifecycle suites (EVM/SVM HTLC, zombienet); ProofForge gates run via `.github/workflows/proof-gates.yml`.

Passing unit tests does not establish production readiness; mainnet claims require all gates to pass and `FEATURE_REGISTRY.toml` scores ≥95% per feature.

## Commit & Pull Request Guidelines

History follows Conventional Commits (`feat(x3-lang): …`, `fix(governance): …`); the optional `.githooks/commit-msg` hook enforces `[area] description; tests: <command>`.

PRs must state scope, motivation, commands executed, test and gate results, remaining risks, and the linked issue, plus screenshots for UI changes and the exact commit SHA for proof/CI evidence.

## Security & Production Requirements

See `AGENTS.md` for authoritative agent requirements; this file is the contributor guide.

- Never commit secrets, private keys, wallet seeds, API tokens, or RPC credentials.
- Mocks, stubs, fake data, dummy success paths, and placeholders must never be presented as production-complete; `scripts/mock-rpc-server.js` is DEV ONLY.
- Do not weaken CI, ProofGate/ProofForge, tests, invariants, security checks, or validation to obtain a passing result.
- State-changing operations must preserve transactional and rollback semantics; security-sensitive code requires explicit negative/failure-path tests.

## Agent-Specific Instructions

Automated coding agents MUST read `AGENTS.md` first; it takes precedence over this guide for execution, proof, and reporting.
