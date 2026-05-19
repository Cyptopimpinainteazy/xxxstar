//! Contract compiler — translates a `UniversalContract` into an ordered
//! `IxlBundle` (sequence of IXL instructions) ready for the interpreter.
//!
//! The compiler is the bridge between the developer-facing `UniversalContract`
//! definition (high-level actions) and the low-level `x3-ixl` bytecode.

use crate::actions::{Action, Domain};
use crate::error::UcError;
use sha2::{Digest, Sha256};
use sp_core::H256;
use x3_ixl::instruction::{AssetKind, Bundle, Instruction};

// Zero address constant for actions that don't specify a receiver.
const ZERO_ADDR: [u8; 32] = [0u8; 32];

/// A compiled IXL bundle: an ordered list of instructions with a
/// domain-separated commitment hash covering the whole sequence.
#[derive(Debug, Clone)]
pub struct IxlBundle {
    pub bundle: Bundle,
    /// SHA-256 commitment over all action commitments in order.
    pub program_hash: [u8; 32],
}

impl IxlBundle {
    /// Number of instructions in the bundle.
    pub fn len(&self) -> usize {
        self.bundle.instructions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.bundle.instructions.is_empty()
    }
}

/// Compiler: converts `Vec<Action>` → `IxlBundle`.
pub struct Compiler;

impl Compiler {
    /// Compile an ordered list of actions into an IXL bundle.
    ///
    /// Rules:
    /// - The action list must not be empty.
    /// - Each action must pass its numeric validation.
    /// - An `Abort` action must be the last action if present.
    pub fn compile(actions: &[Action]) -> Result<IxlBundle, UcError> {
        if actions.is_empty() {
            return Err(UcError::EmptyActionList);
        }

        // Validate all actions first.
        for (idx, action) in actions.iter().enumerate() {
            action.validate()?;
            // Abort must be last.
            if action.is_terminal() && idx < actions.len() - 1 {
                return Err(UcError::AbortNotLast);
            }
        }

        // Map each action to the corresponding IXL instruction.
        let instructions: Vec<Instruction> = actions
            .iter()
            .enumerate()
            .map(|(slot, action)| Self::action_to_instruction(action, slot as u32))
            .collect();

        // Compute the program commitment: SHA-256 over concatenated action commitments.
        let mut h = Sha256::new();
        h.update(b"x3:uc:program:v1:");
        for action in actions {
            h.update(action.commitment());
        }
        let out = h.finalize();
        let mut program_hash = [0u8; 32];
        program_hash.copy_from_slice(&out);

        // Build the bundle with a salt derived from the program hash.
        let mut salt_bytes = [0u8; 32];
        salt_bytes.copy_from_slice(&program_hash);
        let bundle = Bundle {
            salt: H256(salt_bytes),
            instructions,
        };

        Ok(IxlBundle {
            bundle,
            program_hash,
        })
    }

    fn domain_to_kind(domain: Domain) -> AssetKind {
        match domain {
            Domain::X3Native => AssetKind::X3Native,
            Domain::X3Evm => AssetKind::X3Evm,
            Domain::X3Svm => AssetKind::X3Svm,
        }
    }

    fn action_to_instruction(action: &Action, slot_id: u32) -> Instruction {
        match action {
            Action::Lock {
                asset_id,
                amount,
                domain,
            } => Instruction::Lock {
                slot_id,
                kind: Self::domain_to_kind(*domain),
                asset: {
                    let mut a = [0u8; 32];
                    a[..4].copy_from_slice(&asset_id.to_le_bytes());
                    a
                },
                payer: ZERO_ADDR,
                amount: *amount,
            },
            Action::Mint {
                asset_id,
                amount,
                domain,
            } => Instruction::Mint {
                slot_id,
                kind: Self::domain_to_kind(*domain),
                asset: {
                    let mut a = [0u8; 32];
                    a[..4].copy_from_slice(&asset_id.to_le_bytes());
                    a
                },
                receiver: ZERO_ADDR,
                amount: *amount,
            },
            Action::Burn { .. } => Instruction::Burn { slot_id },
            Action::Swap {
                asset_in,
                asset_out,
                amount_in,
                min_out,
                domain,
            } => {
                let mut a_in = [0u8; 32];
                a_in[..4].copy_from_slice(&asset_in.to_le_bytes());
                let mut a_out = [0u8; 32];
                a_out[..4].copy_from_slice(&asset_out.to_le_bytes());
                Instruction::Swap {
                    slot_id,
                    kind: Self::domain_to_kind(*domain),
                    asset_in: a_in,
                    asset_out: a_out,
                    amount_in: *amount_in,
                    min_out: *min_out,
                }
            }
            Action::Settle { packet_id } => Instruction::Settle {
                slot_id,
                kind: AssetKind::X3Native,
                receiver: *packet_id,
            },
            Action::EmitProof { asset_id } => {
                let mut a = [0u8; 32];
                a[..4].copy_from_slice(&asset_id.to_le_bytes());
                Instruction::EmitProof {
                    commitment: H256(a),
                }
            }
            Action::Refund { .. } => Instruction::Refund { slot_id },
            Action::Abort { .. } => Instruction::Abort,
        }
    }
}
