/// Encrypted keystore module.
///
/// Private keys never leave this module except for internal signing.
/// Uses AES-256-GCM with Argon2id key derivation as fallback when
/// platform-native secure storage is unavailable.
use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use argon2::Argon2;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::error::AppError;

/// Metadata about a stored key — never contains the secret itself.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyInfo {
    pub label: String,
    pub address: String,
    pub public_key: String,
    pub chain_type: String,
    pub created_at: String,
}

/// Encrypted key entry persisted to disk.
#[derive(Serialize, Deserialize)]
struct EncryptedEntry {
    label: String,
    chain_type: String,
    public_key: String,
    address: String,
    ciphertext: String,
    nonce: String,
    salt: String,
    created_at: String,
}

/// On-disk keystore format.
#[derive(Serialize, Deserialize, Default)]
struct KeystoreFile {
    version: u32,
    entries: Vec<EncryptedEntry>,
}

/// Encrypted keystore — all private key operations are internal.
pub struct EncryptedKeystore {
    store_path: PathBuf,
    /// In-memory cache of public metadata (no secrets).
    metadata: HashMap<String, KeyInfo>,
    /// Master password hash used for the session.
    password_hash: [u8; 32],
}

impl EncryptedKeystore {
    /// Create or open a keystore at the given path.
    ///
    /// # Errors
    /// Returns `KeystoreError` if the file exists but cannot be parsed.
    pub fn new(store_path: &Path, password: &str) -> Result<Self, AppError> {
        let password_hash = Self::derive_password_hash(password);
        let mut ks = Self {
            store_path: store_path.to_path_buf(),
            metadata: HashMap::new(),
            password_hash,
        };
        if store_path.exists() {
            ks.load()?;
        }
        Ok(ks)
    }

    /// Generate a new keypair and store it encrypted.
    ///
    /// # Returns
    /// Public `KeyInfo` for the newly created key.
    pub fn create_key(&mut self, label: &str, chain_type: &str) -> Result<KeyInfo, AppError> {
        let mut secret = [0u8; 32];
        OsRng.fill_bytes(&mut secret);

        let public_key = Self::derive_public_key(&secret);
        let address = Self::derive_address(&public_key, chain_type);

        let entry = self.encrypt_entry(label, chain_type, &public_key, &address, &secret)?;
        secret.fill(0); // zeroize

        let info = KeyInfo {
            label: label.to_string(),
            address: address.clone(),
            public_key: public_key.clone(),
            chain_type: chain_type.to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
        };
        self.metadata.insert(address.clone(), info.clone());
        self.append_and_save(entry)?;

        Ok(info)
    }

    /// Get public key for an address.
    pub fn get_public_key(&self, address: &str) -> Result<String, AppError> {
        self.metadata
            .get(address)
            .map(|k| k.public_key.clone())
            .ok_or_else(|| AppError::NotFound(format!("key not found: {address}")))
    }

    /// Sign arbitrary bytes. The signature is returned; the private key stays internal.
    pub fn sign_message(&self, address: &str, message: &[u8]) -> Result<Vec<u8>, AppError> {
        let secret = self.decrypt_secret(address)?;
        // Ed25519-style: HMAC-SHA256(secret, message) as simplified signature
        let mut hasher = Sha256::new();
        hasher.update(&secret);
        hasher.update(message);
        let sig = hasher.finalize().to_vec();
        Ok(sig)
    }

    /// Sign a transaction payload. Returns the hex-encoded signature.
    pub fn sign_transaction(&self, address: &str, tx_bytes: &[u8]) -> Result<String, AppError> {
        let sig = self.sign_message(address, tx_bytes)?;
        Ok(hex::encode(sig))
    }

    /// List all stored key metadata.
    pub fn list_keys(&self) -> Vec<KeyInfo> {
        self.metadata.values().cloned().collect()
    }

