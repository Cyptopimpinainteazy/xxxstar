# DEPIN-GPU-001 — Native DePIN GPU Marketplace

| Field       | Value |
|-------------|-------|
| **ID**      | DEPIN-GPU-001 |
| **Status**  | DRAFT |
| **Authors** | X3 Core Team |
| **Created** | 2026-02-13 |
| **Priority** | P0 — Highest (economic engine) |

---

## Summary

Turn validator GPU idle capacity into a revenue-generating compute marketplace
embedded directly inside the X3 node. Validators expose sandboxed GPU workers
for paid off-chain workloads (AI inference, ZK proving, video transcoding, etc.)
while the base chain retains absolute priority over block-building work.
Revenue is split between validators, a burn mechanism, and staking rewards —
creating real yield and a self-reinforcing flywheel.

---

## Motivation

### Problem

- Validators operate high-end GPUs (A100/H100) that are **cost centers** today.
  Between blocks, most GPU compute sits idle — 2.7M+ TPS still leaves
  sub-millisecond gaps that aggregate to >80% idle time.
- No on-chain mechanism exists to monetize spare cycles; the only return is
  block rewards and MEV.
- The existing `GPUMarketplace.sol` contract and `gpu-swarm` coordinator
  operate independently with no on-chain Substrate bridge between them.

### Opportunity

- Every validator already runs GPU hardware and the `gpu-swarm` crate.
- External demand for GPU compute (AI inference, ZK proofs) is growing
  exponentially and currently served by centralized providers (AWS, Lambda)
  or nascent DePIN networks (Akash, Render) with separate hardware.
- X3 validators can offer **zero-marginal-cost compute** since the
  hardware is already paid for and running.

### Economic Flywheel

```
More TPS → More validators → More GPUs → Cheaper compute
→ More customers → Higher token demand → Higher staking rewards
→ More validators → ...
```

---

## Design

### Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                        X3 Node                               │
│  ┌──────────┐  ┌────────────┐  ┌──────────────────────────┐    │
│  │ Block    │  │ GPU Warden │  │  DePIN Marketplace       │    │
│  │ Builder  │──│ (priority  │──│  Service                 │    │
│  │          │  │  preempt)  │  │  ┌────────────────────┐  │    │
│  └──────────┘  └────────────┘  │  │ Job Queue          │  │    │
│                                │  │ Sandbox Manager    │  │    │
│                                │  │ Billing Engine     │  │    │
│                                │  │ Attestation Prover │  │    │
│                                │  └────────────────────┘  │    │
│                                └──────────────────────────┘    │
│                                                                 │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │  pallet-depin-marketplace (Substrate)                     │  │
│  │  - Provider registry & staking                            │  │
│  │  - Job order book (on-chain or off-chain relay)           │  │
│  │  - Revenue accounting: 55% validator / 25% burn / 20% stk│  │
│  │  - SLA enforcement & slashing                             │  │
│  └──────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
         │                              │
         │  gRPC/WebSocket              │  Extrinsics
         ▼                              ▼
┌─────────────────┐          ┌─────────────────────┐
│  External Job   │          │  On-chain Settlement │
│  Relay / API    │          │  & Token Economics    │
└─────────────────┘          └─────────────────────┘
```

### Component Breakdown

#### 1. `pallet-depin-marketplace` (New Substrate Pallet)

Bridges the existing `GPUMarketplace.sol` logic to the native runtime:

```rust
// pallets/depin-marketplace/src/lib.rs (sketch)

#[pallet::config]
pub trait Config: frame_system::Config {
    type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
    type Currency: ReservableCurrency<Self::AccountId>;
    type GpuSwarmCoordinator: GpuJobDispatch;
    type BurnDestination: OnUnbalanced<NegativeImbalanceOf<Self>>;
    /// Revenue split basis points (out of 10_000)
    type ValidatorShareBps: Get<u16>;  // 5_500
    type BurnShareBps: Get<u16>;       // 2_500
    type StakerShareBps: Get<u16>;     // 2_000
    type MinStake: Get<BalanceOf<Self>>;
    type MaxJobDuration: Get<BlockNumberFor<Self>>;
    type SlashFraction: Get<Perbill>;
}

#[pallet::storage]
pub type Providers<T> = StorageMap<_, Blake2_128Concat, T::AccountId, ProviderInfo<T>>;

#[pallet::storage]
pub type ActiveJobs<T> = StorageMap<_, Blake2_128Concat, JobId, Job<T>>;

