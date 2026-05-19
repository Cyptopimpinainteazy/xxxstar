--------------------------- MODULE CpuGpuParity ---------------------------
(*****************************************************************************)
(* TLA+ specification of the X3 CPU↔GPU validator parity invariant.          *)
(*                                                                           *)
(* Pairs with the executable harness at                                      *)
(*   X3-contracts/shared/gpu-parity-core/tests/parity_vectors.rs             *)
(* and the receipt at                                                        *)
(*   proof/receipts/claims/x3.gpu.cpu_gpu_parity.receipt.json.               *)
(*                                                                           *)
(* Claim (mainnet S1):                                                       *)
(*   For every pinned hash vector V in                                       *)
(*   `X3-contracts/shared/test-vectors/gpu_hash_parity.json`, the CPU        *)
(*   validator path and the GPU validator path produce the same canonical   *)
(*   32-byte digest, and that digest equals the pinned `expected_digest_hex`*)
(*   recorded in the spec vectors.                                          *)
(*                                                                           *)
(* Models both validator paths as deterministic functions over a shared     *)
(* input alphabet. Divergence — between the two paths or between either     *)
(* path and the spec — is captured as a state-machine invariant violation.  *)
(*****************************************************************************)
EXTENDS Naturals, FiniteSets, TLC

CONSTANTS
    Vectors,         \* finite set of vector ids
    SpecDigest,      \* function Vectors -> Nat (digest as a model-level integer)
    CpuDigest,       \* function Vectors -> Nat
    GpuDigest        \* function Vectors -> Nat

ASSUME
    /\ Vectors # {}
    /\ DOMAIN SpecDigest = Vectors
    /\ DOMAIN CpuDigest  = Vectors
    /\ DOMAIN GpuDigest  = Vectors

VARIABLES
    pending,         \* set of vector ids not yet executed
    cpu_diverged,    \* CPU path != spec
    gpu_diverged,    \* GPU path != spec
    pair_diverged    \* CPU != GPU on the same vector

vars == << pending, cpu_diverged, gpu_diverged, pair_diverged >>

----------------------------------------------------------------------------
(* Initial state                                                            *)
----------------------------------------------------------------------------
Init ==
    /\ pending       = Vectors
    /\ cpu_diverged  = {}
    /\ gpu_diverged  = {}
    /\ pair_diverged = {}

----------------------------------------------------------------------------
(* Operations                                                               *)
----------------------------------------------------------------------------
\* Execute a single vector through both paths and the spec, recording any
\* divergence. Each predicate is independent so the invariant fails as
\* finely as possible (you can tell *which* path drifted).
ExecuteVector(v) ==
    /\ v \in pending
    /\ pending'       = pending \ {v}
    /\ cpu_diverged'  = IF CpuDigest[v] = SpecDigest[v]
                        THEN cpu_diverged
                        ELSE cpu_diverged \cup {v}
    /\ gpu_diverged'  = IF GpuDigest[v] = SpecDigest[v]
                        THEN gpu_diverged
                        ELSE gpu_diverged \cup {v}
    /\ pair_diverged' = IF CpuDigest[v] = GpuDigest[v]
                        THEN pair_diverged
                        ELSE pair_diverged \cup {v}

Next ==
    \/ \E v \in pending : ExecuteVector(v)
    \/ /\ pending = {}
       /\ UNCHANGED vars

Spec == Init /\ [][Next]_vars

----------------------------------------------------------------------------
(* Invariants                                                               *)
----------------------------------------------------------------------------
\* I1: CPU path agrees with the pinned spec on every executed vector.
CpuMatchesSpec == cpu_diverged = {}

\* I2: GPU path agrees with the pinned spec on every executed vector.
GpuMatchesSpec == gpu_diverged = {}

\* I3: CPU and GPU paths agree with each other on every executed vector.
PathsAgree == pair_diverged = {}

\* I4: every vector eventually exits the pending set.
EventuallyExecuted == <> (pending = {})

\* I5: cleanly partitioned state.
TypeOK ==
    /\ pending       \subseteq Vectors
    /\ cpu_diverged  \subseteq Vectors
    /\ gpu_diverged  \subseteq Vectors
    /\ pair_diverged \subseteq Vectors

\* Composite safety property used by TLC.
Invariant ==
    /\ TypeOK
    /\ CpuMatchesSpec
    /\ GpuMatchesSpec
    /\ PathsAgree
================================================================================
