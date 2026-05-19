# X3 Mainnet Gates

X3 is not mainnet-ready unless all gates pass.

## P0 Gates

### Runtime
- `cargo fmt --all --check` passes
- `cargo clippy --workspace --all-targets -- -D warnings` passes
- `cargo check --workspace` passes
- `cargo test --workspace --lib --tests -- --test-threads=1` passes
- production runtime builds and WASM artifacts compile cleanly
- runtime upgrade rehearsal exists and is documented
- live `chain-specs/x3-mainnet-plain.json` and `chain-specs/x3-mainnet-raw.json` generated and reviewed
- no dev keys, no dev authorities, no testnet-only genesis accounts remain
- no consensus-critical `unwrap` / `panic!` in production hot paths
- failure in any runtime item blocks mainnet readiness

### Universal Asset Kernel
- canonical supply invariant exists and is guarded by invariant tests
- native, EVM, SVM, external_locked, and pending accounting are covered by tests
- overflow, precision, and fixed-point arithmetic tests exist for asset math
- rollback safety tests cover asset/kernel state after failed cross-VM flows
- invariant halt policy and economic safety gates reviewed
- failure in any asset kernel item blocks mainnet readiness

### Cross-VM Atomic Execution
- EVM / SVM / X3VM execution paths are covered by integration and attack scenario tests
- atomic commit/rollback semantics are tested end-to-end
- replay protection is tested for duplicate submission, stale execution, and nonce reuse
- expiry / deadline semantics are validated for timeouts and automatic rollback
- domain separation is enforced between VM payloads, proofs, and bridge routing
- cross-VM settlement proofs reject partial commits and preserve terminal-state uniqueness
- failure in any atomic execution item blocks mainnet readiness

### Bridge / Router
- bridge enablement audit gate exists in code and governance flow
- external bridge remains disabled by default until audit gate is formally passed
- nonce uniqueness and monotonic sequence enforcement tested
- message expiry, finality deadlines, and replay rejection are covered
- replay tests ensure duplicate bridge submissions do not mutate state
- router error handling, proof validation, and audit-gate toggles are documented
- failure in any bridge/router item blocks mainnet readiness

### DEX / Launchpad
- swap flow tests validate token swap correctness and failure rollback
- liquidity provision and removal tests validate lock accounting
- anti-rug and sandwich resistance tests exist for targeted attack scenarios
- fee accounting tests verify correct fee accrual and distribution
- slippage, minimum amount, and TWAP order tests cover front-running and price manipulation
- launchpad caps, vesting, and allocation rules are tested where applicable
- failure in any DEX / launchpad item blocks mainnet readiness

### Security
- no TODO / FIXME comments remain in P0 code paths
- no hardcoded mock values exist in production deployment paths
- weak randomness sources blocked and audited
- unsafe code is reviewed and accepted only where justified
- panic / unwrap audit is complete and documented
- all P0 security gaps are tracked in `reports/panic_unwrap_audit.md`
- failure in any security item blocks mainnet readiness

### Ops
- testnet launch checklist is complete and evidence is recorded
- genesis review is complete and reviewed by the team
- monitoring and alerting plan exists for mainnet launch
- rollback and recovery plan exists and is actionable
- validator bootstrap procedures are documented and tested
- emergency governance / upgrade procedure documented
- failure in any ops item blocks mainnet readiness

## P1 Gates

### Network & Infrastructure
- bootnodes, discovery, and peer routing are validated in a production-like topology
- RPC / websocket endpoints are stable under load and provide correct state responses
- network health checks and service self-healing are documented
- `ExternalBridgesEnabled=false` confirmed in mainnet genesis and node config
- mainnet binaries are signed, packaged, and reproducible across machines

### Governance & Risk
- bridge enablement and external gateway audit gates are enforced by governance
- invariant halt policies are reviewed and the emergency pause path is verified
- runtime upgrade procedure is approved and practiced in rehearsal
- formal proof / gate score reporting is wired to the mainnet gate workflow

### Monitoring, Alerting & Recovery
- alerting rules are defined for consensus stalls, reorgs, slashing, and invariant breaches
- logs and metrics are routed to a monitoring stack with runbook links
- incident response runbooks exist for rollback, chain restart, and bridge failure
- on-call handoff and escalation procedures are documented

### Documentation & Launch Support
- launch instructions are published in `docs/MAINNET_LAUNCH_CHECKLIST.md` and `launch-gates/EXECUTION_GUIDE.md`
- validator onboarding, bootstrap, and genesis ceremony docs are complete
- wallet, explorer, and SDK launch readiness are tracked
- post-launch support and post-mortem procedures are identified
- failure in any documentation or launch support item blocks mainnet readiness

## Evidence and Coverage Links
- `docs/MAINNET_LAUNCH_CHECKLIST.md`
- `launch-gates/EXECUTION_GUIDE.md`
- `crates/x3-launch-validator/`
- `tests_phase4/run-all.sh`
- `tests_core/invariants/registry.toml`
- `proof/receipts/claims/` (atomic rollback, replay protection, supply conservation, DEX safety)
- `reports/panic_unwrap_audit.md`
- `docs/SECURITY_GATES.md`
- `docs/runbooks/testing/VALIDATION_CHECKLIST.md`
- `package.json` (`npm test` / `vitest run`) for frontend / desktop test coverage

## How to Verify
- `cargo fmt --all --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo check --workspace`
- `cargo test --workspace --lib --tests -- --test-threads=1`
- `cargo test -p pallet-x3-cross-vm-router`
- `cargo test -p pallet-x3-supply-ledger`
- `cargo test -p pallet-x3-atomic-kernel`
- `cargo test -p x3-ixl`
- `cargo test -p x3-proof`
- `bash tests_phase4/run-all.sh`
- `npm test` or `vitest run` for frontend/desktop test coverage
- `grep -R "ExternalBridgesEnabled=false" chain-specs/`
- `cat docs/MAINNET_LAUNCH_CHECKLIST.md launch-gates/EXECUTION_GUIDE.md reports/panic_unwrap_audit.md`
- `cargo run -p x3-launch-validator -- --checklist` (if applicable)

