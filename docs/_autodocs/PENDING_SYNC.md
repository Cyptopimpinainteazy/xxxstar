# Auto-Doc Synchronization

This directory is managed by `markdown-autodocs`. It contains auto-generated documentation snippets extracted from inline doc comments across the X3 codebase.

## Modules Documented

- **Bridge Adapters** (`crates/x3-bridge-adapters`) — Ethereum, Solana, and Bitcoin RPC bridge adapters with real JSON-RPC proof generation and header validation.
- **Validator RPC** (`crates/x3-rpc`) — Live validator leaderboard, metrics, and authorities queried from the runtime API.
- **Settlement Engine** (`pallets/x3-settlement-engine`) — Atomic cross-chain settlement with executor authorization gated by `pallet_x3_kernel::AuthorizedAccounts`.
- **Cross-VM Router** (`pallets/x3-cross-vm-router`) — Internal X3Native/X3Evm/X3Svm transfer routing with replay protection and state machine enforcement.
- **Register Allocator** (`x3-lang/compiler/src/regalloc.rs`) — Deterministic linear-scan register allocator with 16 physical registers and stack spill slots.
- **Bytecode Format** (`crates/x3-backend`) — X3BC binary format with CRC32 checksum validation on load.

## Regeneration

To regenerate all auto-doc snippets:
```bash
npx markdown-autodocs
```

See `DOCUMENTATION_INDEX.md` for the full documentation map.
