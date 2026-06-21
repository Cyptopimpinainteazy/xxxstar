//! # RpcQuorum — Multi-provider RPC quorum consensus for atomic swaps.
//!
//! Provides an oracle trait and simple implementation for collecting and
//! verifying RPC provider votes on transaction status. Works alongside the
//! ledger's per-provider [`RpcQuorumProof`] type.
//!
//! ## Design
//!
//! - [`RpcProvider`] describes an RPC endpoint identity.
//! - [`RpcVote`] captures a single provider's observation.
//! - [`RpcQuorumOracle`] trait defines the quorum-check contract.
//! - [`SimpleRpcQuorum`] provides a basic in-memory implementation.
//!
//! The per-provider attestation [`RpcQuorumProof`] defined in the ledger module
//! is used directly — this module does **not** redefine it.

use crate::error::SwapError;
use crate::intent::ChainKind;
use crate::ledger::{RpcQuorumProof, TxStatus};
use alloc::string::String;
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// RPC provider identity
// ---------------------------------------------------------------------------

/// Identity and connection details for an RPC provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcProvider {
    /// Unique identifier for this provider (e.g. "alchemy-eth-mainnet").
    pub provider_id: String,
    /// RPC endpoint URL.
    pub url: String,
    /// Chain this provider serves.
    pub chain: ChainKind,
}

impl RpcProvider {
    /// Create a new RPC provider descriptor.
    pub fn new(provider_id: impl Into<String>, url: impl Into<String>, chain: ChainKind) -> Self {
        Self {
            provider_id: provider_id.into(),
            url: url.into(),
            chain,
        }
    }
}

// ---------------------------------------------------------------------------
// RPC vote
// ---------------------------------------------------------------------------

/// A single vote cast by one RPC provider about a transaction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcVote {
    /// Which provider cast this vote.
    pub provider_id: String,
    /// Block height at which the vote was observed.
    pub block_height: u64,
    /// Transaction status reported by this provider.
    pub tx_status: TxStatus,
    /// Whether this provider agrees with the target consensus.
    pub agreement: bool,
}

// ---------------------------------------------------------------------------
// Consensus result
// ---------------------------------------------------------------------------

/// Outcome of a quorum consensus verification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConsensusResult {
    /// Enough providers agreed on the transaction status.
    ConsensusAchieved {
        /// Number of providers in agreement.
        agreement: u32,
        /// Minimum required for quorum.
        required: u32,
    },
    /// Not enough providers agreed; disagreements are listed.
    ConsensusNotAchieved {
        /// Number of providers in agreement.
        agreement: u32,
        /// Minimum required for quorum.
        required: u32,
        /// Provider IDs that disagreed with the consensus status.
        disagreements: Vec<String>,
    },
}

// ---------------------------------------------------------------------------
// Quorum oracle trait
// ---------------------------------------------------------------------------

/// Oracle trait for multi-provider RPC quorum verification.
///
/// Each method works with the ledger's per-provider [`RpcQuorumProof`]. The
/// aggregate consensus logic (counting agreements, listing disagreements) is
/// the responsibility of this trait.
pub trait RpcQuorumOracle {
    /// Check whether a single provider's proof meets the quorum threshold.
    ///
    /// Returns `true` when `proof.agreement_count >= proof.required_quorum`.
    fn check_quorum(&self, proof: &RpcQuorumProof) -> Result<bool, SwapError> {
        Ok(proof.agreed())
    }

    /// Collect votes from a set of providers for a given transaction hash.
    ///
    /// In a real implementation this would dispatch RPC calls; the returned
    /// [`RpcQuorumProof`] per provider captures the observed status and the
    /// agreement metadata.
    fn collect_votes(
        &self,
        providers: &[RpcProvider],
        tx_hash: &str,
        intent_id: u64,
    ) -> Vec<RpcQuorumProof>;

