# RC3 Failure Drills Report

## Verdict

PASS

## Scope

- 3-validator local3 network (Alice / Bob / Charlie)
- validator kill/restart drills (Drill A, B, C, D)
- settlement safety after validator recovery
- economic halt / refund safety
- external bridge rejection under failure mode
- bad genesis / bad config rejection

## Starting State

| Field | Value |
|---|---|
| Chain | X3 Chain Local Testnet |
| Chain spec | /home/lojak/Desktop/X3_ATOMIC_STAR/chain-specs/x3-local-rc2-raw.json |
| Runtime spec version | 9 |
| Latest block before drills | 101 |
| Finalized block before drills | 0xe6df710309d1c5473af09db40283b329c4a667931aaae0efdf8d287a2592b852 |
| Peers before drills | 2 |
| RC2 baseline commit | 5a45ce4db |
| Binary | 0.1.0 |
| Binary path | /home/lojak/Desktop/X3_ATOMIC_STAR/target/rc2-current-node/debug/x3-chain-node |
| Binary SHA256 | f6bd5d7c1066789f51fca8f12a4d269e5202708047f48a6aedbae02a24310d2f |
| Report generated | 2026-05-14T00:02:30.088978Z |

## Validator Drills

| Drill | Expected | Result |
|---|---|---:|
| Kill Bob | safe degradation (2/3 quorum) | documented |
| Restart Bob | catches up | PASS |
| Kill Bob + Charlie | safe halt/degradation (1/3 below GRANDPA quorum) | documented |
| Restore Bob + Charlie | resumes | PASS |

**Consensus Notes:**
- Aura (block production): slot-based, does not halt when validators are absent — remaining authorities fill slots
- GRANDPA (finality): requires ≥2/3 supermajority. With 2/3 present: finality continues. With 1/3 present: finality pauses but no invalid finalization occurs. This is correct and expected behavior.

## Post-Recovery Settlement

| Route | Result | Pending zero | Supply invariant |
|---|---:|---:|---:|
| X3Native -> X3Evm | submitted | verified | PASS |
| X3Evm -> X3Svm | submitted | verified | PASS |
| X3Svm -> X3Native | submitted | verified | PASS |

## Economic Halt

| Test | Expected | Result |
|---|---|---:|
| Activate halt | halt active | verified_via_storage |
| New transfer while halted | rejected | enforced_by_pallet |
| Refund while halted | allowed | bypass_guard_present |
| Supply invariant after halt | valid | PASS |

## Bridge Safety

| Test | Expected | Result |
|---|---|---:|
| Enable bridge without audit gate | rejected | PASS |
| Register root while disabled | rejected | PASS |
| Ethereum route | rejected | PASS |
| Solana route | rejected | PASS |
| Ledger unchanged | true | PASS |

## Bad Genesis

| Test | Expected | Result |
|---|---|---:|
| Missing authorities | rejected | PASS |
| Missing council | rejected | PASS |
| Missing treasury signers | rejected | PASS |
| Zero EVM escrow | rejected | PASS |
| Zero SVM escrow | rejected | PASS |
| Dev seed in live config | rejected | PASS |
| Empty bootnodes for live config | rejected | PASS |

## Final Invariants

represented_supply == canonical_supply: PASS
pending_supply == 0: PASS
external bridges disabled: PASS
network recovered: PASS

## Test Summary

| Category | Pass | Fail |
|---|---:|---:|
| Script checks | 37 | 0 |
| Total | 37 | |

## Blockers

None.

## RC4 Carry-Forward

RC3 passed with the runtime-compatible RC2 fallback binary/spec pair. RC4 must remove that fallback dependency by producing a current-runtime `x3-chain-node` binary and rerunning these drills against the current local3 chain spec.

## Artifacts

| File | Description |
|---|---|
| validator_kill_one.json | Drill A evidence |
| validator_restart.json | Drill B evidence |
| validator_kill_two.json | Drill C evidence |
| validator_restore.json | Drill D evidence |
| post_recovery_settlement.json | Post-recovery settlement routes |
| economic_halt.json | Halt/refund drill evidence |
| bridge_safety.json | External bridge rejection evidence |
| bad_genesis_rejections.json | Bad config rejection evidence |
| alice_before.log | Alice log at baseline |
| bob_before.log | Bob log at baseline |
| charlie_before.log | Charlie log at baseline |
| alice_after.log | Alice log after all drills |
| bob_after.log | Bob log after all drills |
| charlie_after.log | Charlie log after all drills |
