# X3 Public Mainnet P0 Completion Plan

> **For agentic workers:** This is the release-control and delivery plan approved on 2026-10-23.
> Execute it task-by-task with `superpowers:executing-plans` and TDD. Track progress in the plan
> workspace ledger rather than relying on conversation state.

**Goal:** Ship a public X3 mainnet release only after every current P0 feature row reaches 95 or
above with current evidence, the Ethereum, Solana, and Bitcoin bridges are proven and active, seven
independent authority validators are live, and the full assurance program has closed.

**Architecture:** Keep `FEATURE_MATRIX.toml` and its fragments as the granular source of truth and
`FEATURE_REGISTRY.toml` as the coarse product registry. Add a fail-closed P0 release gate and a
versioned launch-record contract that binds exact commit, binary, runtime WASM, genesis, bridge
configuration, audits, public-testnet evidence, and operator attestations. Build the production
runtime as one explicit `mainnet-v1` feature; keep test verifiers, mock proofs, experimental
execution, and GPU consensus truth out of that build.

**Tech Stack:** Rust/Substrate, X3 language and VMs, Solidity/Foundry, Solana/Anchor, Bitcoin
HTLC/SPV, Python release gates, shell deployment gates, PostgreSQL/SQLite where already used,
systemd, Prometheus/Alertmanager/Loki, and signed JSON evidence.

**Spec:** `LAUNCH_SCOPE.md`, `FEATURE_MATRIX.toml`, `feature-matrix/*.toml`,
`launch-gates/EXTERNAL_VALIDATION_REQUIREMENTS.md`, `docs/economics/X3_ALPHA_ECONOMIC_POLICY.md`,
`docs/STAGING_TESTNET_SETUP.md`, and `docs/release-notes/CANDIDATE_EVIDENCE_PACK_SPEC.md`.

## Global Constraints

- All 82 P0 rows at the implementation baseline must reach implemented, tested, and mainnet-ready
  scores of at least 95, with no open blockers.
- Ethereum L1, Solana mainnet-beta, and Bitcoin mainnet are the only external bridge families active
  at genesis. Every other adapter remains disabled.
- Seven authority-set genesis validators, operated independently. Permissionless staking remains
  disabled after launch until a separately audited upgrade.
- Fixed genesis supply is 8,888,888,888 X3 with zero inflation. Validator compensation must come
  from the governed security reserve, not fabricated inflation or staking rewards.
- Initial bridge limits are $1,000 per transfer, $10,000 per route per day, and $50,000 total bridge
  exposure for the first 30 days.
- Two independent tier-1 audits, a bug bounty with at least a $100,000 pool, an eight-week public
  testnet with at least 50 external validators, and multi-jurisdiction legal review are mandatory.
- Never run production with `test-verifier`, `NoOpCrossChainValidator`, mocks, fixed authority
  seeds, fabricated quorums, or a GPU result that disagrees with the CPU reference.
- Preserve unrelated work, never force-push, never rewrite public history without explicit
  authorization, and treat a BLOCKED gate as not passing.

## Review Focus

- A candidate chain that activates an external route without a verifier or with a test verifier.
- A bridge proof that is internally consistent but not bound to the intended chain, transaction,
  block, intent, proof set, and replay domain.
- A canonical supply, pending balance, refund, fee, bond, or treasury accounting path that changes
  value without a corresponding rejection or reconciliation test.
- A validator or operator drill that succeeds on loopback, stale evidence, one host, or the wrong
  genesis.
- A release record whose hashes, commit, audit, testnet, or operator attestation no longer describe
  the artifact presented for promotion.

## Tasks

### Task 1: Enforce the P0 release contract

**Files:**
- Modify: `scripts/mainnet_release_gate.py`
- Modify: `tests/test_mainnet_release_gate.py`
- Modify: `FEATURE_MATRIX.toml` only if a gate metadata field is required
- Modify: `RELEASE_GATES.md`

**Interfaces:**
- Consumes: `scripts/feature_matrix.py` matrix records and `scripts/x3_audit_matrix.py --check`.
- Produces: a `check_p0_scope()` gate that rejects any P0 row with `implemented`, `tested`, or
  `mainnet_ready` below 95, any nonempty `blockers`, any missing required test, or stale generated
  matrix artifacts.

