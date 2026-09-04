# Phase 4.5 Execution Tickets

## Purpose

Breaks Phase 4.5 into independently buildable slices. Each ticket is self-contained enough to be picked up, implemented, tested, and merged without depending on incomplete work in the same wave. Cross-wave dependencies are explicit.

Source spec: [X3_LIQUIDITY_INVENTORY_SOLVENCY_SPEC.md](../specs/X3_LIQUIDITY_INVENTORY_SOLVENCY_SPEC.md)  
Full plan: [PHASE_4_5_LIQUIDITY_IMPLEMENTATION_PLAN.md](./PHASE_4_5_LIQUIDITY_IMPLEMENTATION_PLAN.md)

---

## Wave 1 — Foundation types and vault storage

No internal cross-ticket dependencies. All three tickets are parallelizable.

---

### TICKET-4.5-001: Core type definitions

**Crate:** `pallets/x3-inventory`  
**Effort:** S  
**Depends on:** Phase 0 constitutional types, Phase 3 accounting types

**Scope:** Types only — no storage, no logic, no extrinsics.

- `VaultId`, `VaultType`, `VaultStatus`, `OwnerType`
- `LaneId`, `LaneClass`, `LaneStatus`
- `ReservationId`, `ReservationStatus`
- `PartnerId`, `PartnerStatus`
- `LiquiditySourceType`
- `FreezeReason`, `OperatorEvidence`
- `SolvencyCheck`, `SolvencyResult`
- `RebalanceTrigger`, `RebalanceMethod`
- `ChainHealthStatus`, `VenueId`, `RouteId`

**Acceptance criteria:**
- All types compile with `cargo check`
- All types implement `Encode`, `Decode`, `MaxEncodedLen`, `TypeInfo`
- All types implement `Debug`, `Clone`, `PartialEq`
- Unit tests confirm round-trip codec for each type

---

### TICKET-4.5-002: Vault storage and band invariants

**Crate:** `pallets/x3-inventory`  
**Effort:** M  
**Depends on:** TICKET-4.5-001

**Scope:**
- `Vaults: StorageMap<VaultId, VaultState>`
- Extrinsics: `create_vault`, `update_vault_bands`
- Internal helper: `check_band_status(vault_id) -> VaultStatus`
- Auto-transition to `Frozen` when `available_balance < critical_min`
- Events: `VaultCreated`, `VaultStatusChanged`

**Acceptance criteria:**
- `create_vault` stores vault with correct initial status
- Balance below `critical_min` transitions status to `Frozen` and emits event
- `check_band_status` returns correct tier for all four band boundaries
- Weight benchmarks written (placeholder values acceptable)

---

### TICKET-4.5-003: Lane storage and freeze mechanics

**Crate:** `pallets/x3-inventory`  
**Effort:** M  
**Depends on:** TICKET-4.5-001

**Scope:**
- `Lanes: StorageMap<LaneId, LaneState>`
- Extrinsics: `register_lane`, `freeze_lane`, `unfreeze_lane`
- `freeze_lane` requires `FreezeReason`; `unfreeze_lane` requires `OperatorEvidence` logged to accounting spine
- Events: `LaneRegistered`, `LaneFrozen`, `LaneUnfrozen`

**Acceptance criteria:**
- Registering a lane stores it with `Active` status
- `freeze_lane` transitions to `Frozen` and emits event with reason
- `unfreeze_lane` logs evidence to the accounting spine
- Calling `freeze_lane` on an already-frozen lane returns `AlreadyFrozenError`

---

## Wave 2 — Inventory management and reservation engine

TICKET-4.5-004 must complete before TICKET-4.5-005. TICKET-4.5-006 depends on TICKET-4.5-005.

---

### TICKET-4.5-004: Inventory reserve and release

**Crate:** `pallets/x3-inventory`  
**Effort:** M  
**Depends on:** TICKET-4.5-002, TICKET-4.5-003

