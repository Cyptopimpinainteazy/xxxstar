# X3Lang ticket ledger — 2026-09-18

Every unresolved finding, deferred item and out-of-scope item from today's
passes, as a concrete follow-up. Evidence for the findings is in
`x3lang-intent-guards-20260918.md`.

## TICKET-001 — Make intent lowering's `Release`/refund representation unambiguous — CLOSED
Type: CLOSED in `706f0d02e` (2026-09-20), on `108785c62` · Subsystem: x3-lang/compiler/lowering
Closed: `Release` says which of its **three** acts it performs — `ReleaseAct::{Payout, Claims(u32),
Refund}`, defined in `x3-lang-common` beside the payload that encodes it and re-exported through the
IR, so the IR and the wire format cannot disagree about a tag. TICKET-101 made the first two explicit;
this is the third, which was still spelled as a payout even though a timeout refund's concrete
release is a different act from a destination endpoint's. Measured on a real program:
```
$ x3c explain timeout_refund.x3b
  0005  0x23  RELEASE  Release { chain: "solana", …, act: Payout }
  0012  0x42  ON_TIMEOUT
  0013  0x23  RELEASE  Release { chain: "ethereum", …, to: "sender", act: Refund }
```
— two records that were identical in kind now say which act each is, which is the thing that made
the builtin invariants false-positive on well-formed intents until they were scoped away from it
(TICKET-002). The acceptance's two clauses are both met: the IR distinguishes a destination-endpoint
release (`Payout`) from a source-chain claim (`Claims`), and a refund's release carries its own act
rather than being a second unlabelled `Release`.
The rules read the act: `release_lock` returns an escrow only for `Claims`, and the range check runs
only for `Claims` — so a payout is still not checked against a route it never claimed, which is what
made the first attempt at that check fail four cross-chain plans. `refund_lock` still matches the
**handler** deliberately, and says why: a refund is one act described twice (the `OnTimeout` says
what and why, the `Release` performs it), so a rule counting refunds must count one of them, not
both. New test: a destination payout and a refund are different acts, and it fails if every release
in a program is a payout — which is what the old spelling produced.
Validation as measured: x3-lang `cargo test --workspace` **1157 passed / 0 failed**; clippy and fmt
clean; pytest 21; sweep check 19/19, build 19/19, warning-free 19/19, run-artifact 18/19.

## TICKET-002 — Make the builtin invariants sound — CLOSED
Type: FIXED in `126b606fe` · Subsystem: x3-lang/compiler/semantic
Closed by scoping the four positional rules to atomic route bodies
(`atomic_scoped_operations`), and skipping `destination_fill_before_source_claim`
when the route contains no bridge. Zero invariant warnings now on
`internal_swap.x3`, `mainnet_safe_swap.x3`, `flagship_b52.x3` and
`timeout_refund.x3`, down from 4, 1, 1 and 4. Five tests prove the rules still
catch two claims in one route, a refund after a claim in one route, a claim
before the route's bridge, and two refund handlers.

## TICKET-002b — Give the route-scoped invariant rules teeth for intents — CLOSED
Type: CLOSED in `8386a4ec5` (2026-09-20) · Subsystem: x3-lang/compiler/semantic
Closed: **all six rules are errors**, and the promotion found two false positives the warnings had
hidden — which is TICKET-002b's own argument made by the change: a warning that cannot be promoted is a
rule that protects nothing.
1. **`no_double_refund` counted one refund per lock across both exit paths.** The canonical bridged
   `parallel` leg writes `timeout 30s refund ethereum.ETH to sender` beside `on_fail refund ethereum.ETH
   to sender`: the timeout refund is a declaration for the timeout/refund engine (the emitter writes
   `[ON_TIMEOUT][0]` and leaves `duration_blocks` to the engine) and the on-fail refund is the failure
   path's instruction, so a run leaves through one of them. The count is per `(lock, exit path)` now —
   two `OnFail` refunds of one lock still violate, two `OnTimeout` declarations for one lock still
   violate, one of each does not.
