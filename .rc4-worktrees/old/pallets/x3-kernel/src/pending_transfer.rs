//! Pending transfer accounting: assets in-flight during cross-VM operations.
//!
//! When an asset transfer is initiated (prepare phase of a Comit) but not yet
//! finalized, the amount must be tracked as "pending" to prevent double-spend.
//! On commit the pending amount is released to the destination; on abort it
//! returns to the source.

use frame_support::pallet_prelude::*;

pub type AssetId = u32;
pub type Balance = u128;
pub type TransferId = u64;

/// A single in-flight transfer record.
#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, TypeInfo, MaxEncodedLen)]
pub struct PendingTransfer {
    pub transfer_id: TransferId,
    pub asset_id: AssetId,
    pub amount: Balance,
    pub state: TransferState,
}

/// State of an in-flight transfer.
#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, TypeInfo, MaxEncodedLen)]
pub enum TransferState {
    Pending,
    Committed,
    Aborted,
}

/// Errors returned by pending transfer operations.
#[derive(Debug, PartialEq, Eq)]
pub enum PendingError {
    /// Transfer ID already exists.
    DuplicateId,
    /// Transfer ID not found.
    NotFound,
    /// Transfer is not in Pending state.
    NotPending,
    /// Amount is zero.
    ZeroAmount,
    /// Arithmetic overflow.
    Overflow,
}

#[cfg(test)]
pub struct InMemoryPendingStore {
    transfers: std::collections::BTreeMap<TransferId, PendingTransfer>,
    pub total_pending: Balance,
}

#[cfg(test)]
impl InMemoryPendingStore {
    pub fn new() -> Self {
        Self {
            transfers: Default::default(),
            total_pending: 0,
        }
    }

    pub fn begin(
        &mut self,
        transfer_id: TransferId,
        asset_id: AssetId,
        amount: Balance,
    ) -> Result<(), PendingError> {
        if amount == 0 {
            return Err(PendingError::ZeroAmount);
        }
        if self.transfers.contains_key(&transfer_id) {
            return Err(PendingError::DuplicateId);
        }
        self.total_pending = self
            .total_pending
            .checked_add(amount)
            .ok_or(PendingError::Overflow)?;
        self.transfers.insert(
            transfer_id,
            PendingTransfer {
                transfer_id,
                asset_id,
                amount,
                state: TransferState::Pending,
            },
        );
        Ok(())
    }

    pub fn commit(&mut self, transfer_id: TransferId) -> Result<Balance, PendingError> {
        let t = self
            .transfers
            .get_mut(&transfer_id)
            .ok_or(PendingError::NotFound)?;
        if t.state != TransferState::Pending {
            return Err(PendingError::NotPending);
        }
        let amount = t.amount;
        t.state = TransferState::Committed;
        self.total_pending = self.total_pending.saturating_sub(amount);
        Ok(amount)
    }

    pub fn abort(&mut self, transfer_id: TransferId) -> Result<Balance, PendingError> {
        let t = self
            .transfers
            .get_mut(&transfer_id)
            .ok_or(PendingError::NotFound)?;
        if t.state != TransferState::Pending {
            return Err(PendingError::NotPending);
        }
        let amount = t.amount;
        t.state = TransferState::Aborted;
        self.total_pending = self.total_pending.saturating_sub(amount);
        Ok(amount)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_begin_and_commit() {
        let mut store = InMemoryPendingStore::new();
        store.begin(1, 42, 500).unwrap();
        assert_eq!(store.total_pending, 500);
        let released = store.commit(1).unwrap();
        assert_eq!(released, 500);
        assert_eq!(store.total_pending, 0);
    }

    #[test]
    fn test_begin_and_abort() {
        let mut store = InMemoryPendingStore::new();
        store.begin(2, 42, 300).unwrap();
        let returned = store.abort(2).unwrap();
        assert_eq!(returned, 300);
        assert_eq!(store.total_pending, 0);
    }

    #[test]
    fn test_duplicate_id_rejected() {
        let mut store = InMemoryPendingStore::new();
        store.begin(1, 1, 100).unwrap();
        assert_eq!(store.begin(1, 1, 200), Err(PendingError::DuplicateId));
    }

    #[test]
    fn test_commit_not_found() {
        let mut store = InMemoryPendingStore::new();
        assert_eq!(store.commit(999), Err(PendingError::NotFound));
    }

    #[test]
    fn test_double_commit_rejected() {
        let mut store = InMemoryPendingStore::new();
        store.begin(1, 1, 100).unwrap();
        store.commit(1).unwrap();
        assert_eq!(store.commit(1), Err(PendingError::NotPending));
    }

    #[test]
    fn test_zero_amount_rejected() {
        let mut store = InMemoryPendingStore::new();
        assert_eq!(store.begin(1, 1, 0), Err(PendingError::ZeroAmount));
    }

    #[test]
    fn test_multiple_pending_independent() {
        let mut store = InMemoryPendingStore::new();
        store.begin(1, 1, 100).unwrap();
        store.begin(2, 1, 200).unwrap();
        assert_eq!(store.total_pending, 300);
        store.commit(1).unwrap();
        assert_eq!(store.total_pending, 200);
        store.abort(2).unwrap();
        assert_eq!(store.total_pending, 0);
    }
}
