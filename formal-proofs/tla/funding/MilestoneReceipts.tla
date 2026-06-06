--------------------------- MODULE MilestoneReceipts ---------------------------
(*****************************************************************************)
(* TLA+ specification of the X3 funding-ask ↔ milestone ↔ deliverable ↔     *)
(* proof-forge-receipt invariant.                                            *)
(*                                                                           *)
(* Pairs with the runtime gate at                                            *)
(*   proof-forge/src/runners/operational.rs::verify_funding                  *)
(* and the on-disk linkage map at                                            *)
(*   proof/funding/milestone-receipt-map.yml.                                *)
(*                                                                           *)
(* Claim (mainnet S1):                                                       *)
(*   Every funding ask maps to a uniquely-identified milestone with a        *)
(*   non-trivial deliverable, a positive budget, and a referenced            *)
(*   proof-forge receipt whose status is "verified". No funded milestone is  *)
(*   ever credited against a missing or non-verified receipt.                *)
(*                                                                           *)
(* The state machine models the registry as a set of milestones, each with  *)
(* a (deliverable, budget, receipt) and each receipt with a status. Safety  *)
(* invariants catch every failure mode the runner gates on.                  *)
(*****************************************************************************)
EXTENDS Naturals, FiniteSets, TLC

CONSTANTS
    Milestones,      \* finite set of milestone identifiers
    Receipts,        \* finite set of receipt identifiers
    Deliverable,     \* function Milestones -> STRING (non-empty)
    Budget,          \* function Milestones -> Nat (must be > 0 to count)
    ReceiptOf,       \* function Milestones -> Receipts
    ReceiptStatus    \* function Receipts -> {"verified", "partial", "unverified", "missing"}

ASSUME
    /\ Milestones # {}
    /\ DOMAIN Deliverable  = Milestones
    /\ DOMAIN Budget       = Milestones
    /\ DOMAIN ReceiptOf    = Milestones
    /\ DOMAIN ReceiptStatus = Receipts
    /\ \A m \in Milestones : ReceiptOf[m] \in Receipts

VARIABLES
    funded,        \* set of milestones the runner has admitted as funded-eligible
    rejected       \* set of milestones the runner rejected (gate fired)

vars == << funded, rejected >>

----------------------------------------------------------------------------
(* Eligibility                                                              *)
----------------------------------------------------------------------------
\* A milestone is eligible for funding-credit iff every gate passes.
Eligible(m) ==
    /\ Deliverable[m] # ""
    /\ Budget[m] > 0
    /\ ReceiptStatus[ReceiptOf[m]] = "verified"

----------------------------------------------------------------------------
(* Initial state                                                            *)
----------------------------------------------------------------------------
Init ==
    /\ funded   = {}
    /\ rejected = {}

----------------------------------------------------------------------------
(* Operations                                                               *)
----------------------------------------------------------------------------
\* Admit an eligible milestone into the funded set.
Admit(m) ==
    /\ m \in Milestones
    /\ m \notin funded
    /\ m \notin rejected
    /\ Eligible(m)
    /\ funded'   = funded \cup {m}
    /\ rejected' = rejected

\* Reject a milestone whose gate fails.
Reject(m) ==
    /\ m \in Milestones
    /\ m \notin funded
    /\ m \notin rejected
    /\ ~ Eligible(m)
    /\ rejected' = rejected \cup {m}
    /\ funded'   = funded

Next ==
    \/ \E m \in Milestones : Admit(m)
    \/ \E m \in Milestones : Reject(m)
    \/ /\ funded \cup rejected = Milestones
       /\ UNCHANGED vars

Spec == Init /\ [][Next]_vars

----------------------------------------------------------------------------
(* Invariants                                                               *)
----------------------------------------------------------------------------
\* I1: nothing in `funded` ever lacks a verified receipt.
NoFundingWithoutVerifiedReceipt ==
    \A m \in funded : ReceiptStatus[ReceiptOf[m]] = "verified"

\* I2: nothing in `funded` has zero budget or empty deliverable.
NoFundingWithoutDeliverableOrBudget ==
    \A m \in funded :
        /\ Budget[m] > 0
        /\ Deliverable[m] # ""

\* I3: funded and rejected are disjoint.
Disjoint == funded \cap rejected = {}

\* I4: type-correct partition.
TypeOK ==
    /\ funded   \subseteq Milestones
    /\ rejected \subseteq Milestones

\* Composite safety property required for the runner to emit VERIFIED.
Invariant ==
    /\ TypeOK
    /\ Disjoint
    /\ NoFundingWithoutVerifiedReceipt
    /\ NoFundingWithoutDeliverableOrBudget
================================================================================
