# Process Isolation Design: Electron/Tauri + IPC Contract

## Core Philosophy
The renderer (UI) runs in a completely unprivileged environment without node integrations. It can only emit structured Intents over the Tauri IPC channel. The Rust backend (Wallet Core) canonicalizes the intent, coordinates attestation/validation, and finally hands it to fully isolated signer implementations.

## Rust Module Mapping

```rust
// x3-desktop/src-tauri/src/wallet_core/signers/mod.rs

pub trait IsolatedSigner {
    fn derive_address(&self, path: &str) -> Result<String, SignerError>;
    
    // Crucial rule: preimage only allowed IF accompanied by valid attestation
    fn sign_intent(&self, preimage: &IntentDraft, attestation: &Attestation) -> Result<String, SignerError>;
    
    // Strict bytes signing ONLY IF the tx exactly matches the approved intent
    fn sign_tx(&self, canonical_tx_bytes: &[u8], intent_id: &str) -> Result<String, SignerError>;
    
    fn get_capabilities(&self) -> SignerCaps;
}
```

## Security Posture
- **Rust Backend Coordinator:** Acts as the explicit gatekeeper (`src-tauri/src/wallet_core/coordinator.rs`).
- **Memory Security:** Implemented `secure_memory` module to eventually leverage Memory Enclaves or Linux mlock commands to prevent OS swapping or core-dump extraction.
- **IPC Typescript Map:** Pre-defined types ensuring type-safe serialization mapping identically to `serde_json` structures in rust (e.g. `AssetRequirement`, `FeeCap`, `Attestation`). See `/home/lojak/Desktop/x3-chain-master/apps/x3-desktop/src/types/ipc/wallet.types.ts`.

## Implementation Files Addressed:
1. `src-tauri/src/wallet_core/mod.rs`
2. `src-tauri/src/wallet_core/ipc.rs`
3. `src-tauri/src/wallet_core/coordinator.rs`
4. `src-tauri/src/wallet_core/signers/mod.rs`
5. `src-tauri/src/wallet_core/signers/evm.rs`
6. `src-tauri/src/wallet_core/signers/svm.rs`
7. `src-tauri/src/wallet_core/signers/btc.rs`
8. `src-tauri/src/wallet_core/signers/common/secure_memory.rs`