2. **`no_claim_after_refund` read any `Release` as a claim.** `Release` is a claim or a payout
   (TICKET-101's `ReleaseAct`), and the sibling rule `no_refund_after_claim` already read
   `release_lock`; this arm kept the broad reading, so the same leg (bridge, refund handlers, destination
   payout) was called a claim-after-refund.
Both existing tests were updated rather than deleted and each keeps **both** halves (two refunds on one
exit path are reported while the pair is not; a claim after a refund is reported while a payout after one
is not), and the in-module claim-after-refund fixture moved from `ReleaseAct::Payout` to `Claims(0)` — it
had been asserting the rule on a shape the rule is not about, which is why it passed while the rule was
wrong.
The acceptance's negative-test clause is `compiler/tests/test_invariant_rules.rs`: for each of the six
rules, an input that violates **that** rule and leaves the other five silent (asserted as a matrix), a
second test that the case set names every rule exactly once, and a third that a well-formed bridging
route trips none. The stale note ("must not be promoted to errors yet") is replaced by what licenses the
promotion: the scoping and the measurement below.
Validation as measured: **every `.x3` outside the skipped directories checks with zero invariant
findings** (25 files, 0 failing); sweep unchanged 19/19/19/18; x3-lang `cargo test --workspace` **1216
passed / 0 failed** (1213 before); clippy `-D warnings` and fmt clean.
Original entry:
Type: FIXABLE_NOW · Subsystem: x3-lang/compiler/lowering
Reason: F1. As written they fire on every well-formed intent, so they cannot be
errors; as warnings they are invisible (F4).
Acceptance criteria: each of the six rules is either corrected to the real IR
shape or scoped to programs where it is meaningful; every rule then has a
negative test that fails only for that rule; all are errors.
Validation: `cargo test -p x3-lang-compiler` plus a conformance case per rule.
Depends on: TICKET-001.

## TICKET-003 — Proof-requirement guard: canonical vocabulary — CLOSED
Type: FIXED in `c5a7ab1e0` · Subsystem: x3-lang/compiler/semantic
The check demanded `lock_proof` / `fill_proof` / `claim_proof`; those exact
names appear in **zero** `.x3` files, so no correct program could satisfy it.
It now classifies by category (source-lock, destination-fill) with substring
matching, which accepts the vocabulary the repo actually uses. `claim_proof` is
dropped rather than renamed — see TICKET-018.

Still open from this ticket: the guard is a **warning**, not an error. With the
vocabulary fixed the warnings are now accurate (programs that declare nothing
are told), so promoting it becomes a question of policy for dev-mode code
rather than of correctness. Filed as TICKET-020.

## TICKET-019 — An intent clause the parser did not recognise was silently ignored
Type: FIXED in `c5a7ab1e0` · Subsystem: x3-lang/compiler/parser
`parse_intent_clause`'s fallback parsed anything unrecognised as a generic
expression statement, which was then dropped. `compiler/tests/fixtures/intent_with_proofs.x3`
wrote `proofs required { ... }` inside its intent body and produced **no**
`ProofRequired` operations — the declaration was accepted and never applied.
The fallback now rejects any clause outside the documented set, with a
diagnostic for the nested `proofs` case naming file scope as the correct place.

## TICKET-018 — What must a release actually prove?
Type: DEFERRED (design decision) · Subsystem: x3-lang
Reason: the old `claim_proof` requirement was dropped because nothing declares
it and the destination-fill proof is what authorises a release. Whether a
release should also require, say, a receipt or a validator-quorum attestation,
and under what name, is a design decision that should not be guessed at.
Acceptance criteria: the release path's proof obligation is stated, named, and
enforced, with a negative test.
Validation: `cargo test -p x3-lang-compiler` plus a corpus program that declares
the release proof.

## TICKET-020 — Should the proof requirement be an error rather than a warning?
Type: CLOSED in `1c665b5b1` (2026-09-19) · Subsystem: x3-lang/compiler/semantic
Closed: decided by mode, which is where every other rule of this shape lives
(`verify_mainnet_safe` runs single-RPC, single-relayer, refund-path, finality and
solver-bond as errors for mainnet only). On mainnet a bridging program that
declares no source-lock proof (or no destination-fill proof) is now refused with a
`mainnet:` message naming the clause to add; in dev it stays the warning it was.
Measured first: no corpus example is affected in either mode — every bridging
example already declares both proofs — so the promotion costs the corpus nothing
and `--deny-warnings` stays clean. Tests: a unit test asserting one program is an
error on mainnet and a warning in dev (and not both), and a CLI test running the
same source in both modes.
Original entry:

Reason: with TICKET-003 closed the warnings are accurate, but `--deny-warnings`
is still opt-in and the guard does not fail a build. Six corpus programs lock
and bridge without declaring any proof; they would need declarations first.
Acceptance criteria: decide, then either promote the guard or keep it a warning
and say why in the docs. Do not promote it while any repo program would fail.
Validation: `x3c --deny-warnings check` over `examples/**/*.x3` and
`tests/**/*.x3` passes, with the fixtures updated if promotion is chosen.

## TICKET-021 — Extend the coded-diagnostic catalogue to the trading path — CLOSED
Type: CLOSED in `551e32b23` (2026-09-20) · Subsystem: x3-lang/compiler/diagnostic + semantic + objective
Closed: the last two uncoded constructors are gone. `semantic.rs`'s `fn err(message)` (115 call sites —
the AST-level verifiers `ast_level_errors` calls and the IR-level ones beside them) and `objective.rs`'s
(10 sites) **require a code** now: a parameter rather than a helper per class, because the class is a
fact about each site, and a helper that chose one code for the whole file would be the uncoded helper
under a new name. The signature is the gate — a site cannot be added without a class.
The classification, per site, read rather than guessed: `X3E4027 GuardClaimUnbacked` (30, the recurring
"a guard whose claim no declaration backs" shape and *new* with this commit), `X3E4025 TradeDeclaration`
(41), `X3E4028 MainnetConfigurationUnsafe` (13, *new*), `X3E0501 UnsafeIr` (24), `X3E4026
DeclarationHasNoArtifactForm` (1), `X3E0401 InvalidCrossChainRoute` (4) and `X3E0101 UndefinedSymbol`
(2) for `validate_atomic_swap`'s unknown chains/hash and `check_safe_symbol`'s empty name. Two codes are
new and both have sites, which is the rule the earlier rounds set: a code nothing emits is a dead
catalogue entry, and a class with thirty sites is not a stretch of an existing name.
**The other clause — severity as the accumulator's channel — is answered by TICKET-104** rather than by
a field on every error: `VerifyOutcome::push_diagnostic` chooses the vector from the diagnostic's own
severity. What remains a channel is the vector *type* (`Vec<X3Error>`), which the ticket's acceptance
allows.
Assertions, one per class, on the fixtures their own test files already had; the new
`a_guard_whose_declaration_is_missing_carries_its_own_class` is **proven load-bearing by mutation** —
flipping that one site's class to `TradeDeclaration` makes it fail, restoring passes.
Two tooling catches worth recording: **clippy caught a doc-comment quote marker** this change
introduced (a line beginning `>= N` reads as a Markdown blockquote), and **rustfmt refused to format a
file the insertion had left trailing whitespace in** — the same trap the previous rounds hit, now hit
twice, which is why the strip is part of the script rather than an afterthought.
Validation as measured: x3-lang `cargo test --workspace` **1207 passed / 0 failed** (1206 before);
clippy `-D warnings` clean; fmt clean; sweep 19/19/19/18.
Original entry:
Type: PARTIAL in `f25be0d3b` + `b2da8b14c` (2026-09-20); `trading_verify.rs` is fully coded
and two other modules are not · Subsystem: x3-lang/compiler/diagnostic
**Progress in `b2da8b14c`.** `trading_verify.rs` had one `semantic_error(message, span)` helper and
**18 call sites**, every one a bare `X3Error::SemanticError`. All 18 name a class now, and the
helper is **deleted** rather than left beside the coded one — a call site that could still produce
an uncoded trading diagnostic is the defect, and leaving the constructor would let the next one
choose it:
```
X3E4022 DebtLifecycle      5 sites  borrowed twice, repaid twice, repaid with nothing open,
                                    left open where the trade can still succeed
X3E4023 TradingSequence    3 sites  a source-chain statement after bridging away, a second
                                    bridge, a bridge that does not move between chains
X3E4024 RiskPolicyBound    5 sites  a bound above the basis-point ceiling, a zero deadline,
                                    private submission with no capability attested
X3E4025 TradeDeclaration   6 sites  a policy it does not declare, an invariant declared twice,
                                    a borrow with no all_debts_repaid, no receipt, no guard
X3E2107 AssetTypeMismatch  1 site   the existing code, for the asset reference that mixes chains
```
Two names are the ticket's own (`debt`, `sequence`). Its other two — `venue` and `quote` — have
**no site in this module**, and a code nothing emits is a catalogue entry with no diagnostic behind
it, so they are not added: the capability a venue needs is `RiskPolicyBound` here, and quote
freshness is checked in the VM rather than the compiler. Four existing tests gained one code
assertion each, on the fixtures they already had; each was proven load-bearing by making the helper
drop the code again (all four fail, all four pass restored).
**Progress in `4d1a0921d`/`4d1a22df0`.** The two modules the paragraph below named are done:
`trading_semantic.rs` (**22 sites**) and `strategy.rs` (**22 sites**), both helpers deleted —
clippy's dead-code lint caught the second one surviving, which is the point of deleting rather
than leaving it beside the new constructor. **No new codes were needed**: the four classes added
for `trading_verify.rs` cover these sites too, which is evidence they were the right classes
rather than names invented per module. `RiskPolicyBound` gained the licence-share and slippage
ceilings, `TradeDeclaration` the module-completeness checks (input, output, domains, risk
profile, max_gas, split totals, permissions a body exceeds), `DebtLifecycle` the two debt sites,
`AssetTypeMismatch` the `min_out`-in-the-wrong-asset and binding-type sites. Two assertions were
added, one per module, on the fixtures their tests already had, and each was proven load-bearing
by making its helper drop the code again.
**Progress in `7a8fdc261`.** Three more modules off the same worklist — `hedge.rs` (3 sites,
`RiskPolicyBound`), `liquidation.rs` (1, `TradeDeclaration`), `rebalance.rs` (1, `TradeDeclaration`),
each helper deleted. Third module in a row where the existing classes fitted without a new name.
These helpers take **two** arguments rather than three (their diagnostics carry `Span::DUMMY`,
because they are decided from a declaration's own numbers rather than a source position) — the
arity is a fact about the module, and reusing `trading_verify.rs`'s three-argument helper was the
wrong move, which the compiler said on five sites at once.
**Remaining, measured rather than estimated.** `objective.rs` (~10 sites) and `semantic.rs`'s
`fn err` (the ~17 AST-level verifiers that `ast_level_errors` calls), plus **severity is still the
accumulator's channel**:
`ast_level_errors` returns `Vec<X3Error>` and `PreEmission` carries `errors` and `warnings` as two
vectors rather than one list with a `severity` field, which is why the four codes above are
`DiagnosticCode`s rendered through `CompilerDiagnostic::into_error` rather than a severity-carrying
list. **The bridge for the rest exists**: `x3_lang_common::diagnostic`'s accumulator accepts an
`X3Error` and converts it with `Diagnostic::from`, so the migration is incremental — a module at a
time, merging at `ast_level_errors`.
Original progress:
Type: PARTIAL in `f25be0d3b` (2026-09-19); the economic classes the ticket names are coded · Subsystem: x3-lang/compiler/diagnostic
Progress: two classes carry stable codes now, in the four-digit spelling the
catalogue already uses and with the spec's own numbers:
`X3E2107 ASSET_TYPE_MISMATCH` (the swap and bridge sites in
`trading_semantic.rs`) and `X3E4021 UNRESOLVED_ECONOMIC_EFFECT` (the `atomic
trade` checks in `trading_verify.rs` and the strategy-module checks in
`strategy.rs`). `CompilerDiagnostic::into_error` is the one renderer of
"code: message", and the IR verifier's conversion goes through it too, so a second
spelling cannot drift. Tests: the trading effects check asserts its code on the
error it already asserted the wording of; the asset-mismatch check — unreachable
from source text, because the parser sets a swap's `from_asset` from the amount's
own asset, but reachable from any other front end — has a hand-built AST test.
Remaining: the other trading diagnostics (sequence, debt, venue, quote) carry no
code, and severity is still the accumulator's channel (errors vs warnings) rather
than a field on the diagnostic. Both are what keeps this open.
Original entry:

Reason: `DiagnosticCode` (X3E0001…X3E0501) covers the parser, numeric and IR
passes, but every trading and intent diagnostic is a bare
`X3Error::SemanticError { message, span }` with no machine-readable code. The
super-prompt's `X3E-4021` "unresolved economic effect" is the shape the new
effects/guarantees check should carry, and audit tooling cannot key on wording.
Acceptance criteria: trading and intent diagnostics carry stable codes with the
message, severity and span; the effects/guarantees check has its own code.
Validation: `cargo test -p x3-lang-compiler -p x3-lang-vm`, plus a test asserting
the code is present for at least one trading diagnostic.

## TICKET-022 — `principal_preserved` and the remaining roadmap guarantees
Type: DEFERRED (needs IR meaning first) · Subsystem: x3-lang
Reason: round 6 admitted only guarantees the compiler can discharge from the
body (`debt_closed`, `min_profit`, `solvent`). `principal_preserved`, hedge and
liquidation guarantees from the roadmap have no IR-level meaning to check
against, and admitting the names would recreate the "label that means nothing"
defect.
Acceptance criteria: each newly admitted guarantee has a defined IR-level
discharge, a verifier check, and a negative test.
Validation: `cargo test -p x3-lang-compiler`.

## TICKET-023 — The writer and the VM disagree about metadata alignment — CLOSED
Type: FIXED in `6756bb994` + `848eff8d8` · Subsystem: x3-lang/vm + compiler/emitter
Resolved by identifying the outlier correctly: `emit_x3ir` writes the metadata
block unpadded, the executor's `first_instruction_pc` walks it unpadded, and the
disassembler now mirrors the writer — so `vm/src/verifier.rs` was the one that
was wrong, not the writer. Padding the writer (attempted in round 7 and reverted)
broke `evm_call_pipeline` and `x3_call_pipeline`, which is what identified it.

The same investigation found that the verifier rejected **every** trading-core
program: `valid_opcode` stopped at `0xAB` while trading opcodes are
`0xB0..=0xBA`, its `is_payload_opcode` omitted the same range, and
`validate_payload_opcode` fell through to the capability decoder, which cannot
read trading payloads at all. The executor's dispatch range had the mirror-image
gap, stopping one short of `TRADING_BRIDGE`. All four now use the spec range.

Measured: `examples/trading_core_v1.x3` and `examples/trading_effects.x3` go
from `X3_VERIFY_FAILED: InvalidOpcode(176, 1)` to `x3c run: ok`. Corpus
execution over `examples/*.x3`: 7 clean, up from 5.

## TICKET-025 — The flagship cross-chain examples fail at runtime — PARTIAL
Type: FIXED IN PART in `5ff7c9ecf`; remainder is TICKET-027 · Subsystem: x3-lang
Reason: `examples/mainnet_safe_swap.x3` and `examples/flagship_b52.x3` build and
verify, then panic at execution:

```
$ x3c run <mainnet_safe_swap.x3b>
x3c: error: VM error: Panic("solver bid: fee must be non-empty")
```

These are the two examples the repo uses to demonstrate a *mainnet-safe*
cross-chain swap, so a hard runtime panic in both is a real defect, not a
cosmetic one. The failure is in the solver-bid host call: the lowered program
carries a `solver_bid` requirement whose fee field is empty.
Two of the three causes are fixed — the fabricated `SolverBid` (a marketplace
configuration was lowered into an executable bid with `bond: min_reputation` and
an empty fee) and the unsatisfiable `RelayerAttest` signature demand (a
declaration cannot carry settlement-time signatures). The examples now get past
both. The remaining cause is TICKET-027, and it is the real one.

## TICKET-029 — Two of three documented verification layers were never called
Type: FIXED in `1c8c35d59` · Subsystem: x3-lang/compiler
`lib.rs` documents three layers: the numeric policy over the AST, structural IR
invariants before emission, and the semantic verifier. Only the semantic verifier
was reachable. `verify_numeric_policy` and `verify_ir` had **no call sites
anywhere in the crate except their own tests**, so the integer-literal and
coercion policy, and the structural invariants (atomic scoping balance, non-zero
amounts and iterations, empty-field safety), were applied to no program that was
ever compiled. Both are wired now — layer 1 into `ast_level_errors`, layer 2
after lowering on both entry points.

Wiring them found two real defects immediately, which is the argument for wiring
dead checks: `examples/multi_leg_route.x3` and `examples/flagship_b52.x3` each
contain a swap with no `min_output`, lowering to an output floor of zero.

## TICKET-030 — Audit the passes for single-path reachability — CLOSED
Type: FIXED in `57202d463` · Subsystem: x3-lang/compiler
Two things. First, the layers are now wired once: `run_pre_emission_layers` is
the only place that runs them, and every entry point goes through it —
`compile_program_with_context` and `compile_program_with_regalloc` had run no AST
or IR checks at all. "Every entry point runs the same checks" is true by
construction rather than something to re-audit.

Second, the audit is a test: every `pub fn verify_*` / `analyze_*` in the crate
must be referenced outside its own definition, ignoring comments. It found two
more orphans on its first run:

- `semantic::verify_with_defaults` — documented as the entry production callers
  should use, called by nobody. `check_ir` now uses it.
- `semantic::analyze_invariants` — a stringly-typed duplicate of the same six
  invariant names `get_builtin_invariants` implements with real check functions,
  called by nobody. Deleted rather than left as a second, weaker source of truth.

## TICKET-031 — The semantic pass list is data — CLOSED
Type: FIXED in `9ee945db0` · Subsystem: x3-lang/compiler/semantic
The fifteen passes are a table of names and variants that `verify_collect`
iterates, dispatched through an exhaustive match — an enum rather than function
pointers, since the passes do not share a signature and the enum makes the table
and the dispatch mutually exhaustive by the compiler. A test compares the table
against the `fn verify_*` definitions in the module, reading references across
the whole crate because entry points and AST-level passes are called from
`lib.rs`. A definition that is neither registered nor called anywhere fails.
The same audit shape should be applied to the VM, which has the same class of
drift in its reader/executor/verifier trio (see TICKET-026).
Reason: TICKET-029 was found by inspection, not by anything systematic. The shape
is always the same — a validator that exists, is documented or tested, and is
reachable from only one of the entry points, or from none. Five separate findings
in this session have had that shape. A check on the `check` path but not the
`build` path, or in a module with no caller, is worth finding before it is found
by a defect.
Acceptance criteria: every `pub fn verify_*` in the compiler is either reachable
from both `compile_with_mode` and `check_source_diagnostics_with_mode`, or is
documented as callable-by-design from outside (and has a test that calls it).
Validation: a test or script that enumerates the passes and asserts each is
reachable from a real entry point.

## TICKET-028 — Fixed-frame operands were read as payload lengths — CLOSED
Type: FIXED in `bd0c755b6` · Subsystem: x3-lang/compiler/emitter (disassemble)
The premise was wrong and the investigation corrected it: the listing does
account for every instruction (`IR operations + header records`), confirmed by a
test over the flagship example (27 + 1 nonce = 28). What was real is a latent
defect: `REQUIRE`, `ON_FAIL`, `ON_TIMEOUT` and the three atomic opcodes are fixed
four-byte frames, but the walker's `is_payload_opcode` listed all six as payload
opcodes, so it read their operand bytes as a length. Invisible while the operand
is zero, and it truncates the listing the moment it is not — which is exactly
what my guard-comparison experiment triggered, and why a hand-written
cross-check agreed: it classified the opcodes the same wrong way.
Two tests now pin it: a four-byte `REQUIRE` with a non-zero operand must not hide
the instruction after it, and emit-then-disassemble must account for every
operation plus the header records.
Reason: for `examples/mainnet_safe_swap.x3` the bytecode plainly contains eight
`REQUIRE` frames plus `ON_TIMEOUT` and `ON_FAIL` (hexdump of the tail:
`4000 0000 4000 0000 4001 5a00 4000 0000 4000 0000 4001 0400 4000 0000 4200
0000 4100 0000`), while `x3c explain` reports 4 `REQUIRE` ops and 22
instructions in total. Either the disassembler stops early or its walk
desynchronises somewhere before the tail. Stated as an observation rather than a
conclusion: my first two attempts to count these instructions disagreed with each
other, so the next step is to instrument the walk rather than trust another
hand-rolled count.
Acceptance criteria: the instruction count from `disassemble` equals the number
of instructions the executor walks for the same stream, on every example.
Validation: compare both counts programmatically over `examples/*.x3`.

## TICKET-027 — Guards are not evaluated — PARTIAL
Type: FIXED IN PART in `c1306b929` and `f2b972a3c` · Subsystem: x3-lang/compiler + vm
Reason: `Operation::Require` is emitted as `[REQUIRE][u16 0]` — no operand, no
condition, no identity. The executor's `REQUIRE` arm therefore tests **r0**, and
nothing sets r0 *for the guard*: it holds whatever the previous operation left
there. Measured on `examples/mainnet_safe_swap.x3`, the sequence is

```
idx  7 pc 264 PROOF_REQUIRED
idx  8 pc 296 PROOF_REQUIRED
idx  9 pc 332 PROOF_REQUIRED
idx 10 pc 368 REQUIRE          -> X3_REQUIRE_FAILED: condition register r0 is zero at pc 368
```

and the mechanism is visible in the executor: the bulk capability arm ends with
`vm.state.registers[0] = bytes_to_register(&result)`, while `ProofRequired`
returns `Ok(vec![])` — so r0 is set to zero by an unrelated op's empty result
and the next `require` fails.

So every guard in the language (`slippage <= 5`, `finality >= 32`,
`relayer_quorum >= 3`, `solver_bond >= 10000`, `nonce unused`) is a coin flip on
register residue rather than a check. The compile-time verifier genuinely checks
that guards are *declared*; nothing evaluates their conditions.

Fixed: the coin flip is gone. `REQUIRE` now carries
`[comparison][threshold u16]` in the flags byte and operand it already had, and
the executor evaluates `REQUIRE_COMPARE_STATIC` as a compile-time assertion
having nothing to test, `REQUIRE_COMPARE_GE` against `r0`, and any unimplemented
comparison code as a failure. `RelayerAttest` leaves the quorum numerator in
`r0` rather than the signature count, and the bulk capability arm no longer
overwrites `r0` with the register encoding of an empty result. Both flagship
examples now run.

`f2b972a3c` closed the *unbacked* half for the guard named here.
`require solver_bond >= N` now has a declaration to compare against —
`solver_market { bond <amount> <ASSET> }` — and an AST-level pass rejects a guard
with no declaration and a requirement above the declared bond. Both flagship
examples declare `bond 10_000 USDC`, which is what their guards require.

**`relayer_quorum` CLOSED in `051c048c7`.** `require relayer_quorum >= N` is now
checked against `relayers { quorum N_of_M }`: no swarm → error, `N` above the
declared numerator → error, otherwise fine. While mirroring the solver-bond
check it turned out that one walked only `Item::IntentDecl` body statements, so
a guard sitting in the `requires` list of `bridge`, `atomic swap`, `strategy` or
`proposal` escaped it entirely; both checks now share one `require_guards`
enumeration and a test exercises the atomic-swap location.

Remaining: **the other guard kinds**, and the run-time comparison. Every guard in
the language is a claim about the program's configuration, so the pattern that
worked for the solver bond generalises.
`require route_score >= N` and `require risk < N` need a declared quantity that
does not exist yet.

And the run-time half remains: **nothing emits
`REQUIRE_COMPARE_GE`, because no instruction puts a guard's quantity in `r0`.**
Declarations sit at the top of a program and guards at the bottom, so `r0` at a
guard still holds an unrelated value — comparing it would be a new wrong answer,
which is why the emitter deliberately emits `STATIC` for every guard today. Two
things have to happen: the lowering must carry the guarded quantity to the
guard, and the compile-time verifier must actually evaluate the static guards
instead of only checking that they are declared (`require solver_bond >= 10000`
currently has no bond to compare against at all, in either place).
Acceptance criteria: each guard kind either has a real runtime quantity at its
guard and emits a comparison, or is evaluated by the compiler; none may be an
unbacked `STATIC` assertion.
Validation: a program whose guard should fail must fail at that guard, one whose
guard should pass must pass, and every guard kind has a test for both.

## TICKET-026 — the verify-before-execute path was bypassable by name — CLOSED
Type: CLOSED in `520e53557` (2026-09-18) · Subsystem: x3-lang/vm

**The premise was wrong, and the correction is the finding.** `rg X3LangVm`
returned nothing because no such type exists — the ticket named the module path
(`x3_lang_vm`) as if it were a type. The module defines `VM`, `VMConfig` and
`InstructionStream`, and those are used throughout the workspace, including by
`x3c run` and `x3_lang_vm::executor` itself. `VM::execute` even calls
`verify_and_execute`, so verification does precede execution.

What *was* true: `VM::execute` → `verify_and_execute` → `execute_unverified` →
`x3_lang_vm::executor::execute`, and that last function was `pub`. So the
verification guarantee was a matter of picking the right name — any integrator
could import the executor directly and run bytecode that was never checked.
`execute_unverified` is deliberately `pub(crate)`, which shows the crate meant
that door to be shut; it just left an identically-named one open.

Fixed: `executor::execute` is `pub(crate)`. Its three callers outside the crate
(`compiler/tests/test_control_flow_e2e.rs`) now use `VM::execute`, so those
end-to-end tests verify their bytecode too — they did not before, and they pass.
The guarantee also had no negative case, so
`vm_execute_verifies_before_it_runs_anything` feeds the VM an opcode the
verifier does not know and asserts the refusal is `X3_VERIFY_FAILED` with gas
and pc untouched.

Lesson for the naming: a ticket that names a symbol which does not exist is
evidence the ticket was written from a path, not from a call-site search. The
`rg` in the ticket should have been `rg 'x3_lang_vm::|VM::'`.
Reason: `pad_to_4`'s own doc says *"the X3 VM verifier requires every instruction
to start on a 4-byte boundary"*, and the VM's `first_instruction_pc` /
`skip_compiler_metadata` align the metadata block. `emit_x3ir` does **not** pad
it: `[version][META_NONCE][u16 len][nonce]` is `3 + len + 1` bytes, so a 15-byte
nonce leaves the first instruction at offset 19 rather than 20.

Measured: padding the writer makes the disassembler agree (verified — the
`PROOF_REQUIRED`/`LOCK`/`BRIDGE` stream decodes cleanly), but it **breaks
`evm_call_pipeline` and `x3_call_pipeline`**, which pass today with the
unpadded layout. So the writer, the VM reader and the alignment invariant have
to be settled together, with the bytecode fixtures updated in the same change.
Half a fix here trades one reader for another.

Evidence also collected: `x3c run` on a proof-declaring program whose
`proofs required` block comes first fails with `InvalidOpcode`, while
`examples/mainnet_safe_swap.x3` — proofs mid-stream — executes past them fine
and only fails later on a business rule (`solver bid: fee must be non-empty`).
Something about the first instruction after the header is still uncharacterised.

Acceptance criteria: one authoritative header layout, documented; writer and
every reader agree; a test asserts the first instruction's offset for nonce
lengths that are and are not congruent to 0 mod 4; all fixtures re-verified.
Validation: `cargo test --workspace` plus `x3c explain`/`x3c run` on
nonce-bearing programs with and without metadata.

## TICKET-024 — Gate the proof requirement on bridging, then decide warn-vs-error
Type: CLOSED in `1c665b5b1` (2026-09-19) · Subsystem: x3-lang/compiler/semantic
Closed: both halves. The gate is `has_bridge` (an earlier round), so a same-chain
intent is no longer told to prove a cross-chain lock; and the warn-vs-error
decision is TICKET-020's, made above. `x3c check --deny-warnings` over the corpus
reports one warning, `atomic_swap.x3`'s `no_refund_after_claim` (TICKET-035), which
is a different check and still open.
Original entry:

Reason: the check is over-broad — it triggers on `Lock`, and intent lowering
emits the `from` endpoint as a `Lock`, so a same-chain intent is told it needs a
source-lock proof for a transfer that never crosses a chain (measured:
`internal_transfer`, one warning, nothing locked). The gate should be
`has_bridge`, and a bridging program should then be required to declare both a
source-lock and a destination-fill proof. The gate fix was written and verified,
but promoting to an error widened `PROOF_REQUIRED` exposure across six corpus
programs and ran into TICKET-023, so it was held back rather than shipped on top
of an unsettled header format. **That blocker is gone:** the header layout is
settled (writer, executor, verifier and disassembler all agree it is unpadded)
and trading-core programs verify and run, so this can be attempted again.
Acceptance criteria: gate on bridging; decide warn-vs-error with the six corpus
programs declaring their proofs; `x3c --deny-warnings check` clean over the
corpus.
Validation: conformance accept/reject cases plus the corpus sweep.

## TICKET-004 — Four fabricated compiled-policy fields: enforce or delete — CLOSED
Type: FIXED · Subsystem: x3-lang/compiler + x3-lang/vm/economic
Reason: F3.
Acceptance criteria: none of `max_total_cost`, `max_price_impact_bps`,
`max_mev_leakage_bps`, `quote_freshness_blocks` remains both unsettable and
unenforced; `EconomicPolicy::validate_not_weaker_than` compares only
source-settable values.
Validation: `cargo test -p x3-lang-compiler -p x3-lang-vm`.
Owner: `policy_honesty` agent (in flight).

## TICKET-005 — Surface verifier warnings instead of discarding them — CLOSED
Type: FIXED · Subsystem: x3-lang/compiler (API) + x3c
Closed in `841cdeb60`. `VerifyOutcome` + `verify_collect` keep errors and
warnings separate; `check_source_diagnostics[_with_mode]` expose them; `x3c
check` reports warnings and gained `--deny-warnings`. The builtin-invariant
false positives are now visible (7 warnings on a well-formed intent), which is
the forcing function for TICKET-002.
Reason: F4. `verify_with_config` drops every warning, so any safety check
written as a warning is dead code. This is the root cause of the finality bug.
Acceptance criteria: `check_source` and `x3c check` report collected warnings to
the caller/user rather than dropping them; a test asserts a program that
produces a warning surfaces it.
Validation: `cargo test -p x3-lang-compiler -p x3-tools`.
Note: coordinate with TICKET-003, which also touches this file.

## TICKET-006 — Adjudicate residual `x3-lang` content on stale agent branches
Type: DEFERRED · Subsystem: repo/reconciliation
Reason: six branches carry `x3-lang` content `origin/master` lacks (~180-320
lines each), all on pre-trading-core bases. Mostly superseded, but not proven
so line by line.
Acceptance criteria: for each branch, either the residual content is shown to
be present in a different form on `origin/master`, or it is landed.
Validation: per-file diff review recorded in a report.

## TICKET-007 — Land the 197 local-only commits — CLOSED
Type: FIXED · Subsystem: repo/version-control
Reason: 197 commits reachable from local branches and no origin ref; the
remote-tracking refs were refreshed with `git fetch --prune` first, so these are
genuinely missing. Largest: `finish/x3vm-live-transport` (56),
`codex/x3-economic-safety-kernel` (45), `feat/live-secret-release-firewall-20260911`
(26).
Acceptance criteria: each branch either pushed or deliberately abandoned with a
reason.
Validation: after pushing, `git rev-list --count --branches ^origin/master`
drops to the intended set.

## TICKET-008 — Move the main worktree off a 444-behind branch
Type: BLOCKED (needs user approval) · Subsystem: repo/version-control
Reason: `~/Desktop/xxxstar-main` is on `chore/reqwest-0.12-rust-highs`
(`d3db02250`), 444 commits behind `origin/master`; `master` is held by the
`/tmp/x3-mine` worktree which has uncommitted work.
Acceptance criteria: main worktree on `master`; `.git` writable for the session
(it is read-only in-sandbox, writable unsandboxed).
Validation: `git -C ~/Desktop/xxxstar-main log --oneline -1` equals
`origin/master`.

## TICKET-009 — Commit the three uncommitted WIP piles
Type: BLOCKED (needs user approval) · Subsystem: repo/version-control
Reason: `/tmp/x3-mine` (10 files, ephemeral `/tmp`), `xxxstar-chatgpt` (22
files), `pasted-text-processing` (8 files). Backed up as patches under
`.ai/wip-backups/`; nothing committed.
Acceptance criteria: each pile committed on its branch and pushed, or
explicitly dropped.
Validation: `git status --porcelain` clean in each worktree.

## TICKET-010 — `compile_source` bypassed the semantic verifier (HIGH) — CLOSED
Type: FIXED · Subsystem: x3-lang/compiler (build path)
Reason: `compiler/src/lib.rs`:

```rust
pub fn compile_source(source: &str) -> Result<Vec<u8>, X3Error> {
    let program = parse_source(source)?;
    compile_program(&program)          // no semantic verification
}
```

Every guard in `semantic.rs` is enforced only by `check_source`/`x3c check`.
The path that produces executable bytecode never runs them.

**Reproduced:** `x3c check tests/conformance/invalid/routes/same_chain_bridge.x3`
reports four safety errors including "bridge from_chain == to_chain; cross-VM
bridge must target a different chain", while
`x3c build --out /tmp/bad2.x3b tests/conformance/invalid/routes/same_chain_bridge.x3`
**emits 280 bytes / 70 ops of bytecode for that same program**.

This also means the finality and slippage guards landed today are only "hard
errors" on the check path, not on the build path — the phase ledger's claim
that finality "is now a hard error for bridge legs" is true only for `check`.

Acceptance criteria: `compile_source` (and every other bytecode-emitting entry
point) runs the semantic verifier and returns `Err` for a program that fails
any guard. No emitted bytecode for a rejected program.
Validation: `x3c build` on each `tests/conformance/invalid/**/*.x3` must fail
with a non-zero exit and no output file.

## TICKET-011 — Remove documented false proof in existing conformance cases — CLOSED
Type: CLOSED in `253cd4cd6` (2026-09-19) · Subsystem: x3-lang/compiler/tests
Closed: every `"expect": "reject"` case now states an `expect_message` (or a `code`),
the harness asserts it appears among the observed diagnostics, and a case that
states neither is refused by the harness itself. Perturbing an expectation fails
the test with the diagnosed message printed, which is the proof it is load-bearing.
It immediately caught a case of my own: `swap_without_slippage.x3` was being
rejected by the new finality-declaration check (TICKET-049) rather than by the
missing slippage bound its name and comment describe — the case was testing the
wrong thing, and nothing said so until the harness was taught to ask why.
Original entry:

Reason: before the lexer comment fix, `tests/conformance/invalid/intents/*.x3`
and `tests/e2e/*.x3` failed with a *parse* error ("expected top-level item")
because line comments were not lexed at all. The conformance harness treats any
rejection as satisfying `"expect": "reject"`, so those cases passed without the
guard under test ever running. Fixed in the lexer; the conformance harness
should additionally assert *which* diagnostic fired so a rejection can never
again be satisfied by an unrelated failure.
Validation: give each rejection case an expected diagnostic substring and assert
it appears.

## TICKET-012 — Lexer had no comment support — CLOSED
Type: CLOSED in `248b59c91` + `253cd4cd6` (2026-09-19) · Subsystem: x3-lang/crates/x3-lexer
Closed: the lexer emits `TokenKind::Comment(CommentKind)` with the comment's span
(TICKET-043), the formatter writes comments back and the corpus round-trip tests
cover documentation-heavy files, and the conformance harness now pins *which*
diagnostic each rejection case expects — the hole the comment defect hid in.
Original entry:
Type: CLOSED (lexer) / verify downstream · Subsystem: x3-lang/crates/x3-lexer
Reason: `Lexer::lex_all_with_file` had no comment handling, so `//` lexed as two
`Slash` operators and `/*` as slash-then-star. Minimal proof before the fix:
`intent a { }` parsed, `// c\nintent a { }` failed with "expected top-level
item". Six of thirteen `examples/*.x3` start with `//` and were unparseable for
this reason alone.
Implemented: line comments (newline preserved so statements cannot merge), block
comments (newlines inside still produce `Newline` tokens), and fail-closed
`Unknown` token for an unterminated block comment. 8 unit tests, all passing.
Validation done: corpus sweep re-run through `x3c check`. Two files that could
not parse at all now compile and run (`tests/e2e/bridge_step.x3`,
`tests/e2e/simple_transfer.x3`), and the two intent rejection cases now fail for
their guard instead of a parse error. No regressions: every file that passed
before still passes.

## TICKET-013 — Implement the `contract { ... }` declaration (parser surface) — CLOSED (not a feature)
Type: CLOSED in `bedf5efe9` (2026-09-20) — **the construct is the dialect the spec says not to build** · Subsystem: x3-lang/compiler
Closed: the five files are in `examples/legacy/` and their own README accounts for each subject
(`arb` superseded by `arb_scope.x3`; `flash` forbidden by PHASE 20; `jit_lp` covered by no phase;
`mev_smooth` present as `CostKind::MevLeakage` rather than a distribution mechanism;
`x3_coin_layer.x3` pseudocode by its own title). The ticket's premise — "the construct is part of
the intended surface" — is contradicted by the spec it cites, in two sentences:
> That is a much bigger idea than a smart-contract language.
> Your mission is not to build another Solidity-like smart-contract language.

`contract` is **not one of the 56 phases** (the spec's only occurrences of the word are prose about
smart contracts generally, plus one entry in an adversarial-case list). The form is a Solidity-shaped
top-level declaration with `const NAME: address = 0x…` constants and `fn` bodies — the dialect the
legacy README describes as "an older `contract <Name> { … fn … }` dialect … that the parser no longer
accepts". Implementing it would be implementing what the spec says the project is not, so the
FIXABLE_NOW reading does not hold. The five subjects remain documented, and the legacy README says how
to bring one back: translate the subject, not the syntax.
Original entry:
Reason: five shipped examples use a top-level `contract NAME { ... }` form that
the parser rejects with "expected top-level item":
`examples/arb.x3`, `flash.x3`, `jit_lp.x3`, `mev_smooth.x3`, `x3_coin_layer.x3`.
They also use `@annotation(...)` attributes, which the parser does support.
The construct is part of the intended surface — the user's roadmap shows
`contract Vault { deposit asset.USDC; invariant total_shares <= total_assets;
emergency_pause allowed_by governance.3_of_5 }`.
Acceptance criteria: `contract` parses into an AST item, lowers to real
operations, and the five examples above pass `x3c check`. Unsupported clauses
must be a parse/semantic error, not silently ignored.
Validation: `x3c check examples/{arb,flash,jit_lp,mev_smooth,x3_coin_layer}.x3`.

## TICKET-014 — Remaining non-parsing corpus files — CLOSED
Type: CLOSED in `bedf5efe9` (2026-09-20) · Subsystem: x3-lang
Closed, and **the gate is the deliverable**: `every_x3_file_the_tooling_walks_is_a_program`
(in `crates/x3-tools/tests/cli.rs`) walks the x3-lang tree for `.x3` and requires `x3c check` on
each — 29 files now. Nothing made that claim before: the existing gate covers `examples/*.x3` and
the harness reads *named* fixtures rather than globbing, so a file in a walked directory could be
unparseable indefinitely. Four directory names are skipped, each with a README beside its contents
saying why: `examples/legacy/`, `tests/sketches/`, `fixtures/invalid/`, plus build directories.
**Two of the ticket's four files were stale when it was written**: `examples/arb_solana_eth.x3` and
`examples/atomic_swap.x3` both check (they are two of the sweep's 19), and `atomic_swap.x3`'s "not
yet supported by the x3-lang compiler" self-label is already gone — its header now describes the
syntax and its guard. The other two are 6- and 7-line sketches in the older dialect, referenced by
nothing, and moved to `tests/sketches/` with a README and a header each.
**The gate then found three more**, which is the finding worth keeping:
`compiler/tests/fixtures/{intent_with_proofs,mainnet_safe_swap,refund_swap}.x3` are referenced by
**nothing** and did not parse — drift of two kinds, both from rules the language gained after they
were written. Syntax: `amount 1000 sender 0x1234` where the clause is `receiver`, and a two-clause
`timeout 3600s` + `on_fail refund sender …` where the language has one clause with the action in it.
Rules: a bridge with no nonce and no proof obligations, a `require route_score >= 80` with no
`risk_policy { min_route_score }`, and a `require finality.<chain>` with no `finality_policy` naming
that chain. All three are **translated rather than moved** — the subjects are still subjects, and the
legacy README's policy is "translate the subject, not the syntax" — with a comment at each added
clause naming the rule it was missing. The one fixture that is *meant* to be refused
(`trading_invalid_missing_min_out.x3`) moved to `fixtures/invalid/` so the skip is a directory that
says so rather than a filename heuristic, and its single `include_str!` reference was updated.
Validation as measured: the gate walks 29 files and all check; `examples/` remains 19/19 check and
build; x3-lang `cargo test --workspace` **1158 passed / 0 failed**; clippy and fmt clean; pytest 21;
sweep check 19/19, build 19/19, warning-free 19/19, run-artifact 18/19.
Original entry:
Reason: after the lexer fix, these still fail:
`examples/arb_solana_eth.x3` (parses under the Python MVP, i.e. it is the other
dialect — decide and label it), `examples/atomic_swap.x3` (self-labelled
"not yet supported by the x3-lang compiler"; the `atomic swap` form is a
documented future feature), `tests/e2e/atomic_swap.x3` (`expected ')'`),
`tests/test_vector_ops.x3` (`expected ';' after let`).
Acceptance criteria: each file either compiles, or is moved under a clearly
named non-language directory (e.g. `examples/pseudo/`), or carries a header
stating the dialect/feature it needs. No file in `examples/` should be
silently unparseable.
Validation: `x3c check` over `examples/**/*.x3` reports only intentional
rejections.

## TICKET-015 — `compiler/src/fusion.rs` is an empty stub
Type: CLOSED in `52c3173ba` (2026-09-19) · Subsystem: x3-lang/compiler
Closed: the module is implemented (`compiler/src/fusion.rs`, 371 lines: ring
search bounded by `MAX_RING_LENGTH`, the five PHASE 21 checks, `Unverifiable` as a
distinct answer from `Satisfied`, canonical ring presentation) and wired to
`x3c fusion`. What was missing was the *test on that path*: every fusion test
called `fusion::rings` directly, so the command could have printed anything and
the workspace would have stayed green — which is why this ticket read as open. Two
CLI tests now run the binary (`crates/x3-tools/tests/cli.rs`): a ring that closes
and a ring whose participants did not opt in.
Original entry:
Reason: `compiler/src/fusion.rs` is 2 lines — a module doc comment for
"intent fusion: deterministic netting" (super-prompt phase 21) with no items.
It is registered in `lib.rs` as `pub mod fusion;`, so the crate advertises a
module that does nothing. AGENTS.md forbids placeholder modules.
Acceptance criteria: either implement the netting engine with tests, or remove
the module until it is real.
Validation: `cargo check -p x3-lang-compiler` plus the module's own tests.

## TICKET-016 — Re-add price-impact and MEV-leakage ceilings as real features
Type: DEFERRED (needs host evidence first) · Subsystem: x3-lang/compiler + vm
Reason: `max_price_impact_bps` and `max_mev_leakage_bps` were deleted in
round 3 rather than enforced, because no host evidence exists for either figure.
Re-adding them as ceilings without evidence would repeat the fabrication that
was just removed.
Acceptance criteria: `SwapResult` (or the quote path) carries a real
price-impact figure and an MEV-leakage figure with a documented unit; each
ceiling gets a source key, is enforced at execution and checked at receipt
replay, and has a negative test that fails only for that ceiling.
Validation: `cargo test -p x3-lang-vm` plus `x3c lower` showing the fields in
IR only when declared.

## TICKET-017 — `quote_freshness` is now checked at receipt replay — CLOSED
Type: CLOSED in `ec83c8503` (2026-09-19) · Subsystem: x3-lang/vm/trading
Closed: `TradeReceipt` carries `legs: Vec<LegQuoteWindow>` (one per executed swap,
in operation order: `venue`, `quote_block`, `executed_at_block`), the trading VM
records the window where the leg is accounted, and `build_receipt` copies it from
the state it was already handed — no signature change. `verify_receipt_economics`
matches the windows to the `ExecuteSwap` operations in order and refuses a leg
priced outside the compiled ceiling, a dropped window, and a window no leg belongs
to. `format_version` is 2. Five tests, including a forgery that is re-hashed and
fails on the economics alone; with the check disabled the three refusal tests fail.
Original entry:

Type: FIXABLE_NOW · Subsystem: x3-lang/vm/trading
Reason: round 3 added `quote_freshness` enforcement at execution time and to
the reconstructed economic policy, but `verify_receipt_economics` does not
re-check a receipt's realized legs against the quote ages they claim, because
receipts do not yet record a quote block per leg. Same gap as the general
realized-leg replay work.
Acceptance criteria: receipts record the quote block each swap leg was priced
from, and replay re-derives the age against the receipt's compiled policy.
Validation: a receipt whose leg was priced outside the ceiling must fail replay.

---

# Round 17 (2026-09-18, `107fb463a`) — new tickets and a correction

## TICKET-032 — `--deny-warnings` was honoured by `check` alone — CLOSED
Type: CLOSED in `107fb463a` · Subsystem: x3-lang/tooling
Reason: `--deny-warnings` is a `global = true` flag whose help text promises
"treat semantic warnings as failures". Only `cmd_check` read it. `cmd_build`,
`cmd_deploy` and `cmd_receipt_execute` accepted it and dropped the warnings.
Measured before the fix: `x3c build --deny-warnings` on a bridging intent
exited 0 while `x3c check` on the same source reported two warnings.
Impact beyond the CLI: the corpus sweep I ran last turn used
`build --deny-warnings` and reported all nine buildable examples "CLEAN". That
was a false green — five of them warn.
Fixed: `compile_with_mode_diagnostics` keeps the verifier's warnings on the
build path; one `report_warnings` helper reports them and enforces the flag in
build, deploy and receipt-execute. `compile_with_mode` delegates, so the two
cannot drift.
Lesson: a flag that is parsed but not read is worse than a missing flag,
because the absence of output reads as a pass. When measuring a corpus with a
new gate, check that the gate can fail at all.

## TICKET-033 — a timeout unit suffix is accepted and ignored — CLOSED
Type: CLOSED in `99f0ca42a` (2026-09-19) · Subsystem: x3-lang/compiler/parser + lowering + semantic
Closed: a duration is classified where it is read (`180s`, `40m`, `2h`, `1d`,
`500ms` → `LiteralExpr::Duration`), a bare number stays blocks, an undefined unit
is refused by name, and one function converts — `SECONDS_PER_BLOCK = 6` named
once, `MAX_TIMEOUT_BLOCKS` derived from it, rounding up because a window shorter
than the program asked for strands the claim. The ordering invariant compares
blocks and now *sees* units (`timeout source 20m / destination 40m` is rejected,
which it never was). The ceiling applies to every timeout, not only to `atomic
swap` clauses and the mainnet pass. `180s` now means 30 blocks, not 180; two
tests that encoded the old semantics derive their expectation from the block time
instead. Report: `x3lang-round35-20260919.md`.
Original entry:
Type: OPEN, security-relevant · Subsystem: x3-lang/lexer + lowering
Reason: `timeout source 40m` means 40 **blocks**, not 40 minutes. `40m` lexes
as an identifier, and the shared rule now reads the digits off the front, so
the suffix is decoration. Today `timeout 180s` lowers to `duration_blocks: 180`
— 18 minutes at 6 s/block, not 3. Every existing program inherits this, and the
`MAX_TIMEOUT_BLOCKS = 14400` comment ("24 hours at 6s/block") is the only place
the block-time assumption is written down.
Why it is not fixed here: changing the conversion moves the
timeout-ordering invariant (`destination + buffers < source`) for every
compiled program, which is a settlement-safety decision, not a bug fix.
Acceptance criteria: the language states one unit contract; the lexer classifies
durations so the suffix reaches lowering; a program written in minutes lowers to
the intended block count; the timeout-ordering invariant is re-derived against
the new units with tests on both sides of the boundary; the block-time constant
is named once and documented.
Validation: timeout ordering tests at 6 s/block and at a different block time,
plus a conformance case that would pass under the old units and must fail under
the new ones.

## TICKET-034 — `ON_TIMEOUT` discarded the deadline it was given — CLOSED
Type: CLOSED in `107fb463a` · Subsystem: x3-lang/emitter + vm
Reason: `Operation::OnTimeout { duration_blocks }` was emitted as
`[ON_TIMEOUT][0][0]` and the executor read the operand as a *register index*,
so the deadline was `r0` at that point — the residue of an unrelated
instruction. Measured: `examples/atomic_swap.x3` failed with
`instruction count 1 exceeded deadline 0`, and `examples/simple_swap.x3`
passed the same opcode only because its residue happened to be large. A
dry-run that passes or fails on register residue is the definition of
nondeterminism.
Fixed: the deadline is the instruction operand, and `0` means "no instruction
budget". `duration_blocks` is deliberately not used as an instruction budget —
it counts blocks, the VM has no block height, and enforcing it as a budget
would bound a program by its size rather than its behaviour. The declared value
stays in the IR for the timeout/refund engine, where the verifier already
rejects zero and out-of-range durations.
Still open (not this ticket): nothing enforces a block-based timeout at run
time. That belongs with the timeout/refund engine and is what TICKET-033's unit
contract has to settle first.

## TICKET-035 — `no_refund_after_claim` fires on a two-legged swap — CLOSED
Type: CLOSED in `4d557483b` (2026-09-19) · Subsystem: x3-lang/ir + semantic
Closed: the rule matched releases and refunds on `(chain, asset)` alone, so a
destination payout read as a claim of an escrow. It now requires the asset to have
been locked by the program — a refund can only double-spend an escrow that exists
— while the ordering stays atomic-scoped (the escrow set is program-wide, because
intent lowering puts the `from` endpoint outside the atomic block). That is the
"distinction the IR does carry" rather than a new `Claim` op: `Lock` is the
escrow, `Release` is either its claim or a payout, and only a claim of a locked
escrow followed by its refund is a violation. Measured:
`x3c check --deny-warnings examples/atomic_swap.x3` is exit 0 with no warnings,
and the corpus sweep's warning-free count moves 16/23 -> 17/23 — the corpus no
longer contains a warning at all. The existing claim-then-refund test now locks
the escrow first (its fixture omitted the `Lock`, which is what made it
indistinguishable from the payout), and a new two-legged test asserts no
violation. TICKET-001 remains open for the deeper ambiguity the ticket names: one
opcode with two meanings.
Original entry:
Type: OPEN · Subsystem: x3-lang/ir + semantic
Reason: `examples/atomic_swap.x3` (roadmap's canonical example) compiles and
runs, and emits one warning:
`invariant 'no_refund_after_claim' violated: Refund of sol.SOL found after its
Release (claim)`. Its IR is
`Lock eth.USDC → Release sol.SOL → OnTimeout(Refund eth.USDC) →
OnTimeout(Refund sol.SOL)`. The destination payout and the destination timeout
refund are the two settlement paths of the same leg, which is what an HTLC
swap is; the rule reads them as "claimed, then refunded".
The rule cannot do better over this IR: `Operation::Release` means both "claim
an escrowed lock" and "pay out the swapped asset", so a structural scan cannot
tell a double-spend from a two-legged swap. `no_double_refund` was fixable
because a refund names its lock; this one is not.
Acceptance criteria: the IR distinguishes a claim from a swap payout (an
explicit `Claim`, or a `Lock` for the destination leg so the payout is a claim
of it), and `no_refund_after_claim` is re-expressed against that distinction
with a violation test and a two-legged-swap test that must not warn.
Validation: a program that claims and refunds the same escrow must be rejected;
`examples/atomic_swap.x3` must be clean under `check --deny-warnings`.

## Test-integrity notes from round 17
- `crates/x3-tools/tests/cli.rs`'s `GOOD_SOURCE` is a **same-chain** intent, and
  its only warning was TICKET-024's false positive. Two `--deny-warnings` tests
  used it as the "program that warns" fixture, so they asserted that the
  compiler complains about a correct program and were green because of the bug.
  Replaced with `WARN_SOURCE`, a bridging program that warns for a correct
  reason, plus a build-path counterpart that also checks no bytecode is written
  on refusal.
- `compiler/src/lib.rs::compile_program_with_regalloc_runs_full_pipeline` had an
  unreachable `Ok` arm: the hand-built fixture's `Duration` timeouts were
  rejected by lowering, so it always took the `Err` arm and `alloc.len() == 0`
  was never evaluated. Fixing the timeout lowering made the arm live and the
  assertion false. It now asserts that the allocation record mirrors the IR it
  walked. Two of the three assertions in that arm asserted the allocator
  allocates nothing.
- Both are the same lesson as the `--deny-warnings` false green: an assertion
  that cannot fail is not evidence.

## TICKET-039 — `x3c fmt` rewrites valid programs into invalid ones — CLOSED
Type: CLOSED in `5c8bca061` · Subsystem: x3-lang/compiler/formatter
Reason: formatter arms written against an older grammar, with nothing tying them
to the parser. Round 25 recorded it on intents, round 32 measured it on
strategies, and the round-33 sweep put the real blast radius at **14 of the 17
examples the parser can read** — plus one rewrite that was semantic rather than
syntactic: `on_fail refund X to Y` came back as `on_fail rollback`, because the
clause folds its asset and receiver into one string and the formatter wrote that
string back as a quoted expression the clause parser does not read as an asset.
Fixed: every arm now emits what the parser reads (`vm`, `solver_market`,
`relayers`, `rpc_quorum`, `finality_policy`, `vm_target`, `risk_policy`,
`privacy`, `proofs`, `invariant`, `error`, `atomic swap`, route fallbacks, swap
venue order, `on_fail refund`, the intent body's clauses, and guards' comparison
operators).
Evidence: `compiler/tests/test_formatter_roundtrip.rs` requires the formatted text
of every parseable example to re-parse, to compile to **the same bytecode**, and
to be idempotent, with a floor on how many examples it checked. Corpus sweep:
17/17 valid, was 3/17. Full report: `x3lang-round33-20260918.md`.
Round 32's evidence, kept for the record: `x3c fmt examples/objective_routing.x3`
then `x3c check` gave `Parser error: chain name: expected identifier`, and
`on_fail refund ethereum.USDC to sender` came back as three statements.
Follow-ups filed: TICKET-043 (comments), TICKET-044 (`invariant` body).

## TICKET-045 — a two-word guard does not parse: `require proof verified` — CLOSED
Type: CLOSED in `f8b2e3bf0` and `1dcd17d58` · Subsystem: x3-lang/compiler/parser + x3-ast
Closed: `RequireGuard.value` is optional, so a guard can require a property
rather than compare against a number. The six shapes the corpus uses all parse to
the same guards they always did (`slippage <= 50`, `finality Ethereum >= 64`,
`finality.sol == finalized`, `nonce unused <id>`, and the two that could not be
written: `canonical_supply USDC` and `proof verified`). `require slippage` and
`require nonce unused` are refused; `RequireKind::asserts_a_property` names the
kinds whose readers do not compare a value, and a bound kind without one is
refused at the parser and again in lowering. A guard stops at `CLAUSE_WORDS`, so
it cannot swallow the clause that follows it — the first cut did, silently, and
the two clause orders are now a test. `arb_solana_eth.x3` parses (18 of 23
examples, was 17) and fails on its own semantic errors, recorded by name in the
formatter round-trip test. Report: `x3lang-round34-20260918.md`.
Round 33's evidence, kept: `require proof verified` in an intent gave
`Parser error: expected expression`; the parser read `verified` as the subject and
then required a value.

## TICKET-027 (progress, 2026-09-19) — the guard kinds, one row each

`5e89e6c7d` closed one more kind by **evaluation**: `require canonical_supply
<ASSET>` is decided against the program's own mint and burn operations
(`semantic::verify_canonical_supply`, run after lowering so it sees the flat IR
and cannot miss a mint inside a nested `atomic` block). Net zero is preserved; a
contradiction is refused with the figures; a guard naming no asset is refused as
unevaluable. Eight tests in `compiler/tests/test_canonical_supply.rs`.

The acceptance criterion is per kind, so here is the state of every kind, with
what backs it. "Unbacked" means the guard lowers to `REQUIRE_COMPARE_STATIC`,
which the executor treats as true, and nothing else reads it either.

| kind | backing | where |
|---|---|---|
| `solver_bond` | compared against `solver_market { bond <amount> <ASSET> }` | compile-time, `f2b972a3c` |
| `relayer_quorum` | compared against `relayers { quorum N_of_M }` | compile-time, `051c048c7` |
| `canonical_supply` | minted vs burned for the named asset | compile-time, `5e89e6c7d` |
| `slippage` | the guard's bound must be a ceiling and within the declared profile | compile-time, partial: the *realised* slippage has no source |
| `profit` | a floor in the body is required to discharge `guarantees [min_profit]` | compile-time, structural only |
| `refund_path` | a refund path must exist, decided in `verify_refund_path_exists` | compile-time, `f5605f857` |
| `finality` | **unbacked** — no declaration states a chain's required depth (`vm`/`finality_policy` declare a *mode*) | needs a depth declaration (TICKET-049) |
| `route_score` | compared against `risk_policy { min_route_score <n> }` | compile-time, `bd9e8f67b` |
| `risk` (`RiskScore`) | **unbacked** — `risk_policy` declares no score | TICKET-049, and TICKET-050 for its parser |
| `bridge_liquidity` | every declared `venue { kind bridge … }` must be at least as deep as the guard requires | compile-time, `b6a9a231d` |
| `nonce` | **unbacked** — the VM has no nonce registry, so the identifier is recorded in `metadata.nonce` and never consulted | needs a run-time registry (TICKET-051) |
| `proof_complete` | the named proof must be in `proofs required { … }` | compile-time, `4105ebc48` |
| `vm_supported` | the named family must appear in a `vm`, `target` or `venue` declaration | compile-time, `af858d3a9` |
| `invariant` | the named invariant must be declared | compile-time, `cdae5ea23` |
| `mainnet_safe`, `audit_gate` | **unbacked** — nothing compares these to anything | TICKET-049 |
| `custom` (a word the compiler does not know) | **refused** rather than recorded: a guard kind nothing can check is a comment | `4105ebc48`, closed |

So, after `bd9e8f67b`: **six kinds decided** (`solver_bond`, `relayer_quorum`,
`canonical_supply`, `proof_complete`, `refund_path`, `route_score`), **two
partial** (`slippage`, `profit`), **one closed by refusal** (an unknown kind), and
**six unbacked** (`finality`, `risk`, `bridge_liquidity`, `nonce`, and the
property kinds `vm_supported`/`mainnet_safe`/`audit_gate`/`invariant`). None of the unbacked ones may stay that way —
that is the ticket's own criterion — but each needs a declaration (or a registry)
that does not exist yet, which is why they are separate tickets rather than one
change.

## TICKET-049 — a declaration for every guard kind that compares against one
Type: CLOSED in `95a952901` (2026-09-19) · Subsystem: x3-lang/compiler (parser + AST + semantic + lowering)
Closed: every name in `REQUIRE_KIND_NAMES` is now either decided by a check or
refused by name, and the rule is stated in `verify_guard_kinds_are_checkable`
(renamed from `verify_guard_kinds_are_known`, because it refuses one *known* kind
too). `risk` is decided against the score `compute_risk_score` computes from the
program itself (a ceiling; a floor is refused, as is a bound that is not a number),
and the IR carries `RequireKind::RiskScore` instead of `Custom("risk_score")` so
the pass cannot be reached by a program that writes that name as an unknown guard.
`mainnet_safe` is a *request for the mainnet checks*: the pass runs them when the
build is for mainnet or the program asks, which is what makes the claim true rather
than recorded — `examples/mainnet_safe_swap.x3` now states the property it is named
for and passes in dev mode. `audit_gate` is refused: an audit is evidence about the
delivery process, not a property of the artifact, so no clause can state one.
Evidence: 31 tests in `compiler/tests/test_guard_declarations.rs` (the risk tests
compute the score rather than hardcoding it; the mainnet pair proves the guard is
what put the checks on the path); with the risk pass disabled the three risk tests
fail; corpus sweep unchanged at 17/23.
Original entries follow.

Progress (round 51, `1ab9a1a69`): `finality` is decided. `finality_policy` gained
`blocks <n>` (the depth the program requires of that chain) and
`semantic::verify_finality_guards_declared` decides both guard shapes: a depth
guard below the declared depth is refused with both numbers, one at or above it is
accepted, a mode guard is decided against the declaration's `requirement` word
case-insensitively, a chain no policy names is refused with the clause to add, a
policy with no `blocks` cannot back a depth guard, and two policies naming one
chain are refused where a guard depends on them. Twelve guards in nine corpus
programs now say what they require (generated from the guards, so no number is
invented). 11 tests in `compiler/tests/test_guard_declarations.rs` plus a corpus
invariant. Remaining: `risk`, `mainnet_safe`, `audit_gate`.
Type: OPEN, five kinds closed in `bd9e8f67b`, `4105ebc48`, `af858d3a9`, `cdae5ea23`, `b6a9a231d` · Subsystem: x3-lang/compiler (parser + AST + semantic)
Progress: `route_score`, `proof_complete`, `vm_supported`, `invariant` and
`bridge_liquidity` are decided now. Remaining: `finality`, `risk`, `mainnet_safe`,
`audit_gate` — each needs a declaration the language does not have (a chain's finality
depth, a risk score, evidence that the mainnet checks ran, an audit).
Progress: `route_score` (against `risk_policy { min_route_score }`), `proof_complete`
(against `proofs required`), `vm_supported` (against the `vm`/`target`/`venue`
adapters, compared through the family map so `solana` matches a declared `svm`) and
`invariant` (against the declared invariants) are decided now. Remaining:
`finality`, `risk`, `bridge_liquidity`, `mainnet_safe`, `audit_gate`.
Reason: seven guard kinds compare against a quantity no clause declares, so the
compiler cannot evaluate them and the emitter records them as `STATIC` (see the
table above). Two of them are one clause each away:
`risk_policy { min_route_score N }` gives `require route_score >= N` something to
be checked against (the same shape as `solver_market.bond`), and
`require proof_complete <proof_type>` has `proofs required { … }` already in the
language to compare against. `bridge_liquidity`, `risk`, `finality` and the
property kinds need the same treatment or an explicit refusal.
Acceptance criteria: each kind either gains a declared quantity and a check, or is
refused with a message naming what would have to exist; the examples that write
such a guard declare what they require (`examples/{simple_swap,mainnet_safe_swap,
staking_intent,route_fallback,multi_leg_route,flagship_b52}.x3` all require
`route_score >= 85..90` with no declaration anywhere).
Validation: a test per kind, in both directions, plus the corpus sweep.

## TICKET-050 — `risk_policy` silently skips fields it does not know — CLOSED
Type: CLOSED in `bd9e8f67b` · Subsystem: x3-lang/compiler/parser
Closed: an unknown field is refused by name (`unknown risk_policy field 'max_route_risk'; the
fields are max_slippage, max_position and min_route_score`), the doc comment now says
what the parser reads, and `min_route_score` — the field `route_score` guards
needed — is implemented. Test: `an_unknown_risk_policy_field_is_refused_by_name`.
Reason: `parse_risk_policy_item` ends its clause match with
`_ => { self.advance(); }`, commented "Skip unknown config fields". The doc
comment above the parser advertises `max_fee`, `max_route_risk` and
`min_liquidity`, and none of them exists — so a program that declares
`risk_policy { max_route_risk 3 }` has written a bound the compiler drops without
a word, which is the same defect class as TICKET-033's units. The corpus writes
only `max_slippage` and `max_position`, so nothing in it depends on the skip.
Acceptance criteria: an unknown field is a parse error naming the field and the
fields that exist; the advertised fields are either implemented (with a reader)
or removed from the doc comment.
Validation: `risk_policy { max_route_risk 3 }` refused by name; the corpus
sweep unchanged.

## TICKET-051 — no nonce registry, so `require nonce unused` is recorded only — CLOSED
Type: CLOSED in `0d22540b6` (2026-09-19) · Subsystem: x3-lang/vm + compiler
Closed: `NONCE_UNUSED` (0x9C) is a capability payload carrying the identifier; the
lowering emits it immediately before the guard, the executor leaves 1 in `r0` when
the nonce is new and records it in the run's `used_nonces`, and the emitter writes
`REQUIRE_COMPARE_GE` with threshold 1 for this kind instead of `STATIC`. That is the
first guard in the language with a real run-time quantity at it — TICKET-027's
criterion ("a real runtime quantity at its guard and emits a comparison") met by a
comparison rather than by evaluation, for the kind that could only be met that way.
Replay protection is a host fact, so the state carries what the run has seen and a
host can start the VM with nonces it already knows.
Measured: the same nonce twice fails at the second guard
(`X3_REQUIRE_FAILED: r0=0 is below the required 1 at pc 212`); two different nonces
pass. `vm/tests/test_nonce_replay.rs` — 4 tests.
Found while doing it: TICKET-055.
Original entry:
Type: OPEN, security-relevant · Subsystem: x3-lang/vm + compiler
Reason: `require nonce unused <id>` is the one guard whose quantity is a *chain*
fact rather than a program fact, and it appears in fourteen corpus programs. The
identifier reaches `metadata.nonce` and nothing consults it: the VM has
`META_NONCE` (a metadata store) and no used-nonce set, so replay protection is a
claim the artifact carries and nothing enforces — at compile time or at run time.
Acceptance criteria: the VM keeps a used-nonce set, an operation tests and
records a nonce, and the guard emits a comparison against it; a program that
replays a nonce fails at the second use.
Validation: two runs of one artifact with the same nonce — the first passes, the
second fails at the guard — plus a negative test that a fresh nonce still passes.

## TICKET-052 — `risk_policy.max_slippage` is a percentage lowered as a risk score — CLOSED
Type: CLOSED in `d40463f74` (2026-09-19) · Subsystem: x3-lang/compiler/lowering
Closed: a slippage bound lowers to
`Require { kind: SlippageTolerance, condition: Expression, comparison: <= }` — the
same kind and unit as `require slippage <= n`, and the operation the mainnet
slippage ceiling reads (so the policy is no longer invisible to it: 600 bps is
refused in mainnet mode as 6.00%). A guard *looser* than the policy it sits under
is refused by the new `verify_risk_policy_bounds_guards`; zero is "unstated".
`b52_test.rs` asserted the conflation and is re-pointed at the meaning;
`compiler/tests/test_risk_policy.rs` is new (five tests).
Found while fixing it: the change moved a guard to the front of a bytecode stream
and exposed TICKET-053.
Original entry:
Type: OPEN · Subsystem: x3-lang/compiler/lowering
Reason: `Item::RiskPolicy` lowers to `Operation::RiskScore { score: policy.max_slippage,
category: "slippage" }`, and the VM's `RiskScore` capability requires `score <= 100`
("risk score: score must be <= 100" — `vm/src/executor.rs`). The field is documented
as `<pct>`, so a program that bounds its slippage at 120 (basis points, as every
guard in the corpus does) becomes a risk score of 120 and **stops running**.
Measured while adding `min_route_score` to the examples: inserting
`risk_policy { max_slippage 120 }` into `examples/multi_leg_route.x3`, which
already requires `slippage <= 120`, took the corpus's run count from 17 to 16 with
exactly that panic. The insert was reverted; the conflation is the defect.
Acceptance criteria: one unit per field. Either `max_slippage` is a percentage and
is not lowered as a risk score (a slippage bound is not a score), or it is a score
and the parser refuses > 100 at the literal with a message saying so; the guards a
program writes and the policy it declares are compared in one unit.
Validation: a program with `risk_policy { max_slippage 120 }` either refuses at the
field or runs; a test pins whichever is chosen, and the corpus sweep stays at 17/23.

Type: OPEN, low · Subsystem: x3-lang/compiler/parser
Reason: TICKET-033's fix covers `timeout` in the intent, `atomic swap` and
`on_timeout` clauses, which now accept `180s` / `40m` / `2h` / `1d` / `500ms` and
refuse a unit they do not define. `parse_deadline_expr` — the trading core's
`deadline: N blocks` — still reads its own unit and accepts `blocks` alone, so
`deadline: 2h` is either a parse error or, worse, a number of blocks the program
did not mean. It is the same defect in a second clause, found while closing the
first and not fixed with it because the trading deadline is lowered by a
different pass.
Acceptance criteria: `parse_deadline_expr` uses `parse_duration_expr`, so a
deadline written as a duration converts at the same block time as a timeout, and
a unit the language does not define is refused by name.
Validation: `compiler/tests/test_timeout_units.rs` extended with the deadline
clause, or a sibling test; a trading program with `deadline: 2h` lowers to the
same blocks as `timeout 2h`.

## TICKET-046 — clause words are a list rather than a lexical class — CLOSED
Type: CLOSED in `c7dfb461a` (2026-09-20), on `4bed37b14` · Subsystem: x3-lang/compiler/parser +
x3-lang/crates/x3-lexer
**Closed: the decision is (a) — keep the union list — and measuring the ticket instead of deciding it
found the defect the acceptance was actually pointing at.** The acceptance asked for the 22 words to
become keyword tokens "so [a clause word] cannot begin an expression and no guard needs a list to stop
at one". That is **already true of nine of them**: `swap`, `bridge`, `require`, `emit`, `use`, `mint`,
`burn`, `lock` and `release` are lexer keywords with `Tok::Kw*` mappings, and `can_start_expression`
refuses a keyword, so a guard stops there with no list entry (the const says so, and it is right).
What was *not* true is the consequence of the same fact, and it is the reason this was worth
measuring: **an arm written against `Tok::Ident(ref s) if s == "<keyword>"` can never run**, and three
clauses were written that way. Each was refused while the comment above it, the error message and the
formatter all said the clause was supported:

```
$ x3c build use_probe.x3          # `use uniswap 1` in an intent body
x3c: compile error: Parser error: unexpected clause in intent body: KwUse; expected one of
  `from`, `to`, `route`, `require`, `timeout`, `on_fail`, `use` or `on`
$ x3c check fp_terse.x3           # finality_policy strict { ethereum require finalized  blocks 12 }
x3c: parsing failed: Parser error: expected '}' after finality_policy body
$ x3c check rq_inline.x3          # rpc_quorum { source require 2_of_3  relayers a b c }
x3c: parsing failed: Parser error: rpc_quorum source chain: expected identifier
```

All three parse now — the arms match `Tok::KwUse` / `Tok::KwRequire` — and the first one runs:
`use uniswap 1` reaches the artifact as `HostCall { function: "use", args: ["uniswap", "1"] }`,
424 bytes, `x3c run: ok`.

`CLAUSE_WORDS` itself lost one entry and kept the rest, and both halves are now measured rather
than argued:

- **`balance` removed.** It was listed under "statements and trade bodies that carry a guard" and
  **no arm anywhere in the parser dispatches on it** (one occurrence in the file: the list entry).
  A lookahead that stops at a word which begins nothing reads `require <kind> balance` as a guard
  with no subject. `every_word_in_this_list_begins_a_clause` reads the const out of the parser source
  and fails for any word no arm dispatches — verified load-bearing by re-adding `balance`, which
  fails it with *"these words stop a guard but begin no clause in the parser: [balance]"*.
- **Nothing is missing.** The words a guard can actually be followed by in a body that holds
  statements are the intent body's (`from`, `to`, `route`, `timeout`, `on_fail`, `allow`, `on`,
  `proofs`) and the swap body's (`amount`, `receiver`, `hashlock`, `min_output`); both are covered.
  `fallback` — the one route-step word that is *not* a lexer keyword — cannot follow a guard: route
  blocks are read by a step loop that accepts only route operations, measured by putting a `require`
  between two steps (`expected route operation (swap/bridge/lock/mint/burn/release/fallback)`), so
  no entry is needed for it or for the rest of the route-step words.

The ticket's own validation was extended from nine words to thirteen: `allow`, `on`, `use` (intent
body) and `min_output` (swap body) were reachable in the two bodies the fixtures already express —
which is what "reaching them means writing a valid fixture for each of those grammars" turned out to
cost, once the question was which bodies can hold a `require` at all rather than which words are
interesting. The `use` fixture is the one that would have caught the dead arm: with the identifier
form restored it panics with the parser's own `unexpected clause in intent body: KwUse`.
The other two are in `compiler/tests/test_keyword_clauses.rs`, and with the identifier lookahead put
back, two of its four tests fail.

Options (b) and (c) from the decision request are recorded as not taken. (b) still requires every
name position to accept a keyword token — `debt.amount` is written by two shipped examples — which is
the opposite of the acceptance's premise, and the measurement above shows the acceptance's *purpose*
(a guard that stops without a list) already holds for the words that are keywords. (c) — the block's
own clause set instead of the union — remains the follow-up if drift is ever judged material; it is
not today, because both directions of drift now fail a test rather than a program.

Proof: 1233 workspace tests (was 1228; +1 staleness, +4 keyword clauses), clippy `-D warnings`, fmt
clean, 23 python tests, sweep `20/20/20/19`, and `no-float-in-consensus`, `cargo-lockfile-locked`,
`invariant-registry`, `workspace-membership` PASS.

Note on the commit message of `c7dfb461a`: it was written with backticks quoted into a shell
argument, so three of them were expanded away (`lists \`use\` among`, and the argument list of the
`HostCall` record). The content is correct; this entry and
`.ai/reports/x3lang-keyword-clauses-20260920.md` carry the text as intended. Not force-pushed —
rewriting a pushed `master` while other agents are working on it is the hazard the merge-queue doc
names.

Original Type: PARTIAL in `4bed37b14` (2026-09-20) — the validation is landed, and the acceptance
**conflicts with the language**.
**Landed: the ticket's own validation.** A guard stops at the next clause because `CLAUSE_WORDS`
names the words that can begin one, and two entries were missing on the first pass (`amount`,
`timeout`) — each cost a round. Two tests covered those two and nothing covered the rest. The
validation is written mechanically now: the fixtures are **lists of clause lines** and, for each
line, the valueless guard goes before it and after it, so the two sources differ only in where a
guard that takes no value sits and must lower to the same program. Over a swap body that is
`amount`, `receiver`, `hashlock`, `timeout`, `finality`; over an intent body `from`, `to`, `route`,
`timeout`, `on_fail` — sixteen placements, one assertion each. Proven load-bearing by removing
`amount` from the list, which fails with *"a guard before `amount` changed the program: the guard
took the clause with it"* and fails the pre-existing test with it: two guards for one mistake, which
is what the first pass did not have.
**Not landed, and the reason is a conflict rather than effort.** The acceptance is that each of the
22 words becomes a **keyword token** so no list is needed. That cannot be done without deciding what
these words are allowed to be, because the language uses them as **field names**:
- `expect_ident` accepts only `Tok::Ident`, and it is what reads the field name after `.`;
- `debt.amount` is written by two shipped examples (`trading_core_v1.x3`, `trading_effects.x3`), so
  making `amount` a keyword token breaks a program the compiler accepts today;
- the flattening happens in the conversion —
  `TokenKind::Keyword(kw) => keyword_to_tok(kw).unwrap_or_else(|| Tok::Ident(kw.as_str()…))` — so
  the lexer *already* knows these words as keywords and the parser turns them back into identifiers.
Making them keywords therefore requires `expect_ident` (and every other name position) to accept
keyword tokens, which is the opposite of the acceptance's own premise, "so it cannot begin an
expression". That is a reserved-word decision for whoever owns the grammar. **The acceptance should
be amended with it before this is attempted**, and the measurement above is why that is known rather
than discovered mid-refactor.
Measured scope for whoever does it: 22 words, 36 `Tok::Ident(..) if .. == "<clause word>"` sites,
42 `keyword_to_tok` arms and 46 `Kw*` variants in a 79-variant `Tok`.
**DECISION REQUEST (2026-09-20, after reading the guard parser end to end).** The ambiguity is
*structural*, and that is what decides the options. A guard stops where the next clause begins, and the
two forms
```
require proof_complete <name>     // the guard is about a proof
require proof_complete            // the guard is a property …
amount 500                        // … and this begins the next clause
```
are the same token sequence up to the identifier — so no lexical rule can separate them; only *which
clauses the enclosing block allows* can. Three options, with their measured costs:
- **(a) keep the union list, and its mechanical validation** (what landed in `4bed37b14`). The list can
  no longer drift silently: sixteen placements assert that a valueless guard parses the same before and
  after each clause line, and removing a word fails two tests. Cost: none. Cost of being wrong: a word
  that a program meant as a guard's *value* is refused loudly rather than mis-parsed.
- **(b) make the 22 words keyword tokens** (the acceptance as written). Measured cost: 42 `keyword_to_tok`
  arms, 46 `Kw*` variants in a 79-variant `Tok`, 36 parser sites, **and every name position must then
  accept a keyword token** — `expect_ident` is what reads the field after `.`, and `debt.amount` is
  written by two shipped examples. So this option makes the lexer stricter and every name position
  looser, which is the opposite of the acceptance's own premise ("so it cannot begin an expression").
- **(c) give the guard the *block's* clause set and try-parse**: a swap body's clauses are not an intent
  body's, so a per-block set is *more* correct than the union (the union stops a guard at a word the
  block may not allow). Measured cost: 122 per-block clause comparisons to derive sets from, and the
  guard parser's signature changes to receive one. It satisfies the acceptance's purpose — no union list
  — without touching the lexer or the field-name positions.
