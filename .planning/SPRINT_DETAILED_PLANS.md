# 🚀 V0.4 IMPLEMENTATION ROADMAP — DETAILED SPRINT PLANS

**Generated:** April 26, 2026  
**Target:** 5-month implementation cycle (20 weeks)  
**Scope:** 8 major modules, ~23,000 LOC, testnet milestone

---

## 📋 SPRINT 0: FOUNDATION AUDIT (Week 1)

**Goal:** Verify kernel is production-grade; establish readiness report infrastructure  
**Team Size:** 2 engineers  
**Deliverable:** Kernel audit complete + readiness-report crate scaffolded

### Phase 0.1: Canonical Supply Invariant Audit
**Time:** 3 days | **Owner:** Core team

**Checklist:**
- [ ] Fuzz test: `canonical_supply == native + evm + svm + x3vm + external_locked + pending` with 10k random transactions
- [ ] Test coverage: All balance ledger update paths
- [ ] Edge cases: Multiple domains, partial transfers, failed swaps, bridge timeouts
- [ ] Document: Invariant proof in `docs/CANONICAL_SUPPLY_INVARIANT.md`

**Files to modify:**
- `tests_phase4/invariant_registry_check.rs` — add fuzz harness
- `pallets/x3-kernel/src/tests.rs` — add 10 new test cases

**Expected Result:** Zero invariant violations across 10k random ops.

---

### Phase 0.2: Emergency Halt Path Verification
**Time:** 2 days | **Owner:** Core team

**Checklist:**
- [ ] Verify: Governance can call `set_emergency_halt(true)` and halt ALL transfers
- [ ] Verify: No balance can be minted/burned/transferred while halted
- [ ] Verify: Halt state is persisted and checked on every pallet call
- [ ] Test: Emergency halt + resume cycle

**Files to check:**
- `pallets/x3-kernel/src/lib.rs` — emergency_halt logic
- `runtime/src/lib.rs` — governance integration

**Expected Result:** Emergency halt tested + documented.

---

### Phase 0.3: Mint/Burn Permission Audit
**Time:** 2 days | **Owner:** Core team

**Checklist:**
- [ ] Only `x3-kernel` pallet can mint canonical balances
- [ ] Only `x3-supply-ledger` can burn (after balance verification)
- [ ] No direct storage writes to balance fields
- [ ] All mint/burn operations update canonical supply simultaneously

**Files to audit:**
- `pallets/x3-kernel/src/lib.rs` — mint guards
- `pallets/x3-supply-ledger/src/lib.rs` — burn guards
- All bridge/gateway pallets — verify they call kernel, not direct storage

**Expected Result:** Permissions audit report in `docs/MINT_BURN_AUDIT.md`.

---

### Phase 0.4: Cross-Domain Balance Reconciliation
**Time:** 2 days | **Owner:** Core team

**Checklist:**
- [ ] Native (Substrate) balances reconcile
- [ ] EVM balances reconcile
- [ ] SVM balances reconcile
- [ ] X3VM balances reconcile
- [ ] External locked balances reconcile
- [ ] Pending settlement balances reconcile
- [ ] All 6 sum to canonical supply

**Files to check:**
- `pallets/x3-asset-registry/src/lib.rs` — balance tracking
- `pallets/x3-cross-vm-router/src/lib.rs` — domain reconciliation

**Expected Result:** Reconciliation script in `scripts/verify_balance_reconciliation.sh`.

---

### Phase 0.5: Readiness Report Crate Scaffold
**Time:** 2 days | **Owner:** Core team

**Create:** `crates/x3-readiness-report/`

**Structure:**
```
x3-readiness-report/
├── src/
│   ├── lib.rs           # Main readiness collector
│   ├── kernel_checks.rs # Kernel health metrics
│   ├── gateway_checks.rs # Gateway readiness
│   ├── consensus_checks.rs # Validator/consensus health
│   ├── invariants.rs    # Invariant verification
│   └── tests.rs
├── Cargo.toml
└── README.md
```

**Checklist:**
- [ ] Define `ReadinessReport` struct with all metrics
- [ ] Implement `collect_kernel_health()` 
- [ ] Implement `collect_gateway_health()`
- [ ] Implement `check_critical_invariants()`
- [ ] Add CLI to read current readiness state
- [ ] Add tests

