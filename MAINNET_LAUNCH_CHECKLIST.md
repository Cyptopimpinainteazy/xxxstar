# X3 Atomic Star — Mainnet Launch Checklist

> Use this document as the authoritative gate tracker before each launch milestone.
> Every item must be ✅ PASSED before proceeding to the next phase.
> No exceptions. No "mostly done". Binary pass/fail per item.

---

## PHASE 0 — Prerequisites (must be done first)

- [ ] `rust-toolchain.toml` is pinned to a specific Rust version (no `stable` floating)
- [ ] `Cargo.lock` is committed to the repository
- [ ] All direct and transitive deps pass `cargo audit`
- [ ] `deny.toml` configured and `cargo deny check` passes
- [ ] Repository has no secrets or private keys committed (run `git log --all --oneline | head -100` + secret scan)

---

## PHASE 1 — Node Binary (Real Chain, Not Mock)

- [ ] `cargo build --release -p x3-chain-node` completes without error
- [ ] Binary exists at `target/release/x3-chain-node`
- [ ] `./target/release/x3-chain-node --version` returns a versioned string
- [ ] `./scripts/start-x3-chain.sh` exits with error when binary is absent (no silent fallback)
- [ ] `./scripts/start-mock-rpc-dev.sh` clearly warns it is DEV ONLY
- [ ] Mock RPC (`mock-rpc-server.js`) is NOT called anywhere in the release path

---

## PHASE 2 — Single Dev Node

- [ ] Dev node starts with `--chain=dev --tmp`
- [ ] Block production begins within 10 seconds
- [ ] RPC endpoint responds at port 9933: `curl -s http://localhost:9933 -H 'Content-Type: application/json' -d '{"id":1,"jsonrpc":"2.0","method":"system_health","params":[]}' | jq`
- [ ] Block height is incrementing (not stuck)
- [ ] Node stops cleanly on SIGINT

---

## PHASE 3 — 3-Validator Local Testnet

- [ ] `./scripts/testnet-full-launch.sh` starts 3 validators without error
- [ ] All 3 validators are peered (check `system_peers` → count ≥ 2)
- [ ] GRANDPA finality is active (finalized block height advances)
- [ ] Aura round-robin block production observed (all 3 validators produce blocks)
- [ ] Testnet shuts down cleanly via `scripts/stop-testnet.sh` or Ctrl+C

---

## PHASE 4 — Cross-VM Transfer Proofs

All of these MUST pass via `cargo test -p pallet-x3-cross-vm-router`:

- [ ] `test_x3_native_evm_svm_roundtrip_preserves_supply` — PASS
- [ ] `test_all_six_internal_routes_succeed` — PASS
- [ ] `test_duplicate_nonce_rejected` — PASS
- [ ] `test_failed_destination_credit_refunds_pending_supply` — PASS (pending_supply returns to zero)
- [ ] `test_canonical_supply_never_breaks` — PASS (10-round stress, all 6 routes)
- [ ] `test_duplicate_message_replay_rejected` — PASS
- [ ] `test_expired_transfer_refunds_to_source` — PASS
- [ ] `test_cannot_cancel_before_expiry` — PASS
- [ ] `test_paused_asset_rejects_transfers` — PASS
- [ ] `test_closed_route_rejects_transfers` — PASS
- [ ] `test_zero_amount_rejected` — PASS
- [ ] `test_external_route_rejected_in_mvp` — PASS
- [ ] `external_bridges_are_paused_at_genesis` — PASS

---

## PHASE 5 — Supply Ledger Invariant

- [ ] `cargo test -p pallet-x3-supply-ledger` — ALL PASS
- [ ] `check_invariant()` is called in every supply-changing operation (grep confirms)
- [ ] No path exists where `represented_total > canonical_supply` (negative test present and passes)
- [ ] Overflow/underflow uses `checked_add`/`checked_sub` (no bare arithmetic on supply values)

---

## PHASE 6 — Security / No Production Panics

- [ ] Zero `unwrap()` calls on `Option` in non-test production code paths (run: `grep -r '\.unwrap()' node/src/ pallets/*/src/ --include='*.rs' | grep -v test | grep -v '#\[cfg(test' | grep -v '// safe'`)
- [ ] Zero `.expect(...)` that could panic from external input in production paths
- [ ] Zero `panic!()` calls outside of test code
- [ ] Zero `todo!()` in any path reachable from extrinsics
- [ ] `service.rs` mutex lock poisoning handled gracefully (returns error, does not crash node)
- [ ] All arithmetic in pallet dispatchables uses `saturating_*` or `checked_*` (not `+`/`-`/`*`/`/`)

