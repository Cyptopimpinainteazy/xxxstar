# X3 Go-Mode Execution Order

This file is the execution sequence for the next build phase. It takes the highest-leverage items across the multichain adapter work, swap router, sidecar daemon, evolution core, security swarm, launchpads, fee system, and user interfaces, then puts them into one order that can actually survive production. The point is still momentum, but the revised rule is stricter: nothing expands unless control, replay, custody, accounting, solvency, and operator truth all stay ahead of the surface area.

## Canonical truth doctrine

Before any phase is considered complete, the platform needs one canonical answer for balances, receipts, execution state, fee events, governance state, chain health, operator evidence, and user-visible status. The router, sidecars, treasury spine, operator cockpit, desktop client, browser extension, and web portal are allowed to project this truth in different ways, but they are not allowed to invent separate truths. If those views diverge, the platform is not scaling. It is fragmenting.

## Phase 0 — constitution, invariants, replay law, and emergency authority

Start by defining the machine before benchmarking the machine. This phase establishes canonical invariants, module boundaries, upgrade authority, deterministic replay rules for cross-chain actions, halt semantics, emergency powers, and the exact conditions under which execution may continue in degraded mode. It also defines the source-of-truth map for balances, receipts, fee events, governance state, and operator evidence so later phases have one reference model instead of competing interpretations.

Entry gate: the active modules, their ownership boundaries, and the current upgrade paths are documented and accepted as the temporary constitution for go-mode. Exit gate: invariants are registered, replay rules are testable, emergency powers are named, kill authority is explicit, and every critical subsystem knows which ledger or event stream is canonical for its truth.

## Phase 1 — prove the base system under load

Once the system is defined, build the proving ground. The universal chain registry, swap router, atomic bundle builder, sidecar daemon, evolution core, and cross-chain position manager need one testnet-facing proof pipeline that shows route correctness, latency, failure handling, fallback routing, rollback correctness, and receipt integrity. This phase ends with an operator-visible compatibility matrix showing which chains passed quoting, bundle construction, submission, rollback, reconciliation, and state-verification checks.

The proving dashboard is mandatory because the weekly chain onboarding program only works if the platform can demonstrate a repeatable benchmark and compatibility harness. It also blocks narrative drift. Claims about speed, reliability, or universal support have to stay anchored to measured scenarios.

Entry gate: Phase 0 invariants and replay rules exist, and the proving harness is wired to canonical receipts rather than ad hoc logs. Exit gate: bundle success rate, rollback correctness, and reconciliation accuracy hit published thresholds, and chains that miss those thresholds are automatically marked degraded or unsupported.

## Phase 2 — wire the security swarm into the execution path

The next move is enforcement. The security swarm scaffold already exists, but it needs to be wired into the orchestrator, quarantine manager, event stream, dispute path, and evidence pipeline so the system can detect, score, contain, and record incidents around swaps, custody actions, presales, auctions, launchpads, and later bot services. Sensitive actions need a real intent-to-action lineage and an operator view that shows the same incident from watcher, judge, warden, and scribe perspectives.

This phase is also where the fire doors become real. Circuit breakers, module-level kill switches, partial shutdown modes, quarantine rules, and dispute windows need explicit ownership and deterministic behavior. Stop conditions are not enough unless the platform can actually stop the correct thing without stopping everything.

Entry gate: the proving pipeline emits events and receipts in a form the swarm can consume. Exit gate: incident detection latency, quarantine activation, evidence sealing, and dispute initiation meet target thresholds, and each sensitive surface has a documented pause path with named authority.

## Phase 3 — finish the accounting and treasury backbone

After proof and enforcement, finish the money layer. The fee-as-infrastructure model needs authoritative implementation rules for where fees are taken, what counts as principal versus yield, how splits are routed, how cross-chain fee legs settle, how maintenance accrues, and how every module emits auditable revenue events. The target is one accounting spine for swaps, lending, flashloans, auctions, DNS, dApp hub fees, RPC monetization, and future AI services.

