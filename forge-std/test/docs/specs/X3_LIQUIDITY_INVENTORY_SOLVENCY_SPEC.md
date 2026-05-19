# X3 Liquidity, Inventory, and Solvency Spec v1

## Placement in the go-mode sequence

This spec defines the operator-grade control layer that sits between [RPC strategy](../RPC_CONFIGURATION.md) and [wrapped X3 portability](../X3_COIN_LAYER_SPEC.md). It exists because omnichain execution is not safe until settlement liquidity, inventory ownership, reservation semantics, and solvency gates are explicit. The router may discover a technically valid path, but the platform must still prove that the path can reserve capital, settle the obligation, reconcile the receipts, and remain solvent after the move.

The spec is written to plug directly into `Phase 4.5` of [GO_MODE_EXECUTION_ORDER.md](../../GO_MODE_EXECUTION_ORDER.md). It also assumes the constitutional controls from `Phase 0`, the accounting spine from `Phase 3`, and the signer and custody boundaries from `Phase 3.5`.

The current build breakdown for implementing this layer lives in [docs/implementation/PHASE_4_5_LIQUIDITY_IMPLEMENTATION_PLAN.md](../implementation/PHASE_4_5_LIQUIDITY_IMPLEMENTATION_PLAN.md). Use that document when turning this spec into concrete work across routing, accounting, sidecar telemetry, settlement linkage, and lane-freeze control paths.

The thin execution slices for the same work live in [docs/implementation/PHASE_4_5_EXECUTION_TICKETS.md](../implementation/PHASE_4_5_EXECUTION_TICKETS.md). Use that file when splitting the implementation into independently buildable waves.

## Purpose and operating doctrine

This layer defines where execution liquidity comes from, who owns settlement inventory, how treasury participates, how rebalancing works, how LPs and market makers are integrated, and which solvency conditions must pass before cross-chain execution is allowed. Its job is to stop the router from turning route availability into hidden balance-sheet risk.

The operating doctrine is narrow and enforceable. The router discovers paths. The inventory manager approves paths. Treasury funds bounded float. The solvency engine blocks unsafe execution. No route becomes executable unless liquidity exists, inventory is reserved, and post-trade solvency remains above required thresholds.

## Goals and non-goals

The system is meant to execute cross-chain routes without hidden inventory risk, keep routing logic separate from capital ownership, prevent treasury from turning into an accidental prop desk, support external liquidity and partner-backed depth alongside limited protocol-owned float, and preserve a complete audit trail from quote to final settlement. It is also meant to scale lane coverage without eroding operator visibility.

This system is not a speculative market-making engine. It is not permission for treasury to warehouse long-tail assets, not a trust-based partner routing layer, and not a free-form internalization mechanism for user flow that bypasses reconciliation.

## System actors and role boundaries

### Router

The router handles venue discovery, price discovery, route scoring, and route ranking. It creates route requests and hands them to the reservation path. It never owns assets, never forces treasury usage, never bypasses the solvency engine, and never upgrades an indicative quote into a firm execution on its own authority.

### Inventory Manager

The inventory manager tracks inventory by chain, asset, vault class, and lane. It reserves and releases balances, enforces exposure limits, activates or freezes lanes, and raises rebalance requests. It is the decision point for whether a route can consume operational capacity.

### Solvency Engine

The solvency engine runs the hard gates. It performs pre-quote checks, pre-reservation checks, pre-submission checks, and post-submission tracking. It owns threshold enforcement and is allowed to block execution even when a route is profitable if the route would push the system outside policy.

### Treasury

Treasury allocates capital, funds top-lane float, funds gas reserves, manages insurance reserves, and defines policy caps and risk budgets. Treasury does not perform ad hoc lane rescues outside policy, does not carry broad speculative books, and does not sit in the hot path of individual route selection.

### LP and market-maker partners

Partners provide lane-specific or route-specific depth, live quote response, and fill reliability. They remain external counterparties with measurable obligations rather than informal relationships.

