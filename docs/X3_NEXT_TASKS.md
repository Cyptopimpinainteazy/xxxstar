# X3 Next Tasks

## Next 10 Tasks

### 1. Run scripts/x3-proof-check.sh against the full project
- Why: First proof run establishes baseline. Will detect Rust, Solidity, Python, and Node test/build status.
- Files: scripts/x3-proof-check.sh, .x3/proof/latest-proof.log
- Proof: bash scripts/x3-proof-check.sh; check exit code
- Done when: Proof check runs and produces PASS/FAIL with log output saved.

### 2. Run stub detector and classify all critical-path stubs
- Why: Need to know which stubs are in runtime/bridge/adapter paths. Critical stubs are blockers.
- Files: scripts/x3-detect-stubs.sh, all pallets/, runtime/, bridges/, adapters/
- Proof: bash scripts/x3-detect-stubs.sh; review [CRITICAL] findings
- Done when: All critical-path stubs are documented in completion status and triaged.

### 3. Install git hooks (pre-commit + pre-push)
- Why: Automates stub detection on commit and proof check on push. No more manual discipline.
- Files: scripts/x3-install-git-hooks.sh, .git/hooks/pre-commit, .git/hooks/pre-push
- Proof: bash scripts/x3-install-git-hooks.sh; ls -la .git/hooks/pre-commit .git/hooks/pre-push
- Done when: Both hooks installed and executable.

### 4. Run cargo check on all workspace crates
- Why: Verify which 25+ pallets and crates actually compile. Unknown how many are broken.
- Files: Cargo.toml, all pallets/, runtime/, node/
- Proof: cargo check --workspace --all-features 2>&1 | tee .x3/proof/cargo-check.log
- Done when: Compile errors cataloged, compilation success rate measured.

### 5. Run cargo test on a focused subset (start with x3-kernel pallet)
- Why: Start building test evidence. Full workspace test is too slow for first run — narrow scope.
- Files: pallets/x3-kernel/
- Proof: cargo test -p pallet-x3-kernel 2>&1 | tee .x3/proof/kernel-test.log
- Done when: Test pass/fail count recorded, failures documented.

### 6. Run forge test on X3-contracts EVM contracts
- Why: Verify Solidity contracts compile and tests pass. Unknown contract test status.
- Files: X3-contracts/evm/
- Proof: cd X3-contracts && forge test 2>&1 | tee ../../.x3/proof/forge-test.log
- Done when: Forge test results recorded, failing tests identified.

### 7. Audit 5 pallets for runtime wiring (construct_runtime! registration)
- Why: Pallets that compile but are not in construct_runtime! are dead code. Need wiring map.
- Files: runtime/src/lib.rs, pallets/
- Proof: grep pallet name in runtime/src/lib.rs construct_runtime! macro
- Done when: Wiring status (WIRED / UNWIRED) documented for each of the 5 audited pallets.

### 8. Run stub detector on pallets/ and classify by severity
- Why: Need to distinguish TODO comments from unimplemented!() in production paths.
- Files: pallets/
- Proof: bash scripts/x3-detect-stubs.sh; filter by pallets/ path
- Done when: Stub list generated with file:line for all pallets, severity classified.

### 9. Verify 3 key adapters have real logic (not return Ok(()))
- Why: adapters/evm.rs and other adapter crates are the touchpoint to external VMs. No-ops here are security risks.
- Files: adapters/evm.rs, crates/cross-vm-bridge/, crates/evm-integration/
- Proof: Read adapter source and verify non-trivial logic; grep for 'return Ok(())' 
- Done when: Each adapter audited and verdict recorded (REAL / STUB / PARTIAL).

### 10. Populate docs/X3_COMPLETION_STATUS.md with proof results from tasks 1-9
- Why: Status doc is currently educated guesses. Need proof-backed percentages.
- Files: docs/X3_COMPLETION_STATUS.md
- Proof: Update percentages based on task 1-9 proof outputs
- Done when: Every area in completion status has a proof-backed percentage (not UNKNOWN unless truly unmeasured).