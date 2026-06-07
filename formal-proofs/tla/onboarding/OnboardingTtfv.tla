--------------------------- MODULE OnboardingTtfv ---------------------------
(*****************************************************************************)
(* TLA+ specification of the X3 developer onboarding / time-to-first-value   *)
(* invariant.                                                                *)
(*                                                                           *)
(* Pairs with the executable harness at                                      *)
(*   scripts/onboarding/measure_ttfv.sh                                      *)
(* the persisted benchmark at                                                *)
(*   proof/onboarding/ttfv_benchmark.json                                    *)
(* and the runner at                                                         *)
(*   proof-forge/src/runners/operational.rs::verify_onboarding.              *)
(*                                                                           *)
(* Claim (mainnet S1):                                                       *)
(*   A fresh developer can clone the repo, build proof-forge, and run two    *)
(*   real claim verifications producing signed receipts within Budget        *)
(*   seconds. Each step succeeds, and the cumulative wall-clock time stays   *)
(*   below the budget on every executed run.                                 *)
(*                                                                           *)
(* The state machine models the benchmark as an ordered sequence of steps    *)
(* with per-step durations and a pass/fail outcome. Safety invariants catch  *)
(* both "a step failed" and "we blew the budget" before VERIFIED is granted. *)
(*****************************************************************************)
EXTENDS Naturals, FiniteSets, Sequences, TLC

CONSTANTS
    Steps,              \* finite, ordered set of step labels (e.g. {build, verify1, verify2})
    StepOrder,          \* function Steps -> 1..|Steps| giving execution order
    StepDuration,       \* function Steps -> Nat (wall-clock seconds for that step)
    StepOutcome,        \* function Steps -> {"ok", "failed"}
    Budget              \* Nat: total seconds budget (e.g. 600)

ASSUME
    /\ Steps # {}
    /\ DOMAIN StepOrder    = Steps
    /\ DOMAIN StepDuration = Steps
    /\ DOMAIN StepOutcome  = Steps
    /\ Budget \in Nat \ {0}
    \* StepOrder must be a bijection onto 1..|Steps|.
    /\ \A s \in Steps : StepOrder[s] \in 1..Cardinality(Steps)
    /\ \A i \in 1..Cardinality(Steps) :
        \E s \in Steps : StepOrder[s] = i

VARIABLES
    pending,        \* set of step labels not yet executed
    elapsed,        \* total wall-clock seconds consumed so far
    failed_step,    \* the first step (if any) that failed; "" otherwise
    over_budget     \* TRUE iff elapsed has exceeded Budget at some point

vars == << pending, elapsed, failed_step, over_budget >>

----------------------------------------------------------------------------
(* Initial state                                                            *)
----------------------------------------------------------------------------
Init ==
    /\ pending     = Steps
    /\ elapsed     = 0
    /\ failed_step = ""
    /\ over_budget = FALSE

----------------------------------------------------------------------------
(* Operations                                                               *)
----------------------------------------------------------------------------
\* The next step to execute is the pending step with the smallest order.
NextStep ==
    CHOOSE s \in pending :
        \A t \in pending : StepOrder[s] <= StepOrder[t]

\* Execute the next step, accumulating duration and recording first failure.
ExecuteStep ==
    /\ pending # {}
    /\ failed_step = ""             \* don't continue past a failure
    /\ LET s == NextStep IN
        /\ pending'     = pending \ {s}
        /\ elapsed'     = elapsed + StepDuration[s]
        /\ failed_step' = IF StepOutcome[s] = "ok" THEN "" ELSE s
        /\ over_budget' = over_budget \/ (elapsed + StepDuration[s] > Budget)

Next ==
    \/ ExecuteStep
    \/ /\ \/ pending = {}
          \/ failed_step # ""
       /\ UNCHANGED vars

Spec == Init /\ [][Next]_vars

----------------------------------------------------------------------------
(* Invariants                                                               *)
----------------------------------------------------------------------------
\* I1: no step in the executed prefix has reported "failed".
NoStepFailed == failed_step = ""

\* I2: the cumulative elapsed time has never exceeded the budget.
WithinBudget == ~ over_budget

\* I3: every step eventually exits the pending set (assuming no failure).
EventuallyExecuted == <> (pending = {} \/ failed_step # "")

\* I4: cleanly partitioned state.
TypeOK ==
    /\ pending     \subseteq Steps
    /\ elapsed     \in Nat
    /\ failed_step \in {""} \cup Steps
    /\ over_budget \in BOOLEAN

\* Composite safety property required for VERIFIED.
Invariant ==
    /\ TypeOK
    /\ NoStepFailed
    /\ WithinBudget
================================================================================
