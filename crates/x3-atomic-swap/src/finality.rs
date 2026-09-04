//! # FinalityOracle — Chain-specific finality verification for atomic swaps.
//!
//! Provides a trait-based finality oracle that maps chains to their required
//! finality parameters and verifies whether on-chain data meets those thresholds.
//!
//! ## Mapping
//!
//! | Spec Chain | ChainKind variant |
//! |------------|-------------------|
//! | EVM        | `ChainKind::Ethereum` |
//! | Solana     | `ChainKind::Solana` |
//! | Bitcoin    | `ChainKind::Bitcoin` |
//! | Substrate  | `ChainKind::X3` (X3 runtime is Substrate-based) |
//! | Cosmos     | `ChainKind::Cosmos` |

use crate::error::SwapError;
use crate::intent::ChainKind;

/// Chain-specific finality configuration.
#[derive(Debug, Clone)]
pub struct FinalityConfig {
    /// The chain this config applies to.
    pub chain: ChainKind,
    /// Number of block confirmations required (EVM, Bitcoin, and general PoW chains).
    pub confirmations: u32,
    /// Solana commitment level (e.g. "finalized", "confirmed"). Ignored for other chains.
    pub commitment_level: String,
    /// Number of GRANDPA rounds to wait (Substrate placeholder).
    pub grandpa_rounds: u32,
    /// Number of Tendermint blocks to wait (Cosmos placeholder).
    pub tendermint_blocks: u32,
}

impl FinalityConfig {
    /// Create an EVM (Ethereum) finality config with the default 12 confirmations.
    pub fn evm() -> Self {
        Self {
            chain: ChainKind::Ethereum,
            confirmations: 12,
            commitment_level: String::new(),
            grandpa_rounds: 0,
            tendermint_blocks: 0,
        }
    }

    /// Create a Solana finality config with the given commitment level.
    pub fn solana(commitment_level: &str) -> Self {
        Self {
            chain: ChainKind::Solana,
            confirmations: 0,
            commitment_level: commitment_level.to_string(),
            grandpa_rounds: 0,
            tendermint_blocks: 0,
        }
    }

    /// Create a Bitcoin finality config with the default 6 confirmations.
    pub fn bitcoin() -> Self {
        Self {
            chain: ChainKind::Bitcoin,
            confirmations: 6,
            commitment_level: String::new(),
            grandpa_rounds: 0,
            tendermint_blocks: 0,
        }
    }

    /// Create a Substrate (X3) finality config (placeholder).
    pub fn substrate() -> Self {
        Self {
            chain: ChainKind::X3,
            confirmations: 0,
            commitment_level: String::new(),
            grandpa_rounds: 1,
            tendermint_blocks: 0,
        }
    }

    /// Create a Cosmos finality config (placeholder).
    pub fn cosmos() -> Self {
        Self {
            chain: ChainKind::Cosmos,
            confirmations: 0,
            commitment_level: String::new(),
            grandpa_rounds: 0,
            tendermint_blocks: 1,
        }
    }
}

/// Data payload for a finality check query.
#[derive(Debug, Clone)]
pub struct FinalityCheckData {
    /// The chain the transaction was submitted on.
    pub chain: ChainKind,
    /// Current block height (for informational purposes).
    pub block_height: u64,
    /// Number of confirmations observed so far.
    pub confirmations: u32,
    /// Commitment level reported by the node (Solana-specific).
    pub commitment_level: String,
}

/// Trait for verifying chain-specific finality of cross-chain swap transactions.
pub trait FinalityOracle {
    /// Return the required finality configuration for a given chain.
    fn required_finality(&self, chain: ChainKind) -> FinalityConfig;

