# Phase 4.5 Liquidity, Inventory, and Solvency — Implementation Plan

## Source

Breaks down [X3_LIQUIDITY_INVENTORY_SOLVENCY_SPEC.md](../specs/X3_LIQUIDITY_INVENTORY_SOLVENCY_SPEC.md) into concrete implementation work. For independently buildable slices, see [PHASE_4_5_EXECUTION_TICKETS.md](./PHASE_4_5_EXECUTION_TICKETS.md).

## Position in go-mode sequence

Depends on:
- Phase 0: Constitutional controls (`pallets/x3-constitution`)
- Phase 3: Accounting spine (`pallets/x3-accounting`)
- Phase 3.5: Signer and custody boundaries (`pallets/x3-custody`)

Produces the control layer consumed by:
- Phase 5: Router execution binding
- Phase 5.5: Operator cockpit and solvency dashboard

## Crate layout

```
pallets/
  x3-inventory/          # Vault model, lane model, inventory manager
  x3-reservation/        # Reservation engine, expiry tracking
  x3-solvency/           # Solvency engine, gate checks, snapshot registry
  x3-rebalance/          # Rebalance planner and executor
  x3-partner/            # Partner capacity tracking, health scoring
  x3-treasury-policy/    # Treasury allocation caps, vault funding controls

runtime/
  src/lib.rs             # Pallet wiring

services/
  x3-solvency-sidecar/   # Off-chain solvency telemetry and alerting
```

---

## Module 1 — Vault and Inventory Manager (`pallets/x3-inventory`)

### Responsibilities

- Maintain `VaultState` storage per `(chain_id, asset_id, vault_type)`
- Track `available_balance`, `reserved_balance`, `pending_out_balance`, `pending_in_balance`
- Enforce `critical_min`, `min`, `target`, `max` inventory bands
- Expose inventory reserve/release calls consumed by the reservation engine
- Activate lane freeze when a vault drops below `critical_min`
- Emit inventory events to the accounting spine

### Storage layout

```rust
Vaults: StorageMap<VaultId, VaultState>
Lanes: StorageMap<LaneId, LaneState>
GlobalUnsettledNotional: StorageValue<Balance>
LaneUnsettledNotional: StorageMap<LaneId, Balance>
```

### Core types

```rust
pub struct VaultState {
    pub vault_id: VaultId,
    pub vault_type: VaultType,        // Gas | SettlementFloat | TreasuryReserve | InsuranceLoss
    pub owner_type: OwnerType,        // Protocol | Treasury | Partner
    pub chain_id: ChainId,
    pub asset_id: AssetId,
    pub available_balance: Balance,
    pub reserved_balance: Balance,
    pub pending_out_balance: Balance,
    pub pending_in_balance: Balance,
    pub critical_min: Balance,
    pub min_band: Balance,
    pub target_band: Balance,
    pub max_band: Balance,
    pub status: VaultStatus,          // Active | Degraded | Frozen
}

pub struct LaneState {
    pub lane_id: LaneId,
    pub source_chain: ChainId,
    pub dest_chain: ChainId,
    pub source_asset: AssetId,
    pub dest_asset: AssetId,
    pub lane_class: LaneClass,        // A | B | C
    pub allowed_liquidity_sources: BoundedVec<LiquiditySourceType, MaxSources>,
    pub status: LaneStatus,           // Active | Degraded | Frozen
    pub exposure_cap: Balance,
    pub unsettled_cap: Balance,
}
```

### Interface

```rust
// Called by reservation engine
fn reserve_inventory(vault_id: VaultId, amount: Balance) -> Result<(), InventoryError>;
fn release_inventory(vault_id: VaultId, amount: Balance) -> Result<(), InventoryError>;
fn record_pending_out(vault_id: VaultId, amount: Balance) -> Result<(), InventoryError>;
fn confirm_settlement(vault_id: VaultId, amount: Balance) -> Result<(), InventoryError>;

// Called by rebalance engine
fn update_available_balance(vault_id: VaultId, new_balance: Balance) -> Result<(), InventoryError>;

// Called by solvency engine (read-only)
fn get_vault_state(vault_id: VaultId) -> Option<VaultState>;
fn get_lane_state(lane_id: LaneId) -> Option<LaneState>;

// Called by treasury policy
fn fund_vault(vault_id: VaultId, amount: Balance) -> Result<(), InventoryError>;

// Called by freeze automation
fn freeze_lane(lane_id: LaneId, reason: FreezeReason) -> Result<(), InventoryError>;
fn unfreeze_lane(lane_id: LaneId, evidence: OperatorEvidence) -> Result<(), InventoryError>;
```

### Balance invariant

At all times: `available_balance + reserved_balance + pending_out_balance <= total_deposited`

This invariant must be upheld atomically on every mutation. Tests must cover all combinations.

### Acceptance criteria

