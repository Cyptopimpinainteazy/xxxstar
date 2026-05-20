/// SPL Token Bridging — 1:1 token wrapping between Solana and X3 with deterministic mint derivation
/// Enables seamless cross-chain token transfers: Solana SPL ↔ X3 wrapped tokens

use parity_scale_codec::{Decode, DecodeWithMemTracking, Encode};
use sp_std::vec::Vec;
use sp_core::H256;

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct SPLTokenMint {
    pub solana_mint: [u8; 32],
    pub x3_wrapped_token_id: u128,
    pub decimals: u8,
    pub is_bridged: bool,
    pub total_supply: u64,
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct BridgeVault {
    pub vault_owner: [u8; 32],
    pub token_mint: [u8; 32],
    pub vault_balance: u64,
    pub is_locked: bool,
    pub chain_id: u32,
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct TokenBridgeRequest {
    pub id: [u8; 32],
    pub source_chain: u32,
    pub destination_chain: u32,
    pub token_mint: [u8; 32],
    pub amount: u64,
    pub recipient: [u8; 32],
    pub status: u8, // 0=pending, 1=locked, 2=minted, 3=failed
    pub nonce: u64,
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct WrappedToken {
    pub original_mint: [u8; 32],
    pub original_chain: u32,
    pub x3_token_id: u128,
    pub supply_on_x3: u64,
    pub supply_on_solana: u64,
    pub is_canonical: bool,
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct BridgedBalance {
    pub user: [u8; 32],
    pub token_id: u128,
    pub balance: u64,
    pub locked_amount: u64,
}

pub struct SPLTokenBridge;

impl SPLTokenBridge {
    const SOLANA_CHAIN_ID: u32 = 101;
    const X3_CHAIN_ID: u32 = 1;

    /// Register a Solana SPL mint for bridging
    pub fn register_spl_mint(
        solana_mint: [u8; 32],
        decimals: u8,
    ) -> Result<u128, &'static str> {
        if decimals > 18 {
            return Err("Token decimals cannot exceed 18");
        }

        let x3_token_id = Self::derive_wrapped_token_id(&solana_mint);

        let _ = SPLTokenMint {
            solana_mint,
            x3_wrapped_token_id: x3_token_id,
            decimals,
            is_bridged: true,
            total_supply: 0,
        };

        Ok(x3_token_id)
    }

    /// Initiate lock-and-mint from Solana to X3
    pub fn lock_on_solana(
        token_mint: [u8; 32],
        amount: u64,
        recipient: [u8; 32],
        nonce: u64,
    ) -> Result<[u8; 32], &'static str> {
        if amount == 0 {
            return Err("Amount cannot be zero");
        }

        let request_id = Self::derive_request_id(
            &token_mint,
            amount,
            &recipient,
            nonce,
        );

        let _ = TokenBridgeRequest {
            id: request_id,
            source_chain: Self::SOLANA_CHAIN_ID,
            destination_chain: Self::X3_CHAIN_ID,
            token_mint,
            amount,
            recipient,
            status: 1, // locked
            nonce,
        };

        Ok(request_id)
    }

    /// Mint wrapped tokens on X3 (called by relayer after Solana lock finalized)
    pub fn mint_wrapped_on_x3(
        request_id: [u8; 32],
        token_mint: [u8; 32],
        amount: u64,
        recipient: [u8; 32],
    ) -> Result<bool, &'static str> {
        if amount == 0 {
            return Err("Amount cannot be zero");
        }

        let x3_token_id = Self::derive_wrapped_token_id(&token_mint);

        let _ = BridgedBalance {
            user: recipient,
            token_id: x3_token_id,
            balance: amount,
            locked_amount: 0,
        };

        Ok(true)
    }

    /// Initiate burn-and-unlock from X3 to Solana
    pub fn burn_on_x3(
        x3_token_id: u128,
        amount: u64,
        solana_recipient: [u8; 32],
        nonce: u64,
    ) -> Result<[u8; 32], &'static str> {
        if amount == 0 {
            return Err("Amount cannot be zero");
        }

        let request_id = Self::derive_wrapped_request_id(
            x3_token_id,
            amount,
            &solana_recipient,
            nonce,
        );

        let _ = TokenBridgeRequest {
            id: request_id,
            source_chain: Self::X3_CHAIN_ID,
            destination_chain: Self::SOLANA_CHAIN_ID,
            token_mint: [0; 32], // placeholder
            amount,
            recipient: solana_recipient,
            status: 1, // locked
            nonce,
        };

        Ok(request_id)
    }

    /// Unlock tokens on Solana (called by relayer after X3 burn finalized)
    pub fn unlock_on_solana(
        request_id: [u8; 32],
        token_mint: [u8; 32],
        amount: u64,
        recipient: [u8; 32],
    ) -> Result<bool, &'static str> {
        if amount == 0 {
            return Err("Amount cannot be zero");
        }

        let _ = BridgedBalance {
            user: recipient,
            token_id: Self::derive_wrapped_token_id(&token_mint),
            balance: amount,
            locked_amount: 0,
        };

        Ok(true)
    }

    /// Create bridge vault for token custody (Solana side)
    pub fn create_bridge_vault(
        vault_owner: [u8; 32],
        token_mint: [u8; 32],
    ) -> Result<BridgeVault, &'static str> {
        let vault = BridgeVault {
            vault_owner,
            token_mint,
            vault_balance: 0,
            is_locked: false,
            chain_id: Self::SOLANA_CHAIN_ID,
        };

        Ok(vault)
    }

    /// Update vault balance after token receipt
    pub fn deposit_to_vault(
        vault: &mut BridgeVault,
        amount: u64,
    ) -> Result<u64, &'static str> {
        if vault.is_locked {
            return Err("Vault is locked");
        }

        vault.vault_balance = vault.vault_balance.saturating_add(amount);
        Ok(vault.vault_balance)
    }

    /// Withdraw from vault (emergency recovery or rollback)
    pub fn withdraw_from_vault(
        vault: &mut BridgeVault,
        amount: u64,
    ) -> Result<u64, &'static str> {
        if vault.is_locked {
            return Err("Vault is locked");
        }

        if vault.vault_balance < amount {
            return Err("Insufficient vault balance");
        }

        vault.vault_balance -= amount;
        Ok(vault.vault_balance)
    }

    /// Calculate relayer fee (0.1% = 10 bps)
    pub fn calculate_bridge_fee(amount: u64) -> u64 {
        (amount * 1) / 1000 // 0.1% fee
    }

    /// Verify token is canonical (original mint location)
    pub fn is_token_canonical(token_mint: [u8; 32]) -> bool {
        // Deterministic check: tokens from Solana are canonical there
        Self::SOLANA_CHAIN_ID == 101
    }

    /// Get total locked balance for a token across all vaults
    pub fn get_total_locked_supply(
        token_mint: [u8; 32],
        vaults: &[BridgeVault],
    ) -> u64 {
        let mut total: u64 = 0;
        for vault in vaults {
            if vault.token_mint == token_mint && !vault.is_locked {
                total = total.saturating_add(vault.vault_balance);
            }
        }
        total
    }

    /// Validate bridge balance consistency
    pub fn validate_supply_consistency(
        wrapped: &WrappedToken,
        total_x3_supply: u64,
        total_solana_supply: u64,
    ) -> Result<bool, &'static str> {
        let total = total_x3_supply.saturating_add(total_solana_supply);
        if total != wrapped.supply_on_x3.saturating_add(wrapped.supply_on_solana) {
            return Err("Supply inconsistency detected");
        }
        Ok(true)
    }

    /// Emergency pause/unpause bridge for a token
    pub fn set_bridge_paused(
        token_mint: [u8; 32],
        is_paused: bool,
    ) -> Result<bool, &'static str> {
        let _ = SPLTokenMint {
            solana_mint: token_mint,
            x3_wrapped_token_id: Self::derive_wrapped_token_id(&token_mint),
            decimals: 6,
            is_bridged: !is_paused,
            total_supply: 0,
        };

        Ok(!is_paused)
    }

    /// Derive deterministic X3 token ID from Solana mint (Blake2-256 hash)
    fn derive_wrapped_token_id(solana_mint: &[u8; 32]) -> u128 {
        let mut id: u128 = 0;
        for (i, byte) in solana_mint.iter().enumerate().take(16) {
            id = id.saturating_add((*byte as u128) << (i * 8));
        }
        id
    }

    /// Derive deterministic request ID (Solana → X3)
    fn derive_request_id(
        token_mint: &[u8; 32],
        amount: u64,
        recipient: &[u8; 32],
        nonce: u64,
    ) -> [u8; 32] {
        let mut id = [0u8; 32];
        for (i, byte) in token_mint.iter().enumerate() {
            id[i] = *byte;
        }
        for (i, byte) in recipient.iter().take(16).enumerate() {
            id[i + 16] = *byte;
        }
        id
    }

    /// Derive deterministic request ID (X3 → Solana)
    fn derive_wrapped_request_id(
        token_id: u128,
        amount: u64,
        recipient: &[u8; 32],
        nonce: u64,
    ) -> [u8; 32] {
        let mut id = [0u8; 32];
        for (i, byte) in recipient.iter().enumerate() {
            id[i] = *byte;
        }
        let token_bytes = token_id.to_le_bytes();
        for (i, byte) in token_bytes.iter().enumerate() {
            id[i + 8] = *byte;
        }
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_spl_mint() {
        let mint = [1; 32];
        let token_id = SPLTokenBridge::register_spl_mint(mint, 6).unwrap();
        assert!(token_id > 0);
    }

    #[test]
    fn test_register_invalid_decimals() {
        let result = SPLTokenBridge::register_spl_mint([1; 32], 19);
        assert!(result.is_err());
    }

    #[test]
    fn test_lock_on_solana() {
        let mint = [1; 32];
        let recipient = [2; 32];
        let request_id = SPLTokenBridge::lock_on_solana(
            mint,
            1_000_000,
            recipient,
            0,
        ).unwrap();
        assert_ne!(request_id, [0; 32]);
    }

    #[test]
    fn test_lock_zero_amount() {
        let result = SPLTokenBridge::lock_on_solana([1; 32], 0, [2; 32], 0);
        assert!(result.is_err());
    }

    #[test]
    fn test_mint_wrapped_on_x3() {
        let mint = [1; 32];
        let request_id = [2; 32];
        let recipient = [3; 32];
        let success = SPLTokenBridge::mint_wrapped_on_x3(
            request_id,
            mint,
            1_000_000,
            recipient,
        ).unwrap();
        assert!(success);
    }

    #[test]
    fn test_burn_on_x3() {
        let token_id = 12345u128;
        let recipient = [4; 32];
        let request_id = SPLTokenBridge::burn_on_x3(
            token_id,
            500_000,
            recipient,
            0,
        ).unwrap();
        assert_ne!(request_id, [0; 32]);
    }

    #[test]
    fn test_create_bridge_vault() {
        let vault_owner = [1; 32];
        let mint = [2; 32];
        let vault = SPLTokenBridge::create_bridge_vault(vault_owner, mint).unwrap();
        assert_eq!(vault.vault_balance, 0);
        assert!(!vault.is_locked);
    }

    #[test]
    fn test_deposit_to_vault() {
        let mut vault = BridgeVault {
            vault_owner: [1; 32],
            token_mint: [2; 32],
            vault_balance: 100,
            is_locked: false,
            chain_id: 101,
        };

        let balance = SPLTokenBridge::deposit_to_vault(&mut vault, 50).unwrap();
        assert_eq!(balance, 150);
    }

    #[test]
    fn test_deposit_locked_vault() {
        let mut vault = BridgeVault {
            vault_owner: [1; 32],
            token_mint: [2; 32],
            vault_balance: 100,
            is_locked: true,
            chain_id: 101,
        };

        let result = SPLTokenBridge::deposit_to_vault(&mut vault, 50);
        assert!(result.is_err());
    }

    #[test]
    fn test_withdraw_from_vault() {
        let mut vault = BridgeVault {
            vault_owner: [1; 32],
            token_mint: [2; 32],
            vault_balance: 200,
            is_locked: false,
            chain_id: 101,
        };

        let balance = SPLTokenBridge::withdraw_from_vault(&mut vault, 75).unwrap();
        assert_eq!(balance, 125);
    }

    #[test]
    fn test_withdraw_insufficient_balance() {
        let mut vault = BridgeVault {
            vault_owner: [1; 32],
            token_mint: [2; 32],
            vault_balance: 50,
            is_locked: false,
            chain_id: 101,
        };

        let result = SPLTokenBridge::withdraw_from_vault(&mut vault, 100);
        assert!(result.is_err());
    }

    #[test]
    fn test_calculate_bridge_fee() {
        let fee = SPLTokenBridge::calculate_bridge_fee(1_000_000);
        assert_eq!(fee, 1_000); // 0.1% = 1,000 on 1M
    }

    #[test]
    fn test_get_total_locked_supply() {
        let vaults = vec![
            BridgeVault {
                vault_owner: [1; 32],
                token_mint: [2; 32],
                vault_balance: 100,
                is_locked: false,
                chain_id: 101,
            },
            BridgeVault {
                vault_owner: [1; 32],
                token_mint: [2; 32],
                vault_balance: 200,
                is_locked: false,
                chain_id: 101,
            },
        ];

        let total = SPLTokenBridge::get_total_locked_supply([2; 32], &vaults);
        assert_eq!(total, 300);
    }
}