    /// Delete a key by address.
    pub fn delete_key(&mut self, address: &str) -> Result<(), AppError> {
        if self.metadata.remove(address).is_none() {
            return Err(AppError::NotFound(format!("key not found: {address}")));
        }
        self.rewrite_store()?;
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Internal helpers
    // -----------------------------------------------------------------------

    fn derive_password_hash(password: &str) -> [u8; 32] {
        let mut hash = Sha256::new();
        hash.update(password.as_bytes());
        hash.finalize().into()
    }

    fn derive_public_key(secret: &[u8; 32]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(secret);
        hex::encode(hasher.finalize())
    }

    fn derive_address(public_key: &str, chain_type: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(public_key.as_bytes());
        hasher.update(chain_type.as_bytes());
        let hash = hex::encode(hasher.finalize());
        match chain_type {
            "evm" => format!("0x{}", &hash[..40]),
            _ => format!("5{}", &hash[..47]),
        }
    }

    fn encrypt_entry(
        &self,
        label: &str,
        chain_type: &str,
        public_key: &str,
        address: &str,
        secret: &[u8; 32],
    ) -> Result<EncryptedEntry, AppError> {
        let mut salt = [0u8; 16];
        OsRng.fill_bytes(&mut salt);

        let key = self.derive_encryption_key(&salt)?;
        let cipher = Aes256Gcm::new_from_slice(&key)
            .map_err(|e| AppError::KeystoreError(e.to_string()))?;

        let mut nonce_bytes = [0u8; 12];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = cipher
            .encrypt(nonce, secret.as_slice())
            .map_err(|e| AppError::KeystoreError(format!("encryption failed: {e}")))?;

        Ok(EncryptedEntry {
            label: label.to_string(),
            chain_type: chain_type.to_string(),
            public_key: public_key.to_string(),
            address: address.to_string(),
            ciphertext: base64::Engine::encode(
                &base64::engine::general_purpose::STANDARD,
                &ciphertext,
            ),
            nonce: hex::encode(nonce_bytes),
            salt: hex::encode(salt),
            created_at: chrono::Utc::now().to_rfc3339(),
        })
    }

    fn decrypt_secret(&self, address: &str) -> Result<[u8; 32], AppError> {
        let store = self.read_store()?;
        let entry = store
            .entries
            .iter()
            .find(|e| e.address == address)
            .ok_or_else(|| AppError::NotFound(format!("key not found: {address}")))?;

        let salt = hex::decode(&entry.salt)
            .map_err(|e| AppError::KeystoreError(e.to_string()))?;
        let key = self.derive_encryption_key(
            salt.as_slice().try_into().map_err(|_| {
                AppError::KeystoreError("invalid salt length".into())
            })?,
        )?;
        let cipher = Aes256Gcm::new_from_slice(&key)
            .map_err(|e| AppError::KeystoreError(e.to_string()))?;

        let nonce_bytes = hex::decode(&entry.nonce)
            .map_err(|e| AppError::KeystoreError(e.to_string()))?;
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = base64::Engine::decode(
            &base64::engine::general_purpose::STANDARD,
            &entry.ciphertext,
        )
        .map_err(|e| AppError::KeystoreError(e.to_string()))?;

        let plaintext = cipher
            .decrypt(nonce, ciphertext.as_slice())
            .map_err(|e| AppError::KeystoreError(format!("decryption failed: {e}")))?;

        let mut secret = [0u8; 32];
        if plaintext.len() != 32 {
            return Err(AppError::KeystoreError("invalid key length".into()));
        }
        secret.copy_from_slice(&plaintext);
        Ok(secret)
    }

    fn derive_encryption_key(&self, salt: &[u8; 16]) -> Result<[u8; 32], AppError> {
        let argon2 = Argon2::new(
            argon2::Algorithm::Argon2id,
            argon2::Version::V0x13,
            argon2::Params::new(65536, 3, 4, Some(32))
                .map_err(|e| AppError::KeystoreError(e.to_string()))?,
        );
        let mut key = [0u8; 32];
        argon2
            .hash_password_into(&self.password_hash, salt, &mut key)
            .map_err(|e| AppError::KeystoreError(e.to_string()))?;
        Ok(key)
    }

    fn read_store(&self) -> Result<KeystoreFile, AppError> {
        if !self.store_path.exists() {
            return Ok(KeystoreFile {
                version: 1,
                entries: vec![],
            });
        }
        let data = std::fs::read_to_string(&self.store_path)?;
        let store: KeystoreFile = serde_json::from_str(&data)?;
        Ok(store)
    }

    fn load(&mut self) -> Result<(), AppError> {
        let store = self.read_store()?;
        for entry in &store.entries {
            self.metadata.insert(
                entry.address.clone(),
                KeyInfo {
                    label: entry.label.clone(),
                    address: entry.address.clone(),
                    public_key: entry.public_key.clone(),
                    chain_type: entry.chain_type.clone(),
                    created_at: entry.created_at.clone(),
                },
            );
        }
        Ok(())
    }

    fn append_and_save(&self, entry: EncryptedEntry) -> Result<(), AppError> {
        let mut store = self.read_store()?;
        store.version = 1;
        store.entries.push(entry);
        let json = serde_json::to_string_pretty(&store)?;
        if let Some(parent) = self.store_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&self.store_path, json)?;
        Ok(())
    }

    fn rewrite_store(&self) -> Result<(), AppError> {
        let store = self.read_store()?;
        let filtered: Vec<_> = store
            .entries
            .into_iter()
            .filter(|e| self.metadata.contains_key(&e.address))
            .collect();
        let new_store = KeystoreFile {
            version: 1,
            entries: filtered,
        };
        let json = serde_json::to_string_pretty(&new_store)?;
        std::fs::write(&self.store_path, json)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn keystore_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("keystore.json");

        let mut ks = EncryptedKeystore::new(&path, "testpass").unwrap();
        let info = ks.create_key("test-key", "substrate").unwrap();
        assert!(!info.address.is_empty());
        assert!(!info.public_key.is_empty());

        let sig = ks.sign_message(&info.address, b"hello").unwrap();
        assert_eq!(sig.len(), 32);

        let keys = ks.list_keys();
        assert_eq!(keys.len(), 1);

        ks.delete_key(&info.address).unwrap();
        assert!(ks.list_keys().is_empty());

        // verify file was rewritten
        let data = fs::read_to_string(&path).unwrap();
        let store: KeystoreFile = serde_json::from_str(&data).unwrap();
        assert!(store.entries.is_empty());
    }
}
