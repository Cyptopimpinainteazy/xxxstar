# Evidence — `reports/atomic_crossvm_completion_audit.md` addendum

Date: 2026-09-22
Scope: correctness of a readiness *record*, not code.

## Why

`reports/atomic_crossvm_completion_audit.md` was written when it was accurate, and has
since drifted in the *understating* direction: several of its "NOT FOUND"/"FAIL" rows are
no longer true, and one of them (the legacy adapter family) is still true and needs to
stay visible as a drift hazard. A reader who trusts the file as-is will either
over-invest in already-solved problems or under-estimate the real remaining ones.

## What was done

Appended a dated addendum that re-measures each headline finding against the current tree
and states, per row, whether the finding is stale, partly superseded, or still true.

| audit row | verdict | basis |
| --- | --- | --- |
| `crates/x3-crosschain-gateway` "excluded from the build" | stale | workspace member; 58 tests in its pallet |
| EVM/SVM legs are in-memory simulations only | superseded for the live path | `crates/x3-atomic-swap/src/{evm_live,x3vm_live}.rs`, real anvil + solana-test-validator behind `scripts/cross-domain-*-gate.sh`, strict posture, receipts-trie inclusion vs real `receiptsRoot` |
| "no hardcoded stub outputs in production path: FAIL" | still true of the legacy adapter family | `evm_htlc.rs`, `bitcoin_htlc.rs`; unreferenced from the live path but still compiled in-crate |
| external-chain settlement unproven on real chain state | partly superseded | proven against locally hosted chains, not public ones |
| no production E2E across all four legs | still true | BTC has no trusted header source; relayer cannot submit a proof for an intent it is not party to |

## Commands run

```
make guard
```

Result: `[agent_guard] ok`, `[no_stub_guard] ok`, `[test_cheat_guard] ok`.

## What this does not claim

No code changed. No completion percentage moved as a result of this commit; the
addendum only stops the record from misrepresenting the tree.
