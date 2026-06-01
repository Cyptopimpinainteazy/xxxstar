# Repository Layout

This document defines directory ownership for the active code paths in the
workspace.

## Canonical Ownership

- `runtime/`: chain runtime and runtime-only integration glue.
- `node/`: Substrate node, RPC, networking, and operator-facing startup code.
- `pallets/`: FRAME pallets that are linked into the runtime.
- `crates/`: shared Rust libraries and supporting services used by the node,
  runtime-adjacent tooling, SDKs, and off-chain daemons.
- `apps/`: user-facing applications and desktop/web shells.
- `docs/`: product, architecture, runbook, and audit documentation.
- `patches/`: vendored third-party patches required to keep the workspace
  buildable on the pinned toolchain.
- `scripts/`: canonical automation root — all build, test, deployment, smoke,
  and daemon helper scripts live here with named subfolders.
- `tests/`: single canonical test tree with phase-specific subfolders
  (`phase_core/`, `phase4/`) instead of separate top-level test roots.
- `X3-contracts/`: canonical contracts workspace (EVM via `evm/`, SVM via
  `svm/`, shared assets via `shared/`). All `.sol` sources, governance,
  treasury, and staking contracts belong here.
- `_junk/`: quarantined archive of old roots, duplicated trees, transient
  worktrees, and generated report folders. Git-ignored; not scanned by tools.
- `x3-lang/`: legacy nested workspace for language prototypes; package names are
  prefixed with `x3-lang-` to avoid collision with the active root workspace.

## Practical Mapping To Product Areas

- VM: `crates/x3-vm`, `crates/x3-compiler`, `crates/x3-backend`,
  `crates/x3-verifier`, `x3-lang/`
- SDK / CLI: `crates/x3-sdk`, `crates/x3-wallet`, `crates/x3-cli`
- Daemons / services: `crates/x3-indexer`, `crates/x3-gateway`,
  `apps/analytics/analytics-service`
- AI / swarm: `crates/gpu-swarm`, `crates/x3-gpu-validator-swarm`,
  `crates/quantum-swarm`
- Contracts: `X3-contracts/evm/`, `X3-contracts/svm/`, `X3-contracts/shared/`
- UI: `apps/`, `x3fronend/`

## Archiving Rule

Transient worktrees (`.kilo/`, `.reports/`), generated report folders, old
contract roots (`contracts/`, `governance/`, `treasury/`, `staking/`), and
deprecated code trees (`scripts_infrastructure/`, `tests_core/`, `tests_phase4/`)
that are no longer canonical must be moved under `_junk/` for human sorting.
`_junk/` is git-ignored and excluded from all build, test, and search tooling.

## Rules

- New production Rust crates belong in `crates/` unless they are pallets,
  runtime, or node code.
- Nested workspaces must use distinct package names to avoid duplicate crate
  identities across the repository.
- Top-level one-off artifacts should be moved under `docs/`, `scripts/`, or an
  explicit archive directory instead of accumulating at repo root.
- All automation scripts live under `scripts/` with named subfolders. Do not
  create new top-level script roots.
- All smart-contract sources live under `X3-contracts/`. Do not create new
  top-level contract roots.
- CI and tooling must only traverse `tests/`, not any duplicated test tree.
- CI must fail on broken intra-repo symlinks/paths and on new undeclared
  top-level code roots not listed in this document.
