//! Shred Module - Block data erasure coding and shred generation
//!
//! This module implements the shredding mechanism for Turbine block propagation.
//! It uses Reed-Solomon erasure coding to create data and coding (parity) shreds.

use crate::config::ShredConfig;
use crate::error::{TurbineError, TurbineResult};
use bincode::{deserialize, serialize};
use blake3::Hasher;
use serde::{Deserialize, Serialize};

/// Shred type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ShredType {
    /// Data shred containing actual block data
    Data = 0x00,
    /// Coding shred containing parity data for recovery
    Coding = 0x01,
}

/// Shred flag for additional metadata
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ShredFlag {
    /// Full shred containing complete data
    Full = 0x00,
    /// Partial shred (first chunk of data)
    First = 0x01,
    /// Middle chunk
    Middle = 0x02,
    /// Last chunk
    Last = 0x03,
}

/// Main Shred structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Shred {
    /// Slot number this shred belongs to
    slot: u64,
    /// Shred index within the slot
    shred_index: u32,
    /// Number of shreds in this slot
    num_shreds: u32,
    /// Type of shred (data or coding)
    shred_type: ShredType,
    /// Flag for shred data
    flag: ShredFlag,
    /// Reference to previous block hash (first 32 bytes)
    reference_block: [u8; 32],
    /// Actual payload data
    payload: ShredPayload,
    /// Signature of the leader that produced this shred
    signature: Option<Vec<u8>>,
    /// Hash of the shred for verification
    hash: [u8; 32],
}

impl Shred {
    /// Create a new data shred
    pub fn new_data(
        slot: u64,
        shred_index: u32,
        num_shreds: u32,
        reference_block: [u8; 32],
        payload: ShredPayload,
    ) -> Self {
        let hash = Self::compute_hash(slot, shred_index, &payload);

        Self {
            slot,
            shred_index,
            num_shreds,
            shred_type: ShredType::Data,
            flag: ShredFlag::Full,
            reference_block,
            payload,
            signature: None,
            hash,
        }
    }

    /// Create a new coding shred
    pub fn new_coding(
        slot: u64,
        shred_index: u32,
        num_shreds: u32,
        _coding_position: u32,
        payload: ShredPayload,
    ) -> Self {
        let hash = Self::compute_hash(slot, shred_index, &payload);

        Self {
            slot,
            shred_index,
            num_shreds,
            shred_type: ShredType::Coding,
            flag: ShredFlag::Full,
            reference_block: [0u8; 32], // Coding shreds don't need reference
            payload,
            signature: None,
            hash,
        }
    }

    /// Get slot number
    pub fn slot(&self) -> u64 {
        self.slot
    }

    /// Get shred index
    pub fn shred_index(&self) -> u32 {
        self.shred_index
    }

    /// Get number of shreds
    pub fn num_shreds(&self) -> u32 {
        self.num_shreds
    }

    /// Get shred type
    pub fn shred_type(&self) -> ShredType {
        self.shred_type
    }

    /// Get flag
    pub fn flag(&self) -> ShredFlag {
        self.flag
    }

    /// Get payload reference
    pub fn payload(&self) -> &ShredPayload {
        &self.payload
    }

    /// Get hash
    pub fn hash(&self) -> &[u8; 32] {
        &self.hash
    }

    /// Verify shred integrity
    pub fn verify(&self) -> bool {
        let computed_hash = Self::compute_hash(self.slot, self.shred_index, &self.payload);
        computed_hash == self.hash
    }

    /// Set signature
    pub fn set_signature(&mut self, signature: [u8; 64]) {
        self.signature = Some(signature.to_vec());
    }

    /// Get signature
    pub fn signature(&self) -> Option<&[u8]> {
        self.signature.as_deref()
    }

    /// Compute hash of shred
    fn compute_hash(slot: u64, index: u32, payload: &ShredPayload) -> [u8; 32] {
        let mut hasher = Hasher::new();
        hasher.update(&slot.to_le_bytes());
        hasher.update(&index.to_le_bytes());
        hasher.update(payload.as_bytes());
        *hasher.finalize().as_bytes()
    }