- `available_balance` decreases by reservation amount when `reserve_inventory` is called
- Vault auto-transitions to `Frozen` when `available_balance` falls below `critical_min`
- Lane freeze emits `LaneFrozen` event that feeds the accounting spine
- Frozen lanes reject all new inventory reservations with `LaneFrozenError`
- All balance mutations are atomic under Substrate storage transaction semantics

---

## Module 2 — Reservation Engine (`pallets/x3-reservation`)

### Responsibilities

- Accept reservation requests from the router layer
- Validate lane status, vault sufficiency, exposure caps, and solvency snapshot reference
- Record time-bounded reservations with expiry indexed by block number
- Release inventory on settlement, failure, or timeout
- Prevent execution on expired reservations
- Emit reservation events to the accounting spine

### Storage layout

```rust
Reservations: StorageMap<ReservationId, ReservationState>
ReservationsByRoute: StorageMap<RouteId, ReservationId>
ExpiryQueue: StorageDoubleMap<BlockNumber, ReservationId, ()>
```

### Core types

```rust
pub struct ReservationState {
    pub reservation_id: ReservationId,
    pub route_id: RouteId,
    pub lane_id: LaneId,
    pub liquidity_source_type: LiquiditySourceType,
    pub partner_id: Option<PartnerId>,
    pub source_amount: Balance,
    pub dest_amount: Balance,
    pub expiry_block: BlockNumber,
    pub status: ReservationStatus,    // Active | Expired | Released | Consumed
    pub solvency_snapshot_hash: H256,
    pub slippage_tolerance_bps: u32,
    pub max_fee_envelope: Balance,
}
```

### Expiry hook

`on_initialize` scans `ExpiryQueue` for the current block, releases expired reservations, restores vault balances, and emits `ReservationExpired` events. Processing must complete within the block it is due.

### Acceptance criteria

- `request_reservation` reduces vault `available_balance` and increases `reserved_balance` atomically
- Expired reservations return `ReservationExpiredError` on consume attempt
- Released/expired reservations restore `available_balance` from `reserved_balance`
- Every reservation emits `ReservationCreated` with a full solvency snapshot hash
- Reservation for a frozen lane returns `LaneFrozenError` immediately

---

## Module 3 — Solvency Engine (`pallets/x3-solvency`)

### Responsibilities

- Implement all four solvency gates: pre-quote, pre-reservation, pre-submission, post-submission
- Evaluate all dimensions from the spec per gate
- Block execution at any failing gate
- Record deterministic solvency snapshots referenced by reservations
- Emit solvency gate events for audit

### Gate interface

```rust
pub trait SolvencyGate {
    fn check_pre_quote(ctx: &QuoteContext) -> SolvencyResult;
    fn check_pre_reservation(ctx: &ReservationContext) -> SolvencyResult;
    fn check_pre_submission(ctx: &SubmissionContext) -> SolvencyResult;
    fn record_post_submission(ctx: &PostSubmissionContext) -> Result<(), SolvencyError>;
}

pub struct SolvencyResult {
    pub passed: bool,
    pub failed_checks: Vec<SolvencyCheck>,
    pub snapshot_hash: H256,
}
```

### Evaluated dimensions per gate

| Gate | Dimensions checked |
|---|---|
| Pre-quote | source chain health, dest chain health, route component health, asset activation, lane freeze, quote freshness, tentative capacity |
| Pre-reservation | source vault sufficiency, destination capacity, gas reserve, exposure caps, unsettled notional, partner health, route profitability, quarantine status |
| Pre-submission | reservation validity, quote freshness, slippage bounds, signer health, incident flags, reconciliation lag, partner reservation live, bridge path exists |
| Post-submission | debit record, pending obligation, timeout tracking, evidence seal, recovery path binding, exposure dashboard update |

### Snapshot registry storage

```rust
SolvencySnapshots: StorageMap<H256, SolvencySnapshotRecord>
```

Snapshots are pruned after a configurable retention window, unless referenced by an active reservation or pending obligation.

### Acceptance criteria

- Any gate failure blocks the route and emits `SolvencyGateFailed` with the specific failing check
- Snapshot hash is deterministic for identical inputs
- Pre-submission gate runs within the same extrinsic as route submission
- `record_post_submission` cannot be skipped after a successful pre-submission gate (enforced by coupling in the submission extrinsic)

---

## Module 4 — Rebalance Engine (`pallets/x3-rebalance`)

### Responsibilities

- Monitor vault bands and trigger rebalance when `available_balance < min_band`
- Support slow (scheduled) and fast (event-driven) rebalance modes
- Execute rebalance steps in priority order
- Enforce daily rebalance volume cap
- Emit rebalance events to the accounting spine

### Rebalance step priority order

1. Internal flow netting — scan for offsetting demand in active reservations
2. Cross-chain sweep — move balance from overfunded vault on another chain
3. Market rebalance — execute through approved venue (oracle or off-chain worker input)
4. Partner-assisted rebalance — request partner temporary depth
5. Treasury refill — Class C lanes only; calls `fund_settlement_vault` via treasury policy

