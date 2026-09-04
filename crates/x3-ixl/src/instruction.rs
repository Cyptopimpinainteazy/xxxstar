//! IXL instruction set.
//!
//! Instructions describe *intent*; they do not directly mutate state.  The
//! interpreter (`interpreter.rs`) translates them into [`crate::LedgerEffect`]s
//! against an `ExecutionContext`.

use alloc::vec::Vec;
use parity_scale_codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use sp_core::H256;

/// Which internal VM/asset domain we are talking about.
///
/// This is a closed enum on purpose: external chains are explicitly out of
/// scope for v0.4 internal-only mainnet.
#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Encode,
    Decode,
    DecodeWithMemTracking,
    TypeInfo,
    MaxEncodedLen,
)]
pub enum AssetKind {
    X3Native,
    X3Evm,
    X3Svm,
}

/// Internal asset identifier — a 32-byte fingerprint.  For native this is
/// typically a fixed constant; for EVM/SVM it is derived from the canonical
/// asset registry entry.
pub type AssetId = [u8; 32];

/// Account address inside one of the internal VMs.  We use a 32-byte
/// canonical form; EVM 20-byte addresses are zero-padded on the right.
pub type AccountAddr = [u8; 32];

/// Errors raised during planning or execution.  Every variant maps cleanly
/// onto a rollback action.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, TypeInfo)]
pub enum IxlError {
    /// Bundle exceeded the configured maximum number of instructions.
    BundleTooLong,
    /// `Settle` / `Refund` / `Burn` referenced a custody slot that has no
    /// matching prior `Lock`.
    UnbalancedCustody,
    /// `Swap` produced fewer output tokens than the declared `min_out`.
    SlippageExceeded,
    /// `Mint` attempted to credit more than is held in custody for that asset.
    InsufficientCustody,
    /// Numeric overflow (e.g. summing custody balances).
    Overflow,
    /// Instruction operands violated a domain rule (e.g. zero amount).
    InvalidOperands,
    /// Bundle cannot end in a non-terminal state — every `Lock` must be
    /// matched by a `Settle`, `Refund`, or `Burn` before the planner accepts.
    UnresolvedCustody,
    /// Explicit abort emitted by `Abort` instruction.
    Aborted,
}

/// One IXL instruction.
#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, TypeInfo)]
pub enum Instruction {
    /// Move `amount` of `asset` from `payer` into router custody slot
    /// `slot_id`.  The custody slot is the unit of accounting the planner
    /// uses to enforce balance.
    Lock {
        slot_id: u32,
        kind: AssetKind,
        asset: AssetId,
        payer: AccountAddr,
        amount: u128,
    },
    /// Mint up to `amount` of `asset` for `receiver` against custody held in
    /// `slot_id`.  Decrements the slot.
    Mint {
        slot_id: u32,
        kind: AssetKind,
        asset: AssetId,
        receiver: AccountAddr,
        amount: u128,
    },
    /// Burn the entire balance of custody slot `slot_id`.  Used after a
    /// successful settlement to drop the escrowed copy.
    Burn { slot_id: u32 },
    /// Spot swap inside one VM.  `slot_id` is debited by `amount_in`,
    /// credited by at least `min_out` of `asset_out`.  Output goes back into
    /// the same slot so a downstream `Mint`/`Settle` can dispense it.
    Swap {
        slot_id: u32,
        kind: AssetKind,
        asset_in: AssetId,
        asset_out: AssetId,
        amount_in: u128,
        min_out: u128,
    },
    /// Finalise: empty `slot_id` to `receiver`.
    Settle {
        slot_id: u32,
        kind: AssetKind,
        receiver: AccountAddr,
    },
    /// Write a packet commitment so a relayer can drive the destination VM.
    /// The interpreter records the commitment; actually inserting it into
    /// pallet storage is the router pallet's job.
    EmitProof { commitment: H256 },
    /// Refund the slot back to the original payer (recorded at `Lock` time).
    Refund { slot_id: u32 },
    /// Explicit abort.  The interpreter immediately stops and the rollback
    /// path runs.
    Abort,
}

impl Instruction {
    /// Maximum number of instructions per bundle.  Keeps interpretation cost
    /// bounded and protects validators from unbounded weight.
    pub const MAX_BUNDLE: usize = 64;
}

/// A bundle is just a vector of instructions plus the bundle-level salt that
/// the planner uses to derive deterministic slot ids when callers do not
/// pre-assign them.
#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, TypeInfo)]
pub struct Bundle {
    pub salt: H256,
    pub instructions: Vec<Instruction>,
}
