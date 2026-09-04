# Skill: X3 Repo Mapper

## Purpose
Map the real repo structure — identify source directories, test directories, build configs, runtime entry points, and language-specific tooling.

## Use When
- Starting work on an unfamiliar area.
- Before the pre-task hook runs.
- When the agent needs to understand what exists.

## Inputs To Inspect
- `Cargo.toml` — workspace members, dependencies.
- `package.json` / `package-lock.json` — Node/TS projects.
- `hardhat.config.ts` / `foundry.toml` — Solidity projects.
- `runtime/src/lib.rs` — Substrate runtime pallet configuration.
- `node/src/` — node entry point.
- `Makefile` — build targets.
- `scripts/` — existing tooling.

## Checks To Perform
- List all languages in use.
- Identify test runners.
- Identify build system.
- Map pallets to runtime wiring.
- Map contracts to deployment scripts.

## Proof To Require
- At minimum, confirm build compiles for the target area.
- List of languages and proof commands available.

## Output Format
- Languages: [Rust, Solidity, TypeScript, Python, ...]
- Build: `cargo build`, `forge build`, `npm run build`, ...
- Test: `cargo test`, `forge test`, `npm test`, `pytest`, ...
- Lint: `cargo clippy`, `npm run lint`, ...
- Runtime entry: `runtime/src/lib.rs`
- Pallets: [list from Cargo.toml]
- Contracts: [list from X3-contracts/]