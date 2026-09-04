# Phase 5 Execution Tickets: Wrapped X3 & Omnichain Governance

## Ticket Structure

Each ticket is a thin vertical slice targeting a specific surface artifact with measurable test coverage. Tickets 1-8 are buildable in parallel after Ticket 0 is done. Tickets 9-12 can run after 1-8 complete.

---

## Ticket 0: State Registry Foundation (4-5 hours)

**Outcome**: Canonical state registry with wrapped supply tracking per chain, governance power mapping, bridge transaction logs, and reconciliation ledger.

**Location**: New crate `crates/wrapped-token-registry/`

**Components**:

### types.rs (280 lines)
- `WrappedTokenState` struct: canonical_total, per_chain_wrapped, bridge_fees_collected, last_reconciliation_ms
- `GovernancePowerRecord` struct: holder, canonical_power, chain_delegations, total_registered_power
- `BridgeTransaction` enum: Mint(chain, amount, proof_id) | Burn(chain, amount, proof_id)
- `ReconciliationResult`: divergence_pct, status (OK|Warning|Diverged|RecoveryInProgress), timestamp

### registry.rs (400 lines)
- `WrappedTokenRegistry` trait (async): register_mint, register_burn, reconcile_supply, update_governance_power, get_canonical_total, get_chain_wrapped_supply, query_bridge_txn
- `InMemoryRegistry` implementation with RwLock<HashMap> storage
- Reconciliation cycle: queries all chains, computes divergence, logs result
- Governance power aggregation: sums votes across all chains

### tests
- 4 tests: mint registration, burn settlement, supply reconciliation within tolerance, divergence detection

**Dependencies**: tokio, serde, chrono (same as custody-service)

---

## Ticket 1: Wrapped Token Types & Errors (2-3 hours)

**Outcome**: Typed definitions for wrap/unwrap operations, mint/burn rules, and bridge-specific errors.

**Location**: `crates/wrapped-token-core/src/`

**Components**:

### types.rs (350 lines)
- `MintRequest` struct: destination_chain, amount, caller, source_lock_proof, priority
- `BurnRequest` struct: source_chain, amount, caller, settlement_destination, proof_id
- `BridgeType` enum: InterChainLock, SidecarValidator, FastProof
- `BridgeCapacity` struct: daily_cap, used_today, reset_time_ms
- `GovernanceProposal` struct: id, author, title, chain_votes, canonical_vote_needed, status
- `StakingDerivative` struct: liquid_token_address, underlying_x3_amount, provider_chain

### error.rs (50 lines)
- `WrappedTokenError` enum: 12 variants
  - MintCapExceeded
  - BurnProofNotFound
  - GovernancePowerDivergence
  - SupplyReconciliationFailed
  - BridgeValidatorQuorumLost
  - InvalidBridgeType
  - StakingDerivativeMismatch
  - SettlementTimeout
  - UnauthorizedSigner
  - ChainNotSupported
  - InsufficientBridgeCapacity
  - GovernanceNotDelegated

### tests
- 3 tests: request validation, error type correctness, struct initialization

---

## Ticket 2: Bridge Type 1 — Lock & Mint (6-7 hours)

**Outcome**: Inter-chain lock-and-mint bridge with escrow, settlement validation, and rollback mechanics.

**Location**: `crates/wrapped-token-core/src/bridge_lock_mint.rs`

**Components**:

### LockedMintBridge struct (300 lines)
- `custody: CustodyServiceClient` (from Phase 4.5)
- `registry: WrappedTokenRegistry`
- `lock_window_ms: u64` (default 90 min)
- `settlement_timeout_ms: u64` (default 30 min)

Methods:
- `initiate_lock(canonical_chain, amount, caller) -> LockProof`
  - Calls custody service to lock X3 in escrow vault
  - Records lock in registry
  - Returns proof ID and settlement window
- `mint_after_settlement(destination_chain, lock_proof) -> Result<MintReceipt>`
  - Verifies lock proof age < settlement_timeout
  - Checks destination chain bridge capacity
  - Mints wrapped tokens on destination
  - Records mint in registry
  - Returns receipt with wrapped token address and amount
