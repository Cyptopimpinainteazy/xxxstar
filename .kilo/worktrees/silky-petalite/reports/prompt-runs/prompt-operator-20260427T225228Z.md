# X3 Full-Stack Build Prompt (Operator Version)

Use this concise prompt when you need an execution run that cannot silently skip features.

---

You are an execution agent inside this X3 repository. Build the full v0.4 stack end-to-end with no missing modules and no fake completion.

## Mission
- Implement all required modules listed below.
- Wire them into runtime/router/ledger paths.
- Prove behavior with tests and launch gates.
- Produce completion evidence files.

## Hard constraints
1. No stubs in production paths (`todo!`, `unimplemented!`, fake placeholders, panic control flow).
2. No silent skips; blocked items must be reported with exact file paths and failing command output.
3. Do not weaken tests to get green.
4. Runtime/consensus/bridge paths are high risk: prefer minimal auditable patches.

## Required module set (must all be complete or explicitly blocked)
1. `x3-packet-standard` (packet lifecycle: sequence/replay/ack/timeout/refund/proof)
2. `x3-ixl` (instruction/planner/interpreter/receipt/rollback/verifier)
3. `pallet-x3-cross-vm-router` wiring to packet+ixl
4. `pallet-x3-atomic-kernel` and `pallet-x3-supply-ledger` invariant hardening
5. Runtime wiring in `runtime/Cargo.toml` and `runtime/src/lib.rs`
6. `x3-liquidity-core`
7. `x3-universal-contracts`
8. `x3-external-liquidity-gateway`
9. `x3-integrated-services`
10. `x3-parallel-executor`
11. `x3-appzone-factory`
12. PQ integration (`x3-pq` or integrated equivalent)
13. `x3-readiness-report` real data collectors (no hardcoded readiness)

## Required gates (all must pass)
```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo check --workspace --all-targets
cargo test --workspace --lib --tests -- --test-threads=1

cargo test -p x3-packet-standard
cargo test -p x3-ixl
cargo test -p x3-readiness-report
cargo test -p pallet-x3-cross-vm-router --lib
cargo test -p pallet-x3-supply-ledger
cargo test -p pallet-x3-atomic-kernel
cargo test -p pallet-x3-asset-registry

bash launch-gates/validate-embarrassment-suppressions.sh launch-gates/embarrassment-suppressions.conf
STRICT=1 STRICT_P2=1 BLOCK_P2_CATEGORIES='DEV LOCALHOST' \
SUPPRESSIONS_FILE='launch-gates/embarrassment-suppressions.conf' \
SCAN_PATHS='crates/x3-packet-standard crates/x3-ixl crates/x3-readiness-report pallets/x3-cross-vm-router' \
bash launch-gates/embarrassment-scan.sh
```

## Required evidence outputs
Write all of these:
- `reports/full_stack_build_report.json`
- `reports/full_stack_build_report.md`
- `launch-gates/evidence/ci/proof-embarrassment-scan.log`
- `launch-gates/evidence/ci/embarrassment-raw-findings.txt`
- `launch-gates/evidence/ci/embarrassment-suppressed-findings.txt`
- `launch-gates/evidence/ci/embarrassment-suppressions-validation.log`
- `launch-gates/evidence/ci/embarrassment-evidence-summary.md`

## Completion criteria
Mark `complete` only if:
1. Every required module is implemented and wired (or marked blocked with root cause).
2. All required gates pass.
3. Evidence outputs exist.
4. No production-path placeholders remain.

Now execute and do not stop early.
