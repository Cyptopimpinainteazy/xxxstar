//! Bundle planner: validates a bundle before any side-effects run.
//!
//! Validation rules:
//!
//! 1. Length ≤ `Instruction::MAX_BUNDLE`.
//! 2. Every `Lock { slot_id }` must be matched by exactly one terminating
//!    instruction targeting that slot: `Settle`, `Refund`, or `Burn`.
//! 3. `Mint` / `Swap` against a slot must follow that slot's `Lock` and
//!    precede its terminator.
//! 4. No instruction may reference a slot that was never `Lock`-ed.
//! 5. Amounts must be non-zero where required.
//!
//! Output: an [`ExecutionPlan`] that the interpreter can run without
//! re-validating.

use alloc::collections::{BTreeMap, BTreeSet};
use alloc::vec::Vec;
use parity_scale_codec::{Decode, DecodeWithMemTracking, Encode};
use scale_info::TypeInfo;

use crate::instruction::{Bundle, Instruction, IxlError};

/// Per-slot lifecycle as observed by the planner.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SlotPhase {
    Locked,
    Terminated,
}

/// A validated, ready-to-execute plan.
#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, TypeInfo)]
pub struct ExecutionPlan {
    pub instructions: Vec<Instruction>,
}

impl ExecutionPlan {
    pub fn len(&self) -> usize {
        self.instructions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.instructions.is_empty()
    }
}

pub struct Planner;

impl Planner {
    pub fn plan(bundle: Bundle) -> Result<ExecutionPlan, IxlError> {
        if bundle.instructions.len() > Instruction::MAX_BUNDLE {
            return Err(IxlError::BundleTooLong);
        }

        let mut phase: BTreeMap<u32, SlotPhase> = BTreeMap::new();
        let mut declared_slots: BTreeSet<u32> = BTreeSet::new();

        for instr in &bundle.instructions {
            match instr {
                Instruction::Lock {
                    slot_id, amount, ..
                } => {
                    if *amount == 0 {
                        return Err(IxlError::InvalidOperands);
                    }
                    if declared_slots.contains(slot_id) {
                        // Re-locking the same slot id is a planner error: the
                        // slot is the unit of accounting and must be unique
                        // within a bundle.
                        return Err(IxlError::InvalidOperands);
                    }
                    declared_slots.insert(*slot_id);
                    phase.insert(*slot_id, SlotPhase::Locked);
                }
                Instruction::Mint {
                    slot_id, amount, ..
                } => {
                    if *amount == 0 {
                        return Err(IxlError::InvalidOperands);
                    }
                    Self::expect_locked(&phase, slot_id)?;
                }
                Instruction::Swap {
                    slot_id,
                    amount_in,
                    min_out,
                    asset_in,
                    asset_out,
                    ..
                } => {
                    if *amount_in == 0 || asset_in == asset_out {
                        return Err(IxlError::InvalidOperands);
                    }
                    let _ = min_out; // zero is allowed (any slippage)
                    Self::expect_locked(&phase, slot_id)?;
                }
                Instruction::Burn { slot_id } => {
                    Self::expect_locked(&phase, slot_id)?;
                    phase.insert(*slot_id, SlotPhase::Terminated);
                }
                Instruction::Settle { slot_id, .. } => {
                    Self::expect_locked(&phase, slot_id)?;
                    phase.insert(*slot_id, SlotPhase::Terminated);
                }
                Instruction::Refund { slot_id } => {
                    Self::expect_locked(&phase, slot_id)?;
                    phase.insert(*slot_id, SlotPhase::Terminated);
                }
                Instruction::EmitProof { .. } => {
                    // No slot interaction.
                }
                Instruction::Abort => {
                    // Abort is allowed anywhere; the interpreter handles
                    // unwinding the partially-executed plan.
                }
            }
        }

        // Every Lock must end Terminated.
        for (slot, p) in &phase {
            if *p != SlotPhase::Terminated {
                let _ = slot;
                return Err(IxlError::UnresolvedCustody);
            }
        }

        Ok(ExecutionPlan {
            instructions: bundle.instructions,
        })
    }

    fn expect_locked(phase: &BTreeMap<u32, SlotPhase>, slot_id: &u32) -> Result<(), IxlError> {
        match phase.get(slot_id) {
            Some(SlotPhase::Locked) => Ok(()),
            _ => Err(IxlError::UnbalancedCustody),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instruction::{AssetKind, Instruction};
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

    #[test]
    fn empty_bundle_plans() {
        let plan = match Planner::plan(Bundle {
            salt: H256::zero(),
            instructions: vec![],
        }) {
            Ok(plan) => plan,
            Err(err) => {
                panic!("planner failed for empty bundle: {:?}", err);
            }
        };
        assert!(plan.is_empty());
    }

    #[test]
    fn lock_then_settle_is_valid() {
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
                    kind: AssetKind::X3Evm,
                    receiver: addr(2),
                },
            ],
        }) {
            Ok(plan) => plan,
            Err(err) => {
                panic!("planner rejected valid lock+settle: {:?}", err);
            }
        };
        assert_eq!(plan.len(), 2);
    }

    #[test]
    fn unresolved_custody_rejected() {
        let err = match Planner::plan(Bundle {
            salt: H256::zero(),
            instructions: vec![Instruction::Lock {
                slot_id: 0,
                kind: AssetKind::X3Native,
                asset: asset(1),
                payer: addr(1),
                amount: 100,
            }],
        }) {
            Ok(_) => {
                panic!("planner accepted unresolved custody");
            }
            Err(err) => err,
        };
        assert_eq!(err, IxlError::UnresolvedCustody);
    }

    #[test]
    fn mint_without_lock_rejected() {
        let err = match Planner::plan(Bundle {
            salt: H256::zero(),
            instructions: vec![Instruction::Mint {
                slot_id: 7,
                kind: AssetKind::X3Native,
                asset: asset(1),
                receiver: addr(1),
                amount: 1,
            }],
        }) {
            Ok(_) => {
                panic!("planner accepted mint without lock");
            }
            Err(err) => err,
        };
        assert_eq!(err, IxlError::UnbalancedCustody);
    }

    #[test]
    fn duplicate_slot_rejected() {
        let err = match Planner::plan(Bundle {
            salt: H256::zero(),
            instructions: vec![
                Instruction::Lock {
                    slot_id: 0,
                    kind: AssetKind::X3Native,
                    asset: asset(1),
                    payer: addr(1),
                    amount: 100,
                },
                Instruction::Lock {
                    slot_id: 0,
                    kind: AssetKind::X3Native,
                    asset: asset(1),
                    payer: addr(1),
                    amount: 50,
                },
                Instruction::Settle {
                    slot_id: 0,
                    kind: AssetKind::X3Native,
                    receiver: addr(2),
                },
            ],
        }) {
            Ok(_) => {
                panic!("planner accepted duplicate slot");
            }
            Err(err) => err,
        };
        assert_eq!(err, IxlError::InvalidOperands);
    }

    #[test]
    fn bundle_too_long() {
        let mut instrs = vec![];
        for i in 0..(Instruction::MAX_BUNDLE + 1) {
            instrs.push(Instruction::Lock {
                slot_id: i as u32,
                kind: AssetKind::X3Native,
                asset: asset(1),
                payer: addr(1),
                amount: 1,
            });
        }
        let err = match Planner::plan(Bundle {
            salt: H256::zero(),
            instructions: instrs,
        }) {
            Ok(_) => {
                panic!("planner accepted oversized bundle");
            }
            Err(err) => err,
        };
        assert_eq!(err, IxlError::BundleTooLong);
    }
}