**Recommendation**: (a) now, with (c) as a follow-up if the drift risk is judged material — the validation
already turns a missing word into two failing tests rather than a silent mis-parse, which is the failure
the list was ever about. (b) is not recommended for the reason in its own row. Whoever owns the grammar
should pick, which is why this is a request rather than a change.
Original entry:
Reason: a guard has to stop where the next clause begins, and it knows where that
is because `CLAUSE_WORDS` names the words that can begin one: `from`, `to`,
`route`, `timeout`, `on_fail`, `allow`, `on`, `proofs`, `amount`, `receiver`,
`hashlock`, `min_output`, `net_output`, `replace`, `leg`, `path`, `choose`,
`repay`, `borrow`, `balance`, `invariant`, `net_profit`. That list is a second
statement of the grammar. Two of its entries were missing on the first pass
(`timeout` was assumed to be a keyword; the parser's own clause arm is
`Tok::Ident("timeout")`) and each cost a round of repair.
Why not fixed here: it is a lexer change affecting every clause in the language,
and it would move tokens the parser's arms are written against.
Acceptance criteria: every word that can begin a clause is a keyword token, so it
cannot begin an expression and no guard needs a list to stop at one; the parser's
clause arms accept the keyword form; `CLAUSE_WORDS` is deleted; the
`test_require_guards.rs` clause-order tests still pass for every clause word.
Validation: a test that, for each clause in the grammar, a valueless guard
immediately before it parses to the same program as the same clauses in the other
order.