### Trigger types

```rust
pub enum RebalanceTrigger {
    BelowMinBand { vault_id: VaultId },
    DemandSpike { lane_id: LaneId, projected_demand: Balance },
    ConcentrationBreach { chain_id: ChainId },
    PartnerCapacityLoss { partner_id: PartnerId },
    VenueLiquidityCollapse { venue_id: VenueId },
    PersistentOneWayFlow { lane_id: LaneId },
    ChainDegradation { chain_id: ChainId },
}
```

### Acceptance criteria

- Vault below `min_band` receives a rebalance trigger within one block finalization cycle
- Fast rebalance preempts the slow scheduled queue
- Daily cap breach emits `RebalanceCapExceeded` and halts further attempts for the day
- `RebalanceCompleted` includes source vault, destination vault, amount, and method used
- Treasury refill on a Class A or B lane returns `TreasuryRefillNotAllowedError`

---

## Module 5 — Partner Capacity Manager (`pallets/x3-partner`)

### Responsibilities

- Track partner status, supported lanes, health score, exposure limit, and current exposure
- Accept and validate partner quote responses
- Record partner reservation confirmations
- Update health scores after each settlement cycle from measured metrics
- Block partner routes when health falls below threshold

### Measured metrics

- Quote response time (p50, p95)
- Fill reliability rate
- Rejected reservation rate
- Stale quote rate
- Average spread by lane
- Dispute count (rolling 30d)
- Settlement delay rate

### Acceptance criteria

- Partner below `MIN_PARTNER_HEALTH` returns `PartnerUnhealthyError` on reservation attempt
- Exposure cap enforcement prevents routes that would breach `exposure_limit`
- Health scores update after each settlement cycle and emit `PartnerHealthUpdated`
- Metric inputs are stored for audit, not only the computed score

---

## Module 6 — Treasury Policy (`pallets/x3-treasury-policy`)

### Responsibilities

- Define and enforce allocation caps by chain, asset, and lane class
- Govern vault funding actions (treasury → settlement float vault)
- Keep insurance reserve separate from operational settlement float
- Require governance approval for allocations above operator threshold
- Emit treasury events to the accounting spine

### Storage layout

```rust
AllocationCaps: StorageMap<(ChainId, AssetId, LaneClass), Balance>
TreasuryDeployedByLaneClass: StorageMap<LaneClass, Balance>
InsuranceReserveBalance: StorageValue<Balance>
```

### Acceptance criteria

- Funding exceeding cap returns `AllocationCapExceeded`
- Insurance reserve balance is unreachable via `fund_settlement_vault`
- Cap expansions above operator threshold emit `GovernanceApprovalRequired` and pause pending governance vote
- Deployed tracker stays consistent across fund and withdraw operations

---

## Module 7 — Solvency Sidecar Service (`services/x3-solvency-sidecar`)

### Responsibilities

- Subscribe to all Phase 4.5 event families from node RPC
- Maintain live dashboard state: vault utilization, lane health, frozen lanes, unsettled notional, partner health, treasury at risk
- Publish Prometheus metrics covering all nine required metric families
- Emit alerting webhooks on threshold breaches
- Provide REST/gRPC query interface for the operator cockpit

### Required metric families

1. Vault utilization by chain and asset
2. Settlement float idle ratio
3. Gas reserve days of coverage
4. Route: firm quote conversion, reservation rejection, stale quote rejection, execution success, settlement time by lane
5. Risk: unsettled notional, under-threshold incidents, frozen lane count, partner concentration ratio, treasury capital at risk
6. Rebalance: frequency, average cost, internally netted flow percentage, rebalance latency

### Acceptance criteria

- All metric families present in Prometheus output at `/metrics`
- Frozen lane visible in REST response within one polling cycle of `LaneFrozen` event
- Alert fires within one polling cycle of a threshold breach
- Service reconnects to node and replays from last known block on disconnection

---

## Integration wiring checkpoints

All seven paths must pass end-to-end tests before Phase 4.5 is considered complete:

1. Router requests route → `check_pre_quote` → if passed, returns indicative quote
2. User confirms → `request_reservation` triggers `check_pre_reservation` internally
3. Router submits → `check_pre_submission` → if passed, `submit_route` called
4. `record_post_submission` called in same extrinsic as step 3
5. Settlement confirmed → `consume_reservation`, `confirm_settlement`, accounting spine updated
6. Vault drops below `min_band` → rebalance trigger fired → rebalance executed → vault restored
7. Lane frozen → all reservation requests for that lane return `LaneFrozenError`

---

## Testing requirements

Each module must have:
- Unit tests for all storage mutations
- Unit tests covering all gate check dimension combinations
- Integration test with a mock router exercising the full reservation → submission → settlement path
- A known-bad scenario test for each hard invariant in the spec

End-to-end tests must cover at minimum the twelve scenarios listed in TICKET-4.5-018.
