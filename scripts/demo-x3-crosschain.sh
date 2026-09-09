#!/usr/bin/env bash
set -euo pipefail

# Reproducible repository demonstration. It proves source compilation and
# X3-internal runtime routing only. It does not deploy to public chains.

cargo test -p x3-x3-integration --features compile --test compiler_bridge
cargo test -p x3-compiler gateway::
cargo test -p pallet-x3-cross-vm-router compiled_x3_lang_gateway_path_routes_and_rejects_direct_unsigned
cargo test -p pallet-x3-cross-vm-router test_expired_transfer_refunds_to_source
cargo test -p pallet-x3-cross-vm-router test_duplicate_message_replay_rejected
cargo test -p pallet-x3-cross-vm-router test_x3_native_evm_svm_roundtrip_preserves_supply

printf '%s\n' "Repository proof complete."
printf '%s\n' "External Anvil/Solana receipts are a separate pending proof; no public-chain deployment is claimed."