**Scope:**
- `reserve_inventory(vault_id, amount) -> Result`
- `release_inventory(vault_id, amount) -> Result`
- `record_pending_out(vault_id, amount) -> Result`
- `confirm_settlement(vault_id, amount) -> Result`
- `fund_vault(vault_id, amount) -> Result`
- Balance invariant enforced atomically on every mutation
- Frozen vault rejects `reserve_inventory`
- Events: `InventoryReserved`, `InventoryReleased`, `SettlementConfirmed`

**Acceptance criteria:**
- Balance invariant holds across all mutation combinations (property test required)
- Frozen vault returns `VaultFrozenError` on reserve attempt
- `record_pending_out` moves balance from `available_balance` to `pending_out_balance`
- `confirm_settlement` reduces `pending_out_balance` and emits event

---

### TICKET-4.5-005: Reservation engine core

**Crate:** `pallets/x3-reservation`  
**Effort:** L  
**Depends on:** TICKET-4.5-004

**Scope:**
- `Reservations: StorageMap<ReservationId, ReservationState>`
- `ReservationsByRoute: StorageMap<RouteId, ReservationId>`
- `ExpiryQueue: StorageDoubleMap<BlockNumber, ReservationId, ()>`
- `request_reservation(params) -> Result<ReservationId, ReservationError>`
- `release_reservation(reservation_id) -> Result`
- `consume_reservation(reservation_id) -> Result`
- `is_reservation_valid(reservation_id) -> bool`
- `on_initialize` hook: scan `ExpiryQueue` for current block, release expired reservations
- Events: `ReservationCreated`, `ReservationExpired`, `ReservationConsumed`, `ReservationReleased`

**Acceptance criteria:**
- `request_reservation` calls `reserve_inventory` and stores reservation atomically
- Consuming an expired reservation returns `ReservationExpiredError`
- `on_initialize` releases all expired reservations in current block and restores vault balance
- Every reservation event includes `solvency_snapshot_hash`
- Reservation for a frozen lane returns `LaneFrozenError`

---

### TICKET-4.5-006: Global and lane unsettled notional tracking

**Crate:** `pallets/x3-inventory`  
**Effort:** S  
**Depends on:** TICKET-4.5-005

**Scope:**
- `GlobalUnsettledNotional: StorageValue<Balance>`
- `LaneUnsettledNotional: StorageMap<LaneId, Balance>`
- Increment both on `ReservationCreated`
- Decrement both on `ReservationReleased`, `ReservationExpired`, `ReservationConsumed`
- Enforce `Lane.unsettled_cap` at reservation time
- Event: `ExposureCapBreached` (warning-level; hard rejection at cap)

**Acceptance criteria:**
- Both counters increment on reservation creation
- Both counters decrement on all three release paths
- Reservation breaching lane unsettled cap returns `UnsettledCapExceededError`

---

## Wave 3 — Solvency engine gates

Parallelizable after TICKET-4.5-005 completes. TICKET-4.5-008 depends on TICKET-4.5-007. TICKET-4.5-010 is parallelizable with TICKET-4.5-008.

---

### TICKET-4.5-007: Pre-quote and pre-reservation gates

**Crate:** `pallets/x3-solvency`  
**Effort:** M  
**Depends on:** TICKET-4.5-005, TICKET-4.5-006

**Scope:**
- `check_pre_quote(ctx: &QuoteContext) -> SolvencyResult`
- `check_pre_reservation(ctx: &ReservationContext) -> SolvencyResult`
- Read chain health from Phase 3.5 custody module
- Read lane status, vault sufficiency, unsettled notional from `x3-inventory`
- Event: `SolvencyGateChecked` on each evaluation

**Acceptance criteria:**
- Frozen lane causes `check_pre_quote` to fail with `LaneFrozen` in `failed_checks`
- Vault below threshold causes `check_pre_reservation` to fail with `InsufficientVault`
- Snapshot hash is deterministic for identical inputs
- All evaluated dimensions from the spec gate table are present in code