    /// Verify that a collection of proofs reaches consensus.
    ///
    /// Determines whether enough providers agree on the majority status. When
    /// quorum is not met, the IDs of disagreeing providers are returned.
    fn verify_consensus(
        &self,
        proofs: &[RpcQuorumProof],
        required_quorum: u32,
    ) -> Result<ConsensusResult, SwapError>;
}

// ---------------------------------------------------------------------------
// Simple (in-memory) implementation
// ---------------------------------------------------------------------------

/// A basic in-memory [`RpcQuorumOracle`] implementation.
///
/// This is suitable for testing and single-node usage. Production deployments
/// should replace this with a network-aware oracle that actually calls RPC
/// endpoints.
#[derive(Debug, Clone, Default)]
pub struct SimpleRpcQuorum;

impl SimpleRpcQuorum {
    /// Create a new [`SimpleRpcQuorum`] with default settings.
    pub fn new() -> Self {
        Self
    }
}

impl RpcQuorumOracle for SimpleRpcQuorum {
    fn check_quorum(&self, proof: &RpcQuorumProof) -> Result<bool, SwapError> {
        Ok(proof.agreed())
    }

    fn collect_votes(
        &self,
        providers: &[RpcProvider],
        tx_hash: &str,
        intent_id: u64,
    ) -> Vec<RpcQuorumProof> {
        providers
            .iter()
            .map(|provider| {
                // Simulate a successful Confirmed response at block 1.
                // Real implementations would call the provider's RPC.
                let _ = tx_hash;
                RpcQuorumProof {
                    intent_id,
                    provider: provider.provider_id.clone(),
                    block_height: 1,
                    tx_status: TxStatus::Confirmed,
                    agreement_count: 1,
                    required_quorum: 1,
                }
            })
            .collect()
    }

    fn verify_consensus(
        &self,
        proofs: &[RpcQuorumProof],
        required_quorum: u32,
    ) -> Result<ConsensusResult, SwapError> {
        if proofs.is_empty() {
            return Ok(ConsensusResult::ConsensusNotAchieved {
                agreement: 0,
                required: required_quorum,
                disagreements: Vec::new(),
            });
        }

        // Count providers whose agreement_count >= required_quorum.
        // This simulates the real logic: a provider "agrees" when their
        // observed status has sufficient corroboration.
        let mut agreement: u32 = 0;
        let mut disagreements: Vec<String> = Vec::new();

        for proof in proofs {
            if proof.agreed() {
                agreement += 1;
            } else {
                disagreements.push(proof.provider.clone());
            }
        }

        if agreement >= required_quorum {
            Ok(ConsensusResult::ConsensusAchieved {
                agreement,
                required: required_quorum,
            })
        } else {
            Ok(ConsensusResult::ConsensusNotAchieved {
                agreement,
                required: required_quorum,
                disagreements,
            })
        }
    }
}

// ---------------------------------------------------------------------------
// Helper: build an aggregate RpcQuorumProof from the ledger's per-provider one
// ---------------------------------------------------------------------------

/// Aggregate multiple per-provider [`RpcQuorumProof`]s into a consolidated
/// result. This is used in the scoreboard and relayer to record a single
/// quorum-check proof entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsolidatedQuorum {
    /// The individual provider proofs.
    pub proofs: Vec<RpcQuorumProof>,
    /// Block height at which the check was performed (max across providers).
    pub block_height: u64,
    /// Whether consensus was reached.
    pub consensus_reached: bool,
    /// Number of providers that agreed.
    pub agreement_count: u32,
    /// Minimum providers required for quorum.
    pub required_quorum: u32,
    /// Transaction hash that was checked.
    pub tx_hash: String,
    /// Chain on which the transaction lives.
    pub chain: ChainKind,
    /// Provider IDs that disagreed (empty when consensus is reached).
    pub disagreements: Vec<String>,
}