## TICKET-047 — fractional amounts in an intent lower to zero — CLOSED
Type: CLOSED in `b6fff9dd1` (2026-09-19) · Subsystem: x3-lang/compiler (lowering)
Closed: `expression_to_u128` already refused a fractional literal — the swap lowering
called the *non-erroring* wrapper (`expression_to_u128_opt(...).unwrap_or(0)`) and wrote
0, so `min_output 0.09` reached the verifier as "must be greater than zero" three passes
later. Both amounts now use the erroring conversion, so the refusal names the literal
and points at the exact path (an asset's declared decimals). A written zero is still
refused *for being zero*, with the verifier's own message — two different mistakes, two
different messages. `expression_to_u128_opt` had no other caller and is gone.
Tests: `compiler/tests/test_fractional_amounts.rs`.
Original entry:
Type: OPEN, decision needed · Subsystem: x3-lang/compiler (parser + checks)
Reason: `examples/arb_solana_eth.x3` writes `swap Raydium Solana.USDC -> Solana.SOL
amount 10 min_output 0.09` and `min_output 10.25`. The values parse (a `Float`
literal), and then five checks refuse the program:
`X3E0501: swap min_output must be greater than zero`,
`bridge amount must be greater than zero`, `swap input_amount must be greater
than zero`, `bridge has zero amount`, `swap has zero input_amount`. The same file
also bridges `Solana.SOL`, which no `from` clause funds, so the bridge's amount
has nothing to inherit.
So the example is wrong in at least one way, and the language may also be wrong
in another: either fractional amounts are not a thing (and the parse should say
so where the number is written) or they are (and the checks should compare them).
Acceptance criteria: one of the two is chosen and stated; if fractional amounts
are refused, the refusal names the literal and its line rather than reporting a
zero three passes later; if they are supported, `min_output 0.09` lowers to the
decimal amount it says and the checks compare it.
Validation: `x3c check examples/arb_solana_eth.x3` either clean (after the
example's bridge is funded from a plausible asset) or refused at the literal.

## TICKET-045 (original report) — a two-word guard does not parse
Type: FIXABLE_NOW (now closed, entry kept for the trail) · Subsystem: x3-lang/compiler/parser
Reason: `parse_require_guard` reads `require <kind> [.subject | <subject>] [cmp] <value>`,
and when there is no comparison it takes the next identifier as the *subject* and
then requires a value expression. A guard written as two words — which is how a
property rather than a threshold reads — therefore consumes its own assertion and
asks for a value that is not there:

    $ printf 'intent a {\n    from Solana.USDC amount 10\n    to Ethereum.USDC\n    require proof verified\n    on_fail rollback\n}\n' > p.x3
    $ x3c check p.x3
    x3c: lowering failed: Parser error: expected expression

It is the **only** thing standing between `examples/arb_solana_eth.x3` (19 lines,
written in the current dialect: `intent`/`from`/`to`/`route`/`swap`/`bridge`/
`require`/`timeout`/`on_fail`) and the corpus: lines 15 and 16 are
`require proof verified` and `require canonical_supply USDC`, and truncating the
file before line 15 makes it parse. The three-word form works, which is why
`require nonce unused <id>` has always been fine.
Acceptance criteria: a guard with a subject and no value is expressible — either
the value becomes optional in `RequireGuard` (and every reader of it says what a
missing value means), or the parser stops at a clause boundary
(`}`, EOF, `require`, `timeout`, `on_fail`, `route`, `from`, `to`, `use`, `on`,
`allow`, `path`, `choose`, `leg`) instead of demanding an expression there.
Whatever is chosen, `require proof verified` and `require canonical_supply USDC`
parse to something a verifier can check, and a guard with genuinely nothing after
it is still an error.
Validation: `x3c check examples/arb_solana_eth.x3` clean; a negative test where
`require <kind>` with nothing after it is refused.

## TICKET-013 and TICKET-014 — the six non-parsing examples, per file
Status: **evidence updated in round 33; the decision is the owner's, see below.**
Round 33 measured each file by deleting the `contract <name> {` wrapper and its
closing brace, which separates "the wrapper is missing" from "the body is another
language":

| file | after removing the wrapper | what is actually missing |
|---|---|---|
| `arb_solana_eth.x3` | still fails | TICKET-045 — a two-word guard; **in the current dialect otherwise** |
| `jit_lp.x3`, `mev_smooth.x3` | `expected ';' after const` | typed consts, `const X: address = 0x…` |
| `arb.x3`, `flash.x3` | `unknown annotation @swarm` | `@swarm` on its own line, no arguments, in five files |
| `x3_coin_layer.x3` | `expected top-level item` | a different dialect entirely: `primitive AtomicSwap { function f(x: ChainID, …) }` |

The five `contract`-using files are Solidity-flavoured — `contract` wrappers,
`function` with typed parameters, `const X: address = 0x…`, `@gas_limit`-style
attributes. The spec is explicit about the direction: *"Your mission is not to
build another Solidity-like smart-contract language"* (`pasted-text-1.txt` line
740), and the roadmap's `contract` sketch is a different, asset-native thing
(`contract Vault { deposit asset.USDC; invariant …; emergency_pause allowed_by
governance.3_of_5 }`).
So this is not a parser gap to close by writing the dialect the spec argues
against. The decision to make is one of:
1. **Retire them** — move the five to `examples/legacy/` (or delete) and keep the
   corpus to the language the spec describes; TICKET-013 and TICKET-014 close as
   "not wanted".
2. **Implement the roadmap's `contract`** — the asset-native form above, with
   `deposit`/`invariant`/`emergency_pause` given real meaning, and leave the
   Solidity spelling out; the five files stay broken until they are rewritten in
   the language that exists.
3. **Support the Solidity spelling too** — the most work, and the only one that
   contradicts the spec.
Recommendation: (1) plus (2)'s roadmap form when a phase asks for it. Not started
here, because a `contract` that parses and does nothing is the "placeholder
logic" the instructions forbid, and the examples' bodies would need real
lowering for `function`, typed parameters and `@swarm` to be anything else.

## TICKET-043 — `x3c fmt` cannot keep a comment — CLOSED
Type: CLOSED in `248b59c91` (2026-09-19) · Subsystem: x3-lang/crates/x3-lexer + x3-lang/compiler/formatter
Closed: the lexer emits `TokenKind::Comment(CommentKind)` with the comment's span; the
parser steps over them (the grammar is unchanged) and `parser::source_comments` reads
the same stream, so the formatter's idea of what a comment is comes from the lexer —
the source-text scanner written for round 33's warning is gone.
`format_program_with_comments` writes each comment before the top-level declaration it
precedes: the AST holds no comments, so that is the finest association available
without re-parsing, and a comment written *inside* a declaration moves to that
declaration's boundary. `x3c fmt` says how many it placed there, because moving
documentation is better than deleting it and worse than leaving it where it was.
Measured: `x3c fmt examples/objective_routing.x3` keeps its 49-comment header and the
file still checks. Tests: `formatting_keeps_the_comments_it_can_place` (header, bare
`//` line, trailing note, position, bytecode equality, idempotence) and
`the_corpus_is_documentation_heavy`.
Remaining limitation, stated: a comment's *line* is not preserved inside a
declaration. That needs comments attached to AST nodes rather than to the item they
precede, which is a larger change and is not claimed here.
Original entry:
Type: OPEN · Subsystem: x3-lang/crates/x3-lexer + x3-lang/compiler/formatter
Reason: the lexer treats a comment as whitespace — that is what TICKET-012's fix
chose, and it was the minimal fix for parsing — so no comment reaches the AST and
the formatter has nothing to place. `x3c fmt` therefore deletes every comment in
whatever file it is pointed at. For this repository that is not a cosmetic loss:
the examples' comments are the documentation the phase audits cite, and a
formatted example loses the reasoning that makes it an example. Round 33 made the
loss *visible* (`x3c fmt` counts the comments and warns) but not smaller.
Acceptance criteria: the lexer emits comment tokens with their spans; the
formatter places each one against the node it preceded or trailed; `x3c fmt` on
`examples/simple_swap.x3` keeps its header comment and its inline comments and the
result still satisfies `test_formatter_roundtrip.rs`; `x3c fmt --check` reports a
file with a comment as already formatted when nothing but layout changes.
Validation: a round-trip test that compares comment *content* before and after,
not just bytecode.

## TICKET-044 — `invariant`'s body is stored as Rust `{:?}` — CLOSED
Type: CLOSED in `864440a70` (2026-09-19) · Subsystem: x3-lang/compiler/parser + formatter
Closed: the body is rendered with a shared `formatter::expression_to_source`, so
`InvariantCheck.assert_expr` is the source the program wrote (`profit >= 5`, not a Rust
value tree), and the formatter writes `invariant <name> { assert <body> }` back
successfully. The same defect was one level down: `format_expression`'s binary arm used
`{:?}` for the operator (`Ge`), which is not a program; `BinOp`'s `Display` is used now.
The `assert` keyword is checked rather than accepted by position.
Test: `an_invariant_body_is_source_text_and_survives_formatting` (stored text, formatted
text, bytecode equality). The body form has no corpus user, so this was exercised for
the first time by that test.
Original entry:
Type: OPEN, low · Subsystem: x3-lang/compiler/parser
Reason: `parse_invariant_decl_item` stores `Symbol::new(&format!("{:?}", expr))`
for the `invariant <name> { assert <expr> }` form, so the AST holds a Rust debug
rendering — not X3 source. It reaches the IR as `InvariantCheck.assert_expr`
(`lowering.rs`), and the formatter cannot write the body back: re-parsing the
stored text is not the same program. The bare `invariant <name>` form (which the
corpus uses) is unaffected.
Acceptance criteria: the declaration keeps the expression rather than a rendering
of it; the IR field is produced by a renderer both the parser and the formatter
agree on; a round-trip test covers the body form.
Validation: `invariant a { assert x <= 5 }` formats, re-parses, and compiles to
the same bytecode.

## TICKET-042 — `x3c graph` does not say that a declared objective is being ignored — CLOSED
Type: CLOSED in `52f79a4b9` (2026-09-19) · Subsystem: x3-lang/crates/x3-tools
Closed by the second of the ticket's two acceptable answers: `graph` still lists what the
graph holds, and now says so, naming the objective and its metric and pointing at the
command that applies them (`x3c optimize`). Measured on `examples/objective_routing.x3`:
the note appears and the 9 bps route the declaration's 8 bps ceiling excludes is still
listed, which is exactly the difference the note is about. A program with no declaration
gets no note. Test: `graph_says_when_it_is_ignoring_a_declared_objective`.
Original entry:
Type: FIXABLE_NOW, low · Subsystem: x3-lang/crates/x3-tools
Reason: `x3c optimize` follows the program's `objective` declaration: its metric
and every ceiling. `x3c graph` searches the same program with command-line
constraints only, so it will print routes the program's own declaration refuses
(`examples/objective_routing.x3 --from ethereum.USDC --to solana.SOL` lists both
routes, including the 9 bps one that `fees <= 8` excludes). Listing everything is
a defensible thing for a reachability command to do, but nothing says it is
doing it, and a reader has no way to tell "these are the routes" from "these are
the routes the declaration allows".
Acceptance criteria: `x3c graph` either applies the declared ceilings (and names
the objective it applied, as `optimize` does) or states that it is ignoring them
and why; a test covers the case where the two answer differently.
Validation: on `examples/objective_routing.x3`, `x3c graph` and `x3c optimize`
either agree on the candidate set or the difference is printed.

## TICKET-053 — the verifier walked a compiler stream out of step with the executor — CLOSED
Type: CLOSED in `50adbcfa9` (2026-09-19) · Subsystem: x3-lang/vm
Reason: `verify` advanced a fixed-frame instruction by `pc + 4` while the executor
advanced it by `align4(pc + 3)`. In a compiler stream the first instruction is at
offset 1 (byte 0 is the version byte) and the emitter pads each instruction to the
next *absolute* multiple of four, so the two walks differed whenever a frame was
padded: the verifier read padding bytes as opcodes and refused an artifact the
executor runs.

    $ x3c build guard_first.x3 --out a.x3b     # 176 bytes
    $ x3c run a.x3b
    x3c: error: VM error: Panic("X3_VERIFY_FAILED: OutOfBounds(73)")

The walk reached offset 73, read a byte of a payload string as an opcode, and
reported a payload length of 19535 past the end of a 176-byte stream. Exposed by
`d40463f74` (a guard became the first instruction); the disagreement is older than
that change, and any program whose first instruction's frame was padded could
reach it. For raw bytecode, which starts at offset 0, the two rules are the same
number, so nothing changes there.
Fixed: `pc = align4(pc + 3)`, making the verifier's boundary set the set the
executor visits. Test:
`vm/tests/test_e2e_examples.rs::a_guard_first_program_verifies_and_runs` asserts
the stream's shape, that offset 1 is a verified boundary, and that the executor
runs the artifact — agreement is the property, not either side alone.

## TICKET-054 — one slippage literal, three readings — CLOSED
Type: CLOSED in `7ff72c3e1` (2026-09-19) · Subsystem: x3-lang/compiler (semantic + risk + strategy)
Reason: the mainnet ceiling read a bare number as basis points (but a fractional
one as a percentage); the risk scorer multiplied a bare number by 100 as a whole
percent; the strategy check compared a bare number directly against
`max_slippage_bps`. So the corpus's most common bound — `require slippage <= 50`,
0.5% — was reported by `x3c score` as
`slippage_risk 40 · slippage risk: high slippage (5000bps / 50.00%)`, and
`<= 120` as 120%.
Closed: one rule in one place — `semantic::slippage_bps_from_expr` (AST) and
`slippage_bps_from_text` (what the IR carries): a bare number is **basis points**,
a percent literal is a **percentage**, and the two agree where they name one bound
(`<= 50` and `<= 0.5%` are both 50 bps). A fraction of a basis point (`<= 50.5`,
`<= 0.005%`) is refused rather than rounded; a fractional part of zero is not a
fraction. All four readers use it: the mainnet ceiling, the risk scorer, the
strategy profile check, and `verify_risk_policy_bounds_guards`.
After: all three corpus examples with a slippage bound score `slippage_risk 5` and
no "high slippage" line. `mainnet_rejects_unsafe_slippage`'s fixture
(`expr: "10.0"`, read as 10% under the old rule) now names `600`, over the ceiling
either way. Six tests in `compiler/tests/test_slippage_units.rs`.

## TICKET-048 — the trading `deadline` still reads blocks only — CLOSED
Type: CLOSED in `78f6bb566` (2026-09-19) · Subsystem: x3-lang/compiler/parser
Closed: `parse_deadline_expr` goes through `parse_duration_expr`, so `deadline: 2h`
is 1200 blocks — the same duration, at the same block time, as `timeout 2h` — and
the trading lowering converts through `expression_to_blocks` instead of demanding
an integer literal, so a fraction of a block is refused rather than truncated. The
unit may also be written as its own word (`30 seconds`), which the timeout clauses
now accept too. `literal_u64` had no other caller and is gone.
Tests: `compiler/tests/test_timeout_units.rs` reads the number out of the *compiled*
policy in the IR (2h→1200, 30s→5, 1d→14400, 45s→8, `2 blocks`/`2`→2, `30 seconds`→5,
`2x` refused by name).
Original entry:
Type: OPEN, low · Subsystem: x3-lang/compiler/parser
Reason: the timeout-unit fix (TICKET-033) covers `timeout` in the intent,
`atomic swap` and `on_timeout` clauses, which accept `180s` / `40m` / `2h` / `1d` /
`500ms` and refuse a unit they do not define. `parse_deadline_expr` — the trading
core's `deadline: N blocks` — still reads its own unit and accepts `blocks` alone,
so `deadline: 2h` is either a parse error or a number of blocks the program did not
mean. Found while closing TICKET-033 and not fixed with it because the trading
deadline is lowered by a different pass.
Acceptance criteria: `parse_deadline_expr` uses `parse_duration_expr`, so a
deadline written as a duration converts at the same block time as a timeout, and a
unit the language does not define is refused by name.
Validation: `compiler/tests/test_timeout_units.rs` extended with the deadline
clause; a trading program with `deadline: 2h` lowers to the same blocks as
`timeout 2h`.

## Note, 2026-09-19 — three ticket entries were silently not written
TICKET-048, TICKET-053 and TICKET-054 were reported in the round reports of
2026-09-19 but their ledger inserts did nothing: the patch script replaced a text
anchor that did not exist in this file, and `str.replace` returns the string
unchanged instead of failing. The lesson is the one this codebase keeps teaching —
a write whose landing is not checked is a write that may not have happened — and it
applies to the untracked `.ai/` trail as much as to code, where the compiler and
the tests catch it. The entries above were appended and then counted.

## TICKET-055 — two structures say which opcodes carry payloads, and they disagree — CLOSED
Type: CLOSED in `4e2890c63`, `1c9d54812` and `92c9c581b` (2026-09-19) · Subsystem: x3-lang/vm + x3-lang/spec
Closed: classification (`is_payload_opcode`), charging (`gas_surcharge`), dispatch
(the walk) and *names* (`opcode_name`, in the file both crates `include!`) are one
statement each, and the walk asserts every payload opcode is both recognised and
named. The disassembler reads the predicate too, so the trace, the verifier, the
executor and the gas charge cannot hold four opinions about one instruction. Along
the way the executor's duplicate name table and the disassembler's separate table
were deleted (the nonce instruction had been missing from the former, and the latter
described a `VECTOR` range no instruction has ever occupied).
Done: the *classification* has one source (`is_payload_opcode`) and two walks over
`0x00..=0xFF` driven by it — the executor must not refuse a payload opcode as
invalid, and the verifier must not desynchronise on one. The test that should have
caught the original bug walked the range `GPU_DISPATCH..=SUB_EXEC` and asserted
names, i.e. it encoded the same assumption as the bug.
Remaining: the *tables that describe* payloads are still per-site — the executor's
`capability_opcode_name` beside the compiler's disassembler names (the nonce
instruction was missing from the executor's, found by the new name assertion), and
the gas surcharge (TICKET-056). A payload opcode missing from a name table is now
caught; one missing from a *dispatch arm* is caught by the walk; a third kind of
table would not be.
Acceptance criteria: one place names each opcode, read by the disassembler, the
executor's diagnostics and any tooling.
Original entry:
Type: OPEN, medium · Subsystem: x3-lang/vm + x3-lang/spec
Reason: adding `NONCE_UNUSED` (0x9C) meant touching every place that enumerates
payload opcodes, and two of them are independent statements of the same fact:
`spec/opcodes.rs::is_payload_opcode` (used by the verifier and the compiler's
disassembler) lists opcodes explicitly, while the executor dispatches by *ranges*
(`GPU_DISPATCH..=SUB_EXEC`, `ROUTE_SCORE..=REFUND_POLICY`, `TRADING_BEGIN..=TRADING_BRIDGE`).
A new opcode can therefore be a payload opcode to the verifier and an invalid one to
the executor: the first build of this change verified its artifacts and then failed
at run time with `InvalidOpcode(156)`, because 0x9C sits between the two ranges.
The same shape bit the verifier before (`is_payload_opcode` once omitted `EMIT` and
`CALL_HOST`, so the walk ran four bytes into them — the comment above the function
records it).
Acceptance criteria: one source of truth for "which opcodes carry a payload", read
by the verifier, the executor, the gas table and the disassembler; a test that walks
every payload opcode and asserts the executor accepts it (not `InvalidOpcode`),
so a new instruction cannot be added to one list and missed by another.
Validation: the table test above, plus the nonce instruction's four tests still
passing.

## TICKET-056 — the gas surcharge is a fifth statement of "which opcodes carry payloads" — CLOSED
Type: CLOSED in `1c9d54812` (2026-09-19) · Subsystem: x3-lang/vm
Closed: `gas_surcharge`'s payload case reads `is_payload_opcode` with the stream's own
framing flag, so every instruction whose payload is read is charged `payload_len / 32`
for it — including `ATOMIC_CHOICE`, `ROUTE_FALLBACK`, `PARALLEL_PLAN`,
`STRATEGY_LICENSE` and the trading range, which were charged nothing. The asset ops
carry a payload only in a compiler stream, which is what the flag says; in raw
bytecode they are fixed frames and there is nothing to read.
Tested exactly: for every payload opcode, `cost(64-byte payload) == cost(empty) + 2`.
"a longer payload does not cost less" would pass when nothing is charged at all,
which was the defect.
The corpus's `gas remaining` figures move slightly and the corpus still runs 17/23.
Original entry:
Type: OPEN, low-medium · Subsystem: x3-lang/vm
Reason: `gas_surcharge` charges `payload_len / 32` for
`0x20..=0x25 | 0x60 | 0x61 | 0x80..=0x9C | 0xA0..=0xAB`, which omits instructions
that carry payloads by the shared predicate and are read as payload frames by both
the verifier and the executor: `ATOMIC_CHOICE` (0x53), `ROUTE_FALLBACK` (0x54),
`PARALLEL_PLAN` (0x55), `STRATEGY_LICENSE` (0x57) and the trading range
(0xB0..=0xBA). Their payloads are read and dispatched while the surcharge they cost
is zero, so gas is a statement about work that does not match the work.
Found while closing TICKET-055, by comparing the surcharge set with
`is_payload_opcode`.
Acceptance criteria: the surcharge is derived from the shared predicate (`payload_len
/ 32` for every payload opcode), or the omissions are argued and written down.
Validation: exact-gas assertions in the VM's tests and the corpus's
`gas remaining` figures updated with the change, plus a test that every payload
opcode's surcharge is proportional to its payload length.

## TICKET-057 — a fixed frame's width is three bytes, or four for `REQUIRE` — CLOSED
Type: CLOSED in `b5d091882` (2026-09-19) · Subsystem: x3-lang/compiler + vm (shared byte format)
Reason: the emitter writes `[opcode][flags][operand]` (three bytes) and pads the
buffer to the next absolute multiple of four, except `REQUIRE`, which writes
`[opcode][flags][threshold u16]` — four bytes of content. Every reader assumed
three (`align4(pc + 3)`; the disassembler used `pc + 4`). The two round to
different boundaries exactly when a frame starts at an offset congruent to one mod
four, which is where the first instruction of a compiler stream without metadata
sits: `align4(1 + 3) == 4` but `align4(1 + 4) == 8`. A program whose first
instruction is a guard was read one byte into the guard's operand and then onto
its padding. Measured on a six-instruction program whose first item is
`risk_policy`: `x3c explain` printed eighteen lines of `UNKNOWN`; the verifier's
boundary set held ten offsets for nine instructions (the padding byte was one of
them); the executor dispatched ten instructions.
Closed: `spec/opcodes.rs` carries `fixed_frame_content_len` and
`fixed_frame_operand`, read by the disassembler, the verifier and the executor. A
three-byte frame in a compiler stream carries only the operand's low byte — the
high byte is the padding, and at offset 1 it was the next instruction's opcode, so
a leading `feature_allow` would have been refused by the VM as feature code
0x5683.
Validation: `x3c explain` now lists the ten instructions in order; the emitter
asserts the walk visits exactly the IR's operations for a guard-first and a
payload-first program and prints no `UNKNOWN`; the VM asserts the verifier's
boundary set equals the emitted operations and the executor dispatches one
instruction per operation. Against the old advance the VM test fails with
`{1, 4, 8, ...}`.

## TICKET-058 — an emitted `if`/`loop` record cannot be walked by any reader — CLOSED (remainder landed as TICKET-106)
Type: CLOSED in `e8aa4ee90` + `bb5737775` (2026-09-20) — a decidable `if` is folded, the taken branch is written, and both are tested; `loop` and an undecidable `if` are still refused · Subsystem: x3-lang/compiler (emitter) + vm
Scope settled (2026-09-20, while closing TICKET-097/098): the ticket's *own* acceptance criteria were
already met by `e8aa4ee90` — a decidable branch is folded and written, and the undecidable class is
refused with its reason — and the spec does not ask for more: PHASE 12's branch construct is
**bounded** (`atomic_choice`, `choose highest_net_output`, "prohibit arbitrary runtime code
mutation"), which is implemented. What is left is the dynamic class, and it is smaller than "a
register machine plus a code generator": a compiler stream already starts every instruction at a
multiple of four and the VM's `IF` already skips whole four-byte units, so the *branch mechanics*
are sound; what is missing is a *value in a register*, and there is no immediate-load instruction
and no arithmetic codegen. The only runtime quantities a program can name are the measured ones,
which the VM compares itself — so the smallest real step is a measured-branch instruction at
version 2, which TICKET-097's gate now makes a stated decision rather than a footnote. Design,
acceptance criteria and validation: **TICKET-106**.
Progress (`e8aa4ee90`): **the feature now exists for a condition the program states.** The
refusal was the fail-closed half; the other half is `lowering::fold_condition`, which decides
comparisons and logical combinations of integer literals — `&&`/`||` with the language's
short-circuit meaning, `!`, and arithmetic through `checked_*`. `Condition::True`/`False` already
existed, so **no opcode and no format change were needed**: the emitter writes the taken branch's
instructions inline, where the writer pads them to the stream's own absolute boundaries, and no
`IF` record is written at all — which is what makes the artifact walkable. Both bodies stay in the
IR, so `x3c lower` shows the branch that did not run beside the one that did.
Measured, with branches of different lengths so which one ran is visible in the artifact:
`if 1 > 0` (true) gives **8** instruction records and `if 1 > 2` (false) gives **9**, one record
apart, matching the branch bodies (one guard vs two). `if steps > 0` is still refused, by both
`check` (`X3E0501: … the condition is not decidable at compile time`) and `emit_x3ir`, and the
refusal reaches both because a `check` that accepted what `build` refuses is the split the
existing test exists to prevent. **Neither an overflow nor a division by zero is rounded into a
decision**: `1 / 0 > 0`, `1 % 0 > 0`, `u128::MAX * 2 > 0`, `u128::MAX + 1 > 0` and `1 - 2 > 0`
are refused rather than folded.
The existing CLI test was **updated, not weakened** — its fixture used `if 1 > 0`, which is now
decided, so it uses `if steps > 0` and keeps testing exactly what it tested before; the new
`cli_folds_a_decidable_branch_and_writes_the_branch_that_runs` is the other half of the rule, and
the pair is what stops either from passing vacuously.
**What is still missing, and why it is a project rather than a patch:** an `if` whose condition is
*not* decidable needs expression codegen into a register — and the compiler emits **no arithmetic
at all**, so this is a register machine plus a code generator, not a record fix. `loop` needs the
same, plus a jump target in stream coordinates that no half of the pipeline computes. The ticket's
own acceptance criteria ("either the emitter pads each branch to the stream's absolute alignment
and the readers learn both record shapes from the shared table, or the language refuses to emit a
nested branch until it can be walked, with the reason in the diagnostic") were already satisfied
by the refusal; what is new is that the *decidable* class is no longer refused.
Also found while doing this and **not** fixed, because it is a different defect: `Statement::While`
lowers with `let _cond_ir = expression_to_condition(cond)?;` — the guard is computed and
**discarded**, so the IR's loop carries `max_iterations: 1000` and no condition. It cannot execute
(`Loop` is refused by the verifier and the emitter), so it is latent rather than live, but the IR
is public and `x3c lower` prints a loop that does not say what it loops on. Recorded as
TICKET-098.
Original entry:
Progress (round 52, `40e8ef44b`): the construct is now **refused** rather than
emitted, so no artifact claims control flow it cannot deliver. Measured before the
refusal, on `strategy TriDexArb { execute { if 1 > 0 { require profit >= 5 } } }`
(a shape the language accepts): `x3c build` wrote 320 bytes, `x3c explain` printed
the condition text as opcodes, `x3c run` failed `X3_VERIFY_FAILED:
OutOfBounds(292)`; the same program without the branch builds, explains and runs.
The IR verifier (so `check` refuses too) and the emitter (because `emit_x3ir` is
public) both refuse, naming the mismatch: the VM branches on a register and skips
whole four-byte instructions, while a compiler stream frames instructions with a
width that varies and pads them to absolute boundaries. CLI test:
`cli_refuses_a_branch_no_reader_could_follow_instead_of_writing_one`.
This is the fail-closed half, **not** the feature. The feature needs: an explicit
jump target in the branch record (a byte offset or a payload frame the reader
rule in `spec/opcodes.rs` can consume), branch bodies emitted inline so their
instructions are padded to the stream rather than to the branch, a VM arm that
sets `pc` to that target, and expression codegen into a register for the
condition — the compiler currently emits no arithmetic at all, so only a
statically decidable condition could be folded without it.
Original entry:
Reason: `Operation::If` is emitted as
`[IF][u16 cond_len][cond][u32 then_len][then][u32 else_len][else]` and `Loop` as
`[LOOP][u16 max_iterations][u32 body_len][body]`. The branch bodies are emitted
into their *own* `Vec` and each nested `emit_operation` pads that vector to a
multiple of four, so an inner instruction's offset is aligned relative to the
branch, not to the stream. The outer record is then written at `pc + 3 + cond_len
+ 4`, which is not a multiple of four, so no reader can find the inner boundaries:
the verifier advances fixed frames by `align4(pc + width)` and would read branch
bytes as opcodes. `is_payload_opcode` also lists neither `IF` nor `LOOP`, so both
are walked as fixed frames. The corpus does not reach this: the four examples that
use `if` (`arb`, `flash`, `jit_lp`, `mev_smooth`) do not parse (TICKET-013/014),
and `compiler/tests/test_ir_verifier.rs` is the only thing that builds the IR.
Acceptance criteria: either the emitter pads each branch to the stream's absolute
alignment and the readers learn both record shapes from the shared table, or the
language refuses to *emit* a nested branch until it can be walked, with the reason
in the diagnostic.
Validation: an emit → walk test whose expectation is the writer's own boundary
set (the shape used for TICKET-057), over a program containing `if`/`else` and
`loop` bodies.

## TICKET-059 — the artifact carries the finality depth its guards were checked against — CLOSED
Type: CLOSED in `b5e7056c9` (2026-09-19) · Subsystem: x3-lang/compiler (IR + emitter)
Closed: the declaration is typed — `Condition::FinalityPolicy { name, requirement, blocks }`
— rather than a rendered `"strict finalized"` string, and the emitter writes the
depth into the `REQUIRE` operand of the `FinalityExplicit` record, so a replayer
holding only the bytecode can re-check the relationship the compiler decided.
The disassembler now prints a fixed frame's operand for `REQUIRE` (the one fixed
frame whose operand is read): `examples/simple_swap.x3` disassembles as
`REQUIRE static 32` for both declared policies and `REQUIRE ge 1` for the nonce
guard, where every guard used to read `REQUIRE`. Two refusals keep the encoding
unambiguous: `blocks 0` (a policy that requires no depth, against a zero operand
that means "none stated") and a depth above `u16::MAX` (the operand's bound,
refused rather than truncated). Tests: the depth in the artifact with and without a
declared depth, plus both refusals.
Original entry:
Reason: `finality_policy` now states `blocks <n>` and `verify_finality_guards_declared`
checks every guard against it (TICKET-049), but the emitted `FinalityExplicit`
record still carries only `"<mode> <requirement>"` as an expression string. A
replayer reading the artifact therefore cannot re-check the guard-versus-declaration
relationship the compiler just decided, and the depth is a declaration only the
source has. Extending the string with `blocks N` was rejected as the fix: a value
nothing parses is a claim that cannot be checked, which is the shape of TICKET-044.
Acceptance criteria: the depth travels in a typed field (an IR variant the emitter
writes into the `REQUIRE` record, or a metadata record), and a test reads it back
out of emitted bytecode.
Validation: emit → decode → the declared depth equals the source's, plus the
existing finality tests.

## TICKET-060 — the Python harness reads 2 of 23 corpus examples — CLOSED
Type: CLOSED in `de690bd29` (2026-09-20) · Subsystem: x3-lang (cli.py and friends)
Closed: **the ticket offered two options — grow the harness to the compiler's language, or retire it
and move the 14 pytest tests to the canonical path — and this closes on neither, so it says why.**
What the ticket asks unconditionally is met: re-measured, of the 19 examples **3 are read, 16 are
refused, and none crashes** — the five `IndexError`s it names were already fixed by TICKET-091, and
`tests/test_surface_drift.py` asserts the no-crash property over the whole directory. What remained is
its validation clause: `for f in examples/*.x3: parse_file(f)` **with the accept/refuse set asserted
per file**. That is now a table in that test, pinning each file to `reads` or to the code it refuses
with — the code being part of the entry, because a refusal for a different reason is a different
boundary — and asserted three ways, each proven load-bearing by mutating what it guards: a changed
outcome fails with the file and both expectations, a new example fails asking to be classified, and a
file in both tables fails on its own.
**Why neither option:** the surface's scope was already *stated* in `cli.py`'s module doc — one
`intent` per file, nine guard kinds of the compiler's eighteen, a stricter address shape — and its
output is a `validated_intent_v1` for the runner and the legacy planner. It is a front-end, not a
compiler: growing its parser to the compiler's language would be a second implementation of a parser
the repository has (the duplicate-work rule), and retiring it would move 14 tests that assert this
front-end's own behaviour, which is still used. The boundary the ticket calls "drift behind the Rust
compiler" is one boundary in three, and `cli.py` names the two that are scope and the one that is not:
`X3_PARSE_RECEIVER` — the address shape, which it calls drift and attributes to **TICKET-091**. That
ticket is the one holding the real remaining work here, and it is open.
Validation as measured: `pytest -q` **22 passed** (21 before) with the two mutations failing as
intended; x3-lang `cargo test --workspace` 1158 passed / 0 failed; clippy and fmt clean; sweep check
19/19, build 19/19, warning-free 19/19, run-artifact 18/19.
Original entry:
Reason: the pytest half of the language's own test plan runs against a second
implementation that has drifted far behind the Rust compiler. Measured after the
leading-declaration fix (which was load-bearing: `arb_solana_eth.x3` starts with
`finality_policy strict {`, so parse_file raised `X3_PARSE_INTENT` on it):
`parse_file` reads 2 of the 23 examples. The rest fail for reasons that are not
about declarations — dotted guards (`require finality.arbitrum >= 32` →
"malformed require"), `fallback` route steps, multi-declaration files
(`expected intent <name>`), and five files that raise `IndexError: list index out
of range` rather than a typed `X3ParseError` (a crash, not a refusal).
Acceptance criteria: the harness either grows to the language the compiler
implements (a test that every example the compiler accepts is readable by the
harness) or it is retired and the 14 pytest tests move to the canonical path.
Either way, a malformed line is refused with an `X3ParseError`, never an index
error.
Validation: `for f in examples/*.x3: parse_file(f)` with the accept/refuse set
asserted per file.

## TICKET-061 — the settlement proof type has no block number, so `tx_hash` supplies one — CLOSED (plumbing)
Type: CLOSED in `1a59b8293` (2026-09-19), with TICKET-063 opened in its place · Subsystem: pallets/x3-settlement-engine
Closed: `SettlementProof` carries `chain_height: Option<u64>` — stated, not derived.
Both verify sites refuse a proof that does not state it, `submit_proof` refuses it
for every chain (the `SettlementProofVerified` event used to *report* the first
eight bytes of `tx_hash` as a block height), and the BTC path refuses a proof whose
stated height disagrees with the header it carries. Tests: a proof with no height is
refused (EVM and SVM); the height reaching the validator is the stated one and
changes when the proof states a different one (the mock validator now records it);
a BTC proof whose stated height disagrees with its header is refused. Against the
pre-fix code the first two fail, the second reporting 10254875657497741951 where
18000000 was stated. The benchmark's proof also gained its second merkle entry,
which the EVM path now requires.
**What this does not fix**: the plumbing was never the protection. See TICKET-063 —
the header check is `LastEvmHeader` (one stored header) and nothing binds the
receipt to it, so a proof that copies the latest header's fields and states its
height still passes. Before this change the attacker additionally had to grind
`tx_hash[0..8]` to the header's block number (2^64 over a receipt they choose); that
was an accident of the derivation, and the fix makes the claim explicit, which is
what a lookup-by-height needs. Stating the height is a prerequisite, not a
mitigation.
Original entry:
Reason: `verify_evm_receipt_proof` derives the EVM block number and
`verify_svm_proof` the SVM slot from the first eight bytes of `proof.tx_hash`
("as proxy", says the comment), and that value is what looks up the canonical
header the proof is checked against. `tx_hash` is proof data, so the prover
chooses the block whose header confirms their own proof. Needs a decision rather
than a patch: either `EthereumProof`/`SolanaProof` gains `block_number`/`slot` and
the validator checks the header it looked up *is* that block, or the validator
trait takes the height from somewhere the prover does not control.
Acceptance criteria: the height the header lookup uses is not proof-supplied, and
a test grinds a `tx_hash` whose first eight bytes name a different block and shows
the proof is refused.
Validation: the new negative test plus the existing settlement suite.
Related: finding 1 (a proof verified against roots it does not carry) is closed in
`77d2c95d4`; see `.ai/reports/settlement-engine-invented-evidence-20260919.md`.

## TICKET-062 — a hand-resolved `Cargo.lock` must be re-checked by cargo — CLOSED
Type: CLOSED in `febd2382f` (2026-09-20) · Subsystem: scripts/local-ci.sh
Closed: `GATES_FAST` gains the check itself rather than a description of it —
`"cargo lockfile locked:cargo metadata --locked --format-version 1"`. Measured **through the script**,
both ways: `PASS cargo lockfile locked 3s` on a consistent lock, and `FAIL cargo lockfile locked 2s`
after adding a dependency to one member's manifest without touching the lock (the same exit 101 a
`--locked` gate reports for the merge that produced the defect — that one was a lock entry cargo
rewrites, this is a missing one, and both are cargo's own check). The command is plain `cargo`, not a
path to a toolchain shim: a gate naming a local path passes on the machine that wrote it and fails
everywhere else.
Original entry:
Type: OPEN, low · Subsystem: repo/CI
Reason: resolving the `Cargo.lock` conflict for the x3-swap-router merge kept the
branch's package entry verbatim, where its sole `sp-std` is the 14.0.0 one. The
merged graph has two (`sp-std` 8.0.0 and 14.0.0), so the bare name is ambiguous
and cargo rewrites it: with the published lock `cargo metadata --locked` exits 101
("the lock file needs to be updated but --locked was passed"), which is what every
`--locked` gate in CI would have reported. Fixed in `0acd2b19a`.
Acceptance criteria: a merge that touches `Cargo.lock` runs a `--locked` cargo
command before the merge is pushed (a check in the merge procedure, or a CI gate
that runs `cargo metadata --locked` on `master`).
Validation: `cargo metadata --locked` on a merge commit whose lock was resolved by
hand.

## TICKET-063 — the EVM/SVM settlement proof is bound to nothing (critical, funds)
Type: **EVМ half CLOSED in `af64a0769`**; SVM half open · Subsystem: pallets/x3-settlement-engine + pallets/cross-chain-validator + runtime
Step 1 (EVM) landed in `af64a0769`: `SettlementProof` carries the receipt's
`receipt_index` and its `trie_proof`, and `verify_evm_receipt_proof` walks the
receipt to the declared receipts root with `x3-verification-router`'s verifier —
the workspace's one MPT implementation, shared with the relayer, and usable by a
`no_std` pallet. Three new tests pin the four ways the binding fails (no path, no
index, a tampered node, a path walked under another key or against a different
receipt); with the walk disabled the tampered-node and wrong-key tests fail, i.e.
those forgeries were accepted before. The refusal flag narrowed to
`AllowUnboundSvmProofs`: EVM no longer consults it, SVM still does, because nothing
proves an SVM transaction is *in* the attested slot.
**(b) CLOSED in `922fbde16`** — declaring in `pallets/cross-chain-validator` that the
stored `merkle_root` must be the *receipts* root for the settlement path. Reading both
implementations gives the reason to state precisely: they validate **two different
structures over the same 32 bytes**. The validator pallet checks
`merkle_root_of(proof_to_leaves(proof)) == merkle_root` — a **flat** Merkle tree over
32-byte leaves, with no RLP, no keccak and no header anywhere in the pallet, so
`block_hash` and `state_root` are assertions checked only for being non-zero. The
settlement pallet walks an **MPT** receipt proof against that same stored root. A submitter
who attests a block's leaf-Merkle root therefore stores a perfectly valid root and makes
that block impossible to settle, and nothing complains: the shape is valid, the root is
stored, and only the walk refuses — **liveness, not theft**, which is why it is stated
rather than enforced, since the meaning of the root is the submitter's claim. Stated at the
four points a reader acts on it (the validator module doc, `validate_evm_header`,
`EvmHeaderInfo::merkle_root`, and `verify_evm_receipt_proof` on the consuming side).
What remains: (a) the SVM binding (a bank-hash / account-state proof — the router's
`SolanaFinalizedVerifier` is an attestation mechanism, a different shape); the chain
currently fails closed (`AllowUnboundSvmProofs = false` in `runtime/src/lib.rs`);
(c) optionally, step 2:
`verify_settlement_evm_header` compares against `LastEvmHeader` (one header) while
`EvmMerkleRoots` stores a root per height — a per-height lookup would let an older
block settle, and is a liveness improvement rather than a security one, since the
single-header comparison is the stricter of the two — i.e. it **loosens** the check, so it
is not taken without a requirement behind it.
(d) the test that would make (b) executable: a runtime-level case where the attested root is
a flat-Merkle root and the receipt walk is refused. It needs the chain runtime's test setup,
because the settlement pallet's own mock does not include `pallet-cross-chain-validator` —
the pallet is not even a dependency of it. A real unit of work, recorded rather than
half-built.
Original entry:
Posture (round 53, `fa478f2b9`): the acceptance was implicit before — the chain
accepted a proof whose receipt nothing bound to the header. `Config::AllowUnboundExternalProofs`
(a `Get<bool>`, default off) makes the runtime *state* whether its validator does the
binding. The chain sets it `false` (`spec_version` 11), so an EVM/SVM proof is now
refused with a reason — "nothing binds this receipt to the header … BTC SPV proofs
are unaffected — theirs is verified" — instead of being reported as merely invalid;
the test runtime sets it `true` because `RecordingCrossChainValidator` stands in for
a binding validator and the lifecycle tests would otherwise be unreachable.
Evidence: `cargo test -p x3-chain-runtime --lib settlement_proof` (a proof of exactly
the shape the pallet accepts, refused by the chain, plus the constant asserted) and
the pallet's 120 + 23 tests. **This is containment, not the fix**: until step 1 below
exists, the chain cannot settle an external leg at all.
Reason: the path checks that `keccak(receipt_data) == tx_hash`, that the proof
carries two roots, and that `{chain_height, block_hash, state_root, merkle_root}`
equal the stored header's fields — via
`pallet_cross_chain_validator::verify_settlement_evm_header`, which compares against
`LastEvmHeader`, a `StorageValue` holding **one** header. Nothing walks
`merkle_proof` as a path (its first two `H256` entries are read as the roots), so no
receipt-trie proof is ever verified and the receipt is not bound to the header. The
proof also carries no asset, amount or recipient, so it does not bind to the intent
either. Anyone can therefore settle a leg by copying the latest validated header's
public fields into a proof and pairing them with any structurally valid receipt RLP
whose keccak they use as `tx_hash`. `runtime/src/lib.rs::RuntimeCrossChainValidator`
wires the real pallet, so this is runtime behaviour, not a mock's. Full write-up
with the code paths, the honest note about the 2^64 grind the previous derivation
imposed, and the ordered fix: `.ai/reports/settlement-engine-invented-evidence-20260919.md`
(finding 3).
Acceptance criteria: a forged proof (valid receipt for another transaction, latest
header's four fields copied, height stated correctly) is refused; a genuine proof (a
receipt in the block the header is for, with its MPT path) is accepted; a regression
test asserts the four header fields alone are insufficient. Ordered fix: (1) bind
the receipt to the header — MPT path from `keccak(receipt)` to the receipts root,
which needs `merkle_proof` to carry RLP node bytes; (2) look the header up *by
height* rather than comparing against the latest; (3) carry what is being settled in
the proof and compare it to the intent.
Validation: the two tests above plus `cargo test -p pallet-x3-settlement-engine`
and a runtime-level test through `RuntimeCrossChainValidator`.

## TICKET-064 — the canonical EVM receipt verifier could not verify any real proof — CLOSED
Type: CLOSED in `ebc22fa47` (2026-09-19) · Subsystem: crates/x3-verification-router (+ crates/x3-relayer, which registers it)
Reason: `ProductionEvmReceiptVerifier` is what the relayer registers for
`VerificationStrategy::EvmReceiptProof`, and four independent defects meant it
rejected every proof it was ever handed:
1. `receipt_trie_key` built `rlp([index])` — a list — where the receipts trie's key
   is `rlp(index)`, the RLP of the integer (0 → `0x80`, 1 → `0x01`). The function's
   own doc comment says `rlp(index)`, and `x3-lang/vm/src/bridge.rs`'s verifier and
   fixture use the raw encoding (key `0x01` for index 1): two implementations in one
   repository disagreed and this was the wrong one.
2. `EvmBlockHeader::decode` read state root, receipts root and logs bloom from
   indices 1/2/3 — the abbreviated field list in its own doc comment — instead of
   the yellow-paper 3/5/6, so `receipts_root` was the beneficiary address.
3. `decode_u64` left-aligned short big-endian values: a header field of `100`
   decoded as `0x6400000000000000`, and the confirmations check
   (`current_block_number - number`) saturated to zero.
4. The inclusion walk was given `None` as the leaf value, and every successful path
   in `verify_merkle_patricia_proof` ends in `Some(stored) == value`.
   `DecodedProof` now keeps `receipt_rlp` and passes `Some` of it.
Why nothing noticed: every test of the verifier asserted a *failure* (short
payload, wrong chain, undecodable receipt, wrong destination chain) — the merkle
walk and the key convention had no positive case at all, so all four defects could
coexist with a green suite. The MPT layer `verify_merkle_patricia_proof` is a real
implementation (keccak-checked nodes, branch/extension/leaf, compact paths).
Closed: one key builder (`rlp_index_key`, used by the verifier and by
`receipt_trie_key`), the header's real indices, right-aligned big-endian decode, and
the value bound into the walk.
Validation: `a_proof_built_with_the_standard_key_verifies` (the verifier's first
positive case, built from the standard convention rather than from the helper, so it
cannot agree with bug 1) and `the_receipt_trie_key_is_the_rlp_of_the_index`
(`0x80` / `0x01` / `0x81 0x80` / leading zeros / empty). Restoring the list-wrapped
key fails the positive test; before fix 3 it failed `InsufficientConfirmations`,
before fix 4 `InclusionFailed`. Router 23 + 37 tests and the relayer's 5 pass;
clippy and fmt clean.

## TICKET-065 — the third EVM receipt "verifier" claims verification it does not do — CLOSED
Type: CLOSED in `bebbe55cf` (2026-09-19) · Subsystem: crates/x3-crosschain-intent/src/proof/evm.rs
Closed: `verify_evm_receipt_proof` no longer computes a root of its own. It takes
`receipts_root` and `trie_proof` and delegates the inclusion proof to
`x3_verification_router::evm_receipt::verify_merkle_patricia_proof` — the canonical
verifier, the one the relayer is wired to — using
`evm_receipt::receipt_trie_key(receipt_index)` as the key, so this crate cannot
reintroduce the list-wrapped key TICKET-064 found in the router. Both acceptance
paths the entry named were available; delegation was chosen because the RLP decode
and the expected-log matching around it are real and were worth keeping.
`compute_receipt_trie_root` is deleted (it ignored `_index` and `_total_receipts` and
returned `keccak256(rlp(receipt))` — the hash of the caller's own bytes), and
`rlp_encode_bytes` / `rlp_encode_list` are `#[cfg(test)]` since their last lib caller
was that root. The fabricated fields are `Option`s — `block_hash`, `tx_hash`,
`confirmations` — with `with_header_attestation` recording a chain view as an
*attestation* rather than a conclusion of the walk, and `require_confirmations`
refusing with `NoHeaderAttestation` when there is nothing to measure against instead
of reading the old default of `1` as one confirmation. `TrieRootMismatch`,
`ReceiptHashMismatch` and `InvalidReceiptIndex` are gone (none was ever constructed);
the walk reports `NotIncluded`, which is. `proof/mod.rs`'s `ProofVerifier` claim is
replaced by who calls what — nothing in the workspace does, and the trait of that name
is in `x3-orchestrator`, which does not depend on this crate.
**Proof**: the tests now build a real one-leaf receipts trie from the standard
encoding (key `rlp(index)`, node `rlp([compact_leaf_path(nibbles(key)), receipt_rlp])`,
proof the RLP list of node byte strings) rather than from anything the verifier
exposes, so a green positive test means the delegation works end to end. Four
failures that could not previously be expressed are now tests: a tampered node, the
receipt proved against a root holding it at another index, a root holding a different
receipt, and an empty proof list. `verify_valid_receipt` no longer asserts success
against `block_hash = [0xab; 32]`, and the dead `keccak256_produces_32_bytes` (which
asserted only "not all zeros" against a `Verified:` comment whose value was the digest
of nothing) is a known-answer test of `keccak256("")`. `cargo test -p
x3-crosschain-intent` 74 + 40 + 5 passed / 0 failed; clippy `--all-targets -D warnings`
and `fmt --check` clean; `cargo check -p pallet-x3-settlement-engine` still builds.
Residual: the dependency makes the settlement pallet's tree larger, and the
wasm32v1-none build of that pallet was not exercised here (see TICKET-082).
Original entry:

Reason: the repository contains four EVM receipt implementations, and the two that
are *not* the canonical one both overstate what they do:
- `crates/x3-verification-router/src/evm_receipt.rs` — the real one (MPT walk against
  a header's receipts root), wired to the relayer, fixed in `ebc22fa47`;
- `x3-lang/vm/src/bridge.rs` — a second real MPT walk, wired to x3-lang programs;
- `pallets/x3-settlement-engine` — reads `merkle_proof[0..2]` as roots and never
  walks a path (TICKET-063);
- `crates/x3-crosschain-intent/src/proof/evm.rs` — `verify_evm_receipt_proof` decodes
  a receipt, checks expected logs, and returns an `EvmReceiptProof` whose
  `block_hash` is copied from the caller, whose `tx_hash` is `[0u8; 32]` and whose
  `confirmations` is a hardcoded `1`; `compute_receipt_trie_root` ignores its `index`
  and `_total_receipts` parameters and returns `keccak(rlp_bytes(receipt_rlp))`,
  which is not a trie root. It has **no callers** outside its own tests, though the
  module doc says "the intent compiler's `VerifyProof` instruction calls into this
  module through the `ProofVerifier` trait" — that trait lives in
  `x3-orchestrator` and is implemented only by a mock.
Acceptance criteria: either wire it to the real verifier (delegate to
`x3_verification_router::evm_receipt::verify_merkle_patricia_proof` with the
receipts root and the trie nodes, keeping the receipt RLP as the value) or delete it
and point callers at the canonical one; the module doc states which verifiers are
wired to what; the fabricated fields become `Option`s so a zeroed hash cannot read as
a real one.
Validation: a proof built with the standard key/Root verifies through it, a tampered
node does not, and `grep` shows the doc claim matches the call graph.

## TICKET-067 — `Statement::Swap.route` holds the amount, not a route — CLOSED
Type: CLOSED in `9be554705` (2026-09-19) · Subsystem: x3-lang/crates/x3-ast + parser + lowering
Closed: the field is `amount`, with `#[serde(default, alias = "route")]` so an AST
serialized before the rename still loads. The three readers that already treated it
as the amount (the lowering's `input_amount`, the formatter's ` amount …` clause, the
profitability check) now read a field named for what it holds, and the body-level
swap parser's two-identical-branches `let _route_expr` dance is gone (it consumes the
arrow and says why no amount is read there: that shape takes the amount from the
matching endpoint). Four tests in `compiler/tests/test_swap_amount.rs` pin which
quantity comes from where.
Original entry:

Reason: `parse_swap_step` stores the step's `amount <expr>` in the AST field named `route`
(`route: amount`), so every reader of that field is reading a field whose name says
something else. `profitability.rs` reads it as the amount with the mistake named in a
comment; the lowering reads the amount from the matching `from` endpoint instead (the
bridge step's parser has a note saying so), which means the same quantity has two
sources depending on the step shape.
Acceptance criteria: one field name per quantity — either `Statement::Swap` gains
`amount` and `route` keeps a route spec (or is removed), or the field is renamed to
`amount` with the parser and every reader updated; the intent bridge's JSON mapping is
checked for the same field. `#[serde(default)]` where a stored AST may carry the old
name.
Validation: `cargo test --workspace`, plus a test that a swap step written with
`amount 1_000 min_output 2_000` lowers to a lock of 1_000 and a minimum output of
2_000 (both from the step, not from the endpoint).

## TICKET-068 — a hedge leg needs a venue adapter before its artifact can exist — CLOSED
Type: CLOSED in `cfedbbe43` (2026-09-20) · Subsystem: x3-lang/vm (+ a host adapter)
Closed: **the runtime reports the delta, so the bound is a post-condition on the trade.** The
measured-reply format already carried an explicit unit byte, so the third measured quantity needed
no new concept: `MEASURED_UNIT_DELTA_BPS` (3) is what a venue's reply to `venue_order` carries, and
a **unit code in bits 5-7** of the `REQUIRE` flags byte names which quantity a measured guard
compares — the comparison-mode field is two bits and its four values were spent before a third
measured quantity existed. **The profit's code is zero**, which is what every artifact emitted
before the field existed carries, so nothing already written changes meaning and no format version
moved; `x3c run --measured-delta-bps <n>` states what the venue did.
Measured:
```
$ x3c explain hedge.x3b
  0004  0x40  REQUIRE measured delta 1        (was: REQUIRE static 0)

delta = 1   -> x3c run: ok
delta = 250 -> X3_DELTA_ABOVE_BOUND: the venue left a delta of 250bps and the program allows
               at most 1bps
no delta    -> X3_GUARD_UNMEASURED: the guard `delta <= 1bps` needs a delta the venue measured,
               and no venue reported one for this hedge
```
The ticket's own validation — "a hedge whose runtime delta exceeds its bound is refused at the
guard rather than at compile time" — is the second line. The disassembler printed `REQUIRE ? 1` for
*any* measured guard before this; it names the quantity now.
**Two defects avoided on the way, both found by reading.** (1) A delta guard shares the profit's
mode, so the simulation's floor reader classified it as a `profit_floor_bps` — a floor compared
against a delta. It refuses an artifact stating a delta bound, naming why, rather than misreading it
(the units mismatch the measured modes exist to prevent) or skipping it (a verdict as if the
artifact had no bound) → TICKET-099. (2) The first attempt flipped the **liquidation** floor to
`measured: true`, because `lowering.rs` holds two `Require` blocks with byte-identical shape;
caught by running `x3c lower` and reading `"measured": false` where it should have been true.
Liquidation is back to a constraint with its reason, and the two are keyed on their distinct
subjects now.
Tests: `test_measured_delta.rs` (a stated delta arrives on the call the hedge made; an unstated one
arrives as nothing, not zero; **a plan's profit and slippage do not answer a hedge's delta guard**,
which is what makes the unit code load-bearing; the code round-trips and zero still means the
profit); a verifier test refusing an unknown quantity and a unit code on a mode that already names
its quantity; and `cli_lowers_a_hedge_to_venue_orders_and_runs_it` **updated, not weakened** — it
asserted the hedge ran, which is no longer the whole truth, so it now asserts the readable guard,
the unmeasured refusal, a delta at the bound settling, and 250-against-1 refused with both figures.
Original entry:
Landed: a hedge now lowers to **venue orders** and runs. `spec::opcodes::VENUE_ORDER` with a
`CapabilityPayload::VenueOrder { action, subject, asset, quantity }`, an `Operation::VenueOrder`
whose actions come from a vocabulary the compiler owns (`spot_buy`, `spot_sell`, `perp_long`,
`perp_short`), `BridgeAdapter::venue_order` with the dry-run adapter answering what it was asked
and the unconfigured adapter refusing by name, and `hedge::orders` resolving each leg's size the
same way `hedge::exposure` resolves the net — so the plan cannot disagree with the delta that was
checked. `x3c check`, `build`, `explain` and `run` all succeed on a hedge.

Still open, and this is the half the ticket was really about: **the runtime does not report the
delta back.** The artifact's bound is a *constraint* (`measured: false`) on the delta the
compiler computed from the declared legs, so a venue that filled something other than what was
asked is not caught at run time. Closing it needs the order's reply to carry a measured delta and
the guard to be a *post-condition* — which needs a fourth measured comparison mode, and the mode
field is full: `REQUIRE_COMPARE_MASK` is two bits and `STATIC`, `GE`,
`MEASURED_PROFIT` and `MEASURED_SLIPPAGE` take all four values. Bits 5-7 of the flags byte are
free (`require_flags` uses 0-1 for the mode and 2-4 for the guard operator), so a unit code
there is the natural shape — one mode, a unit in the high bits — and `MEASURED_UNIT_DELTA_BPS`
is the new unit. Nothing else about the mechanism changes.
Original entry:
Reason: PHASE 9's exposure reasoning is implemented and tested, and
`Operation::Hedge` carries the decided net, but no artifact can be emitted: a perp leg
is a position on a venue the VM has no adapter for, so the IR verifier and the emitter
both refuse with that reason. That is the honest end of the phase (the exposure is
decided, the execution is not pretended) and it is also the reason a hedge program
cannot be run end to end.
Acceptance criteria: a `TradingHost`-shaped adapter that can open a perp position
(long or short, with a size and a venue) and reports it back, so `Operation::Hedge`
lowers to the same execution path a swap leg uses; the delta the runtime reports must
be the delta the compiler decided, or the run fails.
Validation: a hedge program builds, runs against the fixture host, and a hedge whose
runtime delta exceeds its bound is refused at the guard rather than at compile time.

## TICKET-069 — a liquidation's calls need a lending-protocol adapter — CLOSED
Type: CLOSED in `9a5438ef5` + `ce8961990` (2026-09-20) · Subsystem: x3-lang/vm (+ a host adapter)
Closed: the calls lower to venue orders and run (`9a5438ef5`), and the remaining half — the runtime
never reported what was seized, so the net-profit floor was a compile-time constraint — is closed in
`ce8961990` by reporting the net on the reply to those same venue orders and making the floor a
post-condition (TICKET-100, which carries the measurement, the correction to the recommendation this
ticket would otherwise have inherited, and the CLI rule that had to move with it).
Original entry:
Landed: a liquidation lowers to a plan and runs. The two calls are **venue orders**
(`liquidate` and `receive_collateral`, each carrying the position it is about, the asset and
the quantity), the conversion is a `Swap` whose amounts the declaration itself states — which
is what this phase has that arbitrage did not: no price is needed — and the net-profit floor
travels as a guard. `check`, `build`, `explain` and `run` all succeed. The compile-time half of
the ticket's own acceptance criterion is met: `liquidation::verify` refuses a swap whose
minimum cannot repay the capital, with the shortfall, before any of this is reached.
Still open: **the runtime does not report what was actually seized.** The floor is therefore a
*constraint* (`measured: false`) derived from the declared `min_output`, not a post-condition on
the realised net — because the conversion is a `Swap`, an asset-op *record* that never reaches
the host, so no reply could carry a measurement. Closing it needs two things: the asset-op swap
path to report an output (or the conversion to be a capability call), and then the same
measured-delta/measured-net comparison mode TICKET-068's remainder needs — the mode field is
full, and bits 5-7 of the `REQUIRE` flags byte are the natural place for a unit code.
Original entry:
Reason: PHASE 10's accounting is implemented and tested, and `Operation::Liquidation`
carries the decided ledger, but no artifact can be emitted: `liquidate` and `receive`
are calls into a lending protocol the VM has no adapter for, so the IR verifier and
the emitter both refuse with that reason. Same shape as TICKET-068 (the perp venue a
hedge leg needs).
Acceptance criteria: a host adapter that can liquidate a named position and receive
its collateral, reporting the collateral actually seized, so `Operation::Liquidation`
executes through the same path a swap leg does; the runtime must fail closed when the
seized collateral is less than the swap's input, rather than swapping what it did not
receive.
Validation: a liquidation program builds, runs against the fixture host, and a
liquidation whose seized collateral is short is refused at run time with the figures.

## TICKET-070 — a rebalance's target portfolio has no transaction graph to reach it — CLOSED
Type: CLOSED in `9955201ec` + `89749ad29` (2026-09-20) · Subsystem: x3-lang/compiler (+ vm execution)
Closed: the language can state what the portfolio holds, so the artifact carries **both ends** of the
move — the input the target alone lacked. `rebalance <name> { holds { chain.ASSET = <n>; } … }`,
carried in the IR, in the `REBALANCE_TARGET` record and in the host order
(`portfolio\x1fholdings\x1fweights\x1fcriterion`), with the payload's field **appended** so a
record written before it ends early and is refused as short rather than misread. Measured:
```
$ x3c explain portfolio.x3b
  0001  0x9e  REBALANCE_TARGET  portfolio <holdings> <weights> fees
```
An empty holding list is **"stated none"**, which is a different fact from "holds nothing" — the
program is what differs, and a test asserts both readings. `rebalance::portfolio` refuses a holding
stated twice (the same defect as a weight stated twice: the second would replace the first), the
parser refuses a name or a fractional amount (a compiler that cannot see the asset's decimals cannot
scale one), and `x3c fmt` round-trips the clause and does not invent an empty `holds { }` for a
program that wrote none — without that, reformatting would delete the input the trades need
(TICKET-090's defect in the other direction). `#[serde(default)]` keeps an AST stored before the
clause loadable.
**Residual, stated rather than folded into the closure:** the compiler still does not *generate* the
trade graph, and cannot from these two ends alone — a target weight is a share of the portfolio's
**value** and a holding is an **amount**, so the trades need prices the graph does not hold. The
artifact now gives a host both ends and the host prices them, which is the division of labour
`arb.rs` and `hyperarb.rs` already document and the ticket's own acceptance criteria describe
("amounts per asset, which a host then prices"). The phase's sentence says the graph generation is
"eventually"; what was missing from the language is no longer missing.
Validation as measured: x3-lang `cargo test --workspace` **1150 passed / 0 failed**; clippy and fmt
clean; pytest 21; sweep check 19/19, build 19/19, warning-free 19/19, run-artifact 18/19; and the
three surfaces the ticket names (`check`/`build`/`explain`/`run` plus `fmt`) exercised in
`test_rebalance_holdings.rs` and the CLI's rebalance test.
Original entry:
Landed: the phase no longer refuses. The target portfolio and the criterion it was ranked by
travel in the artifact as `REBALANCE_TARGET`, an instruction `BridgeAdapter::rebalance_target`
answers (dry-run echoes it, unconfigured and production refuse by name), and `check`, `build`,
`explain` and `run` all succeed.
Still open, and it is arguably the phase's honest end rather than missing work: the **trades**
that reach the target are not generated, because every one of them depends on the account's
*current* portfolio, and a compiler has no state. The phase's own sentence says the graph
generation is "eventually", so what is missing is not effort but an input the language does not
carry. Two ways to close it, both language decisions rather than compiler work: let a program
*state* its current holdings (amounts per asset, which a host then prices), or define a
compile-time interface for holdings the compiler may read. Until one of those exists, this
ticket's remaining half is the host's, and the artifact says so.
Original entry:
Reason: PHASE 11's weights are decided (`compiler/src/rebalance.rs`: at least two assets,
no zero weight, no duplicate, summing to exactly 100%, every `minimize` target rankable
by the optimizer) and the decided portfolio travels in `Operation::Rebalance`. The
phase's own last sentence says the compiler should generate the transaction graph
"automatically" and that part is not implemented, so the IR verifier and the emitter
both refuse the operation rather than emit a plan that does nothing. A rebalance
therefore cannot be executed or even built today.
Acceptance criteria: `rebalance` lowers to a real graph of operations that moves the
current portfolio to the declared weights — the weights must be satisfied by the legs
the plan emits, the criterion must be the one the plan is ranked by, and a plan whose
legs cannot reach the declaration must be refused with the weights it reaches instead.
The `minimize` set beyond the first target is recorded today and must either be acted
on or kept explicitly ranked-by-one, as `rebalance.rs` already documents.
Validation: a rebalance program builds, runs against the fixture host, and the resulting
positions equal the declared weights within the program's own rounding rule; the
`check`-refuses-it test in `compiler/tests/test_ir_verifier.rs` is replaced by one that
asserts the emitted legs move the portfolio to target.

## TICKET-071 — a netting book reduces to a residual nothing can settle — CLOSED
Type: CLOSED in `fa12e602d` (2026-09-19) · Subsystem: x3-lang/vm (+ a host adapter)
Closed: a book binds each party to an account (`account <party> = <address>;`, refused for an
unbound party, a bind with no obligation behind it, a party bound twice and an empty address)
and lowers to one atomic route per residual transfer — a `Lock` from the debtor's account and a
`Release` to the creditor's — so `x3c check`, `build`, `explain` and `run` all succeed. Two
checks had to be corrected first and neither correction weakens them: `no_double_claim` counted
`Release` operations program-wide while its own description said "for the same lock" (its
sibling `no_double_refund` was fixed for exactly this, and the `release_lock` helper already
existed), and `verify_refund_path_exists` demanded a handler for any `Lock`, which was
*contradictory* because a handler refunding an escrow the same route claims is refused by
`no_refund_after_claim` — it now also accepts the atomic rollback for a lock claimed in its own
route, with bridges and swaps still requiring an explicit handler. What is not expressed is
settlement of the whole residual set as one unit (TICKET-080).
Original entry:
Reason: PHASE 22's offsets are decided (`compiler/src/netting.rs`): obligations are
offset one `(domain, asset)` group at a time, unlike assets and unlike domains are never
combined, every party must have consented, and the analysis refuses to complete if
offsetting would move any party's net position. The residual travels in
`Operation::Netting`. What is missing is a settler: a party in a book is a symbol
(`alice`) rather than an account, so there is no balance for the VM to debit, and the IR
verifier and the emitter both refuse the operation. `x3c netting` reports the offsets;
nothing executes them.
Acceptance criteria: a party-to-account binding that the analysis can resolve (declared
once, refusal if a party in a book has no account, refusal if one name binds to two
accounts), so `Operation::Netting` lowers to the same asset operations a swap leg uses;
the runtime must fail closed if any residual transfer cannot be made, rather than
partially settling a book whose whole point is that the residual is equivalent to the
obligations.
Validation: a netting program builds and runs against the fixture host; each party's
post-run balance differs from its pre-run balance by exactly the net position the
compiler reported; a book with an unbound party is refused with that party named.

## TICKET-072 — `Lock.from` is the payer to one producer and the payee to another — CLOSED (not a defect)
Type: CLOSED in `cc6ea6eb2` (2026-09-20) — **the reported disagreement does not exist** · Subsystem: x3-lang/parser + lowering + common
Closed: the three sites agree, and the ticket's reading of the parser's value as the *payee* is
wrong. An intent's endpoints each name the account on **their own** chain — `typechecker.py`
validates `from.receiver` against `from.chain` and `to.receiver` against `to.chain` — so the from
endpoint's account is the account whose funds are escrowed: the payer. Measured, on an intent whose
two endpoints name different accounts:
```
from ethereum.USDC amount 1_000_000 receiver 0xA1
to   ethereum.USDC receiver 0xA2

Lock    { chain: "ethereum", asset: "USDC", amount: 1000000, from: "0xA1" }
Release { chain: "ethereum", asset: "USDC", to: "0xA2" }
```
0xA1 is the *from* endpoint's account and lands on the lock; 0xA2 is the *to* endpoint's account and
lands on the release. The corpus writes them as different values in every intent that states both
(`arb_scope.x3`, `arb_solana_eth.x3`, `intent_fusion.x3`), which is what makes the two sides
distinguishable at all — a test using one address for both could not tell them apart, which is why
`test_intent_endpoints.rs` uses two.
**The third site is not a semantic claim.** `verifier.rs` binds `Lock { from }` and `Mint { to: from }`
in one match arm to share a validation body — non-empty chain, asset, account, non-zero amount. It
does not assert that a lock's account is a destination; reading the binding as one is what the
ticket did, and it is worth saying plainly because a ticket that misreads a match arm's *reason*
reads as evidence.
**Residual, recorded rather than changed:** the same quantity has two spellings — a concrete address
(an intent's endpoint receiver) and the keyword `"sender"` (the atomic-swap path, where the payer is
whoever submitted the trade). Resolving the keyword is a host's job. That is now stated on the
payload field rather than inferable only from the two writers. No type, encoding or behaviour
changed, so no artifact changed.
Validation as measured: `cargo test --workspace` **1144 passed / 0 failed** (1142 before), clippy and
fmt clean, pytest 21, sweep check 19/19, build 19/19, warning-free 19/19, run-artifact 18/19.
Original entry:
Reason: three places disagree about what the `from` field of a lock means, and a host
adapter that debits it would debit the wrong party.
- `compiler/src/lowering.rs:452` — the atomic-swap path sets `from: "sender"` with the
  comment "Lock on source chain: funds come from the sender, not the receiver", i.e. the
  field is the **payer**.
- `compiler/src/parser.rs:2039` (`parse_intent_endpoint`, `is_from = true`) — `from
  ethereum.ETH amount 10 receiver 0xA1` sets `Statement::Lock.from = receiver_expr`, i.e.
  the field is the **payee**. Verified by probe: that line lowers to
  `Lock { chain: "ethereum", asset: "ETH", amount: 10, from: "0xA1" }`.
- `vm/src/verifier.rs:206` unifies `AssetOpPayload::Lock { .., from }` with
  `Mint { .., to: from }` in one match arm, i.e. it reads the field the way the intent
  path writes it (a destination), which makes the atomic-swap path the odd one out.
The VM reads only `amount` today (`vm/src/executor.rs:1005`), so nothing is mis-debited
yet; the payload is carried to the host, which is exactly the adapter TICKET-068 and
TICKET-069 need. Latent, not live.
Acceptance criteria: one meaning per quantity. Either `Statement::Lock` gains a distinct
field for the payee (and `from` stays the payer, matching lowering.rs:452 and the
`Statement::Lock` source form `lock CHAIN.ASSET amount V from ADDR`), or `from` is
renamed to say it is the destination and the atomic-swap path, the verifier's match arm
and the intent parser are all updated. `AssetOpPayload::Lock`'s field must then be named
for the same meaning, and `#[serde(default)]`/`alias` used where a stored AST or a
serialized payload may carry the old name (the TICKET-067 precedent).
Validation: a test that an intent written `from ethereum.ETH amount 10 receiver 0xA1`
produces a lock whose payer is the intent's sender and whose payee is `0xA1`; a test that
the atomic-swap path and the intent path agree about which field holds which; `cargo
test --workspace` green.

## TICKET-073 — an arb scope has no pipeline that turns it into legs — CLOSED
Type: CLOSED in `8e53a3e84` (2026-09-19) · Subsystem: x3-lang/compiler (+ vm execution)
Closed: `arb::plan` owns the two stages the table used to report as missing. An `arb`
scope lowers to an atomic block holding the asset cycle the search found, the venues the
compiler approved (`RouteFallback`) and its floors as runtime guards, and `arb::missing_stages()`
is empty. The generator plans a *route* rather than a profitable cycle, because the graph
holds venue attributes and no prices — so the profit floor is a runtime guard and the
per-hop outputs are the host's, and nothing is invented to fill either gap. `x3c check`,
`build`, `explain` and `run` all succeed on a planned scope.
Original entry:
Reason: PHASE 37's scope and risk policy are decided (`compiler/src/arb.rs`): the chain
list, hop bound, liquidity floor, capital ceiling and the three bps bounds are all
validated with figures, and the declared profit floor is answered against the program's
own `require profit` guards. What is missing is the phase's own pipeline.
`arb::STAGES` maps it stage by stage and `arb::missing_stages()` reports the two with no
implementation — "Execution Plan" and "Atomic Settlement" — and both the IR verifier and
the emitter refuse `Operation::Arb` quoting them. The five stages that do exist
(`compiler/src/opportunity.rs`, `compiler/src/optimizer.rs`, `compiler/src/objective.rs`,
`compiler/src/dag.rs` with `Operation::ParallelPlan`, and
`compiler/src/profitability.rs`) are wired to other constructs, not to an `arb` scope:
nothing reads the declaration and produces legs.
Acceptance criteria: an `arb` declaration lowers to a real execution plan over the
existing graph/search/objective machinery — the plan's hops must respect `chains`,
`max_hops` and `liquidity_min`; the plan must be refused when no candidate route clears
`min_profit` after `max_slippage` and `max_total_fee`; the plan must carry the atomic
settlement boundary PHASE 16 already implements for parallel legs; and the declared
deadline must bound the plan rather than being recorded. Wiring the existing five stages
is required; a second graph or search is a defect.
Validation: an `arb` program builds and runs against the fixture host; a scope whose
`chains` exclude the only profitable route is refused with the scope and the route
named; a plan that cannot clear the declared floor after the declared ceilings is
refused with the figures.

## TICKET-074 — an opportunity packet cannot say whether the opportunity is real — CLOSED
Type: CLOSED in `faf3115e8` (2026-09-20) · Subsystem: x3-lang/vm/opportunity_packet.rs + x3c
Closed: the vocabulary is closed and every name has a check a host can satisfy, against the packet's
own terms rather than against another field of the evidence. `state_root_freshness` (the block each
domain's root was read at, the block the verifier is at, the age permitted — compared with
`state_roots`), `venue_price_attestation` (each venue's liquidity and fee — compared with the route's
`min_liquidity` and `fee_bps`) and `strategy_commitment` (a trusted key's signature over a commitment —
compared with `execution_commitment`, verified against the **trusted set**, never the key the
attestation carried). `PacketEvidence` is the data-in/verdict-out input and `--evidence <file>` the CLI's
way to state it; `PacketVerification.checked` is what a successful verify reports, so "verified" cannot
mean a packet that declared nothing and one whose three requirements held.
Measured before and after, on a packet declaring two requirements:
```
BEFORE  x3c packet verify packet.json --block 100 --trusted …   → packet verified: strategy 'cli-tri-arb' …
AFTER   (no --evidence)  x3c: the packet requires `state_root_freshness`,
                          `venue_price_attestation` and no `--evidence <file>` was given, so none of
                          them could be checked …
AFTER   (--evidence)     packet verified: strategy 'cli-tri-arb', 2 venue(s), expires at block 500
                          requirements checked: state_root_freshness, venue_price_attestation
```
**The venue-price check is stated, not invented**: a per-leg *price* would need per-leg amounts the
packet does not carry and a scaling this module chose would compare two different things (the defect
TICKET-068 named one instruction over), so the attestation is per-venue liquidity and fee and the
comparison is figure for figure. The vocabulary is enforced in `validate_proof_requirements`, so every
entry point refuses a name nothing can check, while `verify_packet` stays the *form* check an operator
runs with no host evidence — a split stated on both functions.
Four fixtures translated rather than worked around (`vm/src/opportunity_packet.rs`'s two, the VM test
file's, and the CLI's, which now declares none because the form/signature/expiry test is about the
form). Two mistakes the fixtures caught: editing requirements into the JSON of an **already-signed**
packet refuses on `ExecutionCommitmentMismatch` (they are part of the terms the commitment covers), so
the fixture is parameterized and signs after setting them; and a blind `.replace("[]")` rewrote the
route's arrays.
Validation as measured: x3-lang `cargo test --workspace` **1213 passed / 0 failed** (1207 before);
clippy `-D warnings` clean; fmt clean; sweep 19/19/19/18. Five new tests in
`vm/tests/opportunity_packets.rs` and one through the binary.
Original entry:
Reason: PHASE 29's packet is implemented (`vm/src/opportunity_packet.rs`): it is typed,
versioned, hashed over a domain-separated encoding, ed25519-signed and replay-protected
through `OpportunityPacketLedger::admit`, and it decides everything its own fields can
decide — an inverted capital window, a profit floor unreachable after the packet's own
worst-case fee, route liquidity below `max_capital`, route fee at max capital above
`maximum_fee`, route slippage above the ceiling, a mismatched route, an unnamed venue, a
missing state root, a mismatched commitment or hash, an expiry, and a missing, untrusted
or invalid signature. What it cannot decide is whether the opportunity is *real*: whether
the state roots are fresh, whether the venue prices the route was scored from are the
prices that will fill, and whether the `execution_commitment` is backed by a strategy the
seller actually holds. `proof_requirements` is a set of names because the proof
vocabulary belongs to hosts and adapters; an empty set means "no proof attached".
Acceptance criteria: a proof-requirement vocabulary a host can satisfy and a verifier can
check (state-root freshness against a known block, venue price attestation, and a
strategy-commitment linkage the seller cannot forge), so that `x3c packet verify` reports
which requirements it checked and which it could not rather than only checking the
signature and the packet's internal arithmetic.
Validation: a packet whose state root is stale is refused with the two block numbers; a
packet whose venue price attestation disagrees with the route it carries is refused with
both prices; a packet with a proof requirement the verifier does not know is refused
naming the requirement.

> Numbering note: `.ai/memory/agent-memory.md`'s PHASE 29 entry calls this ticket
> "TICKET-071" and calls the arb pipeline "TICKET-070". Both were placeholders taken
> before the numbers were claimed by other agents: the netting settler is TICKET-071 and
> the rebalance transaction graph is TICKET-070. This entry is the packet-realism ticket.

## TICKET-075 — a venue's settlement guarantee does not reach the artifact — CLOSED
Type: CLOSED in `1f976974a` (2026-09-19) · Subsystem: x3-lang/compiler + vm artifact
Closed: a venue declaration now lowers to `Operation::VenueSettlement { venue, guarantee }`,
emitted as an inline record at opcode `0x58` — `[opcode][u16 len][venue:shape]`, padded to
four bytes, registered in the one `is_payload_opcode` table the disassembler and the verifier
both walk by — so the guarantee survives the compilation instead of being dropped with the
declaration:
```
$ x3c inspect artifact.x3b
  0009  0x58  VENUE_SETTLEMENT   venue cex_hedge settles compensating
  0010  0x58  VENUE_SETTLEMENT   venue pool_plain settles none
```
`guarantee: Option<SettlementGuarantee>` is encoded as a present-but-empty shape field, so
"states none" is a fact the artifact states rather than a default it invents. One test asserts
the *absence* rather than only that something arrived, because the compiler requires a
guarantee of an `orderbook` venue and leaves an on-chain venue that states none alone — two
different facts that must stay distinguishable. `verifier::verify` is the **single**
enforcement point (UTF-8, exactly one separator, a non-blank venue name, and a shape
`SettlementGuarantee::parse` knows, checked against the compiler's own enum). The executor's
arm advances the pc and does **not** re-validate: `VM::execute` is `verify_and_execute` and
its only callee of `execute_unverified`, so no public path reaches the executor without the
verifier and a second copy would be a rule nothing exercises. That was found by writing the
test wrong first — it expected the executor's own refusal code and got
`X3_VERIFY_FAILED: InvalidOperand(180)`, which is how the ordering became visible. The
verifier also used `venue.is_empty()` where the compiler uses `trim().is_empty()`, so an
all-spaces name passed the VM and failed the compiler; aligned, reason in the comment.
Not carried: a venue's fee, slippage and liquidity still reach the artifact through the plan a
route produced rather than as their own record. Nothing reads those to decide whether an
off-chain leg is honest, which is what this ticket was about.
Validation as measured: `x3c build` + `x3c inspect` for a `compensating` orderbook venue, an
`atomic` on-chain venue, and a venue stating none; four malformed-record cases each refused
with an unmutated control in the same test; x3-lang `cargo test --workspace` **1131 passed /
0 failed** (1120 before); clippy and fmt clean; pytest 21; the 19-example sweep unchanged at
check 19/19, build 19/19, warning-free 19/19, run-artifact 18/19.
Original entry:

Reason: PHASE 39's `settlement` clause is enforced at compile time
(`semantic::verify_venue_decls` refuses an `orderbook` venue that states no guarantee and
one that claims `atomic`) and it round-trips through `x3c fmt`, so a reader of the *source*
can see the trust model. It is not carried into the artifact: venues do not lower to any
`Operation`, so the emitted bytecode says nothing about whether a leg was atomic, escrowed,
pre-funded, attested or merely compensated. A replayer, an auditor or a counterparty
reading the artifact therefore cannot see the assumption the trade rests on, which is half
of what the phase asks for.
Acceptance criteria: the guarantee a route's venue carries travels into the artifact in a
form a reader can recover — the same way finality depth travels in the executed artifact
(TICKET-059) and the rebalance portfolio travels in `Operation::Rebalance` — and a host or
replayer that recovers it can tell an `atomic` leg from a `compensating` one. Recovering it
must not require the source.
Validation: building a program with a `compensating` venue leg and `x3c inspect` (or the
artifact's own decode path) shows the shape; the same program with an `atomic` on-chain
venue shows `atomic`; a venue with no guarantee shows none rather than a default.

## TICKET-076 — two implementations of PHASE 37 exist, and only one may survive — CLOSED
Type: CLOSED in `fb426988e` (2026-09-19) · Subsystem: x3-lang/compiler
Closed: `compiler/src/arb.rs` is the surviving surface. The distinctive idea of the other
implementation — judging the clauses against the real graph rather than beside it — is now
in it as `arb::venue_standings`, which judges every declared venue against the
declaration's own bounds using the numbers `opportunity.rs` carries (`fee_bps`,
`liquidity`, `slippage_bps` and the chain the venue settles on) and refuses a scope no
venue survives, naming every venue and the bound that removed it. A program that declares
a scope and no venue is refused too. One `arb` surface, one parser dispatch, both sets of
checks, and the module doc records why one of the two survived.
Not carried over, deliberately: `arbitrage.rs` allowed `capital { flash = enabled }` when
a declared flash venue covered the ceiling, while spec PHASE 20 forbids shipping flash
collateral before a formal safety proof. Allowing it would permit a claim the phase
forbids, so the surviving implementation still refuses it.
Original entry:
Reason: PHASE 37 was implemented twice, independently, by two agents working in
different trees, and both were merged into the same phase ledger before anyone
noticed. The one on `master` is `compiler/src/arb.rs` (`db536ab4c`), claiming
`Item::Arb` / `Operation::Arb` and its own parser dispatch for `arb { … }`. The other
is `compiler/src/arbitrage.rs`, preserved unrebased on
`wip/x3lang-preserve-packets-and-arbitrage-20260919` (`1bfaa5243`), with hooks in
`compiler/src/objective.rs` and `compiler/src/opportunity.rs`.
Each has something the other lacks:
- `arbitrage.rs` decides the declaration against the **real graph machinery**,
  lowering the clauses to `OpportunityConstraints` and calling
  `opportunity::reject_reason` / `path_reject_reason`, so a declaration it accepts
  cannot be one the search would refuse. It also refuses an absent bound rather than
  defaulting one, and requires a declared flash venue whose depth covers the ceiling
  before `flash = enabled` is allowed.
- `arb.rs` answers the declared profit floor against the program's own
  `require profit` guards (`arb::enforcement`), which is the check that keeps
  `min_profit = 20bps` from being a label nothing acts on, and it maps the phase's
  seven pipeline stages onto the five modules that implement them while naming the
  two that do not (`arb::missing_stages`), which the verifier and emitter refuse over.
Both pass their own tests and both parse `arb { … }`, so shipping both would give the
language two spellings of one declaration and two answers to one question.
Acceptance criteria: exactly **one** `arb` surface remains —
`Item::Arb`/`Operation::Arb` and one parser dispatch — and it carries **both** sets
of checks: the graph-grounded `OpportunityConstraints` validation *and* the
guard-enforcement answer for `risk.min_profit`, plus the stage mapping. The losing
implementation's file and tests are deleted, not left unreferenced; the phase ledger
row for 37 names the surviving module; and `cargo test --workspace` covers the union
of the two test sets. Deleting one implementation is not weakening coverage as long
as its distinctive assertions are carried over.
Validation: a single `arb` declaration parses to a single AST item; a scope with an
absent bound, a scope whose declared chains cannot host a venue, a scope whose
`flash = enabled` has no covering flash venue, and a scope whose `min_profit` has no
enforcing guard are each refused with the figures; `cargo test --workspace` green and
`grep -rn "fn parse_arb" x3-lang/compiler/src` returns one body.

## TICKET-077 — `arbitrage.rs` was never rebased, so its merge state is unknown — CLOSED
Type: CLOSED in `fb426988e` (2026-09-19) · Subsystem: x3-lang/compiler
Closed by TICKET-076's resolution: the branch's distinctive checks are carried into
`compiler/src/arb.rs` on master, so `wip/x3lang-preserve-packets-and-arbitrage-20260919`
(`1bfaa5243`) is now purely archival history and is no longer pending work. It has not
been deleted, so the reasoning behind the design it contained is still readable, but
nothing needs to be merged from it. If it is deleted later, TICKET-076's acceptance
criteria are already met.
Original entry:
Reason: `wip/x3lang-preserve-packets-and-arbitrage-20260919` is based on `0a68cb883`
and was deliberately not rebased onto `master`, because the work it preserved had been
sitting uncommitted in the canonical working tree and rebasing it would have risked
losing it. Its 934-passing state is therefore a statement about a base four PHASES old
(11, 22, 29, 37, 39 have landed since), and nobody has yet established what it
conflicts with. It is archival, not mergeable.
Acceptance criteria: the branch is either rebased onto `master` and reconciled per
TICKET-076, or deleted with its distinctive checks already carried into the surviving
`arb` implementation; either way the branch's status is recorded and it stops
appearing in merge queues as pending work.
Validation: `git merge-base --is-ancestor master wip/x3lang-preserve-packets-and-arbitrage-20260919`
succeeds, or the branch is gone and TICKET-076's acceptance criteria are met.

## TICKET-078 — lowering silently drops any top-level declaration it does not know — CLOSED
Type: CLOSED in `3bac66b49` + `b0f409b0e` (2026-09-19) · Subsystem: x3-lang/compiler
Closed: the wildcard is gone. Every item that generates no operations is named, with the reason,
so adding a variant to `Item` now fails to compile until somebody decides what it lowers to. The
first attempt at the list was wrong twice — twelve arms it named already had their own (clippy's
unreachable patterns) and three variants it omitted had none (cargo's non-exhaustive match) —
which is the change demonstrating its own value: the wildcard accepted all three errors in
silence, and the explicit list turned each into a compiler diagnostic.
Original entry:
Reason: `compiler/src/lowering.rs:835` ends the item match with
`_ => {} // Other items (types, imports, ErrorDecl, etc.) don't generate operations`.
The catch-all is deliberate for items that really do generate nothing, and it is also a
trap: any *new* declaration added to `Item` is silently omitted from the IR, so a program
whose plan lives in that declaration lowers to an artifact that does not mention it. The
artifact is then not a fake in the sense of a stub that returns a wrong answer — it is
worse, because it is silent: `x3c lower` and `x3c build` both succeed and the declaration
is simply absent. PHASE 38 hit this: `Item::Hyperarb` had to be given an explicit arm
(`bdb8c40e8`) or a `hyperarb` would have vanished from the artifact.
Acceptance criteria: the catch-all is replaced by an explicit list of the items that
generate no operations (with the reason each does not), so adding an `Item` variant fails
to compile until someone decides what it lowers to; and every declaration that carries a
plan or a policy lowers to an `Operation` that `x3c lower` shows.
Validation: adding a variant to `Item` without touching `lowering.rs` fails the build;
`grep -c "_ => {}" compiler/src/lowering.rs` at the item match is zero; the corpus sweep
still builds and runs 17 of 23.

## TICKET-079 — a hyperarb's resolved legs have no generator — CLOSED
Type: CLOSED in `89a15ccc5` (2026-09-19) · Subsystem: x3-lang/compiler
Closed: `hyperarb::plan` selects one candidate leg by the declared criterion and emits its
route — `AtomicBegin, AtomicChoice, MultiHopSwap, RouteFallback, Require, AtomicEnd` — so
`check`, `build`, `explain` and `run` all succeed. The reading the ticket asked about is
settled as the candidate-routes reading, and the three consistency rules the plan needed are
refusals rather than guesses (a leg must move the capital's asset, a leg's venues must agree
on an asset pair, and the program must carry a slippage ceiling because
`verify_slippage_explicit` requires one for any swap leg). `choose highest_net_output` — the
phase's own example — is refused with the missing number named. The VM's verifier and executor
whitelisted only two choice criteria, so the first artifact carrying the third failed to run;
`spec/opcodes.rs` is the single source for that vocabulary and both now accept it.
Original entry:
Reason: `hyperarb::analyse` resolves every leg to a declared venue, chain or domain and
decides the clause set, and `Operation::Hyperarb` refuses at the IR verifier and the
emitter because the generator that turns those resolved legs into operations is not
written. Its sibling `arb` has one (`arb::plan`, `8e53a3e84`), and the two differ in a way
that matters: an `arb` is one cycle, while a `hyperarb` is *several* paths evaluated at
once, so its plan is a `ParallelPlan` over per-leg operation sets (`dag::leg_from_operations`
and `dag::plan`, both already used by PHASE 16) rather than a single `MultiHopSwap`.
Acceptance criteria: `hyperarb::plan` builds one leg's operations per resolved leg from the
venues that leg resolves to (a venue's `asset_in -> asset_out` swap), hands them to
`dag::leg_from_operations` and `dag::plan` for the waves, edges and settlement record, and
emits the plan inside one atomic block with the declaration's `require net_profit >= <n>bps`
as a runtime guard and the chosen path recorded as an `AtomicChoice` over the legs; a leg
that resolves to a venue whose assets the others cannot meet is refused with that leg and
that venue named. The hedge clause keeps pointing at PHASE 9's `atomic_hedge` rather than
re-implementing it. `missing_stages` must stay empty: the shared pipeline is owned, and what
is missing here is hyperarb's own generator.
Validation: a `hyperarb` program checks, builds, disassembles with the chosen route's legs
inside an atomic block, and runs against the fixture host; a `hyperarb` whose legs cannot be
planned is refused with the leg named.

### Two things decided while looking at this, one of which needs a call

**1. `choose highest_net_output` cannot be ranked, so it must be refused with that reason.**
`compiler/src/opportunity.rs` opens by naming what it does *not* model ("What it deliberately
does *not* do: model price impact as a function"), and the graph carries no price and no
output: a venue edge has fee, slippage, liquidity, latency, finality and risk. Ranking legs by
their net output therefore needs a number the compiler does not have, which is already why the
language refuses `maximize profit` and `maximize output` for an objective. The phase's own
example writes exactly the unrankable one. The generator has to accept the criteria it *can*
compute — `fewest_hops`, and `lowest_declared_fee` (added by `8e53a3e84`) — and refuse
`highest_net_output` at the AST layer with the missing-number reason, the way `arb` refuses an
unrankable `minimize` target. Ties go to the earliest declared leg, which is
`atomic_choice`'s own established convention, so a tie is not a refusal.

**2. `parallel { route_a = evaluate(…); … }` has two readings and they emit different
operations.** The phase's prose says several paths are "evaluated" and `choose` then takes one;
the phase's own lowering pipeline says `Candidate Routes → Filter → Dependency DAG → Risk
Verification → Execution Plan`, which is candidates filtered to a winner. But the keyword is
`parallel` and the diagram in the originating brief runs EVM/SVM/X3VM *concurrently* with a
profit check afterwards, which is one plan whose legs run together.
- Under the **candidate-routes** reading, the legs are alternatives, `choose` selects one, and
  the artifact is `AtomicChoice` over the legs followed by the winner's operations — the
  machinery `atomic_choice` already has.
- Under the **concurrent-legs** reading, every leg runs, the artifact is `ParallelPlan` (the
  machinery PHASE 16 already has), and `choose` names a post-hoc decision about where the
  highest output lands — which **no existing operation records**. That reading needs either
  prices or a new operation whose runtime semantics would have to be defined first; inventing
  one to make the phase look finished is what this repository forbids.
Either way the shared, unambiguous work is the same: one leg's operations per resolved leg
from the venues that leg resolves to, and the profit floor as a runtime guard.

**Decided (2026-09-19), from the spec's own text rather than from a reader's preference:** the
candidate-routes reading. The phase's own description of the lowering is "Opportunity Graph →
Candidate Routes → Filter → Dependency DAG → Risk Verification → Execution Plan → Atomic
Settlement" — candidates are *filtered*, which is a selection among alternatives — and
`choose highest_net_output` only means anything if the legs are alternatives, since you cannot
choose one of three things you already ran. So `parallel { … }` is parallel *evaluation* of
candidates, and the artifact carries `AtomicChoice` over the legs followed by the chosen one's
operations: machinery `atomic_choice` already has. The originating diagram's concurrency is the
evaluation's, not the artifact's. What remains for this ticket is therefore only the leg
generation and the criterion rankability above.

## TICKET-080 — a `Release` does not name the lock it claims, so a book cannot settle as one unit — CLOSED
Type: CLOSED in `2e341876e` (2026-09-20) · Subsystem: x3-lang/compiler
Closed: a claim names its lock. `Operation::Release` gains `claims: u32` — the position of its lock
among its route's locks, an index the compiler assigns when it pairs the two — and the whole book is
**one atomic route** now, which is what makes the offsetting valid. Measured, on a book whose
residual is two USDC transfers both paying `bob`:
```
$ x3c explain book.x3b
  0001  0x50  ATOMIC_BEGIN
  0002  0x20  LOCK     Lock { chain: "ethereum", asset: "USDC", amount: 120, from: "0xA1" }
  0003  0x23  RELEASE  Release { chain: "ethereum", asset: "USDC", to: "0xB1", claims: 0 }
  0004  0x20  LOCK     Lock { chain: "ethereum", asset: "USDC", amount: 80, from: "0xC1" }
  0005  0x23  RELEASE  Release { chain: "ethereum", asset: "USDC", to: "0xB1", claims: 1 }
  0006  0x51  ATOMIC_END
```
The index is **not** a field on `Lock`: a position is derivable from the route, so it cannot disagree
with the locks it indexes. The payload's field is appended, so a record written before it exists ends
early and is refused as short rather than misread. `no_double_claim` counts claims by lock index now
(its own description always said "for the same lock"; it was counting by asset), so two claims of one
asset are allowed and one lock claimed twice is refused with the lock named. `no_refund_after_claim`
and `escrows_claimed_in_their_own_route` stay escrow-level deliberately — a refund is keyed by asset,
so comparing a claim's lock index against a refund's asset would be a category error — which is why
the reader split into `release_lock` (the escrow) and `claimed_lock` (the index) rather than widening
one helper's meaning.
**Written, tested, and reverted:** a range check refusing a claim whose index the route does not have.
It failed four tests over a cross-chain parallel plan, because this IR uses `Release` for both
*claiming an escrow this program locked* and *paying out the asset a route delivered* — a distinction
`no_refund_after_claim` already documents, having had to make it to stop warning on every canonical
example. A range check needs to know which of the two a release is, and nothing in the IR says, so the
attempt is recorded in the code and the distinction is its own ticket (TICKET-101).
Validation as measured: a book with two same-asset transfers lowers to **one** route holding two locks
and two claims asserted as `(locks written, claim index) == [(1,0), (2,1)]` *and* on both releases
being of the same asset, so the distinction cannot be coming from the asset; the pairing survives into
the artifact's decode; a route claiming one lock twice is refused while two locks claimed once each is
accepted. x3-lang `cargo test --workspace` **1153 passed / 0 failed**; clippy and fmt clean; pytest 21;
sweep check 19/19, build 19/19, warning-free 19/19, run-artifact 18/19.
Original entry:
Reason: PHASE 22's residual is settled one atomic route per transfer (`fa12e602d`) because a
`Release` carries only `(chain, asset, to)`: two transfers of the same asset in one route are
two claims a replayer cannot tell apart, and the static rules refuse them. So a book whose
transfers share an asset — the normal case, since a book nets within one asset — has no way to
settle all-or-nothing, and all-or-nothing is what makes netting valid: if some residual
transfers settle and others do not, the net positions the analysis preserved are not the
positions that result.
Acceptance criteria: a `Release` (or a new claim operation) names the lock it claims — a
reference the compiler assigns when it pairs a `Lock` with its `Release`, carried in the
artifact so a replayer can check the pairing rather than infer it — and an atomic route may then
carry several claims of one asset, each against its own lock, so one route settles a whole book
and the net-position property holds over the settlement rather than over each transfer.
Validation: a book with two same-asset transfers builds as one atomic route with two
distinguished claims; a route claiming one lock twice is still refused; a replayer reading the
artifact can pair each release with its lock without the source.

## TICKET-027 (progress, 2026-09-19) — a plan's floor is enforced; a program's guard is a constraint

**Landed in `31522c24e`.** The spec distinguishes two things the language had conflated, and
`Operation::Require` now carries the distinction at the IR level as
`measured: bool` (`#[serde(default)]`, so nothing stored changes):

- a guard a **program** writes is a *constraint* — the compiler checks it against the
  venues' and the policy's declared numbers, and the instruction records it
  (`measured: false`, `REQUIRE_COMPARE_STATIC`), exactly as before. Every existing
  program keeps its meaning, which is verified rather than asserted: `simple_swap`'s
  `require slippage <= 50` still runs with no measurement, and the example sweep is
  unchanged at 17/23 run.
- a guard a **plan generator** emits after the trade it bounds (`arb::plan`,
  `hyperarb::plan`) is a *post-condition* on what that trade realised
  (`measured: true`), and it is **enforced**:
  - no measurement → `X3_GUARD_UNMEASURED`, never a comparison against `r0` residue;
  - measured and clearing the floor → settles;
  - measured below the profit floor → `X3_PROFIT_BELOW_FLOOR: the trade realised 5bps and
    the program requires at least 20bps`;
  - measured above the slippage ceiling → `X3_SLIPPAGE_ABOVE_CEILING: the trade realised
    90bps and the program allows at most 8bps`.

A measurement is a fact about the market, so it belongs to the **host**: a reply carries a
*sequence* of tagged records (tag, unit, 16-byte little-endian basis points), one call
answering both questions a plan asks about it. `DryRunBridge::with_measurement(..)` and
`x3c run --measured-profit-bps/--measured-slippage-bps` let a caller *state* the outcome a
dry run is judged against; stating half of one is refused rather than completed by
inventing the rest.

Two things this uncovered, both pre-existing:

- the VM's bytecode verifier accepted only `STATIC` and `GE`, so the first artifact
  carrying a measured mode was rejected as `InvalidOperand` *before* the executor could
  refuse it honestly — the same two-layer whitelist failure the choice criteria hit in
  `89a15ccc5`. Both layers read `spec/opcodes.rs` now.
- **`ON_FAIL` takes its handler target from `r0`** (`decode_reg_reg_imm` then
  `registers[ra]`, with the emitter writing operand 0), so a guard failure routed through
  the handler mechanism lands at whatever pc `r0` happened to hold — the first version of
  this change aborted with `InvalidOpcode(115)` rather than naming the shortfall. A
  measured floor therefore *refuses* rather than dispatching, and **TICKET-058's "explicit
  branch target in the record" is what the handler mechanism needs** for its own sake,
  independently of guards.

**What is still open.** A *program's* economic guard is not enforced: it is a constraint
checked against declarations, and enforcing it as well would refuse ten of the twenty-three
corpus examples at guards written before the trade they name (`simple_swap`,
`atomic_choice`, `flagship_b52`, `intent_fusion`, `mainnet_safe_swap`, `multi_leg_route`,
`objective_routing`, `parallel_dag`, `route_fallback`, `strategy_module`) — measured, by
running it. Whether those guards should also be post-conditions is a language decision, and
if they should, the honest path is to make the *guard's place* say so rather than to have
the executor guess.


Measured: the emitter's `Require` arm states it plainly — "always STATIC today … a guard would
have to find its quantity in `r0`, and no instruction puts it there". Only the **nonce** guard is
emitted as a comparison, because `NONCE_UNUSED` puts its quantity in `r0` immediately before it.
So the profit and slippage floors that PHASE 37 and 38's plans carry are **records**, not tests:
the artifact states `profit >= 20bps` and nothing evaluates it. (An earlier report of mine called
them runtime guards; that was wrong.)

A design was built and verified end to end, then reverted, because it mis-models what the
existing guards *are*:

- a reply-tag convention (`CAPABILITY_REPLY_MEASURED_TAG`, with the unit — profit or slippage in
  basis points — and a 16-byte value) so a host can report a measurement in the reply to the
  trade, which is the only place a measurement can honestly come from;
- two new comparison modes (`REQUIRE_COMPARE_MEASURED_PROFIT` / `..._SLIPPAGE`) that fail closed
  with `X3_GUARD_UNMEASURED` when no host reported one — never comparing `r0` residue;
- `DryRunBridge::with_measurement(..)` and `x3c run --measured-profit-bps/--measured-slippage-bps`,
  so a dry run is judged against a market outcome the caller *states* rather than one the
  harness invents, and a half-stated measurement is refused;
- the VM's bytecode verifier also whitelisted only modes 0 and 1, so the first artifact carrying
  a measured mode was rejected as `InvalidOperand` before the executor could refuse it for the
  honest reason — the same two-layer whitelist failure the choice criterion hit in `89a15ccc5`.

**Why it was reverted.** With enforcement on, ten of the twenty-three corpus examples fail at
run time — `simple_swap`, `atomic_choice`, `flagship_b52`, `intent_fusion`, `mainnet_safe_swap`,
`multi_leg_route`, `objective_routing`, `parallel_dag`, `route_fallback`, `strategy_module` — and
every one of them at a guard written *before* the trade it names:

```
atomic_choice.x3: X3_GUARD_UNMEASURED: the guard `slippage <= 50bps` needs a slippage the host measured…
```

Those guards are **pre-conditions**: `require slippage <= 50` at the top of an intent asserts a
bound the compiler checks against the venues and the policy, and the trade below it is what the
program *then* does. Nothing evaluates them today, and reading them as post-conditions on the
outcome changes the meaning of ten working programs in one commit. The emission is a record
because the language's guards *are* records.

**What enforcement needs, in order:** (1) a rule for where a measured guard may sit — after the
trade it measures, which means deciding whether `require` in an intent is a pre-condition (as it
now reads) or a post-condition (as enforcement would need); (2) the reply-tag convention above,
which is built and discarded but re-derivable from this entry; (3) fail-closed evaluation, which
needs (1) or it refuses programs that are correct under the current reading; and (4) the VM's
verifier whitelist updated in the same commit, because it is a second place that names the
comparison modes. Until then the honest statement is the one the emitter already makes: the floor
travels in the artifact and is re-checkable, and no runtime quantity is tested.

## TICKET-081 — the decimal-literal conversion has two homes, so its rounding rule does — CLOSED
Type: CLOSED in `a24b10f3b` (2026-09-19) · Subsystem: x3-lang/crates/x3-common + compiler
Closed, with one criterion corrected by measurement and one unification found impossible.
**The differential test came first and is the load-bearing part**:
`compiler/tests/test_amount_conversion_agreement.rs` walks 13 literals × 6 decimals × 3
directions through both paths, asserts the mapping the ticket named (`Ok(v) ↔ Some(v)`,
`Err(PrecisionLoss) ↔ None`) and **counts the comparisons** so a table that compared nothing
cannot pass — 234 of 234.
**What the work found**: the compiler's `MAX_DECIMALS` is **38** and `fixed::MAX_SCALE` is
**18**, so `decimal_to_base_units("1.0000000000000000001", 19, Exact)` is `Ok(…)` while
`Decimal::<18>::from_parts` refuses the same literal. The obvious unification — delegate to
`Decimal` — is therefore **impossible**: an 18-scale `u128` mantissa cannot be widened to 38
without giving up almost all of its range, so it is a property of the representation, not an
omission. The criteria did not anticipate it; the boundary is now asserted in the tree
(`an_asset_finer_than_the_fixed_scale_is_the_compilers_alone`).
**What was unified**: the *decision*, since the arithmetic cannot cross that boundary.
`fixed::apply_rounding` is the one place the three directions are decided; `div_rounded`
calls it and so does `decimal_to_base_units`. `fixed_math.rs` gains
`the_rounding_decision_is_one_function`, including `Up` at the top of the range refusing
rather than wrapping. The refactor is proven behaviour-preserving **by the differential test
written first** — green against two independent implementations, still green after one
became a caller. The sweep is unchanged at 19/19, which is the same fact at the artifact
level.
**One ordering preserved deliberately**: the `Exact` refusal stays before the arithmetic, so
a literal that is both lossy *and* too large still reports the loss rather than the overflow.
Moving the shared rule to the end of the function would have swapped that, and the tidier
version is the wrong one — the comment says so at the call site.
**The `checked_pow` criterion holds after all**, measured rather than assumed: `grep -rn
checked_pow compiler/src crates/x3-common/src vm/src` returns exactly one line, the
compiler's, because `fixed` computes powers through `pow10`'s match. The earlier guess that
it could not hold was wrong.
1119 tests / 0 failed.
Original entry:
Reason: PHASE 43 (`1a9274900`) introduced `x3_lang_common::fixed::Decimal`, whose
`from_base_units` / `to_base_units` implement "convert between an asset's decimals
and the fixed scale, with the rounding stated" — and they are tested against the
phase's audit list. Those are not the functions the language actually calls when it
reads an amount. It calls `compiler/src/trading_semantic.rs::decimal_to_base_units`
(`:435`), which parses a literal string (handling `_` separators and reporting
`TradingTypeError::PrecisionLoss`) and then re-implements the same rounding rule
with its own `10u128.checked_pow`. So after this commit the *rule* — how a discarded
fraction behaves under `Down`/`Up`/`Exact`, and where the overflow boundary is — has
two implementations and one home for its tests (the fixed-point one). The two agree
today; nothing makes them agree tomorrow.
There is a smaller instance in the same area: `compiler/src/profitability.rs:134`
writes a bare `10_000` and a `saturating_mul` for the fee figure in a diagnostic.
Left alone deliberately by `1a9274900` — it is a reported number rather than a
guard, and a checked product would change the figure the diagnostic prints — but it
is the last un-named basis-point literal on a path the phase cares about.
Acceptance criteria: one implementation of the rounding rule. Either
`decimal_to_base_units` keeps the *parsing* (separators, `PrecisionLoss`,
`InvalidDecimals`) and delegates the arithmetic to `Decimal::from_parts` +
`to_base_units`, or `Decimal` grows a `from_literal` and the compiler's copy is
deleted; either way `10u128.checked_pow` appears once in the tree, and
`TradingTypeError`'s taxonomy is unchanged so no diagnostic text moves. The
profitability figure is either routed through `Bps::of_floor` or the reason it is not
is written where it is computed.
Validation: a differential test that walks a table of (literal, decimals, rounding)
cases — the boundary ones, a discarded `0001`, an exact fit, a `_`-separated literal,
a literal longer than the asset decimals, and a product that overflows — through
both paths and asserts they agree, with the mapping `Ok(v) ↔ Some(v)` and
`Err(PrecisionLoss) ↔ None`; `cargo test --workspace`; `cargo clippy --workspace
--all-targets -- -D warnings`.

## TICKET-082 — `x3-crosschain-intent`'s `no_std` path does not compile — CLOSED
Type: CLOSED in `6d6dedca6` (2026-09-19) · Subsystem: crates/x3-crosschain-intent
Closed: **244 errors → 0**, and fixing it uncovered a critical defect (TICKET-092).
The crate was already `#![cfg_attr(not(feature = "std"), no_std)]` with `extern crate alloc`,
so it was no_std-aware; its `std` feature existed only to turn on `serde/std`, `hex/alloc`,
`sha2/std` and the router's `std`, and the first two are needed by code with no `std`-only
path — **230 × "the trait bound `String: serde::Deserialize` is not satisfied"** and **13 ×
"cannot find function `hex::encode`"**. They are declared where they are needed now
(`serde` gains `alloc`, `hex` gains `alloc`) and `std` is `["serde/std", "sha2/std",
"x3-verification-router/std"]` — a switch that turns something on rather than off.
This mattered because `pallet-x3-settlement-engine` depends on the crate with
`default-features = false`, so "off" is a configuration the runtime's wasm graph builds.
**The measurement error recorded in round 59 is worth repeating**: the re-measurement must
run from the root workspace, not from `x3-lang/`, or it prints
`cannot specify features for packages outside of workspace` — one line that looks like a
near-fix and measures nothing.
Original entry:
**Re-measured in round 59, after PR #361 edited the same file: still 244 errors, and
still the `hex::encode` shape.** PR #361 ("the embedded runtime could not link std — the
intent crate took the verifier's default features") fixed a *different* leak that
`bebbe55cf` (TICKET-065) introduced: `x3-verification-router` was added as a plain path
dependency, so its default `std` feature was carried into
`pallet-x3-settlement-engine`'s wasm graph. That is fixed with `default-features = false`
plus `x3-verification-router/std` in this crate's `std` feature. This ticket's own
evidence is unchanged by it.
Measurement note, because it nearly misled: the re-measurement must run from
`/tmp/x3lang-p29` (the **root** workspace), not from `x3-lang/`. From `x3-lang/` the
command prints `cannot specify features for packages outside of workspace` — one line
that looks like a near-fix and is an invocation error measuring nothing.
Original entry:
Reason: found while validating TICKET-065. `cargo check -p x3-crosschain-intent
--no-default-features --offline` fails with **244 errors**. The bulk is one shape:
`default = ["std"]` and `std = [..., "hex/alloc", ...]`, while the crate calls
`hex::encode` unconditionally in `compiler.rs` (and twice in `proof/evm.rs`'s
`LogMismatch` arm), so switching the default features off removes an API the code
always uses. The `no_std` path has therefore never been built, and any claim about
this crate being `no_std`-capable is untested.
This matters because `pallets/x3-settlement-engine` depends on the crate and pallets
are built for `wasm32v1-none`. That build uses default features, so the breakage is
latent rather than live — but the workspace already carries a note about a
`blake2`/`std` path leaking into `wasm32v1-none`, i.e. this class of problem has
shipped here before.
Measured, not assumed: moving `hex`'s `alloc` feature to an unconditional
`features = ["alloc"]` takes the count from 244 to **231**, so the remaining 231 are
other feature-gating problems (the same measurement is why this is a ticket rather
than a one-line fix in `bebbe55cf`).
Acceptance criteria: either `--no-default-features` compiles with 0 errors, or the
crate stops declaring a `std` feature it cannot be built without — the second is
acceptable and is what the entry is really asking for, because a feature flag that
does nothing is a claim that does nothing. If the first, `hex` and `thiserror` carry
the features the code needs unconditionally and the `std` feature means something
narrower.
Validation: `cargo check -p x3-crosschain-intent --no-default-features --offline`
reports 0 errors; `cargo check -p x3-crosschain-intent` and `cargo test -p
x3-crosschain-intent` unchanged; `cargo check -p pallet-x3-settlement-engine`
unchanged; if the pallet can be built for `wasm32v1-none` in this environment, that
build too.

## TICKET-083 — 6 of the 23 examples do not check, and that has never been written down — CLOSED
Type: CLOSED in `6f6bc0463` (2026-09-19) · Subsystem: x3-lang/examples
Closed: **19 of 19 examples now check, build and are warning-free** (was 17 of 23), and a
gate keeps it that way. Five are retired to `examples/legacy/` with a README that says, per
file, what its dialect is and **why it is not rewritten**: `arb.x3` is superseded by
`examples/arb_scope.x3`, `flash.x3` *cannot* be rewritten (flash capital is PHASE 20, whose
own text forbids shipping before a formal safety proof), `jit_lp.x3` and `mev_smooth.x3`
have no language feature to translate to, and `x3_coin_layer.x3` calls itself `(Pseudo)`.
The README also says how to bring one back: translate the subject, not the syntax.
`arb_solana_eth.x3` is fixed rather than retired — it was never stale syntax, it was
standing on TICKET-088's hole — and `arb_scope.x3` is new, the only example of PHASE 37's
`arb` + `venue` declarations in the tree.
The gate is `crates/x3-tools/tests/cli.rs::every_example_checks_and_builds`: `check
--deny-warnings` and `build` over every `examples/*.x3`, with the too-few-files guard the
determinism audit uses so a moved directory fails rather than exempts. `--deny-warnings` is
deliberate — it is how `arb_solana_eth.x3` turned out to have two refund operations claiming
one lock — and the gate was verified by adding a broken file and watching it fail.
**The sweep baseline moved and the change is the point**: `files=23 check=17 build=17
warning-free=17 run-artifact=17` became `files=19 check=19 build=19 warning-free=19
run-artifact=18`. The file count fell because five were retired; `run` is 18 because
`arb_scope.x3` refuses a bare `x3c run` **by design** (its floors are measured guards and a
dry run has no host measurement — the example's header now says so and shows the two ways
to see it settle).
Doing this found two further defects: TICKET-090 (the formatter corrupts a percent literal)
and TICKET-091 (the Python surface reads 6 of 19 examples).
Original entry:
Reason: `x3c check` on every `examples/*.x3`, measured this round:
```
FAIL examples/arb.x3              lowering failed: Parser error: expected top-level item
FAIL examples/arb_solana_eth.x3   1 error: declaration requires `proof`, which is not a guard
                                  kind this compiler knows … the kinds it knows are: finality,
                                  slippage, profit, invariant, risk, nonce, audit_gate,
                                  bridge_liquidity, canonical_supply, relayer_quorum,
                                  route_score, solver_bond, proof_complete, refund_path,
                                  refund_to, finality_explicit, vm_supported, mainnet_safe
FAIL examples/flash.x3            lowering failed: Parser error: expected top-level item
FAIL examples/jit_lp.x3           lowering failed: Parser error: expected top-level item
FAIL examples/mev_smooth.x3       lowering failed: Parser error: expected top-level item
FAIL examples/x3_coin_layer.x3    lowering failed: Parser error: expected top-level item
```
The five parse failures are written in an older `contract X { … fn … }` dialect
(`examples/arb.x3` opens `contract CrossDexArbitrage {`, uses `const WETH: address = …`
and declares `fn execute(amount_in: u256, …)`) that the parser no longer accepts. The
semantic failure uses the guard name `proof` where the language now spells
`proof_complete`. Nothing in the repository builds these files, so nothing noticed: the
conformance sweep reports `17/23` every round and the number has been read as a
baseline rather than as six defects.
Acceptance criteria: every `examples/*.x3` either checks and builds, or is deleted or
moved under a directory whose name says it is not a program the current compiler
accepts (`examples/legacy/` with a README stating which syntax each file is from, and
why it is kept). Silently keeping a file a reader will try to run is not an option,
because the examples are the language's documentation. A file rewritten rather than
retired must keep the example's *point* — `arb.x3` is the cross-DEX arbitrage
walkthrough and `flash.x3` the flash-capital one — so the rewritten form uses `arb` /
`capital { flash = … }` as PHASE 37 and PHASE 20 define them, not a translation of the
old dialect that happens to parse.
Validation: a test or a `scripts/` gate that runs `x3c check` (and `build`) over every
`examples/*.x3` and fails on any of them, so the count stops being a manual
observation; `cargo test --workspace`; the sweep's `check` count becomes 23/23 for
`examples/*.x3` or the retired files are out of the glob and the gate says so.

## TICKET-084 — `x3c check` reports a parse failure as a lowering failure — CLOSED
Type: CLOSED in `47e944662` (2026-09-19) · Subsystem: x3-lang/crates/x3-common + x3-tools
Closed: `X3Error` gains `stage()` and `staged()` — one mapping, next to `span()`, exhaustive
over the twelve variants so a new variant cannot be added without a stage — and the two
call sites that hard-coded `"lowering failed"` ask the error instead. Measured before and
after on the same file:
```
before  x3c: lowering failed: Parser error: expected top-level item
after   x3c: parsing failed: Parser error: expected top-level item
```
`cli.rs::check_names_the_failing_stage_rather_than_assuming_one` pins both directions: a
syntax error says `parsing failed` and **not** `lowering failed`, and a semantic error says
`semantic check failed` and **not** `lowering failed`. The inner message already named the
right stage in every case — the prefix is what a grep finds, and the prefix was the lie.
Original entry:
Reason: five of TICKET-083's six failures print
`x3c: lowering failed: Parser error: expected top-level item`. The message names
`lowering` and the error names `Parser`, so a reader is sent to the wrong stage of the
pipeline — and a reader who greps for the lowering pass will not find the defect. The
pipeline appears to label whatever stage failed with one word for the whole front end.
Acceptance criteria: the reported stage is the stage that failed: a syntax error is
reported as a parse failure, a name/type error as a semantic failure, and only a
genuine IR-construction failure as `lowering failed`. The error's own text stays as it
is; only the attribution changes.
Validation: `x3c check examples/arb.x3` (or another file with a syntax error) prints
`parse` rather than `lowering`; a file with a semantic error still prints `semantic`;
`cargo test -p x3-tools`; grep the tests for assertions on the string `lowering failed`
that were pinning the wrong word.

## TICKET-085 — the register allocator is a record, and the doc claimed it was a pass — CLOSED
Type: CLOSED in `b8ea24aec` (2026-09-19) · Subsystem: x3-lang/compiler (+ vm)
Closed: **deleted**, and the evidence is not "it was unwired" but *what the ISA is*.
`compiler/src/regalloc.rs`, the `pub mod`, the two re-exports (`allocate_registers`,
`RegisterAllocationResult`), `compile_program_with_regalloc`,
`compile_program_with_regalloc_str` and `mod regalloc_wiring_tests` are gone — the last
including the byte-identity ratchet PHASE 42 added, which did its job: it pinned the pass as
a record rather than a rewrite, and this is the outcome the acceptance criteria documented
for it.
Why deletion rather than wiring: the `Operation` set is economic records (`Lock`, `Swap`,
`Bridge`, `Require`, `MultiHopSwap`, …) — there is **no `Operation::Add`, no
`Operand::Reg`, and no def/use slot on any variant** — so `compute_live_ranges` computed
ranges over temporary ids no operation declares, and `patch_operation` had nothing to
rewrite. The VM's register file exists for one purpose (`REQUIRE` reads `r0`); operands carry
thresholds and constants, not register numbers. Wiring would mean building a register machine
under a language that does not have one and changing the artifact format to carry operand
slots — a language redesign, not a wiring job, and nothing in the phase set asks for it.
**The deletion is provably invisible to the artifact**: five examples built with the compiler
from before and after, in a worktree of the prior commit, are byte-identical
(`arb_scope 97d93d5647c6d450`, `arb_solana_eth 3171984cad52df9c`, `flagship_b52
f36e7cd8156fae46`, `strategy_module 5e51e54a568d1843`, `trading_core_v1 bcebd01e8c557837`),
and the conformance sweep is unchanged at 19/19.
**The two `pub HashMap`s this ticket carried are resolved by deletion** — `register_assignments`
and `spill_slots` went with the file, so the census this session classified is now 50 mentions
across 12 files (was 68 across 14), and `temp_to_reg.values().any(…)` — the only map
*iteration* the census found — went with it. The word "placeholder" is out of the tree.
1116 tests / 0 failed (the seven fewer are regalloc's four unit tests and three wiring tests).
Original entry:
Reason: found while writing PHASE 42's determinism gates. `compile_program_with_regalloc`
said it "promotes `regalloc::allocate` from a library-only function to a real pass in the
production compilation pipeline", and that without it the linear-scan allocator "is dead
code as far as the compiled binary is concerned". Measured this round:
- `regalloc::patch_operation` is an **empty function** — all three parameters are
  underscored, and the doc explains that the v0.1 IR carries no explicit operand slots
  to rewrite. `rewrite_operations` carries
  `#[expect(dead_code, reason = "v0.2 register allocator placeholder")]`.
- `compile_program_with_regalloc` is called **only from `regalloc_wiring_tests`** in
  `compiler/src/lib.rs`. `x3c build` uses `compile_program`.
- Nothing downstream reads the allocation: `vm/src` never mentions
  `register_assignments` or `spill_slots`, and the `AllocationResult` goes to a caller
  nothing calls.
- Therefore `let _alloc = allocate(&ir.operations)` cannot change the artifact, which
  `the_regalloc_entry_point_emits_the_same_bytes_as_the_plain_one` now **measures**
  (the test passes) rather than assumes.
`e56d1dbb7` corrected the doc to state the measured truth and landed that test as a
**ratchet**: it fails the moment the rewrite is wired, forcing the doc and the test to
be updated together instead of letting a half-wired allocator move consensus-relevant
bytecode with nothing watching (PHASE 42).
Acceptance criteria: the allocator either becomes a pass or stops being described as
one. If it becomes a pass: the IR gains an explicit operand representation
(`Operand::Reg(usize)` / `Operand::Spill(StackSlot)`), `patch_operation` rewrites
`ir.operations`, `emit_x3ir` encodes the assigned registers, the VM's operand decoding
agrees with the encoding (this is a **format** change and needs the version-binding
treatment PHASE 45 gave the artifact), and `rewrite_operations` loses its
`#[expect(dead_code)]`. If it does not: `compile_program_with_regalloc`,
`allocate_registers` and `rewrite_operations` are deleted or moved behind a
non-`pub` feature, and the ratchet test becomes an assertion that no such entry point
exists. Either way the word "placeholder" leaves the tree — placeholder logic is on the
project's forbidden list.
Added by the PHASE 42 census (`cb72223a4`): `AllocationResult::register_assignments` and
`spill_slots` are **`pub HashMap`s**, and they are the one hash-type trap the census left
in place — half-fixing them would prejudge this ticket. If the allocator is wired they
should be `BTreeMap` for the reason `Operation::Emit::data` now is: the first caller that
*lists* an assignment would otherwise get hash order, since `RandomState` is seeded per
map instance. If it is deleted, they go with it.
Validation: if wired, a test that the emitted bytecode **differs** from the plain
path for a fixture with register pressure (so the ratchet is deliberately inverted in
the same commit), that a spilled temporary round-trips, and that the byte-identity test
this round added is replaced rather than deleted; if deleted, `cargo test --workspace`
and a grep showing no `compile_program_with_regalloc` outside this ticket.

## TICKET-086 — the unordered-map half of PHASE 42 is surveyed, not classified — CLOSED
Type: CLOSED in `cb72223a4` (2026-09-19) · Subsystem: x3-lang/{compiler,vm,crates}
Closed: all **68** `HashMap`/`HashSet` mentions were classified item by item, and the
classification found a live defect rather than confirming the survey. Two sites reached
the artifact's bytes: `Operation::Emit::data` (rendered by the emitter with `{:?}`, so a
`HashMap`'s iteration order — seeded per map instance by `RandomState` — went straight
into the bytecode) and `Operation::RouteScore::weights` (collected by `.iter()` into the
payload vector). **Measured before the fix: twelve identical compiles of one program
produced six distinct artifacts**, differing only in the payload's key order; after the
fix, twenty-four compiles produce one. Both fields are `BTreeMap` now, the reproduction
is kept as a language-level test
(`test_determinism_audit.rs::an_emits_payload_map_does_not_reach_the_artifacts_byte_order`),
and the property test that had asserted "IR emission must be deterministic" since it was
written was generating a **single** weight — a one-entry map has no order to disagree
about, which is precisely why it passed; it now generates three.
The rest of the census, recorded so the row can be closed rather than re-surveyed:
`risk.rs`'s `categories.values().sum()` and `regalloc.rs`'s `temp_to_reg.values().any(…)`
are order-independent; `verifier.rs`'s returned `HashSet<usize>` is only `contains`-queried;
`bridge.rs`'s `storage_map`/`lifecycle_states` are keyed get/set and its
`for signature in signatures` loops iterate slices; `jit.rs`'s `hit_counts` is
incremented and read by key; `numeric.rs`/`semantic.rs`/`trading_semantic.rs`/`symbol.rs`
are membership and keyed lookup (`semantic.rs`'s eight `HashSet`s are all `contains`).
One remaining **iteration** was a decision rather than a finding:
`crates/x3-common/src/source.rs::files()` returned `self.files.values()`. `FxHashMap` has
a fixed seed, so that order is reproducible — but reproducible is not *canonical*, and
the function had no callers, which is when a trap is cheapest to remove. It is now
ordered by `file_id` (insertion order), pinned by a test using eight reverse-alphabetical
files so a small map cannot agree with a hash order by chance.
What this ticket did **not** produce: a gate that fails when a new file gains a
`HashMap`. The classification is an artifact
(`.ai/reports/x3lang-round57-20260919.md`) rather than a test, deliberately — a
file-level allowlist would not catch a new *ordering-sensitive use* in an already-listed
file, and a per-site regex gate on Rust source has the false-positive problem
`contains("rand::")` demonstrated. The two tests that do bite are the byte-identity ones,
which fail on any non-determinism however it is introduced, and those are stronger than a
type scan. A future gate, if one is wanted, should be written against those.
Original entry:

Type: OPEN, high (determinism) · Subsystem: x3-lang/{compiler,vm,crates}
Reason: PHASE 42 forbids a consensus-affecting decision from depending on an unordered
map. The tree has **68** `HashMap`/`HashSet` mentions across 14 files:
```
compiler/src/semantic.rs 12   compiler/src/regalloc.rs 12   vm/src/bridge.rs 7
compiler/src/risk.rs 7        compiler/src/numeric.rs 6     compiler/src/trading_semantic.rs 5
vm/src/verifier.rs 3          vm/src/jit.rs 3               crates/x3-common/src/source.rs 3
compiler/src/linter.rs 3      compiler/src/ir.rs 3          crates/x3-common/src/symbol.rs 2
compiler/src/opportunity.rs 1 compiler/src/lowering.rs 1
```
`e56d1dbb7` surveyed the *iteration* sites and found exactly one — `temp_to_reg.values()
.any(|&v| v == *r)` in `regalloc.rs`, an existence test and therefore order-independent —
with the rest appearing to be keyed lookups and membership tests (e.g.
`semantic.rs:1069` builds a `HashSet<&str>` of known bridge adapters for `contains`).
That is *consistent with* determinism and is not proof of it, which is why PHASE 42's
ledger row stays PARTIAL. The phase's row cannot honestly become DONE on a survey.
Acceptance criteria: a per-site classification of all 68 mentions in a checked-in
artifact (`.ai/reports/` or a module doc), each site marked lookup-only, membership-only,
iteration-order-independent by construction, or **ordering-sensitive** with what makes it
safe (explicit sort, `BTreeMap`, canonical tie-break). Any ordering-sensitive site is
fixed. Then the classification becomes a gate: a test that fails when a file gains a
`HashMap`/`HashSet` that is not on the list, and when a listed *iteration* site changes —
the same two-directional shape the sweep gate in TICKET-083 needs.
Validation: the classification covers 68/68 sites; `cargo test --workspace`; the gate
fails on a deliberately added `HashMap::new().values()` in a consensus crate.

## TICKET-087 — nothing survives a restart, so PHASE 48's last six cases cannot be tested — CLOSED
Type: CLOSED in `c4b3a1621` (2026-09-19) · Subsystem: x3-lang/vm (+ compiler)
Closed: the answer is **nothing survives a run**, recorded per candidate in
`.ai/reports/x3lang-ticket087-decision-20260919.md` — the open atomic plan is a region of
the artifact and re-executing reaches the same state (PHASE 42's determinism), the trading
journal is derived from execution and `TradingVm::profit` already refuses when the
assembled profit disagrees with the recorded delta, the receipt is already the durable
form (PHASE 45's version binding, TICKET-017's legs), and the finality view is the
caller's to supply because a cached view is a stale view. The consequence is stated rather
than assumed: **an interrupted atomic plan is not resumed and not executed** — the
fail-closed reading, which is legitimate because a plan that did not commit has no receipt
and a plan with no receipt has nothing a settlement layer can act on.
Then the six became tests, and answering the question found one piece of state that *did*
outlive a run: `ProductionBridgeAdapter::storage_op` and `lifecycle` reached two
`static Mutex<HashMap<…>>`, so the host storage a program's `storage_store`/`storage_load`
uses belonged to the **process** — two adapters in one process shared it and `COUNT` grew
across runs, so the same artifact over the same inputs answered differently depending on
what had run before. Reachable from the language (`Operation::StorageOp` →
`CapabilityPayload::StorageOp` → `bridge.storage_op`). Both are fields on the adapter now,
and the reproduction (`bridge::tests::two_adapters_do_not_share_host_storage`) was run
against both versions: FAILED before, ok after.
The six: `restarts_and_kills_leave_nothing_behind` (also covers process kill),
`a_receipt_exists_only_for_a_committed_plan`, `an_unreachable_second_domain_rolls_the_plan_back`,
`a_host_that_stops_answering_mid_plan_rolls_the_plan_back`,
`a_reorged_finality_view_refuses_a_proof_that_was_final`, and ledger corruption — which
round 58 wrongly counted as uncovered; `vm/tests/trading_receipts.rs` covers it with six
tests, now named in the matrix. `dropped transaction` remains **unverified** rather than
covered: its keyword count is too noisy to call, and it is stated that way on purpose.
Residual recorded rather than fixed: `lifecycle`'s `kind 0` ("query") returns its argument
rather than the recorded state, so the lifecycle map is write-only — what a query should
answer is a bridge-semantics decision.
Original entry:
Reason: PHASE 48 names 30 hostile cases; 23 have tests that assert a refusal and 6 do
not, and the six are one family:
`partial domain outage`, `RPC outage`, `reorg simulation`, `ledger corruption`,
`restart during execution`, `process kill`. Measured hits for each: 1, 1, 2, 2, 0, 0
(`.ai/reports/x3lang-phase48-matrix-20260919.md`).
They cannot be written yet, and the tree says why rather than leaving it to inference:
`TradingState` derives `Debug, Clone, Default, PartialEq, Eq` and **not**
`Serialize`/`Deserialize`; `TradingVm` has no persistence; and the atomic rollback
snapshot is `VMState::atomic_snapshot: Option<VmSnapshot>`, a field in memory with no
reason to believe a killed process leaves it anywhere. **There is nothing yet to restart
*from*.** Two partial answers already exist to build on — PHASE 45's version binding (a
restarted reader cannot silently interpret old bytes) and TICKET-017's receipt legs (a
restarted verifier can re-derive the quote age instead of trusting a conclusion) — and
what neither answers is where the half-executed plan lives.
Acceptance criteria: decide and write down **which state outlives a process**, for each
of: the open atomic plan, the trading journal (`balances`, `open_debts`, `credits`,
`debits`, `costs`, `cost_ledger`, `bindings`, `leg_quote_windows`), the receipt, and the
finality view. For each, either it is persisted (with a documented format, a version, and
what a reader does with a torn or corrupt record) or it is *derived* from something that
is (with the derivation written down and the reason it is sufficient). "Nothing is
persisted, and a killed process loses the plan" is an acceptable answer **if it is an
answer** — the fail-closed reading is that an interrupted plan is simply not executed,
and that has to be stated rather than assumed. Then, and only then, the six cases become
tests: a restart mid-plan, a kill mid-plan, a corrupted ledger record, an RPC that stops
answering mid-plan, a domain that is unreachable after the other domain committed, and a
finality view in which a previously-final block stops being one.
Validation: for each of the six, a test that drives the state through the failure and
asserts the documented outcome — for the persist-and-recover path, that a restarted
reader reaches the same conclusion as an uninterrupted one; for the fail-closed path,
that a restarted reader refuses rather than guessing. Plus `cargo test --workspace`, and
the matrix's six rows flip from "not covered" to a named test for each.

## TICKET-089 — a whole-number percent guard eats the clause that follows it — CLOSED
Type: CLOSED in `4d5afba72` (2026-09-19) · Subsystem: x3-lang/compiler/src/parser.rs
Closed: `1%` is now a percentage in a guard's bound. Before, the literal reader implemented
only `Int.Dot.Int.Percent`, so `1%` kept its `%` and the expression parser took it for the
**modulo** operator, consuming the next token as its right-hand operand — in a guard, the
next clause, which is why `require slippage <= 1%` followed by `timeout 45s …` was refused
with `unexpected clause in intent body: Ident("45s")`, naming a correct line.
**Where it was fixed, and the correction to this ticket's own criteria.** The first attempt
implemented `Int.Percent` in the literal reader — what that function's comment promises —
and broke `compiler/tests/test_parser_coverage.rs::expression_arithmetic`, whose fixture
`let x = 1 + 2 * 3 - 4 / 5 % 6;` is a modulo between integer literals. So the fix moved to
`parse_guard_bound`, a two-token lookahead at the guard's bound, where the language
documents the percent spelling; the literal reader's comment now states which form it
implements and that the omission is deliberate. The criteria's "`%` remains a modulo where
a modulo is meant, so fix at the guard's value path rather than by removing `%` from the
operator table" is satisfied — the table is unchanged — with one refinement the criteria did
not anticipate: inside a guard's *bound*, `Int %` is now claimed as a percentage, so a
modulo between literals there needs parentheses. That narrowing is deliberate and pinned by
`a_percent_in_a_guard_bound_is_a_percentage_and_a_modulo_everywhere_else` (three cases:
non-literals, integer literals in a `let`, and `1%` in a bound), because the spelling it
replaces produced a bound whose value was the text `5 Percent 6`.
**What it found in the tests**: `test_slippage_units.rs::the_mainnet_ceiling_reads_the_same_unit_as_the_guards`
was **passing for the wrong reason** — its row `("require slippage <= 5%", "500", false)`
was satisfied by the parse failure rather than by the ceiling comparison, and a second row
(`501` against a `501` ceiling) was wrong in the same direction. The check refuses only what
is *above* (`bound > policy`), so equal is inside. The table is now five rows that each test
the boundary they describe, with two percent rows pinning `5%` as 500 and `0.5%` as 50 basis
points. Nothing in the compiler changed to make them pass.
Measured before and after on the known-good `examples/timeout_refund_minimal.x3` with one
guard inserted before its `timeout`: `50` OK, `0.5%` OK, `1%` FAIL, `2%` FAIL before;
all four OK after. 1122 tests / 0 failed.
Original entry:
Reason: found while checking whether `examples/arb_solana_eth.x3` was stale. It is not —
its syntax is current — and hunting that turned up this. Measured, from
`examples/timeout_refund_minimal.x3` with one guard inserted before its `timeout` clause:
```
require slippage <= 50      OK
require slippage <= 0.5%    OK
require slippage <= 1%      FAIL  parsing failed: unexpected clause in intent body: Ident("45s")
require slippage <= 2%      FAIL  parsing failed: unexpected clause in intent body: Ident("45s")
require delta <= 0.01%      OK    (parses; then a semantic error, unrelated)
```
The split is whole versus fractional, and the parser's own comment at
`parse_whole_percent_bps` says why: "`70%` is an integer followed by `%`, while `0.5%` is a
single percentage literal whose text still carries the sign". The fractional form is one
token and is handled; the whole form is `Int` then `Tok::Percent`, and in a `require`
guard the value goes through the general expression parser — so `1 %` is read as a binary
modulo that keeps looking for a right-hand operand, and **the next clause is consumed as
that operand**. A guard written as the last clause of a body therefore works and one in
the middle does not, which is why this survived: `require delta <= 0.01%` in
`test_hyperarb.rs` is fractional, and the examples that pass hold no whole-number percent
guard before another clause.
Why it is high rather than cosmetic: the refusal names the **next** clause
(`unexpected clause in intent body: Ident("45s")`), so the author is sent to a line that is
correct — and the risk is not only the error path. A form written as `require slippage <=
1 % 50` would parse as a modulo and mean something no author intended, silently. `1%` is
the most natural way to write one percent.
Acceptance criteria: `require <kind> <= <whole>%` parses exactly as `require <kind> <=
<fraction>%` does, converts to the same basis points, and does not consume the following
clause; `%` remains a modulo operator where a modulo is meant, so the fix is at the
guard's value path (accept a number followed by `Tok::Percent` as a percentage literal, as
`parse_whole_percent_bps` and the rebalance-weight parser already do) rather than by
removing `%` from the operator table. If a percent guard *cannot* be supported in some
position, the parser must refuse it with a message naming the guard rather than letting the
next clause be swallowed.
Validation: a table test over the guard kinds and both spellings (`50`, `50%`, `1%`,
`0.5%`, `0.01%`) asserting the same value and that a clause after the guard still parses;
a test that `require slippage <= 1%` followed by `timeout` checks; a test that a modulo
expression still parses as a modulo; `cargo test --workspace`.

## TICKET-090 — the formatter wrote a comment where a percent literal was — CLOSED
Type: CLOSED in `6f6bc0463` (2026-09-19) · Subsystem: x3-lang/compiler/src/formatter.rs
Closed: found while checking whether `examples/arb_solana_eth.x3` was formatter-stable.
`x3c fmt` on it produced
```
    require slippage <= /* literal */
```
— a **comment** in place of the literal, which is source the parser rejects, so formatting
a file could make it unparseable. The cause was the literal match's catch-all:
`_ => self.write("/* literal */")`, reached by `LiteralExpr::Percentage` (and by
`RawString`, `ByteString`, `Char`, `Byte`, `Size`). The first is reachable from the
language — `require slippage <= 0.5%` is the phase's own example — so the defect was live
for any program with a percent bound.
`LiteralExpr::Percentage` is rendered now (`value.as_str()`, which already carries the `%`
because the parser builds it that way). The remaining five have **no arm in the parser that
produces them**, so a parsed program cannot contain one and they are unreachable from
source; the fallback now says so and carries a `debug_assert!(false, …)`, so a hand-built
AST that reaches it fails loudly rather than being silently rewritten into something that
looks like a literal and is not.
Verified by the round-trip test over `examples/*.x3`
(`compiler/tests/test_formatter_roundtrip.rs::formatting_an_example_preserves_its_meaning`),
which is what caught it, and by `x3c fmt --check` reporting `already formatted` on the
example afterwards.

## TICKET-091 — the Python surface reads 3 of the 19 examples, and that scope was unstated — CLOSED
Type: CLOSED in `97984a30d` + `bea1bd07f` (2026-09-20) · Subsystem: x3-lang/cli.py, runner.py,
typechecker.py, registry.py
Closed: the halves were the crash/scope half (`97984a30d`) and the **drift** half, which this
rounds out. Measured over the 19 examples:
```
before:  3 read, 16 refused  (NO_INTENT ×10, REQUIRE ×3, RECEIVER ×2, OPERATION ×1)
after:   9 read, 10 refused  (NO_INTENT ×10), no crashes either way
```
Every remaining refusal is the **stated scope** — a file whose subject is a compiler program (a bare
`atomic_choice`, `strategy`, `parallel`, …) has no `intent` to offer this surface. The other three
reasons were drift, and all three are gone:
- **guard kinds.** The surface knew nine of the compiler's eighteen names and refused `require
  route_score >= 90` — written by three shipped examples the compiler accepts — as `malformed
  require`. It also held the list *twice*: `registry.REQUIRE_KINDS` and nine hand-written branches in
  `cli.py`. The branches are one rule now (a comparison is `<kind> <op> <value>`, a bare guard is
  `<kind> <value>`) reading the registry's list, because a kind this surface does not *model* is still
  carried — `rust_intent_envelope` passes `requires` to the compiler verbatim, so refusing one was
  claiming authority over a list it does not own.
- **address shape.** Exactly forty hex characters was required, where the compiler validates no
  address shape at all — so `examples/arb_scope.x3` (`receiver 0xA1`) and `examples/intent_fusion.x3`
  (`0x1`) were refused. Any `0x`-prefixed hex is accepted; a string that is not an address shape
  (`not-an-evm-address`) is still refused, which is the typo the check is for.
- **`fallback`.** A block the compiler has (`Operation::RouteFallback`) and this surface did not, so
  `examples/route_fallback.x3` was `unsupported route operation 'fallback'`. Read as a block now:
  each `replace with <venue> [min_output <n>]` is one approval, and the block's own `require` guards
  stay with the step rather than joining the intent's, which is the division the compiler makes.
  A `fallback` with no `replace` is refused — a block that approves nothing is not a fallback.
**The two tests that keep it.** `test_the_guard_kind_vocabulary_is_the_compilers` reads
`compiler/src/parser.rs::REQUIRE_KIND_NAMES` and compares it with `registry.REQUIRE_KINDS`, allowing
exactly one name the compiler lacks (`proof`, this surface's older spelling, named so it cannot grow
into a habit); proven load-bearing by dropping `route_score` from the Python list, which fails it with
`only there: ['route_score']` — and fails the accept/refuse table too, two independent guards catching
one drift. The table now records a **single** refusal reason, with a comment saying that a refusal
which is not that one is drift.
Validation as measured: `pytest -q` **23 passed** (22 before); x3-lang `cargo test --workspace` 1158
passed / 0 failed; clippy and fmt clean; sweep check 19/19, build 19/19, warning-free 19/19,
run-artifact 18/19.
Landed: the part that was not a scope decision. Five examples made the surface raise
`IndexError: list index out of range` — `_intent_line_index` walks past the last line when a
file's every top-level item is a declaration it skips, and returned that index to
`parse_file`, where `lines[start]` raised. They are `X3_PARSE_NO_INTENT` now, with a line and
a message that says what the file is instead. The neighbouring refusal stopped saying
`expected intent <name> {` at a line that is correct in the language, and **the runner
stopped letting a clean `X3ParseError` escape as a traceback** — the one surface a user runs
had no error surface at all. It returns `{status: error, errors:[…]}`, the shape a typechecker
failure uses, and exits 1. The scope is stated in `cli.py`'s module doc rather than inferred.
**Corrected figure**: the round-62 report said "thirteen of nineteen" refused; the real
number is **sixteen of nineteen** — 3 readable, and 16 refused by name:
`X3_PARSE_NO_INTENT` × 10, `X3_PARSE_REQUIRE` × 3, `X3_PARSE_RECEIVER` × 2,
`X3_PARSE_OPERATION` × 1. The first count was a miscount, and round 63 corrects it in place.
**Still open, and it is the scope decision**: whether this surface should read more than it
does. The two clusters are `X3_PARSE_REQUIRE` (three files use a guard kind it does not know
— `route_score` — where the compiler would accept it) and `X3_PARSE_RECEIVER` (two use a
short `0xA1`-style receiver the compiler accepts and this surface refuses for shape). Both
are drift rather than scope — this surface refuses files the compiler accepts — and the
receiver one is left alone deliberately: relaxing a receiver-address check in an intent tool
is a safety decision, not a formatting one. `X3_PARSE_NO_INTENT` × 10 is scope and is stated.
Original entry (with the uncorrected figure):

Reason: this repository has two parsers for one language. Three drifts were found and fixed
this round, all in the same direction — a spelling the compiler or the **formatter** accepts
that the Python surface did not:
```
proof_complete      the compiler's name for what the surface called `proof`
finality.<chain>    what `x3c fmt` writes for `finality <chain>`
refund <a>          what `x3c fmt` writes for `refund <a> to sender` (the default receiver)
```
Each made the surface reject a file the compiler had just accepted. Writing the obvious gate
— "the surface must read every example" — shows the drift is far larger than three
spellings: **thirteen of nineteen `examples/*.x3` are refused**, for reasons that are the
surface's *scope* rather than a bug:
```
expected intent <name> {        seven examples start with finality_policy or risk_policy;
                                the surface accepts only a file whose first item is `intent`
invalid Ethereum address '0xA1' it validates address shape (40 hex chars) and the examples
invalid Ethereum address '0x1'  use short placeholders
malformed require 'route_score' nine guard kinds known here, eighteen in the compiler
unsupported route operation 'fallback'
list index out of range         four files crash the parser rather than being refused
```
`list index out of range` on four files is the one that is a defect rather than a scope
statement: a parser that crashes on valid input has no error surface at all.
Acceptance criteria: decide and write down what this surface is **for**. If it is a narrow
MVP over one `intent` per file — which is what it looks like — say so in its own module
doc, and make every refusal a named error rather than an `IndexError`. If it is meant to be
a full surface of the language, it needs the same kind list as the compiler
(`registry.py::REQUIRE_KINDS` already carries the drift note), the same address handling,
the items it does not accept, and a parser that cannot panic. Either way the boundary is
stated where a reader finds it.
Validation: a gate shaped like the answer — either "every example is readable" (once the
surface is caught up) or "every example is readable or on the stated out-of-scope list with
a reason, and nothing crashes". `tests/test_surface_drift.py` already pins the example the
suite runs, derived from the suite rather than listed, so the three drifts above cannot
return.

## TICKET-092 — the SVM validator quorum counted stake without verifying signatures (critical)
Type: CLOSED in `6d6dedca6` (2026-09-19) · Subsystem: crates/x3-crosschain-intent/src/proof/svm.rs
Found while fixing TICKET-082, and invisible until that build worked: with 244 errors from
the feature gating, nobody could reach this path.
**The defect.** `verify_svm_validator_quorum` gated its Ed25519 check on
`#[cfg(any(test, feature = "std"))]` **with no `else`**:
```rust
for (pubkey, signature) in signatures {
    ...duplicate check, stake lookup...
    #[cfg(any(test, feature = "std"))]
    { ...VerifyingKey::from_bytes, DalekSig::from_slice, vk.verify... }
    // no `else`
    signed_stake = signed_stake.checked_add(stake)?;   // counted either way
}
```
In a build without `std` and not under `test` — the runtime's **wasm** build, since
`pallet-x3-settlement-engine` depends on this crate with `default-features = false` — the
gate removed the verification and the loop counted the signer's stake anyway. A quorum was
reached on stakes whose signatures had never been checked: a proof naming any validator and
carrying any 64 bytes cleared the threshold. The compiler had been pointing at it —
`warning: unused variable: signature` at the loop — and the warning was read past.
**Fixed** by removing the gate: `ed25519-dalek` is `no_std` with `alloc`, the feature set the
crate already declares for it, so the verification runs in both configurations. Confirmed on
master: `vk.verify(message_hash, &sig)` with no `#[cfg]` above it.
**Why it survived**: `verify_quorum_requires_threshold_met` — the one test whose subject is a
fake signature — asserted `result.is_err() || result.is_ok()`, which is true for every value,
under a message saying "signature verification should be attempted". It asserts
`Err(InvalidSignature { .. })` now, and a new test performs the forgery properly: a real key,
a real validator of the set, a signature over the **wrong message** whose stake alone would
clear a 1/3 threshold, so nothing but the verification can refuse it — plus the honest
signature reaching the threshold, so the test cannot pass by the fixture being unusable.
**A sweep for the same shape found no others**: every other `#[cfg(any(test, feature = …))]`
in the repository gates a *mock* implementation on `test`/`dev-mock`/`test-utils`, which is
the correct pattern.
Validation: `cargo check -p x3-crosschain-intent --no-default-features` 0 errors;
`cargo test -p x3-crosschain-intent` 75 + 40 + 5 passed; clippy clean; the pallet builds in
both configurations; the forgery test refuses as `InvalidSignature`.

## TICKET-093 — five crates declare `no_std` and cannot build that way — CLOSED
Type: CLOSED in `7b54cadb8` (2026-09-20) · Subsystem: crates/ + runtime
Closed with **the gate's known list empty**, which is the ticket's own acceptance criterion:
```
$ bash scripts/check-no-default-features.sh
no-default-features: 87 workspace crates declare a no_std posture
87 ok, 0 known, 0 failed                                   (exit 0)
```
Two came off when the gate was written (`984b9aed6`); the remaining three came off here, measured
before and after: `x3-gateway-risk-engine` **7 → 0**, `x3-external-chains` **184 → 0**, `x3-sdk`
**216 → 0**.
**`x3-gateway-risk-engine` builds bare.** Its seven errors were one class — `String`, `format!` and
`.to_string()` written with only `Vec` imported. `sp_std` is not the way to get them: at
`default-features = false` it stubs `Vec` so the *name* resolves, and **`sp-std` 14 has no `alloc`
feature at all** — my first attempt added one and resolution refused it. The crate takes `alloc`
directly (`extern crate alloc; use alloc::{format, string::{String, ToString}, vec::Vec};`), which
is part of the distribution rather than a dependency to declare, so the configuration now builds
something rather than merely resolving names and the gate needs no feature to know about.
**`x3-external-chains` and `x3-sdk` stopped claiming a posture they lack.** Both declared
`#![cfg_attr(not(feature = "std"), no_std)]` and then wrote `std::` throughout — an unresolved `std`
on every one of 184 and 216 errors, which is what a `no_std` path that has never been compiled looks
like. The acceptance allows exactly this resolution ("stops declaring a `no_std` posture it does not
have … a claim that cannot be built is a claim rather than a capability"), so the attribute is gone
and each file says why. **No build regresses**: the attribute is `cfg_attr(not(feature = "std"), …)`
and every build in the repository uses default features, so removing it is a no-op by construction;
the gate derives its list from that attribute, so the false entries went with it. Neither crate
gains a dependency.
Validation as measured: the gate 87/87 with no known entries, exit 0;
`SKIP_WASM_BUILD=1 cargo check --workspace` 0 errors; the three crates' tests 75 + 6 + 2 pass (one
`x3-sdk` failure is pre-existing and environmental — see TICKET-102); clippy and fmt clean; x3-lang
`cargo test --workspace` 1161/0; pytest 23; sweep 19/19/19/18.
Original entry:
Reason: found by the gate round 68 landed (`scripts/check-no-default-features.sh`), which
derives every crate that declares a `no_std` posture and checks it **alone** with
`--no-default-features` (plus its own `alloc` feature when it declares one). **5 of 87**
cannot build:
```
x3-chain-runtime          1 error  (E0599)
x3-external-chains      184 errors (E0433 ×87, E0412 ×55)
x3-gateway-risk-engine    7 errors (E0412, E0599)
x3-liquidity-core         3 errors (E0599)
x3-sdk                  216 errors (E0412 ×147, E0433 ×31)
```
The range matters: `x3-chain-runtime` is one missing method, while `x3-sdk` and
`x3-external-chains` have ~200 errors each, which is what a `no_std` path that has never been
compiled looks like — missing types, unresolved paths and missing macros throughout. There is
no `alloc` feature on any of the five, so `--no-default-features` is the only non-std
configuration they offer, and it is the one the runtime's wasm graph reaches for the crates
in that graph.
**Two came off the list in `984b9aed6`, the same day it was written:**
```
x3-chain-runtime   frame-support/tuples-96 was in the crate's `default`, so
                   --no-default-features turned it off and construct_runtime! failed with
                   "the number of pallets exceeds the maximum number of tuple elements".
                   It is on the dependency now. (My first measurement said "1 error"
                   because I ran it without SKIP_WASM_BUILD=1, which the gate sets — the
                   "error" was the runtime's build script, not the crate. Measure in the
                   gate's own environment.)
x3-liquidity-core  it scored a pool in `f64`: two supply shares and
                   `ln(lock_duration) * 20 / ln(30 days)`. `ln` is not in `core`, so the
                   crate could not be built without `std`; and a platform's `ln` is the
                   platform's, which is PHASE 43's prohibition for a decision like this.
                   Exact integer ratios now, and `ilog2(d) * 20 / ilog2(30 days)` for the
                   duration term — the same ratio (`ln a / ln b == log2 a / log2 b`) with no
                   float. The sub-score can differ from the old one by at most one point of
                   twenty; `anti_rug.rs` had no test module at all, and has one now.
```
The remaining three are on the script's `KNOWN_UNBUILDABLE` list with their measured
counts, so the gate fails on anything **not** on it and the list cannot grow silently.
Acceptance criteria: each crate either builds with `--no-default-features` (plus `alloc` if
it declares one), or stops declaring a `no_std` posture it does not have — the second is
acceptable and is what TICKET-082's own criteria allowed, because a `#![cfg_attr(not(feature
= "std"), no_std)]` that cannot be built is a claim, not a capability. **The list in
`scripts/check-no-default-features.sh` must be empty at the end**, and this ticket closes
with it empty.
Validation: `bash scripts/check-no-default-features.sh` reports 87 of 87 and no known
entries; the five are individually measured before and after; `cargo test --workspace` and
`cargo check --workspace` unchanged (a crate leaving the list must not gain a `std`
dependency it did not have).

## TICKET-094 — the repository outside x3-lang has 551 files with float arithmetic, unclassified — CLOSED (classification complete; conversions landed)
Type: CLOSED in `64899f004`, `5b139733c`, `a02aa07f7`, `93d762b5c`, `883078fbc` (2026-09-20) · Subsystem: crates/ (outside x3-lang), pallets/, runtime/
Closed: **the classification is the artifact the ticket asked for, and it is complete for both shapes that
can decide an amount.** The ticket's "551 files" is a *file* count from a pattern that mentions `f64`
anywhere — most of it vendored third-party source (`apps/x3-desktop/src-tauri/tauri-vendor/`) and most of
the rest arithmetic that *stays* float. What can become a balance, a reward, a slash or a settlement
amount is a float that (a) **becomes an integer** or (b) **gates one in a comparison**, and both are
enumerated:
**(a) a float cast to an integer — 39 sites in 21 files** (`python3 /tmp/x3probe/float_sites.py crates`,
plus one method-call shape the token search cannot see: `x3-integration/mini_x3.rs:982` is an
`F64 -> I64` *instruction*, whose conversion is the opcode's semantics). Verdicts:
| cluster | verdict | evidence |
|---|---|---|
| `x3-bridge-adapters/bitcoin.rs` (2) | **converted** | `(0.29_f64 * 1e8)` = 28_999_999 against the exact 29_000_000; a missing amount was `unwrap_or(0.0)` — a UTXO worth nothing |
| `x3-marketplace/fee_distribution.rs` (1) | **converted** | 80% of `10^18+7` **5 short**, of `1.23e19` **168 short**; the marketplace pocketed the difference |
| `x3-atomic-trade/rollback_listener.rs` (1) | **converted** | a 5% compensation **7** and **860** short — underpaying the party whose trade failed |
| `x3-cli/commands/swap.rs` (2) | **converted** | `"0.29"` -> `289999999999999996` wei, and that value **executes**; `format_amount` hid a wei |
| `x3-rpc/gas_estimation.rs` (2) | **converted** | `(3 * 1.25) as u64` = 3 — a gas limit with no margin at all |
| `x3-staking-analytics/*` (7) | **unreachable — recorded, not converted** | a validator's reward share, a commission take, `claimable_after_fee`'s deduction. The crate is **not a workspace member and no crate depends on it**: never compiled, so a conversion changes nothing that runs (TICKET-096's class) |
| `x3-foundry-core/{revenue,simulator}.rs` (13) | **unreachable — recorded, not converted** | the six-way revenue split, where **95,000 of the first 100,000 totals do not sum to the total** (worst shortfall 5). Members with **no dependents** |
| `x3-evolution/{lib,selection,mutation,crossover}.rs` (5) | **a search step** | the float is the mutation delta and the `u64` is a gene — a strategy parameter the GA scores, not a balance |
| `quantum-swarm/*` (7) | **counts and indices** | swaps, kill counts, qubit counts (`log2`), tournament rounds |
| `contention-predictor/model.rs` (1), `x3-gpu-validator-swarm/metrics.rs` (1), `x3-opt/peephole_autogen.rs` (1), `orchestra/jury/{session,rotation}.rs` (2), `x3-sidecar/benchmark.rs` (1) | **metrics, quotas and noise** | a confidence score clamped to 10 000 bps; a percentile *index*; a mutation rate's noise; a jury quota and a rotation count; `p50_latency_ms` |
**(b) a float comparison on an amount-shaped name — 10 sites in 4 files**: `x3-evolution/simulator.rs` (peak/previous-value guards in a simulator), `quantum-swarm/strategy/portfolio.rs` (a simulator's total), `contention-predictor/lib.rs` (feature thresholds for a block-count classifier), `x3-staking-analytics/reward_calculator.rs` (a zero-balance early return, in an unreachable crate). All are a heuristic or a simulator's own guard rather than a balance.
**The consensus surface is clean and now gated**: `pallets/*/src` and `runtime/src` have **zero** `f64`/`f32`
(187 files), held by `scripts/check-no-float-in-consensus.py` in `local-ci`, with
`// float-exemption: <reason>` for a line that is not a decision.
**What is *not* claimed**: the ~2,700 float uses that never become an integer and never gate one
(analytics, metrics, benchmarks, GA internals) are classified as a *category* with a reason rather than
read one at a time. A float that is compared against another float and then influences an amount
somewhere else would not appear in either enumeration; the two shapes above are the ones a search can
find, and the honest statement is that a per-site read of every float use in 512 files has not been done.
**Residual, recorded rather than fixed**: the decimal->units rule now exists twice
(`x3-bridge-adapters` at 8 decimals, `x3-cli` at 18) because there is no client-side crate to share one
in — `x3-common` is the Substrate-flavoured one; take the decision if a third caller appears.

## TICKET-095 — an amount with no `float` representation ran to completion as `NaN` — CLOSED
Type: was CLOSED by `5ccd725c3` · Subsystem: x3-lang Python pipeline (`numeric.py`, `runner.py`,
`typechecker.py`)
Reason: `parse_decimal` rejected what `Decimal.is_finite` rejects, and then all **nine** of its
callers narrowed the result on the next line — `simulator.py:48`, `planner.py:39,85`,
`runner.py:57,63,85,87`, `typechecker.py:72,137` all read `float(parse_decimal(...))`. `float()`
does not raise on a `Decimal` outside its range; it returns `inf`. So `1e400` — a literal the
parser and the typechecker both accept — was finite at the check and infinite one line later.
Measured before, `intent overflow { from ethereum.USDC amount 1e400 ... }` against HEAD's
sources extracted with `git archive`: `exit 0`, `"estimated_slippage_usd": Infinity`,
`"expected_profit_usd": NaN`, `"status": "rolled_back"`. A run that ended rolled_back on `NaN`
profit, `json.dumps` writing two tokens RFC 8259 does not define, so the runner's own output
document was not JSON — and nothing raised at any point, so nothing said so.
Fix: the check goes in `parse_decimal`, the one place all nine callers pass through. After:
`exit 1`, `X3_INVALID_AMOUNT`, `from.amount`, `"1e400"`. The bound is the pipeline's own
representable range and not a ceiling on amount size — `1.7976931348623157e308` still passes,
`1e309` does not. The message carries the reason (`… (value must be finite)`) because `1e400`
*is* positive and *is* numeric, so the old wording sent its reader to the sign and the spelling.
Tests: `test_typechecker_rejects_an_amount_the_pipeline_cannot_represent` (both directions) and
`test_runner_refuses_an_amount_it_cannot_represent_instead_of_reporting_nan` (end to end, and it
asserts neither `Infinity` nor `NaN` appears in the document).
**Recorded with it, at a different confidence level:** `rust_intent_envelope` and `run` read
`requires`/`policies` with two-argument `get`, whose default covers absence but not null, while
the typechecker decides null means absent (it normalizes `policies is None` to `{}`). A
validated intent carrying `"requires": null` therefore crashed the consumer that blessed it,
and the envelope handed the compiler a `requires: null` its serde `Vec<Requirement>` refuses.
`cli.parse_file` cannot emit either — so this is **contract consistency, not a reachable
crash**, tested as a contract, and it must not be counted as a live defect in any score.
Validation: 21 pytest (19 before); x3-lang `cargo test --workspace` 1119/0; clippy and fmt
clean; the 19-example sweep unchanged at check 19/19, build 19/19, warning-free 19/19,
run-artifact 18/19; every example through `runner.py` yields the same refusal codes as before.
Source: the same two defects were found and fixed on `salvage/x3lang-intent-bridge`
(`fbc095420`, openclaw-agent) and never merged. The salvage patch was 13 lines at `numeric.py`
and 2 at `runner.py`; this lands the same two findings at the choke point instead of at the
readers, plus the typechecker message and the two tests. The branch should not be merged — its
content is in.

## TICKET-097 — the artifact version byte does not move when the opcode set does — CLOSED
Type: CLOSED in `d7b6cba9e` (2026-09-20) · Subsystem: x3-lang/spec/opcodes.rs, compiler emitter, VM verifier
Closed: the policy is decided and it lives where a format change will read it. `spec/opcodes.rs`
now holds `OPCODE_SET: &[(u8, u8)]` — every opcode with the version that introduced it — and the
gate is a **compile-time** assertion: `const _: () = assert!(CURRENT_BYTECODE_VERSION ==
max_opcode_version(), …)`, so registering an opcode at a version the writer does not write fails
the build (`error[E0080]: evaluation panicked: the version byte this pipeline writes is not the
greatest version in OPCODE_SET …`). `is_defined_version` and `is_supported_version` are two
questions asked in order — framing, then compatibility — and the two `has_compiler_header` copies
now use the first, so a version-2 artifact *reaches* the verifier and is refused there instead of
being walked as raw bytecode. `version_refusal`/`opcode_version_refusal` are the one place the two
refusals are worded; the compiler's walker, the trading decoder and the VM's verifier all render
them, and the VM's verify failure is now `Display` rather than `Debug` so `x3c run` shows the
sentence too.
Measured on a real artifact with one byte changed, before (binary from `HEAD`) and after:
```
$ x3c explain gate_v2.x3b          # version byte 0x02        BEFORE
; x3-lang bytecode v0x02
  0001  0x50  ATOMIC_BEGIN
  0002  0x24  SWAP   Swap { … }     ← walked as if it were the reader's own version
$ x3c explain gate_op.x3b          # first instruction 0x03   BEFORE
  0001  0x03  UNKNOWN
  0002  0x24  SWAP   Swap { … }     ← advanced over as a fixed frame
$ x3c run gate_v2.x3b              BEFORE
x3c: error: VM error: Panic("X3_VERIFY_FAILED: OutOfBounds(32)")   ← a shape error, not a version

$ x3c explain gate_v2.x3b          AFTER
x3c: disassembly failed: Codegen error: the artifact states bytecode version 2, and this reader
knows version(s) [1]: an opcode introduced after a version this reader does not know would be read
as a different instruction, or as the length of one, so the artifact is refused rather than walked
$ x3c run gate_v2.x3b              AFTER
x3c: error: VM error: Panic("X3_VERIFY_FAILED: X3_BYTECODE_VERSION_UNSUPPORTED: the artifact
states bytecode version 2, and this reader knows version(s) [1]: …")
$ x3c explain gate_op.x3b          AFTER
x3c: disassembly failed: Codegen error: at pc 12: opcode 0x03 is not in this format's opcode set,
at any version, so it cannot be walked as an instruction
$ x3c run gate.x3b                 AFTER    (the unpatched artifact)
x3c run: ok — 1 asset ops, 0 bridge ops, 0 receipts, gas remaining 999368
```
The VM's range-based `valid_opcode` is **deleted** rather than left beside the table: it accepted
every unassigned byte in `0x00..=0xAB`, so the verifier passed what the executor then refused one
instruction later — its own comment records the cost of a hand-kept range list (the trading range
was missing from it, so `verify` rejected every trading-core artifact).
New `compiler/tests/test_bytecode_version_gate.rs` (6 tests), each proven load-bearing by mutation:
dropping `VENUE_SETTLEMENT` from the registry fails the source-walk gate by name; registering an
opcode at version 2 without moving `CURRENT_BYTECODE_VERSION` fails the build; making
`version_refusal` return `None` fails the version test; making `opcode_version_refusal` return
`None` fails the opcode test. The source-walk gate reads `spec/opcodes.rs` itself and holds a
five-entry documented exemption list for non-opcode `u8` constants, with a second test that fails
when an exemption outlives its constant.
Validation as measured: x3-lang `cargo test --workspace` **1172 passed / 0 failed** (1166 before);
clippy `-D warnings` clean; fmt clean; 19-example sweep unchanged at 19/19/19/18.
**Open, recorded rather than decided:** the policy asserts the *global* greatest version is the
version written, so an artifact containing no new opcode would still carry the new version byte.
Whether the byte should instead be the greatest version among the opcodes an *artifact* contains
(old readers then keep reading artifacts that use nothing new) is a decision to take with the
first version-2 opcode, not before — see TICKET-105.
Original entry:
Reason: found while adding `VENUE_SETTLEMENT` (`0x58`) in `1f976974a`. `BYTECODE_VERSION_1`

Reason: found while adding `VENUE_SETTLEMENT` (`0x58`) in `1f976974a`. `BYTECODE_VERSION_1`
is `0x01`, it is written as the first byte of every artifact and checked by the emitter's
decoder and the VM, and **it is not bumped when an opcode is added**. Every declaration
record added this session and before it — `0x54` `ROUTE_FALLBACK`, `0x55` `PARALLEL_PLAN`,
`0x56` `FEATURE_ALLOW`, `0x57` `STRATEGY_LICENSE`, `0x58` `VENUE_SETTLEMENT`, and the whole
`0x80..=0xAB` capability block — travelled inside the same version.

The consequence is in `spec/opcodes.rs`'s own words about `is_payload_opcode`, which is the
**single** boundary source for the compiler's disassembler and the VM's verifier: "A reader
that classifies a payload-carrying instruction as fixed-width advances four bytes and then
reads bytes that are not instructions, which is how the same defect has surfaced four times
in this format." A reader that predates an opcode has no way to refuse it — the version it
would check is unchanged — so it misparses: for `VENUE_SETTLEMENT` it would read the record's
first three bytes as instructions and continue from a payload byte. That is worse than a
refusal, because a refusal is visible and a misparse is a different program.

Nothing exchanges artifacts across runtime versions today: `x3-lang`'s artifact has no
consumer outside its own workspace (verified — the only mentions of `x3-lang-vm`,
`x3-lang-compiler` and `x3-lang-ast` in other `Cargo.toml` files are comments pointing at the
real crates), and this is pre-mainnet. So the risk is latent rather than live, which is why
this is a ticket and not a change to `1f976974a`. It stops being latent the moment one
artifact outlives the runtime that produced it — a saved `.x3b`, a proof bundle, a replay
fixture in a release tarball.
Acceptance criteria: the policy is decided and written where a format change will read it.
Either the version byte changes when the opcode set changes, with a reader that *refuses* a
version it does not know rather than falling through to the instruction walk; or a statement
that artifacts never cross runtime versions, with the evidence that makes that true. The
decision must come with a gate: adding an opcode cannot be possible without restating it —
the natural shape is a check that the opcode set is a function of the version byte.
Validation: the gate fails on a new opcode added without the decision being revisited, and an
artifact carrying an opcode the reader's version does not include is refused with a named
error rather than misparsed. A test that builds an artifact with a synthetic future opcode
and asserts the refusal is the shape.

## TICKET-098 — a `while`'s condition is computed and then discarded, so the IR's loop does not say what it loops on — CLOSED
Type: CLOSED in `dd07b0bc5` (2026-09-20) · Subsystem: x3-lang/compiler (lowering + IR + verifier + emitter)
Closed: `Operation::Loop` carries its condition, and the class the compiler can decide is decided.
`lowering.rs` folds a loop's guard with the same `fold_condition` an `if` uses and passes the result
into the IR, so a reader sees what the loop tests; `Condition::describe()` is one renderer for the
guard, used by the verifier's refusal and the emitter's so the two cannot disagree about what the
program said. Measured on a program whose loop is `while steps < 10 { require profit >= 5 }`:
```
BEFORE  $ x3c lower …        Loop {max_iterations: 1000}
        $ x3c build …        x3c: … `loop` cannot be executed — … no target it could jump back to
AFTER   $ x3c lower …        Loop {max_iterations: 1000, condition: Expression {expr: "steps < 10"}}
        $ x3c build …        x3c: … `loop` over `steps < 10` cannot be executed — … and no
                             instruction that puts its condition in a register
```
`while 1 > 2 { require profit >= 9 }` — a loop the compiler decides false — was refused before and
now builds, contributing nothing to the artifact:
```
$ x3c build cmp_a.x3   # require profit >= 5; while 1 > 2 { require profit >= 9 }
x3c build: 108 bytes (27 ops) -> cmp_a.x3b
$ x3c build cmp_b.x3   # the same program without the loop
x3c build: 108 bytes (27 ops) -> cmp_b.x3b
$ cmp cmp_a.x3b cmp_b.x3b && echo IDENTICAL
IDENTICAL
```
`while true { … }` is **not** dropped — a folder that read every decision as a licence to delete a
loop would silently delete a program's body — so it is refused, named `` `loop` over `true` ``.
Two rendering defects fell out of the same work and are fixed with it, because a diagnostic that
must name a guard cannot name it in Rust's syntax: `expression_to_string` rendered a binary
operator with `{:?}`, so `while steps < 10` reached the IR and every diagnostic as `steps Lt 10` — a
guard the program never wrote, in a spelling nothing can parse back — and a unary expression fell
through to `{:?}` as a `Unary { … }` debug string. `BinOp`/`UnOp` already implement `Display` with
the language's own symbols; the renderer now uses them.
Tests: `compiler/tests/test_loop_condition.rs` (4) — the condition travels; a decided-false loop is
written as nothing and its artifact is byte-identical to the program without it; a loop the compiler
cannot decide is refused *by name* by both the verifier and the emitter; a decided-true loop is
refused rather than dropped. `test_ir_verifier.rs`'s loop test now asserts the guard is named and
its new sibling asserts a decided-false loop is *not* refused — the pair is what stops either from
passing vacuously.
Validation as measured: x3-lang `cargo test --workspace` **1166 passed / 0 failed** (1161 before);
clippy `-D warnings` clean; fmt clean; sweep 19/19/19/18.
Original entry:
Reason: found while folding decidable branches in `e8aa4ee90`. `Statement::While` lowers as
Reason: found while folding decidable branches in `e8aa4ee90`. `Statement::While` lowers as
```
Statement::While { cond, body } => {
    let _cond_ir = expression_to_condition(cond)?;   // computed, then dropped
    ...
    ir.push(Operation::Loop { max_iterations: 1000, body: body_ops });
}
```
so `while a < b { … }` becomes a loop with a fixed iteration cap and **no condition at all**. The
condition is parsed, converted and thrown away, and the variable's leading underscore is the only
record that anyone noticed. `x3c lower` prints that loop, and `Operation::Loop` has no field a
condition could live in.
It is not reachable as an *execution* defect today: `Loop` is refused by the IR verifier and by the
emitter, for the same reason `if` was — a branch record no reader can walk, and no register to hold
a condition. So this is a latent defect in a refused path, which is why it is recorded rather than
patched into the folding commit.
Acceptance criteria: `Operation::Loop` carries its condition, so the IR states what the loop tests
and a refusal can name it; and either the construct executes (which needs the register codegen
TICKET-058 names) or it is refused with the condition it could not evaluate in the diagnostic.
Validation: `x3c lower` on a `while` program shows the condition; the refusal for a non-decidable
loop names the condition rather than only the construct; a decidable loop either folds like `if`
does now or is refused with a reason that says which.

## TICKET-099 — a simulation does not model a hedge's delta bound, so it refuses such an artifact — CLOSED
Type: CLOSED in `7e23e3738` (2026-09-20) · Subsystem: x3-lang/crates/x3-tools (simulation + CLI)
Closed: the bound is decidable. `ArtifactFloors.delta_ceiling_bps` is read from the delta *unit
code*, taking the smallest ceiling the artifact states, exactly as the slippage ceiling does;
`SimulationSnapshot.delta_bps` is an additive `#[serde(default)]` field; `evaluate` folds the delta
into the verdict, and `Verdict::DeltaAboveCeiling` converts a **non-failing** verdict into a failure
— `NoFloorStated` (a hedge's normal shape: a delta bound and no profit floor, where leaving it alone
would have been the silent pass in the most likely combination) as well as `Pass`. A ceiling with no
stated delta is refused by `SimulationError::DeltaUnstated`, its own variant rather than the slippage
one, because the two name different quantities.
Measured before and after, on a hedge artifact (`require delta <= 0.01%`) with a snapshot stating
`delta_bps: 1`:
```
BEFORE  $ x3c simulate hedge.x3b --state hedge_within.json
        x3c: this artifact states a measured `delta` bound, which a simulation does not model: …
AFTER   $ x3c simulate hedge.x3b --state hedge_within.json --explain
        Net: 0 ethereum.ETH
        Minimum required: none — the artifact states no profit floor
        Delta: 1bps against a ceiling of 1bps — within
        Result: NO FLOOR STATED (net 0bps)
```
The CLI also passes the figure to the VM now: it passed `None` for the delta — a placeholder that was
unreachable only because the artifact was refused before it got that far — so a run judged against a
delta the VM was never told would have refused `X3_GUARD_UNMEASURED` while the report said the bound
held.
**Schema decision, stated with the change**: `SNAPSHOT_VERSION` does not move. The field is additive
and optional, so this build reads a snapshot written before it; the compatibility is one-way on
purpose, because `deny_unknown_fields` makes a build from before refuse a snapshot that has it — the
fail-closed direction, since an older reader that ignored a delta would compare nothing and report a
verdict as if there were no bound. The version number is for changes an old reader would silently
*mis-read*.
Validation as measured: three unit tests (inside the bound is reported within; outside it is
`DeltaAboveCeiling` with both figures in the report and the result line; a ceiling with no delta is
refused naming the delta and *not* the slippage) and
`cli_simulates_a_hedge_against_its_delta_bound` through the binary, which asserts all three
outcomes — including that the over-bound run does not settle, because the artifact's own guard
refuses it now that the figure reaches the VM. x3-lang `cargo test --workspace` **1181 passed / 0
failed** (1177 before); clippy `-D warnings` clean; fmt clean; sweep 19/19/19/18.
Original entry:
Reason: found while closing TICKET-068 in `cfedbbe43`. A hedge's delta bound is a measured guard
with the **profit's** comparison mode and the delta's unit code; the simulation's floor reader
(`artifact_floors`) classified by mode alone, so a delta ceiling became `profit_floor_bps` — a
floor compared against a delta, two different quantities, and exactly the units mismatch
`spec/opcodes.rs` says the measured modes exist to prevent. It now **refuses** an artifact stating
a delta bound and names the reason, which is the fail-closed direction; a simulation that skipped
the bound would report a verdict as if the artifact had none.
Acceptance criteria: `ArtifactFloors` carries `delta_ceiling_bps`, `SimulationSnapshot` carries the
delta a venue reported (an optional, `#[serde(default)]` field, so a schema-1 snapshot written
before it still loads), and `evaluate` folds the delta into the verdict — refusing with a named
error when the artifact states a ceiling and the snapshot states no delta, the way
`SlippageUnstated` already does. The snapshot's `schema_version` decision belongs with the change.
Validation: `x3c simulate` on a hedge artifact with a snapshot stating a delta inside the bound
passes; one outside it fails with the figures; one stating nothing refuses rather than passing.
Until then `x3c simulate` on a hedge artifact exits non-zero with the reason, which is why this is
recorded rather than left to be discovered.

## TICKET-100 — a liquidation's conversion cannot report what it realised — CLOSED
Type: CLOSED in `ce8961990` (2026-09-20) — option 3, and **the recommendation on this ticket was wrong** · Subsystem: x3-lang/vm + compiler
Closed: the floor is a **post-condition on the net the venue realised**. It was a constraint derived
from the declared `min_output`, emitted as `REQUIRE static 0`, because the conversion is an
`Operation::Swap` — an asset-op record the executor resolves locally, so no reply could carry what
was seized. Measured:
```
$ x3c explain liquidation.x3b
  0015  0x40  REQUIRE measured profit 100        (was: static 0)

--measured-profit-bps 100  -> x3c run: ok
--measured-profit-bps 40   -> X3_PROFIT_BELOW_FLOOR: the trade realised 40bps and the program
                              requires at least 100bps
(nothing measured)         -> X3_GUARD_UNMEASURED: the guard `profit >= 100bps` needs a profit
                              the host measured
```
**The recommendation this ticket carried was wrong.** Option 1 (lower the conversion as a
`VenueOrder`) would have **dropped the declared `min_output`**: a venue order carries an action, a
subject, an asset and a quantity, and no minimum — so it would have replaced a constraint with a
weaker one and called it a post-condition. The receipt for that mistake is in the ticket's own
text and in round 76's recommendation, which is why both say so now.
What worked is option 3 refined: the quantity the floor is about is the **net the seizure
realised**, and the two venue orders above are the calls that did the seizing — `liquidate` and
`receive_collateral` already reach the host, so the net travels on their reply using the profit
unit that already exists. No new unit, no new vocabulary, no plan-shape change, no format change.
The compile-time check is unchanged (`liquidation::verify` refuses a swap whose declared minimum
cannot repay), so the compiler bounds the plan and the runtime measures it.
My earlier objection to this option — "the net becomes the venue's claim where the plan computed
one locally" — does not hold, and the hedge is the counter-example: its exposure is computed at
compile time *and* its delta is measured at run time. Two figures for one quantity is the pattern,
not the defect, as long as the one that decides settlement is the measured one.
**The CLI rule had to move with it**: `x3c run` refused a stated profit without a stated slippage
("state both measurements or neither"), true of a plan and false of a liquidation. Every quantity
is independent now, because a *program* states which ones it has; the property is unchanged and the
diagnosis is better — a guard whose quantity nobody stated refuses with `X3_GUARD_UNMEASURED`
**naming the guard and the quantity**, where the old message named neither. `cmd_simulate`'s rule
about `--state` versus the measured flags is a different rule and stays.
`DryRunBridge`'s stated outcome is two fields rather than a pair for the same reason.
Tests, each **updated rather than weakened** where the design moved: the liquidation CLI test now
asserts the readable guard, the unmeasured refusal, a net at the floor settling, and 40-against-100
refused with both figures; the plan test keeps its property (nothing is invented) and asserts the
*guard* refuses and names the quantity; `test_measured_delta.rs` asserts a profit reaches a venue
order as a profit and never as a delta, and that a delta reaches only the call a hedge makes.
Validation as measured: x3-lang `cargo test --workspace` **1144 passed / 0 failed**; clippy and fmt
clean; pytest 21; sweep check 19/19, build 19/19, warning-free 19/19, run-artifact 18/19.
Original entry:
Reason: TICKET-069's remaining half, re-scoped after closing TICKET-068 and therefore now cheap to
state precisely. A liquidation lowers to two venue orders (`liquidate`, `receive_collateral`) and a
conversion, and the conversion is an `Operation::Swap` — which `executor::execute_asset_opcode`
resolves **locally** from the declaration's own amounts (`apply_asset_payload`) and never puts to a
host. So no reply can carry what was seized, and the net-profit floor stays `measured: false`: a
constraint derived from the declared `min_output` rather than a post-condition on the trade.
Three ways to close it, and the choice is a language decision rather than compiler work:
1. **The conversion becomes a capability call** — lower it as a `VenueOrder`, so it reaches the
   host's `venue_order` and its reply carries the realised net. Narrowest: it changes the shape of
   a liquidation's plan and adds a word to the venue-order vocabulary, and nothing else.
2. **The asset-op `Swap` path reports an output** — make `SWAP` a host call. Broadest: every
   program that swaps would then depend on the host answering, including the 19 examples.
3. **The venue orders report the net** — reuse `MEASURED_UNIT_PROFIT_BPS` on the reply to
   `liquidate`/`receive_collateral`, since the venue that seized the collateral is the one that
   knows what it seized. No new unit and no plan-shape change, but it means trusting the venue's
   number over the VM's own local accounting — which is what a post-condition is, and the guard
   still refuses when nothing is reported.
Recommendation: (1) on the language's own terms — the phase's honest modelling is that an external
venue's action is asked for and answered, which is what the venue-order vocabulary exists for; (3)
is the smallest but makes the *net* a venue's claim where the plan already computed one locally,
and two numbers for one quantity is the shape TICKET-075 and the measured modes were built to avoid.
Not decided here because it changes what a liquidation's floor *means*, and that belongs in a
commit that argues it rather than one that slips it in beside a mechanism.
Also required whichever way it goes: the CLI's "state both measurements or neither" rule blocks a
program that states only a profit floor, which a liquidation does — it has no slippage guard. The
rule is convenience rather than safety (a measured guard with no stated measurement already refuses
with `X3_GUARD_UNMEASURED`), so relaxing it to allow a subset is safe, but it should be relaxed
*deliberately*, with the reason, not as a side effect.
Validation: a liquidation program runs against a fixture host that reports a net; a net below the
floor is refused at the guard with the figures rather than at compile time; a liquidation whose swap
cannot repay is still refused before any of this.

## TICKET-101 — `Release` means two different things in the IR and nothing says which — CLOSED
Type: CLOSED in `108785c62` (2026-09-20) · Subsystem: x3-lang/compiler (IR + semantic)
Closed: `Release.claims` is `Option<u32>` — `Some(index)` claims a lock, **`None` claims nothing** —
and every producing site says which act it emits, with the reason. In the payload a **tag byte**
precedes the index, so the absence is *written* rather than a sentinel index, and a tag the encoder
never writes gets `CapabilityCodecError::UnknownTag` rather than a fall-through to "no claim".
The acts, as the sites now state them: netting's residual transfer is a **claim** of the escrow
written immediately above it; an atomic swap's destination release, an intent's `to` endpoint and a
bare `release` statement are **payouts** of the asset the route delivered; a timeout refund's
concrete release is **neither** — it *returns* an escrow, the inverse of a lock.
**What the distinction bought.** (1) The range check TICKET-080 had to abandon now exists: a claim
must name a lock its route has written, and a claim outside an atomic route is refused, because a
claim is about a lock *its own route* wrote. (2) `locked_escrows` is **gone**: `release_lock` returns
the escrow only for a release that claims one, so `no_refund_after_claim` no longer needs a second
lookup to tell a payout from a claim — and the helper was then used by nothing and was deleted rather
than left as dead code. (3) `no_double_claim` ignores payouts, which its description always said.
**Three fixtures were corrected rather than retyped.** `a_two_legged_swap_pays_out_and_refunds_
different_assets` is a payout (its own comment already said "the destination asset is paid out with
no lock of its own"), while `invariant_no_double_claim_detects_violation` and
`invariants_still_catch_two_claims_inside_one_route` are two claims of the **same** lock — the
violation the rule names. A heuristic (is a `Lock` nearby?) got all three wrong; the tests' names
said which act each described, and reading them also confirmed the index is the right identity,
since two claims of two locks are the case TICKET-080 made possible.
Validation as measured: a payout with no lock in its route is accepted; a claim on lock #3 of a
one-lock route is refused with the lock and the count; a claim outside an atomic route is refused
with the reason; x3-lang `cargo test --workspace` **1156 passed / 0 failed**; clippy and fmt clean;
pytest 21; sweep check 19/19, build 19/19, warning-free 19/19, run-artifact 18/19.
Original entry:
Reason: found while writing a range check for TICKET-080 in `2e341876e`. `Operation::Release` is used
for **claiming an escrow this program locked** and for **paying out the asset a route delivered**. The
second reading is why `no_refund_after_claim` carries a `locked_escrows` guard — its own comment says a
two-legged swap pays the destination asset out and refunds it on timeout, so a scan that read the payout
as a claim called it a claim-then-refund, and `examples/atomic_swap.x3` warned on every check and build
(TICKET-035). Three rules now infer which reading applies from context: `no_double_claim` counts claims,
`no_refund_after_claim` requires the asset to have been locked, and
`escrows_claimed_in_their_own_route` matches locks against claims by asset.
The range check attempted for TICKET-080 — a claim naming a lock its route does not have — is the case
that made the cost concrete: it refused four cross-chain parallel-plan cases with "the release claims
lock #0 of its route, which has written 0 lock(s) so far", because a payout claims no lock and the IR
has no way to say so. It was reverted rather than shipped, and the code says why.
Acceptance criteria: a release says which of the two it is — a claim carries the lock it names, and a
payout says it claims none (an explicit absence, the way `settlement none` and the empty-shape field
are spelled, rather than a default index). Then the range check can exist: a claim's index must name a
lock the route has, and a payout must not name one. The rules that currently guess read the distinction
instead.
Validation: a cross-chain parallel plan keeps lowering (the four cases that failed); a hand-built route
with a claim naming a lock it does not have is refused naming the lock; a payout with a claim index is
refused; and `no_refund_after_claim` no longer needs `locked_escrows` to tell a payout from a claim —
or says why it still does.

## TICKET-102 — a unit test opens a WebSocket, so `cargo test -p x3-sdk` can never be green offline — CLOSED
Type: CLOSED in `aa4ce2add` (2026-09-20) · Subsystem: crates/x3-sdk
Closed: `rpc::tests::test_ws_client_creation` did `WsRpcClient::connect("ws://localhost:9944").await
.unwrap()` — connecting to a node nobody had started — so it panicked with
`Connection("WebSocket connection failed: IO error: Operation not permitted (os error 1)")` wherever
a socket was not available. It binds a **loopback listener** on `127.0.0.1:0` now, accepts one
connection and completes the handshake with `tokio_tungstenite::accept_async`, then asserts the
client constructed. Measured: **44 passed / 1 failed → 45 passed / 0 failed**, and proven
load-bearing by mutating the constructor to refuse a loopback endpoint, which fails the test with
*"the client must construct against a listener that answers: Connection(\"mutated: no loopback\")"*.
It needs a socket but no **external** network now — this sandbox denies even `bind("127.0.0.1", 0)`,
so it is run under escalation here and would report as an environment failure without it.
**Corrected attribution.** When this ticket was filed it said the root `cargo test --workspace` could
not be green because of this line. That was wrong: the root run aborts earlier, at `e2e_tests`, and
this suite is a different one. The defect in *this* crate is real and fixed; the root command's
blocker is TICKET-103.
Validation as measured: `cargo test -p x3-sdk` **45 passed / 0 failed**; clippy `-p x3-sdk
--all-targets -- -D warnings` and `fmt --all -- --check` clean.
Original entry:
Reason: found while closing TICKET-093 in `7b54cadb8`, where the ticket's validation asks that
`cargo test --workspace` be unchanged. `crates/x3-sdk/src/rpc.rs:444` does
`...unwrap()` on a WebSocket connect, so the test panics with
`Connection("WebSocket connection failed: IO error: Operation not permitted (os error 1)")` in any
environment without a network — a sandbox, an air-gapped release build, a CI runner with egress
closed. Proven pre-existing rather than caused by this round: stashing the `x3-sdk` change and
re-running the single test fails identically.
Why it matters beyond tidiness: `cargo test --workspace` is one of the commands the repository's own
rules require as proof before a change is called complete, and it cannot be green here. A reader who
runs it and sees a red line has no way to tell a real regression from this, which is the cost.
Acceptance criteria: the test either runs against a **loopback** server it starts itself (the usual
answer for a client-construction test: bind `127.0.0.1:0`, assert the client constructs, close), or
it is marked `#[ignore]` with a comment saying it needs a network and why that is acceptable. The
first is preferred — construction is testable without egress, and the assertion is about the client,
not about the internet.
Validation: `cargo test -p x3-sdk` passes with no network access, and the test still fails if the
client's construction is broken (prove it by mutating the constructor).

## TICKET-103 — `cargo test --workspace` aborts at an e2e suite that needs a pre-built node, and that suite ignores `CARGO_TARGET_DIR` — CLOSED
Type: CLOSED in `a7df85f4c` (2026-09-20) · Subsystem: tests/e2e
Closed, and the fix covered **two** suites rather than the one the ticket named:
```
before:  aborts at cross_vm_real_chain_test — 6 panics, "x3-chain-node binary not found"
after:   3168 passed, 2 failed, and the 2 name a prerequisite rather than the code
```
**The target directory is honoured.** `node_binary()` looked only at
`<workspace_root>/target/{release,debug}/x3-chain-node`, so a build into any `CARGO_TARGET_DIR`
was invisible. The search is `node_candidates(target_dir, workspace_root)` + `choose(...)` now —
`CARGO_TARGET_DIR` first (release before debug), then the workspace's default dir — and the
*search order* became testable without a node: `node_binary_prefers_release_build` builds a temp
directory holding both binaries and asserts the release one is chosen, which it could not do
before because it asserted a suffix of a path it had to find first. Measured: with a node in
`CARGO_TARGET_DIR` the suite finds it.
**Both live suites are asked for rather than assumed.** `cross_vm_real_chain_test` and
`live_internal_mainnet_e2e` each need a built node, and the second's `required-features = []`
requires nothing — so `cargo test --workspace` ran them and they panicked. Both are behind a
`real-chain` feature now, so the workspace command neither builds them nor *silently skips* them.
Run deliberately with `cargo test -p e2e_tests --features real-chain --test <name>`; with no node
each live test prints a named reason and passes, and `X3_NODE_BIN` set to a path that does not
exist is a **failure** because setting it is the ask. The three in-process suites
(`gateway_integration_test` 10, `internal_mainnet_happy_path` 7, `mainnet_rc1` 16 — **33 tests**)
run without a node and are deliberately left ungated.
**What is left red, and why it is not this ticket.** The workspace run now reaches 3168 passed
with two failures in `-p x3-chain-node --lib`, and they name their prerequisite:
`"Embedded runtime WASM is missing for 'dev'. … Ensure SKIP_WASM_BUILD is unset and rebuild"`.
That is the acceptance's second branch — "fails with one clearly-labelled prerequisite rather than
six panics" — and it is not verifiable further here: building the WASM needs a git fetch of
`paritytech/polkadot-sdk`, which this environment cannot do. `scripts/local-ci.sh`'s `test node`
gate builds exactly that, and its own comment says it deliberately does not set `SKIP_WASM_BUILD`.
Acceptance as amended, met: a clean `cargo test --workspace` no longer panics six times for a
missing binary; the live suites are excluded from it and runnable on purpose; `CARGO_TARGET_DIR` is
honoured; and the remaining failure names its fix.
Original entry:
Reason: found while closing TICKET-102. `cargo test --workspace` stops at
`-p e2e_tests --test cross_vm_real_chain_test`, where all six tests panic with
```
thread 'node_binary_prefers_release_build' panicked at tests/e2e/cross_vm_real_chain_test.rs:45:13:
x3-chain-node binary not found; set X3_NODE_BIN or build target/debug/x3-chain-node
```
Two separate problems behind one panic:
1. **It hardcodes the default target directory.** `node_binary()` looks only at
   `<workspace_root>/target/{release,debug}/x3-chain-node`, so a build into any `CARGO_TARGET_DIR`
   can never satisfy it — which is why `SKIP_WASM_BUILD=1 CARGO_TARGET_DIR=… cargo test --workspace`
   fails here regardless of what has been built. Honouring `CARGO_TARGET_DIR` (or `OUT_DIR`) is the
   fix; the escape hatch (`X3_NODE_BIN`) exists but has to be set by hand, which is not a property a
   test command should need.
2. **It needs a node binary and a running chain at all.** That is legitimate for an e2e suite and it
   is why `scripts/local-ci.sh` does not run it — but `cargo test --workspace` *does*, because
   `tests/e2e` is a workspace member (root `Cargo.toml:90`), and cargo stops at the first failing
   test binary. So the command the repository's own rules name as required proof cannot be green on
   a clean checkout, and a reader cannot tell a real regression from a missing prerequisite.
Acceptance criteria: the suite skips with a **named reason** when no node binary is found (rather
than panicking six times), honouring `CARGO_TARGET_DIR`; and either `cargo test --workspace` excludes
it (a `required-features` gate or an `#[ignore]` with the reason) or `make test`/`local-ci.sh` is
documented as the command of record for the workspace, with this suite's prerequisite stated. The
choice belongs with whoever owns the test plan; the requirement here is that the two commands stop
disagreeing about whether the workspace is green.
Validation: from a clean checkout with no node built, `cargo test --workspace` either passes or fails
with one clearly-labelled prerequisite rather than six panics; with `X3_NODE_BIN` set to a built
node, the suite runs; and `SKIP_WASM_BUILD=1 CARGO_TARGET_DIR=<dir> cargo test --workspace` finds the
binary in `<dir>`.

## TICKET-104 — a coded diagnostic loses its severity and its secondary spans on the way to the accumulator — CLOSED
Type: CLOSED in `97d7af4f7` (2026-09-20) · Subsystem: x3-lang/compiler/diagnostic + semantic
Closed: a coded diagnostic can be a warning, the conversion that carries severity exists, and both
renderers refuse the severity they are not for. `CompilerDiagnostic::warning(code, message, span)`;
`From<CompilerDiagnostic> for x3_lang_common::Diagnostic` keeps the level, the code, the message,
**every** span (secondary spans as secondary labels) and the help text; `into_error` asserts its
severity and the new `into_warning` asserts the opposite, so filing an error as a warning — a
rejected program reading as a clean one — is as loud as the reverse. Both directions have a
`#[should_panic]` test.
**The ticket's own characterisation was wrong**: it asks for the conversion on the grounds that
"the accumulator already accepts that type", and the accumulator does not —
`VerifyOutcome.errors` and `.warnings` are both `Vec<X3Error>` (measured). The accumulator's half is
`VerifyOutcome::push_diagnostic(CompilerDiagnostic)`, the one place the vector is chosen, chosen from
the diagnostic's own field, because the two vectors *are* the severity channel and a caller picking
one by hand decides severity twice. The method's doc comment records both facts.
**Not claimed**: no production site emits a warning yet, so `push_diagnostic` and the conversion are
exercised by tests rather than by a compiler pass — a warning needs a real finding that is not a
rejection, and the first candidate is TICKET-110's `GasAdaptive` placeholder bodies, which are
invisible today. The trap is closed; its first user was not invented to have one.
Validation as measured: x3-lang `cargo test --workspace` **1188 passed / 0 failed** (1181 before);
clippy `-D warnings` clean; fmt clean; sweep 19/19/19/18. `compiler/tests/test_diagnostic_severity.rs`
(7 tests).
Original entry:
Reason: found while working TICKET-021. `CompilerDiagnostic` carries `code`, `severity`,
`message`, `primary_span`, `secondary_spans` and `help`; `into_error()` keeps **two** of the six:
```
pub fn into_error(self) -> x3_lang_common::X3Error {
    x3_lang_common::X3Error::SemanticError {
        message: format!("{}: {}", self.code.as_str(), self.message),
        span: self.primary_span,
    }
}
```
`X3Error` has no severity and no secondary spans, so every coded diagnostic built on the trading
path is rendered into a type that cannot carry them, and the accumulator's two vectors are what
tells a warning from an error. That is fine **for the severities the path emits today** — all of
them come from `CompilerDiagnostic::error`, so the rendering is faithful — and it is why the
conversion is not wrong: it is lossy in a way nothing currently notices.
It is a trap rather than a defect because **there is no `CompilerDiagnostic::warning`**: a
warning-severity coded diagnostic cannot be built, so the lossy path has no caller that would
suffer from it (grep confirms it — the constructor does not exist and nothing sets
`DiagnosticSeverity::Warning`). The moment someone adds one, `into_error` renders it as an error
message and the caller chooses the vector by hand, which is exactly the drift severity-as-a-field
exists to prevent.
Acceptance criteria: a coded diagnostic can be a warning, and the conversion that feeds the
accumulator preserves the severity rather than the caller re-deciding it — either by adding the
constructor and a `From<CompilerDiagnostic> for Diagnostic` that maps severity to the x3-common
level (the accumulator already accepts that type), or by making `into_error` refuse a diagnostic
whose severity it cannot carry, so the loss is a compile-time fact instead of a silent one.
Validation: a warning-severity coded diagnostic reaches a consumer as a warning; an error-severity
one as an error; and the secondary spans arrive with it, or the conversion refuses rather than
dropping them.

## TICKET-105 — the version policy is a global maximum, and a per-artifact one may be the right shape — CLOSED
Type: CLOSED in `bb5737775` (2026-09-20) · Subsystem: x3-lang/spec/opcodes.rs, compiler emitter, VM
Closed with TICKET-106, which is the first version-2 opcode this ticket was waiting for. **The
per-artifact reading is the decision**: `CURRENT_BYTECODE_VERSION` is the *ceiling* the writer may
write (still `== max_opcode_version()`, still a compile-time gate), and `emit_x3ir` narrows the byte
it wrote to the greatest version among the opcodes the artifact actually contains — read back out of
the stream with the reader's own framing (`is_payload_opcode` + `fixed_frame_content_len` + `align4`),
so an unregistered opcode is a refusal at that walk rather than a version byte that promises less
than the stream holds. Measured: `cmp_b.x3` (no version-2 opcode) writes `0x01`; a program with
`IF_MEASURED` writes `0x02`; `SUPPORTED_BYTECODE_VERSIONS` is `[1, 2]` so both are read.
**A hole this exposed is closed with it**: with `is_defined_version` as the only gate, a version-3
artifact from a *future* build would have been walked as raw bytecode — "nothing defines version 3"
was exactly the reasoning TICKET-097 exists to forbid. `is_reserved_version_byte` claims `0x01..=0x0F`,
so a reserved version this build does not know is a stream that is refused by name;
`test_bytecode_version_gate.rs`'s "unknown version" test uses a future version for that reason.
Validation as measured: the two new tests in `test_measured_branch.rs` (`the_artifacts_version_byte_is_the_greatest_version_it_contains`),
the updated gate test, and two trading tests that caught a bug this introduced (the trading decoder
demanded *exactly* the ceiling, so it refused every trading artifact this build had just compiled —
it accepts any supported version now). x3-lang `cargo test --workspace` 1197/0; clippy and fmt clean.
Original entry:
Reason: found while closing TICKET-097 in `d7b6cba9e`. The gate asserts
`CURRENT_BYTECODE_VERSION == max_opcode_version()`, so the version written is the greatest version
**anywhere in the registry**. That is the simplest reading of "the version byte is a function of the
opcode set" and it is exact while every opcode is version 1. It diverges from the per-artifact
reading — the greatest version among the opcodes *an artifact contains* — the moment a version-2
opcode exists: under the global reading every artifact a new compiler writes carries version 2,
including one that uses nothing new, and every version-1-only reader refuses all of them. Under the
per-artifact reading, a reader that knows version 1 keeps reading artifacts that use only version-1
opcodes, and only an artifact that really uses a new instruction is refused.
Acceptance criteria: the reading is chosen and written where the gate is, and the emitter either
tracks the greatest version of the opcodes it actually emitted (per-artifact) or says in the
constant's own doc that it does not, with the reason. The two documents that state the policy today
— `CURRENT_BYTECODE_VERSION`'s and `OPCODE_SET`'s — must agree, whichever way it goes; they are one
sentence about a global maximum and one about the opcodes an artifact contains, which are the same
fact only while nothing is version 2.
Validation: a version-2 opcode in a program, and the same compiler's artifact for a program without
one, each read by a version-1-only reader: the first refused by name and the second either read or
refused, per the decision, with the refusal naming the version it needs.
Depends on: TICKET-097 (closed), TICKET-106 (the first version-2 opcode, if it lands first).

## TICKET-106 — a runtime branch needs a value in a register, and the compiler emits no arithmetic (TICKET-058's remainder) — CLOSED
Type: CLOSED in `bb5737775` (2026-09-20) · Subsystem: x3-lang (IR, lowering, emitter, spec, VM) + CLI
Closed: the class is compiled and decided at run time. `Condition::Measured { quantity, comparison,
threshold_bps }` in the IR; `IF_MEASURED` (`0x34`, registered at `BYTECODE_VERSION_2`) as a payload
record `<unit>:<invert>:<threshold_bps>:<skip>`; two records per branch with the bodies between them
(there is no unconditional jump — `CALL`/`RET` push and pop a return address), the first skipping the
then body when the comparison does not hold and the second skipping the else body when it does. The
skip is sound because every instruction in a compiler stream starts on a four-byte boundary, so a
body's byte length is the same at any aligned offset and the record can be written *before* the body
rather than patched. `measured_comparison` is the single evaluator, shared with `REQUIRE`: a failure
is a refusal in a guard and a fork in a branch, while a quantity **nothing reported** refuses in both.
Measured through the binary:
```
$ x3c explain measured.x3b
  0005  0x34  IF_MEASURED profit >= 20bps, skip 2
  0006  0x88  MEMPOOL_SCAN             MempoolScan { max_results: 10 }
$ x3c run measured.x3b --measured-profit-bps 25   → ok, gas remaining 999296   (body ran)
$ x3c run measured.x3b --measured-profit-bps 5    → ok, gas remaining 999346   (body skipped)
$ x3c run measured.x3b                            → X3_GUARD_UNMEASURED: the branch `profit >= 20bps` …
```
The bound is read by the guards' own exact converter, so `if profit >= 0.005%` is refused as a half
basis point exactly as `require` refuses it; `profit == 20` is refused **by name** with the two
comparisons the quantity does support, because "not decidable at compile time" would have sent its
author looking for a way to decide it. `report_measurement`/`report_outcome` now seed the VM **state**
as well as the bridge — a figure the caller had already stated was only reachable after the first
call that answers with a measurement, which is what made the construct unreachable from a source
program until it was fixed.
What is *not* claimed: the branch's bodies are the statements a generic block accepts (`require`,
capability calls); a statement-position asset operation is the route and atomic-choice grammars' to
place, which is why the skip test builds its IR by hand. A measured branch chooses between
*statements*, not between *plans* — choosing between plans by measurement is a design question this
ticket did not open, and `atomic_choice` remains the bounded form the spec asks for (PHASE 12).
Tests: `compiler/tests/test_measured_branch.rs` (8) and `cli_branches_on_a_quantity_a_host_measured`.
Validation as measured: x3-lang `cargo test --workspace` **1197 passed / 0 failed** (1188 before);
clippy `-D warnings` clean; fmt clean; sweep 19/19/19/18.
Original entry:
Reason: the remainder of TICKET-058, and it is now scoped rather than open-ended. Three facts bound
it, all measured this session:
- A compiler stream starts every instruction at a multiple of four and the VM's `IF` skips whole
  four-byte units from `pc_next`, so **the branch mechanics are already sound**: a branch body
  emitted inline is padded to the stream's own boundaries, and the skip is `body_bytes / 4`. What
  TICKET-058 called "no target a reader could follow" is not the blocker.
- The VM's `IF` tests `registers[ra] == 0`, and `fixed_frame_content_len(IF) == 3` in a compiler
  stream means only the operand's low byte survives, so `ra` is always 0 there. The compiler emits
  **no arithmetic at all** and there is no immediate-load instruction: a constant can only reach a
  register through memory, which starts zeroed, or through an opcode's own result. So a general
  condition cannot be materialised, and `while steps < 10` names a value no instruction produces.
- The runtime quantities a `.x3` program can *name* are the measured ones (profit / slippage /
  delta, in bps), and the VM holds them in its own state, compared by `REQUIRE`'s measured modes.
  `record_measured_reply` does leave the last figure in `r0`, but relying on that is the register
  residue the codebase has already been bitten by twice.
So the smallest real step is not a register machine: it is a **measured branch** — an instruction
whose flags carry the same comparison mode and unit code `REQUIRE` uses and whose operand is the
threshold, branching instead of refusing, with the "no measurement" case refusing exactly as the
measured guard does. `if profit >= 20 { … } else { … }` then lowers to two such records with
complementary modes (`>=` and `<`), which needs no unconditional jump — there is no `JMP` opcode,
and `CALL`/`RET` cannot express one.
**Landed in `bb5737775`**: the remainder is `IF_MEASURED`, and its own entry is TICKET-106 (closed).
Original acceptance criteria, kept for the record: `if <measured guard> { A } else { B }` compiles to an artifact whose two bodies
are both written and whose branch is decided at run time by the figure a host reported; a branch
whose quantity nothing reported refuses (`X3_GUARD_UNMEASURED`'s shape) rather than picking a path;
`x3c explain` prints the branch with its mode, unit and threshold; and the IR's `Condition` for this
class says which quantity is compared, so a refusal can name it. The new opcode is registered at
version 2 and moves `CURRENT_BYTECODE_VERSION` with it — the gate TICKET-097 added is what makes
that a decision rather than a footnote, and TICKET-105 is the question to answer at that moment.
Validation: `x3c run` on such a program with a host reply inside the bound takes the then-body, one
outside it takes the else-body, and one with no reply refuses with the guard's own message; the
artifact's version byte is the one the registry says it must be; and a version-1-only reader refuses
the artifact by name rather than walking the new opcode.
Depends on: TICKET-097 (closed).

## TICKET-107 — the cost table says two opcodes are "no instruction in this catalogue" and one of them executes — CLOSED
Type: CLOSED in `339779ff3` (2026-09-20) · Subsystem: x3-lang/spec/opcodes.rs, vm/src/executor.rs
Closed: the two codes were two different things and the table's single comment about them was wrong
about one. **`0x0A` is an opcode** — the executor has had an arm for it all along (`POW_RRR`,
`ra = rb ^ rc` saturating) — so it is named `POW` on the same footing as `ADD`/`SUB` and for the reason
their own comment gives (nothing in `opcodes.yaml` declares it, and it is here so the catalogue, the
executor and the cost table can name the same code): constant, `OPCODE_SET`, `opcode_name`, and the
executor's two arms written `POW =>` rather than by value. **`0x70`'s row is gone** — no instruction in
this format has ever had that value, and the table's own comment admitted both rows were kept "only
because this is a move of the table rather than a redesign"; the move is done, so a price for it was a
price for nothing.
**Two more defects fell out of the gate the ticket asked for.** The new test walks three halves of the
agreement — every registered opcode has a name built from a *constant*, the executor matches no opcode
by a bare value, and every code the cost table prices (ranges included) is registered — and it found
that `NOP`, `ADD`, `SUB` and **`FEATURE_ALLOW`** were registered and nameless, so `x3c explain` printed
`0x56 UNKNOWN` for every `allow <feature>` statement (`examples/intent_fusion.x3` writes three):
```
BEFORE  0008  0x56  UNKNOWN
AFTER   0008  0x56  FEATURE_ALLOW
```
The name map's `ADD`/`SUB` arms being bare hex is *how* `0x0A` came to be registered, priced and
nameless at once, so they are constants now and the test refuses a bare-hex arm there.
Validation as measured: x3-lang `cargo test --workspace` **1206 passed / 0 failed** (1205 before);
clippy `-D warnings` clean; fmt clean; sweep 19/19/19/18.
**Recorded with it, because it nearly shipped:** a python rewrite of the test file **truncated it** —
every test after the insertion point was deleted. The workspace total dropped by three and the
per-binary comparison named the file; the tests are restored from `HEAD`. A scripted edit must know the
file's structure, and the test *count* is what catches the failure.
Original entry:
Reason: found while registering the opcode set in `d7b6cba9e`. `base_gas_cost` carries `0x0A => 50`
under the comment "0x0A: no instruction in this catalogue", and `0x70 => 2` under the same claim.
`0x70` is indeed in no arm of the executor — but **`0x0A` is**: `executor.rs`'s first arm is
`0x0A => { … POW_RRR … }`, which is why the new registry records `0x0A` as an opcode of version 1
and why the verifier must not refuse it. So the table tells its reader that a code it charges 50 gas
for is not an instruction, in the same file the VM reads. Two readings are possible — the comment is
stale, or the executor's arm is a leftover that should go — and the answer decides whether `0x0A`
stays in `OPCODE_SET`.
Acceptance criteria: `0x0A`'s status is decided (an opcode this format defines, with a named
constant and the comment fixed, or a code with no instruction behind it, with the executor's arm
removed and the registry entry deleted), and the gas table, the executor and `OPCODE_SET` say the
same thing about it.
Validation: `opcode_version(0x0A)` agrees with whether the executor has an arm for it, asserted by a
test that reads both.

## TICKET-108 — an assertion in `test_control_flow_e2e.rs` cannot fail — CLOSED
Type: CLOSED in `4f322939d` (2026-09-20) · Subsystem: x3-lang/compiler/tests
Closed with TICKET-109, because it turned out to be the same defect seen from the test side: the two
tests that asserted nothing were programs of `let` bindings, and what they were "proving" was that a
dropped program compiles. The file is rewritten so every assertion is the outcome measured on the
artifact the test itself builds — gas charged (`vm.state.gas < GAS`), an asset-operation count
(`1` for the route's swap, `2` for the intent's swap and refund), an opcode byte that must be
present, and an `Err` naming the binding — and its module doc no longer claims to test `if`/`loop`,
which it never did. The one test whose fixture cannot exist any more asserts the refusal that
replaced it, so the arithmetic subject stays covered one stage earlier and visibly.
Original entry:
Reason: found while looking for where the control-flow tests live. `e2e_compiled_bytecode_passes_through_vm_without_panic`
ends with `assert!(result.is_ok() || result.is_err(), "VM should not panic")` — a tautology, which
is why its own comment above it ("May succeed or fail at bridge level, but should not crash") is
doing the work the assertion looks like it is doing. The file also says of `if`/`loop`: "the actual
IF/LOOP opcodes are tested at the bytecode level in executor.rs", and the tests above it compile
`fn`-shaped sources that its own helper treats as intents — so this file's E2E claim is weaker than
its name.
Acceptance criteria: every test in the file asserts an outcome that can fail — the rule the module
doc claims — with the tautology either replaced by the property it meant (a `Result` that is an
`Err` for a reason the test names) or the test deleted with its coverage moved somewhere it can
fail. Deleting coverage is not the fix; making it able to fail is.
Validation: the new assertion fails when the property it names is broken (a mutation of the code
path it covers), and the file's tests still cover what the module doc says they cover.

## TICKET-109 — a statement the compiler cannot lower is dropped, not refused — CLOSED
Type: CLOSED in `4f322939d` (2026-09-20) · Subsystem: x3-lang/compiler (lowering) + tests corpora
Closed: `lower_statement` ended in `_ => ir.push(Operation::Nop)` under the comment "Other statement
types (return, break, etc.)". `NOP` is emitted as four zero bytes, which the compiler's instruction
walker skips as padding and the VM's verifier breaks on as the end of the stream — so every statement
that arm caught was dropped from the artifact, invisibly. The arm is gone; every statement the parser
can produce has its own arm and the match has no catch-all, so rustc enumerates the missing ones:
`let`, `return`, `break`, `continue` and `for` each refuse with the construct named and what is
missing. A bare `loop { … }` is not refused in lowering — it is the unbounded loop `while true`
spells, so it lowers with a decided-true condition and TICKET-098's one refusal names it.
Measured on `tests/test_arithmetic.x3`, whose entire subject is arithmetic:
```
BEFORE  x3c check: 3 ops, no semantic errors       (3 `Nop`s; 16-byte artifact, no instructions)
AFTER   x3c: semantic check failed: `let a = …` binds a name this compiler has no place to keep: it
        emits no arithmetic and no register holds a source-level binding, so the value would be
        dropped and every *use* of `a` already refuses. Write the value where it is used. …
```
The corpus gate was passing three files for this reason: `tests/test_arithmetic.x3` and
`tests/e2e/{simple_transfer,bridge_step}.x3` are written in an older dialect that the compiler
accepted and discarded, so the gate's claim — "a `.x3` in a directory the tooling walks is a claim
that it is a program" — was satisfied by the artifact of the dropping. All three moved to
`tests/sketches/` with the reason in its README; `tests/test_arithmetic.x3b`, a committed one-byte
file (`0x06`) that no reader accepts, is deleted. The gate's comment and `PLAN.md`'s ✅ claim are
corrected: there is no `x3-lang/compiler/src/regalloc.rs` (the linear-scan allocator is
`crates/x3-opt/src/regalloc.rs`, another crate), and the x3-lang compiler emits no arithmetic.
Tests: `compiler/tests/test_refused_statements.rs` (5) — each construct refused by name, a bare
`loop` reaching the IR as `Loop { condition: True }` and refused by `while true`'s own message, and
a supported statement still reaching the IR with **no `Nop` anywhere** (the pair that stops "refuse
everything" from passing the first four).
Validation as measured: x3-lang `cargo test --workspace` **1177 passed / 0 failed** (1172 before),
including the corpus gate; clippy `-D warnings` clean; fmt clean; sweep 19/19/19/18.

## TICKET-110 — a `GasAdaptive` annotation lowers to `Nop` bodies — CLOSED
Type: CLOSED in `e05b42cfe` (2026-09-20) · Subsystem: x3-lang/compiler (lowering + tests)
Closed: it lowers to nothing now. The annotation states two gas paths and has no way to name them — it
takes no arguments — while the artifact's record demands two **non-empty** bodies (`verify_ir`:
"gas-adaptive branches must not be empty"); the arm satisfied that rule with `vec![Operation::Nop]` on
each side, so the record said the program had two paths, neither of which was one, and both bodies
were four zero bytes no reader can see. No `GAS_ADAPTIVE` record is written at all, which is what the
ticket's acceptance allowed and the honest of the two options: the opcode is a real VM capability
(`bridge.gas_adaptive_select()` answers it), a hand-built IR can still carry it, and the verifier still
refuses empty bodies.
The assertion the ticket asked for is in `test_refused_statements.rs` as a source scan of
`lowering.rs`: no `Operation::Nop` and no `Operation::GasAdaptive {` construction survives there. The
scan is a scan because a `Nop` construction is legal Rust that compiles, and its failure is a program
that checks clean and runs as if a statement had not been written (TICKET-109).
Validation as measured: x3-lang `cargo test --workspace` **1198 passed / 0 failed** (1197 before);
clippy `-D warnings` clean; fmt clean; sweep 19/19/19/18.
Original entry:
Reason: found while removing TICKET-109's catch-all, by grepping for the other `Operation::Nop`
constructions in `lowering.rs`. `Annotation::GasAdaptive` lowers to
`Operation::GasAdaptive { high_gas_ops: vec![Operation::Nop], low_gas_ops: vec![Operation::Nop] }` —
two placeholder bodies inside an operation that reaches the artifact (`GAS_ADAPTIVE`, `0x99`), so the
instruction says "here is the low-gas path" and the path is one invisible instruction. It is not the
same defect as TICKET-109 (nothing is *dropped*; the record is written and the bodies are stated),
which is why it is recorded rather than folded into it.
Acceptance criteria: `@gas_adaptive` either carries the program's own two bodies (the annotation's
arguments name them) or does not emit `GAS_ADAPTIVE` at all — and if the artifact does carry the
instruction, a reader can tell which body a run took. `Operation::Nop` then has no construction left
in `lowering.rs`, and that is the assertion to add.
Validation: `x3c lower` on a program with `@gas_adaptive` shows two named bodies or no
`GasAdaptive`; a test asserts the absence of `Nop` in the lowered IR for that program.

## TICKET-111 — an annotation the artifact has no form for leaves no trace, eleven times over — CLOSED
Type: CLOSED in `5c41c6d7e` (2026-09-20) · Subsystem: x3-lang/compiler (annotations + diagnostics + parser)
Closed: the decision is one and it is applied in one table. `compiler/src/annotations.rs` holds
`DISPOSITIONS` — a row per spelling with what this build does with it — and the pass that raises the
finding. Six of the seven silent annotations are **policy** (`@no_heap`, `@no_recursion`, `@on_chain`,
`@off_chain`, `@concurrent`, `@payable`: properties of the code, and the artifact is not where a
property of the code is stated); `@gas_adaptive` is **reported**, because it claims the program has two
gas paths and the annotation names no bodies:
```
"X3E4026: `@gas_adaptive` has no effect on this artifact: it claims the program has two gas paths, and
 the annotation names no bodies, so the paths cannot be written: the artifact states no record for it
 rather than one whose bodies are placeholders (TICKET-110)"
```
That is the **first production use of TICKET-104's warning path**: a `CompilerDiagnostic::warning` filed
through `VerifyOutcome::push_diagnostic`, which chooses the vector from the diagnostic's own severity;
`DeclarationHasNoArtifactForm`/`X3E4026` is the catalogue's first warning-severity code.
**Two more silent drops fell out of the enumeration, both found by the mechanical test rather than by
reading**:
1. `@upgrade_from` **alone is dropped** — `lower_annotations_suffix` collects the value and pushes
   `VersionMeta` only when a version was stated too. Its row says `Reported`, and the pair
   (`@version` + `@upgrade_from`) is asserted to carry both.
2. `@subscription` **cannot be written at all** — the lexer reserves `subscription` for the
   `subscription <name>: <amount>, <period> { … }` item, so `expect_ident("annotation name")` refuses
   the token and the name map's `"subscription"` arm was unreachable (measured: 20 of 21 spellings
   parse, and that one does not). The dead arm is deleted, and `parse_single_annotation` now says what
   the word is for instead of "annotation name: expected identifier". The item form is the reachable
   one and is untouched.
Three facts are held against the table by `compiler/tests/test_annotation_policy.rs` (7 tests): every
`annotation_from_name_args` arm has a row (a source scan, because a new arm is legal Rust), every row's
spelling parses back to an annotation this module names, and the lowerer's IR is non-empty **exactly**
for the rows the table calls carried — the check that would have caught TICKET-110's placeholder.
**Effect on `--deny-warnings`**: a program carrying `@gas_adaptive`, or `@upgrade_from` with no
`@version`, now warns, so the strict gate fails on it. No example, fixture or test program uses either
(measured), which is why the 19-example sweep is unchanged at 19/19/19/18.
Validation as measured: x3-lang `cargo test --workspace` **1205 passed / 0 failed** (1198 before);
clippy `-D warnings` clean; fmt clean.
Original entry:
Reason: found while closing TICKET-110. That arm lowered `@gas_adaptive` to a record with two `Nop`
bodies, which is now nothing — and the fix exposed the wider shape: **eleven** annotations
(`NoHeap`, `NoRecursion`, `OnChain`, `OffChain`, `Concurrent`, `Scheduled`, `Version`,
`UpgradeFrom`, `Payable`, `Simd`, `GasAdaptive`) lower to `{}` in `lower_annotations_prefix`, and
nothing tells a program's author that the modifier they wrote had no effect on the artifact. For ten of
them that is defensible — a property of a function body the artifact need not carry — and for
`@gas_adaptive` it was not, because it claims two *paths*, which is why the record existed at all.
Acceptance criteria: one decision, applied once. Either a modifier with no artifact form is reported
(the new warning path TICKET-104 built exists for exactly this: `CompilerDiagnostic::warning` +
`VerifyOutcome::push_diagnostic`, and has no production user yet), or the silence is stated as the
policy in one place with the reason, so the next reader does not have to infer it from eleven empty
arms. The list is the enumeration, and a test should hold it: a new annotation that lowers to nothing
must appear in whichever answer is chosen.
Validation: `x3c check --deny-warnings` on a program carrying each of the eleven says what the
decision says it should, and the test that enumerates them fails when a twelfth is added silently.

## TICKET-112 — the trading decoder framed the stream by hand and landed inside a payload — CLOSED
Type: CLOSED in `2419bc2fe` (2026-09-20) · Subsystem: x3-lang/compiler/emitter (`decode_trading_program`)
Closed: found by the test written for `x3c replay`, which is the pairing worth noting — the command's
test drove `receipt execute` on a fixture the command does not use, and the *pipeline* failed instead:
```
$ x3c receipt execute examples/arb_scope.x3
x3c: decode error: Codegen error: truncated instruction payload for opcode 0x65
```
`0x65` **is not an opcode this format defines**. `decode_trading_program` walked the stream by hand —
`pos += 1` per byte, then every non-zero byte read as `[opcode][u16 len][payload]` — with no
`is_payload_opcode`, no `fixed_frame_content_len` and no `align4`. A fixed frame's flags byte and operand
were therefore read as a *length*, and the walk landed inside a payload. It now walks with
`instructions()`, the boundary source the disassembler and the VM already use, so the three cannot
disagree about where an instruction begins — the rule TICKET-097 wrote down, one walker later.
Measured after: `receipt execute examples/arb_scope.x3` refuses honestly
(`compiled trading program must begin with BeginAtomicTrade` — the fixture is an arb plan, not a trading
program) and a trading program's receipt still verifies. Validation: x3-lang `cargo test --workspace`
**1218 passed / 0 failed** (1216 before); clippy `-D warnings` and fmt clean; sweep 19/19/19/18.

## TICKET-113 — seven more arms match an identifier for a word the lexer reserves — OPEN
Type: OPEN, cleanup · Subsystem: x3-lang/compiler/parser
Reason: TICKET-046 closed three clauses whose arms could never run because they tested
`Tok::Ident(ref s) if s == "<word>"` for a word the lexer sends as a keyword token. The same shape
survives in seven places where a `Tok::Kw*` arm in the same match does the work, so nothing is
broken — but an unreachable arm is what let the three broken ones look correct for as long as they
did, and a reader of `parse_route_step` currently has two arms for `swap` with no way to tell which
one runs.
Measurement (the method, so the fix can be re-derived rather than trusted): the words are
`{w : w is a lexer keyword}` ∩ `{w : keyword_to_tok has a Tok::Kw* arm for it}`, and the sites are
every `Tok::Ident(ref s) if s == w` for such a `w`. As of `c7dfb461a`, ignoring the three that were
fixed:
- `parse_trade_stmt` — `bridge` (its `Tok::KwBridge` twin is the live arm)
- `parse_intent_clause` — `on_fail` (twin `Tok::KwOnFail`)
- `parse_route_step` — `swap`, `bridge`, `lock`/`mint`/`burn`/`release` (twins `Tok::KwSwap`,
  `Tok::KwBridge`, `Tok::KwLock..KwRelease`)
- `parse_rpc_quorum_item` — `require` (twin `Tok::KwRequire`)
- `parse_finality_policy_item` — the `s == "require"` half of `requirement || require`; the
  `requirement` half is live and is what the long form uses
Acceptance criteria: each site is either deleted (the keyword arm is the only path) or annotated
with the reason it can be reached. Deleting is the expected answer; the one thing that would make it
wrong is a second front-end that constructs `Tok::Ident("swap")` directly, which nothing does today.
Validation: `cargo test --workspace` in `x3-lang` stays green at 1233+, and the clause fixtures in
`test_require_guards.rs` plus the four tests in `test_keyword_clauses.rs` still pass.
Why not fixed in this pass: it is cleanup with no user-visible effect, and the three that *were*
broken are fixed and pinned. Recorded rather than dropped.