**Files to create:**
- `crates/x3-readiness-report/src/lib.rs` (500 LOC)
- `crates/x3-readiness-report/src/kernel_checks.rs` (300 LOC)
- `crates/x3-readiness-report/src/gateway_checks.rs` (200 LOC)

**Expected Result:** Readiness crate compiles + provides `fn get_readiness() -> ReadinessReport`.

---

## 📋 SPRINT 1: X3 PACKET STANDARD (Weeks 2–3)

**Goal:** Define cross-chain packet protocol with replay protection, timeouts, refunds  
**Team Size:** 2 engineers  
**Deliverable:** `crates/x3-packet-standard/` battle-tested, wired into pallet

### Phase 1.1: Packet Type Definition
**Time:** 3 days | **Owner:** Protocol team

**Create:** `crates/x3-packet-standard/src/packet.rs`

**Code Target:**
```rust
pub enum X3Packet {
    AssetLock {
        source_chain: ChainId,
        dest_domain: DomainId,
        asset_id: AssetId,
        amount: Balance,
        sender: AccountBytes,
        recipient: AccountBytes,
        sequence: u128,
        nonce: u128,
        timeout_height: BlockNumber,
        timeout_timestamp: u64,
        proof_hash: H256,
        relayers: Vec<ValidatorId>,
    },
    AssetUnlock { /* ... */ },
    CanonicalMint { /* ... */ },
    CanonicalBurn { /* ... */ },
    SwapIntent { /* ... */ },
    BridgeAttestation { /* ... */ },
    FailedExecution { /* ... */ },
    Refund { /* ... */ },
}

impl X3Packet {
    pub fn compute_id(&self) -> PacketId;
    pub fn is_expired(&self, current_block: BlockNumber) -> bool;
    pub fn verify_signature_quorum(&self, min_signatures: u32) -> bool;
}
```

**Deliverable:**
- [ ] Packet enum with all 8 types
- [ ] Field validation per type
- [ ] Serialization/deserialization tests

---

### Phase 1.2: Replay Protection
**Time:** 4 days | **Owner:** Security team

**Create:** `crates/x3-packet-standard/src/replay.rs`

**Code Target:**
```rust
pub struct ReplayProtectionMap {
    // (source_domain, sender, sequence) → proof_hash
    executed: BTreeMap<(DomainId, AccountBytes, u128), H256>,
}

impl ReplayProtectionMap {
    pub fn submit_packet(&mut self, packet: &X3Packet) -> Result<(), ReplayError>;
    pub fn verify_not_replayed(&self, packet_id: &PacketId) -> Result<(), ReplayError>;
    pub fn prune_old_entries(&mut self, horizon_blocks: u64);
}
```

**Tests:**
- [ ] Same packet submitted twice → rejected
- [ ] Packet with same sequence, different sender → accepted
- [ ] Packet with different sequence, same sender → accepted
- [ ] Pruning removes expired entries

---

### Phase 1.3: Timeout & Refund State Machine
**Time:** 4 days | **Owner:** Protocol team

**Create:** `crates/x3-packet-standard/src/timeout.rs`

**Code Target:**
```rust
pub enum PacketState {
    Pending { submitted_at: BlockNumber },
    Locked { lock_at: BlockNumber },
    Settled { settled_at: BlockNumber },
    Failed { reason: String, failed_at: BlockNumber },
    Refunded { refunded_at: BlockNumber },
}

pub struct PacketTimeout {
    timeout_height: BlockNumber,
    timeout_timestamp: u64,
    fast_chain_deadline: BlockNumber,
    slow_chain_deadline: BlockNumber,
}

impl PacketTimeout {
    pub fn is_expired(&self, current_block: BlockNumber) -> bool;
    pub fn can_refund(&self, current_block: BlockNumber) -> bool;
}
```

**Tests:**
- [ ] Packet expires after timeout_height
- [ ] Refund auto-triggers at timeout
- [ ] Fast chain timeout < slow chain timeout
- [ ] Can refund once expired

---

### Phase 1.4: Proof & Signature Verification
**Time:** 3 days | **Owner:** Crypto team