    /// Serialize shred to bytes
    pub fn to_bytes(&self) -> TurbineResult<Vec<u8>> {
        serialize(self).map_err(|e| TurbineError::SerializationError(e.to_string()))
    }

    /// Deserialize shred from bytes
    pub fn from_bytes(data: &[u8]) -> TurbineResult<Self> {
        deserialize(data).map_err(|e| TurbineError::DeserializationError(e.to_string()))
    }
}

/// Shred payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShredPayload {
    /// Data content
    data: Vec<u8>,
}

impl ShredPayload {
    /// Create new payload from data
    pub fn new(data: Vec<u8>) -> Self {
        Self { data }
    }

    /// Get data reference
    pub fn as_bytes(&self) -> &[u8] {
        &self.data
    }

    /// Get data as mutable slice
    pub fn as_mut_slice(&mut self) -> &mut Vec<u8> {
        &mut self.data
    }

    /// Get data length
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Check if payload is empty
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}

/// Reed-Solomon erasure coding context
pub struct ErasureCode {
    /// Number of data chunks
    data_shreds: usize,
    /// Number of parity chunks
    coding_shreds: usize,
    /// Matrix for encoding
    encoding_matrix: Vec<Vec<f64>>,
}

impl ErasureCode {
    /// Create new erasure code context
    pub fn new(data_shreds: usize, coding_shreds: usize) -> Self {
        let total = data_shreds + coding_shreds;
        let encoding_matrix = Self::generate_matrix(data_shreds, total);

        Self {
            data_shreds,
            coding_shreds,
            encoding_matrix,
        }
    }

    /// Generate encoding matrix using Vandermonde matrix
    fn generate_matrix(k: usize, n: usize) -> Vec<Vec<f64>> {
        let mut matrix = vec![vec![0.0; k]; n];

        for (i, row) in matrix.iter_mut().enumerate().take(n) {
            for (j, cell) in row.iter_mut().enumerate().take(k) {
                // Use Galois Field arithmetic (simplified with floating point for demo)
                *cell = Self::gf_pow(j as u32, i as u32, k as u32);
            }
        }

        matrix
    }

    /// GF(2^8) exponentiation (simplified)
    fn gf_pow(base: u32, exp: u32, m: u32) -> f64 {
        let mut result = 1u32;
        let mut b = base % 256;
        let mut e = exp;

        while e > 0 {
            if e & 1 == 1 {
                result = Self::gf_mul(result, b, m);
            }
            b = Self::gf_mul(b, b, m);
            e >>= 1;
        }

        result as f64
    }

    /// GF(2^8) multiplication
    fn gf_mul(a: u32, b: u32, m: u32) -> u32 {
        let mut result = 0u32;
        let mut a = a;
        let mut b = b;

        while b > 0 {
            if b & 1 == 1 {
                result ^= a;
            }
            let hi_bit = a & 0x80;
            a <<= 1;
            if hi_bit != 0 {
                a ^= m;
            }
            b >>= 1;
        }

        result
    }

