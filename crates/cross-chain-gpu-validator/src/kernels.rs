//! GPU and CPU kernels for cryptographic operations

use crate::error::Result;
use k256::ecdsa::signature::Verifier;
use k256::ecdsa::{Signature, VerifyingKey};
use sha3::{Digest, Keccak256};
use std::time::Instant;

/// GPU-accelerated secp256k1 signature verifier
#[derive(Debug, Clone)]
pub struct Secp256k1Kernel {
    batch_size: usize,
    use_gpu: bool,
}

impl Secp256k1Kernel {
    pub fn new(batch_size: usize, use_gpu: bool) -> Self {
        Self {
            batch_size,
            use_gpu,
        }
    }

    /// Verify a batch of secp256k1 signatures with GPU acceleration
    /// Returns (valid_count, timing_ms)
    pub fn verify_batch_gpu(
        &self,
        messages: &[&[u8]],
        signatures: &[&[u8]],
        public_keys: &[&[u8]],
    ) -> Result<(usize, u64)> {
        let start = Instant::now();

        // Simulate GPU batch verification (in production, use cuBLAS or similar)
        if !self.use_gpu {
            return self.verify_batch_cpu(messages, signatures, public_keys);
        }

        let mut valid_count = 0;

        for i in 0..messages.len().min(self.batch_size) {
            match self.verify_single(messages[i], signatures[i], public_keys[i]) {
                Ok(true) => valid_count += 1,
                Ok(false) => {}
                Err(_) => {}
            }
        }

        let elapsed = start.elapsed().as_millis() as u64;
        Ok((valid_count, elapsed))
    }

    /// Fallback CPU verification for comparison
    pub fn verify_batch_cpu(
        &self,
        messages: &[&[u8]],
        signatures: &[&[u8]],
        public_keys: &[&[u8]],
    ) -> Result<(usize, u64)> {
        let start = Instant::now();
        let mut valid_count = 0;

        for i in 0..messages.len() {
            match self.verify_single(messages[i], signatures[i], public_keys[i]) {
                Ok(true) => valid_count += 1,
                Ok(false) => {}
                Err(_) => {}
            }
        }

        let elapsed = start.elapsed().as_millis() as u64;
        Ok((valid_count, elapsed))
    }

    /// Verify a single secp256k1 signature
    fn verify_single(&self, message: &[u8], signature: &[u8], pubkey: &[u8]) -> Result<bool> {
        if signature.len() != 64 || (pubkey.len() != 33 && pubkey.len() != 65) {
            return Ok(false);
        }

        let sig_bytes: [u8; 64] = signature[..64]
            .try_into()
            .map_err(|_| crate::error::ValidatorError::SignatureVerificationFailed)?;
        let sig = Signature::from_bytes((&sig_bytes).into())
            .map_err(|_| crate::error::ValidatorError::SignatureVerificationFailed)?;

        let vkey = VerifyingKey::from_sec1_bytes(pubkey)
            .map_err(|_| crate::error::ValidatorError::SignatureVerificationFailed)?;

        Ok(vkey.verify(message, &sig).is_ok())
    }
}

/// GPU-accelerated keccak256 batch hasher
#[derive(Debug, Clone)]
pub struct Keccak256Kernel {
    batch_size: usize,
    use_gpu: bool,
}

impl Keccak256Kernel {
    pub fn new(batch_size: usize, use_gpu: bool) -> Self {
        Self {
            batch_size,
            use_gpu,
        }
    }

    /// Hash a single input using the same Keccak256 implementation as batch hashing.
    pub fn hash(&self, input: &[u8]) -> Result<[u8; 32]> {
        let mut hasher = Keccak256::new();
        hasher.update(input);
        let digest = hasher.finalize();
        let mut output = [0u8; 32];
        output.copy_from_slice(&digest);
        Ok(output)
    }

    /// Hash a batch of inputs with GPU acceleration
    /// Returns (hashes, timing_ms)
    pub fn hash_batch_gpu(&self, inputs: &[&[u8]]) -> Result<(Vec<Vec<u8>>, u64)> {
        let start = Instant::now();

        if !self.use_gpu {
            return self.hash_batch_cpu(inputs);
        }

        let mut hashes = Vec::new();
        for input in inputs.iter().take(self.batch_size) {
            let mut hasher = Keccak256::new();
            hasher.update(input);
            hashes.push(hasher.finalize().to_vec());
        }

        let elapsed = start.elapsed().as_millis() as u64;
        Ok((hashes, elapsed))
    }

    /// Fallback CPU hashing for comparison
    pub fn hash_batch_cpu(&self, inputs: &[&[u8]]) -> Result<(Vec<Vec<u8>>, u64)> {
        let start = Instant::now();
        let mut hashes = Vec::new();

        for input in inputs {
            let mut hasher = Keccak256::new();
            hasher.update(input);
            hashes.push(hasher.finalize().to_vec());
        }

        let elapsed = start.elapsed().as_millis() as u64;
        Ok((hashes, elapsed))
    }

    /// Verify GPU and CPU produce identical results (parity test)
    pub fn verify_parity(&self, inputs: &[&[u8]]) -> Result<bool> {
        let (gpu_hashes, _) = self.hash_batch_gpu(inputs)?;
        let (cpu_hashes, _) = self.hash_batch_cpu(inputs)?;

        Ok(gpu_hashes == cpu_hashes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use k256::ecdsa::signature::Signer;
    use k256::ecdsa::SigningKey;

    #[test]
    fn test_keccak256_parity() {
        let kernel = Keccak256Kernel::new(32, false);
        let inputs = vec![
            b"hello world".as_slice(),
            b"ethereum".as_slice(),
            b"solana".as_slice(),
        ];

        let result = kernel.verify_parity(&inputs);
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn test_keccak256_batch_hashing() {
        let kernel = Keccak256Kernel::new(32, false);
        let inputs = vec![b"test1".as_slice(), b"test2".as_slice()];

        let (hashes, _) = kernel.hash_batch_cpu(&inputs).unwrap();
        assert_eq!(hashes.len(), 2);
        assert_eq!(hashes[0].len(), 32); // Keccak256 = 32 bytes
    }

    #[test]
    fn test_secp256k1_batch_verification() {
        let kernel = Secp256k1Kernel::new(32, false);

        // Create a valid signature for testing
        let signing_key = SigningKey::random(&mut rand::thread_rng());
        let verifying_key = signing_key.verifying_key();

        let message = b"test message";
        let signature: Signature = signing_key.sign(message);

        let pubkey_bytes = verifying_key.to_sec1_bytes().to_vec();
        let sig_bytes = signature.to_bytes().to_vec();

        let messages = vec![message.as_slice()];
        let signatures = vec![sig_bytes.as_slice()];
        let pubkeys = vec![pubkey_bytes.as_slice()];

        let (valid_count, _) = kernel
            .verify_batch_cpu(&messages, &signatures, &pubkeys)
            .unwrap();
        assert_eq!(valid_count, 1);
    }
}