- `rollback_failed_settlement(lock_proof) -> Result<UnlockReceipt>`
  - Releases escrowed canonical X3 back to caller
  - Clears lock from registry
  - Returns unlock proof

### tests
- 4 tests: lock initiation, mint after lock confirms, settlement timeout rollback, escrow release

---

## Ticket 3: Bridge Type 2 — Sidecar Validators (7-8 hours)

**Outcome**: Multi-sig sidecar validator consensus for burn-and-mint, with quorum validation.

**Location**: `crates/wrapped-token-core/src/bridge_sidecar.rs`

**Components**:

### SidecarBridge struct (350 lines)
- `validators: Vec<ValidatorNode>` with RwLock
- `quorum_size: usize` (e.g., 3 of 5)
- `consensus_timeout_ms: u64` (default 20 min)
- `registry: WrappedTokenRegistry`

Methods:
- `observe_burn(source_chain, burn_txn_id, caller, amount) -> BurnObservation`
  - Records burn event from source chain
  - Broadcasts observation to all validators
  - Waits for quorum of validator signatures
  - Returns consensus proof
- `mint_after_consensus(destination_chain, consensus_proof) -> Result<MintReceipt>`
  - Validates quorum signatures
  - Verifies consensus_proof age < settlement_timeout
  - Checks destination bridge capacity
  - Mints wrapped tokens
  - Records mint in registry
  - Returns receipt
- `handle_validator_outage(validator_id) -> RecoveryAction`
  - Removes failed validator from active set
  - If quorum drops below threshold, triggers fallback to Type 1
  - Notifies governance
  - Returns recovery plan

### ValidatorNode struct (100 lines)
- `id: String`
- `endpoint: String`
- `signing_key: Vec<u8>`
- `health_status: ValidatorHealth` enum (Healthy|Degraded|Failed)
- `last_heartbeat_ms: u64`

### tests
- 4 tests: consensus achievement, quorum enforcement, validator outage fallback, mint after consensus

---

## Ticket 4: Wrapped Token State Machine (5-6 hours)

**Outcome**: Lifecycle management for mint/burn operations with state transitions and recovery paths.

**Location**: `crates/wrapped-token-core/src/state_machine.rs`

**Components**:

### MintLifecycle state machine (200 lines)
States: Initiated → SettlementPending → Confirmed → Minted → Complete | Rolled Back
- On Initiated: lock is escrowed or consensus requested
- On SettlementPending: waiting for proof confirmation
- On Confirmed: proof is validated
- On Minted: tokens created on destination
- On Complete: no more transitions
- On RolledBack: canonical tokens returned to user

### BurnLifecycle state machine (200 lines)
States: Initiated → SettlementPending → Confirmed → Burned → Complete | Failed
- On Initiated: user initiates burn request
- On SettlementPending: waiting for settlement authorization
- On Confirmed: settlement confirmed
- On Burned: wrapped tokens destroyed
- On Complete: settlement complete
- On Failed: wrapped tokens restored, retry queued

### WrappedTokenCoordinator struct (250 lines)
- Manages both mint and burn state machines
- `mint_in_flight: HashMap<proof_id, MintState>`
- `burn_in_flight: HashMap<proof_id, BurnState>`
- `per_chain_daily_mint_cap: HashMap<chain_id, u128>`
- Enforces caps, timeouts, and retry logic

Methods:
- `initiate_mint(req) -> Result<MintProof>`
- `confirm_mint(proof, settlement_result) -> Result<MintReceipt>`
- `rollback_mint(proof, reason) -> Result<RollbackReceipt>`
- `initiate_burn(req) -> Result<BurnProof>`
- `confirm_burn(proof, settlement_result) -> Result<BurnReceipt>`
- `recover_stuck_operation(proof) -> RecoveryAction`

### tests
- 5 tests: mint state transitions, burn state transitions, timeout recovery, cap enforcement, stuck operation recovery

---

## Ticket 5: Supply Reconciliation Engine (6-7 hours)

