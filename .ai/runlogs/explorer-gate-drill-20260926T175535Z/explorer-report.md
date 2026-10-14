# Public Testnet Gate Report

- **RPC**: `http://127.0.0.1:3411`
- **Chain spec**: `/home/lojak/Desktop/xxxstar-main/chain-specs/x3-testnet-raw.json`
- **Generated**: 2026-09-26T17:55:15Z
- **Overall**: FAIL

## Gate Results

| Gate | Criterion | Result |
|------|-----------|--------|
| 1  | Min 7 validators | FAIL |
| 2  | Public bootnodes | FAIL |
| 3  | No dev seeds | FAIL |
| 4  | External bridges disabled | PASS |
| 5  | Faucet separated from treasury | SKIP |
| 6  | Block production stable 0h | SKIP |
| 7  | Node restart drill | PASS |
| 8  | Validator removal drill | PASS |
| 9  | Runtime upgrade drill | PASS |
| 10 | Invariant halt drill | PASS |
| 11 | Refund drill | PASS |
| 12 | Indexer/RPC/API smoke | SKIP |
| 13 | Wallet/SDK transfers | PASS |
| 14 | Explorer/dashboard | PASS (reached `http://127.0.0.1:3410`, showing finalized head #4242, body identifies as the X3 Chain Explorer) |
| 15 | Production chain spec | SKIP |

## Gate Decision

**public_testnet_gate: FAIL** — resolve all FAIL items before opening public participation.

_Report hash: `5012c7a9fd67e86bb22b0fd46292a70df4a02bc6e6834a99d05b59b4c43bdd0c`_