#[pallet::storage]
pub type OrderBook<T> = StorageValue<_, BoundedVec<Order<T>, MaxOrders>>;
```

**Key types:**

```rust
pub struct ProviderInfo<T: Config> {
    pub account: T::AccountId,
    pub stake: BalanceOf<T>,
    pub gpu_capabilities: GpuCapabilities,  // from gpu-swarm
    pub reputation: u32,
    pub total_jobs_completed: u64,
    pub total_revenue: BalanceOf<T>,
    pub status: ProviderStatus,
    pub registered_at: BlockNumberFor<T>,
}

pub enum ProviderStatus {
    Active,
    Paused,         // voluntarily paused
    BlockBuilding,  // preempted for chain work
    Slashed,
}

pub struct Job<T: Config> {
    pub id: JobId,
    pub customer: T::AccountId,
    pub job_type: DePinJobType,
    pub gpu_requirements: GpuRequirements,
    pub max_price_per_unit: BalanceOf<T>,
    pub escrow: BalanceOf<T>,
    pub assigned_provider: Option<T::AccountId>,
    pub status: JobStatus,
    pub submitted_at: BlockNumberFor<T>,
    pub deadline: BlockNumberFor<T>,
    pub result_hash: Option<H256>,
}

pub enum DePinJobType {
    AiInference { model_hash: H256, input_size: u64 },
    ZkProving { circuit_hash: H256, witness_size: u64 },
    VideoTranscode { codec: Codec, resolution: Resolution },
    ProteinFolding { sequence_hash: H256 },
    Custom { workload_hash: H256, compute_units: u64 },
}
```

#### 2. DePIN Marketplace Service (Rust — in `crates/gpu-swarm`)

Extends the existing `SwarmCoordinator` and `Warden`:

- **New `ComputeLane::Marketplace`** in Warden policy — lowest priority,
  instantly preemptable by `ChainOps`.
- **`SandboxManager`** — runs customer workloads in isolated CUDA MPS
  contexts with VRAM limits, timeout enforcement, and syscall filtering.
- **`BillingEngine`** — tracks compute-seconds consumed; emits
  `JobCompleted { job_id, compute_units, cost }` events to the pallet.
- **`AttestationProver`** — generates a lightweight proof-of-computation
  (re-execution by a second validator or hash-based spot checks).

#### 3. External Job Relay API

- gRPC + REST + WebSocket endpoints on a configurable port (default 9955).
- Authentication via API keys tied to on-chain customer accounts.
- Endpoints: `SubmitJob`, `GetJobStatus`, `CancelJob`, `ListProviders`,
  `GetPricing`.
- Off-chain relay aggregates orders and settles on-chain in batches
  (every N blocks or when escrow threshold reached).

#### 4. Preemption Protocol

**Critical invariant**: chain work always takes priority.

```
1. Block interval timer fires
2. Warden sends PREEMPT signal to Marketplace lane
3. SandboxManager checkpoints all running rental jobs (CUDA stream sync)
4. GPU fully available for block building within 2ms
5. After block finalized, rental jobs resume from checkpoint
```

Checkpointing uses CUDA Unified Memory snapshots — proven mechanism from
the existing `stream_pipeline.cu` infrastructure.

---

## Integration Points

| Existing Component | Change Required |
|---|---|
| `crates/gpu-swarm/src/warden/` | Add `ComputeLane::Marketplace`, preemption logic |
| `crates/gpu-swarm/src/coordinator.rs` | Bridge to pallet via off-chain worker |
| `crates/gpu-swarm/src/task.rs` | Add `TaskType::MarketplaceRental` variant |
| `crates/gpu-swarm/src/jobs/mod.rs` | Add `RentalJob` implementing `SwarmJob` |
| `crates/gpu-swarm/src/node.rs` | Extend `GpuCapabilities` with pricing, attestation |
| `contracts/ai-swarm/src/GPUMarketplace.sol` | Bridge to pallet via cross-VM comit |
| `pallets/x3-kernel/` | Couple with `pallet-depin-marketplace` for fee accounting |
| `pallets/treasury/` | Route burn + staker share through treasury |
| `node/src/rpc.rs` | Expose DePIN RPC methods |
| `runtime/` | Add `pallet-depin-marketplace` to runtime config |

---

## Invariants

New entries for `tests/invariants/registry.toml`:

```toml
[[invariant]]
id = "DEPIN-MARKET-001"
description = "Block-building work always preempts marketplace jobs within 2ms"
severity = "CRITICAL"
layer = "CONSENSUS"
tested_by = ["tests/depin/test_preemption.rs::preemption_within_deadline"]
property = "preempt_latency <= 2ms"

