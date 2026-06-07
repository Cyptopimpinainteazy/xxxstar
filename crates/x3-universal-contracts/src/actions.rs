//! Action definitions — the primitive operations that a Universal Contract may perform.
//!
//! Each `Action` maps to a single IXL instruction plus the metadata the SDK
//! needs to build the packet and compute the proof commitment.

use crate::error::UcError;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Domain-chain selector used as routing hint in actions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Domain {
    X3Native,
    X3Evm,
    X3Svm,
}

impl Domain {
    pub fn as_u8(self) -> u8 {
        match self {
            Domain::X3Native => 0,
            Domain::X3Evm => 1,
            Domain::X3Svm => 2,
        }
    }
}

/// A single primitive action within a Universal Contract.
///
/// Actions are ordered and executed atomically — if any action fails, the
/// entire bundle is rolled back via the IXL rollback mechanism.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Action {
    /// Lock `amount` of `asset_id` on `domain`.
    Lock {
        asset_id: u32,
        amount: u128,
        domain: Domain,
    },
    /// Mint `amount` of `asset_id` on `domain` (governance-gated).
    Mint {
        asset_id: u32,
        amount: u128,
        domain: Domain,
    },
    /// Burn `amount` of `asset_id` on `domain`.
    Burn {
        asset_id: u32,
        amount: u128,
        domain: Domain,
    },
    /// Execute a spot swap: sell `amount_in` of `asset_in`, receive at least
    /// `min_out` of `asset_out`, on `domain`.
    Swap {
        asset_in: u32,
        asset_out: u32,
        amount_in: u128,
        min_out: u128,
        domain: Domain,
    },
    /// Settle a cross-VM transfer proof identified by `packet_id`.
    Settle { packet_id: [u8; 32] },
    /// Emit a supply proof for `asset_id`.
    EmitProof { asset_id: u32 },
    /// Refund `amount` of `asset_id` to `recipient` on `domain`.
    Refund {
        asset_id: u32,
        amount: u128,
        recipient: [u8; 32],
        domain: Domain,
    },
    /// Abort the bundle with a human-readable `reason` (used in conditional logic).
    Abort { reason: [u8; 32] },
}

impl Action {
    /// Domain-separated commitment hash for this action.
    ///
    /// Used when computing the bundle commitment and packet proof hash.
    pub fn commitment(&self) -> [u8; 32] {
        let mut h = Sha256::new();
        h.update(b"x3:uc:action:v1:");
        match self {
            Action::Lock {
                asset_id,
                amount,
                domain,
            } => {
                h.update(b"Lock:");
                h.update(&asset_id.to_le_bytes());
                h.update(&amount.to_le_bytes());
                h.update(&[domain.as_u8()]);
            }
            Action::Mint {
                asset_id,
                amount,
                domain,
            } => {
                h.update(b"Mint:");
                h.update(&asset_id.to_le_bytes());
                h.update(&amount.to_le_bytes());
                h.update(&[domain.as_u8()]);
            }
            Action::Burn {
                asset_id,
                amount,
                domain,
            } => {
                h.update(b"Burn:");
                h.update(&asset_id.to_le_bytes());
                h.update(&amount.to_le_bytes());
                h.update(&[domain.as_u8()]);
            }
            Action::Swap {
                asset_in,
                asset_out,
                amount_in,
                min_out,
                domain,
            } => {
                h.update(b"Swap:");
                h.update(&asset_in.to_le_bytes());
                h.update(&asset_out.to_le_bytes());
                h.update(&amount_in.to_le_bytes());
                h.update(&min_out.to_le_bytes());
                h.update(&[domain.as_u8()]);
            }
            Action::Settle { packet_id } => {
                h.update(b"Settle:");
                h.update(packet_id);
            }
            Action::EmitProof { asset_id } => {
                h.update(b"EmitProof:");
                h.update(&asset_id.to_le_bytes());
            }
            Action::Refund {
                asset_id,
                amount,
                recipient,
                domain,
            } => {
                h.update(b"Refund:");
                h.update(&asset_id.to_le_bytes());
                h.update(&amount.to_le_bytes());
                h.update(recipient);
                h.update(&[domain.as_u8()]);
            }
            Action::Abort { reason } => {
                h.update(b"Abort:");
                h.update(reason);
            }
        }
        let out = h.finalize();
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&out);
        arr
    }

    /// Returns `true` if this action crosses VM boundaries.
    pub fn is_cross_vm(&self) -> bool {
        matches!(self, Action::Settle { .. })
    }

    /// Returns `true` if this action is terminal (ends the sequence regardless of outcome).
    pub fn is_terminal(&self) -> bool {
        matches!(self, Action::Abort { .. })
    }

    /// Validate the action's numeric invariants.
    pub fn validate(&self) -> Result<(), UcError> {
        match self {
            Action::Lock { amount, .. }
            | Action::Mint { amount, .. }
            | Action::Burn { amount, .. } => {
                if *amount == 0 {
                    return Err(UcError::ZeroAmount);
                }
            }
            Action::Swap {
                amount_in,
                min_out,
                asset_in,
                asset_out,
                ..
            } => {
                if *amount_in == 0 {
                    return Err(UcError::ZeroAmount);
                }
                if asset_in == asset_out {
                    return Err(UcError::SameAsset);
                }
                let _ = min_out; // 0 min_out is allowed (caller accepts any slippage)
            }
            Action::Refund { amount, .. } => {
                if *amount == 0 {
                    return Err(UcError::ZeroAmount);
                }
            }
            Action::Settle { .. } | Action::EmitProof { .. } | Action::Abort { .. } => {}
        }
        Ok(())
    }
}
