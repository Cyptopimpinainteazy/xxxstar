# X3 Feature Summary

| Subsystem | Count | Implemented | Tested | Mainnet Ready | Composite | P0 | Mainnet <40 |
|---|---:|---:|---:|---:|---:|---:|---:|
| agents_experimental | 3 | 69.3 | 67.7 | 33.3 | 54.3 | 0 | 2 |
| claims_hygiene | 3 | 65.0 | 56.7 | 36.0 | 51.7 | 3 | 3 |
| consensus_l1 | 12 | 80.8 | 68.9 | 57.6 | 68.6 | 9 | 0 |
| cross_chain | 22 | 85.1 | 76.5 | 61.5 | 73.5 | 21 | 1 |
| cross_vm_atomic | 18 | 87.7 | 79.0 | 76.1 | 81.1 | 9 | 1 |
| economic_defi | 15 | 78.3 | 73.7 | 59.9 | 69.7 | 4 | 0 |
| gpu_performance | 17 | 67.6 | 64.4 | 41.5 | 56.2 | 4 | 7 |
| language_trading | 10 | 77.9 | 82.7 | 43.6 | 65.4 | 10 | 4 |
| mev_privacy | 8 | 70.0 | 63.8 | 38.8 | 56.0 | 6 | 3 |
| operations_user_tools | 11 | 78.0 | 72.9 | 49.8 | 65.5 | 3 | 3 |
| runtime_core | 12 | 82.0 | 71.9 | 58.2 | 70.0 | 9 | 1 |
| security_proofgate | 15 | 81.4 | 69.4 | 64.4 | 71.5 | 4 | 0 |
| swarm_compute | 3 | 74.0 | 76.0 | 12.0 | 49.7 | 0 | 3 |

## P0 features (82)

