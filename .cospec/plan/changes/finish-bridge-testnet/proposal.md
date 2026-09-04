# Change: Finish Remaining Bridge Testnet Tasks

## Rationale
The bridge testnet enablement (change `enable-bridge-testnet`) was substantially completed in commit `2f8753f89`, but several critical items remain unfinished: `ExternalBridgesEnabled` is not set in genesis config (defaults to `false`), the raw chain spec lacks the storage key, and workspace build verification has not been performed. This change completes the remaining tasks to make the bridge testnet fully functional.

## Changes
- Inject `ExternalBridgesEnabled = true` storage key into the raw testnet chain spec JSON
- Verify workspace build compiles (`cargo check --workspace`)
- Resolve any remaining Cargo.lock version conflicts
- Update `node/src/chain_spec.rs` to include `x3_cross_vm_router` genesis config
- Update task.md to reflect actual completion status of all items

## Impact
- **Affected Specifications**: Bridge/Router, Chain Spec, Workspace Build
- **Affected Code**:
  - `deployment/chain-specs/x3-testnet-raw.json`: Add `ExternalBridgesEnabled` storage key with value `0x01`
  - `node/src/chain_spec.rs`: Add `x3_cross_vm_router` genesis config to `x3_chain_genesis()` function
  - `Cargo.lock`: Resolve any remaining version conflicts via `cargo update`
  - `.cospec/plan/changes/enable-bridge-testnet/task.md`: Mark completed items as done
