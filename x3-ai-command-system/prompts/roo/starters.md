# X3 AI Command System — Roo Code Starter Prompts

## Setup

1. Copy `.roomodes` to your project root: `cp roo-config/.roomodes /path/to/your/project/`
2. In VS Code, open Roo Code settings:
   - **Provider**: OpenAI Compatible
   - **Base URL**: `http://localhost:11435/v1`
   - **API Key**: (leave empty or any string)
   - **Model**: `lojak/cryptomaster`
   - **Context Window**: `32768`

The X3 Router on port 11435 will auto-route your prompts to the right specialist model.

## Mode Quick Reference

| Mode | Slug | Best For |
|---|---|---|
| 🎮 CryptoMaster | `x3-cryptomaster` | Architecture, planning, general X3 questions |
| 🔒 Auditor | `x3-auditor` | Security audits, mainnet readiness |
| 🦀 Rust Runtime | `x3-rust-runtime` | Substrate, pallets, X3VM runtime |
| 🛡️ Solidity Guard | `x3-solidity-guard` | Solidity, EVM, Foundry, ERC standards |
| ⚡ SVM Guard | `x3-svm-guard` | Solana, Anchor, SPL tokens |
| 🌌 CosmWasm Guard | `x3-cosmwasm-guard` | CosmWasm, IBC, Cosmos appchains |
| ₿ BTC Guard | `x3-btc-guard` | Bitcoin, UTXO, Taproot, PSBT |
| 👑 Arb King | `x3-arb-king` | Arbitrage, DEX strategies, spread analysis |
| ⚡ Flashloan Executor | `x3-flashloan-executor` | Flashloan routes, callback validation |
| 🧭 Route Oracle | `x3-route-oracle` | Route scoring, finality risk, bridge safety |
| 📊 Quant Risk | `x3-quant-risk` | PnL, risk models, volatility, stop-loss |
| 🔧 Trade Ops | `x3-trade-ops` | Trading infra, daemons, dashboards |
| 🛡️ MEV Defense | `x3-mev-defense` | MEV protection, private relays |
| 🗄️ Data Engineer | `x3-data-engineer` | Indexers, ETL, price feeds |
| 🚀 DevOps Commander | `x3-devops-commander` | Docker, systemd, GPU, CI/CD |
| 🧪 Testsmith | `x3-testsmith` | Testing, fuzzing, invariants |
| 📝 Docsmith | `x3-docsmith` | Documentation, model cards |
| 📋 Compliance Ops | `x3-compliance-ops` | Grants, audit reports, compliance |
| ⚖️ Eval Judge | `x3-eval-judge` | Model scoring, benchmarking |
| 🏁 Finisher | `x3-cline-finisher` | TODOs, stubs, broken imports |

## Starter Prompts by Mode

### 🎮 CryptoMaster — Architecture & Planning
```
Design the X3 Chain cross-VM architecture. Show how EVM, SVM, and X3VM
intents interact through the Universal Asset Kernel. Include canonical
supply invariant enforcement and mainnet readiness checklist.
```

```
Plan a mainnet launch sequence for X3 Chain. Cover: validator onboarding,
bridge activation, DEX bootstrap, governance handover, and emergency
procedures. Apply the Mainnet Readiness checklist from knowledge-core.
```

### 🔒 Auditor — Security Review
```
Audit this contract for: reentrancy, integer overflow, access control,
front-running, and fund-loss vectors. For each finding, provide severity,
proof-of-concept, and remediation. Apply the X3 Security Boundaries.
```

```
Review the mainnet readiness of this deployment. Check: key management,
upgrade paths, circuit breakers, monitoring, incident response, and
the Universal Asset Kernel invariant across all VMs.
```

### 🦀 Rust Runtime — Substrate/Pallets
```
Implement a Substrate pallet for X3 Chain that handles cross-VM intent
processing. Include: weight calculation, origin checks, event emission,
and storage items for canonical receipts.
```

### 🛡️ Solidity Guard — EVM/Foundry
```
Write a Foundry test suite for this Solidity vault contract. Cover:
deposit, withdraw, reentrancy protection, ERC-4626 compliance, and
edge cases. Apply the EVM Rules from knowledge-core.
```

### ⚡ SVM Guard — Solana/Anchor
```
Write an Anchor program for a cross-chain token swap on Solana. Include:
CPI safety checks, PDA derivation, rent-exemption, and error handling.
Apply the SVM Rules from knowledge-core.
```