- [ ] Add failing tests for 82 passing rows and for exactly one P0 row at 94.
- [ ] Add failing tests for a nonempty blocker list and a named required test that does not resolve.
- [ ] Implement the release gate check and wire it into `make mainnet-check`.
- [ ] Run `python3 -m pytest -q tests/test_mainnet_release_gate.py` and the release gate.
- [ ] Commit with `[release] gate every P0 row at 95 with current evidence; tests: ...`.

### Task 2: Version the launch evidence contract

**Files:**
- Modify: `scripts/mainnet/launch_record.py`
- Modify: `tests/test_launch_record.py`
- Modify: `docs/release-notes/CANDIDATE_EVIDENCE_PACK_SPEC.md`

**Interfaces:**
- Produces: launch-record schema 2 with commit, artifacts, feature-matrix digest, P0 snapshot,
  test gates, audits, bounty window, public testnet window, bridge configuration, validators, and
  approvals.
- `verify` rejects stale or mismatched evidence instead of checking only the RC marker.

- [ ] Add failing tests for every missing or stale schema-2 evidence class.
- [ ] Implement deterministic serialization, hashing, and verification.
- [ ] Run the focused test suite and the existing launch-record checks.
- [ ] Commit the schema change.

### Task 3: Add the explicit `mainnet-v1` runtime surface

**Files:**
- Modify: `runtime/Cargo.toml`
- Modify: `node/Cargo.toml`
- Modify: `runtime/src/lib.rs`
- Modify: relevant runtime and feature-gate tests

**Interfaces:**
- `mainnet-v1` enables real verifier-backed Ethereum, Solana, and Bitcoin routes and excludes
  `test-verifier`, `parallel-executor`, `ai-optimizer`, and GPU consensus truth.
- Boot tests assert real verifier registration and fail closed when it is absent.

- [ ] Add failing feature-combination and boot tests.
- [ ] Implement `mainnet-v1` without weakening or deleting `mainnet-rc1`.
- [ ] Run no-default-feature checks, runtime tests, boot tests, and clippy with the new feature.
- [ ] Commit the runtime surface.

### Task 4: Close economic policy and release hygiene blockers

**Files:**
- Modify: the token, fee, treasury, chain-spec, secret-guard, and release artifacts identified by
  the current economic and security gates.
- Test in the owning pallet/crate and the end-to-end economics gate.

**Interfaces:**
- One canonical token scale (`10^12`), one canonical asset id, a real validator payout path, a
  multisig-controlled treasury, and zero supply inflation.
- No tracked `dist/` output or crawler state, and no unrevoked credential remains.

- [ ] Add failing economic conservation, payout, treasury-origin, decimal, and asset-id tests.
- [ ] Implement the smallest coherent fixes and untrack generated output.
- [ ] Run `scripts/mainnet/x3_economic_model_gate.sh`, secret guards, treasury tests, and the
  related P0 rows' named tests.
- [ ] Commit the economic and hygiene closure.

### Task 5: Complete runtime, consensus, and internal cross-VM P0 rows

**Files:**
- Modify: owning pallets/crates under `pallets/`, `crates/`, `runtime/`, and `node/`.
- Add or extend tests next to each owning module and the live multi-validator harnesses.

**Interfaces:**
- Existing atomic, finality, supply, and packet wire types remain compatible unless a deliberate
  migration is added.
- Every P0 row's blocker list names exact evidence and no unresolved blocker remains.

- [ ] For each of the 27 runtime/consensus/cross-VM P0 rows, add the missing behavior test first.
- [ ] Implement the minimum production change, then run its owner crate test.
- [ ] Run the three-validator and seven-validator live drills after the local suites.
- [ ] Update matrix evidence only from measured output.
- [ ] Commit coherent groups, never a score-only change.

### Task 6: Complete language, trading, privacy, and acceleration P0 rows

**Files:**
- Modify: `x3-lang/`, `crates/x3-vm`, trading/MEV pallets and crates, and the GPU validation crates.
- Extend tests and evidence under the owning components.

**Interfaces:**
- CPU consensus remains authoritative. GPU output must match byte-for-byte or the accelerator is
  quarantined and the CPU result is used.
