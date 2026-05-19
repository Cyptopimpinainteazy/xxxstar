//! IXL interpreter.
//!
//! The interpreter is intentionally **side-effect-free against any real
//! ledger**.  It models effects against an [`ExecutionContext`] which the
//! caller (typically the cross-VM router pallet) is then responsible for
//! committing into pallet storage in a single transactional block.
//!
//! This split keeps the interpreter unit-testable and lets the same code be
//! reused by the offchain relayer for plan simulation / dry-run.

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use crate::instruction::{AccountAddr, AssetId, AssetKind, Instruction, IxlError};
use crate::planner::ExecutionPlan;
use crate::receipt::{Receipt, ReceiptEntry};

/// A single ledger mutation the interpreter wants the host to perform.
///
/// The host (router pallet, simulator, etc.) translates these into real
/// balance changes.  The interpreter never touches storage directly.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LedgerEffect {
    DebitPayer {
        kind: AssetKind,
        asset: AssetId,
        payer: AccountAddr,
        amount: u128,
    },
    CreditReceiver {
        kind: AssetKind,
        asset: AssetId,
        receiver: AccountAddr,
        amount: u128,
    },
    EmitProof {
        commitment: sp_core::H256,
    },
}

/// State the interpreter mutates while running a plan.  The host hands one in
/// fresh per bundle.  `swap` is a callback so we can plug in any internal AMM
/// without coupling this crate to a specific implementation.
pub struct ExecutionContext<'a> {
    /// Pending custody balances, keyed by `slot_id`.
    custody: BTreeMap<u32, CustodyEntry>,
    /// Effects to be applied by the host after the plan succeeds.
    pub effects: Vec<LedgerEffect>,
    /// Internal-only AMM/spot swap.  Returns `amount_out` actually delivered
    /// or an error.  The interpreter checks `amount_out >= min_out`.
    swap_fn: &'a dyn Fn(AssetKind, AssetId, AssetId, u128) -> Result<u128, IxlError>,
}

#[derive(Clone, Debug)]
struct CustodyEntry {
    kind: AssetKind,
    asset: AssetId,
    payer: AccountAddr,
    amount: u128,
}

impl<'a> ExecutionContext<'a> {
    pub fn new(
        swap_fn: &'a dyn Fn(AssetKind, AssetId, AssetId, u128) -> Result<u128, IxlError>,
    ) -> Self {
        Self {
            custody: BTreeMap::new(),
            effects: Vec::new(),
            swap_fn,
        }
    }
}

pub struct Interpreter;

impl Interpreter {
    /// Execute a validated plan.  Returns a [`Receipt`] of all committed
    /// effects.  On error, the receipt is returned alongside the error so the
    /// caller can pass it to `Rollback`.
    pub fn execute(
        plan: &ExecutionPlan,
        ctx: &mut ExecutionContext<'_>,
    ) -> Result<Receipt, (Receipt, IxlError)> {
        let mut receipt = Receipt::new();

        for instr in &plan.instructions {
            if let Err(e) = Self::step(instr, ctx, &mut receipt) {
                return Err((receipt, e));
            }
        }

        Ok(receipt)
    }

