#!/bin/bash
# Script to generate basic fuzz harnesses for X3 pallets

PALLET_LIST=(
    "agent-accounts"
    "agent-memory"
    "atomic-trade-engine"
    "cross-chain-validator"
    "depin-marketplace"
    "evolution-core"
    "fraud-proofs"
    "governance"
    "meme-overlord"
    "private-execution"
    "svm-runtime"
    "swarm"
    "treasury"
    "x3-account-registry"
    "x3-asset-registry"
    "x3-automation"
    "x3-coin"
    "x3-da"
    "x3-domain-registry"
    "x3-invariants"
    "x3-inventory"
    "x3-jury-anchor"
    "x3-kernel"
    "x3-reservation"
    "x3-sequencer"
    "x3-slash"
    "x3-solvency"
    "x3-supply-ledger"
    "x3-token-factory"
    "x3-verifier"
    "x3-wallet-pallet"
)

for pallet in "${PALLET_LIST[@]}"; do
    echo "Generating fuzz harness for pallet-${pallet}..."

    # Create fuzz directory
    mkdir -p "pallets/${pallet}/fuzz/fuzz_targets"

    # Create Cargo.toml
    cat > "pallets/${pallet}/fuzz/Cargo.toml" << EOF
[package]
name = "pallet-${pallet}-fuzz"
version = "0.0.0"
publish = false
edition = "2021"

[package.metadata]
cargo-fuzz = true

# Exclude from root workspace to allow nightly fuzz builds
[workspace]

[dependencies]
libfuzzer-sys = "0.4"
parity-scale-codec = { version = "3", features = ["derive"] }
sp-core = { default-features = false, git = "https://github.com/paritytech/polkadot-sdk", tag = "polkadot-stable2603" }

[dependencies.pallet-${pallet}]
path = ".."

[[bin]]
name = "fuzz_codec_parsing"
path = "fuzz_targets/fuzz_codec_parsing.rs"
test = false
doc = false
bench = false
EOF

    # Create basic codec fuzz target
    cat > "pallets/${pallet}/fuzz/fuzz_targets/fuzz_codec_parsing.rs" << EOF
//! Fuzz target: Codec parsing for ${pallet} structures
//!
//! Feeds arbitrary bytes through pallet structure decoders
//! to ensure no panics in serialization.

#![no_main]

use libfuzzer_sys::fuzz_target;
use parity_scale_codec::Decode;

// Note: This is a basic template. Add specific structure imports and tests as needed.
// Look at the pallet's lib.rs for structures that can be fuzzed.

fuzz_target!(|data: &[u8]| {
    // TODO: Add specific structure decoding tests for pallet-${pallet}
    // Example:
    // if let Ok(structure) = SomeStructure::decode(&mut &*data) {
    //     // Test invariants
    //     let re_encoded = structure.encode();
    //     let re_decoded = SomeStructure::decode(&mut &re_encoded[..]).unwrap();
    //     assert_eq!(structure, re_decoded, "Codec must be deterministic");
    // }

    // For now, just ensure no panics occur with arbitrary data
    let _ = data; // Prevent unused variable warning
});
EOF

done

echo "Generated basic fuzz harnesses for ${#PALLET_LIST[@]} pallets."
echo "Next steps:"
echo "1. Review each pallet's lib.rs to identify fuzzeable structures and functions"
echo "2. Update the fuzz targets with specific tests (codec parsing, calculation functions, etc.)"
echo "3. Add the fuzz targets to CI build.yml"
echo "4. Test compilation and fix any issues"