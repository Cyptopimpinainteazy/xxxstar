## 1. Runtime Canonical Asset
- [ ] 1.1 Define X3 canonical asset in runtime configuration (asset id, symbol, decimals, fixed supply).
- [ ] 1.2 Add genesis issuance + treasury allocation and lock rules.
- [ ] 1.3 Expose runtime API for canonical balance and total supply.
- [ ] 1.4 Implement genesis allocation splits and recipient account mapping.
 - [ ] 1.5 Implement bonus pool claim ledger and vesting hooks.

## 2. Proof & Bridge Layer
- [ ] 2.1 Define deterministic proof types for mirror mint/burn (x3-proof types + encoding).
- [ ] 2.2 Implement replay protection (nonce, domain separation, proof hash registry).
- [ ] 2.3 Add threshold signature verification hooks and aggregation limits.
 - [ ] 2.4 Implement BLS aggregation verification and signer-set pallet integration.

## 3. EVM Mirror Token
- [ ] 3.1 Implement EVM mirror token contract (mint/burn gated by proof verification).
- [ ] 3.2 Add pause/emergency controls and role-based access for upgrades.
- [ ] 3.3 Add proof verification tests (valid proof, replay, invalid signature).

## 4. SVM Mirror Program
- [ ] 4.1 Implement SVM mirror program (PDA escrow + proof-gated mint/burn).
- [ ] 4.2 Add timeout enforcement and hashlock unlock logic.
- [ ] 4.3 Add proof verification tests (valid proof, replay, invalid signature).

## 5. BTC HTLC Adapter
- [ ] 5.1 Define BTC HTLC script template and proof-gated unlock flow.
- [ ] 5.2 Add timeout/refund enforcement in the adapter.
- [ ] 5.3 Add BTC proof validation tests (valid proof, replay, invalid signature).

## 6. Relayer + Observability
- [ ] 6.1 Add relayer paths to submit proofs and capture EVM/SVM/BTC receipts.
- [ ] 6.2 Persist mirror events in X3 canonical ledger for reconciliation.
- [ ] 6.3 Add metrics for mirror mint/burn throughput + latency.

## 7. Tests & Invariants
- [ ] 7.1 Add deterministic serialization tests across platforms.
- [ ] 7.2 Add invariants: no mint without proof; replay protection enforced.
- [ ] 7.3 Add integration tests: X3 event → proof → EVM/SVM/BTC mint → X3 acknowledgement.
 - [ ] 7.4 Add stress-test harness (2,000,000,000 TPS target) with batch + parallelism settings.

## 8. Documentation + Scaffolds
- [x] 8.1 Add YOLO architecture diagram and detailed spec doc.
- [x] 8.2 Add runnable scaffold script for multi-chain atomic swap flow.