This is also where the go-mode revenue model becomes measurable rather than aspirational. If the accounting system cannot reconcile by module, chain, asset, and receipt lineage, every later growth forecast is noise.

Entry gate: Phase 2 evidence paths are live for money-moving actions. Exit gate: reconciliation accuracy reaches target, every revenue surface emits canonical events, and treasury reports can be regenerated from receipts without operator patchwork.

## Phase 3.5 — key management, signer architecture, and custody boundaries

Signer design gets its own phase because X3 has too many trust edges to treat custody as an implementation detail. This phase defines validator keys, treasury keys, wrapped-asset mint and burn authority, sidecar signing separation, operator-versus-protocol keys, HSM or enclave or MPC policy, key rotation, compromise recovery, approval thresholds, and the audit trail for every signing request. The output is a custody map that tells the platform which signer may act, under which policy, for which asset, on which chain.

The rule for this phase is simple: no chain-facing authority is allowed to exist without explicit ownership, rotation policy, compromise playbook, and log evidence. If that sounds slow, it is still faster than discovering the real signer model during a bridge incident.

Entry gate: the accounting spine knows which actions require signing and which authority domain each action belongs to. Exit gate: signer separation is implemented, rotations are tested, compromise recovery is rehearsed, and no critical operation depends on a single undocumented hot path.

## Phase 4 — settle the RPC strategy before scaling traffic

RPC strategy is part of the product surface, not a background utility. Lock the hybrid model: self-host the traffic-heavy chains that determine router latency and execution safety, outsource or federate the long tail, define health-based fallback order, define cache rules for read-heavy endpoints, and decide whether premium RPC is a first-wave monetization surface or a later platform service.

This phase matters because the wallet, explorer, launchpads, dApp hub, bot rental surfaces, proving arena, and operator dashboard all become unreliable if RPC routing stays ad hoc.

Entry gate: signer architecture and accounting paths are already stable enough to distinguish infra failure from settlement failure. Exit gate: fallback recovery time, provider health scoring, cache behavior, and degraded-mode routing meet target thresholds across the priority chain set.

## Phase 4.5 — liquidity, inventory, and solvency backbone

This phase closes the balance-sheet gap between infrastructure reliability and omnichain asset portability. It defines where execution liquidity comes from, who owns settlement inventory, how treasury participates, how cross-chain balances are rebalanced, how LP and market-maker relationships are integrated, and what solvency checks must pass before any route becomes executable. The router is not allowed to assume liquidity. It has to request it, reserve it, and survive the move.

The current operator-grade definition for this layer lives in [docs/specs/X3_LIQUIDITY_INVENTORY_SOLVENCY_SPEC.md](docs/specs/X3_LIQUIDITY_INVENTORY_SOLVENCY_SPEC.md). Use that document as the implementation contract for vault classes, lane classes, reservation semantics, solvency gates, rebalancing order, and failure handling.

The current execution breakdown for building Phase 4.5 into the repo lives in [docs/implementation/PHASE_4_5_LIQUIDITY_IMPLEMENTATION_PLAN.md](docs/implementation/PHASE_4_5_LIQUIDITY_IMPLEMENTATION_PLAN.md). Use that plan when assigning slices across routing, accounting, settlement, sidecar telemetry, and freeze automation.

The operating doctrine is strict. The router discovers, prices, and ranks paths, but it owns no assets. An inventory manager reserves or rejects capacity by chain, asset, and lane. Treasury allocates bounded capital, funds approved vaults, and acts as a backstop only within policy. Liquidity should come first from external venue depth, then from partner LP or market-maker lanes, then from limited protocol-owned settlement float on top corridors, with internal user-flow netting used wherever flows offset naturally. Treasury is not a prop desk, and long-tail inventory is not a strategy.

Rebalancing must run on inventory bands with minimum, target, and maximum thresholds for each chain and asset pair. Internal netting happens first, cross-chain sweeps second, market rebalancing third, and emergency treasury refill last. Solvency gates must exist before quote, before reservation, before submission, and after submission until final settlement. A route is executable only if liquidity exists, inventory is reserved, and post-trade solvency remains above threshold.