**Create:** `crates/x3-packet-standard/src/proof.rs`

**Code Target:**
```rust
pub fn compute_packet_hash(packet: &X3Packet) -> H256;
pub fn verify_relayer_signatures(
    packet: &X3Packet,
    signatures: &[Signature],
    min_quorum: u32,
) -> Result<(), VerifyError>;
pub fn emit_proof_for_packet(packet_id: &PacketId) -> ProofEmission;
```

**Tests:**
- [ ] Hash is deterministic
- [ ] Signatures verified correctly
- [ ] Quorum check works

---

### Phase 1.5: Integration with Pallet
**Time:** 3 days | **Owner:** Pallet team

**Create:** `pallets/x3-packet-registry/` (new pallet)

**Code:**
```rust
pub mod pallet {
    #[pallet::storage]
    pub type Packets<T: Config> = 
        StorageMap<_, Blake2_128Concat, PacketId, X3Packet>;
    
    #[pallet::call]
    impl<T: Config> Pallet<T> {
        pub fn submit_packet(packet: X3Packet) -> DispatchResult;
        pub fn refund_on_timeout(packet_id: PacketId) -> DispatchResult;
        pub fn prove_execution(packet_id: PacketId, proof: ProofEmission) -> DispatchResult;
    }
}
```

**Tests:**
- [ ] Submit packet
- [ ] Query packet by ID
- [ ] Refund on timeout
- [ ] Proof emission

---

### Phase 1.6: Comprehensive Fuzz Testing
**Time:** 3 days | **Owner:** QA team

**Create:** `crates/x3-packet-standard/fuzz/` and `tests/`

**Test Cases:**
- [ ] 1,000 random packets with varying timeout/expiry
- [ ] 500 replay attempts (all rejected)
- [ ] 500 concurrent submissions
- [ ] Edge case: packet expires mid-flight
- [ ] Edge case: relayer disappears (quorum still validates)

**Deliverable:** Fuzz report in `docs/PACKET_STANDARD_FUZZ_REPORT.md`

---

## 📋 SPRINT 2: X3-IXL (Cross-VM Instruction Language) (Weeks 4–6)

**Goal:** Unified cross-VM execution with rollback + proof  
**Team Size:** 3 engineers  
**Deliverable:** `crates/x3-ixl/` ready for all complex cross-VM flows

### Phase 2.1: Instruction Set Definition
**Time:** 3 days | **Owner:** Protocol team

**Create:** `crates/x3-ixl/src/instruction.rs`

**Code Target:**
```rust
pub enum X3Instruction {
    LockAsset { asset_id: AssetId, amount: Balance },
    MintCanonical { asset_id: AssetId, amount: Balance },
    BurnCanonical { asset_id: AssetId, amount: Balance },
    UnlockAsset { asset_id: AssetId, recipient: AccountId },
    RouteSwap { pool_id: PoolId, asset_in: AssetId, amount_in: Balance },
    CallEvm { contract: H160, data: Vec<u8> },
    CallSvm { program: [u8; 32], data: Vec<u8> },
    CallX3Vm { target: AccountId, data: Vec<u8> },
    SettleBalance { asset_id: AssetId, final_amount: Balance },
    EmitProof { proof_hash: H256 },
    Refund { reason: String },
    Abort { reason: String },
}

pub struct X3InstructionSequence {
    version: u16,
    instructions: Vec<X3Instruction>,
    domain: DomainId,
    sender: AccountId,
    nonce: u128,
}
```

**Tests:**
- [ ] All 12 instruction types serialize/deserialize
- [ ] Version check works
- [ ] Invalid sequences rejected

---

### Phase 2.2: Interpreter Implementation
**Time:** 5 days | **Owner:** VM team

**Create:** `crates/x3-ixl/src/interpreter.rs`

**Code Target:**
```rust
pub struct X3Interpreter;

impl X3Interpreter {
    pub fn execute(
        sequence: X3InstructionSequence,
        context: ExecutionContext,
    ) -> Result<X3ExecutionReceipt, ExecutionError>;
}

pub struct X3ExecutionReceipt {
    status: ExecutionStatus,
    gas_used: u64,
    proof_hash: H256,
    results: Vec<InstructionResult>,
}

pub enum InstructionResult {
    Success { output: Vec<u8> },
    Failed { reason: String },
}
```