---

## PHASE 7 — CI Hard Gates

- [ ] `.github/workflows/ci.yml` exists and contains all 10 jobs
- [ ] Branch protection on `main` requires `x3 / critical-path-all-pass`
- [ ] `cargo fmt --all -- --check` passes in CI
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` passes in CI
- [ ] `cargo check -p x3-chain-runtime` passes in CI
- [ ] `cargo check -p x3-chain-node` passes in CI
- [ ] `cargo test -p pallet-x3-cross-vm-router` passes in CI
- [ ] `cargo test -p pallet-x3-supply-ledger` passes in CI
- [ ] `cargo test -p pallet-x3-settlement-engine` passes in CI
- [ ] `cargo test -p pallet-x3-atomic-kernel` passes in CI

---

## PHASE 8 — Feature Scope Freeze Verification

- [ ] `ExternalBridgesEnabled` storage defaults to `false` in genesis config
- [ ] Attempting `set_external_bridges_enabled(true)` without Root origin fails
- [ ] Compile-time guards prevent `external-gateway + mainnet-rc1` (`compile_error!` macros)
- [ ] Compile-time guards prevent `parallel-executor + mainnet-rc1`
- [ ] No `BridgeRootRegistered` events emit on a fresh dev chain

---

## PHASE 9 — Documentation

- [ ] `README.md` has working "Quick Start" that builds and runs a real node
- [ ] `CURRENT_MAINNET_STATUS.md` is accurate (no aspirational claims)
- [ ] `docs/deployment/DEPLOYMENT_GUIDE.md` covers: build, chain spec, launch, connect wallet, send tx, observe settlement
- [ ] Every "TODO: post-RC1" or "TESTNET_ONLY" comment in source has a corresponding issue/milestone

---

## PHASE 10 — Public Testnet Launch Gates

- [ ] All Phases 0–9 above are ✅ PASSED
- [ ] Chain spec generated from `--chain x3-testnet` and published
- [ ] At least 3 independent validator operators configured from spec
- [ ] Block explorer connected and showing live blocks
- [ ] Faucet operational
- [ ] Slashing tested on testnet (intentional equivocation + observer confirms slash)
- [ ] `MAINNET_READINESS_PUSH_COMPLETE.md` updated with results
- [ ] Community announcement drafted

---

## PHASE 11 — External Security Audit

> Required before ANY value-bearing mainnet launch. No exceptions.

- [ ] External audit firm engaged and scope signed
- [ ] Audit scope covers **all of the following**:
  - [ ] `pallet-x3-cross-vm-router` — route dispatch, replay guards, supply invariant
  - [ ] `pallet-x3-supply-ledger` — canonical supply consistency, overflow guards
  - [ ] `pallet-x3-asset-registry` — registration gating, cross-domain asset IDs
  - [ ] `pallet-x3-account-registry` — account type enforcement, spoofing guards
  - [ ] `pallet-x3-atomic-kernel` — settlement finality, halt/resume path
  - [ ] Runtime upgrade flow — `on_runtime_upgrade`, storage version migration, `pre_upgrade`/`post_upgrade`
  - [ ] Genesis configuration — no dev seeds, correct initial supply allocation
  - [ ] `Dockerfile.mainnet-check` — reproducibility, no secrets in layers
- [ ] Audit report delivered and all Critical/High findings resolved
- [ ] All Medium findings either resolved or have accepted risk with written justification
- [ ] Audit report hash committed to `reports/external_audit_report.md` (can be redacted summary)
- [ ] Fix verification re-test completed by auditors
- [ ] `cargo audit` passes with current `advisory-db` at time of audit sign-off
- [ ] `cargo deny check` passes with current rules

---

## PHASE 12 — Launch Hardening

> Final hardening before mainnet genesis. These items lock the release.

### Bug Bounty

- [ ] Bug bounty program published on Immunefi or equivalent
- [ ] Scope document published (in-scope: cross-VM router, supply ledger, asset registry, atomic kernel, runtime upgrade)
- [ ] Minimum bounty reward tiers defined and funded
- [ ] Bug bounty program live for ≥ 7 days before genesis

### Emergency Response

- [ ] Emergency governance multisig configured (at least 3-of-5 keyholders)
- [ ] Multisig addresses published and verified on-chain
- [ ] Emergency contact list in `docs/EMERGENCY_CONTACTS.md`
- [ ] Incident response runbook at `docs/INCIDENT_RUNBOOK.md`
- [ ] Kill-switch / halt extrinsic tested on testnet
- [ ] Node operator alert channel established

### Supply and Asset Verification

- [ ] Genesis total supply verified: sum of all allocated accounts = canonical_supply
- [ ] Zero pre-mine exceptions undisclosed (all allocations documented)
- [ ] Vesting schedules (if any) verified in genesis config

### Reproducible Build

- [ ] `docker build --build-arg COMMIT=$(git rev-parse HEAD) -f Dockerfile.mainnet-check -t x3-mainnet-check .` exits 0
- [ ] WASM blob built with `srtool` (deterministic builder)
- [ ] `srtool` output hash matches hash in genesis chain spec
- [ ] `release_hashes.txt` updated with final artifact hashes (node binary, WASM, chain spec)
- [ ] All artifacts signed with release key and signatures published

### External Bridges (final lockout check)

- [ ] `ExternalBridgesEnabled` = false in raw genesis chain spec (manual inspection)
- [ ] Governance proposal to enable external bridges requires 2/3 supermajority (spec confirmed)
- [ ] No pending bridge enable proposals in genesis config

---

## PHASE 13 — Genesis Ceremony

> The genesis ceremony is a one-time irreversible event. Complete all prior phases first.

- [ ] All Phase 0–12 items ✅ PASSED
- [ ] `scripts/mainnet/genesis_ceremony.sh` executed to completion
- [ ] Genesis chain spec (`chain-specs/x3-mainnet-genesis.json`) generated from clean srtool build
- [ ] Genesis WASM hash verified against `release_hashes.txt`
- [ ] Genesis chain spec converted to raw format
- [ ] Genesis raw spec SHA256 computed and recorded: `sha256sum chain-specs/x3-mainnet-genesis-raw.json`
- [ ] Genesis spec published to public URL (IPFS + GitHub release)
- [ ] At least 3 independent bootnodes confirmed reachable from external network
- [ ] Frozen commit hash tagged: `git tag -a mainnet-genesis-v1.0.0 -m "Genesis commit"`
- [ ] Tag pushed and signed: `git push --tags`
- [ ] `CURRENT_MAINNET_STATUS.md` updated to reflect genesis block number and timestamp
- [ ] Block explorers updated to mainnet genesis spec

---

## PHASE 7 — Token and NFT Launchpads

> Verifies that launchpad, auction, and token-factory pallets use shared treasury,
> governance, dispute, and compliance rails. Gate script: `scripts/mainnet/phase7_launchpad_gate.sh`.

### Pallet Build and Tests

- [ ] `pallet-x3-launchpad` compiles clean (`cargo check -p pallet-x3-launchpad`)
- [ ] `pallet-x3-auction` compiles clean (`cargo check -p pallet-x3-auction`)
- [ ] `pallet-x3-token-factory` compiles clean (`cargo check -p pallet-x3-token-factory`)
- [ ] All three pallet test suites pass (`cargo test -p pallet-x3-launchpad` etc.)

### Treasury and Compliance Rails

- [ ] Launchpad uses cap-based settlement (hard_cap enforces treasury protection)
- [ ] Auction pallet has `settle_auction` and `force_cancel` dispute paths
- [ ] Governance-origin guard on sensitive launchpad/auction operations confirmed
- [ ] ≥3 LAUNCH-* invariants documented in `pallets/x3-launchpad/src/lib.rs`
- [ ] ≥2 AUCTION-* invariants documented in `pallets/x3-auction/src/lib.rs`

### Security

- [ ] No dev-seed accounts present in chain spec (`//Alice`, `//Bob`, etc.)
- [ ] `cargo deny check advisories` passes with zero CVSS ≥ 7 findings
- [ ] Cross-VM supply invariant test present (`test_02_native_to_evm_preserves_invariant`)

