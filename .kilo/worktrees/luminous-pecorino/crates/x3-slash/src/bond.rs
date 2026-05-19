//! Bond management — escrow, locking, release, and forfeiture.

use crate::error::SlashError;
use crate::types::*;
use std::collections::HashMap;
use x3_proof::types::{AgentIdentity, BlockHeight, IntentId};

/// Manages all bonds in the system.
pub struct BondManager {
    /// Active bonds indexed by ID.
    bonds: HashMap<BondId, Bond>,
    /// Bonds indexed by agent.
    agent_bonds: HashMap<[u8; 32], Vec<BondId>>,
    /// Next bond ID.
    next_id: u64,
    /// Configuration.
    config: SlashConfig,
}

impl BondManager {
    /// Create a new bond manager.
    pub fn new(config: SlashConfig) -> Self {
        Self {
            bonds: HashMap::new(),
            agent_bonds: HashMap::new(),
            next_id: 0,
            config,
        }
    }

    /// Post a new bond. Returns the bond ID.
    /// Bond amount must meet minimum requirements.
    pub fn post_bond(
        &mut self,
        agent_id: AgentIdentity,
        amount: Amount,
        current_block: BlockHeight,
        intent_id: Option<IntentId>,
    ) -> Result<BondId, SlashError> {
        if amount < self.config.min_bond {
            return Err(SlashError::InsufficientBond {
                required: self.config.min_bond,
                provided: amount,
            });
        }

        let id = BondId(self.next_id);
        self.next_id += 1;

        let bond = Bond {
            id,
            agent_id: agent_id.clone(),
            amount,
            posted_at: current_block,
            expires_at: current_block + self.config.finality_window,
            intent_id,
            status: BondStatus::Active,
        };

        self.bonds.insert(id, bond);
        self.agent_bonds
            .entry(agent_id.pubkey)
            .or_default()
            .push(id);

        Ok(id)
    }

    /// Slash a bond by the given severity.
    /// Returns the amount slashed.
    pub fn slash(
        &mut self,
        bond_id: BondId,
        severity: SlashSeverity,
    ) -> Result<Amount, SlashError> {
        let bond = self
            .bonds
            .get_mut(&bond_id)
            .ok_or(SlashError::BondNotFound(bond_id))?;

        match bond.status {
            BondStatus::Active | BondStatus::PartiallySlashed { .. } => {}
            _ => return Err(SlashError::BondNotSlashable(bond_id)),
        }

        let current_amount = match bond.status {
            BondStatus::PartiallySlashed { remaining_bps } => {
                (bond.amount * remaining_bps as u128) / 10000
            }
            _ => bond.amount,
        };

        let slash_amount = (current_amount * severity.slash_bps() as u128) / 10000;

        if severity.slash_bps() >= 10000 {
            bond.status = BondStatus::FullySlashed;
        } else {
            let remaining = match bond.status {
                BondStatus::PartiallySlashed { remaining_bps } => {
                    remaining_bps - (remaining_bps * severity.slash_bps()) / 10000
                }
                _ => 10000 - severity.slash_bps(),
            };
            bond.status = BondStatus::PartiallySlashed {
                remaining_bps: remaining,
            };
        }

        Ok(slash_amount)
    }

    /// Release a bond back to the agent (successful execution).
    pub fn release(&mut self, bond_id: BondId) -> Result<Amount, SlashError> {
        let bond = self
            .bonds
            .get_mut(&bond_id)
            .ok_or(SlashError::BondNotFound(bond_id))?;

        match bond.status {
            BondStatus::Active => {
                bond.status = BondStatus::Released;
                Ok(bond.amount)
            }
            BondStatus::PartiallySlashed { remaining_bps } => {
                bond.status = BondStatus::Released;
                Ok((bond.amount * remaining_bps as u128) / 10000)
            }
            _ => Err(SlashError::BondNotReleasable(bond_id)),
        }
    }

    /// Check for and process expired bonds.
    pub fn process_expiries(&mut self, current_block: BlockHeight) -> Vec<BondId> {
        let mut expired = Vec::new();
        for (id, bond) in &mut self.bonds {
            if bond.status == BondStatus::Active
                && current_block > bond.expires_at + self.config.expiry_grace_period
            {
                bond.status = BondStatus::Expired;
                expired.push(*id);
            }
        }
        expired
    }

    /// Get a bond by ID.
    pub fn get(&self, bond_id: BondId) -> Option<&Bond> {
        self.bonds.get(&bond_id)
    }

    /// Get all bonds for an agent.
    pub fn agent_bonds(&self, pubkey: &[u8; 32]) -> Vec<&Bond> {
        self.agent_bonds
            .get(pubkey)
            .map(|ids| ids.iter().filter_map(|id| self.bonds.get(id)).collect())
            .unwrap_or_default()
    }

    /// Get total active bond amount for an agent.
    pub fn agent_total_bonded(&self, pubkey: &[u8; 32]) -> Amount {
        self.agent_bonds(pubkey)
            .iter()
            .filter(|b| matches!(b.status, BondStatus::Active))
            .map(|b| b.amount)
            .sum()
    }

    /// Get the config.
    pub fn config(&self) -> &SlashConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_agent() -> AgentIdentity {
        AgentIdentity {
            pubkey: [1u8; 32],
            ephemeral: false,
        }
    }

    #[test]
    fn test_post_and_release_bond() {
        let mut mgr = BondManager::new(SlashConfig::default());
        let bond_id = mgr.post_bond(test_agent(), 2_000_000, 100, None).unwrap();

        let released = mgr.release(bond_id).unwrap();
        assert_eq!(released, 2_000_000);
    }

    #[test]
    fn test_insufficient_bond() {
        let mut mgr = BondManager::new(SlashConfig::default());
        let result = mgr.post_bond(test_agent(), 100, 100, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_slash_bond() {
        let mut mgr = BondManager::new(SlashConfig::default());
        let bond_id = mgr.post_bond(test_agent(), 2_000_000, 100, None).unwrap();

        let slashed = mgr.slash(bond_id, SlashSeverity::Moderate).unwrap();
        assert_eq!(slashed, 1_000_000); // 50%

        let bond = mgr.get(bond_id).unwrap();
        assert!(matches!(bond.status, BondStatus::PartiallySlashed { .. }));
    }

    #[test]
    fn test_full_slash() {
        let mut mgr = BondManager::new(SlashConfig::default());
        let bond_id = mgr.post_bond(test_agent(), 2_000_000, 100, None).unwrap();

        mgr.slash(bond_id, SlashSeverity::Major).unwrap();
        let bond = mgr.get(bond_id).unwrap();
        assert_eq!(bond.status, BondStatus::FullySlashed);
    }

    #[test]
    fn test_expiry_processing() {
        let mut mgr = BondManager::new(SlashConfig {
            finality_window: 10,
            expiry_grace_period: 5,
            ..Default::default()
        });
        mgr.post_bond(test_agent(), 2_000_000, 100, None).unwrap();

        // Not expired yet
        assert!(mgr.process_expiries(110).is_empty());

        // Expired beyond grace period
        let expired = mgr.process_expiries(116);
        assert_eq!(expired.len(), 1);
    }
}