### Vault Controller

The vault controller manages operational balances, chooses the signer or custody path for a vault action, reports vault state, and executes transfers, sweeps, and rebalance actions. It is the custody-aware execution boundary underneath inventory management.

## Liquidity source model

X3 lanes may use four liquidity buckets. The first is netted internal flow, where offsetting user demand reduces the need for external movement. The second is external market liquidity through DEX pools, aggregators, order books, or approved bridge liquidity. The third is partner LP or market-maker liquidity for lanes that need committed depth or tighter operational guarantees. The fourth is protocol-owned settlement float, which exists only as a bounded backstop on approved corridors where continuity and latency matter.

The priority order is deliberate. Internal netting is cheapest and reduces market footprint. External market liquidity gives broad coverage without turning treasury into an inventory owner on every lane. Partner liquidity is for strategic corridors that need reliability beyond public markets. Protocol-owned float comes last because treasury should act as operator of last resort, not default market maker.

## Ownership model

The router owns no assets. Operational balances live in vaults with clear owner and purpose. Treasury owns designated reserves and approved deployed float, but that capital enters execution only through policy-governed funding actions. Partner balances remain partner-owned until a reservation is confirmed and a route is actually bound for execution.

This split keeps accounting legible. The planning layer remains separate from the capital layer, and route selection cannot quietly mutate into treasury risk-taking.

## Vault model

### Gas Reserve Vault

This vault holds chain gas tokens and operational fee assets. It covers execution overhead and emergency routing overhead. It is never used as settlement principal, and each supported chain must maintain a minimum threshold.

### Settlement Float Vault

This vault holds route settlement balances for approved chain and asset combinations. It is bounded by treasury policy, monitored by inventory bands, and used for execution continuity on selected corridors.

### Treasury Reserve Vault

This vault holds strategic reserve capital and non-operational assets. It is a deployment source rather than a route execution pool. The router cannot touch it directly.

### Insurance and Loss Reserve Vault

This vault contains funds used to contain approved loss events or failed settlement recovery. It is accounted for separately, reported explicitly, and never blended into normal settlement PnL.

### Partner Capacity Record

This is not an X3-owned vault, but it is modeled as one capacity source in the control plane. It tracks available partner capacity, quote health, reservation windows, and partner exposure usage.

## Lane model and classes

A lane is a route-support domain defined by source chain, destination chain, asset pair, execution method, approved liquidity sources, and risk class. Lane definitions are how X3 turns a generic route graph into enforceable operating policy.

Class A lanes are market-only lanes. They use public liquidity, accept wider spreads, and do not depend on protocol backstop. Class B lanes are partner-backed lanes with approved counterparties, expected depth commitments, and tighter service expectations. Class C lanes are protocol-backed strategic corridors where limited protocol-owned float is allowed and monitoring is stricter than on any other class.

## Risk limits and threshold tiers

Every chain, asset, partner, and lane must have hard limits. These include maximum inventory per asset, maximum inventory per chain, maximum inventory per lane, maximum partner exposure, maximum unsettled notional, maximum pending bridge notional, maximum treasury deployment by lane class, maximum daily rebalance volume, and maximum tolerated loss before automatic freeze.

Each monitored object moves through `normal`, `warning`, `degraded`, and `frozen` states. A frozen state means no new execution. A degraded state may still allow reduced functionality, but only under the rules established in the constitutional control layer.

## Inventory band policy

Each `(chain, asset)` pair carries four inventory bands: `critical_min`, `min`, `target`, and `max`. Falling below `critical_min` freezes new routes that consume that inventory. Falling below `min` triggers rebalance. Remaining between `min` and `max` is normal operation. Exceeding `max` triggers a sweep or redeployment action. Top corridors may apply lane-aware overlays on top of base chain and asset bands.

## Solvency model

