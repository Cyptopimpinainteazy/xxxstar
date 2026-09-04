-------------------- MODULE AssetKernelSupply --------------------
EXTENDS Naturals, FiniteSets

CONSTANTS Accounts, Bridges, MaxAmount, InitSupply

VARIABLES balances, inflight

vars == << balances, inflight >>

ASSUME /\ Cardinality(Accounts) > 0
       /\ MaxAmount \in Nat
       /\ InitSupply \in Nat

RECURSIVE SumOver(_, _)
SumOver(f, keys) ==
    IF keys = {} THEN 0
    ELSE LET k == CHOOSE x \in keys : TRUE
         IN  f[k] + SumOver(f, keys \ {k})

SumBalances == SumOver(balances, Accounts)
SumInflight == SumOver(inflight, Bridges)
TotalSupply == SumBalances + SumInflight

TypeOK ==
    /\ balances \in [Accounts -> 0..MaxAmount]
    /\ inflight \in [Bridges  -> 0..MaxAmount]

Init ==
    /\ balances = [a \in Accounts |-> IF a = CHOOSE x \in Accounts : TRUE
                                       THEN InitSupply ELSE 0]
    /\ inflight = [b \in Bridges  |-> 0]

Transfer(from, to, amt) ==
    /\ from \in Accounts /\ to \in Accounts /\ from /= to
    /\ amt \in 1..MaxAmount
    /\ balances[from] >= amt
    /\ balances' = [balances EXCEPT ![from] = @ - amt, ![to] = @ + amt]
    /\ UNCHANGED inflight

BridgeOut(from, b, amt) ==
    /\ from \in Accounts /\ b \in Bridges
    /\ amt \in 1..MaxAmount
    /\ balances[from] >= amt
    /\ balances' = [balances EXCEPT ![from] = @ - amt]
    /\ inflight' = [inflight EXCEPT ![b] = @ + amt]

BridgeIn(to, b, amt) ==
    /\ to \in Accounts /\ b \in Bridges
    /\ amt \in 1..MaxAmount
    /\ inflight[b] >= amt
    /\ inflight' = [inflight EXCEPT ![b] = @ - amt]
    /\ balances' = [balances EXCEPT ![to] = @ + amt]

Next ==
    \/ \E a, c \in Accounts, n \in 1..MaxAmount : Transfer(a, c, n)
    \/ \E a \in Accounts, b \in Bridges, n \in 1..MaxAmount : BridgeOut(a, b, n)
    \/ \E a \in Accounts, b \in Bridges, n \in 1..MaxAmount : BridgeIn(a, b, n)

Spec == Init /\ [][Next]_vars

SupplyConservation == TotalSupply = InitSupply

==============================================================================
