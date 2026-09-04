/// Token Manager — ERC-20-like token tracking and whitelist management
/// Track balances, whitelist tokens, detect spam
use parity_scale_codec::{Decode, DecodeWithMemTracking, Encode};
use scale_info::TypeInfo;
#[allow(unused_imports)]
use sp_std::vec;
use sp_std::vec::Vec;

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, TypeInfo, Debug, PartialEq, Eq)]
pub struct Token {
    pub id: [u8; 32],
    pub address: [u8; 32],
    pub symbol: Vec<u8>, // max 10 bytes
    pub name: Vec<u8>,   // max 50 bytes
    pub decimals: u8,
    pub supply: u128,
    pub is_verified: bool,
    pub is_blacklisted: bool,
    pub owner: [u8; 32],
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, TypeInfo, Debug, PartialEq, Eq)]
pub struct TokenBalance {
    pub token_id: [u8; 32],
    pub holder: [u8; 32],
    pub balance: u128,
    pub last_updated_block: u64,
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, TypeInfo, Debug, PartialEq, Eq)]
pub struct TokenWhitelist {
    pub id: [u8; 32],
    pub owner: [u8; 32],
    pub whitelisted_tokens: Vec<[u8; 32]>,
    pub is_default_allow: bool, // whitelist mode or blacklist mode
    pub created_block: u64,
}

pub struct TokenManager;

impl TokenManager {
    /// Register a new token
    pub fn register_token(
        address: [u8; 32],
        symbol: Vec<u8>,
        name: Vec<u8>,
        decimals: u8,
        supply: u128,
        owner: [u8; 32],
    ) -> Result<Token, &'static str> {
        if symbol.is_empty() || symbol.len() > 10 {
            return Err("Invalid symbol length");
        }
        if name.is_empty() || name.len() > 50 {
            return Err("Invalid name length");
        }
        if decimals > 18 {
            return Err("Decimals too high");
        }

        let mut id = [0u8; 32];
        id[0..16].copy_from_slice(&address[0..16]);

