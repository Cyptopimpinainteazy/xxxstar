//! Vendor-neutral accelerator abstraction for X3 validator batch work.
//!
//! Consensus remains CPU-deterministic. Accelerator backends are optional
//! sidecars for batchable work such as hashing, signature checks, and Merkle
//! tree construction.

use blake2::{Blake2b512, Digest as BlakeDigest};
use ed25519_dalek::{Signature as Ed25519Signature, Verifier, VerifyingKey};
use secp256k1::{ecdsa::Signature as Secp256k1Signature, Message, PublicKey, Secp256k1};
use sha2::{Digest as ShaDigest, Sha256};

/// Accelerator backend selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendKind {
    Cpu,
    OpenCl,
    Vulkan,
    Wgpu,
    CudaOptional,
}

impl BackendKind {
    pub fn from_env_value(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "opencl" => Self::OpenCl,
            "vulkan" => Self::Vulkan,
            "wgpu" => Self::Wgpu,
            "cuda" | "cuda_optional" | "cuda-optional" => Self::CudaOptional,
            _ => Self::Cpu,
        }
    }
}

/// Accelerator execution errors.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum AccelError {
    #[error("backend {0:?} is not available")]
    BackendUnavailable(BackendKind),
    #[error("accelerator output diverged from CPU baseline")]
    ParityMismatch,
    #[error("invalid batch input: {0}")]
    InvalidInput(&'static str),
}

/// secp256k1 verification job.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Secp256k1VerifyJob {
    pub message_hash: [u8; 32],
    pub signature: [u8; 64],
    pub public_key: Vec<u8>,
}

/// Ed25519 verification job.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ed25519VerifyJob {
    pub message: Vec<u8>,
    pub signature: [u8; 64],
    pub public_key: [u8; 32],
}

/// Optional accelerator backend.
pub trait AccelBackend: Send + Sync {
    fn name(&self) -> &'static str;

    fn verify_secp256k1_batch(&self, batch: &[Secp256k1VerifyJob])
        -> Result<Vec<bool>, AccelError>;

    fn verify_ed25519_batch(&self, batch: &[Ed25519VerifyJob]) -> Result<Vec<bool>, AccelError>;

    fn keccak256_batch(&self, inputs: &[Vec<u8>]) -> Result<Vec<[u8; 32]>, AccelError>;

    fn sha256_batch(&self, inputs: &[Vec<u8>]) -> Result<Vec<[u8; 32]>, AccelError>;

    fn blake2b256_batch(&self, inputs: &[Vec<u8>]) -> Result<Vec<[u8; 32]>, AccelError>;

    fn build_merkle_root(&self, leaves: &[[u8; 32]]) -> Result<[u8; 32], AccelError>;
}

/// Canonical CPU implementation. This is the consensus truth source.
#[derive(Debug, Default, Clone, Copy)]
pub struct CpuBackend;

impl CpuBackend {
    pub fn new() -> Self {
        Self
    }
}