- `X3-CLAIM-001` Million-TPS GPU claims — mainnet 35%
- `X3-CLAIM-002` MEV-proof marketing claim — mainnet 35%
- `X3-CLAIM-003` Cross-chain complete claim — mainnet 38%
- `X3-L1-001` Multi-validator authority network — mainnet 73%
- `X3-L1-002` Validator key management — mainnet 50%
- `X3-L1-003` Runtime upgrade rehearsal — mainnet 55%
- `X3-L1-004` Runtime upgrade / migration framework — mainnet 55%
- `X3-L1-005` Chain-spec production configuration — mainnet 55%
- `X3-L1-006` Raw + plain chain-spec artifacts — mainnet 55%
- `X3-L1-007` GRANDPA finality — mainnet 68%
- `X3-L1-008` X3 chain node — mainnet 70%
- `X3-L1-009` Aura block authoring — mainnet 72%
- `X3-XCHAIN-001` BTC Fortress gateway — mainnet 50%
- `X3-XCHAIN-002` Bitcoin HTLC script logic — mainnet 40%
- `X3-XCHAIN-003` Relayer framework — mainnet 25%
- `X3-XCHAIN-004` Ethereum bridge adapter — mainnet 45%
- `X3-XCHAIN-005` Finality proof model — mainnet 45%
- `X3-XCHAIN-006` Atomic swap intent model — mainnet 58%
- `X3-XCHAIN-007` Atomic timeout engine — mainnet 58%
- `X3-XCHAIN-008` General external gateway — mainnet 70%
- `X3-XCHAIN-009` RPC quorum oracle — mainnet 65%
- `X3-XCHAIN-010` Cross-domain semantic idempotency — mainnet 68%
- `X3-XCHAIN-011` Gateway revoke / kill switch — mainnet 68%
- `X3-XCHAIN-012` RPC disagreement fail-closed behavior — mainnet 68%
- `X3-XCHAIN-013` Secret-release firewall — mainnet 72%
- `X3-XCHAIN-014` Coordinator restart recovery — mainnet 70%
- `X3-XCHAIN-015` Split-write crash recovery — mainnet 70%
- `X3-XCHAIN-016` Conflicting retry rejection — mainnet 72%
- `X3-XCHAIN-017` Cross-session secret ownership — mainnet 72%
- `X3-XCHAIN-018` Secret-release domain binding — mainnet 72%
- `X3-XCHAIN-019` Secret-release proof reuse rejection — mainnet 72%
- `X3-XCHAIN-020` Secret-release refund rejection — mainnet 72%
- `X3-XCHAIN-021` Secret-release tx/block binding — mainnet 72%
- `X3-XVM-001` Settlement engine — mainnet 60%
- `X3-XVM-002` Pending-supply accounting — mainnet 68%
- `X3-XVM-003` Canonical supply preservation across routes — mainnet 72%
- `X3-XVM-004` SHA-256 HTLC hash compatibility across VMs — mainnet 72%
- `X3-XVM-005` EVM → SVM route — mainnet 78%
- `X3-XVM-006` Expired transfer refund — mainnet 78%
- `X3-XVM-007` External bridge circuit breaker — mainnet 78%
- `X3-XVM-008` Failed destination credit refund — mainnet 78%
- `X3-XVM-009` SVM → EVM route — mainnet 78%
- `X3-ECO-001` Biometric wallet registration — mainnet 40%
- `X3-ECO-002` Pending supply returns-to-zero invariant — mainnet 55%
- `X3-ECO-003` Supply conservation ledger — mainnet 52%
- `X3-ECO-004` Wallet recovery flow — mainnet 45%
- `X3-GPU-001` GPU finalized-TPS proof — mainnet 5%
- `X3-GPU-002` Production runtime weights — mainnet 55%
- `X3-GPU-003` FRAME benchmarking — mainnet 55%
- `X3-GPU-004` GPU/CPU parity checking — mainnet 55%
- `X3-LANG-001` Compile→encode→decode→execute→receipt E2E — mainnet 30%
- `X3-LANG-002` Economic replay validation in receipts — mainnet 55%
- `X3-LANG-003` Signed / attested trading receipts — mainnet 35%
- `X3-LANG-004` Bytecode routing to atomic kernel — mainnet 35%
- `X3-LANG-005` Chain/capability version enforcement — mainnet 50%
- `X3-LANG-006` Decoded bytecode → TradingVm execution — mainnet 58%
- `X3-LANG-007` Stateful trading IR verifier — mainnet 45%
- `X3-LANG-008` .x3 compile_source integration — mainnet 40%
- `X3-LANG-009` Checksum-verified bytecode decoder — mainnet 38%
- `X3-LANG-010` Versioned runtime bytecode envelope — mainnet 50%
- `X3-MEV-001` Cross-domain MEV protection — mainnet 25%
- `X3-MEV-002` Private transaction submission controls — mainnet 55%
- `X3-MEV-003` Costs-before-profit evaluation — mainnet 45%
- `X3-MEV-004` Transactional host rollback — mainnet 45%
- `X3-MEV-005` Gas / economic execution limits — mainnet 40%
- `X3-MEV-006` MEV-resistant architecture — mainnet 50%
- `X3-OPS-001` Genesis ceremony tooling — mainnet 58%
- `X3-OPS-002` Snapshot / restore tooling — mainnet 58%
- `X3-OPS-003` Public testnet gate — mainnet 52%
- `X3-RT-001` Halt permits recovery/refund paths — mainnet 62%
- `X3-RT-002` Atomic Kernel pallet — mainnet 35%
- `X3-RT-003` mainnet-rc1 feature mode — mainnet 58%
- `X3-RT-004` Canonical state ledger updates — mainnet 60%
- `X3-RT-005` Atomic bundle submission — mainnet 65%
- `X3-RT-006` Economic Halt safety valve — mainnet 60%
- `X3-RT-007` Halt blocks new atomic bundles — mainnet 60%
- `X3-RT-008` X3 runtime — mainnet 68%
- `X3-RT-009` Forbidden experimental compile guards — mainnet 70%
- `X3-SEC-001` Reproducible srtool runtime build — mainnet 65%
- `X3-SEC-002` Runtime artifact hash evidence — mainnet 65%
- `X3-SEC-003` Mainnet release audit script — mainnet 65%
- `X3-SEC-004` Production gate — mainnet 62%