---

### TICKET-4.5-008: Pre-submission gate

**Crate:** `pallets/x3-solvency`  
**Effort:** M  
**Depends on:** TICKET-4.5-007

**Scope:**
- `check_pre_submission(ctx: &SubmissionContext) -> SolvencyResult`
- Checks: reservation validity, quote freshness (configurable delta), slippage bounds, signer path health, no new incident flags, reconciliation lag, partner reservation live, bridge path exists

**Acceptance criteria:**
- Expired reservation → `ReservationExpired` in `failed_checks`
- Stale quote → `QuoteStale` in `failed_checks`
- Failed signer health → `SignerPathUnhealthy` in `failed_checks`
- Gate runs within the same extrinsic as route submission

---

### TICKET-4.5-009: Post-submission tracking

**Crate:** `pallets/x3-solvency`  
**Effort:** M  
**Depends on:** TICKET-4.5-008

**Scope:**
- `record_post_submission(ctx: &PostSubmissionContext) -> Result<(), SolvencyError>`
- Record debit event to accounting spine
- Record pending obligation with route ID
- Enqueue timeout entry at `current_block + config_timeout_blocks`
- Seal evidence record (route ID, reservation ID, submission hash, block timestamp)
- Bind recovery path to route ID
- Update lane and global exposure dashboards

**Acceptance criteria:**
- Skipping this call after a successful pre-submission gate is a consensus error (enforced by coupling in submission extrinsic)
- Evidence record retrievable by route ID
- Pending obligation count increments and visible to subsequent gate checks
- Timeout queue entry created with correct expiry block

---

### TICKET-4.5-010: Solvency snapshot registry

**Crate:** `pallets/x3-solvency`  
**Effort:** S  
**Depends on:** TICKET-4.5-007

**Scope:**
- `SolvencySnapshots: StorageMap<H256, SolvencySnapshotRecord>`
- Record created on every gate evaluation
- Contains: block number, checked dimensions, pass/fail result, route/reservation context
- Prune snapshots older than configurable retention window, unless referenced by active reservation or pending obligation

**Acceptance criteria:**
- Snapshot retrievable by hash immediately after gate check
- Pruning does not remove snapshots for active reservations or pending obligations
- Hash is stable across identical inputs (determinism test)

---

## Wave 4 — Partner integration and treasury policy

Parallelizable with each other after Wave 2 completes.

---

### TICKET-4.5-011: Partner capacity tracking

**Crate:** `pallets/x3-partner`  
**Effort:** M  
**Depends on:** TICKET-4.5-003

**Scope:**
- `Partners: StorageMap<PartnerId, PartnerState>`
- `PartnerSupportedLanes: StorageDoubleMap<PartnerId, LaneId, ()>`
- Extrinsics: `register_partner`, `update_partner_status`, `record_partner_quote`, `confirm_partner_reservation`, `update_partner_exposure`
- Enforce `exposure_limit` at reservation time

**Acceptance criteria:**
- Partner below `MIN_PARTNER_HEALTH` returns `PartnerUnhealthyError`
- Exposure cap breach returns `PartnerExposureCapExceeded`
- Reservation confirmation emits `PartnerReservationAccepted`

---

### TICKET-4.5-012: Partner health scoring

**Crate:** `pallets/x3-partner`  
**Effort:** M  
**Depends on:** TICKET-4.5-011

**Scope:**
- `update_health_score(partner_id, metrics: PartnerMetrics) -> Result`
- `PartnerMetrics` covers all seven measured dimensions from the spec
- Health score: weighted composite in range `[0, 10000]` bps
- Weighting constants configurable via pallet config
- Metric inputs stored separately from computed score
- Event: `PartnerHealthUpdated`

**Acceptance criteria:**
- Score above `MIN_PARTNER_HEALTH` allows partner routes
- Score below threshold blocks routes without operator override
- Metric inputs stored for audit
- Weighting constants are runtime-configurable

---

