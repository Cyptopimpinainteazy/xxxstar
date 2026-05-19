#!/bin/bash
set -e

echo "Building parallel-proposer components..."

# Build all crates
echo "Building parallel-proposer crate..."
cd crates/parallel-proposer
cargo build --release
cd ../..

echo "Building gpu-sig-verifier crate..."
cd crates/gpu-sig-verifier
cargo build --release
cd ../..

echo "Building import-queue-wrapper crate..."
cd crates/import-queue-wrapper
cargo build --release
cd ../..

echo "Building contention-predictor crate..."
cd crates/contention-predictor
cargo build --release
cd ../..

# Run integration tests
echo "Running integration tests..."
cd integration-tests
cargo test --release

echo "Build and test completed successfully!"