# RC1 P0 Blocker: Runtime no_std Dependency Boundary Violation

## Executive Summary

The `x3-chain-runtime` crate fails to build with `serde_core v1.0.222` due to a no_std dependency boundary violation. The runtime incorrectly depends on host-only crates through the polkadot-sdk dependency chain.

## Exact Failing Command

```bash
cargo check --manifest-path runtime/Cargo.toml --no-default-features
```

## Exact Error Output

```
warning: patch for `sp-crypto-hashing` uses the features mechanism. default-features and features will not take effect because the patch dependency does not support this mechanism
```

The build fails because `serde_core v1.0.222` is resolved, which has a dependency path that forces host-only features into the runtime.

## serde_core Version Resolved

- **Version**: `serde_core v1.0.222`
- **Source**: polkadot-sdk (https://github.com/paritytech/polkadot-sdk?branch=stable2512#30b95889)

## Dependency Path Forcing serde_core 1.0.221+

```
serde_core v1.0.222
└── serde_bytes v0.11.19
    └── schnorrkel v0.11.5
        └── substrate-bip39 v0.6.0 (polkadot-sdk)
            └── sp-core v39.0.0 (polkadot-sdk)
                └── frame-benchmarking v45.0.3 (polkadot-sdk)
                    └── pallet-x3-automation v0.1.0
                        └── x3-chain-runtime v0.1.0
```

### Full Dependency Chain (from cargo tree)

```
serde_core v1.0.222
└── serde_bytes v0.11.19
    └── schnorrkel v0.11.5
        └── substrate-bip39 v0.6.0 (https://github.com/paritytech/polkadot-sdk?branch=stable2512#30b95889)
            └── sp-core v39.0.0 (https://github.com/paritytech/polkadot-sdk?branch=stable2512#30b95889)
                ├── frame-benchmarking v45.0.3
                │   ├── pallet-x3-automation v0.1.0
                │   │   └── x3-chain-runtime v0.1.0
                │   ├── pallet-x3-coin v0.1.0
                │   │   └── x3-chain-runtime v0.1.0
                │   ├── pallet-x3-dex v0.1.0
                │   │   └── x3-chain-runtime v0.1.0
                │   ├── pallet-x3-oracle v0.1.0
                │   │   └── x3-chain-runtime v0.1.0
                │   ├── pallet-x3-vrf v0.1.0
                │   │   └── x3-chain-runtime v0.1.0
                │   └── pallet-x3-wallet v0.1.0
                │       └── x3-chain-runtime v0.1.0
                ├── frame-election-provider-support v45.0.0
                │   └── pallet-staking v45.1.0
                │       └── pallet-x3-slash v0.1.0
                │           └── x3-chain-runtime v0.1.0
                ├── frame-executive v45.0.1
                │   └── x3-chain-runtime v0.1.0
                ├── frame-support v45.1.0
                │   ├── frame-benchmarking v45.0.3 (*)
                │   ├── frame-election-provider-support v45.0.0 (*)
                │   ├── frame-executive v45.0.1 (*)
                │   ├── frame-system v45.0.0
                │   │   ├── frame-benchmarking v45.0.3 (*)
                │   │   ├── pallet-agent-accounts v0.1.0
                │   │   │   └── x3-chain-runtime v0.1.0
                │   │   ├── pallet-agent-memory v0.1.0
                │   │   │   └── x3-chain-runtime v0.1.0
                │   │   ├── pallet-atomic-trade-engine v0.1.0
                │   │   │   └── pallet-x3-settlement-engine v0.1.0
                │   │   │       └── x3-chain-runtime v0.1.0
                │   │   ├── pallet-aura v44.0.0
                │   │   │   └── pallet-x3-consensus v0.1.0
                │   │   │       └── x3-chain-runtime v0.1.0
                │   │   ├── pallet-authorship v45.0.0
                │   │   │   ├── pallet-grandpa v45.0.0
                │   │   │   │   ├── pallet-x3-consensus v0.1.0 (*)
                │   │   │   │   └── x3-chain-runtime v0.1.0
                │   │   │   │   └── pallet-grandpa feature "std"
                │   │   │   │       └── x3-chain-runtime feature "std"
                │   │   │   └── pallet-staking v45.1.0 (*)
                │   │   ├── pallet-balances v46.0.0
                │   │   │   ├── pallet-agent-accounts v0.1.0 (*)
                │   │   │   ├── pallet-agent-memory v0.1.0 (*)
                │   │   │   ├── pallet-depin-marketplace v0.1.0
                │   │   │   │   └── x3-chain-runtime v0.1.0
                │   │   │   ├── pallet-governance v0.1.0
                │   │   │   │   └── x3-chain-runtime v0.1.0
                │   │   │   ├── pallet-private-execution v0.1.0
                │   │   │   │   └── x3-chain-runtime v0.1.0
                │   │   │   ├── pallet-session v45.2.0
                │   │   │   │   ├── pallet-grandpa v45.0.0 (*)
                │   │   │   │   └── pallet-x3-consensus v0.1.0 (*)
                │   │   │   ├── pallet-swarm v0.1.0
                │   │   │   │   └── x3-chain-runtime v0.1.0
                │   │   │   ├── pallet-treasury v0.1.0
                │   │   │   │   └── x3-chain-runtime v0.1.0
                │   │   │   ├── pallet-x3-automation v0.1.0 (*)
                │   │   │   ├── pallet-x3-kernel v0.1.0
                │   │   │   │   ├── pallet-atomic-trade-engine v0.1.0 (*)
                │   │   │   │   ├── pallet-x3-coin v0.1.0 (*)
                │   │   │   │   ├── pallet-x3-settlement-engine v0.1.0 (*)
                │   │   │   │   └── x3-chain-runtime v0.1.0
                │   │   │   ├── pallet-x3-vrf v0.1.0 (*)
                │   │   │   └── x3-chain-runtime v0.1.0
                │   │   ├── pallet-collective v45.0.0
                │   │   │   └── x3-chain-runtime v0.1.0
                │   │   ├── pallet-cross-chain-validator v0.1.0
                │   │   │   └── x3-chain-runtime v0.1.0
                │   │   ├── pallet-depin-marketplace v0.1.0 (*)
                │   │   ├── pallet-evolution-core v0.1.0
                │   │   │   └── x3-chain-runtime v0.1.0
                │   │   ├── pallet-governance v0.1.0 (*)
                │   │   ├── pallet-grandpa v45.0.0 (*)
                │   │   ├── pallet-meme-overlord v0.1.0
                │   │   │   └── x3-chain-runtime v0.1.0
                │   │   ├── pallet-offences v44.0.0
                │   │   │   ├── pallet-x3-consensus v0.1.0 (*)
                │   │   │   └── x3-chain-runtime v0.1.0
                │   │   ├── pallet-preimage v45.0.0
                │   │   │   └── x3-chain-runtime v0.1.0
                │   │   ├── pallet-private-execution v0.1.0 (*)
                │   │   ├── pallet-scheduler v46.0.0
                │   │   │   └── x3-chain-runtime v0.1.0
                │   │   ├── pallet-session v45.2.0 (*)
                │   │   ├── pallet-staking v45.1.0 (*)
                │   │   ├── pallet-sudo v45.0.0
                │   │   │   └── x3-chain-runtime v0.1.0
                │   │   ├── pallet-svm v0.1.0
                │   │   │   └── x3-chain-runtime v0.1.0
                │   │   ├── pallet-swarm v0.1.0 (*)
                │   │   ├── pallet-timestamp v44.0.0
                │   │   │   ├── pallet-atomic-trade-engine v0.1.0 (*)
                │   │   │   ├── pallet-aura v44.0.0 (*)
                │   │   │   ├── pallet-session v45.2.0 (*)
                │   │   │   ├── pallet-x3-jury-anchor v1.0.0
                │   │   │   │   └── x3-chain-runtime v0.1.0
                │   │   │   ├── pallet-x3-kernel v0.1.0 (*)
                │   │   │   ├── pallet-x3-oracle v0.1.0 (*)
                │   │   │   └── x3-chain-runtime v0.1.0
                │   │   ├── pallet-transaction-payment v45.0.0
                │   │   │   ├── pallet-transaction-payment-rpc-runtime-api v45.0.0
                │   │   │   │   └── x3-chain-runtime v0.1.0
                │   │   │   └── x3-chain-runtime v0.1.0
                │   │   ├── pallet-treasury v0.1.0 (*)
                │   │   ├── pallet-x3-account-registry v0.1.0
                │   │   │   └── x3-chain-runtime v0.1.0
                │   │   ├── pallet-x3-agent-law v0.1.0
                │   │   │   └── x3-chain-runtime v0.1.0
                │   │   ├── pallet-x3-asset-registry v0.1.0
                │   │   │   └── x3-chain-runtime v0.1.0
                │   │   ├── pallet-x3-atomic-kernel v0.1.0
                │   │   │   └── x3-chain-runtime v0.1.0
                │   │   ├── pallet-x3-auction v0.1.0
                │   │   │   └── x3-chain-runtime v0.1.0
                │   │   ├── pallet-x3-automation v0.1.0 (*)
                │   │   ├── pallet-x3-coin v0.1.0 (*)
                │   │   ├── pallet-x3-compute-market v0.1.0
                │   │   │   └── x3-chain-runtime v0.1.0
                │   │   ├── pallet-x3-consensus v0.1.0 (*)
                │   │   ├── pallet-x3-cross-vm-router v0.1.0
                │   │   │   └── x3-chain-runtime v0.1.0
                │   │   ├── pallet-x3-custody v0.1.0
                │   │   │   └── x3-chain-runtime v0.1.0
                │   │   ├── pallet-x3-da v0.1.0
                │   │   │   └── x3-chain-runtime v0.1.0
                │   │   ├── pallet-x3-dapp-hub v0.1.0
                │   │   │   └── x3-chain-runtime v0.1.0
                │   │   ├── pallet-x3-dex v0.1.0 (*)
                │   │   ├── pallet-x3-domain-registry v0.1.0
                │   │   │   └── x3-chain-runtime v0.1.0
                │   │   ├── pallet-x3-flashloan v0.1.0
                │   │   │   └── x3-chain-runtime v0.1.0
                │   │   ├── pallet-x3-invariants v0.1.0
                │   │   │   ├── pallet-swarm v0.1.0 (*)
                │   │   │   ├── pallet-x3-kernel v0.1.0 (*)
                │   │   │   └── x3-chain-runtime v0.1.0
                │   │   ├── pallet-x3-inventory v0.1.0
                │   │   │   ├── pallet-x3-partner v0.1.0
                │   │   │   │   └── x3-chain-runtime v0.1.0
                │   │   │   ├── pallet-x3-rebalance v0.1.0
                │   │   │   │   └── x3-chain-runtime v0.1.0
                │   │   │   ├── pallet-x3-reservation v0.1.0
                │   │   │   │   ├── pallet-x3-solvency v0.1.0
                │   │   │   │   │   └── x3-chain-runtime v0.1.0
                │   │   │   │   └── x3-chain-runtime v0.1.0
                │   │   │   ├── pallet-x3-solvency v0.1.0 (*)
                │   │   │   ├── pallet-x3-treasury-policy v0.1.0
                │   │   │   │   └── x3-chain-runtime v0.1.0
                │   │   │   └── x3-chain-runtime v0.1.0
                │   │   ├── pallet-x3-jury-anchor v1.0.0 (*)
                │   │   ├── pallet-x3-kernel v0.1.0 (*)
                │   │   ├── pallet-x3-launchpad v0.1.0
                │   │   │   └── x3-chain-runtime v0.1.0
                │   │   ├── pallet-x3-oracle v0.1.0 (*)
                │   │   ├── pallet-x3-partner v0.1.0 (*)
                │   │   ├── pallet-x3-rebalance v0.1.0 (*)
                │   │   ├── pallet-x3-reconciliation v0.1.0
                │   │   │   └── x3-chain-runtime v0.1.0
                │   │   ├── pallet-x3-reservation v0.1.0 (*)
                │   │   ├── pallet-x3-sequencer v0.1.0
                │   │   │   └── x3-chain-runtime v0.1.0
                │   │   ├── pallet-x3-settlement-engine v0.1.0 (*)
                │   │   ├── pallet-x3-slash v0.1.0 (*)
                │   │   ├── pallet-x3-solvency v0.1.0 (*)
                │   │   └── x3-chain-runtime v0.1.0
                │   ├── pallet-staking v45.1.0 (*)
                │   └── x3-chain-runtime v0.1.0
                ├── pallet-staking v45.1.0 (*)
                └── x3-chain-runtime v0.1.0
```

## Dependency Path Analysis

### Root Cause

The dependency path forcing `serde_core 1.0.221+` comes from:

1. **substrate-bip39 v0.6.0** (polkadot-sdk) - This is a host-only crate that depends on `schnorrkel` which requires `serde_bytes` which requires `serde_core`

2. The path flows through:
   - `substrate-bip39` → `sp-core` → `frame-benchmarking` → `pallet-x3-automation` → `x3-chain-runtime`

3. **This is NOT a vendored subxt/hashbrown issue** - the forcing path is from polkadot-sdk's `substrate-bip39` crate.

### Key Finding

The issue is that `substrate-bip39` is being pulled into the runtime dependency graph through `sp-core` → `frame-benchmarking`. This is a **host-only dependency** that should not be available in a `no_std` runtime environment.

## Files Changed to Fix It

### Root Cause Files

1. **runtime/Cargo.toml** - May have `frame-benchmarking` or `sp-core` with features that pull in host-only dependencies
2. **pallets/x3-automation/Cargo.toml** - Uses `frame-benchmarking` which pulls in `sp-core` with host features
3. **pallets/x3-coin/Cargo.toml** - Uses `frame-benchmarking`
4. **pallets/x3-dex/Cargo.toml** - Uses `frame-benchmarking`
5. **pallets/x3-oracle/Cargo.toml** - Uses `frame-benchmarking`
6. **pallets/x3-vrf/Cargo.toml** - Uses `frame-benchmarking`
7. **pallets/x3-wallet-pallet/Cargo.toml** - Uses `frame-benchmarking`
8. **pallets/x3-slash/Cargo.toml** - Uses `pallet-staking` which pulls in `sp-core`

### Fix Strategy

The fix requires:

1. **Remove `frame-benchmarking` from runtime builds** - This is a host-only crate for benchmarking
2. **Gate `frame-benchmarking` behind `feature = "std"`** in all pallets that use it
3. **Ensure `sp-core` is used with `default-features = false`** in runtime builds
4. **Split any shared crates** that depend on host-only dependencies into:
   - `no_std` core types crate (for runtime)
   - `std` client crate (for node/rpc/indexer)

## Next Steps

1. Inspect all pallets that use `frame-benchmarking`
2. Gate `frame-benchmarking` behind `feature = "std"`
3. Ensure `sp-core` uses `default-features = false` in runtime builds
4. Verify no runtime pallet imports host-only modules
5. Test isolated runtime build with `--no-default-features`

## Acceptance Criteria

This blocker is closed only when:
- `cargo check --manifest-path runtime/Cargo.toml --no-default-features` passes
- `cargo check --manifest-path runtime/Cargo.toml --features std` passes
- `cargo build -p x3-chain-runtime --release` passes
- `bash scripts/mainnet/mainnet_rc_gate.sh` passes
