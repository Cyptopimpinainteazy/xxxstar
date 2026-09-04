// Output verifier for shadow execution

use alloc::option::Option;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use parity_scale_codec::{Decode, Encode};
use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};

/// Result of a single block replay
#[derive(Debug, Clone, Encode, Decode, TypeInfo, Serialize, Deserialize)]
pub struct ReplayResult {
    /// Block number that was replayed
    pub block_number: u32,
    /// Hash computed during replay
    pub block_hash: [u8; 32],
    /// Expected hash (from chain)
    pub expected_hash: [u8; 32],
    /// Whether hashes match
    pub matches: bool,
    /// Execution time in milliseconds
    pub execution_time_ms: u64,
    /// Error message if failed
    pub error: Option<String>,
}

/// Output verifier trait
pub trait OutputVerifier {
    fn verify_state_root(&self, expected: &[u8; 32], actual: &[u8; 32]) -> bool;
    fn verify_receipts(&self, expected: &[u8], actual: &[u8]) -> bool;
    fn verify_events(&self, expected: &[u8], actual: &[u8]) -> bool;
}

/// Standard output verifier implementation
pub struct StandardVerifier;

impl OutputVerifier for StandardVerifier {
    fn verify_state_root(&self, expected: &[u8; 32], actual: &[u8; 32]) -> bool {
        expected == actual
    }

    fn verify_receipts(&self, expected: &[u8], actual: &[u8]) -> bool {
        expected == actual
    }

    fn verify_events(&self, expected: &[u8], actual: &[u8]) -> bool {
        expected == actual
    }
}

impl Default for StandardVerifier {
    fn default() -> Self {
        Self
    }
}