impl AccelBackend for CpuBackend {
    fn name(&self) -> &'static str {
        "cpu"
    }

    fn verify_secp256k1_batch(
        &self,
        batch: &[Secp256k1VerifyJob],
    ) -> Result<Vec<bool>, AccelError> {
        Ok(batch
            .iter()
            .map(|job| {
                let Ok(message) = Message::from_digest_slice(&job.message_hash) else {
                    return false;
                };
                let Ok(signature) = Secp256k1Signature::from_compact(&job.signature) else {
                    return false;
                };
                let Ok(public_key) = PublicKey::from_slice(&job.public_key) else {
                    return false;
                };
                Secp256k1::verification_only()
                    .verify_ecdsa(&message, &signature, &public_key)
                    .is_ok()
            })
            .collect())
    }

    fn verify_ed25519_batch(&self, batch: &[Ed25519VerifyJob]) -> Result<Vec<bool>, AccelError> {
        Ok(batch
            .iter()
            .map(|job| {
                let Ok(public_key) = VerifyingKey::from_bytes(&job.public_key) else {
                    return false;
                };
                let signature = Ed25519Signature::from_bytes(&job.signature);
                public_key.verify(&job.message, &signature).is_ok()
            })
            .collect())
    }

    fn keccak256_batch(&self, inputs: &[Vec<u8>]) -> Result<Vec<[u8; 32]>, AccelError> {
        Ok(inputs
            .iter()
            .map(|input| {
                let hash = keccak_hash::keccak(input);
                let mut output = [0u8; 32];
                output.copy_from_slice(&hash);
                output
            })
            .collect())
    }

    fn sha256_batch(&self, inputs: &[Vec<u8>]) -> Result<Vec<[u8; 32]>, AccelError> {
        Ok(inputs
            .iter()
            .map(|input| {
                let mut hasher = Sha256::new();
                ShaDigest::update(&mut hasher, input);
                hasher.finalize().into()
            })
            .collect())
    }

    fn blake2b256_batch(&self, inputs: &[Vec<u8>]) -> Result<Vec<[u8; 32]>, AccelError> {
        Ok(inputs
            .iter()
            .map(|input| {
                let mut hasher = Blake2b512::new();
                BlakeDigest::update(&mut hasher, input);
                let digest = hasher.finalize();
                let mut output = [0u8; 32];
                output.copy_from_slice(&digest[..32]);
                output
            })
            .collect())
    }

    fn build_merkle_root(&self, leaves: &[[u8; 32]]) -> Result<[u8; 32], AccelError> {
        if leaves.is_empty() {
            return Ok([0u8; 32]);
        }

        let mut level = leaves.to_vec();
        while level.len() > 1 {
            let mut next = Vec::with_capacity(level.len().div_ceil(2));
            for pair in level.chunks(2) {
                let left = pair[0];
                let right = pair.get(1).copied().unwrap_or(left);
                let mut input = Vec::with_capacity(64);
                input.extend_from_slice(&left);
                input.extend_from_slice(&right);
                next.push(self.sha256_batch(&[input])?[0]);
            }
            level = next;
        }
        Ok(level[0])
    }
}

/// Fail-closed adapter for future non-CUDA backends.
#[derive(Debug, Clone, Copy)]
pub struct UnavailableBackend {
    kind: BackendKind,
}

impl UnavailableBackend {
    pub fn new(kind: BackendKind) -> Self {
        Self { kind }
    }
}

impl AccelBackend for UnavailableBackend {
    fn name(&self) -> &'static str {
        match self.kind {
            BackendKind::OpenCl => "opencl-unavailable",
            BackendKind::Vulkan => "vulkan-unavailable",
            BackendKind::Wgpu => "wgpu-unavailable",
            BackendKind::CudaOptional => "cuda-unavailable",
            BackendKind::Cpu => "cpu-unavailable",
        }
    }

    fn verify_secp256k1_batch(
        &self,
        _batch: &[Secp256k1VerifyJob],
    ) -> Result<Vec<bool>, AccelError> {
        Err(AccelError::BackendUnavailable(self.kind))
    }

    fn verify_ed25519_batch(&self, _batch: &[Ed25519VerifyJob]) -> Result<Vec<bool>, AccelError> {
        Err(AccelError::BackendUnavailable(self.kind))
    }

    fn keccak256_batch(&self, _inputs: &[Vec<u8>]) -> Result<Vec<[u8; 32]>, AccelError> {
        Err(AccelError::BackendUnavailable(self.kind))
    }

    fn sha256_batch(&self, _inputs: &[Vec<u8>]) -> Result<Vec<[u8; 32]>, AccelError> {
        Err(AccelError::BackendUnavailable(self.kind))
    }

    fn blake2b256_batch(&self, _inputs: &[Vec<u8>]) -> Result<Vec<[u8; 32]>, AccelError> {
        Err(AccelError::BackendUnavailable(self.kind))
    }

    fn build_merkle_root(&self, _leaves: &[[u8; 32]]) -> Result<[u8; 32], AccelError> {
        Err(AccelError::BackendUnavailable(self.kind))
    }
}