**Execution Logic:**
- [ ] Lock → Mint → Swap → Settle → Proof → Success
- [ ] If any step fails → Rollback all prior steps → Refund
- [ ] Gas tracking per instruction
- [ ] Proof hash accumulation

**Tests:**
- [ ] Simple lock+mint
- [ ] Lock+mint+swap+settle (full happy path)
- [ ] Failure mid-sequence → rollback
- [ ] Gas tracking correct

---

### Phase 2.3: Transaction Planner
**Time:** 4 days | **Owner:** Compiler team

**Create:** `crates/x3-ixl/src/planner.rs`

**Code Target:**
```rust
pub struct X3Planner;

impl X3Planner {
    pub fn plan_swap(
        source_domain: DomainId,
        asset_in: AssetId,
        amount_in: Balance,
        dest_domain: DomainId,
        asset_out: AssetId,
    ) -> Result<X3InstructionSequence, PlanError>;
    
    pub fn plan_bridge_and_swap(
        source_chain: ChainId,
        asset: AssetId,
        amount: Balance,
        route: SwapRoute,
    ) -> Result<X3InstructionSequence, PlanError>;
}
```

**Planning Rules:**
- [ ] Lock first (establish source)
- [ ] Mint on destination
- [ ] Route through chosen pools
- [ ] Settle final balance
- [ ] Emit proof
- [ ] Always end with Refund or Success

**Tests:**
- [ ] Simple swap plan
- [ ] Cross-chain bridge+swap plan
- [ ] Multi-hop plan
- [ ] Invalid plan rejected

---

### Phase 2.4: Verifier
**Time:** 4 days | **Owner:** Crypto team

**Create:** `crates/x3-ixl/src/verifier.rs`

**Code Target:**
```rust
pub struct X3Verifier;

impl X3Verifier {
    pub fn verify_execution(
        receipt: &X3ExecutionReceipt,
        expected_proof_hash: H256,
    ) -> Result<(), VerifyError>;
    
    pub fn verify_cross_vm_call(
        call: &CrossVmCall,
        receipt: &CrossVmReceipt,
    ) -> Result<(), VerifyError>;
}
```

**Verification:**
- [ ] Proof hash matches receipt
- [ ] All instructions executed in order
- [ ] State changes are consistent
- [ ] Cross-VM calls have valid receipts

**Tests:**
- [ ] Valid receipt accepted
- [ ] Tampered receipt rejected
- [ ] Missing instructions rejected

---

### Phase 2.5: Rollback Mechanism
**Time:** 4 days | **Owner:** VM team

**Create:** `crates/x3-ixl/src/rollback.rs`

**Code Target:**
```rust
pub struct X3Rollback;

impl X3Rollback {
    pub fn checkpoint(state: &ExecutionState) -> Checkpoint;
    pub fn rollback_to(state: &mut ExecutionState, cp: &Checkpoint) -> Result<(), RollbackError>;
}

pub struct Checkpoint {
    instruction_index: usize,
    balances_snapshot: BTreeMap<AssetId, Balance>,
    state_snapshot: Vec<u8>,
}
```

**Rollback Logic:**
- [ ] On failure, restore all prior state
- [ ] Release any locks
- [ ] Issue refund
- [ ] Log failure reason

**Tests:**
- [ ] Rollback after step 1
- [ ] Rollback after step 5 (mid-sequence)
- [ ] Rollback releases locks
- [ ] State identical to pre-execution

---

### Phase 2.6: Cross-VM Call Integration
**Time:** 4 days | **Owner:** VM team

**Create:** `crates/x3-ixl/src/cross_vm_calls.rs`

**Code Target:**
```rust
pub fn execute_cross_vm_call(
    instruction: &X3Instruction,
    context: &ExecutionContext,
) -> Result<CrossVmReceipt, ExecutionError>;

// Dispatch based on target VM
impl X3Interpreter {
    fn call_evm(&self, contract: H160, data: &[u8]) -> Result<Vec<u8>, Error>;
    fn call_svm(&self, program: &[u8; 32], data: &[u8]) -> Result<Vec<u8>, Error>;
    fn call_x3vm(&self, target: &AccountId, data: &[u8]) -> Result<Vec<u8>, Error>;
}
```

