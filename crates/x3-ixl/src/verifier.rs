//! Receipt verifier.
//!
//! The verifier operates **after** execution: it proves a completed
//! [`Receipt`] is internally consistent and that every claim is reproducible
//! from public inputs.  It does **not** touch any live ledger state; it is
//! purely a read-only structural check so the cross-VM router, relayer, and
//! light clients can independently confirm a receipt is well-formed before
//! forwarding it.
//!
//! Verification rules enforced here:
//!
//! 1. **Custody balance conservation** — the sum of `Locked` amounts for
//!    every slot equals the sum of `Minted + Swapped(in) + Settled + Refunded
//!    + Burned` amounts for that slot.  No slot may end in credit without a
//!      corresponding debit.
//!
//! 2. **Slot lifecycle monotonicity** — each slot must follow
//!    `Locked → [Minted|Swapped]* → Settled|Refunded|Burned`.  A slot may not
//!    be credited before it is locked, and it may not receive any entry after
//!    it is terminated.
//!
//! 3. **No zero-amount transfers** — every credit/debit entry must carry a
//!    non-zero amount.
//!
//! 4. **Proof commitment uniqueness** — each `ProofEmitted` commitment hash
//!    must appear at most once in the receipt.
//!
//! 5. **Overflow safety** — all running totals are accumulated with checked
//!    arithmetic; any overflow is a hard verification failure.

use alloc::collections::{BTreeMap, BTreeSet};
use alloc::vec::Vec;

use crate::instruction::IxlError;
use crate::receipt::{Receipt, ReceiptEntry};

// ────────────────────────────────────────────────────────────────────────────
// Verification error (superset of IxlError; kept separate so the verifier
// can surface richer diagnostics without changing the core error type).
// ────────────────────────────────────────────────────────────────────────────

/// Why receipt verification failed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VerifyError {
    /// Wraps an execution-layer error (re-exposed for convenience).
    Execution(IxlError),
    /// A slot was debited before it was ever locked.
    SlotNeverLocked { slot_id: u32 },
    /// A credit or debit was recorded for a slot that had already been
    /// terminated.
    SlotAlreadyTerminated { slot_id: u32 },
    /// Debits and credits for a slot do not balance.
    UnbalancedSlot { slot_id: u32 },
    /// A zero-amount entry was found (forbidden; indicates logic error).
    ZeroAmount { slot_id: u32 },
    /// The same packet commitment appeared more than once.
    DuplicateProofCommitment,
    /// Checked arithmetic overflowed accumulating a slot balance.
    BalanceOverflow { slot_id: u32 },
    /// Receipt is empty — an empty bundle should not produce an empty receipt.
    EmptyReceipt,
}

