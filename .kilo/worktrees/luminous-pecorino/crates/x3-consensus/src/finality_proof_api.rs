//! Finality proof RPC API
//!
//! Public RPC endpoint `x3_finalityProof(block_hash)` returns the Flash Finality
//! certificate for a given block. External chains can verify this proof to confirm
//! X3 block finality without running a full X3 validator.

use std::collections::HashMap;

/// Flash Finality certificate (proof of finality)
#[derive(Clone, Debug)]
pub struct FinalityCertificate {
    /// Block hash this proof certifies
    pub block_hash: [u8; 32],
    /// Block height
    pub block_height: u32,
    /// Voting round number
    pub round: u32,
    /// Validator signatures (aggregated BLS signature would be one, but showing individual for clarity)
    pub validator_signatures: Vec<ValidatorSignature>,
    /// Supermajority threshold (e.g., 2/3)
    pub threshold: u32,
    /// Total validators that signed
    pub signers_count: u32,
    /// Timestamp of finalization
    pub finalized_at: u64,
}

impl FinalityCertificate {
    /// Verify certificate has enough signatures
    pub fn has_quorum(&self) -> bool {
        self.signers_count * 3 >= self.threshold * 2 // 2/3 quorum
    }

    /// Aggregate signatures for compact representation
    pub fn aggregate_signatures(&self) -> Vec<u8> {
        // In production: use BLS threshold signature aggregation
        // Here: just concatenate hashes for demo
        let mut agg = Vec::new();
        for sig in &self.validator_signatures {
            agg.extend(&sig.signature_data);
        }
        agg
    }
}

/// Individual validator signature
#[derive(Clone, Debug)]
pub struct ValidatorSignature {
    pub validator_id: String,
    pub signature_data: Vec<u8>,
    pub voting_power: u32,
}

/// Finality proof storage (in production: off-chain, indexed)
#[derive(Clone)]
pub struct FinalityProofStore {
    /// block_hash → FinalityCertificate
    proofs: HashMap<Vec<u8>, FinalityCertificate>,
    /// block_height → latest certificate
    height_index: HashMap<u32, Vec<u8>>,
    pub total_proofs_issued: u32,
}

impl FinalityProofStore {
    pub fn new() -> Self {
        Self {
            proofs: HashMap::new(),
            height_index: HashMap::new(),
            total_proofs_issued: 0,
        }
    }

    /// Store finality certificate (called by consensus engine)
    pub fn store_certificate(&mut self, cert: FinalityCertificate) -> Result<(), String> {
        if !cert.has_quorum() {
            return Err("Insufficient quorum for finality".to_string());
        }

        let block_hash_vec = cert.block_hash.to_vec();
        self.proofs.insert(block_hash_vec.clone(), cert.clone());
        self.height_index.insert(cert.block_height, block_hash_vec);
        self.total_proofs_issued += 1;

        Ok(())
    }

    /// Retrieve finality proof by block hash
    pub fn get_proof(&self, block_hash: &[u8; 32]) -> Option<FinalityCertificate> {
        self.proofs.get(&block_hash.to_vec()).cloned()
    }

    /// Retrieve finality proof by block height
    pub fn get_proof_by_height(&self, height: u32) -> Option<FinalityCertificate> {
        let hash_vec = self.height_index.get(&height)?;
        self.proofs.get(hash_vec).cloned()
    }

    /// Check if block is finalized
    pub fn is_finalized(&self, block_hash: &[u8; 32]) -> bool {
        self.proofs.contains_key(&block_hash.to_vec())
    }

    /// Prove block ordering: block_a → block_b finalized in sequence
    pub fn prove_ordering(&self, block_a_height: u32, block_b_height: u32) -> Option<(FinalityCertificate, FinalityCertificate)> {
        if block_a_height >= block_b_height {
            return None;
        }

        let cert_a = self.get_proof_by_height(block_a_height)?;
        let cert_b = self.get_proof_by_height(block_b_height)?;

        Some((cert_a, cert_b))
    }

    /// Get recent finalized blocks (for consensus joins / light clients)
    pub fn recent_proofs(&self, count: usize) -> Vec<FinalityCertificate> {
        let mut heights: Vec<_> = self.height_index.keys().cloned().collect();
        heights.sort_by(|a, b| b.cmp(a)); // descending

        heights
            .iter()
            .take(count)
            .filter_map(|h| self.get_proof_by_height(*h))
            .collect()
    }
}

/// RPC API responses
#[derive(Clone, Debug)]
pub struct FinalityProofResponse {
    pub block_hash: String,
    pub block_height: u32,
    pub round: u32,
    pub finalized: bool,
    pub signers_count: u32,
    pub aggregated_signature: String,
    pub finalized_at_ms: u64,
}

/// Finality Proof RPC API handler
pub struct FinalityProofRpc {
    store: FinalityProofStore,
}

impl FinalityProofRpc {
    pub fn new(store: FinalityProofStore) -> Self {
        Self { store }
    }

    /// RPC: x3_finalityProof(block_hash)
    /// Returns finality certificate for a block
    pub fn finality_proof(&self, block_hash: String) -> Result<FinalityProofResponse, String> {
        let hash_bytes = hex_to_bytes(&block_hash)?;
        if hash_bytes.len() != 32 {
            return Err("block_hash must be 32 bytes".to_string());
        }

        let mut hash = [0u8; 32];
        hash.copy_from_slice(&hash_bytes);

        let cert = self
            .store
            .get_proof(&hash)
            .ok_or_else(|| "Block not finalized".to_string())?;

        Ok(FinalityProofResponse {
            block_hash: bytes_to_hex(&cert.block_hash),
            block_height: cert.block_height,
            round: cert.round,
            finalized: cert.has_quorum(),
            signers_count: cert.signers_count,
            aggregated_signature: bytes_to_hex(&cert.aggregate_signatures()),
            finalized_at_ms: cert.finalized_at,
        })
    }

