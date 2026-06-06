//! Solana Wormhole Adapter: SPL token bridging via VAA attestation
//!
//! Integrate Wormhole's cross-chain messaging (VAA = Verified Action Approval)
//! Enable SPL tokens on X3 with native Solana devnet compatibility.

use std::collections::HashMap;

/// Wormhole VAA (Verified Action Approval)
#[derive(Clone, Debug)]
pub struct VAA {
    pub version: u8,
    pub guardian_set_index: u32,
    pub signatures: Vec<GuardianSignature>,
    pub timestamp: u64,
    pub nonce: u32,
    pub emitter_chain: u16, // 1 = Solana
    pub emitter_address: Vec<u8>,
    pub sequence: u64,
    pub consistency_level: u8,
    pub payload: Vec<u8>,
}

#[derive(Clone, Debug)]
pub struct GuardianSignature {
    pub index: u8,
    pub signature: Vec<u8>,
}

/// SPL Token wrapped on X3
#[derive(Clone, Debug)]
pub struct WrappedSPLToken {
    pub mint_x3: String,
    pub mint_solana: String,
    pub decimals: u8,
    pub supply: u128,
    pub canonical: bool,
}

/// Wormhole bridge state
pub struct WormholeBridge {
    // Solana side
    pub solana_locked: HashMap<String, u128>, // SPL mint → amount locked
    pub solana_vaa_log: HashMap<String, VAA>, // sequence → VAA

    // X3 side
    pub wrapped_tokens: HashMap<String, WrappedSPLToken>,
    pub x3_balances: HashMap<String, u128>, // (mint_x3, account) → balance

    // Bridge config
    pub guardian_count: u8,
    pub signature_threshold: u8, // 13 of 19 guardians
    pub wormhole_chain_id: u16,  // 30 = X3
}

impl WormholeBridge {
    pub fn new(guardian_count: u8) -> Self {
        Self {
            solana_locked: HashMap::new(),
            solana_vaa_log: HashMap::new(),
            wrapped_tokens: HashMap::new(),
            x3_balances: HashMap::new(),
            guardian_count,
            signature_threshold: (guardian_count * 2 / 3) + 1, // 2/3 + 1
            wormhole_chain_id: 30,
        }
    }

    /// Register an SPL token for bridging
    pub fn register_spl_token(
        &mut self,
        mint_solana: String,
        mint_x3: String,
        decimals: u8,
        canonical: bool,
    ) -> Result<(), String> {
        if mint_solana.is_empty() || mint_x3.is_empty() {
            return Err("Token mints cannot be empty".to_string());
        }

        let wrapped = WrappedSPLToken {
            mint_x3,
            mint_solana,
            decimals,
            supply: 0,
            canonical,
        };

        self.wrapped_tokens.insert(wrapped.mint_x3.clone(), wrapped);

        Ok(())
    }

    /// Validate VAA structure and signatures
    pub fn verify_vaa(&self, vaa: &VAA) -> Result<(), String> {
        if vaa.guardian_set_index == 0 {
            return Err("Invalid guardian set".to_string());
        }

        if vaa.signatures.len() < self.signature_threshold as usize {
            return Err(format!(
                "Need {} signatures, got {}",
                self.signature_threshold,
                vaa.signatures.len()
            ));
        }

        // Verify no duplicate signatures
        let mut seen = std::collections::HashSet::new();
        for sig in &vaa.signatures {
            if !seen.insert(sig.index) {
                return Err("Duplicate guardian signature".to_string());
            }
        }

        // In production: verify ECDSA signatures against guardian keys

        Ok(())
    }

    /// Parse Wormhole payload from VAA
    pub fn parse_transfer_payload(payload: &[u8]) -> Result<TransferPayload, String> {
        if payload.len() < 100 {
            return Err("Payload too short".to_string());
        }

        // Simplified: extract key fields
        let token_chain = u16::from_be_bytes([payload[0], payload[1]]);
        let token_address = payload[2..34].to_vec();
        let amount = u128::from_be_bytes(payload[34..50].try_into().unwrap_or([0; 16]));

        Ok(TransferPayload {
            token_chain,
            token_address,
            amount,
            recipient_chain: 30, // X3
            recipient: String::new(),
        })
    }

    /// Process Wormhole cross-chain transfer
    pub fn process_wormhole_transfer(
        &mut self,
        vaa: VAA,
        recipient_x3: String,
    ) -> Result<WrappedSPLToken, String> {
        // Verify VAA
        self.verify_vaa(&vaa)?;

        // Parse payload
        let payload = Self::parse_transfer_payload(&vaa.payload)?;

        // Find registered SPL token
        let wrapped = self
            .wrapped_tokens
            .values()
            .find(|t| t.canonical && payload.token_chain == 1)
            .ok_or("SPL token not registered")?
            .clone();

        // Log VAA
        let vaa_id = format!("vaa_{}", vaa.sequence);
        self.solana_vaa_log.insert(vaa_id, vaa);

        // Mint wrapped token on X3
        let key = format!("{}_{}", wrapped.mint_x3, recipient_x3);
        self.x3_balances.insert(key, payload.amount);

        Ok(wrapped)
    }

