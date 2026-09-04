# RC4 Runtime Upgrade Rehearsal Report

## Verdict

PASS

## Scope

- local3 live runtime upgrade rehearsal
- old runtime -> new runtime
- internal settlement only
- external bridges disabled

## Refs

| Field | Value |
|---|---|
| OLD_REF | x3-atomic-star-rc2-internal-settlement-smoke |
| NEW_REF | HEAD |
| Old commit | 5a45ce4dbfc26cfd25ecb9e1c5ce88f63bac7106 |
| New commit | 419011ae8db43f9a71dfa12aef913e6baacfa0ea |
| Old WASM hash | f79cd3392ef7b2b6b45b266b86e6ad92e7c084041caf955dd1f06660ad412c35 |
| New WASM hash | ffb09a52c5c561ade7fbae7d238be8f9240e8f3b979099c95cc00c41b4e8c2c1 |

## Pre-Upgrade State

| Check | Result | Evidence |
|---|---:|---|
| Old node build | PASS | build skipped by BUILD_OLD=0 |
| Old runtime WASM build | PASS | build skipped by BUILD_OLD=0 |
| local3 boot | PASS | /ip4/127.0.0.1/tcp/30543/p2p/12D3KooWEyoppNCUx8Yx66oV9fJnriXwCcXwDDUA2kj6vnc6iDEp |
| Blocks advancing | PASS | blocks advanced |
| GRANDPA finality advancing | PASS | finalized head advanced |
| Pre-upgrade settlement smoke | PASS | test_x3_native_evm_svm_roundtrip_preserves_supply |
| Supply invariant | PASS | pre_upgrade_state.json |

## Upgrade Submission

| Field | Value |
|---|---|
| Extrinsic method | council-governance(system.setCode(compressed-runtime)) |
| Extrinsic hash | 0x43f399ff853da66231a86a56d30572481616213217e7a1a5b70852583aac3f7d |
| Included block | 129 |
| Old runtime version/hash | {'apis': [['0xdf6acb689907609b', 5], ['0xab3c0572291feb8b', 1], ['0xfbc577b9d747efd6', 1], ['0xd2bc9897eed08f15', 3], ['0xfbb6f67195cd4d67', 1], ['0xada7e9c48441a11a', 1], ['0xdd718d5cc53262d4', 1], ['0xed99c5acb25eedf5', 3], ['0x40fe3ad401f8959a', 6], ['0xbc9d89904f5b923f', 1], ['0x37c8bb1350a9a2a8', 4], ['0xf3ff14d5ab527059', 3], ['0x37e397fc7c91f5e4', 2], ['0xf7d800f3e08fc689', 1], ['0xfedda405e90ae979', 1], ['0x1465bdedeceda484', 1]], 'authoringVersion': 1, 'implName': 'x3-chain', 'implVersion': 1, 'specName': 'x3-chain', 'specVersion': 9, 'stateVersion': 1, 'systemVersion': 1, 'transactionVersion': 1} / 0xbd01e00178f65b6c5a908847c7f9f0d8ca1513cfa77bc1c4e00ebb68bec69759 |
| New runtime version/hash | {'apis': [['0xfedda405e90ae979', 1], ['0x1465bdedeceda484', 1], ['0x40fe3ad401f8959a', 6], ['0x37e397fc7c91f5e4', 2], ['0xfbb6f67195cd4d67', 1], ['0x37c8bb1350a9a2a8', 4], ['0xab3c0572291feb8b', 1], ['0xada7e9c48441a11a', 1], ['0xf7d800f3e08fc689', 1], ['0xed99c5acb25eedf5', 3], ['0xdf6acb689907609b', 5], ['0xbc9d89904f5b923f', 1], ['0xd2bc9897eed08f15', 3], ['0xf3ff14d5ab527059', 3], ['0xdd718d5cc53262d4', 1]], 'authoringVersion': 1, 'implName': 'x3-chain', 'implVersion': 1, 'specName': 'x3-chain', 'specVersion': 10, 'stateVersion': 1, 'systemVersion': 1, 'transactionVersion': 1} / 0x1b52ffd1da302553bd776fea13d7c7057c77ad3f3bbc109e4d5d8f9b66898b4f |
| Result | success |

## Post-Upgrade State

| Check | Result | Evidence |
|---|---:|---|
| New runtime active | PASS | runtime version or code hash changed |
| Blocks advancing | PASS | blocks advanced |
| GRANDPA finality advancing | PASS | finalized head advanced |
| Validators still connected | PASS | post_upgrade_state.json |
| No panic loop | PASS | post_upgrade_state.json |

## Post-Upgrade Settlement

| Route | Result | Pending zero | Supply invariant |
|---|---:|---:|---:|
| X3Native -> X3Evm | PASS | PASS | PASS |
| X3Evm -> X3Svm | PASS | PASS | PASS |
| X3Svm -> X3Native | PASS | PASS | PASS |

## Post-Upgrade Halt/Refund

| Test | Expected | Result |
|---|---|---:|
| Activate halt | halt active | UNKNOWN |
| New transfer while halted | rejected | PASS |
| Refund while halted | allowed | PASS |
| Supply invariant after halt/refund | valid | PASS |

## Final Invariants

represented_supply == canonical_supply: PASS  
pending_supply == 0: PASS  
external bridges disabled: PASS  
blocks advancing after upgrade: PASS  
finality advancing after upgrade: PASS  
external_locked_supply == 0: PASS  

## Blockers

None
