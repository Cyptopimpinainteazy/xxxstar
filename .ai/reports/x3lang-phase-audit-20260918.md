# X3Lang phase audit — 56 phases, 2026-09-18

## Why this file exists

The build order's 25 items are complete and verified (`c120305b4`). The spec has
**56 phases**, and "finish x3lang with all features" is the larger claim. This is
the audit that has to precede any completion claim: for each phase, name the
artifact that would prove it and inspect *that*.

**It is deliberately partial, and the verdict column says so.** A verdict of
`unaudited` is not a guess in either direction; it means this pass did not gather
the evidence, and the phase remains open until it does. Filling those in is the
next pass.

## Method, and why not a keyword sweep

A keyword sweep was tried first and is recorded here because it *misled twice*:
`fixed_point` appears nowhere in the tree, yet the trading core does fixed-point
arithmetic in integer base units with per-asset decimals (`AssetId.decimals`,
"converted to base units during semantic analysis only when the conversion is
exact"); and `fuzzing` appears nowhere while five test files use `proptest!`. A
phase is implemented when the *behaviour* exists on a reachable path, so each
verdict below names the artifact it was checked against.

Evidence labels: **[now]** a command run for this audit; **[round N]** a command
recorded in `.ai/reports/x3lang-roundN-20260918.md`; **[inferred]** an artifact
exists but was not traced to a reachable path; **[absent]** no artifact found;
**[unaudited]** not examined.

## Verdicts

| # | Phase | Verdict | Evidence |
|---|---|---|---|
| 1 | Economic type system | **partial** | `AssetId` with VM/chain/canonical-id/decimals as identity [now]; asset and chain mismatches rejected for trading statements [round 22 tests]. The `Amount<T>`/`Price<A,B>` *generics* and general dimensional analysis are absent — `1 ETH + 1 USDC` is an unchecked integer add. |
| 2 | Financial ownership model | **implemented** | debt opens on `borrow`, closes on `repay`; negative tests for missing, duplicate and unknown repayment [round 5–13, `test_trading_verifier.rs`]. Not checked: use-before-borrow, phantom collateral, debt reuse after closure — see GAPS-1. |
| 3 | Economic effect system | **implemented** | declared effects and guarantees must be discharged by the body, with the fix named in the diagnostic; extended from `atomic trade` to `strategy` modules [round 26]. Coded diagnostics (`X3E-4021`-style) are absent for the trading path — TICKET-021. |
| 4 | Stateful trading IR | **partial** | `TradingOperation` carries the trading instruction set and the verifier reasons across the sequence [now]; BEGIN/END balance, debt-closed-once and effect discharge are checked. The spec's list also names POSITION_OPEN/CLOSE, HEDGE, LIQUIDATE, COLLATERALIZE and FINALITY_CHECK — positions and hedging are not modelled. |
| 5 | Profit as a first-class value | **implemented** | `vm/src/profit.rs`: one field per `CostKind` exhaustively, `net` signed, `margin_bps` integer, `realized()`, and `hedging_costs()`/`unrealized()` returning `None` with their reasons. The decomposition is **reconciled** against the recorded delta (`ProfitReconciliationMismatch`), and both floor checks read its `net` [round 31]. |
| 6 | Profit-only transactions | **implemented** | below the floor the trade does not settle (`NetProfitBelowFloor`, and `TradeOutcome` refusals) [now, round 13]. |
| 7 | Native risk policy | **implemented** | `risk policy` / `risk_policy` declarations with slippage, fee, gas and deadline ceilings, enforced at execution [rounds 10–13]. |
| 8–13 | Atomic cross-VM, branch, fallback, freshness, state-locking | **implemented (12, 13, 17 partly)** | item 14 (`atomic_choice`), item 15 (`fallback`), quote freshness; corpus runs [rounds 17, 21, 22]. |
| 14 | Opportunity graph | **implemented** | `compiler/src/opportunity.rs`, `x3c graph` [round 19]. |
| 15 | **Objective-driven programs** | **implemented** | `ObjectiveDecl` / `ObjectiveMetric` / `ObjectiveConstraints` in `crates/x3-ast/src/ast.rs`; `objective { … }` parses, formats and verifies; `x3c optimize` follows the declared metric and ceilings and names the objective it followed, refusing a contradicting `--objective` [round 32]. The four metrics the graph has no quantity for are refused **with their reason** rather than ranked by a solver, which is what the phase's determinism requirement asks for. Two checks this found: `max_fee_bps` was enforced nowhere, and `risk <= strategy.policy` compared a slippage ceiling against a risk score — both fixed. Not done: the phase's phrase "maximize net_profit" is only *refusable* until the graph models amounts and prices. |
| 16 | Parallel financial execution | **implemented** | `compiler/src/dag.rs`, `PARALLEL_PLAN`, races and cycles refused [round 21]. |
| 17 | Route freshness | **implemented** | `quote_freshness` and `QuoteStale` [rounds 13, 23]. |
| 18 | State-locked execution | **absent** | no state-lock primitive; `state-locked` matches nothing [now]. |
| 19 | Multi-asset flash capital | **partial** | `borrow` and the flash-capital permission exist [round 26]; multi-asset flash is not modelled, and `flash_capital` cannot even be *required* in a module body (TICKET-040). |
| 20 | Flash collateral | **partial** | as 19. |
| 21 | Intent fusion | **implemented** | `compiler/src/fusion.rs`, `x3c fusion`; rings, consent, five checks [round 24]. |
| 22 | Cross-domain netting | **partial** | fusion rings net assets within what the graph knows [round 24]; there is no cross-*domain* netting of settlement paths. |
| 23 | Strategy modules | **implemented** | eight declarations, each verified against the body [round 26]. |
| 24 | Strategy licensing | **implemented** | licence, royalty and split checked against each other [round 27]. |
| 25 | Profit splitting | **implemented** | `split profit`, sums to 10,000 bps, requires a profit floor [round 27]. |
| 26 | Strategy marketplace foundation | **implemented** | `x3c metadata`: id, artifact hash, compiler version, risk class, chains, capital min/max, permissions, licence, author, and the two fields a compiler cannot fill labelled [round 29]. |
| 27 | Private strategies | **partial** | privacy levels labelled and unimplemented ones refused; the commitment level is real [round 28]. No encryption, deliberately. |
| 28 | Private submission | **implemented** | policy in the artifact, enforced fail-closed against the runtime's capability [round 28]. |
| 29 | **Opportunity packets** | **absent** | no `OpportunityPacket` type [now]. |
| 30 | **Dedicated execution lanes** | **absent** | no lane class [now]. |
| 31 | Economic receipts | **implemented** | `build_receipt`, `verify_receipt` [now]. |
| 32 | Economic replay | **implemented** | `verify_receipt_economics`, `EconomicReplayMismatch` [now]. |
| 33 | Programmable finality | **implemented** | `finality_policy` declaration lowered to a finality requirement [round 4]. |
| 34 | Execution deadlines | **implemented** | deadline in the trade policy, `DeadlineExpired` [now]. |
| 35 | Compiler cost model | **partial** | per-opcode gas costs exist in the VM (`gas_cost_for_opcode`, surcharges) [now]; the *compiler* does not model cost ahead of execution. |
| 36 | Static route profitability | **partial** | `RiskScorer` over intent routes; the graph ranks by declared attributes, not by computed profit [round 19]. |
| 37 | X3 arbitrage IR | **partial** | the trading IR is the arbitrage IR in substance; the spec's name and its `HYPERARB` companion are absent. |
| 38 | **Hyperarb** | **absent** | no hyperarb primitive [now]. |
| 39 | **Atomic CEX/DEX intents** | **absent** | no CEX venue model [now]. |
| 40 | Permission and capability system | **implemented** | `CapabilityPayload`, `CapabilityManifest`, per-op capability checks [rounds 5–13, 28]. |
| 41 | Resource caps | **implemented** | `max_gas`, `max_steps`, register/memory caps, gas-metered execution [now]. |
| 42 | Determinism | **scanned** | `compiler/tests/test_computation_discipline.rs` scans `compiler/src` and `vm/src`: no clock reads, no randomness, no float code shapes in money or policy paths (one exemption, for Solana's own `f64` account field, with its reason on the line) [round 30]. Unordered maps are not scanned — membership use is fine and iteration order is what matters, which a textual scan cannot tell. |
| 43 | Fixed-point financial math | **scanned** | integer base units with per-asset decimals; the float uses that existed were defects and are fixed — a fractional amount truncated to 0, a fractional timeout truncated to blocks, and a basis-point guard compared as a percentage [round 30]. No `FixedPoint` type; the discipline is enforced by the scan above rather than by a type. |
| 44 | Fail-closed semantics | **partial** | bridge backend, private submission and the verifier paths fail closed [rounds 5–13, 28], and a fabricated VRF seed now fails closed too [round 30]; no audit that *every* error path does — an `unwrap` on a fallible path is the shape to look for and nothing scans for it. |
| 45 | Version binding | **implemented** | `validate_version` / `validate_not_weaker_than` on the economic policy; bytecode version header [now]. |
| 46 | Artifact hashing | **implemented** | SHA-256 over the emitted bytecode, algorithm named in the metadata document [round 29]. |
| 47 | Source provenance | **partial** | `EventProvenance` exists [now]; end-to-end source→bytecode→host→receipt provenance is not demonstrated. |
| 48 | **Adversarial test matrix** | **partial** | conformance accept/reject fixtures exist; there is no named matrix covering the spec's listed attacks. |
| 49 | Fuzzing and property tests | **partial** | five files use `proptest!` [now]; no fuzzing of the parser or bytecode reader. |
| 50 | **Performance targets** | **absent** | no budgets or measurements [now]. |
| 51 | GPU acceleration | **partial** | `GPU_DISPATCH` opcode and `gpu` handling exist [now]; there is no GPU execution in this workspace (the GPU crates live elsewhere in the repo). |
| 52 | Compiler diagnostics | **partial** | `DiagnosticCode` covers parser, numeric and IR layers [now]; the trading path has none (TICKET-021). |
| 53 | X3Lang tooling | **implemented** | `x3c` carries 29 subcommands, including the ones this session added [now]. |
| 54 | Simulation | **implemented** | `simulate` op and `x3c simulate` / `run` on the dry-run VM [now]. |
| 55 | Explainability | **partial** | `x3c explain` disassembles with decoded records [rounds 26, 28]; there is no explanation of *why* a plan was chosen beyond the optimizer's tie report. |
| 56 | Build order | **implemented** | this audit's subject: all 25 items [rounds 17–29]. |

## GAPS, in the order I would take them

- **GAPS-1 (phase 2)** — use-before-borrow, phantom collateral and debt-reuse-after-closure have no negative test. They may be impossible by construction (a `borrow` binds a debt before any `repay` can name it), which is a *design* guarantee worth stating rather than a test worth writing — but nobody has stated it.
- **GAPS-2 (phase 5)** — *closed in round 31*: profit is a value with parts, both floor checks read its signed `net`, and the decomposition is reconciled against the recorded delta. The wiring also exposed two defects: the policy-level floor took the maximum delta across every asset, and the floor had no asset of its own.
- **GAPS-3 (phase 15)** — the spec's end-state syntax `objective { maximize net_profit; … }` has no implementation.
- **GAPS-4 (phases 29, 30)** — opportunity packets and execution lanes are absent, and both are named in the build order's neighbourhood.
- **GAPS-5 (phases 38, 39, 50)** — hyperarb, CEX venues and performance targets are absent.
- **GAPS-6 (phase 42/43/44)** — *two thirds closed in round 30*: determinism and fixed-point now have a scan, and it found four real violations (a clock-derived VRF seed in the production bridge adapter, a fractional amount truncated to an integer, a fractional timeout truncated to blocks, and a basis-point slippage guard compared as a percentage — which had been rejecting every mainnet program with a guard above 5 bps). What remains unscanned: **unordered-map iteration** (membership use is fine, so a textual scan cannot decide it) and **`unwrap` on fallible paths** (phase 44).

## What this audit does not prove

It proves the phases marked **absent** have no artifact by the probes named, and
that the phases marked **implemented** have one. It does not prove the
`unaudited` phases are anything, and it does not prove the **partial** ones are
wrong — only that the specific behaviour named in the evidence column is present
or missing. Every row is one probe, and one probe is not a phase.
