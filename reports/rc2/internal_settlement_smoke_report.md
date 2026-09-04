# RC2 Internal Settlement Smoke Report

## Verdict

PASS

## Scope

- One canonical X3 asset
- X3Native / X3Evm / X3Svm
- Six internal settlement routes
- External bridges disabled

## Chain Info

- RPC: ws://127.0.0.1:9944
- Chain: X3 Chain Local 3-Validator Testnet
- Runtime spec version: 9
- Latest block: 273
- Finalized block: 269

## Initial Supply State

| Field | Amount |
|---|---:|
| canonical_supply | 120 |
| native_supply | 40 |
| evm_supply | 40 |
| svm_supply | 40 |
| external_locked_supply | 0 |
| pending_supply | 0 |
| represented_supply | 120 |

## Route Results

| Route | Transfer | Finalized | Pending zero | Supply invariant | Result |
|---|---:|---:|---:|---:|---:|
| X3Native -> X3Evm | 10 | PASS | PASS | PASS | PASS |
| X3Native -> X3Svm | 10 | PASS | PASS | PASS | PASS |
| X3Evm -> X3Native | 10 | PASS | PASS | PASS | PASS |
| X3Evm -> X3Svm | 10 | PASS | PASS | PASS | PASS |
| X3Svm -> X3Native | 10 | PASS | PASS | PASS | PASS |
| X3Svm -> X3Evm | 10 | PASS | PASS | PASS | PASS |

## Negative Tests

| Test | Expected | Result |
|---|---|---:|
| external Ethereum route rejected | reject | PASS |
| wrong EVM recipient rejected | reject | PASS |
| wrong SVM recipient rejected | reject | PASS |
| wrong sender type rejected | reject | PASS |
| duplicate message rejected | reject | PASS |
| duplicate nonce rejected | reject | PASS |
| refund after finalized rejected | reject | PASS |
| refund before expiry rejected | reject | PASS |
| completion after refund rejected | reject | PASS |

## Final Supply State

| Field | Amount |
|---|---:|
| canonical_supply | 120 |
| native_supply | 30 |
| evm_supply | 50 |
| svm_supply | 40 |
| external_locked_supply | 0 |
| pending_supply | 0 |
| represented_supply | 120 |

## Final Invariant

represented_supply == canonical_supply: PASS  
pending_supply == 0: PASS  
external bridges disabled: PASS

## Blockers

None