**Tests:**
- [ ] EVM → SVM call succeeds
- [ ] SVM → X3VM call succeeds
- [ ] X3VM → EVM call succeeds
- [ ] Failed call triggers rollback
- [ ] Receipt contains proof of execution

---

### Phase 2.7: Comprehensive Integration Tests
**Time:** 4 days | **Owner:** QA team

**Create:** `tests/x3_ixl_integration_tests.rs`

**Test Scenarios:**
- [ ] Lock on EVM → Mint on X3 → Swap on X3 → Success
- [ ] Deposit on SVM → Route through X3 DEX → Withdraw on EVM
- [ ] Failed swap → Automatic refund to sender
- [ ] Partial execution → Rollback all
- [ ] Timeout during execution → Refund triggered

**Deliverable:** 50+ test cases passing

---

## 📋 SPRINT 3: LIQUIDITY CORE REFACTOR (Weeks 7–8)

**Goal:** Consolidate trading + add launchpad + anti-rug  
**Team Size:** 2 engineers  
**Deliverable:** `crates/x3-liquidity-core/` with launchpad + anti-rug

### Phase 3.1: Rename x3-dex → x3-liquidity-core
**Time:** 1 day | **Owner:** Build team

**Actions:**
- [ ] Rename crate directory
- [ ] Update all imports in runtime and pallets
- [ ] Update Cargo.toml references
- [ ] Verify builds

---

### Phase 3.2: Launchpad Module
**Time:** 4 days | **Owner:** Product team

**Create:** `crates/x3-liquidity-core/src/launchpad.rs` (800 LOC)

**Code Target:**
```rust
pub struct LaunchpadPool {
    token_id: AssetId,
    launch_stage: LaunchStage,
    total_raise_target: Balance,
    raised_amount: Balance,
    graduation_threshold: Balance,
    liquidity_lock_period: BlockNumber,
}

pub enum LaunchStage {
    Configuration,
    SeedRound { cap: Balance },
    PublicRound { cap: Balance },
    GraduationPool { locked_until: BlockNumber },
    TradingPool,
}

pub mod launchpad {
    pub fn create_launch(token: AssetId, target: Balance) -> Result<PoolId, Error>;
    pub fn add_liquidity_to_launch(pool_id: PoolId, amount: Balance) -> Result<(), Error>;
    pub fn graduate_pool(pool_id: PoolId) -> Result<(), Error>;
    pub fn unlock_liquidity(pool_id: PoolId) -> Result<(), Error>;
}
```

**Features:**
- [ ] Seed round with cap
- [ ] Public round transition
- [ ] Graduation when threshold met
- [ ] Locked liquidity period
- [ ] Anti-withdrawal during lock

**Tests:**
- [ ] Create launch pool
- [ ] Submit to seed round
- [ ] Transition to public round
- [ ] Graduate to trading pool
- [ ] Unlock after expiry
- [ ] Reject early unlock

---

### Phase 3.3: Anti-Rug Module
**Time:** 4 days | **Owner:** Risk team

**Create:** `crates/x3-liquidity-core/src/anti_rug.rs` (700 LOC)

**Code Target:**
```rust
pub struct AntiRugScore {
    liquidity_score: u8,    // 0-100
    holder_concentration: u8,
    volume_stability: u8,
    lock_duration_score: u8,
    final_score: u8,        // composite
}

pub mod anti_rug {
    pub fn compute_score(
        pool_id: PoolId,
        lock_duration: BlockNumber,
        top_holder_pct: u8,
    ) -> AntiRugScore;
    
    pub fn should_allow_launch(score: &AntiRugScore) -> bool;
}
```

**Scoring Factors:**
- [ ] Liquidity ratio (higher = safer)
- [ ] Holder concentration (lower = safer)
- [ ] Volume stability (consistent = safer)
- [ ] Lock duration (longer = safer)

**Rules:**
- [ ] Score < 40 → Block launch
- [ ] Score 40-70 → Require governance approval
- [ ] Score > 70 → Auto-approve

**Tests:**
- [ ] Good token (high score) auto-approves
- [ ] Rug signal (low score) blocks
- [ ] Mid-range requires governance