Entry gate: RPC health, signer health, and accounting events are all stable enough to distinguish route profitability from solvency failure. Exit gate: reservation service, inventory bands, partner scorecards, solvency checks, and rebalance workflows are live, and failed routes due to solvency gates are observable rather than mysterious.

## Phase 5 — make X3 portable across chains with wrapped-token discipline

Only after proof, security, accounting, custody, RPC routing, and solvency controls are stable should the platform treat X3 as an omnichain asset. This phase covers native token rules, wrapped token mint and burn flow, governance power mapping, staking compatibility, bridge fee collection, supply reconciliation, and failure recovery. The target is not merely wide presence. The target is one governed asset with many representations and no accounting ambiguity.

Entry gate: custody, solvency, and replay guarantees already cover mint, burn, bridge, and rollback paths. Exit gate: supply reconciliation is exact across chains, wrapped asset failure budgets are defined, and governance power mapping does not diverge from canonical token state.

## Phase 6 — ship the validator presale and blockspace auction only after the base rails exist

The validator presale and blockspace auction are high-leverage monetization and coordination primitives, but they should not go live before the earlier phases. They need tested wrapped-token participation, treasury routing, receipt integrity, dispute handling, kill-switch coverage, and operator visibility. Once those exist, auctions become a legitimate way to price scarce access rather than a risk surface waiting for the first exploit.

Entry gate: wrapped X3 is live with reconciled supply, and dispute and settlement paths are operational. Exit gate: auction rules are explicit, appeals and settlement windows are implemented, and the accounting and operator layers can reconstruct every auction outcome from canonical evidence.

## Phase 7 — launch the token and NFT launchpads on the same rails

The launchpads should run on the same treasury hooks, approval flows, wrapped-token support, signer separation, and security-swarm protections already established. That keeps presales, mints, creator allocations, treasury splits, and cross-chain participation inside the same evidence and accounting system instead of creating a separate product silo.

This phase also needs product segmentation discipline. Token launchpads, NFT launchpads, validator sales, fee-sharing SDK flows, and wrapped-token surfaces do not all carry the same legal or operational assumptions, so launch rules must inherit the compliance map instead of improvising one per release.

Entry gate: auction, custody, dispute, and compliance controls already exist for the assets and jurisdictions being exposed. Exit gate: creator flows, treasury splits, incident handling, and jurisdictional gating are enforced through shared rails rather than manual exceptions.

## Phase 8 — open the dApp hub with revenue-sharing SDK and clear boundaries

The dApp hub should come after launchpads. By then the platform already has wallet flows, fee routing, launch mechanics, compliance segmentation, and treasury accounting that third-party applications can inherit. The first version should favor curated entry, a revenue-share SDK, optional boosted placement, and explicit policy rules for what the platform can rank, throttle, suspend, or feature.

Entry gate: shared fee and evidence primitives are stable enough for third-party inheritance. Exit gate: listing policy, throttling policy, revenue-share accounting, and incident escalation are enforceable from the core platform rather than from per-dApp custom logic.

## Phase 9 — finish the user-facing triangle: desktop, browser extension, and web portal

The user ecosystem should be completed as one coordinated surface, not as three disconnected apps. The desktop app is the heavy-control and GPU-participation client. The browser extension is the transaction and notification surface. The web portal is the analytics, launchpad, governance, and marketplace view. All three need one identity model, one asset model, one treasury and reporting model, and one approval narrative grounded in the canonical truth doctrine from Phase 0.

Entry gate: core systems already publish canonical balances, receipts, incident state, and fee state through stable interfaces. Exit gate: the three surfaces agree on user balances, route state, incident status, and treasury-derived rewards with no unexplained drift.

## Phase 10 — productize the AI swarm as a service, not just an internal engine

Only after the base economy, user surfaces, and marketplace are stable should the platform externalize its AI edge. That includes bot rental, premium execution tiers, compute marketplace services, AI media and content services, analytics APIs, and related monetized products. At that point the GPU swarm stops being only an optimization engine for internal products and becomes an external business line.

