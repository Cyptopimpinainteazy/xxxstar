//! Transaction Module

use crate::error::{GulfstreamError, GulfstreamResult};
use bincode::{serialize, deserialize};
use blake3::Hasher;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// Transaction status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransactionStatus {
    /// Transaction is pending
    Pending,
    /// Transaction has been forwarded
    Forwarded,
    /// Transaction has been included in a block
    Confirmed,
    /// Transaction has expired
    Expired,
    /// Transaction failed
    Failed,
}

/// Transaction metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionMeta {
    /// Creation timestamp
    pub created_at: u64,
    /// Slot the transaction was created in
    pub created_slot: u64,
    /// Priority level (0 = highest)
    pub priority: u8,
    /// Whether the transaction has been forwarded
    pub forwarded: bool,
    /// Number of forwarding attempts
    pub forward_attempts: u32,
}

impl Default for TransactionMeta {
    fn default() -> Self {
        Self {
            created_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
            created_slot: 0,
            priority: 0,
            forwarded: false,
            forward_attempts: 0,
        }
    }
}

/// Main transaction structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    /// Transaction data (signed)
    pub data: Vec<u8>,
    /// Transaction hash
    hash: String,
    /// Signature
    pub signature: Vec<u8>,
    /// Recent blockhash
    pub recent_blockhash: [u8; 32],
    /// Fee payer
    pub fee_payer: [u8; 32],
    /// Metadata
    meta: TransactionMeta,
}

impl Transaction {
    /// Create new transaction
    pub fn new(
        data: Vec<u8>,
        signature: Vec<u8>,
        recent_blockhash: [u8; 32],
        fee_payer: [u8; 32],
    ) -> Self {
        let hash = Self::compute_hash(&data, &signature);
        
        Self {
            data,
            hash,
            signature,
            recent_blockhash,
            fee_payer,
            meta: TransactionMeta::default(),
        }
    }

    /// Get transaction hash
    pub fn hash(&self) -> &str {
        &self.hash
    }

    /// Get metadata reference
    pub fn meta(&self) -> &TransactionMeta {
        &self.meta
    }

    /// Get mutable metadata
    pub fn meta_mut(&mut self) -> &mut TransactionMeta {
        &mut self.meta
    }

    /// Set slot
    pub fn set_slot(&mut self, slot: u64) {
        self.meta.created_slot = slot;
    }

    /// Set priority
    pub fn set_priority(&mut self, priority: u8) {
        self.meta.priority = priority.min(4);
    }

    /// Mark as forwarded
    pub fn mark_forwarded(&mut self) {
        self.meta.forwarded = true;
        self.meta.forward_attempts += 1;
    }

    /// Compute transaction hash
    fn compute_hash(data: &[u8], signature: &[u8]) -> String {
        let mut hasher = Hasher::new();
        hasher.update(data);
        hasher.update(signature);
        let hash = hasher.finalize();
        hex::encode(hash.as_bytes())
    }

    /// Validate transaction
    pub fn validate(&self) -> GulfstreamResult<()> {
        if self.data.is_empty() {
            return Err(GulfstreamError::InvalidTransaction("Empty data".into()));
        }
        
        if self.signature.is_empty() {
            return Err(GulfstreamError::InvalidTransaction("Empty signature".into()));
        }
        
        // Verify hash
        let computed_hash = Self::compute_hash(&self.data, &self.signature);
        if computed_hash != self.hash {
            return Err(GulfstreamError::InvalidTransaction("Hash mismatch".into()));
        }
        
        Ok(())
    }

    /// Serialize to bytes
    pub fn to_bytes(&self) -> GulfstreamResult<Vec<u8>> {
        serialize(self).map_err(|e| GulfstreamError::SerializationError(e.to_string()))
    }

    /// Deserialize from bytes
    pub fn from_bytes(data: &[u8]) -> GulfstreamResult<Self> {
        deserialize(data).map_err(|e| GulfstreamError::DeserializationError(e.to_string()))
    }
}

/// Simple hex encoding helper
mod hex {
    const HEX_CHARS: &[u8; 16] = b"0123456789abcdef";
    
    pub fn encode(data: &[u8]) -> String {
        let mut result = String::with_capacity(data.len() * 2);
        for byte in data {
            result.push(HEX_CHARS[(byte >> 4) as usize] as char);
            result.push(HEX_CHARS[(byte & 0x0f) as usize] as char);
        }
        result
    }
}