- Receipt replay and MEV ordering must be deterministic across the standard and on-chain engines.

- [ ] Add failing cross-engine, replay, profit-floor, rollback, ordering, and CPU/GPU parity tests.
- [ ] Implement the missing paths and update weights only from measured benchmarks.
- [ ] Run each focused suite, both GPU benchmarks separately and together, and the complete local
  CI profile appropriate to the touched paths.
- [ ] Commit only after the matrix rows have verified evidence.

### Task 7: Complete Ethereum, Solana, and Bitcoin bridge P0 rows

**Files:**
- Modify: bridge adapters, verifier router, finality oracle, relayer, gateway/settlement/wrapped
  pallets, EVM contracts, SVM programs, Bitcoin vault/SPV code, and testnet harnesses.
- Add tests next to each owning component and a cross-family end-to-end harness.

**Interfaces:**
- Finality certificates are derived from real chain evidence and bind chain, block, transaction,
  intent, proof set, and replay domain.
- Genesis bridge configuration names exactly `ethereum-mainnet`, `solana-mainnet-beta`, and
  `bitcoin-mainnet` with the approved caps and kill-switch state.

- [ ] Add failing tests for deposit, claim, timeout, refund, duplicate, wrong secret, wrong chain,
  reorg, provider disagreement, restart, split-write, and proof reuse.
- [ ] Implement real verifier registration and relayer quorum for all three families.
- [ ] Run Sepolia, Solana devnet, and Bitcoin testnet end-to-end flows.
- [ ] Run controlled low-value mainnet canaries against each external chain.
- [ ] Commit after all 21 cross-chain P0 rows and related atomic rows have measured evidence.

### Task 8: Complete physical seven-validator and operational evidence

**Files:**
- Modify: inventory, Ansible playbooks, systemd units, monitoring, backup/restore, DNS/RPC
  configuration, and operator runbooks.
- Extend the physical preflight and disaster drills.

**Interfaces:**
- Seven distinct non-loopback hosts with unique identities and matching artifacts.
- Finality with two validators offline; no finality with three equally weighted validators offline;
  convergence after recovery.
- No validator exposes administrative RPC.

- [ ] Add failing preflight cases for loopback, duplicate identity, missing artifact, stale commit,
  clock skew, disk pressure, and finalized-hash disagreement.
- [ ] Provision and run the seven-host topology.
- [ ] Run restart, partition, disk-full, key-rotation, snapshot, upgrade, and 72-hour soak drills.
- [ ] Retain signed, hashed evidence under `audit-artifacts/`.
- [ ] Commit the operational fixes and evidence.

### Task 9: Run external assurance and public testnet

**Files:**
- Modify only code and evidence required by audit findings; no launch gate may be relaxed.
- Publish audit, bounty, public-testnet, legal, and remediation records.

**Interfaces:**
- Two independent audits cover runtime/consensus/atomic execution and bridges/contracts/operations.
- The bug bounty is live for at least four weeks before launch.
- The public testnet has at least 50 validators across five continents for eight continuous weeks.

- [ ] Open audit and bounty engagements from the approved external providers.
- [ ] Run the public testnet program and archive its load, chaos, governance, and recovery results.
- [ ] Remediate all critical/high findings and re-audit fixes.
- [ ] Record legal approval and residual-risk acceptance for any medium finding.
- [ ] Commit evidence records only after their referenced gates pass.

### Task 10: Build, sign, and launch the release candidate

**Files:**
- Generate final plain/raw genesis and signed release artifacts.
- Generate and verify the schema-2 launch record.
- Update release notes and operator handoff documents.

**Interfaces:**
- Two independent reproducible builds agree byte-for-byte.
- Seven operator attestations reach the required supermajority.
- `mainnet-v1` contains no test verifier, no experimental consensus acceleration, and exactly the
  approved external routes.

- [ ] Build and compare the final artifacts on two independent machines.
- [ ] Run both release gates against the exact clean candidate commit.
- [ ] Run the genesis ceremony and verify the signed manifest.
- [ ] Launch with conservative caps and 24/7 monitoring.
- [ ] Execute the 30-day hypercare plan and stop exposure growth on any critical invariant failure.

