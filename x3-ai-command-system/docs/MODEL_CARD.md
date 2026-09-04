# CryptoMaster X3 — Model Card

## Model Information

- **Name:** CryptoMaster X3 AI Command System
- **Version:** 0.1.0
- **Base Model:** Qwen2.5-Coder (Apache-2.0)
- **Runtime:** Ollama
- **Type:** Modelfile-customized specialist model pack
- **Organization:** lojak

## Description

CryptoMaster is an X3-specialized omnichain engineering and trading model pack for Ollama, Cline, and local AI development.

It is designed for:
- X3 Chain architecture
- X3-Lang / X3VM
- EVM / Solidity
- SVM / Solana / Anchor
- Substrate / Rust runtime
- BTC UTXO / Taproot / PSBT
- CosmWasm / IBC
- Cross-VM atomic routing
- Canonical asset accounting
- Arbitrage architecture
- Flashloan route design
- Route scoring and finality risk
- MEV defense
- PnL / risk modeling
- Production DevOps
- Testing / fuzzing / invariant review

## Intended Use

- Production blockchain engineering
- Security auditing
- Cross-VM architecture design
- Trading system design (with safety controls)
- Code review and testing
- Documentation
- Compliance preparation

## Out-of-Scope Use

- Theft, fraud, or unauthorized exploitation
- Phishing or social engineering
- Rug-pull mechanics or deceptive tokens
- Malicious MEV targeting retail users
- DAO vote hijacking
- Unauthorized exploit execution
- Any activity that drains user funds without consent

## Model Specializations

| Model | Role | Temperature |
|---|---|---|
| cryptomaster | Omnichain architect | 0.15 |
| x3-auditor | Security reviewer | 0.05 |
| x3-rust-runtime | Substrate/Rust/X3VM | 0.12 |
| x3-solidity-guard | EVM/Solidity security | 0.08 |
| x3-svm-guard | Solana/SVM/Anchor | 0.08 |
| x3-cosmwasm-guard | Cosmos/CosmWasm/IBC | 0.08 |
| x3-btc-guard | BTC/UTXO/Taproot/PSBT | 0.08 |
| x3-arb-king | Arbitrage/trading | 0.12 |
| x3-flashloan-executor | Flashloan routes | 0.08 |
| x3-route-oracle | Route scoring/finality | 0.05 |
| x3-quant-risk | PnL/risk modeling | 0.05 |
| x3-trade-ops | Trading infrastructure | 0.10 |
| x3-mev-defense | MEV protection | 0.05 |
| x3-data-engineer | Indexers/ETL/feeds | 0.10 |
| x3-devops-commander | Infrastructure/ops | 0.10 |
| x3-testsmith | Testing/fuzzing/invariants | 0.10 |
| x3-docsmith | Documentation | 0.15 |
| x3-compliance-ops | Grants/compliance | 0.10 |
| x3-eval-judge | Model quality scoring | 0.05 |
| x3-cline-finisher | Repo completion | 0.10 |

## Safety

All models enforce:
- X3 Knowledge Core (architecture, invariants, rules)
- Trading Safety Kernel (dry-run, limits, kill switches, PnL logging)
- Forbidden Patterns (no theft, phishing, rug mechanics, malicious MEV)
- Mainnet Readiness rules (no fake readiness, no test cheating)

See `safety/SECURITY_BOUNDARIES.md` and `safety/TRADING_LIMITS.md` for details.

## Evaluation

See `evals/` for the eval harness. Every model must score 4+ average with zero dangerous outputs.

## Limitations

- Current version is prompt/Modelfile-customized, not fine-tuned
- Base model capabilities are inherited from Qwen2.5-Coder
- System prompts constrain behavior but do not guarantee it
- No model is a substitute for human security review
- Trading models are for design and analysis, not autonomous execution

## Version History

- **v0.1.0** — Initial release. 20 Modelfile-customized specialist models.

## Attribution

- Base model: Qwen2.5-Coder (Apache-2.0) by Alibaba
- X3 customizations by lojak
- Running on Ollama