**Outcome**: Deterministic daily reconciliation with divergence detection, recovery coordination, and immutable audit log.

**Location**: `crates/wrapped-token-core/src/reconciliation.rs`

**Components**:

### ReconciliationEngine struct (400 lines)
- `registry: WrappedTokenRegistry`
- `coordinator: WrappedTokenCoordinator`
- `chains: Vec<ChainPoller>` (futures for querying each chain in parallel)
- `tolerance_pct: f64` (default 0.01%)
- `recovery_timeout_ms: u64` (default 60 min)
- `audit_log: Vec<ReconciliationRecord>`

Methods:
- `run_reconciliation() -> Result<ReconciliationReport>`
  1. Spawn parallel queries to all chains
  2. Collect wrapped supplies
  3. Query canonical treasury holding
  4. Calculate totals: canonical + sum(wrapped)
  5. Compute divergence_pct = abs(observed - expected) / expected
  6. If divergence_pct <= tolerance: mark OK and return
  7. If divergence_pct > tolerance: enter recovery mode
  8. Record result in audit log
  
- `recover_divergence(divergence) -> RecoveryPlan`
  1. Audit recent mint/burn operations for errors
  2. Check IN-FLIGHT settlements (may explain divergence)
  3. Identify diverged chain(s)
  4. Classify divergence as IN-FLIGHT or PERMANENT
  5. If IN-FLIGHT: extend settlement window
  6. If PERMANENT: escalate to treasury/governance
  7. Return recovery plan with owner and timeline

- `execute_recovery(plan) -> Result<ReconciliationReport>`
  - Implements recovery steps
  - Re-runs reconciliation after recovery
  - Logs all recovery actions immutably

### ReconciliationRecord struct (100 lines)
- timestamp_ms, total_canonical, total_wrapped, divergence_pct, status, recovery_plan, proof_hash

### tests
- 5 tests: successful daily reconciliation, divergence detection within tolerance, divergence exceeding tolerance with recovery, recovery completion, audit log immutability

---

## Ticket 6: Governance Power & Delegation (6-7 hours)

**Outcome**: Canonical governance power registration, per-chain delegation, and aggregated voting with power divergence protection.

**Location**: `crates/wrapped-token-core/src/governance.rs`

**Components**:

### GovernancePowerRegistry struct (350 lines)
- `canonical_powers: RwLock<HashMap<Address, u128>>` (canonical holder → voting power)
- `chain_holdings: RwLock<HashMap<(chain_id, Address), u128>>` (wrapped holdings per chain)
- `delegations: RwLock<HashMap<Address, Address>>` (from → to delegatee)
- `divergence_check_interval_ms: u64` (default 1 hour)
- `tolerance_pct: f64` (default 1.0%)

Methods:
- `register_holder_canonical(holder, amount) -> Result<GovernancePowerRecord>`
  - Record canonical X3 holding
  - Calculate voting power = amount
  - Return power record
  
- `register_chain_wrapped(chain_id, holder, amount) -> Result<ChainPowerRecord>`
  - Record wrapped holdings on specific chain
  - Auto-grant same canonical voting power
  - Check divergence
  - Return chain power record

- `delegate_power(from, to) -> Result<DelegationProof>`
  - Allow holder to delegate to another address
  - Delegation applies globally across all chains
  - Record delegation in canonical registry
  - Return proof

- `check_governance_power_divergence() -> Result<DivergenceReport>`
  - Calculate canonical_total_power = sum(all canonical holdings)
  - Calculate chain_total_power = sum(all chain holdings)
  - Compute divergence = abs(canonical_total - chain_total) / canonical_total
  - If divergence > tolerance_pct: alert and suspend governance
  - Return report with divergence details

- `aggregate_proposal_votes(proposal_id) -> Result<ProposalResult>`
  - Sum votes from all chains
  - Weight by chain wrapped supply
  - Verify canonical power ≈ sum(chain powers)
  - Return final vote tally and result

- `resolve_divergence(divergence_report) -> Result<ResolutionProof>`
  - Manual governance review
  - Adjust canonical or chain records
  - Re-check after adjustment
  - Return resolution proof