    /// Verify that a transaction has reached the required number of confirmations
    /// or commitment level on the given chain.
    ///
    /// Returns `Ok(true)` if finality is met, `Ok(false)` if not yet met,
    /// or `Err(SwapError::FinalityNotMet)` on a definitive failure.
    fn verify_finality(
        &self,
        chain: ChainKind,
        current_confirms: u32,
        commitment: &str,
    ) -> Result<bool, SwapError> {
        let config = self.required_finality(chain);
        match chain {
            ChainKind::Ethereum
            | ChainKind::Base
            | ChainKind::Arbitrum
            | ChainKind::Optimism
            | ChainKind::Bsc
            | ChainKind::Polygon
            | ChainKind::Avalanche
            | ChainKind::Bitcoin => {
                if current_confirms >= config.confirmations {
                    Ok(true)
                } else {
                    Err(SwapError::FinalityNotMet {
                        chain: chain.as_str().to_string(),
                        required: config.confirmations,
                        current: current_confirms,
                    })
                }
            }
            ChainKind::Solana => {
                if commitment == "finalized"
                    || (commitment == "confirmed" && config.commitment_level == "confirmed")
                {
                    Ok(true)
                } else {
                    Err(SwapError::FinalityNotMet {
                        chain: chain.as_str().to_string(),
                        required: 0,
                        current: current_confirms,
                    })
                }
            }
            ChainKind::X3 => {
                if current_confirms >= config.grandpa_rounds {
                    Ok(true)
                } else {
                    Err(SwapError::FinalityNotMet {
                        chain: chain.as_str().to_string(),
                        required: config.grandpa_rounds,
                        current: current_confirms,
                    })
                }
            }
            ChainKind::Cosmos => {
                if current_confirms >= config.tendermint_blocks {
                    Ok(true)
                } else {
                    Err(SwapError::FinalityNotMet {
                        chain: chain.as_str().to_string(),
                        required: config.tendermint_blocks,
                        current: current_confirms,
                    })
                }
            }
        }
    }

    /// Higher-level check: query whether the transaction data indicates finality.
    fn is_finalized(
        &self,
        chain: ChainKind,
        tx_data: &FinalityCheckData,
    ) -> Result<bool, SwapError> {
        self.verify_finality(chain, tx_data.confirmations, &tx_data.commitment_level)
    }
}

/// A simple in-memory finality oracle with default chain configurations.
///
/// Uses `ChainKind::default_safe_confirmations()` from intent.rs for EVM-like chains,
/// "finalized" for Solana, and placeholder stubs for Substrate/Cosmos.
#[derive(Debug, Clone, Default)]
pub struct InMemoryFinalityOracle;

impl InMemoryFinalityOracle {
    /// Create a new oracle with all-default finality configs.
    pub fn new() -> Self {
        Self
    }
}