Entry gate: accounting, evidence, SRE ownership, and signer boundaries already cover the service surface being exposed. Exit gate: bot services, compute billing, analytics usage, and incident controls all inherit canonical accounting and operator truth instead of inventing separate service rails.

## Phase 11 — run the weekly chain proving and onboarding cycle

Once the proving arena, wrapped asset layer, treasury hooks, liquidity model, and governance surfaces are stable, turn on the weekly chain program. The weekly cycle should combine technical proof, economic score, infrastructure requirements, liquidity opportunity, compliance constraints, and commercial terms. New chains should not be added on narrative or relationships alone. They should enter through a measured process the market can inspect.

Entry gate: the proving harness, liquidity scorecard, and operational support model are all mature enough to classify chains honestly. Exit gate: each admitted chain has a measured compatibility record, an operating owner, a liquidity policy, and an explicit downgrade or removal path.

## Phase 12 — track A-tier progress with hard metrics, not vibes

Go-mode should finish with a scorecard, not a slogan. Track throughput, block latency, bundle success rate, rollback correctness, reconciliation accuracy, TVL, route volume, daily active users, developer count, dApp revenue, launchpad volume, treasury growth, RPC revenue, security incident rates, and bot service usage. Add the balance-sheet metrics too: inventory utilization by lane, rebalance cost per routed dollar, partner fill reliability, stale quote rejection rate, post-trade reconciliation lag, undercollateralized lane incidents, failed routes due to solvency gates, treasury capital efficiency, protocol-owned inventory PnL, and netted flow percentage.

Entry gate: prior phases already emit these metrics from canonical systems rather than synthetic spreadsheets. Exit gate: the scorecard can expose wins and failures without operator reinterpretation.

## Cross-cutting control tracks

Three tracks begin early and continue through the roadmap even though they are not separate product phases. The first is the risk, dispute, and kill-switch model. It starts in Phase 0, becomes executable in Phase 2, and must be live before public auctions, launchpads, or third-party dApps. The second is the compliance segmentation map. It begins as a jurisdiction-by-product matrix in Phase 0, expands across launch surfaces in Phases 6 through 8, and blocks any rollout where product assumptions and jurisdictional assumptions no longer match. The third is SRE and day-two operations. It starts with proving and RPC ownership, then matures into pager paths, severity definitions, backups, restore drills, region failover, chain-specific playbooks, evidence retention, and operator accountability before the user-facing triangle and AI service layer go broad.

## Immediate go-mode order

1. Constitution, invariants, replay rules, emergency powers, and canonical truth map.
2. Testnet proving pipeline and compatibility scoreboard.
3. Security swarm wiring into live execution, kill-switches, and evidence paths.
4. Accounting, treasury, and fee routing completion.
5. Key management, signer architecture, and custody model.
6. RPC hosting, outsourcing, fallback, and monetization plan.
7. Liquidity, inventory, solvency, and rebalancing backbone.
8. Wrapped X3 and omnichain governance portability.
9. Validator presale and blockspace auction.
10. Token and NFT launchpads.
11. dApp hub revenue-sharing platform.
12. Desktop, extension, and web portal unification.
13. External AI services and bot marketplace.
14. Weekly chain proving plus onboarding program.
15. A-tier scorecard and milestone tracking.

## Red-line dependencies

Wrapped X3 does not ship before custody, replay, and solvency controls are working together. Auctions and launchpads do not ship before dispute windows, compliance segmentation, and operator pause paths are live. The dApp hub does not ship before shared accounting and evidence primitives can be inherited by third parties. User-facing surfaces do not diverge from the canonical truth map. AI services do not create a second accounting system, a second custody model, or a second incident pipeline.

## Stop conditions

Pause feature expansion if route correctness becomes ambiguous, accounting cannot reconcile across modules, signer authority is unclear, security containment is not wired into the new surface, liquidity cannot be reserved deterministically, solvency gates are bypassed, RPC fallback stays manual, compliance assumptions are unresolved for the product being launched, or user-facing apps diverge from the same source of truth. Those are the signals that the platform is adding shape faster than it is adding discipline.