### tests
- 5 tests: canonical power registration, chain wrapped registration, delegation, divergence detection & alert, proposal voting with power aggregation

---

## Ticket 7: Staking Integration (5-6 hours)

**Outcome**: Staking mechanics for wrapped X3 with liquid staking support, vault integration, and staking rewards.

**Location**: `crates/wrapped-token-core/src/staking.rs`

**Components**:

### StakingManager struct (300 lines)
- `staking_pools: RwLock<HashMap<chain_id, StakingPool>>`
- `liquid_derivatives: Vec<LiquidTokenAdapter>` (providers like Lido for stX3)
- `vault_staker: VaultStakingProxy` (treasury vault staking strategy)
- `rewards_distributor: RewardsDistributor`
- `daily_sync_interval_ms: u64`

Methods:
- `stake(chain_id, holder, amount, lock_period) -> Result<StakingProof>`
  - Lock wrapped tokens in staking pool
  - Grant voting power (not removed)
  - Enqueue holder for rewards
  - Return staking proof
  
- `unstake(chain_id, holder, proof_id) -> Result<UnstakeReceipt>`
  - After lock period, unlock tokens
  - Collect accrued rewards
  - Reinstate transfer permissions
  - Return receipt with rewards amount

- `reconcile_liquid_derivatives(provider) -> Result<ReconciliationReport>`
  - Query liquid token supply from provider
  - Compare to underlying X3 balance
  - If divergence: alert provider and halt derivative minting
  - Return reconciliation report
  
- `sync_governance_power_staked() -> Result<GovernancePowerAdjustment>`
  - Confirm staked tokens retain full governance power
  - Sync with governance registry
  - Return adjustment proof

- `execute_vault_staking_rebalance() -> Result<RebalanceReceipt>`
  - Treasury vault can stake for yield on approved chains
  - Respects exposure caps and daily rebalance limits
  - Returns receipt with vault new staking position

### LiquidTokenAdapter trait (100 lines)
- `query_underlying_balance(holder_address) -> Result<u128>`
- `query_total_supply() -> Result<u128>`
- `reconcile_daily() -> Result<ReconciliationReport>`

### StakingPool struct (100 lines)
- chain_id, locked_amount, reward_rate, lock_period_min_ms, holders (RwLock<HashMap>)

### tests
- 4 tests: staking lock and unlock, liquid derivative reconciliation, governance power remains during staking, vault staking strategy execution

---

## Ticket 8: Bridge Fee Routing & Collection (4-5 hours)

**Outcome**: Per-corridor bridge fee schedule, collection, accounting, and treasury routing.

**Location**: `crates/wrapped-token-core/src/bridge_fees.rs`

**Components**:

### BridgeFeeSchedule struct (200 lines)
- `corridors: HashMap<(chain_from, chain_to), FeePolicy>`
- `FeePolicy { base_bps: u16, per_mm_bps: u16, min_fee, max_fee }`
- Example: US→EU might be 15 bps base + 5 bps per million
- Can be updated by governance

Methods:
- `calculate_fee(from_chain, to_chain, amount) -> Result<u128>`
  - Lookup corridor
  - Calculate: base + per_mm component
  - Clamp to [min_fee, max_fee]
  - Return fee in bps-equivalent stable units
  
- `apply_fee(amount) -> (net_amount, fee_amount)`
  - Deduct fee from user amount
  - Return net for bridging and fee for collection

### BridgeFeeCollector struct (200 lines)
- `schedule: BridgeFeeSchedule`
- `custody_client: CustodyServiceClient` (from Phase 4.5)
- `collected_by_corridor: RwLock<HashMap<Corridor, u128>>` (per-corridor fee accumulator)
- `cumulative_collected: RwLock<u128>` (total fees ever collected)

Methods:
- `collect_fee(from_chain, to_chain, amount) -> Result<FeeReceipt>`
  - Calculate fee
  - Transfer fee to treasury reserve vault
  - Record in per-corridor bucket
  - Return receipt with fee_id and proof
  