A route is solvent if source obligations are funded, destination obligations are fundable, gas and operational fees remain covered, exposure caps remain within policy, and unsettled obligations remain within safety budget after reservation and execution. Solvency is checked across source chain, destination chain, asset, route, partner, treasury policy envelope, and global unsettled exposure. That scope matters because a route that looks safe in isolation may still be unsafe in aggregate.

## Route lifecycle state machine

The route lifecycle must be explicit because reservations, solvency decisions, and failure recovery all depend on state identity.

```text
DISCOVERED
  -> INDICATIVE_QUOTED
  -> PRECHECK_PASSED
  -> RESERVATION_REQUESTED
  -> RESERVED
  -> SUBMISSION_READY
  -> SUBMITTED
  -> PENDING_SETTLEMENT
  -> SETTLED
  -> RECONCILED

Failure branches:
PRECHECK_FAILED
RESERVATION_REJECTED
QUOTE_STALE
SUBMISSION_ABORTED
SETTLEMENT_DELAYED
SETTLEMENT_FAILED
ROLLBACK_INITIATED
LOSS_EVENT_RECORDED
LANE_FROZEN
```

The transition model is intentionally conservative. A route cannot skip from discovery to submission because reservation and solvency state must exist in the middle. A route that reaches `SUBMITTED` must already have an obligation record, an evidence seal, and a recovery binding.

## Solvency gates

### Pre-quote gate

Before returning a firm quote, the engine checks source chain health, destination chain health, route component health, asset activation on both sides, lane freeze status, quote freshness threshold, and tentative capacity. If any of these checks fail, the system may return an indicative quote but not a firm executable route.

### Pre-reservation gate

Before reserving inventory, the engine checks source vault sufficiency, destination capacity, gas reserve sufficiency, exposure caps, pending unsettled notional, partner health, route profitability after fee and failure-cost modeling, and quarantine status on route components.

### Pre-submission gate

Right before execution, the engine checks that the reservation is still valid, the quote is still fresh, slippage bounds remain valid, the signer path is healthy, no new incident flag has appeared, reconciliation lag is within threshold, partner reservation remains live, and the bridge or settlement path still exists.

### Post-submission tracking gate

After submission, the system records debit events, records pending obligations, starts timeout tracking, seals evidence, binds a recovery path to the route ID, and updates exposure dashboards. Submission without these side effects is invalid because it creates economic exposure without operator truth.

## Reservation model

Reservations are explicit, time-bounded, and balance-reducing. A reservation contains route ID, lane ID, source chain, destination chain, source asset, destination asset, source amount, destination amount, liquidity source type, optional partner ID, reservation start time, reservation expiry time, slippage tolerance, maximum fee envelope, and a solvency snapshot hash.

Expired reservations cannot execute. Reserved inventory is removed from available capacity immediately. Release occurs on settlement, failure, or timeout. Stale reservations continue to count toward risk until release, which prevents invisible capacity leakage.

## Rebalancing model

Rebalancing exists to preserve lane continuityn without letting treasury drift into unmanaged exposure. The system first attempts internal flow netting, then cross-chain sweep from overfunded vaults, then market rebalance through approved venues, then partner-assisted rebalance, and finally treasury refill for critical lanes only.

Two rebalance modes are required. Slow rebalance is scheduled and cost-aware. Fast rebalance is event-driven and used when critical thresholds approach or when a chain or partner suddenly degrades. Rebalance triggers include falling below `min`, projected demand spikes, concentration breaches, partner capacity loss, venue liquidity collapse, persistent one-way flow, and chain degradation.

## LP and market-maker integration model

Partners enter the system only after legal approval, technical integration approval, reliability review, auditable reporting approval, supported-lane definition, response-time benchmarking, and settlement dispute format agreement. Their measured metrics include quote response time, fill reliability, rejected reservation rate, stale quote rate, average spread by lane, dispute count, and settlement delay rate.

A partner route is eligible only if the partner is active, the lane is approved, a live quote exists, reservation is confirmed, exposure caps remain within limits, and partner health remains above threshold. That rule removes handshake routes from the control plane.

## Treasury policy