---

### Phase 3.4: Settlement Consolidation
**Time:** 2 days | **Owner:** Core team

**Create:** `crates/x3-liquidity-core/src/settlement.rs` (300 LOC)

**Actions:**
- [ ] Consolidate settlement logic from route_finder + swap logic
- [ ] Ensure every swap updates canonical supply
- [ ] Add settlement invariant checks

---

### Phase 3.5: Perpetuals Feature Gate
**Time:** 1 day | **Owner:** Build team

**Actions:**
- [ ] Gate perpetuals behind `#[cfg(feature = "perpetuals-beta")]`
- [ ] Add to Cargo.toml as optional feature
- [ ] Document in README

---

## 📋 SPRINT 4: UNIVERSAL CONTRACTS (Weeks 9–10)

**Goal:** Developer-facing intent + action SDK  
**Team Size:** 2 engineers  
**Deliverable:** `crates/x3-universal-contracts/` with full SDK

### Phase 4.1: Create Crate & Consolidate Intent Types
**Time:** 2 days | **Owner:** SDK team

**Create:** `crates/x3-universal-contracts/src/lib.rs`

**Actions:**
- [ ] Move intent types from `x3-intent` crate
- [ ] Consolidate action types
- [ ] Define unified interface

---

### Phase 4.2: Action Types & SDK Format
**Time:** 3 days | **Owner:** SDK team

**Create:** `crates/x3-universal-contracts/src/actions.rs`

**Code Target:**
```rust
pub enum X3Action {
    Swap {
        asset_in: AssetId,
        amount_in: Balance,
        asset_out: AssetId,
        min_out: Balance,
    },
    SwapAndStake {
        asset_in: AssetId,
        amount_in: Balance,
        pool_id: PoolId,
        min_tokens: Balance,
    },
    BridgeAndSwap {
        source_chain: ChainId,
        asset: AssetId,
        amount: Balance,
        route: SwapRoute,
    },
    LaunchToken {
        token_id: AssetId,
        target_raise: Balance,
    },
    LockLiquidity {
        pool_id: PoolId,
        amount: Balance,
        lock_period: BlockNumber,
    },
    CallEvm { contract: H160, data: Vec<u8> },
    CallSvm { program: [u8; 32], data: Vec<u8> },
    CallX3Vm { target: AccountId, data: Vec<u8> },
    Refund { reason: String },
}

pub struct X3Intent {
    from_chain: ChainId,
    from_domain: DomainId,
    action: X3Action,
}

pub struct X3SendRequest {
    from_chain: String,
    to_domain: String,
    asset: String,
    amount: String,
    action: ActionSpec,
}

// SDK usage target:
// await x3.send({
//   fromChain: "base",
//   toDomain: "x3-evm",
//   asset: "USDC",
//   amount: "1000",
//   action: {
//     type: "swapAndStake",
//     pool: "X3/USDC",
//     minOut: "990"
//   }
// });
```

**Tests:**
- [ ] All action types serialize/deserialize
- [ ] Intent validation
- [ ] Action → Instruction compilation

---

### Phase 4.3: SDK Integration with IXL
**Time:** 3 days | **Owner:** SDK team

**Create:** `crates/x3-universal-contracts/src/sdk.rs`

**Code:**
```rust
impl X3Intent {
    pub fn compile_to_instructions(&self) -> Result<X3InstructionSequence, CompileError>;
}

pub async fn x3_send(request: X3SendRequest) -> Result<TransactionHash, SdkError> {
    let intent = X3Intent::from_request(request)?;
    let instructions = intent.compile_to_instructions()?;
    submit_to_x3_network(instructions).await
}
```

**Tests:**
- [ ] Intent → Instructions compilation
- [ ] All action types compile
- [ ] Compilation preserves semantics

---

### Phase 4.4: Integration Tests
**Time:** 2 days | **Owner:** QA team

**Create:** `tests/universal_contracts_tests.rs`

**Test Scenarios:**
- [ ] Base USDC → X3 swap
- [ ] Ethereum lock → X3 mint
- [ ] Solana asset → X3 SVM call
- [ ] Failed action refunds user
- [ ] Invalid action rejected
- [ ] Bad min_out reverts

