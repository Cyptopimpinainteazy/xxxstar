//! Wallet Store for Tauri Desktop Core
//!
//! This module provides secure wallet storage functionality with:
//!
//! - Local storage persistence using Tauri's store plugin
//! - Encryption for sensitive data (mnemonics, private keys)
//! - Wallet recovery functionality
//! - Multi-chain wallet support

use aes_gcm::{aead::{Aead, KeyInit}, Aes256Gcm, Nonce};
use base64::{Engine, engine::general_purpose::STANDARD};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::num::NonZeroU32;

/// Wallet encryption configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletEncryptionConfig {
    pub algorithm: String,
    pub key_derivation: String,
    pub salt: String,
}

/// Encrypted wallet data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedWalletData {
    pub encrypted_mnemonic: String,
    pub encrypted_seed: String,
    pub iv: String,
    pub salt: String,
    pub derivation_path: String,
    pub created_at: u64,
}

/// Wallet account information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletAccount {
    pub chain: String,
    pub address: String,
    pub public_key: String,
    pub derivation_path: String,
    pub is_default: bool,
}

/// Wallet metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletMetadata {
    pub id: String,
    pub name: String,
    pub created_at: u64,
    pub last_used: u64,
    pub chain_count: usize,
    pub is_encrypted: bool,
}

/// Wallet store error
#[derive(Debug, thiserror::Error)]
pub enum WalletStoreError {
    #[error("Storage error: {0}")]
    Storage(String),
    #[error("Encryption error: {0}")]
    Encryption(String),
    #[error("Decryption error: {0}")]
    Decryption(String),
    #[error("Wallet not found: {0}")]
    WalletNotFound(String),
    #[error("Invalid wallet data: {0}")]
    InvalidData(String),
}

/// Wallet store state
#[derive(Debug, Clone)]
pub struct WalletStoreState {
    pub is_initialized: bool,
    pub wallet_count: usize,
    pub active_wallet_id: Option<String>,
}

/// Wallet store - manages encrypted wallet storage
pub struct WalletStore {
    state: WalletStoreState,
    wallets: HashMap<String, EncryptedWalletData>,
    accounts: HashMap<String, Vec<WalletAccount>>,
    encryption_config: WalletEncryptionConfig,
}

impl WalletStore {
    /// Create a new wallet store
    pub fn new() -> Self {
        Self {
            state: WalletStoreState {
                is_initialized: false,
                wallet_count: 0,
                active_wallet_id: None,
            },
            wallets: HashMap::new(),
            accounts: HashMap::new(),
            encryption_config: WalletEncryptionConfig {
                algorithm: "AES-256-GCM".to_string(),
                key_derivation: "PBKDF2-HMAC-SHA256".to_string(),
                salt: Self::generate_salt(),
            },
        }
    }

    /// Generate a random salt for encryption
    fn generate_salt() -> String {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let salt: [u8; 16] = rng.gen();
        hex::encode(salt)
    }

    /// Initialize the wallet store
    pub fn initialize(&mut self) {
        self.state.is_initialized = true;
    }

    /// Check if the store is initialized
    pub fn is_initialized(&self) -> bool {
        self.state.is_initialized
    }

    /// Derive encryption key from master password using PBKDF2
    fn derive_key(&self, master_password: &str) -> [u8; 32] {
        use ring::pbkdf2::{derive, PBKDF2_HMAC_SHA256};
        
        let salt = self.encryption_config.salt.as_bytes();
        let mut key = [0u8; 32];
        let iterations = NonZeroU32::new(100_000).expect("PBKDF2 iteration count is non-zero");
        derive(PBKDF2_HMAC_SHA256, iterations, salt, master_password.as_bytes(), &mut key);
        key
    }