impl FinalityOracle for InMemoryFinalityOracle {
    fn required_finality(&self, chain: ChainKind) -> FinalityConfig {
        match chain {
            ChainKind::Ethereum
            | ChainKind::Base
            | ChainKind::Arbitrum
            | ChainKind::Optimism
            | ChainKind::Bsc
            | ChainKind::Polygon
            | ChainKind::Avalanche => FinalityConfig {
                chain,
                confirmations: chain.default_safe_confirmations(),
                commitment_level: String::new(),
                grandpa_rounds: 0,
                tendermint_blocks: 0,
            },
            ChainKind::Bitcoin => FinalityConfig::bitcoin(),
            ChainKind::Solana => FinalityConfig::solana("finalized"),
            ChainKind::X3 => FinalityConfig::substrate(),
            ChainKind::Cosmos => FinalityConfig::cosmos(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// EVM finality: 12 confirmations required, 10 current → false, 12 current → true.
    #[test]
    fn test_evm_finality() {
        let oracle = InMemoryFinalityOracle::new();

        // 10 confirmations – should fail.
        let result = oracle.verify_finality(ChainKind::Ethereum, 10, "");
        assert!(result.is_err(), "expected FinalityNotMet at 10 confirms");
        if let Err(SwapError::FinalityNotMet {
            required, current, ..
        }) = &result
        {
            assert_eq!(*required, 12);
            assert_eq!(*current, 10);
        } else {
            panic!("unexpected error variant");
        }

        // 12 confirmations – should pass.
        let result = oracle.verify_finality(ChainKind::Ethereum, 12, "");
        assert_eq!(result, Ok(true), "expected finality met at 12 confirms");
    }

    /// Solana: "finalized" commitment passes.
    #[test]
    fn test_solana_finalized_passes() {
        let oracle = InMemoryFinalityOracle::new();

        let result = oracle.verify_finality(ChainKind::Solana, 0, "finalized");
        assert_eq!(result, Ok(true), "finalized commitment should pass");

        let result = oracle.verify_finality(ChainKind::Solana, 0, "confirmed");
        assert!(
            result.is_err(),
            "confirmed should fail when oracle requires finalized"
        );
    }

    /// Substrate (X3) finality: requires grandpa_rounds confirmations.
    #[test]
    fn test_substrate_finality() {
        let oracle = InMemoryFinalityOracle::new();

        let config = oracle.required_finality(ChainKind::X3);
        assert_eq!(
            config.grandpa_rounds, 1,
            "X3 should require 1 grandpa round"
        );

        // Zero confirmations — should fail.
        let result = oracle.verify_finality(ChainKind::X3, 0, "");
        assert!(result.is_err(), "X3 must fail with 0 confirms");
        if let Err(SwapError::FinalityNotMet {
            required, current, ..
        }) = &result
        {
            assert_eq!(*required, 1);
            assert_eq!(*current, 0);
        } else {
            panic!("expected FinalityNotMet");
        }

        // Sufficient confirmations — should pass.
        let result = oracle.verify_finality(ChainKind::X3, 1, "");
        assert_eq!(result, Ok(true), "X3 should pass with 1 confirmation");
        let result = oracle.verify_finality(ChainKind::X3, 5, "");
        assert_eq!(result, Ok(true), "X3 should pass with >1 confirmation");

        // Also via is_finalized.
        let tx_data = FinalityCheckData {
            chain: ChainKind::X3,
            block_height: 42,
            confirmations: 1,
            commitment_level: String::new(),
        };
        let result = oracle.is_finalized(ChainKind::X3, &tx_data);
        assert_eq!(
            result,
            Ok(true),
            "is_finalized should pass with sufficient confirms"
        );
    }

    // ------------------------------------------------------------------
    // Edge case: zero confirmations fails for EVM chains
    // ------------------------------------------------------------------
    #[test]
    fn test_zero_confirmations_fails_evm() {
        let oracle = InMemoryFinalityOracle::new();

        // Ethereum requires 12 confirmations, zero should fail.
        let result = oracle.verify_finality(ChainKind::Ethereum, 0, "");
        assert!(result.is_err(), "zero confirmations must fail for EVM");
        if let Err(SwapError::FinalityNotMet {
            required, current, ..
        }) = &result
        {
            assert_eq!(*required, 12);
            assert_eq!(*current, 0);
        } else {
            panic!("expected FinalityNotMet");
        }

        // Bitcoin requires 6 confirmations, zero should fail.
        let result = oracle.verify_finality(ChainKind::Bitcoin, 0, "");
        assert!(result.is_err(), "zero confirmations must fail for Bitcoin");
    }

    // ------------------------------------------------------------------
    // Edge case: mismatched chain kind — query Bitcoin with Solana commitment
    // ------------------------------------------------------------------
    #[test]
    fn test_mismatched_chain_kind() {
        let oracle = InMemoryFinalityOracle::new();

        // Bitcoin requires 6 confirmations, zero should fail even if commitment is "finalized".
        let result = oracle.verify_finality(ChainKind::Bitcoin, 0, "finalized");
        assert!(
            result.is_err(),
            "Bitcoin must fail with 0 confirms regardless of commitment"
        );

        // Solana is commitment-based, confirmations are ignored.
        let result = oracle.verify_finality(ChainKind::Solana, 0, "finalized");
        assert_eq!(
            result,
            Ok(true),
            "Solana should pass with finalized commitment"
        );
    }

    // ------------------------------------------------------------------
    // Edge case: via FinalityCheckData with is_finalized
    // ------------------------------------------------------------------
    #[test]
    fn test_is_finalized_with_varied_data() {
        let oracle = InMemoryFinalityOracle::new();

        // EVM with sufficient confirmations
        let tx_data = FinalityCheckData {
            chain: ChainKind::Ethereum,
            block_height: 1000,
            confirmations: 12,
            commitment_level: String::new(),
        };
        assert!(oracle.is_finalized(ChainKind::Ethereum, &tx_data).unwrap());

        // EVM with insufficient confirmations
        let tx_data = FinalityCheckData {
            chain: ChainKind::Ethereum,
            block_height: 1000,
            confirmations: 3,
            commitment_level: String::new(),
        };
        assert!(oracle.is_finalized(ChainKind::Ethereum, &tx_data).is_err());

        // Cosmos requires tendermint_blocks confirmations (1)
        let tx_data = FinalityCheckData {
            chain: ChainKind::Cosmos,
            block_height: 500,
            confirmations: 1,
            commitment_level: String::new(),
        };
        assert!(oracle.is_finalized(ChainKind::Cosmos, &tx_data).unwrap());

        // Cosmos with insufficient confirmations should fail
        let tx_data = FinalityCheckData {
            chain: ChainKind::Cosmos,
            block_height: 500,
            confirmations: 0,
            commitment_level: String::new(),
        };
        assert!(oracle.is_finalized(ChainKind::Cosmos, &tx_data).is_err());
    }
}
