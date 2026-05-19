//! Collateral / Bonding module (skeleton)
//!
//! Minimal CollateralManager trait and an in-pallet in-memory implementation to
//! start integration work. Includes unit tests demonstrating deposit/withdraw/
//! slashing flows. This is a lightweight, safe starting point and should be
//! expanded into a full FRAME pallet storage design as part of the ADR work.

use codec::{Decode, DecodeWithMemTracking, Encode};
use core::fmt::Debug;
use scale_info::TypeInfo;
use sp_core::H256;
use sp_std::vec::Vec;

/// Bond types used for different policies
#[derive(Encode, Decode, DecodeWithMemTracking, Clone, Copy, PartialEq, Eq, Debug, TypeInfo)]
pub enum BondType {
    InitialMargin,
    MaintenanceMargin,
    PerformanceBond,
}

/// Bond state
#[derive(Encode, Decode, DecodeWithMemTracking, Clone, Copy, PartialEq, Eq, Debug, TypeInfo)]
pub enum BondState {
    Locked,
    Withdrawable,
    Slashed,
}

/// Simple bond record
#[derive(Encode, Decode, DecodeWithMemTracking, Clone, PartialEq, Eq, Debug, TypeInfo)]
pub struct Bond<AccountId, Balance> {
    pub id: H256,
    pub owner: AccountId,
    pub asset: Vec<u8>,
    pub amount: Balance,
    pub bond_type: BondType,
    pub state: BondState,
}

/// Errors for collateral operations
#[derive(Debug, PartialEq, Eq)]
pub enum CollateralError {
    InsufficientBalance,
    BondNotFound,
    AlreadyWithdrawn,
    NotWithdrawable,
}

/// A minimal trait describing collateral operations. Concrete runtime
/// implementations should use FRAME StorageMap for persistence and security.
pub trait CollateralManager<AccountId, Balance> {
    fn deposit_bond(
        &mut self,
        who: AccountId,
        asset: Vec<u8>,
        amount: Balance,
        bond_type: BondType,
    ) -> Result<H256, CollateralError>;

    fn request_withdraw(&mut self, bond_id: H256) -> Result<(), CollateralError>;

    fn finalize_withdraw(&mut self, bond_id: H256) -> Result<(), CollateralError>;

    fn slash(&mut self, bond_id: H256, amount: Balance) -> Result<(), CollateralError>;

    fn get_bond(&self, bond_id: H256) -> Option<&Bond<AccountId, Balance>>;
}

/// In-memory collateral manager used for unit tests and PoC.
pub struct InMemoryCollateral<AccountId, Balance> {
    pub bonds: Vec<Bond<AccountId, Balance>>,
}

impl<
        AccountId: Clone + PartialEq,
        Balance: Copy + PartialOrd + core::ops::Sub<Output = Balance> + Default,
    > InMemoryCollateral<AccountId, Balance>
{
    pub fn new() -> Self {
        Self { bonds: Vec::new() }
    }
}

impl<
        AccountId: Clone + PartialEq,
        Balance: Copy + PartialOrd + core::ops::Sub<Output = Balance> + Default,
    > CollateralManager<AccountId, Balance> for InMemoryCollateral<AccountId, Balance>
{
    fn deposit_bond(
        &mut self,
        who: AccountId,
        asset: Vec<u8>,
        amount: Balance,
        bond_type: BondType,
    ) -> Result<H256, CollateralError> {
        // For PoC we accept any non-zero amount
        // Real impl must check balances, reserve funds via Currency trait
        let idx = self.bonds.len() as u64;
        let mut id_bytes = [0u8; 32];
        id_bytes[..8].copy_from_slice(&idx.to_le_bytes());
        let id = H256::from(id_bytes);
        let bond = Bond {
            id,
            owner: who,
            asset,
            amount,
            bond_type,
            state: BondState::Locked,
        };
        self.bonds.push(bond);
        Ok(id)
    }

    fn request_withdraw(&mut self, bond_id: H256) -> Result<(), CollateralError> {
        let b = self
            .bonds
            .iter_mut()
            .find(|b| b.id == bond_id)
            .ok_or(CollateralError::BondNotFound)?;
        if b.state == BondState::Locked {
            b.state = BondState::Withdrawable;
            Ok(())
        } else {
            Err(CollateralError::NotWithdrawable)
        }
    }

    fn finalize_withdraw(&mut self, bond_id: H256) -> Result<(), CollateralError> {
        let idx = self
            .bonds
            .iter()
            .position(|b| b.id == bond_id)
            .ok_or(CollateralError::BondNotFound)?;
        let b = &self.bonds[idx];
        if b.state != BondState::Withdrawable {
            return Err(CollateralError::NotWithdrawable);
        }
        // Remove bond (simulate transfer of funds)
        self.bonds.remove(idx);
        Ok(())
    }

    fn slash(&mut self, bond_id: H256, _amount: Balance) -> Result<(), CollateralError> {
        let b = self
            .bonds
            .iter_mut()
            .find(|b| b.id == bond_id)
            .ok_or(CollateralError::BondNotFound)?;
        b.state = BondState::Slashed;
        Ok(())
    }

    fn get_bond(&self, bond_id: H256) -> Option<&Bond<AccountId, Balance>> {
        self.bonds.iter().find(|b| b.id == bond_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deposit_and_withdraw_flow() {
        let mut c = InMemoryCollateral::<u64, u128>::new();
        let id = c
            .deposit_bond(1u64, b"USDC".to_vec(), 1_000u128, BondType::PerformanceBond)
            .unwrap();
        let b = c.get_bond(id).unwrap();
        assert_eq!(b.amount, 1_000u128);
        assert_eq!(b.state, BondState::Locked);

        // Request withdraw
        assert!(c.request_withdraw(id).is_ok());
        let b2 = c.get_bond(id).unwrap();
        assert_eq!(b2.state, BondState::Withdrawable);

        // Finalize withdraw
        assert!(c.finalize_withdraw(id).is_ok());
        assert!(c.get_bond(id).is_none());
    }

    #[test]
    fn test_slash_flow() {
        let mut c = InMemoryCollateral::<u64, u128>::new();
        let id = c
            .deposit_bond(2u64, b"USDC".to_vec(), 5_000u128, BondType::InitialMargin)
            .unwrap();
        assert!(c.slash(id, 1_000u128).is_ok());
        let b = c.get_bond(id).unwrap();
        assert_eq!(b.state, BondState::Slashed);
    }
}