---

## 📋 SPRINT 5: EXTERNAL LIQUIDITY GATEWAY (Weeks 11–14)

**Goal:** Multi-chain liquidity with 6+ chain adapters  
**Team Size:** 3 engineers  
**Deliverable:** All 6 chains wired with witness quorum + emergency pause

**Timeline:**
- Weeks 11-12: Consolidate, Base adapter, Ethereum adapter
- Week 13: Arbitrum, BSC adapters + Solana
- Week 14: Bitcoin + Testing + Emergency pause

### Phase 5.1: Consolidate into x3-external-liquidity-gateway
**Time:** 3 days | **Owner:** Build team

**Create:** `crates/x3-external-liquidity-gateway/`

**Actions:**
- [ ] Extract from x3-bridge, x3-crosschain-gateway, x3-bridge-adapters
- [ ] Create modular structure:
  ```
  x3-external-liquidity-gateway/
  ├── src/
  │   ├── lib.rs
  │   ├── ethereum.rs
  │   ├── base.rs
  │   ├── arbitrum.rs
  │   ├── bsc.rs
  │   ├── solana.rs
  │   ├── bitcoin.rs
  │   ├── watcher.rs
  │   ├── relayer.rs
  │   ├── attestation.rs
  │   └── refund.rs
  ```

---

### Phase 5.2: Base Adapter (Hardened)
**Time:** 5 days | **Owner:** Chain team

**Create:** `src/base.rs` (600 LOC)

**Features:**
- [ ] Lock USDC on Base
- [ ] Verify Lock Packet
- [ ] Mint xUSDC on X3
- [ ] Witness validation (4-of-7 quorum)
- [ ] Emergency pause integration

**Tests:**
- [ ] Lock → Mint happy path
- [ ] Replay attack fails
- [ ] Stale packet fails
- [ ] Wrong chain ID fails
- [ ] Emergency pause blocks mint

---

### Phase 5.3: Ethereum Adapter (Enhanced)
**Time:** 5 days | **Owner:** Chain team

**Create:** `src/ethereum.rs` (600 LOC)

**Actions:**
- [ ] Enhance from existing x3-bridge/ethereum_bridge.rs
- [ ] Add witness validation
- [ ] Add emergency pause

**Features:**
- [ ] Lock ETH/USDC/USDT on Ethereum
- [ ] Verify with Ethereum validators
- [ ] Mint on X3
- [ ] Refund on timeout

**Tests:**
- Similar to Base

---

### Phase 5.4: Arbitrum Adapter (New)
**Time:** 4 days | **Owner:** Chain team

**Create:** `src/arbitrum.rs` (500 LOC)

**Similar pattern to Base/Ethereum**

---

### Phase 5.5: BSC Adapter (New)
**Time:** 4 days | **Owner:** Chain team

**Create:** `src/bsc.rs` (500 LOC)

**Similar pattern to Base/Ethereum**

---

### Phase 5.6: Solana Adapter (New)
**Time:** 5 days | **Owner:** SVM team

**Create:** `src/solana.rs` (600 LOC)

**Features:**
- [ ] Lock SPL tokens on Solana
- [ ] Verify via Solana validators (via IBC light client)
- [ ] Mint on X3
- [ ] Refund on timeout

**Tests:**
- [ ] SPL token lock → X3 mint
- [ ] Witness validation

---

### Phase 5.7: Bitcoin Adapter (HTLC)
**Time:** 5 days | **Owner:** Bitcoin team

**Create:** `src/bitcoin.rs` (700 LOC)

**Features:**
- [ ] HTLC setup (hash-time-locked contract)
- [ ] Bitcoin SPV proof verification
- [ ] Lock BTC on Bitcoin
- [ ] Mint xBTC on X3
- [ ] Refund after timeout

**Tests:**
- [ ] HTLC setup and fund
- [ ] SPV proof verification
- [ ] BTC refund after timeout

---

### Phase 5.8: Watcher & Attestation
**Time:** 4 days | **Owner:** Protocol team

**Create:** `src/watcher.rs` + `src/attestation.rs` (800 LOC)

**Features:**
- [ ] Watch source chains for lock events
- [ ] Collect validator signatures
- [ ] Require 4-of-7 quorum for attestation
- [ ] Emit Attestation packet
- [ ] Trigger mint on X3

