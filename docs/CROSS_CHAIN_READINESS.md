# Cross-Chain Readiness

**Reviewed:** 2026-09-08  
**Source lineage:** `master`  
**Scope:** internal cross-VM routing and external EVM, SVM, and Bitcoin bridging

## Bottom line

Internal cross-VM routing is the strongest part of the stack and is tracked at **88% readiness**. External cross-chain transfer is approximately **44% ready by the gate method below**. That number does not mean external transfers are safe for public funds. The production trust model, signer quorum, independent root validation, and live multi-operator staging evidence are still missing.

## Scoring method

This score uses eight equally weighted external-bridge gates. A complete gate earns 1 point, partial earns 0.5, and incomplete earns 0. The result is `3.5 / 8 = 43.75%`, rounded to **44%**.

| External bridge gate | Credit | Evidence | Remaining work |
|---|---:|---|---|
| Genesis bridge kill switch | 1.0 | `ExternalBridgesEnabled` defaults false and is checked on dispatch | Keep the guard mandatory in every production chain spec |
| EVM receipt proof math | 1.0 | Repository audit identifies real RLP/Keccak Merkle-Patricia verification | Bind proof roots to a production finality source |
| EVM contract test coverage | 1.0 | Repository audit records 169 passing Foundry tests at the audited commit | Re-run on every active-branch change and complete external review |
| SVM program verification | 0.5 | HTLC unit tests now exist; broader SVM programs have shallow or incomplete coverage | Add per-program Anchor tests and live validator evidence |
| Production relayer quorum | 0.0 | Current production trust path lacks independent multi-validator quorum | Implement threshold approval with independent keys and failure tests |
| Bridge-root validation | 0.0 | One root-registration path checks only that proof bytes are non-empty | Verify consensus/finality proofs before accepting a root |
| Bitcoin production path | 0.0 | Registry score is 25%; current mode is `SIM_TESTNET` | Add regtest evidence, SPV/finality validation, and production signer quorum |
| Public multi-operator round trip | 0.0 | No public staging deployment evidence | Run EVM→X3→EVM, SVM→X3→SVM, and BTC test cycles with independent operators |

## Internal cross-VM status

| Capability | Status | Notes |
|---|---|---|
| Native ↔ EVM ↔ SVM route matrix | Implemented for internal domains | Six directed routes are represented by named tests |
| Replay protection | Implemented | Duplicate message and nonce rejection tests are named in the registry |
| Timeout and refund | Implemented | Expired and failed-destination paths have named tests |
| Supply invariant | Implemented with evidence gaps elsewhere | Router tests exist; atomic-kernel invariant coverage was overstated and has been corrected |
| External bridge activation | Disabled | This is a safety control, not proof that the external bridge is complete |

## Critical blockers

1. **Production quorum:** the bridge must not depend on one environment-variable key or one operator.
2. **Root of trust:** receipt proof math is useful only when the accepted header/root comes from a verified finalized chain state.
3. **Root registration:** non-empty proof bytes are not proof verification.
4. **SVM coverage:** every deployed program needs its own negative-path and authority tests.
5. **Bitcoin:** regtest code and SPV components do not equal a production withdrawal system.
6. **Operational evidence:** recovery, validator loss, key rotation, halted-chain, replay, and reorg drills must run on the actual bridged staging profile.
7. **External audit:** contracts, runtime bridge paths, and relayer/key operations need independent review before public funds.

## Evidence paths

- `FEATURE_REGISTRY.toml`
- `LAUNCH_SCOPE.md`
- `docs/current/FAILURES_AND_TODOS.md`
- `pallets/x3-cross-vm-router/`
- `crates/x3-verification-router/`
- `crates/x3-relayer/`
- `crates/x3-bitcoin-vault/`
- `X3-contracts/evm/`
- `X3-contracts/svm/`
- `audit-artifacts/mainnet-readiness/`

## Grant-safe description

> X3 Atomic Star has an evidence-backed internal Native/EVM/SVM routing core and a guarded external-bridge codebase. External bridges are disabled while the project completes production quorum, finality-root validation, broader SVM and Bitcoin testing, public staging drills, and independent audits.
