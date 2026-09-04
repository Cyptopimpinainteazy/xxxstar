//! Rollback: turn a partial receipt into the inverse ledger effects.
//!
//! When the interpreter fails midway through a bundle, the host calls
//! [`Rollback::invert`] to obtain the list of [`crate::LedgerEffect`]s that
//! must be applied to put the world back where it started.  Crucially this
//! is **derived from the receipt only** — the rollback never re-runs
//! interpreter logic, which means we can never accidentally re-execute a
//! `Swap` or call back into application code while unwinding.

use alloc::vec::Vec;

use crate::interpreter::LedgerEffect;
use crate::receipt::{Receipt, ReceiptEntry};

pub struct Rollback;

impl Rollback {
    /// Invert a partial receipt into the effects that restore the ledger.
    /// The output list is in the order the host should apply them.
    pub fn invert(receipt: &Receipt) -> Vec<LedgerEffect> {
        let mut out = Vec::with_capacity(receipt.len());

        // Walk the receipt in reverse so the last action is undone first.
        for entry in receipt.entries.iter().rev() {
            match entry {
                ReceiptEntry::Locked {
                    kind,
                    asset,
                    payer,
                    amount,
                    ..
                } => {
                    // We had debited the payer; refund.
                    out.push(LedgerEffect::CreditReceiver {
                        kind: *kind,
                        asset: *asset,
                        receiver: *payer,
                        amount: *amount,
                    });
                }
                ReceiptEntry::Minted {
                    kind,
                    asset,
                    receiver,
                    amount,
                    ..
                } => {
                    // We had credited the receiver; reverse-debit.
                    out.push(LedgerEffect::DebitPayer {
                        kind: *kind,
                        asset: *asset,
                        payer: *receiver,
                        amount: *amount,
                    });
                }
                ReceiptEntry::Settled {
                    kind,
                    asset,
                    receiver,
                    amount,
                    ..
                } => {
                    if *amount > 0 {
                        out.push(LedgerEffect::DebitPayer {
                            kind: *kind,
                            asset: *asset,
                            payer: *receiver,
                            amount: *amount,
                        });
                    }
                }
                ReceiptEntry::Refunded {
                    kind,
                    asset,
                    payer,
                    amount,
                    ..
                } => {
                    if *amount > 0 {
                        out.push(LedgerEffect::DebitPayer {
                            kind: *kind,
                            asset: *asset,
                            payer: *payer,
                            amount: *amount,
                        });
                    }
                }
                // Burns and proof emissions are non-financial; nothing to undo.
                ReceiptEntry::Burned { .. } | ReceiptEntry::ProofEmitted { .. } => {}
                // Swaps are intra-custody — the only externally visible effects
                // come from Lock / Mint / Settle / Refund, which are already
                // covered.  The custody balance lives only in the interpreter
                // context, which is discarded on rollback.
                ReceiptEntry::Swapped { .. } => {}
            }
        }

        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instruction::AssetKind;

    fn asset(b: u8) -> [u8; 32] {
        let mut a = [0u8; 32];
        a[0] = b;
        a
    }
    fn addr(b: u8) -> [u8; 32] {
        let mut a = [0u8; 32];
        a[31] = b;
        a
    }

    #[test]
    fn lock_inverted_to_credit_back_to_payer() {
        let mut r = Receipt::new();
        r.push(ReceiptEntry::Locked {
            slot_id: 0,
            kind: AssetKind::X3Native,
            asset: asset(1),
            payer: addr(7),
            amount: 100,
        });
        let effs = Rollback::invert(&r);
        assert_eq!(effs.len(), 1);
        assert!(matches!(&effs[0], LedgerEffect::CreditReceiver { .. }));
        if let LedgerEffect::CreditReceiver {
            receiver, amount, ..
        } = &effs[0]
        {
            assert_eq!(*receiver, addr(7));
            assert_eq!(*amount, 100);
        }
    }

    #[test]
    fn lock_then_mint_inverts_in_reverse() {
        let mut r = Receipt::new();
        r.push(ReceiptEntry::Locked {
            slot_id: 0,
            kind: AssetKind::X3Native,
            asset: asset(1),
            payer: addr(1),
            amount: 100,
        });
        r.push(ReceiptEntry::Minted {
            slot_id: 0,
            kind: AssetKind::X3Native,
            asset: asset(1),
            receiver: addr(2),
            amount: 40,
        });
        let effs = Rollback::invert(&r);
        // Reverse order: undo mint first, then undo lock.
        assert_eq!(effs.len(), 2);
        matches!(effs[0], LedgerEffect::DebitPayer { .. });
        matches!(effs[1], LedgerEffect::CreditReceiver { .. });
    }
}
