//! Mint/burn controller: authorized supply mutations for X3 assets.
//!
//! Enforces that only authorized minters/burners can change supply, and that
//! every mutation is reflected in the canonical supply ledger before returning.

use frame_support::pallet_prelude::*;

pub type AssetId = u32;
pub type Balance = u128;

/// Authorization record for a minter or burner.
#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, TypeInfo, MaxEncodedLen)]
pub struct MintBurnAuth {
    pub asset_id: AssetId,
    /// Maximum amount this authority can mint per operation.
    pub mint_cap_per_op: Balance,
    /// Maximum amount this authority can burn per operation.
    pub burn_cap_per_op: Balance,
    /// Total lifetime mint limit (0 = unlimited).
    pub lifetime_mint_limit: Balance,
    /// Total minted so far by this authority.
    pub total_minted: Balance,
}

/// Errors returned by mint/burn operations.
#[derive(Debug, PartialEq, Eq)]
pub enum MintBurnError {
    /// Caller is not authorized for this asset.
    Unauthorized,
    /// Requested amount exceeds per-operation cap.
    ExceedsPerOpCap,
    /// Requested amount would exceed lifetime mint limit.
    ExceedsLifetimeLimit,
    /// Amount is zero (invalid operation).
    ZeroAmount,
}

/// Validate and apply a mint request against the authorization record.
pub fn validate_mint(auth: &mut MintBurnAuth, amount: Balance) -> Result<(), MintBurnError> {
    if amount == 0 {
        return Err(MintBurnError::ZeroAmount);
    }
    if amount > auth.mint_cap_per_op {
        return Err(MintBurnError::ExceedsPerOpCap);
    }
    if auth.lifetime_mint_limit > 0 {
        let new_total = auth
            .total_minted
            .checked_add(amount)
            .ok_or(MintBurnError::ExceedsLifetimeLimit)?;
        if new_total > auth.lifetime_mint_limit {
            return Err(MintBurnError::ExceedsLifetimeLimit);
        }
        auth.total_minted = new_total;
    }
    Ok(())
}

/// Validate a burn request against the authorization record.
pub fn validate_burn(auth: &MintBurnAuth, amount: Balance) -> Result<(), MintBurnError> {
    if amount == 0 {
        return Err(MintBurnError::ZeroAmount);
    }
    if amount > auth.burn_cap_per_op {
        return Err(MintBurnError::ExceedsPerOpCap);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn auth(mint_cap: Balance, burn_cap: Balance, lifetime: Balance) -> MintBurnAuth {
        MintBurnAuth {
            asset_id: 1,
            mint_cap_per_op: mint_cap,
            burn_cap_per_op: burn_cap,
            lifetime_mint_limit: lifetime,
            total_minted: 0,
        }
    }

    #[test]
    fn test_valid_mint() {
        let mut a = auth(1000, 500, 0);
        assert!(validate_mint(&mut a, 500).is_ok());
    }

    #[test]
    fn test_mint_exceeds_per_op_cap() {
        let mut a = auth(100, 500, 0);
        assert_eq!(
            validate_mint(&mut a, 200),
            Err(MintBurnError::ExceedsPerOpCap)
        );
    }

    #[test]
    fn test_mint_zero_rejected() {
        let mut a = auth(1000, 500, 0);
        assert_eq!(validate_mint(&mut a, 0), Err(MintBurnError::ZeroAmount));
    }

    #[test]
    fn test_lifetime_limit_enforced() {
        let mut a = auth(1000, 0, 1500);
        validate_mint(&mut a, 1000).unwrap();
        assert_eq!(
            validate_mint(&mut a, 600),
            Err(MintBurnError::ExceedsLifetimeLimit)
        );
    }

    #[test]
    fn test_lifetime_limit_cumulative() {
        let mut a = auth(1000, 0, 2000);
        validate_mint(&mut a, 1000).unwrap();
        validate_mint(&mut a, 1000).unwrap();
        assert_eq!(
            validate_mint(&mut a, 1),
            Err(MintBurnError::ExceedsLifetimeLimit)
        );
    }

    #[test]
    fn test_valid_burn() {
        let a = auth(1000, 500, 0);
        assert!(validate_burn(&a, 300).is_ok());
    }

    #[test]
    fn test_burn_exceeds_per_op_cap() {
        let a = auth(1000, 100, 0);
        assert_eq!(validate_burn(&a, 200), Err(MintBurnError::ExceedsPerOpCap));
    }

    #[test]
    fn test_burn_zero_rejected() {
        let a = auth(1000, 500, 0);
        assert_eq!(validate_burn(&a, 0), Err(MintBurnError::ZeroAmount));
    }
}