### TICKET-4.5-013: Treasury policy caps and vault funding

**Crate:** `pallets/x3-treasury-policy`  
**Effort:** M  
**Depends on:** TICKET-4.5-002

**Scope:**
- `AllocationCaps: StorageMap<(ChainId, AssetId, LaneClass), Balance>`
- `TreasuryDeployedByLaneClass: StorageMap<LaneClass, Balance>`
- `InsuranceReserveBalance: StorageValue<Balance>`
- `fund_settlement_vault(vault_id, amount)` — cap enforced, deployed tracker updated
- `withdraw_from_vault(vault_id, amount)` — deployed tracker decremented
- `set_allocation_cap` — governance-gated extrinsic
- Insurance reserve unreachable via `fund_settlement_vault`

**Acceptance criteria:**
- Funding exceeding cap returns `AllocationCapExceeded`
- Insurance reserve unreachable from `fund_settlement_vault`
- Large cap expansions emit `GovernanceApprovalRequired`
- Deployed tracker consistent across fund and withdraw (property test)

---

## Wave 5 — Rebalance engine

Depends on Wave 2 completion and TICKET-4.5-013.

---

### TICKET-4.5-014: Rebalance trigger detection

**Crate:** `pallets/x3-rebalance`  
**Effort:** M  
**Depends on:** TICKET-4.5-004, TICKET-4.5-006

**Scope:**
- `on_initialize` hook: scan all vaults, check bands, enqueue triggers
- `RebalanceQueue: StorageValue<BoundedVec<RebalanceTrigger, MaxQueue>>`
- `RebalanceVolumeToday: StorageValue<Balance>` — reset each day via block number
- All seven trigger types from the spec
- Fast triggers prepend ahead of slow triggers

**Acceptance criteria:**
- Vault below `min_band` enqueues `BelowMinBand` trigger within the same block
- Fast trigger processed before any pending slow triggers
- Daily cap breach emits `RebalanceCapExceeded` and halts queue for the remainder of the day

---

### TICKET-4.5-015: Rebalance execution steps

**Crate:** `pallets/x3-rebalance`  
**Effort:** L  
**Depends on:** TICKET-4.5-014, TICKET-4.5-013

**Scope:**

Five-step execution in priority order:
1. Internal netting — scan `ReservationsByRoute` for offsetting demand
2. Cross-chain sweep — move balance from overfunded vault
3. Market rebalance — approved venue via signed oracle or off-chain worker
4. Partner-assisted rebalance
5. Treasury refill — Class C lanes only; calls `fund_settlement_vault`

- Each step emits `RebalanceStepAttempted` with outcome and reason
- Final emit: `RebalanceCompleted` with method, amount, source vault, dest vault

**Acceptance criteria:**
- Steps execute in order; treasury refill never runs if earlier step succeeds
- Failed step logs reason and advances to next step
- `RebalanceCompleted` includes all required fields
- Treasury refill on Class A or B lane returns `TreasuryRefillNotAllowedError`

---

## Wave 6 — Solvency sidecar and operator surface

TICKET-4.5-016 and TICKET-4.5-017 are parallelizable. TICKET-4.5-018 depends on TICKET-4.5-017.

---

### TICKET-4.5-016: Solvency sidecar service scaffold

**Crate:** `services/x3-solvency-sidecar`  
**Effort:** M  
**Depends on:** TICKET-4.5-009

**Scope:**
- Subscribe to all Phase 4.5 event families via node RPC
- In-memory state: vault utilization map, lane health map, frozen lane set, unsettled notional, partner health map, treasury at risk
- Prometheus endpoint: `/metrics`
- Configurable alerting webhook dispatch on threshold breaches
- REST: `GET /solvency/status`, `GET /solvency/vaults`, `GET /solvency/lanes`, `GET /solvency/partners`

**Acceptance criteria:**
- All nine metric families from the spec present in Prometheus output
- Frozen lane visible in REST response after `LaneFrozen` event received
- Alert fires within one polling cycle of threshold breach
- Disconnection triggers reconnect with state replay from last known block

