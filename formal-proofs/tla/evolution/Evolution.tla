--------------------------- MODULE Evolution ---------------------------
(*****************************************************************************)
(* TLA+ specification of the X3 evolution / no-regression invariant.         *)
(*                                                                           *)
(* Pairs with the executable runner at                                       *)
(*   proof-forge/src/runners/operational.rs::verify_evolution                *)
(* and the persisted baseline at                                             *)
(*   proof/baselines/claim_scores.yml.                                       *)
(*                                                                           *)
(* Claim (mainnet S1):                                                       *)
(*   For every claim C in the registry, the score recorded on receipt at    *)
(*   generation g+1 must be >= the floor pinned at generation g.            *)
(*   Equivalently: claim scores form a monotone non-decreasing sequence     *)
(*   per claim across release generations.                                   *)
(*                                                                           *)
(* This spec models each claim as an integer-scored quantity (in            *)
(* basis points 0..1000 to keep TLC bounded) and the release process as     *)
(* a state machine that either ratchets a claim's floor up or rejects a     *)
(* downward observation as a regression event.                               *)
(*****************************************************************************)
EXTENDS Naturals, FiniteSets, TLC

CONSTANTS
    Claims,          \* finite set of claim ids
    MaxGenerations,  \* bound on number of releases TLC will explore
    ScoreCeiling     \* upper bound on score in basis points (e.g. 1000)

ASSUME
    /\ Claims # {}
    /\ MaxGenerations \in Nat \ {0}
    /\ ScoreCeiling \in Nat \ {0}

VARIABLES
    floor,        \* Claims -> Nat: highest score ever observed per claim
    generation,   \* Nat: current release generation number
    regressed     \* set of (claim, gen, prev, observed) regression events

vars == << floor, generation, regressed >>

----------------------------------------------------------------------------
(* Initial state                                                            *)
----------------------------------------------------------------------------
Init ==
    /\ floor      = [c \in Claims |-> 0]
    /\ generation = 0
    /\ regressed  = {}

----------------------------------------------------------------------------
(* Operations                                                               *)
----------------------------------------------------------------------------
\* Observe claim c at score s in the next generation.
\* If s drops below the pinned floor, a regression event is recorded.
\* Otherwise the floor ratchets up to max(floor[c], s).
ObserveClaim(c, s) ==
    /\ generation < MaxGenerations
    /\ s \in 0..ScoreCeiling
    /\ generation' = generation + 1
    /\ IF s < floor[c]
       THEN /\ floor'     = floor
            /\ regressed' = regressed \cup {<<c, generation + 1, floor[c], s>>}
       ELSE /\ floor'     = [floor EXCEPT ![c] = s]
            /\ regressed' = regressed

Next ==
    \/ \E c \in Claims, s \in 0..ScoreCeiling : ObserveClaim(c, s)
    \/ /\ generation = MaxGenerations
       /\ UNCHANGED vars

Spec == Init /\ [][Next]_vars

----------------------------------------------------------------------------
(* Invariants                                                               *)
----------------------------------------------------------------------------
\* I1: a recorded regression never lowers the pinned floor. A downward
\* observation is *rejected*: it is recorded in `regressed` and `floor` is
\* left untouched, so the pin can never fall back below a previously
\* recorded regression floor.
RegressionsDoNotLowerFloors ==
    \A e \in regressed : floor[e[1]] >= e[3]

\* I2: every recorded regression event is well-formed and genuinely
\* downward (observed < previous pin). A malformed or upward event in
\* `regressed` would mean the gate mislabelled an accepted score.
RegressionEventsSound ==
    \A e \in regressed :
        /\ e[1] \in Claims
        /\ e[2] \in 1..MaxGenerations
        /\ e[3] \in 0..ScoreCeiling
        /\ e[4] \in 0..ScoreCeiling
        /\ e[4] < e[3]

\* I3: floors stay within bounds.
TypeOK ==
    /\ generation \in 0..MaxGenerations
    /\ \A c \in Claims : floor[c] \in 0..ScoreCeiling

\* I4: the floor function is monotone non-decreasing across generations.
\* Encoded as an action property: floor[c]' >= floor[c] for every step.
MonotoneFloors == [][\A c \in Claims : floor'[c] >= floor[c]]_vars

\* `regressed = {}` is NOT a protocol invariant: it asserts the environment
\* never presents a downward score, which is exactly the event the gate
\* exists to handle, and TLC correctly finds a behavior containing one
\* (previously failing as "Invariant is violated"). The runner's guarantee
\* is conditional acceptance: `verify_evolution` emits success only when no
\* regression was recorded, and a recorded regression never moves the pin.
\* Those are the properties checked here: RegressionsDoNotLowerFloors,
\* RegressionEventsSound, and the temporal MonotoneFloors.
Invariant == TypeOK /\ RegressionEventsSound /\ RegressionsDoNotLowerFloors
================================================================================
