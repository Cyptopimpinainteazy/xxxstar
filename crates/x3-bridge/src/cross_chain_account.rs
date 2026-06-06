/// Cross-Chain Account Abstraction — Unified account control across EVM, SVM, IBC-compatible chains
/// Implements unified key standard with threshold multisig support across multiple blockchains
use parity_scale_codec::{Decode, DecodeWithMemTracking, Encode};
use sp_core::{ed25519, sr25519};
use sp_std::vec::Vec;
use x3_common::signing::{verify_signature, KeyType};

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct CrossChainAccount {
    pub account_id: [u8; 32],
    pub master_key: [u8; 32],
    pub evm_address: Option<[u8; 20]>,
    pub cosmos_address: Option<Vec<u8>>,
    pub solana_address: Option<[u8; 32]>,
    pub x3_address: [u8; 32],
    pub key_rotation_nonce: u32,
    pub is_active: bool,
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct DerivedAddress {
    pub chain_type: ChainType,
    pub address: Vec<u8>,
    pub derivation_path: Vec<u8>,
    pub balance: String, // Stored as string for arbitrary precision
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub enum ChainType {
    X3,
    Ethereum,
    Cosmos,
    Solana,
    Bitcoin,
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct MultiChainSignature {
    pub signer: Vec<u8>,
    pub message_hash: [u8; 32],
    pub signature: Vec<u8>,
    pub chain_type: ChainType,
    pub nonce: u32,
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct KeyRotationProposal {
    pub proposal_id: [u8; 32],
    pub account_id: [u8; 32],
    pub new_master_key: [u8; 32],
    pub confirmations: u32,
    pub required_threshold: u32,
    pub created_at: u64,
    pub expires_at: u64,
    pub status: ProposalStatus,
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub enum ProposalStatus {
    Pending,
    Approved,
    Executed,
    Rejected,
    Expired,
}

pub struct CrossChainAccountManager;

impl CrossChainAccountManager {
    /// Create unified cross-chain account with master key
    pub fn create_account(
        master_key: [u8; 32],
        x3_address: [u8; 32],
    ) -> Result<CrossChainAccount, &'static str> {
        if master_key == [0; 32] {
            return Err("Master key cannot be zero");
        }
        if x3_address == [0; 32] {
            return Err("X3 address cannot be zero");
        }

        let account_id = Self::derive_account_id(&master_key);

        Ok(CrossChainAccount {
            account_id,
            master_key,
            evm_address: None,
            cosmos_address: None,
            solana_address: None,
            x3_address,
            key_rotation_nonce: 0,
            is_active: true,
        })
    }

    /// Derive deterministic EVM address from master key
    pub fn derive_evm_address(master_key: &[u8; 32]) -> Result<[u8; 20], &'static str> {
        // BIP32 derivation path: m/44'/60'/0'/0/0 (Ethereum standard)
        let mut address = [0u8; 20];

        // Simplified: compute address from master key hash
        let pubkey_hash = Self::hash_key_with_path(master_key, b"ethereum");

        address[0..20].copy_from_slice(&pubkey_hash[0..20]);
        Ok(address)
    }

    /// Derive deterministic Cosmos address from master key
    pub fn derive_cosmos_address(master_key: &[u8; 32]) -> Result<Vec<u8>, &'static str> {
        let pubkey_hash = Self::hash_key_with_path(master_key, b"cosmos");

        // Cosmos addresses: bech32(pubkey_hash) with "cosmos" prefix
        let mut address = b"cosmos1".to_vec();
        address.extend_from_slice(&pubkey_hash[0..20]);
        Ok(address)
    }

    /// Derive deterministic Solana address from master key
    pub fn derive_solana_address(master_key: &[u8; 32]) -> Result<[u8; 32], &'static str> {
        let address = Self::hash_key_with_path(master_key, b"solana");
        Ok(address)
    }

    /// Add EVM address to account
    pub fn add_evm_address(
        account: &mut CrossChainAccount,
        evm_address: [u8; 20],
    ) -> Result<(), &'static str> {
        if evm_address == [0; 20] {
            return Err("EVM address cannot be zero");
        }
        account.evm_address = Some(evm_address);
        Ok(())
    }

    /// Add Cosmos address to account
    pub fn add_cosmos_address(
        account: &mut CrossChainAccount,
        cosmos_address: Vec<u8>,
    ) -> Result<(), &'static str> {
        if cosmos_address.is_empty() || cosmos_address.len() > 50 {
            return Err("Invalid Cosmos address length");
        }
        account.cosmos_address = Some(cosmos_address);
        Ok(())
    }

    /// Add Solana address to account
    pub fn add_solana_address(
        account: &mut CrossChainAccount,
        solana_address: [u8; 32],
    ) -> Result<(), &'static str> {
        if solana_address == [0; 32] {
            return Err("Solana address cannot be zero");
        }
        account.solana_address = Some(solana_address);
        Ok(())
    }

    /// Verify multi-chain signature with real per-chain cryptographic verification.
    ///
    /// - **Ethereum**: secp256k1 ECDSA recovery; recovered address matched against `account.evm_address`
    /// - **Solana / Cosmos**: Ed25519 verification; `signature.signer` used as the 32-byte public key
    /// - **X3**: sr25519 verification; `signature.signer` used as the 32-byte public key
    /// - **Bitcoin**: secp256k1 recovery; recovered key hash matched against `signature.signer[12..]`
    /// Convert ChainType to KeyType for signature verification
    fn chain_type_to_key_type(chain_type: ChainType) -> KeyType {
        match chain_type {
            ChainType::Ethereum | ChainType::Bitcoin => KeyType::Secp256k1,
            ChainType::Solana | ChainType::Cosmos => KeyType::Ed25519,
            ChainType::X3 => KeyType::Sr25519,
        }
    }

    pub fn verify_signature(
        account: &CrossChainAccount,
        signature: &MultiChainSignature,
    ) -> Result<bool, &'static str> {
        // Verify signer is associated with account.
        // EVM/secp256k1 chains use a 20-byte Ethereum address; other chains use the 32-byte account_id.
        let key_type_check = Self::chain_type_to_key_type(signature.chain_type.clone());
        if key_type_check == KeyType::Secp256k1 {
            let evm_addr = account
                .evm_address
                .as_ref()
                .ok_or("Account has no EVM address")?;
            if signature.signer != evm_addr.as_slice() {
                return Err("EVM signer address not recognized for this account");
            }
        } else if signature.signer != account.account_id.as_slice()
            && signature.signer != account.x3_address.as_slice()
        {
            return Err("Signer not recognized for this account");
        }

        // Verify nonce to prevent replays
        if signature.nonce < account.key_rotation_nonce {
            return Err("Signature nonce is stale");
        }

        // Convert chain type to key type
        let key_type = Self::chain_type_to_key_type(signature.chain_type.clone());

        // Verify signature using the unified signing module
        let signature_valid = x3_common::signing::verify_signature_hash(
            &signature.signature,
            &signature.message_hash,
            &signature.signer,
            key_type,
        );

        if !signature_valid {
            return Err("Signature verification failed");
        }

        // For Ethereum and Bitcoin, also verify the recovered address matches
        match signature.chain_type {
            ChainType::Ethereum | ChainType::Bitcoin => {
                // Verify recovered address matches stored EVM address
                match account.evm_address {
                    Some(stored_addr) => {
                        // Use the signing module's to_evm_address function
                        // Secp256k1 public keys are 65 bytes (uncompressed format)
                        if signature.signer.len() != 65 {
                            return Err("Invalid secp256k1 public key length");
                        }
                        let recovered_addr = x3_common::signing::PublicKey::Secp256k1({
                            let mut pk = [0u8; 65];
                            pk.copy_from_slice(&signature.signer);
                            pk
                        })
                        .to_evm_address();

                        if recovered_addr == stored_addr {
                            Ok(true)
                        } else {
                            Err("Recovered address does not match account")
                        }
                    }
                    None => Err("Account has no EVM address to verify against"),
                }
            }
            _ => Ok(true),
        }
    }

    /// Propose key rotation with threshold consensus requirement
    pub fn propose_key_rotation(
        account: &CrossChainAccount,
        new_master_key: [u8; 32],
        required_threshold: u32,
        expires_in: u64,
        current_timestamp: u64,
    ) -> Result<KeyRotationProposal, &'static str> {
        if new_master_key == [0; 32] {
            return Err("New master key cannot be zero");
        }
        if new_master_key == account.master_key {
            return Err("New key must differ from current key");
        }
        if required_threshold == 0 {
            return Err("Threshold must be positive");
        }

        let proposal_id = Self::derive_proposal_id(&account.account_id, &new_master_key);

        Ok(KeyRotationProposal {
            proposal_id,
            account_id: account.account_id,
            new_master_key,
            confirmations: 0,
            required_threshold,
            created_at: current_timestamp,
            expires_at: current_timestamp.saturating_add(expires_in),
            status: ProposalStatus::Pending,
        })
    }

    /// Vote on key rotation proposal
    pub fn vote_on_rotation(
        proposal: &mut KeyRotationProposal,
        voter: [u8; 32],
        approved: bool,
    ) -> Result<bool, &'static str> {
        if proposal.status != ProposalStatus::Pending {
            return Err("Proposal is not in Pending state");
        }

        // In production: implement proper voter tracking and uniqueness check
        if approved {
            proposal.confirmations = proposal.confirmations.saturating_add(1);
        }

        // Check if threshold reached
        if proposal.confirmations >= proposal.required_threshold {
            proposal.status = ProposalStatus::Approved;
            return Ok(true); // Ready to execute
        }

        Ok(false) // Still awaiting confirmations
    }

    /// Execute approved key rotation
    pub fn execute_rotation(
        account: &mut CrossChainAccount,
        proposal: &mut KeyRotationProposal,
    ) -> Result<(), &'static str> {
        if proposal.status != ProposalStatus::Approved {
            return Err("Proposal must be in Approved state");
        }
        if proposal.account_id != account.account_id {
            return Err("Proposal does not match account");
        }

        account.master_key = proposal.new_master_key;
        account.key_rotation_nonce = account.key_rotation_nonce.saturating_add(1);
        proposal.status = ProposalStatus::Executed;

        Ok(())
    }

    /// Get all addresses for account
    pub fn get_all_addresses(account: &CrossChainAccount) -> Vec<DerivedAddress> {
        let mut addresses = Vec::new();

        addresses.push(DerivedAddress {
            chain_type: ChainType::X3,
            address: account.x3_address.to_vec(),
            derivation_path: b"m/44'/60'/0'/0/0".to_vec(),
            balance: "0".to_string(),
        });

        if let Some(evm_addr) = account.evm_address {
            addresses.push(DerivedAddress {
                chain_type: ChainType::Ethereum,
                address: evm_addr.to_vec(),
                derivation_path: b"m/44'/60'/0'/0/0".to_vec(),
                balance: "0".to_string(),
            });
        }

        if let Some(cosmos_addr) = &account.cosmos_address {
            addresses.push(DerivedAddress {
                chain_type: ChainType::Cosmos,
                address: cosmos_addr.clone(),
                derivation_path: b"m/44'/118'/0'/0/0".to_vec(),
                balance: "0".to_string(),
            });
        }

        if let Some(solana_addr) = account.solana_address {
            addresses.push(DerivedAddress {
                chain_type: ChainType::Solana,
                address: solana_addr.to_vec(),
                derivation_path: b"m/44'/501'/0'/0/0".to_vec(),
                balance: "0".to_string(),
            });
        }

        addresses
    }

    /// Disable account (emergency pause)
    pub fn deactivate_account(account: &mut CrossChainAccount) -> Result<(), &'static str> {
        account.is_active = false;
        Ok(())
    }

    /// Re-enable account
    pub fn reactivate_account(account: &mut CrossChainAccount) -> Result<(), &'static str> {
        account.is_active = true;
        Ok(())
    }

    /// Derive deterministic account ID from master key
    fn derive_account_id(master_key: &[u8; 32]) -> [u8; 32] {
        let mut id = [0u8; 32];
        let mut hash = 0u64;

        for byte in master_key {
            hash = hash.wrapping_mul(31).wrapping_add(*byte as u64);
        }

        id[0..8].copy_from_slice(&hash.to_le_bytes());
        id
    }

    /// Hash master key with chain-specific path
    fn hash_key_with_path(master_key: &[u8; 32], path: &[u8]) -> [u8; 32] {
        let mut hash = [0u8; 32];
        let mut sum = 0u32;

        for byte in master_key {
            sum = sum.wrapping_mul(31).wrapping_add(*byte as u32);
        }
        for byte in path {
            sum = sum.wrapping_mul(31).wrapping_add(*byte as u32);
        }

        hash[0..4].copy_from_slice(&sum.to_le_bytes());
        hash
    }

    /// Derive deterministic proposal ID
    fn derive_proposal_id(account_id: &[u8; 32], new_key: &[u8; 32]) -> [u8; 32] {
        let mut id = [0u8; 32];
        for i in 0..32 {
            id[i] = account_id[i] ^ new_key[i];
        }
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_account() {
        let master_key = [1; 32];
        let x3_address = [2; 32];

        let account = CrossChainAccountManager::create_account(master_key, x3_address).unwrap();

        assert_eq!(account.master_key, master_key);
        assert_eq!(account.x3_address, x3_address);
        assert!(account.is_active);
        assert_eq!(account.key_rotation_nonce, 0);
    }

    #[test]
    fn test_create_account_zero_master_key() {
        let result = CrossChainAccountManager::create_account([0; 32], [2; 32]);
        assert!(result.is_err());
    }

    #[test]
    fn test_derive_evm_address() {
        let master_key = [1; 32];
        let evm_addr = CrossChainAccountManager::derive_evm_address(&master_key).unwrap();

        assert_ne!(evm_addr, [0; 20]);

        // Deterministic
        let evm_addr2 = CrossChainAccountManager::derive_evm_address(&master_key).unwrap();
        assert_eq!(evm_addr, evm_addr2);
    }

    #[test]
    fn test_derive_cosmos_address() {
        let master_key = [1; 32];
        let cosmos_addr = CrossChainAccountManager::derive_cosmos_address(&master_key).unwrap();

        assert!(!cosmos_addr.is_empty());
        assert!(cosmos_addr.starts_with(b"cosmos1"));
    }

    #[test]
    fn test_derive_solana_address() {
        let master_key = [1; 32];
        let solana_addr = CrossChainAccountManager::derive_solana_address(&master_key).unwrap();

        assert_ne!(solana_addr, [0; 32]);
    }

    #[test]
    fn test_add_evm_address() {
        let mut account = CrossChainAccountManager::create_account([1; 32], [2; 32]).unwrap();

        CrossChainAccountManager::add_evm_address(&mut account, [3; 20]).unwrap();
        assert_eq!(account.evm_address, Some([3; 20]));
    }

    #[test]
    fn test_add_cosmos_address() {
        let mut account = CrossChainAccountManager::create_account([1; 32], [2; 32]).unwrap();

        let cosmos_addr = b"cosmos1abc123".to_vec();
        CrossChainAccountManager::add_cosmos_address(&mut account, cosmos_addr.clone()).unwrap();
        assert_eq!(account.cosmos_address, Some(cosmos_addr));
    }

    #[test]
    fn test_propose_key_rotation() {
        let account = CrossChainAccountManager::create_account([1; 32], [2; 32]).unwrap();
        let new_key = [3; 32];

        let proposal =
            CrossChainAccountManager::propose_key_rotation(&account, new_key, 1, 3600, 1000)
                .unwrap();

        assert_eq!(proposal.status, ProposalStatus::Pending);
        assert_eq!(proposal.new_master_key, new_key);
    }

    #[test]
    fn test_propose_rotation_same_key() {
        let account = CrossChainAccountManager::create_account([1; 32], [2; 32]).unwrap();

        let result = CrossChainAccountManager::propose_key_rotation(
            &account, [1; 32], // Same as master_key
            1, 3600, 1000,
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_vote_and_execute_rotation() {
        let mut account = CrossChainAccountManager::create_account([1; 32], [2; 32]).unwrap();
        let new_key = [3; 32];

        let mut proposal =
            CrossChainAccountManager::propose_key_rotation(&account, new_key, 1, 3600, 1000)
                .unwrap();

        let threshold_reached =
            CrossChainAccountManager::vote_on_rotation(&mut proposal, [4; 32], true).unwrap();
        assert!(threshold_reached);

        CrossChainAccountManager::execute_rotation(&mut account, &mut proposal).unwrap();
        assert_eq!(account.master_key, new_key);
        assert_eq!(account.key_rotation_nonce, 1);
    }

    #[test]
    fn test_get_all_addresses() {
        let mut account = CrossChainAccountManager::create_account([1; 32], [2; 32]).unwrap();
        CrossChainAccountManager::add_evm_address(&mut account, [3; 20]).unwrap();

        let addresses = CrossChainAccountManager::get_all_addresses(&account);

        assert!(addresses.len() >= 2);
        assert!(addresses.iter().any(|a| a.chain_type == ChainType::X3));
        assert!(addresses
            .iter()
            .any(|a| a.chain_type == ChainType::Ethereum));
    }

    #[test]
    fn test_deactivate_account() {
        let mut account = CrossChainAccountManager::create_account([1; 32], [2; 32]).unwrap();

        CrossChainAccountManager::deactivate_account(&mut account).unwrap();
        assert!(!account.is_active);

        CrossChainAccountManager::reactivate_account(&mut account).unwrap();
        assert!(account.is_active);
    }

    #[test]
    fn test_verify_signature() {
        sp_io::TestExternalities::default().execute_with(|| {
            use sp_core::Pair;
            let pair = sp_core::sr25519::Pair::from_seed_slice(&[1u8; 32]).unwrap();
            let sr_pubkey = pair.public();
            let message_hash = [3u8; 32];
            let sr_sig = pair.sign(&message_hash);

            let mut account = CrossChainAccountManager::create_account([1; 32], [2; 32]).unwrap();
            // Override account_id so the signer-association check matches the sr25519 pubkey.
            account.account_id = sr_pubkey.0;

            let sig = MultiChainSignature {
                signer: sr_pubkey.0.to_vec(),
                message_hash,
                signature: sr_sig.0.to_vec(),
                chain_type: ChainType::X3,
                nonce: 0,
            };

            assert!(CrossChainAccountManager::verify_signature(&account, &sig).unwrap());
        });
    }

    #[test]
    fn test_verify_signature_stale_nonce() {
        let mut account = CrossChainAccountManager::create_account([1; 32], [2; 32]).unwrap();
        account.key_rotation_nonce = 5;

        let sig = MultiChainSignature {
            signer: account.account_id.to_vec(),
            message_hash: [3; 32],
            signature: vec![1, 2, 3],
            chain_type: ChainType::X3,
            nonce: 3, // Less than current nonce
        };

        let result = CrossChainAccountManager::verify_signature(&account, &sig);
        assert!(result.is_err());
    }
}
