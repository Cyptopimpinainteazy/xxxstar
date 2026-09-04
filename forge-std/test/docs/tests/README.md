# X3 Test Suite Overview ✅

This folder contains the testing harness and registry for the X3 / X3 Chain project.

Quick start:

- Unit tests (Rust): cargo test --workspace -p <crate>
- All workspace tests (unit+integration): cargo test --workspace
- Python tests (swarm): pytest -q
- TypeScript tests (SDK): npm test --workspace packages/ts-sdk

Key files:

- `tests/invariants/registry.toml` - centralized invariant registry (every invariant has an ID & severity)
- `tests/run-all.sh` - simple test runner that executes workspace unit tests and reports status

Guidelines:

- Every test must reference at least one invariant ID (via `#[invariant("ID")]` for Rust tests, or documented in the module docstring).
- Add new invariants to `tests/invariants/registry.toml` with `tested_by` paths.
- Use the `invariant-macros` crate for heavy-lifting attribute validation.
