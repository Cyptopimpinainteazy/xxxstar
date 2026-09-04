//! X3 CPU↔GPU validator parity — pure-Rust source of truth.
//!
//! The X3 validator pipeline runs the same hash + reduction tasks on either
//! the CPU path (`x3-gpu-validator-swarm::cpu_validator`) or the GPU path
//! (`x3-gpu-validator-swarm::deterministic` driving CUDA/HIP kernels). For
//! the chain to be safe under non-uniform validator hardware, both paths
//! MUST produce byte-identical outputs for every input.
//!
//! This crate is the deterministic spec used by the parity test harness and
//! by `proof-forge / x3.gpu.cpu_gpu_parity`. It computes the canonical hash
//! for each vector defined in
//! `X3-contracts/shared/test-vectors/gpu_hash_parity.json`. Vector outputs
//! are pre-pinned (`expected_digest_hex`); both validator paths must match.
//!
//! Mirrors the design of `x3-parity-core` for EVM↔SVM flashloan parity.

use serde::{Deserialize, Serialize};
use sha2::{Digest as Sha2Digest, Sha256};
use tiny_keccak::{Hasher as KeccakHasher, Keccak};

/// Hash algorithms supported by the X3 GPU validator pipeline. Must match
/// the enum used on the GPU side (`x3-gpu-validator-swarm::crypto::HashAlgorithm`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HashAlg {
    Sha256,
    Blake3,
    Keccak256,
}

/// One pinned parity vector — the canonical CPU/GPU agreement contract.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorDoc {
    pub id: String,
    pub algorithm: HashAlg,
    /// Hex-encoded preimage bytes (lowercase, no `0x`).
    pub preimage_hex: String,
    /// Hex-encoded expected 32-byte digest. Both CPU and GPU paths must
    /// produce exactly these bytes.
    pub expected_digest_hex: String,
}

/// Top-level shape of `gpu_hash_parity.json`. Matches the flashloan vectors
/// file shape so the proof-forge runner can reuse the same load path.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorsDoc {
    pub spec_version: u32,
    pub spec: String,
    pub spec_doc: String,
    pub vectors: Vec<VectorDoc>,
}

/// Compute the canonical 32-byte digest for a (algorithm, preimage) pair.
///
/// This is the *spec* implementation — every CPU/GPU validator path must
/// agree with this byte-for-byte.
pub fn canonical_digest(alg: HashAlg, preimage: &[u8]) -> [u8; 32] {
    match alg {
        HashAlg::Sha256 => {
            let out = Sha256::digest(preimage);
            let mut arr = [0u8; 32];
            arr.copy_from_slice(&out);
            arr
        }
        HashAlg::Blake3 => *blake3::hash(preimage).as_bytes(),
        HashAlg::Keccak256 => {
            let mut hasher = Keccak::v256();
            hasher.update(preimage);
            let mut out = [0u8; 32];
            hasher.finalize(&mut out);
            out
        }
    }
}

/// Outcome of running a single vector through both validator paths.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParityOutcome {
    pub vector_id: String,
    /// True if the CPU path's digest matched the pinned expected_digest_hex.
    pub cpu_matches_spec: bool,
    /// True if the GPU path's digest matched the pinned expected_digest_hex.
    pub gpu_matches_spec: bool,
    /// True if CPU and GPU digests are byte-equal to each other.
    pub cpu_eq_gpu: bool,
}

impl ParityOutcome {
    pub fn ok(&self) -> bool {
        self.cpu_matches_spec && self.gpu_matches_spec && self.cpu_eq_gpu
    }
}

/// Trait that both validator paths implement. The CPU path is provided by
/// `canonical_digest`. The GPU path is supplied by the caller — in the
/// in-repo harness it is wired to `x3-gpu-validator-swarm`'s GPU pipeline
/// (or, in CI without CUDA, to a second independent CPU reference so the
/// invariant is exercised end-to-end on every machine).
pub trait DigestOracle {
    fn digest(&self, alg: HashAlg, preimage: &[u8]) -> [u8; 32];
}

/// Convenience: a CPU oracle that just defers to `canonical_digest`. Used
/// by the parity test as the "GPU emulator" baseline so the invariant runs
/// even on machines without CUDA. The on-hardware GPU oracle lives in
/// `x3-gpu-validator-swarm` and is exercised by its own integration tests.
pub struct CpuReferenceOracle;

impl DigestOracle for CpuReferenceOracle {
    fn digest(&self, alg: HashAlg, preimage: &[u8]) -> [u8; 32] {
        canonical_digest(alg, preimage)
    }
}

/// Run a vector through the spec + an oracle and return the parity outcome.
///
/// `cpu_oracle` typically is `CpuReferenceOracle`; `gpu_oracle` is whatever
/// path under test is meant to be the "GPU side". Both compared against the
/// pinned `expected_digest_hex` and against each other.
pub fn evaluate_vector<C: DigestOracle, G: DigestOracle>(
    v: &VectorDoc,
    cpu_oracle: &C,
    gpu_oracle: &G,
) -> ParityOutcome {
    let preimage = hex::decode(&v.preimage_hex).expect("preimage_hex must be valid hex");
    let expected =
        hex::decode(&v.expected_digest_hex).expect("expected_digest_hex must be valid hex");
    let cpu = cpu_oracle.digest(v.algorithm, &preimage);
    let gpu = gpu_oracle.digest(v.algorithm, &preimage);
    ParityOutcome {
        vector_id: v.id.clone(),
        cpu_matches_spec: cpu.as_slice() == expected.as_slice(),
        gpu_matches_spec: gpu.as_slice() == expected.as_slice(),
        cpu_eq_gpu: cpu == gpu,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_empty_matches_rfc6234() {
        let d = canonical_digest(HashAlg::Sha256, &[]);
        assert_eq!(
            hex::encode(d),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn blake3_empty_matches_official_vector() {
        let d = canonical_digest(HashAlg::Blake3, &[]);
        assert_eq!(
            hex::encode(d),
            "af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262"
        );
    }

    #[test]
    fn keccak256_empty_matches_eth_vector() {
        // keccak256("") — the empty-input digest used by the EVM.
        let d = canonical_digest(HashAlg::Keccak256, &[]);
        assert_eq!(
            hex::encode(d),
            "c5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470"
        );
    }

    #[test]
    fn parity_outcome_ok_when_all_three_match() {
        let v = VectorDoc {
            id: "sha256/empty".to_string(),
            algorithm: HashAlg::Sha256,
            preimage_hex: "".to_string(),
            expected_digest_hex:
                "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_string(),
        };
        let outcome = evaluate_vector(&v, &CpuReferenceOracle, &CpuReferenceOracle);
        assert!(outcome.ok());
    }

    #[test]
    fn parity_outcome_flags_divergence() {
        // Adversarial oracle that returns the wrong digest.
        struct BadOracle;
        impl DigestOracle for BadOracle {
            fn digest(&self, _alg: HashAlg, _preimage: &[u8]) -> [u8; 32] {
                [0xFFu8; 32]
            }
        }
        let v = VectorDoc {
            id: "sha256/empty".to_string(),
            algorithm: HashAlg::Sha256,
            preimage_hex: "".to_string(),
            expected_digest_hex:
                "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_string(),
        };
        let outcome = evaluate_vector(&v, &CpuReferenceOracle, &BadOracle);
        assert!(!outcome.ok());
        assert!(outcome.cpu_matches_spec);
        assert!(!outcome.gpu_matches_spec);
        assert!(!outcome.cpu_eq_gpu);
    }
}