        Ok(Token {
            id,
            address,
            symbol,
            name,
            decimals,
            supply,
            is_verified: false,
            is_blacklisted: false,
            owner,
        })
    }

    /// Verify token (mark as safe)
    pub fn verify_token(token: &mut Token, caller: [u8; 32]) -> Result<(), &'static str> {
        if caller != token.owner {
            return Err("Only owner can verify");
        }
        token.is_verified = true;
        Ok(())
    }

    /// Mark token as blacklisted (spam/scam)
    pub fn blacklist_token(token: &mut Token, caller: [u8; 32]) -> Result<(), &'static str> {
        if caller != token.owner {
            return Err("Only owner can blacklist");
        }
        token.is_blacklisted = true;
        Ok(())
    }

    /// Update token balance
    pub fn update_balance(
        token_id: [u8; 32],
        holder: [u8; 32],
        new_balance: u128,
        current_block: u64,
    ) -> TokenBalance {
        TokenBalance {
            token_id,
            holder,
            balance: new_balance,
            last_updated_block: current_block,
        }
    }

    /// Create token whitelist for user
    pub fn create_whitelist(
        owner: [u8; 32],
        allow_by_default: bool,
        current_block: u64,
    ) -> Result<TokenWhitelist, &'static str> {
        let mut id = [0u8; 32];
        id[0..16].copy_from_slice(&owner[0..16]);

        Ok(TokenWhitelist {
            id,
            owner,
            whitelisted_tokens: vec![],
            is_default_allow: allow_by_default,
            created_block: current_block,
        })
    }

    /// Add token to whitelist
    pub fn add_to_whitelist(
        whitelist: &mut TokenWhitelist,
        token_id: [u8; 32],
        caller: [u8; 32],
    ) -> Result<(), &'static str> {
        if caller != whitelist.owner {
            return Err("Only owner can modify whitelist");
        }
        if whitelist.whitelisted_tokens.contains(&token_id) {
            return Err("Token already in whitelist");
        }
        if whitelist.whitelisted_tokens.len() >= 100 {
            return Err("Whitelist full");
        }

        whitelist.whitelisted_tokens.push(token_id);
        Ok(())
    }

    /// Remove token from whitelist
    pub fn remove_from_whitelist(
        whitelist: &mut TokenWhitelist,
        token_id: [u8; 32],
        caller: [u8; 32],
    ) -> Result<(), &'static str> {
        if caller != whitelist.owner {
            return Err("Only owner can modify whitelist");
        }
        if !whitelist.whitelisted_tokens.contains(&token_id) {
            return Err("Token not in whitelist");
        }

        whitelist.whitelisted_tokens.retain(|t| t != &token_id);
        Ok(())
    }

    /// Check if token is allowed by whitelist
    pub fn is_token_allowed(whitelist: &TokenWhitelist, token_id: [u8; 32]) -> bool {
        if whitelist.is_default_allow {
            !whitelist.whitelisted_tokens.contains(&token_id) // default allow, blacklist items
        } else {
            whitelist.whitelisted_tokens.contains(&token_id) // default deny, whitelist items
        }
    }

    /// Detect if token might be spam
    pub fn is_token_likely_spam(token: &Token) -> bool {
        // Simple heuristic: unverified, blacklisted, low supply, or suspicious decimals
        if token.is_blacklisted {
            return true;
        }
        if token.supply == 0 {
            return true;
        }
        if !token.is_verified && token.symbol.len() == 1 {
            return true; // single-char unverified token is suspicious
        }
        false
    }

    /// Get token symbol as string
    pub fn get_symbol(token: &Token) -> Vec<u8> {
        token.symbol.clone()
    }

    /// Get token name as string
    pub fn get_name(token: &Token) -> Vec<u8> {
        token.name.clone()
    }

    /// Check token is valid before sending
    pub fn validate_token(token: &Token) -> Result<(), &'static str> {
        if token.is_blacklisted {
            return Err("Token is blacklisted");
        }
        if token.supply == 0 {
            return Err("Token has zero supply");
        }
        if token.symbol.is_empty() {
            return Err("Token has no symbol");
        }
        Ok(())
    }

    /// Transfer token (simplified)
    pub fn transfer_token(
        from: [u8; 32],
        to: [u8; 32],
        amount: u128,
        balance: &mut TokenBalance,
    ) -> Result<(), &'static str> {
        if amount == 0 {
            return Err("Transfer amount is zero");
        }
        if balance.balance < amount {
            return Err("Insufficient balance");
        }

        balance.balance -= amount;
        Ok(())
    }

    /// Get whitelist size
    pub fn get_whitelist_size(whitelist: &TokenWhitelist) -> usize {
        whitelist.whitelisted_tokens.len()
    }

    /// Check token in whitelist
    pub fn is_in_whitelist(whitelist: &TokenWhitelist, token_id: [u8; 32]) -> bool {
        whitelist.whitelisted_tokens.contains(&token_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_token() {
        let result = TokenManager::register_token(
            [1u8; 32],
            vec![85, 83, 68, 67],                        // "USDC"
            vec![85, 83, 68, 67, 32, 67, 111, 105, 110], // "USDC Coin"
            6,
            1_000_000_000,
            [2u8; 32],
        );
        assert!(result.is_ok());
        let token = result.unwrap();
        assert_eq!(token.decimals, 6);
        assert!(!token.is_verified);
    }

    #[test]
    fn test_register_token_invalid_symbol() {
        let result = TokenManager::register_token([1u8; 32], vec![], vec![1], 6, 1000, [2u8; 32]);
        assert!(result.is_err());
    }

    #[test]
    fn test_register_token_invalid_decimals() {
        let result = TokenManager::register_token(
            [1u8; 32],
            vec![1, 2, 3],
            vec![1, 2, 3],
            19,
            1000,
            [2u8; 32],
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_verify_token() {
        let mut token = TokenManager::register_token(
            [1u8; 32],
            vec![85, 83, 68, 67],
            vec![1, 2, 3],
            6,
            1000,
            [2u8; 32],
        )
        .unwrap();

        assert!(!token.is_verified);
        let result = TokenManager::verify_token(&mut token, [2u8; 32]);
        assert!(result.is_ok());
        assert!(token.is_verified);
    }

    #[test]
    fn test_verify_token_not_owner() {
        let mut token = TokenManager::register_token(
            [1u8; 32],
            vec![1, 2, 3],
            vec![1, 2, 3],
            6,
            1000,
            [2u8; 32],
        )
        .unwrap();

        let result = TokenManager::verify_token(&mut token, [99u8; 32]);
        assert!(result.is_err());
    }

    #[test]
    fn test_blacklist_token() {
        let mut token = TokenManager::register_token(
            [1u8; 32],
            vec![1, 2, 3],
            vec![1, 2, 3],
            6,
            1000,
            [2u8; 32],
        )
        .unwrap();

        let result = TokenManager::blacklist_token(&mut token, [2u8; 32]);
        assert!(result.is_ok());
        assert!(token.is_blacklisted);
    }

    #[test]
    fn test_create_whitelist() {
        let result = TokenManager::create_whitelist([1u8; 32], true, 100);
        assert!(result.is_ok());
        let whitelist = result.unwrap();
        assert!(whitelist.is_default_allow);
    }

    #[test]
    fn test_add_to_whitelist() {
        let mut whitelist = TokenManager::create_whitelist([1u8; 32], true, 100).unwrap();

        let result = TokenManager::add_to_whitelist(&mut whitelist, [2u8; 32], [1u8; 32]);
        assert!(result.is_ok());
        assert_eq!(whitelist.whitelisted_tokens.len(), 1);
    }

    #[test]
    fn test_add_to_whitelist_duplicate() {
        let mut whitelist = TokenManager::create_whitelist([1u8; 32], true, 100).unwrap();

        TokenManager::add_to_whitelist(&mut whitelist, [2u8; 32], [1u8; 32]).unwrap();
        let result = TokenManager::add_to_whitelist(&mut whitelist, [2u8; 32], [1u8; 32]);
        assert!(result.is_err());
    }

    #[test]
    fn test_remove_from_whitelist() {
        let mut whitelist = TokenManager::create_whitelist([1u8; 32], true, 100).unwrap();

        TokenManager::add_to_whitelist(&mut whitelist, [2u8; 32], [1u8; 32]).unwrap();
        let result = TokenManager::remove_from_whitelist(&mut whitelist, [2u8; 32], [1u8; 32]);
        assert!(result.is_ok());
        assert_eq!(whitelist.whitelisted_tokens.len(), 0);
    }

    #[test]
    fn test_is_token_allowed_allow_mode() {
        let mut whitelist = TokenManager::create_whitelist([1u8; 32], true, 100).unwrap();
        TokenManager::add_to_whitelist(&mut whitelist, [2u8; 32], [1u8; 32]).unwrap();

        // in allow mode, items in whitelist are blocked
        assert!(!TokenManager::is_token_allowed(&whitelist, [2u8; 32]));
        assert!(TokenManager::is_token_allowed(&whitelist, [99u8; 32]));
    }

    #[test]
    fn test_is_token_allowed_deny_mode() {
        let mut whitelist = TokenManager::create_whitelist([1u8; 32], false, 100).unwrap();
        TokenManager::add_to_whitelist(&mut whitelist, [2u8; 32], [1u8; 32]).unwrap();

        // in deny mode, only items in whitelist are allowed
        assert!(TokenManager::is_token_allowed(&whitelist, [2u8; 32]));
        assert!(!TokenManager::is_token_allowed(&whitelist, [99u8; 32]));
    }

    #[test]
    fn test_is_token_likely_spam_blacklisted() {
        let mut token = TokenManager::register_token(
            [1u8; 32],
            vec![1, 2, 3],
            vec![1, 2, 3],
            6,
            1000,
            [2u8; 32],
        )
        .unwrap();
        TokenManager::blacklist_token(&mut token, [2u8; 32]).unwrap();

        assert!(TokenManager::is_token_likely_spam(&token));
    }

    #[test]
    fn test_is_token_likely_spam_zero_supply() {
        let token = Token {
            id: [1u8; 32],
            address: [1u8; 32],
            symbol: vec![1, 2, 3],
            name: vec![1, 2, 3],
            decimals: 6,
            supply: 0,
            is_verified: true,
            is_blacklisted: false,
            owner: [2u8; 32],
        };

        assert!(TokenManager::is_token_likely_spam(&token));
    }

    #[test]
    fn test_validate_token() {
        let token = TokenManager::register_token(
            [1u8; 32],
            vec![1, 2, 3],
            vec![1, 2, 3],
            6,
            1000,
            [2u8; 32],
        )
        .unwrap();

        let result = TokenManager::validate_token(&token);
        assert!(result.is_ok());
    }

    #[test]
    fn test_transfer_token() {
        let mut balance = TokenBalance {
            token_id: [1u8; 32],
            holder: [2u8; 32],
            balance: 1000,
            last_updated_block: 0,
        };

        let result = TokenManager::transfer_token([2u8; 32], [3u8; 32], 500, &mut balance);
        assert!(result.is_ok());
        assert_eq!(balance.balance, 500);
    }

    #[test]
    fn test_transfer_token_insufficient_balance() {
        let mut balance = TokenBalance {
            token_id: [1u8; 32],
            holder: [2u8; 32],
            balance: 100,
            last_updated_block: 0,
        };

        let result = TokenManager::transfer_token([2u8; 32], [3u8; 32], 200, &mut balance);
        assert!(result.is_err());
    }

    #[test]
    fn test_update_balance() {
        let balance = TokenManager::update_balance([1u8; 32], [2u8; 32], 5000, 100);
        assert_eq!(balance.balance, 5000);
        assert_eq!(balance.last_updated_block, 100);
    }

    #[test]
    fn test_get_whitelist_size() {
        let mut whitelist = TokenManager::create_whitelist([1u8; 32], true, 100).unwrap();
        TokenManager::add_to_whitelist(&mut whitelist, [2u8; 32], [1u8; 32]).unwrap();

        assert_eq!(TokenManager::get_whitelist_size(&whitelist), 1);
    }

    #[test]
    fn test_is_in_whitelist() {
        let mut whitelist = TokenManager::create_whitelist([1u8; 32], true, 100).unwrap();
        TokenManager::add_to_whitelist(&mut whitelist, [2u8; 32], [1u8; 32]).unwrap();

        assert!(TokenManager::is_in_whitelist(&whitelist, [2u8; 32]));
        assert!(!TokenManager::is_in_whitelist(&whitelist, [99u8; 32]));
    }
}
