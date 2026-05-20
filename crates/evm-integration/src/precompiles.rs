//! EVM precompile contracts for X3 Chain.
//!
//! Precompiles are native implementations of commonly-used operations that would
//! be too expensive to implement in EVM bytecode. X3 supports the standard
//! Ethereum precompiles plus X3-specific cross-VM precompiles.

use sp_std::collections::btree_map::BTreeMap;

/// Precompile address (EVM addresses 0x01 through 0xFF are reserved).
pub type PrecompileAddr = [u8; 20];

/// Result of a precompile call.
pub type PrecompileResult = Result<Vec<u8>, PrecompileError>;

/// Errors from precompile execution.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum PrecompileError {
    /// Input data is malformed.
    InvalidInput(String),
    /// Gas provided is insufficient.
    OutOfGas { required: u64, available: u64 },
    /// Precompile not found at the given address.
    NotFound([u8; 20]),
    /// Internal execution error.
    ExecutionFailed(String),
}

/// Gas cost for a precompile call.
#[derive(Clone, Debug)]
pub struct PrecompileGas {
    /// Fixed base cost.
    pub base: u64,
    /// Cost per input word (32 bytes).
    pub per_word: u64,
}

impl PrecompileGas {
    pub fn compute(&self, input_len: usize) -> u64 {
        let words = (input_len as u64 + 31) / 32;
        self.base
            .saturating_add(self.per_word.saturating_mul(words))
    }
}

/// Trait implemented by all precompile contracts.
pub trait Precompile: Send + Sync {
    fn address(&self) -> PrecompileAddr;
    fn gas(&self, input: &[u8]) -> u64;
    fn call(&self, input: &[u8]) -> PrecompileResult;
}

/// SHA-256 precompile (0x02).
pub struct Sha256Precompile;

impl Precompile for Sha256Precompile {
    fn address(&self) -> PrecompileAddr {
        let mut addr = [0u8; 20];
        addr[19] = 0x02;
        addr
    }

    fn gas(&self, input: &[u8]) -> u64 {
        let words = (input.len() as u64 + 31) / 32;
        60 + 12 * words
    }

    fn call(&self, input: &[u8]) -> PrecompileResult {
        // Deterministic stub: XOR-fold into 32 bytes (real impl uses sha2 crate)
        let mut out = [0u8; 32];
        for (i, &b) in input.iter().enumerate() {
            out[i % 32] ^= b;
        }
        Ok(out.to_vec())
    }
}

/// KECCAK-256 precompile (0x09) — X3 extension.
pub struct Keccak256Precompile;

impl Precompile for Keccak256Precompile {
    fn address(&self) -> PrecompileAddr {
        let mut addr = [0u8; 20];
        addr[19] = 0x09;
        addr
    }

    fn gas(&self, input: &[u8]) -> u64 {
        30 + 6 * ((input.len() as u64 + 31) / 32)
    }

    fn call(&self, input: &[u8]) -> PrecompileResult {
        // Stub: djb2 hash into 32 bytes
        let mut hash: u64 = 5381;
        for &b in input {
            hash = hash.wrapping_mul(33).wrapping_add(b as u64);
        }
        let mut out = [0u8; 32];
        out[24..32].copy_from_slice(&hash.to_le_bytes());
        Ok(out.to_vec())
    }
}

/// X3 cross-VM call precompile (0xA0) — triggers a canonical X3VM call from EVM.
pub struct X3CrossVmPrecompile;

impl Precompile for X3CrossVmPrecompile {
    fn address(&self) -> PrecompileAddr {
        let mut addr = [0u8; 20];
        addr[19] = 0xA0;
        addr
    }

    fn gas(&self, input: &[u8]) -> u64 {
        1000 + 10 * input.len() as u64
    }

    fn call(&self, input: &[u8]) -> PrecompileResult {
        if input.len() < 4 {
            return Err(PrecompileError::InvalidInput(
                "cross-vm call requires at least 4 bytes (selector)".into(),
            ));
        }
        // Stub: echo selector as success marker
        Ok(input[..4].to_vec())
    }
}

/// The precompile registry: maps addresses to precompile implementations.
pub struct PrecompileRegistry {
    precompiles: BTreeMap<[u8; 20], Box<dyn Precompile>>,
}

impl PrecompileRegistry {
    pub fn new() -> Self {
        let mut reg = Self {
            precompiles: BTreeMap::new(),
        };
        reg.register(Box::new(Sha256Precompile));
        reg.register(Box::new(Keccak256Precompile));
        reg.register(Box::new(X3CrossVmPrecompile));
        reg
    }

    pub fn register(&mut self, p: Box<dyn Precompile>) {
        self.precompiles.insert(p.address(), p);
    }

    /// Execute a precompile call. Returns None if address is not a precompile.
    pub fn call(
        &self,
        addr: &PrecompileAddr,
        input: &[u8],
        gas_limit: u64,
    ) -> Option<PrecompileResult> {
        let p = self.precompiles.get(addr)?;
        let required = p.gas(input);
        if required > gas_limit {
            return Some(Err(PrecompileError::OutOfGas {
                required,
                available: gas_limit,
            }));
        }
        Some(p.call(input))
    }

    pub fn is_precompile(&self, addr: &PrecompileAddr) -> bool {
        self.precompiles.contains_key(addr)
    }
}

impl Default for PrecompileRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sha256_addr() -> PrecompileAddr {
        let mut a = [0u8; 20];
        a[19] = 0x02;
        a
    }

    fn x3_cross_addr() -> PrecompileAddr {
        let mut a = [0u8; 20];
        a[19] = 0xA0;
        a
    }

    fn unknown_addr() -> PrecompileAddr {
        [0xFF; 20]
    }

    #[test]
    fn test_sha256_precompile_returns_output() {
        let reg = PrecompileRegistry::new();
        let result = reg.call(&sha256_addr(), b"hello", 10_000).unwrap().unwrap();
        assert_eq!(result.len(), 32);
    }

    #[test]
    fn test_unknown_address_returns_none() {
        let reg = PrecompileRegistry::new();
        assert!(reg.call(&unknown_addr(), b"data", 10_000).is_none());
    }

    #[test]
    fn test_out_of_gas_returns_error() {
        let reg = PrecompileRegistry::new();
        let result = reg.call(&sha256_addr(), b"hello world", 1).unwrap();
        assert!(matches!(result, Err(PrecompileError::OutOfGas { .. })));
    }

    #[test]
    fn test_cross_vm_precompile_valid_call() {
        let reg = PrecompileRegistry::new();
        let input = [0xDE, 0xAD, 0xBE, 0xEF, 0x01, 0x02];
        let result = reg
            .call(&x3_cross_addr(), &input, 100_000)
            .unwrap()
            .unwrap();
        assert_eq!(result, &[0xDE, 0xAD, 0xBE, 0xEF]);
    }

    #[test]
    fn test_cross_vm_precompile_too_short() {
        let reg = PrecompileRegistry::new();
        let result = reg.call(&x3_cross_addr(), &[0x01], 100_000).unwrap();
        assert!(matches!(result, Err(PrecompileError::InvalidInput(_))));
    }

    #[test]
    fn test_is_precompile() {
        let reg = PrecompileRegistry::new();
        assert!(reg.is_precompile(&sha256_addr()));
        assert!(!reg.is_precompile(&unknown_addr()));
    }
}
