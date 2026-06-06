use x3_verification_router::{ExternalAssetRef, ExternalChainId};

pub type ProofId = [u8; 32];
pub type BlockNumber = u64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProofEnvelope {
    pub version: u16,
    pub proof_id: ProofId,
    pub source_chain: ExternalChainId,
    pub source_block: BlockNumber,
    pub source_tx_hash: [u8; 32],
    pub event_index: u32,
    pub external_asset: ExternalAssetRef,
    pub sender: String,
    pub recipient: String,
    pub amount: u128,
    pub nonce: u64,
    pub observed_at_block: BlockNumber,
    pub finalized_at_block: BlockNumber,
    pub proof_payload: Vec<u8>,
}

impl ProofEnvelope {
    pub fn deterministic_proof_id(
        source_chain: ExternalChainId,
        source_tx_hash: [u8; 32],
        event_index: u32,
        nonce: u64,
    ) -> ProofId {
        let mut out = [0u8; 32];
        let chain_tag = match source_chain {
            ExternalChainId::EthereumSepolia => 1u8,
            ExternalChainId::BaseSepolia => 2,
            ExternalChainId::SolanaDevnet => 3,
            ExternalChainId::EthereumMainnet => 4,
            ExternalChainId::BaseMainnet => 5,
            ExternalChainId::SolanaMainnet => 6,
        };
        out[0] = chain_tag;
        for (idx, byte) in source_tx_hash.iter().enumerate() {
            out[idx % 32] ^= *byte;
        }
        for (idx, byte) in event_index.to_be_bytes().iter().enumerate() {
            out[24 + idx] ^= *byte;
        }
        for (idx, byte) in nonce.to_be_bytes().iter().enumerate() {
            out[idx] ^= *byte;
        }
        out
    }

    pub fn external_nonce_key(&self) -> (ExternalChainId, String, u64) {
        (
            self.source_chain,
            self.external_asset.token_address_or_mint.clone(),
            self.nonce,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proof_id_is_deterministic_and_nonce_sensitive() {
        let a = ProofEnvelope::deterministic_proof_id(ExternalChainId::BaseSepolia, [7; 32], 1, 10);
        let b = ProofEnvelope::deterministic_proof_id(ExternalChainId::BaseSepolia, [7; 32], 1, 10);
        let c = ProofEnvelope::deterministic_proof_id(ExternalChainId::BaseSepolia, [7; 32], 1, 11);
        assert_eq!(a, b);
        assert_ne!(a, c);
    }
}
