# X3 Chain Developer Templates

This document curates production-ready starting templates for X3 Chain development across runtime, frontend, backend automation, and mobile.

## Template Sources

### 1. Polkadot SDK Templates

- Purpose: chain/runtime/node development in Rust
- Use for:
  - Polkadot-compatible Layer 2 rollup
  - Independent Layer 1 blockchain
- Source: <https://paritytech.github.io/polkadot-sdk/master/polkadot_sdk_docs/polkadot_sdk/templates/index.html>

### 2. Polkadot API (PAPI) Examples

- Purpose: JavaScript/TypeScript integration with Polkadot chains
- Use for:
  - Front-end interfaces (React/Vue)
  - CLIs
  - Node services
  - Mobile JavaScript stacks
- Source: <https://github.com/polkadot-api/polkadot-api/tree/main/examples>

### 3. Python Substrate Interface Examples

- Purpose: Python automation/integration against Substrate/Polkadot chains
- Use for:
  - Operations scripts
  - Data indexing jobs
  - Back-office tooling
- Source: <https://github.com/polkascan/py-substrate-interface/tree/master/examples>

### 4. Substrate SDK iOS Example

- Purpose: Native iOS client integration in Swift
- Use for:
  - Wallet-grade mobile clients
  - Native signing and chain interactions
- Source: <https://github.com/novasamatech/substrate-sdk-ios/tree/master/Example>

## Recommended Adoption Order For X3 Chain

1. Polkadot SDK templates:
   - Establish canonical runtime/node baseline for X3 Chain.
2. PAPI examples:
   - Standardize TypeScript integration across apps and dashboards.
3. Python substrate interface examples:
   - Build ops/test automation and health tooling.
4. iOS template:
   - Add when mobile product scope is active.

## Repo Starter Matrix

Use these local starter folders as entry points:

- `templates/x3-chain/polkadot-sdk-l1/`
- `templates/x3-chain/polkadot-sdk-l2/`
- `templates/x3-chain/papi-app/`
- `templates/x3-chain/py-substrate-interface/`
- `templates/x3-chain/substrate-sdk-ios/`

Each folder contains an X3-specific starter checklist and a pointer to upstream templates.

## Notes

- These are starter references, not vendored full frameworks.
- Keep upstream template provenance in each starter README.
- Prefer pinning exact upstream commits/tags before production rollout.
