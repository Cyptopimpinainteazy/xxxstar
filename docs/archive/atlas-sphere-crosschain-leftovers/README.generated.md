# Atlas Sphere X3 Cross-Chain Token System

## Overview

Atlas Sphere X3 is a fully modular, AI-optimized, cross-chain token system supporting atomic swaps, staking, governance, and treasury automation across 103+ chains.

- **Native Token**: ERC20/ERC777 compatible, with configurable fees and treasury control.
- **Wrapped Tokens**: One per chain, atomic mint/burn, treasury cut on all operations.
- **Bridges**: Atomic, fee-collecting, with fallback for chain downtime.
- **Staking**: Pools on every chain, NFT positions, rewards from all protocol revenue.
- **Governance**: Cross-chain voting, DAO-controlled treasury, upgradeable logic.
- **Treasury**: All fees routed and split (dev, DAO, LP), with full accounting.
- **AI Hooks**: Yield optimization, fee tuning, governance suggestions, and arbitrage.

## Quickstart

1. `npm install`
2. Configure `hardhat.config.ts` with RPC endpoints and keys.
3. Run `npm run deploy` to deploy all contracts to all chains.
4. Run `npm test` to verify atomic swaps, staking, governance, and fee capture.

## Directory Structure

- `contracts/` — Core token and wrapped token contracts
- `adapters/` — Universal adapter for cross-chain ops
- `bridges/` — Atomic bridge logic
- `staking/` — Staking pools and NFT logic
- `governance/` — Cross-chain governance
- `treasury/` — Treasury and fee routing
- `ai-hooks/` — AI agent integration
- `scripts/` — Deployment and registry scripts
- `tests/` — Hardhat/TypeScript tests
- `docs/` — Fee schedule, governance, whitepaper, deployment checklist

## Placeholders
- Replace `<RPC_ENDPOINT_X>`, `<TREASURY_WALLET_X>`, `<PRIVATE_KEY>`, etc. with your actual values.

## License
MIT