    /// Encrypt wallet data using AES-256-GCM
    pub fn encrypt_data(&self, plaintext: &str, master_password: &str) -> Result<String, WalletStoreError> {
        let key = self.derive_key(master_password);
        let cipher = Aes256Gcm::new_from_slice(&key)
            .map_err(|e| WalletStoreError::Encryption(format!("Invalid key: {}", e)))?;
        
        let mut nonce_bytes = [0u8; 12];
        rand::rngs::OsRng.fill(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);
        
        let ciphertext = cipher.encrypt(nonce, plaintext.as_bytes())
            .map_err(|e| WalletStoreError::Encryption(format!("Encryption failed: {}", e)))?;
        
        // Combine nonce and ciphertext, then base64 encode
        let mut combined = Vec::new();
        combined.extend_from_slice(&nonce_bytes);
        combined.extend_from_slice(&ciphertext);
        
        Ok(STANDARD.encode(combined))
    }

    /// Decrypt wallet data using AES-256-GCM
    pub fn decrypt_data(&self, ciphertext: &str, master_password: &str) -> Result<String, WalletStoreError> {
        let key = self.derive_key(master_password);
        let cipher = Aes256Gcm::new_from_slice(&key)
            .map_err(|e| WalletStoreError::Decryption(format!("Invalid key: {}", e)))?;
        
        let combined = STANDARD.decode(ciphertext)
            .map_err(|e| WalletStoreError::Decryption(format!("Invalid base64: {}", e)))?;
        
        if combined.len() < 12 {
            return Err(WalletStoreError::Decryption("Ciphertext too short".to_string()));
        }
        
        let (nonce_bytes, ciphertext_bytes) = combined.split_at(12);
        let nonce = Nonce::from_slice(nonce_bytes);
        
        let plaintext = cipher.decrypt(nonce, ciphertext_bytes)
            .map_err(|e| WalletStoreError::Decryption(format!("Decryption failed: {}", e)))?;
        
        String::from_utf8(plaintext)
            .map_err(|e| WalletStoreError::Decryption(format!("Invalid UTF-8: {}", e)))
    }

    /// Store a wallet
    pub fn store_wallet(
        &mut self,
        wallet_id: &str,
        mnemonic: &str,
        seed: &str,
        derivation_path: &str,
        master_password: &str,
    ) -> Result<(), WalletStoreError> {
        let encrypted_mnemonic = self.encrypt_data(mnemonic, master_password)?;
        let encrypted_seed = self.encrypt_data(seed, master_password)?;

        let wallet_data = EncryptedWalletData {
            encrypted_mnemonic,
            encrypted_seed,
            iv: Self::generate_salt(),
            salt: self.encryption_config.salt.clone(),
            derivation_path: derivation_path.to_string(),
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };

        self.wallets.insert(wallet_id.to_string(), wallet_data);
        self.state.wallet_count += 1;

        Ok(())
    }

    /// Retrieve a wallet (returns decrypted data)
    pub fn retrieve_wallet(
        &self,
        wallet_id: &str,
        master_password: &str,
    ) -> Result<(String, String), WalletStoreError> {
        let wallet_data = self
            .wallets
            .get(wallet_id)
            .ok_or_else(|| WalletStoreError::WalletNotFound(wallet_id.to_string()))?;

        let mnemonic = self.decrypt_data(&wallet_data.encrypted_mnemonic, master_password)?;
        let seed = self.decrypt_data(&wallet_data.encrypted_seed, master_password)?;

        Ok((mnemonic, seed))
    }

    /// Delete a wallet
    pub fn delete_wallet(&mut self, wallet_id: &str) -> Result<(), WalletStoreError> {
        if self.wallets.remove(wallet_id).is_some() {
            self.state.wallet_count -= 1;
            Ok(())
        } else {
            Err(WalletStoreError::WalletNotFound(wallet_id.to_string()))
        }
    }

    /// Add an account to a wallet
    pub fn add_account(
        &mut self,
        wallet_id: &str,
        account: WalletAccount,
    ) -> Result<(), WalletStoreError> {
        let accounts = self.accounts.entry(wallet_id.to_string()).or_default();
        accounts.push(account);
        Ok(())
    }

    /// Get accounts for a wallet
    pub fn get_accounts(&self, wallet_id: &str) -> Result<&[WalletAccount], WalletStoreError> {
        self.accounts
            .get(wallet_id)
            .map(|v| v.as_slice())
            .ok_or_else(|| WalletStoreError::WalletNotFound(wallet_id.to_string()))
    }