### 🌌 CosmWasm Guard — Cosmos/IBC
```
Implement a CosmWasm contract for IBC token transfer with custom routing.
Include: channel handshake verification, timeout handling, and ack
callbacks. Apply the CosmWasm/IBC Rules from knowledge-core.
```

### ₿ BTC Guard — Bitcoin/Bridge
```
Design a Taproot script for the X3 BTC bridge custody. Include:
key-path and script-path spends, PSBT construction, and merkle
proof verification. Apply the BTC/UTXO Rules from knowledge-core.
```

### 👑 Arb King — Arbitrage Strategy
```
Design an arbitrage strategy for cross-chain DEX price discrepancies.
Consider: gas costs, bridge fees, slippage, MEV exposure, and
execution ordering. Apply the Trading Safety Kernel. Calculate
expected PnL and risk score.
```

### ⚡ Flashloan Executor — Flashloan Routes
```
Design a flashloan route: borrow from Aave V3, execute cross-DEX swap,
repay loan, capture profit. Include: callback validation, repayment
guarantee proof, and gas estimation. Apply the Flashloan Safety rules.
```

### 🧭 Route Oracle — Route Scoring
```
Score the following cross-chain route: Uniswap (Ethereum) → Wormhole →
Raydium (Solana) → X3 DEX. Evaluate: finality risk, bridge risk,
liquidity depth, gas costs, and MEV exposure. Apply the Route Scoring
framework from knowledge-core.
```

### 📊 Quant Risk — PnL & Risk Modeling
```
Build a risk model for this trading strategy. Include: VaR calculation,
max drawdown, Sharpe ratio, slippage estimation, gas cost analysis,
and circuit breaker thresholds. Apply the Trading Safety Kernel.
```

### 🔧 Trade Ops — Trading Infrastructure
```
Set up the trading infrastructure for X3: daemon process management,
RPC node configuration, telemetry pipeline, dashboard, and kill switch.
Apply the Trading Safety Kernel requirements.
```

### 🛡️ MEV Defense — Anti-Extraction
```
Design MEV protection for this trading route. Include: private relay
integration, commit-reveal scheme, slippage tolerance, and sandwich
attack detection. Apply the MEV Defense strategies from knowledge-core.
```

### 🗄️ Data Engineer — Indexers & ETL
```
Build an event indexer for X3 Chain that tracks: canonical supply
changes, cross-VM transfers, and trading volume. Include: ETL pipeline,
price feed integration, and data quality checks.
```

### 🚀 DevOps Commander — Infrastructure
```
Configure the X3 validator node infrastructure: Docker Compose setup,
systemd services, Nginx reverse proxy, GPU monitoring, secrets
management, and CI/CD pipeline. Apply production hardening.
```

### 🧪 Testsmith — Testing & Fuzzing
```
Write a comprehensive test suite for this X3 pallet/contract. Include:
unit tests, integration tests, property-based tests, fuzz tests,
and invariant checks. Never weaken tests to force green.
```

### 📝 Docsmith — Documentation
```
Write complete API documentation for the X3 cross-VM intent system.
Include: endpoint descriptions, request/response schemas, error codes,
and usage examples. Follow the X3 documentation standards.
```

### 📋 Compliance Ops — Grants & Reporting
```
Draft a grant application for X3 Chain development. Include: technical
approach, milestones, team qualifications, budget breakdown, and
deliverable schedule. Apply compliance and disclosure requirements.
```

### ⚖️ Eval Judge — Model Evaluation
```
Score this model output against the X3 Knowledge Core. Evaluate:
correctness, safety compliance, mainnet readiness, forbidden pattern
adherence, and trading safety kernel compliance. Score 0-5.
```

### 🏁 Finisher — TODO Killer
```
Find and resolve all TODOs, FIXMEs, stubs, broken imports, and
incomplete code in this module. Leave no placeholder behind. Apply
the X3 production rules: real code only, no stubs, no hacks.
```

## Auto-Routing with X3 Router

You can also use the default mode (`x3-cryptomaster`) and let the router auto-select:

| You type | Router sends to |
|---|---|
| "Audit this Solidity contract" | `x3-solidity-guard` |
| "Fix the Rust pallet tests" | `x3-rust-runtime` |
| "Design an arbitrage strategy" | `x3-arb-king` |
| "Build a flashloan executor" | `x3-flashloan-executor` |
| "Score this cross-chain route" | `x3-route-oracle` |
| "Check MEV exposure" | `x3-mev-defense` |
| "Review mainnet readiness" | `x3-auditor` |
| "What is X3?" (generic) | `cryptomaster` |