Treasury may hold gas assets, top-lane settlement assets, contingency reserves, and strategic X3 reserves. Treasury may not default to broad long-tail inventory, speculative directional books, unsupported lane assets, or partner liabilities. Allocation caps must exist by chain, asset, and lane class, and the insurance reserve must remain distinct from the operational settlement float. Large expansions require governance approval rather than operator improvisation.

## Accounting requirements

Every material action in this system emits an auditable event. Required event families include route quoted, route reserved, route submitted, route settled, route failed, reservation expired, rebalance triggered, rebalance completed, treasury funded vault, treasury withdrew from vault, partner reservation accepted, partner reservation failed, loss event recorded, and lane frozen or unfrozen.

These events must feed the accounting spine so reconciliation remains exact by module, chain, lane, asset, and route ID. This spec does not tolerate inventory logic that can only be reconstructed from logs after the fact.

## Hard invariants

Several rules are non-negotiable. The router never owns capital. No execution happens without reservation. No firm route exists without solvency pass. Treasury deployment stays within policy caps. Settlement float and insurance reserve remain separate. Every pending obligation maps to a route ID. Every route is reconcilable end to end. Frozen lanes accept no new execution. Protocol-backed lanes carry stricter monitoring than market-only lanes. User-visible execution state matches accounting state.

## Failure handling and recovery paths

A lane freezes automatically if balance drops below `critical_min`, if unresolved settlement failure exceeds budget, if a partner defaults or becomes unresponsive, if quote integrity becomes unreliable, if chain health degrades beyond threshold, or if reconciliation mismatch exceeds tolerance. Those conditions force the platform to stop consuming risk rather than continue in denial.

Recovery begins with reservation release where possible. Before submission, the system may reroute onto an alternate path if solvency and freshness still hold. After submission, the system moves into delayed-settlement watch, rollback if supported, or approved insurance-reserve use. Manual operator intervention is allowed only with evidence logging and under the control rules defined in the go-mode constitution.

## Metrics

This subsystem adds its own scorecard. Inventory metrics include utilization by chain and asset, settlement float idle ratio, and gas reserve days of coverage. Route metrics include firm quote conversion, reservation rejection, stale quote rejection, execution success, and settlement time by lane. Risk metrics include unsettled notional, under-threshold incidents, frozen lane count, partner concentration ratio, and treasury capital at risk. Rebalance metrics include frequency, average cost, internally netted flow percentage, and rebalance latency.

## Suggested implementation order

Start by defining vault classes, lane classes, exposure caps, and inventory bands. Then build the inventory manager, reservation engine, and solvency engine. Then wire the router to reservation requests, treasury to vault funding controls, and accounting to event emission. After the reservation and solvency path is stable, add the rebalance engine, partner integration layer, and lane freeze or unfreeze automation. The last implementation step is the operator surface that exposes solvency status, lane health, inventory state, and pending obligations.

## Minimal data model

The control plane needs a stable data model that the accounting spine, operator cockpit, and execution services can all share.

```text
Vault {
  vault_id
  vault_type
  owner_type
  chain_id
  asset_id
  available_balance
  reserved_balance
  pending_out_balance
  pending_in_balance
  critical_min
  min
  target
  max
  status
}

Lane {
  lane_id
  source_chain
  dest_chain
  source_asset
  dest_asset
  lane_class
  allowed_liquidity_sources[]
  status
  exposure_cap
  unsettled_cap
}

Reservation {
  reservation_id
  route_id
  lane_id
  liquidity_source_type
  partner_id?
  source_amount
  dest_amount
  expiry_ts
  status
  solvency_snapshot
}

Partner {
  partner_id
  status
  supported_lanes[]
  health_score
  exposure_limit
  current_exposure
}
```

## Final operating rule

X3 should never ask only whether a route can technically execute. The control plane has to ask whether the route can execute, settle, reconcile, and leave the system solvent afterward. That is the real execution question, and this spec exists to keep the platform honest when speed and growth pressure start arguing otherwise.