- `report_corridor_fees(from_chain, to_chain) -> CorridorFeeReport`
  - Total fees for corridor this period
  - Fee rate breakdown (base vs. per-mm)
  - Most common amount ranges
  - Return report for compliance

- `report_total_fees(start_ms, end_ms) -> TreasuryFeeReport`
  - Aggregate fees across all corridors in time window
  - Return final treasury accrual value
  - Immutably record in audit log

### tests
- 4 tests: fee calculation for various corridors, fee collection and custody routing, corridor reporting, treasury audit report

---

## Ticket 9: Chain-Specific Bridge Integrations (5-6 hours each, run after 1-8)

**Outcome**: Working bridge integrations for at least 2 major chains (e.g., Ethereum + Cosmos).

**Chains**: Ethereum (Type 1), Cosmos (Type 2), Secondary (flexible)

**Per-Chain**: (100-150 lines each)
- Chain-specific RPC client configuration
- Wrapped token contract address
- Bridge contract/escrow address
- Signer key setup
- Integration tests that call real testnet endpoints

---

## Ticket 10: Settlement & Rollback Testing (6-7 hours)

**Outcome**: End-to-end test suite covering all failure modes, recoveries, and settlement timeouts.

**Location**: `crates/wrapped-token-core/tests/integration_tests/`

**Test Suite** (50+ test cases):
- Successful mint-burn round-trip
- Settlement timeout and rollback
- Governance power divergence detection and resolution
- Bridge fee calculation and routing
- Staking lock/unlock with rewards
- Liquid derivative reconciliation
- Supply reconciliation within and exceeding tolerance
- Validator outage fallback
- Vault staking rebalance

**Test Coverage Goal**: 85%+ line coverage across all modules

---

## Ticket 11: Documentation & Audit Readiness (4-5 hours)

**Outcome**: Operator runbooks, audit-ready docs, and security summary.

**Deliverables**:
- `WRAPPED_X3_OPERATOR_RUNBOOK.md` (failure modes, remediation steps, escalation paths)
- `WRAPPED_X3_COMPLIANCE_GUIDE.md` (regulatory mapping, jurisdiction coverage, audit trail requirements)
- `WRAPPED_X3_SECURITY_AUDIT_CHECKLIST.md` (pre-audit validation steps)
- API documentation (rustdoc on all public types and methods)

---

## Ticket 12: Phase 5 Exit Gate Validation (3-4 hours)

**Outcome**: Validate all Phase 5 exit gates and produce signed completion report.

**Exit Gates to Verify**:
- ✅ Wrapped X3 live on 2+ chains
- ✅ Daily reconciliation divergence < 0.01%
- ✅ Bridge validators running 30 days without failure
- ✅ Governance aggregation correct across chains
- ✅ All recovery modes tested and latencies within bounds
- ✅ 500+ bridge transactions processed without loss
- ✅ Compliance audit and jurisdictional filings complete

**Deliverable**: Phase 5 completion report signed by architecture lead and security reviewer

---

## Dependency Graph

```
Ticket 0 (Registry Foundation)
    ↓
Tickets 1-4 (Types, Bridges, State Machine) — parallel
Ticket 1 ↓
Ticket 2 ↓
Ticket 3 ↓
Ticket 4 ↓
    ↓
Tickets 5-8 (Reconciliation, Governance, Staking, Fees) — parallel after 1-4
Ticket 5 ↓
Ticket 6 ↓
Ticket 7 ↓
Ticket 8 ↓
    ↓
Tickets 9-10 (Integration, Testing) — after 1-8
Ticket 9 ↓
Ticket 10 ↓
    ↓
Tickets 11-12 (Documentation, Gate Validation) — after 9-10
```

**Timeline Estimate**: 55-65 hours for sequential phase, 35-40 hours with full parallelization

---

## Acceptance Criteria

Each ticket is done when:
- All specified code files are created and compile without errors
- All specified methods are implemented
- All specified tests pass
- Code follows Rust idioms and handles all error cases
- `cargo fmt` and `cargo clippy` pass
- Documentation is complete (rustdoc + markdown guide)