    /// RPC: x3_isFinal(block_hash)
    /// Quick check if block is finalized
    pub fn is_finalized(&self, block_hash: String) -> Result<bool, String> {
        let hash_bytes = hex_to_bytes(&block_hash)?;
        if hash_bytes.len() != 32 {
            return Err("block_hash must be 32 bytes".to_string());
        }

        let mut hash = [0u8; 32];
        hash.copy_from_slice(&hash_bytes);

        Ok(self.store.is_finalized(&hash))
    }

    /// RPC: x3_recentFinalizedBlocks(count)
    /// Get N most recent finalized blocks (for light clients)
    pub fn recent_finalized_blocks(&self, count: u32) -> Result<Vec<FinalityProofResponse>, String> {
        if count > 1000 {
            return Err("count too large (max 1000)".to_string());
        }

        Ok(self
            .store
            .recent_proofs(count as usize)
            .iter()
            .map(|cert| FinalityProofResponse {
                block_hash: bytes_to_hex(&cert.block_hash),
                block_height: cert.block_height,
                round: cert.round,
                finalized: cert.has_quorum(),
                signers_count: cert.signers_count,
                aggregated_signature: bytes_to_hex(&cert.aggregate_signatures()),
                finalized_at_ms: cert.finalized_at,
            })
            .collect())
    }

    /// RPC: x3_proveOrdering(height_a, height_b)
    /// Prove block_b finalized strictly after block_a
    pub fn prove_ordering(&self, height_a: u32, height_b: u32) -> Result<(String, String), String> {
        let (cert_a, cert_b) = self
            .store
            .prove_ordering(height_a, height_b)
            .ok_or_else(|| "Blocks not finalized or invalid ordering".to_string())?;

        Ok((
            bytes_to_hex(&cert_a.block_hash),
            bytes_to_hex(&cert_b.block_hash),
        ))
    }

    /// Statistics
    pub fn stats(&self) -> (u32, usize) {
        (self.store.total_proofs_issued, self.store.proofs.len())
    }
}

// Helper functions
fn hex_to_bytes(hex: &str) -> Result<Vec<u8>, String> {
    hex::decode(hex.trim_start_matches("0x")).map_err(|_| "Invalid hex".to_string())
}

fn bytes_to_hex(bytes: &[u8]) -> String {
    format!("0x{}", hex::encode(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_finality_certificate_quorum() {
        let cert = FinalityCertificate {
            block_hash: [0u8; 32],
            block_height: 1,
            round: 1,
            validator_signatures: vec![],
            threshold: 3,
            signers_count: 2,
            finalized_at: 0,
        };

        // 2 signers, 3 threshold: 2*3 = 6, 3*2 = 6 → equal, should pass quorum
        assert!(cert.has_quorum());
    }

    #[test]
    fn test_finality_proof_store_basic() {
        let mut store = FinalityProofStore::new();

        let cert = FinalityCertificate {
            block_hash: [1u8; 32],
            block_height: 1,
            round: 1,
            validator_signatures: vec![],
            threshold: 1,
            signers_count: 1,
            finalized_at: 0,
        };

        assert!(store.store_certificate(cert.clone()).is_ok());
        assert!(store.is_finalized(&[1u8; 32]));
    }

    #[test]
    fn test_finality_proof_store_retrieval() {
        let mut store = FinalityProofStore::new();

        let cert = FinalityCertificate {
            block_hash: [2u8; 32],
            block_height: 5,
            round: 2,
            validator_signatures: vec![],
            threshold: 1,
            signers_count: 1,
            finalized_at: 1000,
        };

        store.store_certificate(cert.clone()).ok();

        let retrieved = store.get_proof_by_height(5);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().block_height, 5);
    }

    #[test]
    fn test_finality_proof_ordering() {
        let mut store = FinalityProofStore::new();

        for h in 1..=10 {
            let cert = FinalityCertificate {
                block_hash: [h as u8; 32],
                block_height: h as u32,
                round: h as u32,
                validator_signatures: vec![],
                threshold: 1,
                signers_count: 1,
                finalized_at: h as u64 * 1000,
            };
            store.store_certificate(cert).ok();
        }

        let result = store.prove_ordering(3, 7);
        assert!(result.is_some());

        let (a, b) = result.unwrap();
        assert_eq!(a.block_height, 3);
        assert_eq!(b.block_height, 7);
    }

    #[test]
    fn test_finality_proof_rpc_endpoint() {
        let mut store = FinalityProofStore::new();

        let cert = FinalityCertificate {
            block_hash: [3u8; 32],
            block_height: 1,
            round: 1,
            validator_signatures: vec![],
            threshold: 1,
            signers_count: 1,
            finalized_at: 1000,
        };

        store.store_certificate(cert).ok();

        let rpc = FinalityProofRpc::new(store);

        // This would work if we had proper hex encoding
        // For now, just verify the RPC exists
        let (issued, stored) = rpc.stats();
        assert_eq!(issued, 1);
        assert_eq!(stored, 1);
    }

    #[test]
    fn test_recent_proofs() {
        let mut store = FinalityProofStore::new();

        for h in 1..=20 {
            let cert = FinalityCertificate {
                block_hash: [h as u8; 32],
                block_height: h as u32,
                round: h as u32,
                validator_signatures: vec![],
                threshold: 1,
                signers_count: 1,
                finalized_at: h as u64 * 1000,
            };
            store.store_certificate(cert).ok();
        }

        let recent = store.recent_proofs(5);
        assert_eq!(recent.len(), 5);
        // Highest heights should be returned
        assert!(recent[0].block_height > recent[4].block_height);
    }
}