### Gate

- [ ] `bash scripts/mainnet/phase7_launchpad_gate.sh` exits 0
- [ ] `reports/phase7_launchpad_gate.md` shows `phase7_launchpad_gate: PASS`

---

## PHASE 8 — dApp Hub and Revenue-Sharing SDK

> Verifies that the dApp hub provides stable third-party-inheritable fee/evidence
> primitives with platform-level listing, throttling, and incident policy.
> Gate script: `scripts/mainnet/phase8_dapp_hub_gate.sh`.

### Pallet and Crate Build

- [ ] `pallet-x3-dapp-hub` compiles clean (`cargo check -p pallet-x3-dapp-hub`)
- [ ] `x3-revenue-sharing` crate compiles clean (`cargo check -p x3-revenue-sharing`)
- [ ] dApp hub test suite passes (`cargo test -p pallet-x3-dapp-hub`)

### Business Logic Invariants

- [ ] Revenue split validation enforces exactly 10 000 bps (`validate_split` check)
- [ ] Pending→Approved→Suspended lifecycle enforced in `pallet-x3-dapp-hub`
- [ ] DAPP-002 guard tests pass (`revenue_rejected/suspended/pending_dapp_fails`)
- [ ] Non-governance calls to approve/reject/suspend fail (governance-origin tests)
- [ ] Revenue policy invalid-sum test passes
- [ ] `withdraw_earnings` test passes (DAPP-006)

