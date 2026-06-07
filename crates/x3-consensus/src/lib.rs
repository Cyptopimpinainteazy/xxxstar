//! X3 Consensus Engine
//!
//! Consensus protocols, finality, proposal generation.

pub mod flash_finality_gossip;
pub mod parallel_proposer;
pub mod proof_of_history;
pub mod finality_proof_api;
pub mod network_partition_recovery;
pub mod ghost_fork_choice;
pub mod hotstuff;

pub use hotstuff::*;
pub use flash_finality_gossip::GossipBridge;
pub use parallel_proposer::{ParallelProposer, MempoolTx, AccessSet, Shard};
pub use proof_of_history::{PoHTick, PoHGenerator, PoHVerifier, PoHBlockProof};
pub use finality_proof_api::{FinalityCertificate, FinalityProofStore, FinalityProofRpc};
pub use network_partition_recovery::{PartitionDetector, ViewChangeOrchestrator};
pub use ghost_fork_choice::{GhostForkChoice, BlockWeight};
