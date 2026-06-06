//! Finality Oracle Module
//!
//! Tracks confirmation depth and finality across external chains.
//! X3 uses this to determine when proofs are trustworthy.

use crate::types::{ExternalChainId, FinalityConfig, ProofType};
use codec::{Decode, DecodeWithMemTracking, Encode};
use core::fmt::Debug;
use scale_info::TypeInfo;
use sp_core::H256;
use sp_std::vec::Vec;

/// Chain finality status
#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, TypeInfo)]
pub struct ChainStatus {
    /// Chain identifier
    pub chain: ExternalChainId,
    /// Latest known block height
    pub latest_height: u64,
    /// Latest known block hash
    pub latest_hash: H256,
    /// Last update timestamp
    pub last_update: u64,
    /// Is chain healthy
    pub is_healthy: bool,
    /// Recent reorg depth (if any)
    pub recent_reorg_depth: u32,
}

/// Finality oracle for external chains
pub struct FinalityOracle;

impl FinalityOracle {
    /// Get default finality configuration for a chain
    pub fn default_config(chain: ExternalChainId) -> FinalityConfig {
        match chain {
            ExternalChainId::Bitcoin | ExternalChainId::BitcoinTestnet => FinalityConfig {
                chain,
                confirmations_required: 6,
                block_time_ms: 600_000, // 10 minutes
                proof_type: ProofType::BitcoinSpv,
                challenge_period_seconds: 0,
                max_reorg_depth: 6,
            },
            ExternalChainId::Ethereum => FinalityConfig {
                chain,
                confirmations_required: 12,
                block_time_ms: 12_000, // 12 seconds
                proof_type: ProofType::MerkleTrie,
                challenge_period_seconds: 0,
                max_reorg_depth: 12,
            },
            ExternalChainId::Arbitrum => FinalityConfig {
                chain,
                confirmations_required: 1,
                block_time_ms: 250, // ~0.25 seconds
                proof_type: ProofType::Optimistic,
                challenge_period_seconds: 604800, // 7 days for fraud proof
                max_reorg_depth: 0,
            },
            ExternalChainId::Base => FinalityConfig {
                chain,
                confirmations_required: 1,
                block_time_ms: 2_000,
                proof_type: ProofType::Optimistic,
                challenge_period_seconds: 604800,
                max_reorg_depth: 0,
            },
            ExternalChainId::Optimism => FinalityConfig {
                chain,
                confirmations_required: 1,
                block_time_ms: 2_000,
                proof_type: ProofType::Optimistic,
                challenge_period_seconds: 604800,
                max_reorg_depth: 0,
            },
            ExternalChainId::Polygon => FinalityConfig {
                chain,
                confirmations_required: 128,
                block_time_ms: 2_000,
                proof_type: ProofType::MerkleTrie,
                challenge_period_seconds: 0,
                max_reorg_depth: 128,
            },
            ExternalChainId::Avalanche => FinalityConfig {
                chain,
                confirmations_required: 1,
                block_time_ms: 2_000, // Sub-second finality
                proof_type: ProofType::MerkleTrie,
                challenge_period_seconds: 0,
                max_reorg_depth: 0, // Avalanche has instant finality
            },
            ExternalChainId::Bnb => FinalityConfig {
                chain,
                confirmations_required: 15,
                block_time_ms: 3_000,
                proof_type: ProofType::MerkleTrie,
                challenge_period_seconds: 0,
                max_reorg_depth: 15,
            },
            ExternalChainId::Solana | ExternalChainId::SolanaDevnet => FinalityConfig {
                chain,
                confirmations_required: 1,
                block_time_ms: 400,
                proof_type: ProofType::SolanaProof,
                challenge_period_seconds: 0,
                max_reorg_depth: 1,
            },
            ExternalChainId::X3Native => FinalityConfig {
                chain,
                confirmations_required: 1,
                block_time_ms: 6_000, // 6 second blocks
                proof_type: ProofType::LightClient,
                challenge_period_seconds: 0,
                max_reorg_depth: 0, // GRANDPA finality
            },
            ExternalChainId::EvmChain(chain_id) => {
                // Default config for unknown EVM chains
                FinalityConfig {
                    chain: ExternalChainId::EvmChain(chain_id),
                    confirmations_required: 20, // Conservative
                    block_time_ms: 12_000,
                    proof_type: ProofType::MerkleTrie,
                    challenge_period_seconds: 0,
                    max_reorg_depth: 20,
                }
            }
        }
    }

    /// Calculate finality score (0-100)
    ///
    /// Score indicates confidence that transaction won't be reverted
    pub fn finality_score(config: &FinalityConfig, confirmations: u32) -> u32 {
        if confirmations >= config.confirmations_required {
            100
        } else {
            // Linear scale up to required confirmations.
            // checked_div guards against a zero confirmations_required value reaching
            // this branch (should never happen after the update_finality_config guard,
            // but defense-in-depth avoids a runtime panic).
            (confirmations * 100)
                .checked_div(config.confirmations_required)
                .unwrap_or(100)
        }
    }

    /// Calculate reorg probability in basis points
    pub fn reorg_probability(config: &FinalityConfig, confirmations: u32) -> u32 {
        if confirmations >= config.confirmations_required {
            return 0;
        }

        // Exponential decay based on confirmations
        match config.chain {
            ExternalChainId::Bitcoin | ExternalChainId::BitcoinTestnet => {
                match confirmations {
                    0 => 10000, // 100%
                    1 => 2500,  // 25%
                    2 => 500,   // 5%
                    3 => 100,   // 1%
                    4 => 50,    // 0.5%
                    5 => 10,    // 0.1%
                    _ => 1,     // 0.01%
                }
            }
            ExternalChainId::Avalanche | ExternalChainId::X3Native => 0, // Instant finality
            _ => {
                // Generic EVM formula
                if confirmations == 0 {
                    5000 // 50%
                } else {
                    5000 / (2u32.pow(confirmations))
                }
            }
        }
    }

