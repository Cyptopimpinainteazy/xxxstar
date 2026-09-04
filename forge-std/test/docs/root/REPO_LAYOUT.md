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
- UI: `apps/`, `x3fronend/`

## Rules

- New production Rust crates belong in `crates/` unless they are pallets,
  runtime, or node code.
- Nested workspaces must use distinct package names to avoid duplicate crate
  identities across the repository.
- Top-level one-off artifacts should be moved under `docs/`, `scripts/`, or an
  explicit archive directory instead of accumulating at repo root.
