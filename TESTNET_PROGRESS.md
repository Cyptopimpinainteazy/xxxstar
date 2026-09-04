# TESTNET_PROGRESS.md

Continuous state machine for the autonomous audit. One line per milestone; newest on top.

## 2026-09-03 Session start
- Git baseline: initialised repo (tree was unversioned), committed full tree `091d3be`. Working tree clean.
- Created ledger/matrix/verification files under repo root (TESTNET_*.md) + scratch notes dir `.testnet-audit/`.
- Phase 1 inventory started: canonical node crate `node/` (bin `x3-chain-node`), 154 main.rs sources across tree (many are tools/test binaries), 132 crates + 59 pallets + Solidity(EVM/SVM)+ TS apps present.
- Chain spec loader ids confirmed in node/src/chain_spec.rs: dev, local, local-testnet, three-validator/3-validator, staging, production.
- Next: establish true baseline — does prebuilt `target/release/x3-chain-node` start a dev chain and produce blocks? Then 1x/2x/3x validator runs, tx submission, finality. Record commands/evidence in TESTNET_VERIFICATION.md.
