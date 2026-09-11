# Useful Linux commands

- Activate Serena project from repo root: `serena project activate` (or use the IDE integration); inspect memories with `serena memories check`.
- Rust dev node: `cargo build --release -p x3-chain-node`; verify `./target/release/x3-chain-node --version`; start with `./scripts/start-x3-chain.sh`.
- Easy validator: `./scripts/start-validator-easy.sh --chain deployment/chain-specs/x3-testnet-raw.json`.
- Local 3-validator testnet: `./scripts/testnet-full-launch.sh`.
- Targeted pallet tests: `cargo test -p pallet-x3-cross-vm-router -- --nocapture`, `cargo test -p pallet-x3-supply-ledger -- --nocapture`, `cargo test -p pallet-x3-settlement-engine -- --nocapture`.
- Use `cargo fmt --all -- --check` for formatting. Use `git --no-pager` for non-interactive history/diffs.
- Do not use `scripts/start-mock-rpc-dev.sh` or `scripts/mock-rpc-server.js` for consensus/testnet claims; they are frontend-only fake RPC tooling.