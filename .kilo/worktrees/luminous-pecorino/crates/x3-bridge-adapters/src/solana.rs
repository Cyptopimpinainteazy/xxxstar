//! Solana Bridge Adapter
//!
//! Provides bridge functionality for Solana-compatible chains.

use crate::{BridgeAdapter, BridgeError};

/// Solana Bridge Adapter
pub struct SolanaBridgeAdapter {
    chain_id: u64,
    rpc_url: String,
}

impl SolanaBridgeAdapter {
    /// Create a new Solana bridge adapter
    pub fn new(chain_id: u64, rpc_url: String) -> Self {
        Self { chain_id, rpc_url }
    }

    /// Get the RPC URL
    pub fn rpc_url(&self) -> &str {
        &self.rpc_url
    }
}

impl BridgeAdapter for SolanaBridgeAdapter {
    fn chain_name(&self) -> &str {
        "solana"
    }

    fn chain_id(&self) -> u64 {
        self.chain_id
    }

    fn validate_header(&self, _header: &[u8]) -> Result<(), BridgeError> {
        // TODO: Implement header validation
        Ok(())
    }

    fn generate_proof(&self, _block_number: u64) -> Result<Vec<u8>, BridgeError> {
        // TODO: Implement proof generation
        Ok(vec![])
    }

    fn get_latest_block_number(&self) -> Result<u64, BridgeError> {
        // TODO: Implement block number retrieval
        Ok(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_solana_adapter_creation() {
        let adapter = SolanaBridgeAdapter::new(1399811149, "http://localhost:8899".to_string());
        assert_eq!(adapter.chain_name(), "solana");
        assert_eq!(adapter.chain_id(), 1399811149);
    }
}