/// Select a backend from `X3_ACCEL`; unsupported accelerators fail over to CPU
/// unless strict mode is requested.
pub fn select_backend() -> Box<dyn AccelBackend> {
    select_backend_with(
        std::env::var("X3_ACCEL").unwrap_or_default().as_str(),
        false,
    )
}

pub fn select_backend_with(value: &str, strict: bool) -> Box<dyn AccelBackend> {
    match BackendKind::from_env_value(value) {
        BackendKind::Cpu => Box::new(CpuBackend::new()),
        kind if strict => Box::new(UnavailableBackend::new(kind)),
        _ => Box::new(CpuBackend::new()),
    }
}

/// Execute a hash batch and compare accelerator output to CPU truth.
pub fn keccak256_with_parity<B: AccelBackend + ?Sized>(
    backend: &B,
    inputs: &[Vec<u8>],
) -> Result<Vec<[u8; 32]>, AccelError> {
    let accelerated = backend.keccak256_batch(inputs)?;
    let cpu = CpuBackend::new().keccak256_batch(inputs)?;
    if accelerated != cpu {
        return Err(AccelError::ParityMismatch);
    }
    Ok(accelerated)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cpu_hash_batches_are_deterministic() {
        let backend = CpuBackend::new();
        let inputs = vec![b"alpha".to_vec(), b"beta".to_vec()];

        let first = backend.sha256_batch(&inputs).unwrap();
        let second = backend.sha256_batch(&inputs).unwrap();

        assert_eq!(first, second);
        assert_eq!(first.len(), 2);
    }

    #[test]
    fn merkle_root_handles_empty_and_odd_levels() {
        let backend = CpuBackend::new();
        let leaves = backend
            .sha256_batch(&[b"a".to_vec(), b"b".to_vec(), b"c".to_vec()])
            .unwrap();

        assert_eq!(backend.build_merkle_root(&[]).unwrap(), [0u8; 32]);
        assert_ne!(backend.build_merkle_root(&leaves).unwrap(), [0u8; 32]);
    }

    #[test]
    fn backend_selection_defaults_to_cpu_for_cuda_bypass() {
        let backend = select_backend_with("cuda", false);

        assert_eq!(backend.name(), "cpu");
    }

    #[test]
    fn strict_unavailable_backend_fails_closed() {
        let backend = select_backend_with("vulkan", true);

        assert!(matches!(
            backend.keccak256_batch(&[b"x".to_vec()]),
            Err(AccelError::BackendUnavailable(BackendKind::Vulkan))
        ));
    }

    #[test]
    fn parity_wrapper_accepts_cpu_backend() {
        let backend = CpuBackend::new();
        let inputs = vec![b"hello".to_vec()];

        assert_eq!(
            keccak256_with_parity(&backend, &inputs).unwrap(),
            backend.keccak256_batch(&inputs).unwrap()
        );
    }

    #[test]
    fn cpu_backend_verifies_real_signature_batches() {
        use ed25519_dalek::{Signer, SigningKey};
        use secp256k1::SecretKey;

        let backend = CpuBackend::new();

        let secp = Secp256k1::new();
        let secret = SecretKey::from_slice(&[7u8; 32]).unwrap();
        let public_key = PublicKey::from_secret_key(&secp, &secret);
        let message_hash = [9u8; 32];
        let message = Message::from_digest_slice(&message_hash).unwrap();
        let signature = secp.sign_ecdsa(&message, &secret).serialize_compact();
        let secp_jobs = vec![Secp256k1VerifyJob {
            message_hash,
            signature,
            public_key: public_key.serialize().to_vec(),
        }];

        assert_eq!(
            backend.verify_secp256k1_batch(&secp_jobs).unwrap(),
            vec![true]
        );

        let signing_key = SigningKey::from_bytes(&[11u8; 32]);
        let message = b"x3 accelerator parity";
        let signature = signing_key.sign(message).to_bytes();
        let ed_jobs = vec![Ed25519VerifyJob {
            message: message.to_vec(),
            signature,
            public_key: signing_key.verifying_key().to_bytes(),
        }];

        assert_eq!(backend.verify_ed25519_batch(&ed_jobs).unwrap(), vec![true]);
    }
}
