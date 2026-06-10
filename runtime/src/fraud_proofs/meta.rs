// runtime/src/fraud_proofs/meta.rs
//
// Disputed-block metadata helper — the integration point between the fraud-proof
// pallet and the runtime's block production / sequencer layers.
//
// ## Why this is a separate module
// The `load_disputed_block_meta()` function needs access to FRAME types
// (`frame_system::BlockHash`, `frame_system::BlockNumberProvider`) which are
// not available in the no_std `types.rs` module.  By placing it here, we keep
// the core types pure (no FRAME dependency) while still providing a convenient
// runtime helper.
//
// ## Usage
// ```ignore
// use crate::fraud_proofs::meta::load_disputed_block_meta;
// use crate::fraud_proofs::types::{NoSchedulerCommitment, NoProposer};
//
// let meta = load_disputed_block_meta::<
//     AccountId,
//     frame_system::Pallet<Runtime>,
//     SequencerPallet,   // implements SchedulerCommitmentQuery
//     AuraPallet,        // implements ProposerQuery<AccountId>
// >(disputed_block_number);
// ```

use crate::fraud_proofs::types::{
    DisputedBlockMeta, ProposerQuery, SchedulerCommitmentQuery,
};
use frame_support::traits::Get;
use sp_core::H256;

/// Construct a `DisputedBlockMeta` from on-chain data sources.
///
/// This is the canonical integration point between the fraud-proof pallet and
/// the runtime's block production / sequencer layers.  It reads:
///
/// - `block_hash` from the block header (via `frame_system::BlockHash`)
/// - `scheduler_commitment` from the sequencer's batch commitment for the block
/// - `rules_version` from the runtime's current scheduler rules version
/// - `proposer` from the block author (via Aura / Babe / authority lookup)
///
/// # Type parameters
/// - `BlockNumberProvider`: something that yields the current block number
///   (typically `frame_system::Pallet<T>`)
/// - `SchedulerCommitmentProvider`: something that yields the scheduler
///   commitment for a given block number (typically the sequencer pallet)
/// - `ProposerProvider`: something that yields the block author for a given
///   block number (typically the Aura / Babe pallet)
///
/// # Returns
/// `Some(DisputedBlockMeta)` if all data is available, `None` if the block
/// is unknown or the scheduler commitment is not yet finalized.
pub fn load_disputed_block_meta<AccountId, BlockNumberProvider, SchedulerCommitmentProvider, ProposerProvider>(
    block_number: u32,
) -> Option<DisputedBlockMeta<AccountId>>
where
    BlockNumberProvider: frame_system::BlockNumberProvider<BlockNumber = u32>,
    SchedulerCommitmentProvider: SchedulerCommitmentQuery,
    ProposerProvider: ProposerQuery<AccountId>,
{
    // 1. Get block hash from the disputed block number
    let block_hash = frame_system::BlockHash::<BlockNumberProvider>::get(block_number)?;

    // 2. Get the scheduler commitment for this block
    let scheduler_commitment = SchedulerCommitmentProvider::get_scheduler_commitment(block_number)?;

    // 3. Get the current rules version
    let rules_version = crate::fraud_proofs::witness_v1::WITNESS_VERSION as u32;

    // 4. Get the block proposer
    let proposer = ProposerProvider::get_proposer(block_number)?;

    Some(DisputedBlockMeta {
        block_hash,
        block_number,
        rules_version,
        scheduler_commitment,
        proposer,
    })
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fraud_proofs::types::{NoProposer, NoSchedulerCommitment};
    use sp_core::H256;

    /// A mock `SchedulerCommitmentQuery` that returns a fixed commitment.
    struct MockSchedulerCommitment;

    impl SchedulerCommitmentQuery for MockSchedulerCommitment {
        fn get_scheduler_commitment(block_number: u32) -> Option<H256> {
            if block_number == 0 || block_number > 100 {
                return None;
            }
            let mut h = [0u8; 32];
            h[..4].copy_from_slice(&block_number.to_le_bytes());
            Some(H256(h))
        }
    }

    /// A mock `ProposerQuery` that returns a fixed proposer.
    struct MockProposer;

    impl ProposerQuery<u64> for MockProposer {
        fn get_proposer(block_number: u32) -> Option<u64> {
            if block_number == 0 || block_number > 100 {
                return None;
            }
            Some(42u64)
        }
    }

    /// A mock `BlockNumberProvider` that returns a fixed block number.
    struct MockBlockNumberProvider;

    impl frame_system::BlockNumberProvider for MockBlockNumberProvider {
        type BlockNumber = u32;

        fn block_number() -> Self::BlockNumber {
            42
        }

        fn can_assume_height(_h: Self::BlockNumber) -> bool {
            true
        }
    }

    /// LOAD-META-001: SchedulerCommitmentQuery returns None for unknown blocks
    #[test]
    fn scheduler_commitment_none_for_unknown_block() {
        assert_eq!(
            NoSchedulerCommitment::get_scheduler_commitment(999),
            None
        );
    }

    /// LOAD-META-002: ProposerQuery returns None for unknown blocks
    #[test]
    fn proposer_none_for_unknown_block() {
        assert_eq!(NoProposer::get_proposer(999), None::<u64>);
    }

    /// LOAD-META-003: MockSchedulerCommitment returns deterministic values
    #[test]
    fn mock_scheduler_commitment_deterministic() {
        let c1 = MockSchedulerCommitment::get_scheduler_commitment(5);
        let c2 = MockSchedulerCommitment::get_scheduler_commitment(5);
        assert_eq!(c1, c2);
        assert!(c1.is_some());
    }

    /// LOAD-META-004: MockProposer returns deterministic values
    #[test]
    fn mock_proposer_deterministic() {
        let p1 = MockProposer::get_proposer(10);
        let p2 = MockProposer::get_proposer(10);
        assert_eq!(p1, p2);
        assert_eq!(p1, Some(42u64));
    }
}
