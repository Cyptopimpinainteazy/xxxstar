//! Shared types for the constitution module.

use serde::{Deserialize, Serialize};

/// A 32-byte SHA-256 hash of the canonical constitution text.
///
/// This is stored on-chain and verified before any invariant-touching operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ConstitutionHash(pub [u8; 32]);

impl ConstitutionHash {
    /// The zero hash — used as a sentinel for "not yet set".
    pub const ZERO: Self = Self([0u8; 32]);

    /// Returns the hex-encoded string of this hash.
    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }

    /// Parse from a 64-char hex string.
    pub fn from_hex(s: &str) -> Result<Self, hex::FromHexError> {
        let bytes = hex::decode(s)?;
        if bytes.len() != 32 {
            return Err(hex::FromHexError::InvalidStringLength);
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        Ok(Self(arr))
    }
}

impl std::fmt::Display for ConstitutionHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

/// Bounds on core system invariants.
///
/// These values define the outer limits of the constitutional guarantees.
/// They may only be narrowed (refined), never widened, via amendment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvariantBounds {
    /// Maximum total token supply (in smallest units).
    pub max_supply: u128,
    /// Maximum treasury balance as a fraction of total supply (0–100).
    pub max_treasury_pct: u8,
    /// Maximum number of registered agents at any time.
    pub max_agent_count: u64,
    /// Maximum governance proposal execution depth (prevents re-entrancy).
    pub max_proposal_depth: u8,
    /// Maximum budget a single agent may spend per epoch.
    pub max_agent_epoch_budget: u128,
}

impl Default for InvariantBounds {
    fn default() -> Self {
        Self {
            max_supply: 1_000_000_000 * 1_000_000_000_000_000_000u128, // 1B tokens * 10^18
            max_treasury_pct: 30,
            max_agent_count: 100_000,
            max_proposal_depth: 1,
            max_agent_epoch_budget: 1_000_000 * 1_000_000_000_000_000_000u128,
        }
    }
}

impl InvariantBounds {
    /// Check that `other` is a valid refinement (narrowing) of `self`.
    /// A refinement may only reduce limits, never increase them.
    pub fn is_refinement_of(&self, prior: &InvariantBounds) -> bool {
        self.max_supply <= prior.max_supply
            && self.max_treasury_pct <= prior.max_treasury_pct
            && self.max_agent_count <= prior.max_agent_count
            && self.max_proposal_depth <= prior.max_proposal_depth
            && self.max_agent_epoch_budget <= prior.max_agent_epoch_budget
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tighter_bounds_are_valid_refinement() {
        let prior = InvariantBounds::default();
        let tighter = InvariantBounds {
            max_supply: prior.max_supply / 2,
            max_treasury_pct: 20,
            max_agent_count: 50_000,
            max_proposal_depth: 1,
            max_agent_epoch_budget: prior.max_agent_epoch_budget / 10,
        };
        assert!(tighter.is_refinement_of(&prior));
    }

    #[test]
    fn wider_bounds_are_not_refinement() {
        let prior = InvariantBounds::default();
        let wider = InvariantBounds {
            max_supply: prior.max_supply * 2,
            ..prior.clone()
        };
        assert!(!wider.is_refinement_of(&prior));
    }

    #[test]
    fn constitution_hash_hex_round_trip() {
        let hash = ConstitutionHash([1u8; 32]);
        let hex = hash.to_hex();
        let back = ConstitutionHash::from_hex(&hex).unwrap();
        assert_eq!(hash, back);
    }
}
