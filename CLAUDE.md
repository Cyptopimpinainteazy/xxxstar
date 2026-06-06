# X3 Atomic Star / X3 Chain Development Rules

## Project Identity

This repo is for X3 Atomic Star / X3 Chain.

Core architecture:
- Multi-VM L1: X3VM + EVM + SVM.
- x3-lang is the language layer for cross-VM intents, atomic execution, settlement, rollback, replay safety, expiry, route validation, and adapter calls.
- Universal Asset Kernel invariants matter more than cosmetic progress.
- BTC/UTXO and CosmWasm paths must be implemented only when real repo support exists; otherwise fail safely behind feature gates.

## Source of Truth

Use actual code as source of truth.

Do not trust stale markdown, old roadmaps, or optimistic docs unless code confirms them.

## Non-Negotiable Rules

- Do not weaken tests to pass.
- Do not delete failing tests unless objectively obsolete and explained.
- Do not replace implementation with mocks except inside test-only modules.
- Do not hide unfinished paths behind docs.
- Do not claim COMPLETE until verification passes.
- Do not leave reachable `todo!`, `unimplemented!`, fake stubs, no-op adapters, or silent partial execution in production paths.

## x3-lang Required Capabilities

x3-lang must support and verify:
- intents
- assets
- cross-VM calls
- atomic routes
- settlement blocks
- rollback clauses
- guards/preconditions
- fee/gas declarations
- replay protection
- expiry/deadline rules
- adapter compatibility
- receipt/proof handling
- deterministic simulation and execution

## Atomic Execution Model

Every atomic route must follow:

1. parse
2. semantic check
3. lower to typed IR
4. validate IR
5. preflight all legs
6. reserve/lock assets
7. execute route
8. verify receipts/proofs
9. commit all legs
10. rollback all failed/partial legs
11. settle final state
12. assert invariants

Partial success without rollback is forbidden.

## Required Verification

Use:

```bash
./scripts/x3-verify.sh

Preferred Rust gates:

cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo llvm-cov --workspace --all-features --summary-only
Status Format

When reporting progress:

<status>
completion: X%
finished:
- ...
remaining:
- ...
tests_run:
- command => result
coverage:
- ...
risks:
- ...
next:
- ...
great_idea:
- ...
</status>
Completion Promise

Only output:

<promise>COMPLETE</promise>

after all implementation, tests, lint, coverage, and docs pass.
