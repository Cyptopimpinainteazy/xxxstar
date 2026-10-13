--------------------------- MODULE AtomicSettlement ---------------------------
(*****************************************************************************)
(* TLA+ specification of the X3 cross-VM atomic swap claim/refund machine.   *)
(*                                                                           *)
(* Pairs with the coordinator at                                             *)
(*   crates/cross-vm-coordinator/src/state_machine.rs                        *)
(*     - SwapPhase / HtlcStatus (types.rs)                                   *)
(*     - validate_phase_transition()                                         *)
(*     - begin_settlement() / record_fast_claim() / record_slow_claim()      *)
(*     - abort() / record_refunds()                                          *)
(*                                                                           *)
(* Claim (mainnet S0):                                                       *)
(*   A swap leg can never be both claimed and refunded; a swap only reaches  *)
(*   Complete with both legs claimed; a swap only reaches Refunded with no   *)
(*   claim ever made and every locked leg refunded; terminal phases          *)
(*   (Complete / Refunded / Failed) are absorbing.                           *)
(*                                                                           *)
(* The transition edges below mirror validate_phase_transition() edge for    *)
(* edge; the idempotent retry self-edges it allows (LockingHtlcs ->          *)
(* LockingHtlcs, ClaimingFast -> ClaimingFast, ClaimingSlow -> ClaimingSlow) *)
(* are stuttering steps, covered once by [][Next]_vars rather than by an     *)
(* action. The Rust side is pinned exhaustively by                             *)
(* crates/cross-vm-coordinator/src/state_machine.rs::                        *)
(* validate_phase_transition_matches_the_pinned_edge_table.                  *)
(* The money booleans mirror the htlc_fast / htlc_slow statuses:             *)
(* record_fast_claim sets the fast leg Claimed and lands in ClaimingSlow,    *)
(* record_slow_claim sets the slow leg Claimed and lands in Complete, and    *)
(* record_refunds sets every existing (locked) leg Refunded without          *)
(* consulting the claim state. That last point is the whole reason the       *)
(* phase table must refuse every path into Aborting after a claimant: the    *)
(* refund path itself has no per-leg claim guard to fall back on.            *)
(*                                                                           *)
(* LegacyAbortSkipsTable / LegacySlowClaimGuard model the two defects        *)
(* PR #558 fixed, both found by the deterministic simulator:                 *)
(*   - abort() used to set Aborting without consulting the table, so a       *)
(*     Complete swap could walk Complete -> Aborting -> Refunded and pay     *)
(*     both sides twice (the double spend atomic swaps exist to prevent);     *)
(*   - record_slow_claim() validated the transition into ClaimingSlow — the  *)
(*     phase the fast claim produces — while setting Complete, so a caller   *)
(*     could settle straight from ClaimingFast with the fast leg still       *)
(*     Funded: a false completion that loses the fast-side depositor funds. *)
(* The shipped .cfg sets both FALSE. AtomicSettlement.legacy.cfg sets both   *)
(* TRUE as the deliberately broken fixture; TLC must find a counterexample   *)
(* there — if it ever passes, the model has stopped distinguishing the two   *)
(* releases and the negative control is dead.                                *)
(*                                                                           *)
(* Out of scope: timelocks, flash-loan legs, secret-hash binding, replay     *)
(* stores, persistence and crash recovery. Executable tests own those; this  *)
(* module owns the phase/claim/refund lattice.                               *)
(*****************************************************************************)
EXTENDS Naturals, TLC

CONSTANTS
    LegacyAbortSkipsTable,   \* TRUE = pre-#558 abort(): no table check at all
    LegacySlowClaimGuard     \* TRUE = pre-#558 record_slow_claim() guard target

Phases ==
    { "Setup", "LockingHtlcs", "HtlcsLocked", "ExecutingFlashLegs",
      "LegsComplete", "ClaimingFast", "ClaimingSlow",
      "Complete", "Aborting", "Refunded", "Failed" }

TerminalPhases == {"Complete", "Refunded", "Failed"}

\* Sources of the (SwapPhase -> Aborting) edges in validate_phase_transition.
\* ClaimingSlow is deliberately absent: once the secret is on the fast chain
\* it is public, so the slow leg must be claimed, not refunded.
AbortablePhases ==
    { "Setup", "LockingHtlcs", "HtlcsLocked", "ExecutingFlashLegs",
      "LegsComplete", "ClaimingFast" }

VARIABLES
    phase,                                    \* SwapPhase
    fastLocked, fastClaimed, fastRefunded,    \* htlc_fast status lattice
    slowLocked, slowClaimed, slowRefunded     \* htlc_slow status lattice

vars == << phase, fastLocked, fastClaimed, fastRefunded,
           slowLocked, slowClaimed, slowRefunded >>

----------------------------------------------------------------------------
(* Initial state                                                            *)
----------------------------------------------------------------------------
Init ==
    /\ phase        = "Setup"
    /\ fastLocked   = FALSE
    /\ fastClaimed  = FALSE
    /\ fastRefunded = FALSE
    /\ slowLocked   = FALSE
    /\ slowClaimed  = FALSE
    /\ slowRefunded = FALSE

----------------------------------------------------------------------------
(* Actions — one per mutator on SwapCoordinator                             *)
----------------------------------------------------------------------------
BeginLocking ==
    /\ phase  = "Setup"
    /\ phase' = "LockingHtlcs"
    /\ UNCHANGED <<fastLocked, fastClaimed, fastRefunded,
                   slowLocked, slowClaimed, slowRefunded>>

\* record_htlc_fast + record_htlc_slow: the phase only advances to
\* HtlcsLocked once both HTLCs are funded.
LockHtlcs ==
    /\ phase  = "LockingHtlcs"
    /\ phase' = "HtlcsLocked"
    /\ fastLocked' = TRUE
    /\ slowLocked' = TRUE
    /\ UNCHANGED <<fastClaimed, fastRefunded, slowClaimed, slowRefunded>>

BeginFlashExecution ==
    /\ phase  = "HtlcsLocked"
    /\ phase' = "ExecutingFlashLegs"
    /\ UNCHANGED <<fastLocked, fastClaimed, fastRefunded,
                   slowLocked, slowClaimed, slowRefunded>>

\* record_leg_outcome(Success). Flash legs are abstract here — a failure
\* aborts the swap instead, which the Abort action covers. Only the phase
\* advance matters for the claim/refund lattice.
FlashLegsSucceeded ==
    /\ phase  = "ExecutingFlashLegs"
    /\ phase' = "LegsComplete"
    /\ UNCHANGED <<fastLocked, fastClaimed, fastRefunded,
                   slowLocked, slowClaimed, slowRefunded>>

\* begin_settlement(): validate(LegsComplete -> ClaimingFast).
BeginSettlement ==
    /\ phase  = "LegsComplete"
    /\ phase' = "ClaimingFast"
    /\ UNCHANGED <<fastLocked, fastClaimed, fastRefunded,
                   slowLocked, slowClaimed, slowRefunded>>

\* record_fast_claim(): the guard checks the edge INTO ClaimingFast
\* (LegsComplete -> ClaimingFast or ClaimingFast -> ClaimingFast) and the
\* mutation lands directly in ClaimingSlow with the fast leg Claimed.
\* Secret-hash matching and the cross-session replay store are abstracted.
RecordFastClaim ==
    /\ phase \in {"LegsComplete", "ClaimingFast"}
    /\ fastLocked
    /\ fastClaimed' = TRUE
    /\ phase' = "ClaimingSlow"
    /\ UNCHANGED <<fastLocked, fastRefunded,
                   slowLocked, slowClaimed, slowRefunded>>

\* record_slow_claim(): the fixed guard validates the edge INTO Complete
\* (only ClaimingSlow -> Complete exists). The pre-#558 guard validated the
\* edge into ClaimingSlow instead, and the table admits
\* ClaimingFast -> ClaimingSlow, so the legacy fixture can mark a swap
\* Complete from ClaimingFast with the fast leg never claimed.
RecordSlowClaim ==
    /\ IF LegacySlowClaimGuard
       THEN phase = "ClaimingFast"
       ELSE phase = "ClaimingSlow"
    /\ slowLocked
    /\ slowClaimed' = TRUE
    /\ phase' = "Complete"
    /\ UNCHANGED <<fastLocked, fastClaimed, fastRefunded,
                   slowLocked, slowRefunded>>

\* abort(): fixed = the table edge, with an Aborting retry being a no-op
\* (modelled by the stuttering step of [][Next]_vars). Pre-#558 = no check.
Abort ==
    /\ IF LegacyAbortSkipsTable THEN TRUE ELSE phase \in AbortablePhases
    /\ phase' = "Aborting"
    /\ UNCHANGED <<fastLocked, fastClaimed, fastRefunded,
                   slowLocked, slowClaimed, slowRefunded>>

\* record_refunds(): sets every existing (locked) HTLC's status to Refunded
\* without consulting the claim state — see the header. The phase table is
\* the only guard standing between this action and a double pay.
RecordRefunds ==
    /\ phase  = "Aborting"
    /\ phase' = "Refunded"
    \* Parenthesised: `a' = b \/ c` parses as `(a' = b) \/ c` in TLA+.
    /\ fastRefunded' = (fastRefunded \/ fastLocked)
    /\ slowRefunded' = (slowRefunded \/ slowLocked)
    /\ UNCHANGED <<fastLocked, fastClaimed, slowLocked, slowClaimed>>

Next ==
    \/ BeginLocking
    \/ LockHtlcs
    \/ BeginFlashExecution
    \/ FlashLegsSucceeded
    \/ BeginSettlement
    \/ RecordFastClaim
    \/ RecordSlowClaim
    \/ Abort
    \/ RecordRefunds
    \/ /\ phase \in TerminalPhases      \* terminal swaps stop evolving
       /\ UNCHANGED vars

Spec == Init /\ [][Next]_vars

----------------------------------------------------------------------------
(* Invariants                                                               *)
----------------------------------------------------------------------------
TypeOK ==
    /\ phase \in Phases
    /\ fastLocked   \in BOOLEAN
    /\ fastClaimed  \in BOOLEAN
    /\ fastRefunded \in BOOLEAN
    /\ slowLocked   \in BOOLEAN
    /\ slowClaimed  \in BOOLEAN
    /\ slowRefunded \in BOOLEAN

\* I1 — the property PR #558 existed to protect (P0 double spend).
NoLegBothClaimedAndRefunded ==
    /\ ~(fastClaimed /\ fastRefunded)
    /\ ~(slowClaimed /\ slowRefunded)

\* I2 — claims and refunds only ever touch a locked leg.
ClaimsRequireLocks ==
    /\ fastClaimed  => fastLocked
    /\ slowClaimed  => slowLocked
    /\ fastRefunded => fastLocked
    /\ slowRefunded => slowLocked

\* I3 — Complete means both legs were claimed (the fixed slow-claim guard).
CompleteImpliesBothClaimed ==
    phase = "Complete" => (fastClaimed /\ slowClaimed)

\* I4 — a refunded swap paid nobody: abort is refused once any leg is
\* claimed, so a Refunded state can never carry a claim.
RefundedImpliesNothingClaimed ==
    phase = "Refunded" => (~fastClaimed /\ ~slowClaimed)

\* I5 — no dangling locked funds: every locked leg was refunded on the way
\* to Refunded (record_refunds refunds all existing HTLCs).
RefundedAccountsForLockedLegs ==
    phase = "Refunded" =>
        /\ fastLocked => fastRefunded
        /\ slowLocked => slowRefunded

----------------------------------------------------------------------------
(* Temporal property                                                        *)
----------------------------------------------------------------------------
\* Complete / Refunded / Failed are absorbing: no edge of Next moves the
\* phase or the money out of them. This is exactly what abort() violated
\* before #558 when it walked a Complete swap back to Aborting.
\* (The action lives in its own operator: inside `[ ]_v` the parser would
\* otherwise read `[ phase \in ...` as a function constructor and demand
\* `|->`.)
TerminalStutter ==
    phase \in TerminalPhases =>
        ( phase' = phase
          /\ fastLocked'   = fastLocked
          /\ fastClaimed'  = fastClaimed
          /\ fastRefunded' = fastRefunded
          /\ slowLocked'   = slowLocked
          /\ slowClaimed'  = slowClaimed
          /\ slowRefunded' = slowRefunded )

TerminalPhasesAbsorbing ==
    [][TerminalStutter]_vars
================================================================================