### SDK Surface

- [ ] `packages/x3-marketplace-sdk/src/` has ≥1 file (SDK not empty scaffold)
- [ ] ≥6 DAPP-* invariants documented in `pallets/x3-dapp-hub/src/lib.rs`

### Gate

- [ ] `bash scripts/mainnet/phase8_dapp_hub_gate.sh` exits 0
- [ ] `reports/phase8_dapp_hub_gate.md` shows `phase8_dapp_hub_gate: PASS`

---

## PHASE 9 — User-Facing Triangle

> Verifies that desktop app, browser extension, and web portal are unified under
> one identity, asset, and treasury model via `apps/shared`.
> Gate script: `scripts/mainnet/phase9_user_triangle_gate.sh`.

### Shared Canonical Model

- [ ] `apps/shared/config/chain.ts` is the single chain config source (no duplicate `chain.ts` in surface apps)
- [ ] `apps/shared/hooks/useWalletConnection.ts` is the single wallet identity hook
- [ ] `apps/shared/hooks/useChainSubscription.ts` is the single live-chain subscription hook
- [ ] `apps/shared/index.ts` exports `config`, `hooks`, and `components`

### Surface Correctness

- [ ] `apps/x3-desktop` has ≥1 page (`src/pages/*.tsx`)
- [ ] `apps/x3-extension` has `background.ts` and `popup.ts` (both surfaces present)
- [ ] `apps/x3-intelligence/src/pages` has ≥4 views (proof, governance, analytics, agents)
- [ ] No surface defines its own chain config or standalone wallet hook (drift check passes)

### Build Integrity

- [ ] `apps/x3-desktop` TypeScript compiles with zero `error TS` lines
- [ ] `apps/x3-extension` TypeScript compiles with zero `error TS` lines
- [ ] `apps/x3-intelligence` TypeScript compiles with zero `error TS` lines

### Gate

- [ ] `bash scripts/mainnet/phase9_user_triangle_gate.sh` exits 0
- [ ] `reports/phase9_user_triangle_gate.md` shows `phase9_user_triangle_gate: PASS`

---

## PHASE 10 — AI Swarm as a Service

> Verifies that bot rental, compute marketplace, analytics APIs, and incident controls
> inherit canonical accounting and operator truth — no separate service rails.
> Gate script: `scripts/mainnet/phase10_ai_swarm_gate.sh`.

### Governance and Evidence

- [ ] `x3-security-swarm/governance/charter.md` defines swarm authority bounds
- [ ] `x3-security-swarm/governance/quorum.rules` defines signer quorum requirements
- [ ] `x3-security-swarm/governance/appeals.yaml` defines incident escalation path
- [ ] `x3-security-swarm/evidence/retention.policy` defines evidence retention for swarm actions

### Intelligence API Service

- [ ] `apps/x3-intelligence/server.js` has `/health` endpoint (SRE requirement)
- [ ] Analytics API endpoints present: `/api/v1/floor/stats`, `/api/v1/intents`, `/api/v1/agents`
- [ ] `/api/v1/disputes` endpoint feeds canonical incident system (no separate incident pipe)
- [ ] `/api/v1/validator/metrics` endpoint present (compute billing usage signal)

### Accounting Integrity

- [ ] `x3-swarm-orchestra/` contains no standalone `total_supply` / `fn mint` / `fn burn` (no duplicate ledger)
- [ ] All service billing events consumed from chain events (not swarm-local storage)

### E2E Workflows

- [ ] `x3-swarm-orchestra/tests/e2e_user_test.py` exists and exits 0 (`python3 e2e_user_test.py`)
- [ ] E2E test covers: arbitrage, AI content, court-dispute, and human-CRM paths (6/6 workflows)

### Gate

- [ ] `bash scripts/mainnet/phase10_ai_swarm_gate.sh` exits 0
- [ ] `reports/phase10_ai_swarm_gate.md` shows `phase10_ai_swarm_gate: PASS`

---

## PHASE 11 — RC4 Runtime Upgrade Rehearsal

> Required before ANY value-bearing mainnet launch. No exceptions.

- [x] RC4 runtime upgrade rehearsal: PASS (automation, report, and evidence all consistent)
- [ ] RC5: Not started