    fn step(
        instr: &Instruction,
        ctx: &mut ExecutionContext<'_>,
        receipt: &mut Receipt,
    ) -> Result<(), IxlError> {
        match instr {
            Instruction::Lock {
                slot_id,
                kind,
                asset,
                payer,
                amount,
            } => {
                ctx.custody.insert(
                    *slot_id,
                    CustodyEntry {
                        kind: *kind,
                        asset: *asset,
                        payer: *payer,
                        amount: *amount,
                    },
                );
                ctx.effects.push(LedgerEffect::DebitPayer {
                    kind: *kind,
                    asset: *asset,
                    payer: *payer,
                    amount: *amount,
                });
                receipt.push(ReceiptEntry::Locked {
                    slot_id: *slot_id,
                    kind: *kind,
                    asset: *asset,
                    payer: *payer,
                    amount: *amount,
                });
                Ok(())
            }
            Instruction::Mint {
                slot_id,
                kind,
                asset,
                receiver,
                amount,
            } => {
                let entry = ctx
                    .custody
                    .get_mut(slot_id)
                    .ok_or(IxlError::UnbalancedCustody)?;
                if entry.kind != *kind || entry.asset != *asset {
                    // The asset minted must match the asset under custody;
                    // cross-asset minting goes through Swap.
                    return Err(IxlError::InvalidOperands);
                }
                if entry.amount < *amount {
                    return Err(IxlError::InsufficientCustody);
                }
                entry.amount = entry
                    .amount
                    .checked_sub(*amount)
                    .ok_or(IxlError::Overflow)?;
                ctx.effects.push(LedgerEffect::CreditReceiver {
                    kind: *kind,
                    asset: *asset,
                    receiver: *receiver,
                    amount: *amount,
                });
                receipt.push(ReceiptEntry::Minted {
                    slot_id: *slot_id,
                    kind: *kind,
                    asset: *asset,
                    receiver: *receiver,
                    amount: *amount,
                });
                Ok(())
            }
            Instruction::Burn { slot_id } => {
                let entry = ctx
                    .custody
                    .remove(slot_id)
                    .ok_or(IxlError::UnbalancedCustody)?;
                receipt.push(ReceiptEntry::Burned {
                    slot_id: *slot_id,
                    kind: entry.kind,
                    asset: entry.asset,
                    amount: entry.amount,
                });
                Ok(())
            }
            Instruction::Swap {
                slot_id,
                kind,
                asset_in,
                asset_out,
                amount_in,
                min_out,
            } => {
                let entry = ctx
                    .custody
                    .get_mut(slot_id)
                    .ok_or(IxlError::UnbalancedCustody)?;
                if entry.kind != *kind || entry.asset != *asset_in {
                    return Err(IxlError::InvalidOperands);
                }
                if entry.amount < *amount_in {
                    return Err(IxlError::InsufficientCustody);
                }
                let out = (ctx.swap_fn)(*kind, *asset_in, *asset_out, *amount_in)?;
                if out < *min_out {
                    return Err(IxlError::SlippageExceeded);
                }
                // Slot now holds the output asset.  Replace the entry.
                entry.amount = entry
                    .amount
                    .checked_sub(*amount_in)
                    .ok_or(IxlError::Overflow)?;
                // Carry over any residue of the input back-to-back with the
                // output — but since custody slots are single-asset by
                // construction, we require amount_in == entry.amount here.
                if entry.amount != 0 {
                    return Err(IxlError::InvalidOperands);
                }
                entry.asset = *asset_out;
                entry.amount = out;
                receipt.push(ReceiptEntry::Swapped {
                    slot_id: *slot_id,
                    kind: *kind,
                    asset_in: *asset_in,
                    asset_out: *asset_out,
                    amount_in: *amount_in,
                    amount_out: out,
                });
                Ok(())
            }
            Instruction::Settle {
                slot_id,
                kind,
                receiver,
            } => {
                let entry = ctx
                    .custody
                    .remove(slot_id)
                    .ok_or(IxlError::UnbalancedCustody)?;
                if entry.kind != *kind {
                    return Err(IxlError::InvalidOperands);
                }
                if entry.amount > 0 {
                    ctx.effects.push(LedgerEffect::CreditReceiver {
                        kind: entry.kind,
                        asset: entry.asset,
                        receiver: *receiver,
                        amount: entry.amount,
                    });
                }
                receipt.push(ReceiptEntry::Settled {
                    slot_id: *slot_id,
                    kind: entry.kind,
                    asset: entry.asset,
                    receiver: *receiver,
                    amount: entry.amount,
                });
                Ok(())
            }
            Instruction::Refund { slot_id } => {
                let entry = ctx
                    .custody
                    .remove(slot_id)
                    .ok_or(IxlError::UnbalancedCustody)?;
                if entry.amount > 0 {
                    ctx.effects.push(LedgerEffect::CreditReceiver {
                        kind: entry.kind,
                        asset: entry.asset,
                        receiver: entry.payer,
                        amount: entry.amount,
                    });
                }
                receipt.push(ReceiptEntry::Refunded {
                    slot_id: *slot_id,
                    kind: entry.kind,
                    asset: entry.asset,
                    payer: entry.payer,
                    amount: entry.amount,
                });
                Ok(())
            }
            Instruction::EmitProof { commitment } => {
                ctx.effects.push(LedgerEffect::EmitProof {
                    commitment: *commitment,
                });
                receipt.push(ReceiptEntry::ProofEmitted {
                    commitment: *commitment,
                });
                Ok(())
            }
            Instruction::Abort => Err(IxlError::Aborted),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instruction::{Bundle, Instruction};
    use crate::planner::Planner;
    use sp_core::H256;

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

    fn no_swap(_: AssetKind, _: AssetId, _: AssetId, _: u128) -> Result<u128, IxlError> {
        Err(IxlError::InvalidOperands)
    }

    #[test]
    fn lock_settle_emits_two_effects() {
        let plan = match Planner::plan(Bundle {
            salt: H256::zero(),
            instructions: vec![
                Instruction::Lock {
                    slot_id: 0,
                    kind: AssetKind::X3Native,
                    asset: asset(1),
                    payer: addr(1),
                    amount: 100,
                },
                Instruction::Settle {
                    slot_id: 0,
                    kind: AssetKind::X3Native,
                    receiver: addr(2),
                },
            ],
        }) {
            Ok(plan) => plan,
            Err(err) => {
                panic!("planner rejected valid test bundle: {:?}", err);
            }
        };
        let mut ctx = ExecutionContext::new(&no_swap);
        let receipt = match Interpreter::execute(&plan, &mut ctx) {
            Ok(receipt) => receipt,
            Err((_partial, err)) => {
                panic!("interpreter failed on lock+settle: {:?}", err);
            }
        };
        assert_eq!(ctx.effects.len(), 2);
        assert_eq!(receipt.len(), 2);
    }

    #[test]
    fn refund_returns_to_payer() {
        let plan = match Planner::plan(Bundle {
            salt: H256::zero(),
            instructions: vec![
                Instruction::Lock {
                    slot_id: 0,
                    kind: AssetKind::X3Native,
                    asset: asset(1),
                    payer: addr(7),
                    amount: 250,
                },
                Instruction::Refund { slot_id: 0 },
            ],
        }) {
            Ok(plan) => plan,
            Err(err) => {
                panic!("planner rejected refund bundle: {:?}", err);
            }
        };
        let mut ctx = ExecutionContext::new(&no_swap);
        let exec_result = Interpreter::execute(&plan, &mut ctx);
        assert!(exec_result.is_ok());
        // First effect debits payer 250, second effect credits payer 250.
        assert_eq!(ctx.effects.len(), 2);
        assert!(matches!(
            &ctx.effects[1],
            LedgerEffect::CreditReceiver { .. }
        ));
        if let LedgerEffect::CreditReceiver {
            receiver, amount, ..
        } = &ctx.effects[1]
        {
            assert_eq!(*receiver, addr(7));
            assert_eq!(*amount, 250);
        }
    }

    #[test]
    fn swap_slippage_rejected() {
        let always_one =
            |_k: AssetKind, _a: AssetId, _b: AssetId, _amt: u128| -> Result<u128, IxlError> {
                Ok(1)
            };
        let plan = match Planner::plan(Bundle {
            salt: H256::zero(),
            instructions: vec![
                Instruction::Lock {
                    slot_id: 0,
                    kind: AssetKind::X3Native,
                    asset: asset(1),
                    payer: addr(1),
                    amount: 100,
                },
                Instruction::Swap {
                    slot_id: 0,
                    kind: AssetKind::X3Native,
                    asset_in: asset(1),
                    asset_out: asset(2),
                    amount_in: 100,
                    min_out: 90,
                },
                Instruction::Settle {
                    slot_id: 0,
                    kind: AssetKind::X3Native,
                    receiver: addr(2),
                },
            ],
        }) {
            Ok(plan) => plan,
            Err(err) => {
                panic!("planner rejected slippage bundle: {:?}", err);
            }
        };
        let mut ctx = ExecutionContext::new(&always_one);
        let exec_result = Interpreter::execute(&plan, &mut ctx);
        assert!(exec_result.is_err());
        let mut err = IxlError::InvalidOperands;
        if let Err((_partial, execute_err)) = exec_result {
            err = execute_err;
        }
        assert_eq!(err, IxlError::SlippageExceeded);
    }

    #[test]
    fn abort_returns_error_with_partial_receipt() {
        // Bundle: Lock -> Abort -> Settle.  Planner accepts (slot 0 is
        // resolved by Settle).  Interpreter must stop at Abort with the
        // Lock already committed to the receipt.
        let plan = match Planner::plan(Bundle {
            salt: H256::zero(),
            instructions: vec![
                Instruction::Lock {
                    slot_id: 0,
                    kind: AssetKind::X3Native,
                    asset: asset(1),
                    payer: addr(1),
                    amount: 100,
                },
                Instruction::Abort,
                Instruction::Settle {
                    slot_id: 0,
                    kind: AssetKind::X3Native,
                    receiver: addr(2),
                },
            ],
        }) {
            Ok(plan) => plan,
            Err(err) => {
                panic!("planner rejected abort bundle: {:?}", err);
            }
        };
        let mut ctx = ExecutionContext::new(&no_swap);
        let exec_result = Interpreter::execute(&plan, &mut ctx);
        assert!(exec_result.is_err());
        let mut partial = Receipt::new();
        let mut err = IxlError::InvalidOperands;
        if let Err((partial_receipt, execute_err)) = exec_result {
            partial = partial_receipt;
            err = execute_err;
        }
        assert_eq!(err, IxlError::Aborted);
        assert_eq!(partial.len(), 1, "only Lock should be in receipt");
    }
}