    /// Estimate time until finality (in seconds)
    pub fn time_to_finality(config: &FinalityConfig, current_confirmations: u32) -> u64 {
        if current_confirmations >= config.confirmations_required {
            return 0;
        }

        let remaining = config.confirmations_required - current_confirmations;
        (remaining as u64 * config.block_time_ms) / 1000
    }

    /// Check if we should wait for more confirmations
    pub fn should_wait(config: &FinalityConfig, confirmations: u32, urgency: Urgency) -> bool {
        match urgency {
            Urgency::Immediate => confirmations < 1,
            Urgency::Normal => confirmations < config.confirmations_required,
            Urgency::Conservative => {
                confirmations < config.confirmations_required.saturating_mul(2)
            }
        }
    }
}

/// Settlement urgency level
#[derive(Clone, Copy, Encode, Decode, DecodeWithMemTracking, Debug, TypeInfo, PartialEq, Eq)]
pub enum Urgency {
    /// Accept minimal confirmations (risky)
    Immediate,
    /// Wait for standard confirmations
    Normal,
    /// Wait for extra confirmations (safe)
    Conservative,
}

/// Light client header for X3 verification
#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, TypeInfo)]
pub struct LightClientHeader {
    /// Chain this header is from
    pub chain: ExternalChainId,
    /// Block number
    pub number: u64,
    /// Block hash
    pub hash: H256,
    /// Parent hash
    pub parent_hash: H256,
    /// State root
    pub state_root: H256,
    /// Timestamp
    pub timestamp: u64,
    /// Validator signatures (if applicable)
    pub signatures: Vec<[u8; 65]>,
}

/// Reorg detector
#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, TypeInfo)]
pub struct ReorgDetector {
    /// Chain being monitored
    pub chain: ExternalChainId,
    /// Recent block hashes (circular buffer)
    pub recent_hashes: Vec<(u64, H256)>,
    /// Maximum depth to track
    pub max_depth: u32,
}

impl ReorgDetector {
    pub fn new(chain: ExternalChainId, max_depth: u32) -> Self {
        Self {
            chain,
            recent_hashes: Vec::with_capacity(max_depth as usize),
            max_depth,
        }
    }

    /// Record a new block
    pub fn record_block(&mut self, height: u64, hash: H256) {
        // Check for reorg
        if let Some(existing) = self.recent_hashes.iter().find(|(h, _)| *h == height) {
            if existing.1 != hash {
                // Reorg detected! This height has a different hash
                log::warn!(
                    "Reorg detected on {:?} at height {}: {} -> {}",
                    self.chain,
                    height,
                    existing.1,
                    hash
                );
            }
        }

        // Add/update block
        self.recent_hashes.retain(|(h, _)| *h != height);
        self.recent_hashes.push((height, hash));

        // Trim old blocks
        if self.recent_hashes.len() > self.max_depth as usize {
            self.recent_hashes.sort_by_key(|(h, _)| *h);
            self.recent_hashes.remove(0);
        }
    }

    /// Check if a block at given height is stable
    pub fn is_stable(&self, height: u64, current_height: u64) -> bool {
        current_height.saturating_sub(height) >= self.max_depth as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_btc_finality_config() {
        let config = FinalityOracle::default_config(ExternalChainId::Bitcoin);
        assert_eq!(config.confirmations_required, 6);
        assert_eq!(config.block_time_ms, 600_000);
    }

    #[test]
    fn test_finality_score() {
        let config = FinalityOracle::default_config(ExternalChainId::Bitcoin);

        assert_eq!(FinalityOracle::finality_score(&config, 0), 0);
        assert_eq!(FinalityOracle::finality_score(&config, 3), 50);
        assert_eq!(FinalityOracle::finality_score(&config, 6), 100);
        assert_eq!(FinalityOracle::finality_score(&config, 10), 100);
    }

    #[test]
    fn test_reorg_probability() {
        let config = FinalityOracle::default_config(ExternalChainId::Bitcoin);

        assert_eq!(FinalityOracle::reorg_probability(&config, 0), 10000);
        assert_eq!(FinalityOracle::reorg_probability(&config, 1), 2500);
        assert_eq!(FinalityOracle::reorg_probability(&config, 6), 0);
    }

    #[test]
    fn test_time_to_finality() {
        let config = FinalityOracle::default_config(ExternalChainId::Bitcoin);

        // 0 confirmations -> need 6 more -> 60 minutes
        assert_eq!(FinalityOracle::time_to_finality(&config, 0), 3600);

        // 3 confirmations -> need 3 more -> 30 minutes
        assert_eq!(FinalityOracle::time_to_finality(&config, 3), 1800);

        // Already final
        assert_eq!(FinalityOracle::time_to_finality(&config, 6), 0);
    }

    #[test]
    fn test_reorg_detector() {
        let mut detector = ReorgDetector::new(ExternalChainId::Bitcoin, 10);

        // Record blocks
        detector.record_block(100, H256::repeat_byte(0x01));
        detector.record_block(101, H256::repeat_byte(0x02));
        detector.record_block(102, H256::repeat_byte(0x03));

        assert!(!detector.is_stable(100, 105));
        assert!(detector.is_stable(100, 111));
    }
}
