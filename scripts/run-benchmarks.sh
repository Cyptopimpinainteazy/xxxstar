#!/bin/bash
# X3 Chain Benchmark Runner
# Runs all pallet benchmarks and generates weight files

set -e

echo "=== X3 Chain Benchmark Runner ==="
echo "This script runs all pallet benchmarks and generates weight files."
echo ""

# Check if runtime-benchmarks feature is enabled
if ! grep -q "runtime-benchmarks" runtime/Cargo.toml; then
    echo "ERROR: runtime-benchmarks feature not found in runtime/Cargo.toml"
    exit 1
fi

# Build with runtime-benchmarks feature
echo "Building with runtime-benchmarks feature..."
cargo build --release --features runtime-benchmarks

echo ""
echo "=== Running Pallet Benchmarks ==="
echo ""

# List of pallets to benchmark
PALETS=(
    "pallet_x3_kernel"
    "pallet_x3_invariants"
    "pallet_x3_atomic_kernel"
    "pallet_x3_cross_vm_router"
    "pallet_x3_coin"
    "pallet_x3_asset_registry"
    "pallet_x3_domain_registry"
    "pallet_x3_settlement_engine"
    "pallet_x3_oracle"
    "pallet_x3_vrf"
    "pallet_x3_dex"
    "pallet_governance"
    "pallet_treasury"
    "pallet_agent_accounts"
    "pallet_agent_memory"
    "pallet_evolution_core"
    "pallet_x3_verifier"
    "pallet_cross_chain_validator"
    "pallet_x3_token_factory"
    "pallet_x3_automation"
    "pallet_x3_slash"
    "pallet_depin_marketplace"
    "pallet_private_execution"
    "pallet_meme_overlord"
    "pallet_swarm"
)

# Run benchmarks for each pallet
for pallet in "${PALETS[@]}"; do
    echo "=== Benchmarking $pallet ==="
    ./target/release/x3-chain-node benchmark pallet \
        --chain dev \
        --steps 50 \
        --repeat 20 \
        --pallet "$pallet" \
        --extrinsic "*" \
        --execution=wasm \
        --wasm-execution=compiled \
        --heap-pages=4096 \
        --output "./pallets/${pallet#pallet_}/src/weights.rs" \
        --template ./.maintain/frame-weight-template.hbs
    echo ""
done

echo "=== Benchmarking Complete ==="
echo "Weight files have been generated in each pallet's src/weights.rs"
