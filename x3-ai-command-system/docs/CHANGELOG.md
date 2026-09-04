# Changelog

## v0.1.0 — Initial Release

### Added

- 20 specialist Ollama Modelfiles:
  - lojak/cryptomaster — Omnichain architect
  - lojak/x3-auditor — Security reviewer
  - lojak/x3-rust-runtime — Substrate/Rust/X3VM
  - lojak/x3-solidity-guard — EVM/Solidity security
  - lojak/x3-svm-guard — Solana/SVM/Anchor
  - lojak/x3-cosmwasm-guard — Cosmos/CosmWasm/IBC
  - lojak/x3-btc-guard — BTC/UTXO/Taproot/PSBT
  - lojak/x3-arb-king — Arbitrage/trading architect
  - lojak/x3-flashloan-executor — Flashloan route builder
  - lojak/x3-route-oracle — Route scoring/finality risk
  - lojak/x3-quant-risk — PnL/risk modeling
  - lojak/x3-trade-ops — Live trading infrastructure
  - lojak/x3-mev-defense — MEV protection
  - lojak/x3-data-engineer — Indexers/ETL/feeds
  - lojak/x3-devops-commander — Infrastructure/ops
  - lojak/x3-testsmith — Testing/fuzzing/invariants
  - lojak/x3-docsmith — Documentation/model cards
  - lojak/x3-compliance-ops — Grants/compliance
  - lojak/x3-eval-judge — Model quality scoring
  - lojak/x3-cline-finisher — Repo completion

- Knowledge core documents (14 files)
- Model routing registry (model_registry.yaml)
- Eval harness (26 eval cases, scoring script)
- Fine-tuning pipeline (train_x3_lora.py, dataset template)
- Cline starter prompts (20 task-specific prompts)
- Trading safety kernel (TRADING_LIMITS.md)
- Security boundaries (SECURITY_BOUNDARIES.md)
- Build and push scripts
- Model card and README