impl ConsolidatedQuorum {
    /// Build a [`ConsolidatedQuorum`] from provider proofs and verification
    /// metadata.
    pub fn new(
        proofs: Vec<RpcQuorumProof>,
        required_quorum: u32,
        tx_hash: impl Into<String>,
        chain: ChainKind,
        oracle: &dyn RpcQuorumOracle,
    ) -> Result<Self, SwapError> {
        let block_height = proofs.iter().map(|p| p.block_height).max().unwrap_or(0);

        let consensus = oracle.verify_consensus(&proofs, required_quorum)?;

        let (agreement_count, disagreements) = match &consensus {
            ConsensusResult::ConsensusAchieved { agreement, .. } => (*agreement, Vec::new()),
            ConsensusResult::ConsensusNotAchieved {
                agreement,
                disagreements,
                ..
            } => (*agreement, disagreements.clone()),
        };

        let consensus_reached = matches!(consensus, ConsensusResult::ConsensusAchieved { .. });

        Ok(Self {
            proofs,
            block_height,
            consensus_reached,
            agreement_count,
            required_quorum,
            tx_hash: tx_hash.into(),
            chain,
            disagreements,
        })
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ledger::TxStatus;

    fn make_proof(provider: &str, agreement_count: u32, required_quorum: u32) -> RpcQuorumProof {
        RpcQuorumProof {
            intent_id: 0,
            provider: provider.into(),
            block_height: 100,
            tx_status: TxStatus::Confirmed,
            agreement_count,
            required_quorum,
        }
    }

    #[test]
    fn test_quorum_3_of_3_consensus_achieved() {
        let oracle = SimpleRpcQuorum::new();

        let proofs = vec![
            make_proof("provider-a", 3, 3),
            make_proof("provider-b", 3, 3),
            make_proof("provider-c", 3, 3),
        ];

        let result = oracle.verify_consensus(&proofs, 3).unwrap();

        assert_eq!(
            result,
            ConsensusResult::ConsensusAchieved {
                agreement: 3,
                required: 3,
            }
        );
    }

    #[test]
    fn test_quorum_2_of_3_required_3_not_achieved() {
        let oracle = SimpleRpcQuorum::new();

        let proofs = vec![
            make_proof("provider-a", 1, 3), // not agreed
            make_proof("provider-b", 3, 3), // agreed
            make_proof("provider-c", 3, 3), // agreed
        ];

        let result = oracle.verify_consensus(&proofs, 3).unwrap();

        assert_eq!(
            result,
            ConsensusResult::ConsensusNotAchieved {
                agreement: 2,
                required: 3,
                disagreements: vec!["provider-a".into()],
            }
        );
    }

    #[test]
    fn test_quorum_3_of_5_required_3_consensus_achieved() {
        let oracle = SimpleRpcQuorum::new();

        let proofs = vec![
            make_proof("provider-a", 3, 3), // agreed
            make_proof("provider-b", 1, 3), // not agreed
            make_proof("provider-c", 3, 3), // agreed
            make_proof("provider-d", 2, 3), // not agreed
            make_proof("provider-e", 3, 3), // agreed
        ];

        let result = oracle.verify_consensus(&proofs, 3).unwrap();

        assert_eq!(
            result,
            ConsensusResult::ConsensusAchieved {
                agreement: 3,
                required: 3,
            }
        );
    }

    #[test]
    fn test_quorum_zero_proofs_fails() {
        let oracle = SimpleRpcQuorum::new();

        let proofs: Vec<RpcQuorumProof> = vec![];

        let result = oracle.verify_consensus(&proofs, 3).unwrap();

        assert_eq!(
            result,
            ConsensusResult::ConsensusNotAchieved {
                agreement: 0,
                required: 3,
                disagreements: vec![],
            }
        );
    }

    #[test]
    fn test_collect_votes_returns_proof_per_provider() {
        let oracle = SimpleRpcQuorum::new();

        let providers = vec![
            RpcProvider::new("alchemy-eth", "https://eth.alchemy.io", ChainKind::Ethereum),
            RpcProvider::new("infura-eth", "https://eth.infura.io", ChainKind::Ethereum),
            RpcProvider::new(
                "quicknode-eth",
                "https://eth.quicknode.io",
                ChainKind::Ethereum,
            ),
        ];

        let proofs = oracle.collect_votes(&providers, "0xdeadbeef", 0);

        assert_eq!(proofs.len(), 3);
        for proof in &proofs {
            assert_eq!(proof.block_height, 1);
            assert_eq!(proof.tx_status, TxStatus::Confirmed);
        }
    }

    #[test]
    fn test_check_quorum_uses_ledger_agreed() {
        let oracle = SimpleRpcQuorum::new();

        let agreed = make_proof("provider-a", 3, 2);
        assert!(oracle.check_quorum(&agreed).unwrap());

        let not_agreed = make_proof("provider-a", 1, 2);
        assert!(!oracle.check_quorum(&not_agreed).unwrap());
    }

    // ------------------------------------------------------------------
    // Edge case: empty provider list in collect_votes
    // ------------------------------------------------------------------
    #[test]
    fn test_empty_provider_list() {
        let oracle = SimpleRpcQuorum::new();

        let providers: Vec<RpcProvider> = vec![];
        let proofs = oracle.collect_votes(&providers, "0xtx", 0);
        assert!(
            proofs.is_empty(),
            "empty provider list must yield empty proofs"
        );
    }

    // ------------------------------------------------------------------
    // Edge case: all votes disagree
    // ------------------------------------------------------------------
    #[test]
    fn test_all_votes_disagree() {
        let oracle = SimpleRpcQuorum::new();

        let proofs = vec![
            make_proof("provider-a", 1, 3), // not agreed
            make_proof("provider-b", 1, 3), // not agreed
            make_proof("provider-c", 1, 3), // not agreed
        ];

        let result = oracle.verify_consensus(&proofs, 3).unwrap();
        assert_eq!(
            result,
            ConsensusResult::ConsensusNotAchieved {
                agreement: 0,
                required: 3,
                disagreements: vec![
                    "provider-a".into(),
                    "provider-b".into(),
                    "provider-c".into()
                ],
            }
        );
    }

    // ------------------------------------------------------------------
    // Edge case: quorum boundary — exact threshold (2 of 3 with required=2)
    // ------------------------------------------------------------------
    #[test]
    fn test_quorum_boundary_exact_threshold() {
        let oracle = SimpleRpcQuorum::new();

        // 2 agreed, 1 not agreed, required=2 → consensus achieved at exact boundary.
        let proofs = vec![
            make_proof("provider-a", 3, 2), // agreed (3 >= 2)
            make_proof("provider-b", 3, 2), // agreed (3 >= 2)
            make_proof("provider-c", 1, 2), // not agreed
        ];

        let result = oracle.verify_consensus(&proofs, 2).unwrap();
        assert_eq!(
            result,
            ConsensusResult::ConsensusAchieved {
                agreement: 2,
                required: 2,
            }
        );

        // 1 agreed, 2 not agreed, required=2 → not achieved (1 < 2).
        let proofs = vec![
            make_proof("provider-a", 3, 2), // agreed
            make_proof("provider-b", 1, 2), // not agreed
            make_proof("provider-c", 1, 2), // not agreed
        ];

        let result = oracle.verify_consensus(&proofs, 2).unwrap();
        assert_eq!(
            result,
            ConsensusResult::ConsensusNotAchieved {
                agreement: 1,
                required: 2,
                disagreements: vec!["provider-b".into(), "provider-c".into()],
            }
        );
    }

    // ------------------------------------------------------------------
    // Edge case: single provider with quorum requirement 1
    // ------------------------------------------------------------------
    #[test]
    fn test_single_provider_quorum_one() {
        let oracle = SimpleRpcQuorum::new();

        let proofs = vec![make_proof("solo-provider", 1, 1)];
        let result = oracle.verify_consensus(&proofs, 1).unwrap();
        assert_eq!(
            result,
            ConsensusResult::ConsensusAchieved {
                agreement: 1,
                required: 1,
            }
        );
    }
}
