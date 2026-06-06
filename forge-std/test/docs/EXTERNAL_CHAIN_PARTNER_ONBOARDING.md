# External Chain Partner Onboarding (Easy Path)

This runbook is the fastest way for partner chains to integrate with X3 atomic cross-chain trading.

## Fee Policy (Canonical)

- Cross-VM trades: **4%** protocol fee
- Same-VM, different-blockchain trades: **2%** protocol fee
- Same-chain trades: **0%** protocol fee

These rules are enforced in `crates/x3-swap-router/src/fee_calculator.rs`.

## One-Call Chain Onboarding API

Use `ChainConfig::onboard_external_chain(...)` to onboard an external chain with validation.

```rust
use external_chains::adapter::ChainConfig;
use sp_core::H160;

let config = ChainConfig::onboard_external_chain(
    9999,
    "https://rpc.partner-chain.example",
    H160::from_low_u64_be(11),
    H160::from_low_u64_be(12),
    2,
)?;

config.validate()?;
```

If the partner chain is not in the built-in registry, create a ready adapter in one call:

```rust
use external_chains::onboard_external_adapter;
use sp_core::H160;

let adapter = onboard_external_adapter(
    9999,
    "https://rpc.partner-chain.example",
    H160::from_low_u64_be(11),
    H160::from_low_u64_be(12),
    2,
)?;

// Validated config now available through adapter.config()
let chain_id = adapter.config().chain_type;
```

### Required inputs

- `chain_id`: Partner chain numeric ID (must be non-zero)
- `rpc_url`: HTTP(S) RPC endpoint
- `bridge_contract`: Partner bridge contract address
- `settlement_contract`: Partner settlement contract address
- `confirmations`: Confirmation count for finality (must be > 0)

### Built-in validation

- Rejects empty/invalid chain ID
- Rejects non-HTTP RPC URLs
- Rejects zero-address contracts
- Rejects zero confirmations
- Rejects zero gas limit in final config validation

## Router Integration for Correct Fees

When executing swaps, provide VM context explicitly so fee tiering is deterministic.

```rust
use x3_swap_router::{SwapParams, VmType};

let params = SwapParams {
    token_in,
    token_out,
    amount_in,
    min_amount_out,
    chain_in,
    chain_out,
    deadline,
    recipient,
    slippage_tolerance_bps: 50,
    gas_price_limit: None,
    source_vm: VmType::Evm,
    destination_vm: VmType::Svm,
};
```

If VM fields are left `Unknown`, the router uses built-in chain-ID inference for common EVM/SVM IDs.

## Production Checklist

1. Verify partner RPC uptime and latency SLOs
2. Deploy and verify bridge + settlement contracts
3. Set conservative confirmations per chain finality model
4. Run integration tests for transfer, message proof, and rollback behavior
5. Monitor fee calculations for cross-VM and same-VM cross-chain paths