**Tests:**
- [ ] Lock detected on Base
- [ ] Signatures collected
- [ ] Quorum validated
- [ ] Mint triggered

---

### Phase 5.9: Emergency Pause
**Time:** 2 days | **Owner:** Core team

**Wire:** `src/refund.rs` (200 LOC)

**Actions:**
- [ ] Integrate with x3-circuit-breaker
- [ ] Governance can pause all gateway operations
- [ ] Triggers auto-refund of pending transfers
- [ ] Only governance can resume

---

### Phase 5.10: Comprehensive Testing
**Time:** 4 days | **Owner:** QA team

**Create:** `tests/gateway_integration_tests.rs` (1000+ LOC)

**Test Scenarios:**
- [ ] Lock on Base → Mint on X3 → Swap → Success
- [ ] Replay attack fails
- [ ] Stale packet (old lock) fails
- [ ] Wrong chain ID fails
- [ ] Relayer double-submit fails
- [ ] Watcher quorum required
- [ ] Emergency pause blocks all ops
- [ ] Refund on timeout
- [ ] Cross-chain atomic failure → refund

---

## 📋 SPRINT 6: INTEGRATED SERVICES (Weeks 15–16)

**Goal:** Oracle, VRF, automation, keepers, risk, routing  
**Team Size:** 2 engineers

### Phase 6.1-6.7: Create `crates/x3-integrated-services/`

**Effort:** Similar to Phase 4, but simpler (wrapping existing services)

**Modules:**
- `oracle_net.rs` — native price feeds (expand from x3-oracle)
- `vrf.rs` — randomness
- `automation.rs` — task scheduler
- `keeper.rs` — job queue
- `bridge_watchers.rs` — watcher formalization
- `risk_classifier.rs` — wire from x3-gateway-risk-engine
- `route_optimizer.rs` — wire from x3-swap-router

**Label all AI-assisted modules clearly** (recommend, score, optimize — NOT consensus)

---

## 📋 SPRINT 7: PARALLEL EXECUTOR (Weeks 17–19)

**Goal:** Speed layer with deterministic correctness  
**Team Size:** 2 engineers

**Similar sprint structure; create `crates/x3-parallel-executor/`**

**Key guarantee:** Parallel execution produces same final state as serial execution (proven by tests)

---

## 📋 SPRINT 8: TESTNET MILESTONE (Week 20)

**Goal:** Package everything for public testnet launch  
**Team Size:** 2 engineers

**Deliverables:**
- [ ] Integration test suite (all modules together)
- [ ] One-command testnet launch script
- [ ] Public documentation
- [ ] Readiness dashboard
- [ ] Benchmark report
- [ ] Incident runbooks

---

## 🎯 CRITICAL DEPENDENCIES

```
Sprint 0: Foundation ✓
    ↓
Sprint 1: Packet Standard
    ↓
Sprint 2: X3-IXL ← (depends on Packets)
    ↓
Sprint 3: LiquidityCore (independent)
Sprint 4: Universal Contracts ← (depends on IXL)
Sprint 5: Gateway ← (depends on Packets + IXL)
    ↓
Sprint 6: Services (independent)
Sprint 7: Parallel Executor ← (depends on IXL)
    ↓
Sprint 8: Testnet Milestone ← (all complete)
```

**Parallelization:** Sprints 3, 4, 6 can run in parallel with Sprints 1–2.

---

## ✅ SUCCESS CRITERIA

### End of Sprint 2 (Week 6):
- [ ] Packets fuzz tested (1,000 random ops)
- [ ] IXL executes cross-VM swaps atomically
- [ ] Rollback tested and working
- [ ] 100% test coverage for both crates

### End of Sprint 5 (Week 14):
- [ ] 6 chains connected with witness quorum
- [ ] Emergency pause tested
- [ ] Timeout/refund flow end-to-end
- [ ] No replay attacks possible

### End of Sprint 8 (Week 20):
- [ ] All 8 modules integrated
- [ ] Public testnet deployed
- [ ] 100+ node network running
- [ ] 1,000 TPS demonstrated (parallel executor)
- [ ] Zero critical bugs in audit

