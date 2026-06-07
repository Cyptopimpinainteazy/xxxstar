//! Cross-VM message format.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{ChainId, OrchestratorError, Result, VmKind};

/// Canonical cross-VM message exchanged between adapters.
///
/// The message identifier is the SHA-256 digest of its canonical JSON
/// encoding. This makes IDs deterministic and replay detection robust
/// across adapter implementations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossVmMessage {
    pub source_chain: ChainId,
    pub target_chain: ChainId,
    pub source_vm: VmKind,
    pub target_vm: VmKind,
    pub sender: Vec<u8>,
    pub target: Vec<u8>,
    pub payload: Vec<u8>,
    pub gas_limit: u64,
    pub nonce: u64,
    pub expiry_block: u64,
}

impl CrossVmMessage {
    /// Compute the deterministic message id (hex-encoded SHA-256 of the
    /// canonical JSON serialization).
    pub fn id(&self) -> Result<String> {
        let encoded = serde_json::to_vec(self)
            .map_err(|e| OrchestratorError::RoutingFailed(format!("serialize message: {e}")))?;
        let hash = Sha256::digest(encoded);
        Ok(hex::encode(hash))
    }
}
