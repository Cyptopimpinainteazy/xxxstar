//! Bundle legs executed by the chain itself, and the journal that lets a rollback undo them.
//!
//! A leg used to run off chain: the node's gateway executed it in memory and posted a state diff
//! back through an unsigned call, so the diff a rollback would apply was whatever its submitter
//! said, and no chain storage was ever written by a leg in the first place. Now the bundle's
//! executor calls `execute_bundle_leg`, the runtime's [`LegExecutor`] runs the leg against chain
//! state and returns the [`JournalEntry`]s for every write it made, and the pallet stores that
//! journal together with its hash in `ChainJournal`. A rollback applies a journal only when its hash
//! is the one the chain recorded, so no submitted bytes can choose what a rollback writes.
//!
//! Journal entries are inverse operations, not snapshots: a transfer is undone by moving the amount
//! back, a contract slot only when it still holds the value the leg wrote. A rollback therefore
//! never overwrites a change something else made after the leg; if one is in the way, the revert
//! fails and the pallet reports `IncompleteVmRevert`.

use crate::vm_revert::{RevertError, StateDiff};
use frame_support::pallet_prelude::*;
use parity_scale_codec::{Decode, DecodeWithMemTracking, Encode};
use sp_core::H256;
use sp_std::vec::Vec;

/// Largest X3 program a leg may carry.
pub const MAX_LEG_PROGRAM_BYTES: u32 = 64 * 1024;

/// Magic prefix of a chain journal inside a leg receipt's `state_diff`.
pub const JOURNAL_MAGIC: [u8; 8] = *b"x3jrnl\0\0";

/// What an executed leg does on chain.
#[derive(
    Clone,
    PartialEq,
    Eq,
    Encode,
    Decode,
    DecodeWithMemTracking,
    TypeInfo,
    RuntimeDebug,
    MaxEncodedLen,
)]
pub enum LegExecution<AccountId> {
    /// Run an X3BC program on the kernel's engine against the chain's contract slots; its slot
    /// writes persist.
    X3 {
        program: BoundedVec<u8, ConstU32<MAX_LEG_PROGRAM_BYTES>>,
    },
    /// Move `amount` of `CanonicalLedger` asset `asset` from the bundle's executor to `to`. Only
    /// the executor's own funds: the executor chooses the leg, so a leg cannot spend anyone else's.
    Transfer {
        asset: u32,
        to: AccountId,
        amount: u128,
    },
}

/// One write an executed leg made, as the operation that undoes it.
#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug)]
pub enum JournalEntry {
    /// Undone by moving `amount` of `asset` back from `to` to `from` (accounts SCALE-encoded).
    LedgerTransfer {
        asset: u32,
        from: Vec<u8>,
        to: Vec<u8>,
        amount: u128,
    },
    /// Undone by restoring `old`, only while the slot still holds `new`.
    X3Slot {
        key: H256,
        old: Option<[u8; 32]>,
        new: Option<[u8; 32]>,
    },
}

/// Runs bundle legs against chain state and undoes them from their journals.
pub trait LegExecutor<AccountId> {
    /// Execute `execution` as `executor`, the bundle's assigned executor (a transfer spends its
    /// funds); `context` identifies the leg (for events). Returns the journal of every write, in order.
    fn execute(
        context: H256,
        executor: &AccountId,
        execution: &LegExecution<AccountId>,
    ) -> Result<Vec<JournalEntry>, DispatchError>;

    /// Undo a journal, newest write first. All or nothing: an implementation must leave state
    /// unchanged when it returns an error.
    fn revert(entries: &[JournalEntry]) -> Result<(), RevertError>;

    /// The most weight one `execute` can take.
    fn max_weight() -> Weight;
}

/// No on-chain leg execution: every leg is refused and nothing can be reverted.
impl<AccountId> LegExecutor<AccountId> for () {
    fn execute(
        _context: H256,
        _executor: &AccountId,
        _execution: &LegExecution<AccountId>,
    ) -> Result<Vec<JournalEntry>, DispatchError> {
        Err(DispatchError::Other("no on-chain leg executor"))
    }
    fn revert(_entries: &[JournalEntry]) -> Result<(), RevertError> {
        Err(RevertError::VmNotAvailable(crate::proof::VmType::X3))
    }
    fn max_weight() -> Weight {
        Weight::zero()
    }
}

/// A journal as a leg receipt's `state_diff`: [`JOURNAL_MAGIC`] then the SCALE-encoded entries.
pub fn encode_journal(entries: &[JournalEntry]) -> Option<StateDiff> {
    let mut bytes = JOURNAL_MAGIC.to_vec();
    entries.to_vec().encode_to(&mut bytes);
    let bounded: BoundedVec<u8, crate::vm_revert::MaxStateDiffBytes> = bytes.try_into().ok()?;
    Some(StateDiff(bounded))
}

/// The entries of a chain journal; `None` for a diff in any other format.
pub fn decode_journal(diff: &StateDiff) -> Option<Vec<JournalEntry>> {
    let bytes = diff.as_bytes();
    let body = bytes.strip_prefix(JOURNAL_MAGIC.as_slice())?;
    let mut input = body;
    let entries = Vec::<JournalEntry>::decode(&mut input).ok()?;
    input.is_empty().then_some(entries)
}

/// The hash `ChainJournal` records for a journal.
pub fn journal_hash(diff: &StateDiff) -> H256 {
    H256(sp_io::hashing::blake2_256(diff.as_bytes()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_journal_round_trips_and_nothing_else_reads_as_one() {
        let entries = vec![
            JournalEntry::LedgerTransfer {
                asset: 3,
                from: vec![1; 32],
                to: vec![2; 32],
                amount: 77,
            },
            JournalEntry::X3Slot {
                key: H256::repeat_byte(9),
                old: None,
                new: Some([5; 32]),
            },
        ];
        let diff = encode_journal(&entries).expect("fits");
        assert_eq!(decode_journal(&diff), Some(entries));
        assert_eq!(decode_journal(&StateDiff::from(vec![1, 2, 3])), None);
        let mut trailing = diff.as_bytes().to_vec();
        trailing.push(0);
        assert_eq!(decode_journal(&StateDiff::from(trailing)), None);
    }
}
