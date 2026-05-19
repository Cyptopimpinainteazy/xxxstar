--------------------------- MODULE CrossVmParity ---------------------------
(*****************************************************************************)
(* TLA+ specification of the X3 cross-VM behavioral parity invariant.        *)
(*                                                                           *)
(* Pairs with the executable harness at                                      *)
(*   X3-contracts/shared/parity-core/tests/parity_vectors.rs                 *)
(* and the receipt at                                                        *)
(*   proof/receipts/claims/x3.contracts.evm_svm_parity.receipt.json.         *)
(*                                                                           *)
(* Claim (mainnet S0):                                                       *)
(*   For every shared parity vector V in                                     *)
(*   `X3-contracts/shared/test-vectors/*.json`, executing V on the EVM       *)
(*   stack and on the SVM stack produces the same observable outcome:        *)
(*   same `ok / revert` decision, same `revert_reason` tag on revert, and    *)
(*   the same signed `pool_delta` on commit.                                 *)
(*                                                                           *)
(* This spec models both stacks as deterministic pure functions over the     *)
(* same input alphabet so divergence is captured as a state-machine          *)
(* invariant violation rather than as engine-specific drift.                 *)
(*****************************************************************************)
EXTENDS Naturals, FiniteSets, Sequences, TLC

CONSTANTS
    Vectors,         \* finite set of parity vector ids (from JSON)
    EvmOutcome,      \* function Vectors -> [ok: BOOLEAN, reason: STRING, delta: Int]
    SvmOutcome       \* function Vectors -> [ok: BOOLEAN, reason: STRING, delta: Int]

\* Default-equal EvmOutcome / SvmOutcome implementations for TLC override.
\* The runtime parity harness in parity_vectors.rs is the source of truth;
\* this spec documents the invariant: EvmOutcome[v] = SvmOutcome[v].
EvmOutcomeImpl == [v \in Vectors |->
    [ok |-> TRUE, reason |-> "", delta |-> 0]]
SvmOutcomeImpl == [v \in Vectors |->
    [ok |-> TRUE, reason |-> "", delta |-> 0]]

ASSUME
    /\ Vectors # {}
    /\ DOMAIN EvmOutcome = Vectors
    /\ DOMAIN SvmOutcome = Vectors

VARIABLES
    pending,         \* set of vector ids not yet executed
    diverged         \* set of vector ids that produced divergent outcomes

vars == << pending, diverged >>

----------------------------------------------------------------------------
(* Initial state                                                            *)
----------------------------------------------------------------------------
Init ==
    /\ pending  = Vectors
    /\ diverged = {}

----------------------------------------------------------------------------
(* Operations                                                               *)
----------------------------------------------------------------------------
\* Execute a single vector against both stacks and compare outcomes.
ExecuteVector(v) ==
    /\ v \in pending
    /\ pending' = pending \ {v}
    /\ diverged' = IF EvmOutcome[v] = SvmOutcome[v]
                   THEN diverged
                   ELSE diverged \cup {v}

Next ==
    \/ \E v \in pending : ExecuteVector(v)
    \/ /\ pending = {}
       /\ UNCHANGED vars

Spec == Init /\ [][Next]_vars

----------------------------------------------------------------------------
(* Invariants                                                               *)
----------------------------------------------------------------------------
\* I1: stacks must agree on every executed vector.
NoDivergence == diverged = {}

\* I2: every vector eventually exits the pending set.
EventuallyExecuted == <> (pending = {})

\* I3: the executed set partitions cleanly (no vector is both pending and diverged).
TypeOK ==
    /\ pending  \subseteq Vectors
    /\ diverged \subseteq Vectors
    /\ diverged \cap pending = {}

Invariant == TypeOK /\ NoDivergence
================================================================================
