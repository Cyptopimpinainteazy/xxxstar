use super::ipc::{IntentDraft, Attestation, VerifiedIntent};

pub struct WalletCoordinator {
    // Isolated process pipe configurations
    pub evm_pipe: String,
    pub svm_pipe: String,
    pub btc_pipe: String,
}

impl WalletCoordinator {
    pub fn new() -> Self {
        Self {
            evm_pipe: "/var/run/x3-wallet/evm.sock".into(),
            svm_pipe: "/var/run/x3-wallet/svm.sock".into(),
            btc_pipe: "/var/run/x3-wallet/btc.sock".into(),
        }
    }

    /// Primary interface called by the UI (Tauri Command layer)
    /// UI ONLY sends the Intent.
    pub async fn create_intent_draft(draft: IntentDraft) -> Result<String, String> {
        // Step 1: Canonicalize intent
        let intent_hash = Self::canonicalize(&draft);

        // Step 2: Push to Verifier Quorum (Multi-RPC sanity check)
        let attestation = super::verifier::verify_intent(&draft).await?;

        // In a true IPC boundary, we wait for an async response from the isolated signer.
        // For development GUI execution, we simulate handing the VerifiedIntent to a signer instance.
        Ok(format!("Intent {} verified & compiled successfully! System Attestation: {}", intent_hash, attestation.signature))

    }

    /// Strict canonical hashing to prevent tx manipulation
    fn canonicalize(draft: &IntentDraft) -> String {
        // e.g. Bencode serialization + SHA256 ensures deterministic field ordering
        let bytes = bincode::serialize(draft).unwrap();
        hex::encode(sp_core::hashing::sha2_256(&bytes))
    }
}
