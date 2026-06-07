//! Receipt: append-only log of every effect the interpreter committed.
//!
//! The rollback machinery walks the receipt in reverse to undo a partially
//! executed bundle.  The receipt is also the canonical artefact emitted to
//! the relayer / explorer.

use alloc::vec::Vec;
use parity_scale_codec::{Decode, DecodeWithMemTracking, Encode};
use scale_info::TypeInfo;
use sp_core::H256;

use crate::instruction::{AccountAddr, AssetId, AssetKind};

/// One side-effect performed by the interpreter.
#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, TypeInfo)]
pub enum ReceiptEntry {
    /// Funds were debited from `payer` and added to custody slot.
    Locked {
        slot_id: u32,
        kind: AssetKind,
        asset: AssetId,
        payer: AccountAddr,
        amount: u128,
    },
    /// `amount` of `asset` was credited to `receiver` from custody slot.
    Minted {
        slot_id: u32,
        kind: AssetKind,
        asset: AssetId,
        receiver: AccountAddr,
        amount: u128,
    },
    /// Custody slot was burned (held balance dropped).
    Burned {
        slot_id: u32,
        kind: AssetKind,
        asset: AssetId,
        amount: u128,
    },
    /// Internal swap executed.
    Swapped {
        slot_id: u32,
        kind: AssetKind,
        asset_in: AssetId,
        asset_out: AssetId,
        amount_in: u128,
        amount_out: u128,
    },
    /// Custody slot finalised to `receiver`.
    Settled {
        slot_id: u32,
        kind: AssetKind,
        asset: AssetId,
        receiver: AccountAddr,
        amount: u128,
    },
    /// Refund issued back to original payer.
    Refunded {
        slot_id: u32,
        kind: AssetKind,
        asset: AssetId,
        payer: AccountAddr,
        amount: u128,
    },
    /// Outbound packet commitment recorded.
    ProofEmitted { commitment: H256 },
}

/// Append-only receipt.
#[derive(Clone, Debug, Default, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, TypeInfo)]
pub struct Receipt {
    pub entries: Vec<ReceiptEntry>,
}

impl Receipt {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, entry: ReceiptEntry) {
        self.entries.push(entry);
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn iter(&self) -> core::slice::Iter<'_, ReceiptEntry> {
        self.entries.iter()
    }
}
