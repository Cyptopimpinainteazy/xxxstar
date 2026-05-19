# Change: Add X3 Canonical Coin Layer + Multi-Chain Mirror Projection

## Why
The X3 coin must be a consensus-native asset to guarantee deterministic settlement, slashing economics, and canonical authority. External-chain representations must be provable mirrors, not alternative sources of truth.

## What Changes
- Define **X3** as a runtime-native canonical asset in the Substrate runtime (source of truth).
- Add a proof-based mirror projection system for EVM, SVM, and BTC (mint/burn via threshold-signed proofs only).
- Introduce deterministic proof serialization and replay protection for mirror events.
- Wire relayer paths that submit proofs to EVM contracts, SVM programs, and BTC HTLCs, recording acknowledgements back on X3.
- Provide a single proof protocol across all mirror domains with explicit finality adapters per chain type.

## Canonical Parameters (Approved Inputs)
- Asset ID: **1000**
- Symbol: **X3**
- Decimals: **18**
- Total Supply: **8,888,888,888 X3**

## Genesis Allocations (Approved Inputs)
- Treasury: **25%** (2,222,222,222 X3)
- Validators / Staking: **20%** (1,777,777,778 X3)
- Ecosystem / Grants: **20%** (1,777,777,778 X3)
- Presale / Early Investors: **15%** (1,333,333,333 X3)
- Bonus Pool: **10%** (888,888,889 X3)
  - GPU / Hardware Contributors: **40%**
  - Validators / Staking Spot Buyers: **20%**
  - Presale / Traders: **10%**
  - Auditors / Bug Hunters: **30%**
- Team / Core Contributors: **10%** (888,888,888 X3)

## Proof Scheme (Approved Inputs)
- Threshold scheme: **BLS aggregation**
- Minimum signatures: **2/3 of active validator set**
- Signer set managed via **dedicated pallet**

## Target EVM Chains (Approved Inputs)
- Ethereum Mainnet: **1**
- BSC: **56**
- Polygon: **137**
- Base: **8453**
- Optional later: **SVM‑Test, BTC‑L1**

## Stress-Test Parameters (Approved Inputs)
- Target TPS: **2,000,000,000**
- Max batch per validator: **50,000**
- Atomic swap parallelism: **100,000**
- Proof emission frequency: **10ms**
- GPU nodes required: **128** (linear scaling with batch size)

## Impact
- Affected specs: new `x3-coin`, `x3-mirror-evm`, `x3-mirror-svm`, `x3-mirror-btc`, `x3-proof-bridge`, `x3-bonus-pool` capabilities.
- Affected code: runtime (`pallets/x3-kernel`, `pallets/x3-settlement-engine`, `runtime/src/lib.rs`), proof engine (`crates/x3-proof`), relayer (`scripts/relayer`), contracts (`contracts/`), dashboard/API surface.
- Security: introduces external mint/burn flow; requires deterministic proof encoding + replay protection.
- Performance: adds proof verification and signature aggregation paths; requires early benchmarks across EVM/SVM/BTC.