---

### TICKET-4.5-017: Accounting spine event wiring

**Crates:** `pallets/x3-inventory`, `pallets/x3-reservation`, `pallets/x3-solvency`, `pallets/x3-rebalance`, `pallets/x3-partner`, `pallets/x3-treasury-policy`  
**Effort:** M  
**Depends on:** all Wave 1–5 tickets

**Scope:**

All required event families wired to the Phase 3 accounting spine:
- Route quoted, reserved, submitted, settled, failed
- Reservation expired
- Rebalance triggered, rebalance completed
- Treasury funded vault, treasury withdrew from vault
- Partner reservation accepted, partner reservation failed
- Loss event recorded
- Lane frozen, lane unfrozen

Each event must carry module, chain, lane, asset, and route ID where applicable.

**Acceptance criteria:**
- Integration test confirms all event families appear in accounting spine after a full route lifecycle
- No event family from the spec list is missing
- Event schema matches accounting spine ingestion contract

---

### TICKET-4.5-018: Phase 4.5 integration test suite

**Crate:** `tests/` (integration)  
**Effort:** L  
**Depends on:** TICKET-4.5-017

**Scope:**

Twelve required scenarios:
1. Full happy path: quote → reservation → submission → settlement → reconciliation
2. Route blocked at pre-quote gate (frozen lane)
3. Route blocked at pre-reservation gate (vault below threshold)
4. Route blocked at pre-submission gate (stale quote)
5. Reservation expiry and inventory release
6. Rebalance triggered and completed
7. Lane freeze on `critical_min` breach
8. Treasury cap enforcement
9. Partner health below threshold blocks route
10. Global unsettled cap breach blocks reservation
11. `record_post_submission` cannot be skipped (consensus enforcement verified)
12. Insurance reserve unreachable from settlement float path

**Acceptance criteria:**
- All twelve scenarios pass `cargo test`
- No scenario produces a storage inconsistency (invariant assertions after each scenario)
- Test output is deterministic across runs

---

## Dependency graph

```
Wave 1:  001, 002, 003            (fully parallelizable)
Wave 2:  004 -> 005 -> 006
Wave 3:  007 -> 008 -> 009
         007 -> 010               (010 parallel with 008)
Wave 4:  011 -> 012
         013                      (parallel with 011/012, after Wave 2)
Wave 5:  014 -> 015               (015 also needs 013)
Wave 6:  016                      (parallel with 017)
         017 -> 018
```

---

## Status tracking

| Ticket | Wave | Status |
|---|---|---|
| TICKET-4.5-001 | 1 | ✅ Complete |
| TICKET-4.5-002 | 1 | 🔲 Not started |
| TICKET-4.5-003 | 1 | 🔲 Not started |
| TICKET-4.5-004 | 2 | 🔲 Not started |
| TICKET-4.5-005 | 2 | 🔲 Not started |
| TICKET-4.5-006 | 2 | 🔲 Not started |
| TICKET-4.5-007 | 3 | 🔲 Not started |
| TICKET-4.5-008 | 3 | 🔲 Not started |
| TICKET-4.5-009 | 3 | 🔲 Not started |
| TICKET-4.5-010 | 3 | 🔲 Not started |
| TICKET-4.5-011 | 4 | 🔲 Not started |
| TICKET-4.5-012 | 4 | 🔲 Not started |
| TICKET-4.5-013 | 4 | 🔲 Not started |
| TICKET-4.5-014 | 5 | 🔲 Not started |
| TICKET-4.5-015 | 5 | 🔲 Not started |
| TICKET-4.5-016 | 6 | 🔲 Not started |
| TICKET-4.5-017 | 6 | 🔲 Not started |
| TICKET-4.5-018 | 6 | 🔲 Not started |
cd /home/lojak/Desktop/x3-chain-master/apps/x3-desktop && npm run tauri:dev