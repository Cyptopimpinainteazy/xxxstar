# Skill: X3 Runtime Wiring Inspector

## Purpose
Prove code is actually wired into the runtime, router, CLI, API, pallet, module, or build path. File existence is not wiring.

## Use When
- After adding a new pallet, crate, module, endpoint, or command.
- Before claiming a feature is "integrated."
- When auditing completion claims.

## Inputs To Inspect
- `runtime/src/lib.rs` — `construct_runtime!` macro for pallets.
- `Cargo.toml` — workspace members, dependencies.
- `node/src/` — service, chain spec, CLI.
- `X3-contracts/` — deployment/migration scripts.
- `apps/` — router, API handler registrations.

## Checks To Perform
- Pallet: search for pallet name in `construct_runtime!`.
- Crate: search for `mod` or `use` + crate name in runtime or node.
- Contract: search for contract in deployment/migration scripts.
- CLI: search for subcommand in `cli.rs` or `main.rs`.
- API: search for route registration in server/router files.

## Proof To Require
- Grep output showing the wiring line.
- If not wired, state UNWIRED and exact location where it should be added.

## Output Format
- Module: <name>
- Wired: YES at <file:line> / NO (missing <specific wiring>)