    /// Burn wrapped SPL token on X3 to unlock on Solana
    pub fn burn_for_solana_unlock(
        &mut self,
        x3_account: String,
        mint_x3: String,
        amount: u128,
    ) -> Result<(), String> {
        let wrapped = self
            .wrapped_tokens
            .get(&mint_x3)
            .ok_or("Token not found")?
            .clone();

        let key = format!("{}_{}", mint_x3, x3_account);
        let balance = self.x3_balances.get(&key).ok_or("No balance")?;

        if amount > *balance {
            return Err("Insufficient balance".to_string());
        }

        // Burn
        self.x3_balances.insert(key, balance - amount);

        // Log unlock event (validators observe and sign)
        self.solana_locked.insert(wrapped.mint_solana, amount);

        Ok(())
    }

    /// Get wrapped SPL token info
    pub fn get_wrapped_token(&self, mint_x3: &str) -> Option<WrappedSPLToken> {
        self.wrapped_tokens.get(mint_x3).cloned()
    }

    /// Get balance of wrapped token
    pub fn get_balance(&self, x3_account: &str, mint_x3: &str) -> u128 {
        let key = format!("{}_{}", mint_x3, x3_account);
        *self.x3_balances.get(&key).unwrap_or(&0)
    }

    /// Check VAA delivery status
    pub fn is_vaa_delivered(&self, vaa_sequence: u64) -> bool {
        self.solana_vaa_log
            .values()
            .any(|v| v.sequence == vaa_sequence)
    }
}

#[derive(Clone, Debug)]
pub struct TransferPayload {
    pub token_chain: u16,
    pub token_address: Vec<u8>,
    pub amount: u128,
    pub recipient_chain: u16,
    pub recipient: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wormhole_creation() {
        let bridge = WormholeBridge::new(19);
        assert_eq!(bridge.guardian_count, 19);
        assert!(bridge.signature_threshold > 0);
    }

    #[test]
    fn test_register_spl_token() {
        let mut bridge = WormholeBridge::new(19);

        let result = bridge.register_spl_token(
            "So11111111111111111111111111111111111111112".to_string(), // SOL devnet
            "0x01".to_string(),
            9,
            true,
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_verify_vaa() {
        let bridge = WormholeBridge::new(19);

        let vaa = VAA {
            version: 1,
            guardian_set_index: 0, // Invalid
            signatures: vec![],
            timestamp: 1000,
            nonce: 1,
            emitter_chain: 1,
            emitter_address: vec![],
            sequence: 100,
            consistency_level: 15,
            payload: vec![],
        };

        assert!(bridge.verify_vaa(&vaa).is_err());
    }

    #[test]
    fn test_verify_vaa_insufficient_sigs() {
        let bridge = WormholeBridge::new(19);

        let vaa = VAA {
            version: 1,
            guardian_set_index: 1,
            signatures: vec![GuardianSignature {
                index: 0,
                signature: vec![1, 2, 3],
            }],
            timestamp: 1000,
            nonce: 1,
            emitter_chain: 1,
            emitter_address: vec![],
            sequence: 100,
            consistency_level: 15,
            payload: vec![],
        };

        assert!(bridge.verify_vaa(&vaa).is_err());
    }

    #[test]
    fn test_parse_transfer_payload() {
        let mut payload = vec![0u8; 100];
        payload[0] = 0; // token_chain high byte
        payload[1] = 1; // token_chain low byte (Solana)

        let parsed = WormholeBridge::parse_transfer_payload(&payload);
        assert!(parsed.is_ok());
    }

    #[test]
    fn test_get_wrapped_token() {
        let mut bridge = WormholeBridge::new(19);

        bridge
            .register_spl_token(
                "So11111111111111111111111111111111111111112".to_string(),
                "0x01".to_string(),
                9,
                true,
            )
            .ok();

        let token = bridge.get_wrapped_token("0x01");
        assert!(token.is_some());
    }

    #[test]
    fn test_get_balance() {
        let mut bridge = WormholeBridge::new(19);

        bridge
            .register_spl_token(
                "So11111111111111111111111111111111111111112".to_string(),
                "0x01".to_string(),
                9,
                true,
            )
            .ok();

        // Manually set balance for testing
        let key = "0x01_0xAlice".to_string();
        bridge.x3_balances.insert(key, 1_000_000_000u128);

        let balance = bridge.get_balance("0xAlice", "0x01");
        assert_eq!(balance, 1_000_000_000);
    }

    #[test]
    fn test_burn_for_unlock() {
        let mut bridge = WormholeBridge::new(19);

        bridge
            .register_spl_token(
                "So11111111111111111111111111111111111111112".to_string(),
                "0x01".to_string(),
                9,
                true,
            )
            .ok();

        // Set balance
        let key = "0x01_0xAlice".to_string();
        bridge.x3_balances.insert(key, 1_000_000_000u128);

        let result = bridge.burn_for_solana_unlock(
            "0xAlice".to_string(),
            "0x01".to_string(),
            500_000_000u128,
        );

        assert!(result.is_ok());
    }
}