[[invariant]]
id = "DEPIN-MARKET-002"
description = "Revenue split exactly matches configured BPS (validator + burn + staker = 10000)"
severity = "CRITICAL"
layer = "PALLET"
tested_by = ["pallets/depin-marketplace/src/tests.rs::revenue_split_conservation"]
property = "validator_share + burn_share + staker_share == total_revenue"

[[invariant]]
id = "DEPIN-MARKET-003"
description = "Customer escrow is locked before job assignment and released only on completion or timeout"
severity = "CRITICAL"
layer = "PALLET"
tested_by = ["pallets/depin-marketplace/src/tests.rs::escrow_lifecycle"]
property = "lock(escrow) -> assigned -> (complete|timeout) -> release(escrow)"

[[invariant]]
id = "DEPIN-MARKET-004"
description = "Sandboxed GPU workloads cannot access host memory outside allocated VRAM region"
severity = "CRITICAL"
layer = "INFRA"
tested_by = ["tests/depin/test_sandbox.rs::memory_isolation"]
property = "forall rental_job: accessible_memory ⊆ allocated_vram_region"

[[invariant]]
id = "DEPIN-MARKET-005"
description = "Provider stake is slashed on verified job failure"
severity = "HIGH"
layer = "PALLET"
tested_by = ["pallets/depin-marketplace/src/tests.rs::slash_on_failure"]
property = "job_failed && verified => provider.stake -= slash_amount"
```

---

## Testing Strategy

| Phase | Scope | Method |
|-------|-------|--------|
| Unit | Pallet logic (escrow, splits, slashing) | `cargo test -p pallet-depin-marketplace` |
| Unit | Sandbox memory isolation | CUDA test harness with fault injection |
| Integration | Warden preemption under load | `cargo test -p gpu-swarm --test preemption` |
| Integration | Pallet ↔ SwarmCoordinator bridge | Off-chain worker mock tests |
| E2E | Full job lifecycle (submit → assign → complete → settle) | Docker Compose with test node + mock customer |
| Benchmark | Preemption latency p99 < 2ms | `frame-benchmarking` + CUDA event timers |
| Adversarial | Malicious workload escape, VRAM overallocation | Fuzzing sandbox boundaries |

---

## Rollout Plan

| Phase | Duration | Deliverable |
|-------|----------|-------------|
| **Phase 1** | 2 weeks | `pallet-depin-marketplace` with unit tests; provider registration + staking |
| **Phase 2** | 2 weeks | `ComputeLane::Marketplace` in Warden; preemption protocol; sandbox manager |
| **Phase 3** | 2 weeks | Job lifecycle end-to-end; billing engine; off-chain relay API |
| **Phase 4** | 1 week | Revenue split integration with treasury; burn mechanism |
| **Phase 5** | 1 week | E2E tests, benchmarks, documentation |
| **Phase 6** | 1 week | Testnet deployment; invite early marketplace customers |

Total: **~9 weeks** to testnet-ready MVP.

---

## Risks & Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Rental workload escapes sandbox | Low | Critical | CUDA MPS isolation + VRAM limits + syscall filter; security audit |
| Preemption latency exceeds 2ms | Medium | High | CUDA stream sync is O(μs); benchmark on target hardware; fallback to killing rental jobs |
| Low external demand at launch | Medium | Medium | Seed with internal AI inference workloads; subsidize early customers |
| Regulatory classification of compute rental | Low | Medium | Legal review; jurisdiction-aware provider registration |
| Token price volatility affects pricing | Medium | Medium | USD-denominated pricing with on-chain oracle conversion |

---

## Open Questions

- [ ] Should the order book be fully on-chain or use an off-chain relay with on-chain settlement?
      (Recommend: off-chain relay for latency, on-chain settlement for trust.)
- [ ] What's the minimum stake for providers? (Propose: tiered, matching `GPUMarketplace.sol` tiers.)
- [ ] Should the burn percentage be governance-adjustable? (Propose: yes, via `pallet-governance`.)
- [ ] How does this interact with the existing `GPUMarketplace.sol` contract?
      (Propose: bridge via cross-VM comit in x3-kernel; eventually migrate to native pallet.)
- [ ] What attestation scheme for proof-of-computation? (Propose: start with re-execution spot checks,
      move to optimistic attestation with fraud proofs.)