    /// Encode data into shreds
    pub fn encode(&self, data: &[u8]) -> (Vec<Vec<u8>>, Vec<Vec<u8>>) {
        let chunk_size = data.len().div_ceil(self.data_shreds);

        // Pad data if necessary
        let mut padded_data = data.to_vec();
        while padded_data.len() < chunk_size * self.data_shreds {
            padded_data.push(0);
        }

        // Split into data chunks
        let mut data_chunks: Vec<Vec<u8>> = Vec::with_capacity(self.data_shreds);
        for i in 0..self.data_shreds {
            let start = i * chunk_size;
            let end = std::cmp::min(start + chunk_size, padded_data.len());
            data_chunks.push(padded_data[start..end].to_vec());
        }

        // Generate coding chunks
        let mut coding_chunks: Vec<Vec<u8>> = Vec::with_capacity(self.coding_shreds);

        for i in 0..self.coding_shreds {
            let row = &self.encoding_matrix[self.data_shreds + i];
            let mut coding_chunk = vec![0u8; chunk_size];

            for byte_idx in 0..chunk_size {
                let mut value: u8 = 0;
                for j in 0..self.data_shreds {
                    if byte_idx < data_chunks[j].len() {
                        value ^=
                            Self::gf_mul(data_chunks[j][byte_idx].into(), row[j] as u32, 285) as u8;
                    }
                }
                coding_chunk[byte_idx] = value;
            }

            coding_chunks.push(coding_chunk);
        }

        (data_chunks, coding_chunks)
    }

    /// Decode data from available shreds
    pub fn decode(
        &self,
        data_chunks: &[Vec<u8>],
        _coding_chunks: &[Vec<u8>],
        _required_indices: &[usize],
    ) -> Option<Vec<u8>> {
        // Simplified decode - in production would use proper Gaussian elimination
        if data_chunks.len() >= self.data_shreds {
            // We have enough data chunks
            let chunk_size = data_chunks[0].len();
            let mut result = Vec::with_capacity(chunk_size * self.data_shreds);

            for chunk in data_chunks.iter().take(self.data_shreds) {
                result.extend_from_slice(chunk);
            }

            Some(result)
        } else {
            // Need to use coding chunks for recovery
            // This is a simplified implementation
            None
        }
    }
}

/// Shredder for creating shreds from block data
pub struct Shredder {
    config: ShredConfig,
    erasure_code: ErasureCode,
}

impl Shredder {
    /// Create new shredder
    pub fn new(config: ShredConfig) -> Self {
        let erasure_code = ErasureCode::new(config.data_shreds, config.coding_shreds);

        Self {
            config,
            erasure_code,
        }
    }

    /// Create shreds from block data
    pub fn create_shreds(&self, slot: u64, block_data: Vec<u8>) -> TurbineResult<Vec<Shred>> {
        let reference_block = Self::compute_reference(&block_data);

        // Encode data using erasure coding
        let (data_chunks, coding_chunks) = self.erasure_code.encode(&block_data);

        let mut shreds = Vec::new();
        let num_shreds = (self.config.data_shreds + self.config.coding_shreds) as u32;

        // Create data shreds
        for (i, chunk) in data_chunks.into_iter().enumerate() {
            let shred = Shred::new_data(
                slot,
                i as u32,
                num_shreds,
                reference_block,
                ShredPayload::new(chunk),
            );
            shreds.push(shred);
        }

        // Create coding shreds
        for (i, chunk) in coding_chunks.into_iter().enumerate() {
            let shred = Shred::new_coding(
                slot,
                (self.config.data_shreds + i) as u32,
                num_shreds,
                i as u32,
                ShredPayload::new(chunk),
            );
            shreds.push(shred);
        }

        Ok(shreds)
    }

    /// Compute reference block hash
    fn compute_reference(block_data: &[u8]) -> [u8; 32] {
        let mut hasher = Hasher::new();
        hasher.update(block_data);
        let hash = hasher.finalize();
        let mut result = [0u8; 32];
        result.copy_from_slice(hash.as_bytes());
        result
    }

    /// Get config reference
    pub fn config(&self) -> &ShredConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shred_creation() {
        let config = ShredConfig::default();
        let shredder = Shredder::new(config);

        let data = vec![0u8; 16000];
        let shreds = shredder.create_shreds(1, data).unwrap();

        assert!(!shreds.is_empty());
    }

    #[test]
    fn test_shred_verification() {
        let config = ShredConfig::default();
        let shredder = Shredder::new(config);

        let data = vec![1u8; 8000];
        let shreds = shredder.create_shreds(1, data).unwrap();

        for shred in &shreds {
            assert!(shred.verify());
        }
    }
}