    /// Set the active wallet
    pub fn set_active_wallet(&mut self, wallet_id: &str) -> Result<(), WalletStoreError> {
        if self.wallets.contains_key(wallet_id) {
            self.state.active_wallet_id = Some(wallet_id.to_string());
            Ok(())
        } else {
            Err(WalletStoreError::WalletNotFound(wallet_id.to_string()))
        }
    }

    /// Get the active wallet ID
    pub fn get_active_wallet_id(&self) -> Option<&str> {
        self.state.active_wallet_id.as_deref()
    }

    /// Get wallet metadata
    pub fn get_wallet_metadata(&self, wallet_id: &str) -> Result<WalletMetadata, WalletStoreError> {
        let wallet_data = self
            .wallets
            .get(wallet_id)
            .ok_or_else(|| WalletStoreError::WalletNotFound(wallet_id.to_string()))?;

        let account_count = self.accounts.get(wallet_id).map(|v| v.len()).unwrap_or(0);

        Ok(WalletMetadata {
            id: wallet_id.to_string(),
            name: format!("Wallet {}", &wallet_id[0..8]),
            created_at: wallet_data.created_at,
            last_used: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            chain_count: account_count,
            is_encrypted: true,
        })
    }

    /// Export wallet backup (for recovery)
    pub fn export_backup(&self, wallet_id: &str) -> Result<String, WalletStoreError> {
        let wallet_data = self
            .wallets
            .get(wallet_id)
            .ok_or_else(|| WalletStoreError::WalletNotFound(wallet_id.to_string()))?;

        // In production, this would create an encrypted backup file
        // For now, return the encrypted data as a JSON string
        serde_json::to_string(wallet_data).map_err(|e| WalletStoreError::Storage(e.to_string()))
    }

    /// Import wallet from backup
    pub fn import_backup(&mut self, backup: &str) -> Result<String, WalletStoreError> {
        let wallet_data: EncryptedWalletData =
            serde_json::from_str(backup).map_err(|e| WalletStoreError::InvalidData(e.to_string()))?;

        // Generate a new wallet ID
        let wallet_id = format!(
            "wallet_{}",
            hex::encode(sp_core::hashing::keccak_256(backup.as_bytes()))
                [0..16]
                .to_uppercase()
        );

        self.wallets.insert(wallet_id.clone(), wallet_data);
        self.state.wallet_count += 1;

        Ok(wallet_id)
    }

    /// Get the number of stored wallets
    pub fn wallet_count(&self) -> usize {
        self.state.wallet_count
    }

    /// Check if a wallet exists
    pub fn has_wallet(&self, wallet_id: &str) -> bool {
        self.wallets.contains_key(wallet_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wallet_store_creation() {
        let store = WalletStore::new();
        assert!(!store.is_initialized());
        assert_eq!(store.wallet_count(), 0);
    }

    #[test]
    fn test_wallet_store_initialization() {
        let mut store = WalletStore::new();
        store.initialize();
        assert!(store.is_initialized());
    }

    #[test]
    fn test_wallet_store_encryption() {
        let store = WalletStore::new();
        let encrypted = store.encrypt_data("test_mnemonic").unwrap();
        assert!(encrypted.starts_with("encrypted_"));
    }

    #[test]
    fn test_wallet_store_roundtrip() {
        let mut store = WalletStore::new();
        store.initialize();

        let wallet_id = "test_wallet";
        store
            .store_wallet(wallet_id, "test mnemonic", "test seed", "m/44'/60'/0'/0/0")
            .unwrap();

        let (mnemonic, seed) = store.retrieve_wallet(wallet_id).unwrap();
        assert_eq!(mnemonic, "test mnemonic");
        assert_eq!(seed, "test seed");
    }

    #[test]
    fn test_wallet_store_recovery() {
        let mut store = WalletStore::new();
        store.initialize();

        let wallet_id = "test_wallet";
        store
            .store_wallet(wallet_id, "test mnemonic", "test seed", "m/44'/60'/0'/0/0")
            .unwrap();

        let backup = store.export_backup(wallet_id).unwrap();
        let new_id = store.import_backup(&backup).unwrap();

        assert_ne!(wallet_id, new_id);
        assert_eq!(store.wallet_count(), 2);
    }
}