impl From<IxlError> for VerifyError {
    fn from(e: IxlError) -> Self {
        VerifyError::Execution(e)
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Per-slot accounting state
// ────────────────────────────────────────────────────────────────────────────

#[derive(Default)]
struct SlotLedger {
    locked: u128,
    debited: u128, // Minted + swap amount_in + Settled + Refunded + Burned
    terminated: bool,
}

impl SlotLedger {
    fn add_lock(&mut self, amount: u128, slot_id: u32) -> Result<(), VerifyError> {
        self.locked = self
            .locked
            .checked_add(amount)
            .ok_or(VerifyError::BalanceOverflow { slot_id })?;
        Ok(())
    }

    fn add_debit(&mut self, amount: u128, slot_id: u32) -> Result<(), VerifyError> {
        if self.terminated {
            return Err(VerifyError::SlotAlreadyTerminated { slot_id });
        }
        self.debited = self
            .debited
            .checked_add(amount)
            .ok_or(VerifyError::BalanceOverflow { slot_id })?;
        Ok(())
    }

    fn terminate(&mut self, amount: u128, slot_id: u32) -> Result<(), VerifyError> {
        if self.terminated {
            return Err(VerifyError::SlotAlreadyTerminated { slot_id });
        }
        self.debited = self
            .debited
            .checked_add(amount)
            .ok_or(VerifyError::BalanceOverflow { slot_id })?;
        self.terminated = true;
        Ok(())
    }

    fn is_balanced(&self) -> bool {
        self.locked == self.debited
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Public verifier
// ────────────────────────────────────────────────────────────────────────────

/// Stateless receipt verifier.
pub struct Verifier;

impl Verifier {
    /// Verify `receipt` is internally consistent.
    ///
    /// Returns `Ok(())` when all rules pass.  Returns the first
    /// [`VerifyError`] encountered on failure.  The check is purely
    /// structural; it does not query any pallet storage.
    pub fn verify(receipt: &Receipt) -> Result<(), VerifyError> {
        if receipt.is_empty() {
            return Err(VerifyError::EmptyReceipt);
        }

        let mut slots: BTreeMap<u32, SlotLedger> = BTreeMap::new();
        let mut seen_proofs: BTreeSet<[u8; 32]> = BTreeSet::new();

        for entry in receipt.iter() {
            match entry {
                // ── Lock: opens a new slot (or adds to an existing one on
                //    multi-payer patterns, but for v0.4 each slot is opened
                //    exactly once — the planner already enforces this).
                ReceiptEntry::Locked {
                    slot_id, amount, ..
                } => {
                    if *amount == 0 {
                        return Err(VerifyError::ZeroAmount { slot_id: *slot_id });
                    }
                    let ledger = slots.entry(*slot_id).or_default();
                    ledger.add_lock(*amount, *slot_id)?;
                }

                // ── Mint: partial debit from an open slot.
                ReceiptEntry::Minted {
                    slot_id, amount, ..
                } => {
                    if *amount == 0 {
                        return Err(VerifyError::ZeroAmount { slot_id: *slot_id });
                    }
                    let ledger = slots
                        .get_mut(slot_id)
                        .ok_or(VerifyError::SlotNeverLocked { slot_id: *slot_id })?;
                    ledger.add_debit(*amount, *slot_id)?;
                }

                // ── Swapped: amount_in is debited; amount_out is re-credited
                //    to the same slot and handled by a subsequent Mint/Settle.
                //    We only track the debit side here.
                ReceiptEntry::Swapped {
                    slot_id, amount_in, ..
                } => {
                    if *amount_in == 0 {
                        return Err(VerifyError::ZeroAmount { slot_id: *slot_id });
                    }
                    let ledger = slots
                        .get_mut(slot_id)
                        .ok_or(VerifyError::SlotNeverLocked { slot_id: *slot_id })?;
                    ledger.add_debit(*amount_in, *slot_id)?;
                }

                // ── Settle: terminal debit — finalises the slot.
                ReceiptEntry::Settled {
                    slot_id, amount, ..
                } => {
                    if *amount == 0 {
                        return Err(VerifyError::ZeroAmount { slot_id: *slot_id });
                    }
                    let ledger = slots
                        .get_mut(slot_id)
                        .ok_or(VerifyError::SlotNeverLocked { slot_id: *slot_id })?;
                    ledger.terminate(*amount, *slot_id)?;
                }

                // ── Refunded: terminal debit — returns custody to payer.
                ReceiptEntry::Refunded {
                    slot_id, amount, ..
                } => {
                    if *amount == 0 {
                        return Err(VerifyError::ZeroAmount { slot_id: *slot_id });
                    }
                    let ledger = slots
                        .get_mut(slot_id)
                        .ok_or(VerifyError::SlotNeverLocked { slot_id: *slot_id })?;
                    ledger.terminate(*amount, *slot_id)?;
                }

                // ── Burned: terminal debit — drops custody to zero.
                ReceiptEntry::Burned {
                    slot_id, amount, ..
                } => {
                    if *amount == 0 {
                        return Err(VerifyError::ZeroAmount { slot_id: *slot_id });
                    }
                    let ledger = slots
                        .get_mut(slot_id)
                        .ok_or(VerifyError::SlotNeverLocked { slot_id: *slot_id })?;
                    ledger.terminate(*amount, *slot_id)?;
                }

                // ── ProofEmitted: commitment must be unique in this receipt.
                ReceiptEntry::ProofEmitted { commitment } => {
                    if !seen_proofs.insert(commitment.0) {
                        return Err(VerifyError::DuplicateProofCommitment);
                    }
                }
            }
        }

        // ── Final balance check across all slots.
        for (slot_id, ledger) in &slots {
            if !ledger.is_balanced() {
                return Err(VerifyError::UnbalancedSlot { slot_id: *slot_id });
            }
        }

        Ok(())
    }

    /// Convenience wrapper: returns `true` when the receipt is valid.
    pub fn is_valid(receipt: &Receipt) -> bool {
        Self::verify(receipt).is_ok()
    }

    /// Collect all slot ids present in the receipt.
    pub fn slot_ids(receipt: &Receipt) -> Vec<u32> {
        let mut ids: BTreeSet<u32> = BTreeSet::new();
        for entry in receipt.iter() {
            let slot = match entry {
                ReceiptEntry::Locked { slot_id, .. } => Some(*slot_id),
                ReceiptEntry::Minted { slot_id, .. } => Some(*slot_id),
                ReceiptEntry::Burned { slot_id, .. } => Some(*slot_id),
                ReceiptEntry::Swapped { slot_id, .. } => Some(*slot_id),
                ReceiptEntry::Settled { slot_id, .. } => Some(*slot_id),
                ReceiptEntry::Refunded { slot_id, .. } => Some(*slot_id),
                ReceiptEntry::ProofEmitted { .. } => None,
            };
            if let Some(s) = slot {
                ids.insert(s);
            }
        }
        ids.into_iter().collect()
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Tests
// ────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::receipt::{Receipt, ReceiptEntry};
    use sp_core::H256;

    const ASSET: [u8; 32] = [0xAA; 32];
    const ADDR: [u8; 32] = [0xBB; 32];
    use crate::instruction::AssetKind;

    fn locked(slot_id: u32, amount: u128) -> ReceiptEntry {
        ReceiptEntry::Locked {
            slot_id,
            kind: AssetKind::X3Native,
            asset: ASSET,
            payer: ADDR,
            amount,
        }
    }

    fn settled(slot_id: u32, amount: u128) -> ReceiptEntry {
        ReceiptEntry::Settled {
            slot_id,
            kind: AssetKind::X3Native,
            asset: ASSET,
            receiver: ADDR,
            amount,
        }
    }

    fn refunded(slot_id: u32, amount: u128) -> ReceiptEntry {
        ReceiptEntry::Refunded {
            slot_id,
            kind: AssetKind::X3Native,
            asset: ASSET,
            payer: ADDR,
            amount,
        }
    }

    fn burned(slot_id: u32, amount: u128) -> ReceiptEntry {
        ReceiptEntry::Burned {
            slot_id,
            kind: AssetKind::X3Native,
            asset: ASSET,
            amount,
        }
    }

    fn minted(slot_id: u32, amount: u128) -> ReceiptEntry {
        ReceiptEntry::Minted {
            slot_id,
            kind: AssetKind::X3Native,
            asset: ASSET,
            receiver: ADDR,
            amount,
        }
    }

    fn proof(seed: u8) -> ReceiptEntry {
        ReceiptEntry::ProofEmitted {
            commitment: H256([seed; 32]),
        }
    }

    // ── Happy paths

    #[test]
    fn lock_settle_balanced() {
        let mut r = Receipt::new();
        r.push(locked(0, 100));
        r.push(settled(0, 100));
        assert_eq!(Verifier::verify(&r), Ok(()));
    }

    #[test]
    fn lock_refund_balanced() {
        let mut r = Receipt::new();
        r.push(locked(0, 50));
        r.push(refunded(0, 50));
        assert_eq!(Verifier::verify(&r), Ok(()));
    }

    #[test]
    fn lock_burn_balanced() {
        let mut r = Receipt::new();
        r.push(locked(0, 77));
        r.push(burned(0, 77));
        assert_eq!(Verifier::verify(&r), Ok(()));
    }

    #[test]
    fn lock_mint_settle_balanced() {
        let mut r = Receipt::new();
        r.push(locked(0, 200));
        r.push(minted(0, 50));
        r.push(settled(0, 150));
        assert_eq!(Verifier::verify(&r), Ok(()));
    }

    #[test]
    fn multi_slot_balanced() {
        let mut r = Receipt::new();
        r.push(locked(0, 100));
        r.push(locked(1, 200));
        r.push(settled(0, 100));
        r.push(refunded(1, 200));
        assert_eq!(Verifier::verify(&r), Ok(()));
    }

    #[test]
    fn proof_commitment_accepted() {
        let mut r = Receipt::new();
        r.push(locked(0, 1));
        r.push(settled(0, 1));
        r.push(proof(0x01));
        r.push(proof(0x02));
        assert_eq!(Verifier::verify(&r), Ok(()));
    }

    // ── Failure paths

    #[test]
    fn empty_receipt_rejected() {
        let r = Receipt::new();
        assert_eq!(Verifier::verify(&r), Err(VerifyError::EmptyReceipt));
    }

    #[test]
    fn unbalanced_slot_detected() {
        let mut r = Receipt::new();
        r.push(locked(0, 100));
        r.push(settled(0, 90)); // 10 left in custody
        assert_eq!(
            Verifier::verify(&r),
            Err(VerifyError::UnbalancedSlot { slot_id: 0 })
        );
    }

    #[test]
    fn slot_never_locked_detected() {
        let mut r = Receipt::new();
        r.push(locked(0, 100));
        r.push(settled(0, 100));
        r.push(minted(99, 50)); // slot 99 was never locked
        assert_eq!(
            Verifier::verify(&r),
            Err(VerifyError::SlotNeverLocked { slot_id: 99 })
        );
    }

    #[test]
    fn double_settle_rejected() {
        let mut r = Receipt::new();
        r.push(locked(0, 100));
        r.push(settled(0, 50));
        r.push(settled(0, 50)); // second settle on terminated slot
        assert_eq!(
            Verifier::verify(&r),
            Err(VerifyError::SlotAlreadyTerminated { slot_id: 0 })
        );
    }

    #[test]
    fn duplicate_proof_commitment_rejected() {
        let mut r = Receipt::new();
        r.push(locked(0, 1));
        r.push(settled(0, 1));
        r.push(proof(0xCC));
        r.push(proof(0xCC)); // duplicate
        assert_eq!(
            Verifier::verify(&r),
            Err(VerifyError::DuplicateProofCommitment)
        );
    }

    #[test]
    fn zero_amount_lock_rejected() {
        let mut r = Receipt::new();
        r.push(locked(0, 0)); // zero not allowed
        assert_eq!(
            Verifier::verify(&r),
            Err(VerifyError::ZeroAmount { slot_id: 0 })
        );
    }

    #[test]
    fn slot_ids_collects_correctly() {
        let mut r = Receipt::new();
        r.push(locked(0, 10));
        r.push(locked(3, 20));
        r.push(settled(0, 10));
        r.push(refunded(3, 20));
        let mut ids = Verifier::slot_ids(&r);
        ids.sort();
        assert_eq!(ids, vec![0, 3]);
    }
}
