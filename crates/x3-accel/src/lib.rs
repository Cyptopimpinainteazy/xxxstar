//! Vendor-neutral accelerator abstraction for X3 validator batch work.
//!
//! Consensus remains CPU-deterministic. Accelerator backends are optional
//! sidecars for batchable work such as hashing, signature checks, and Merkle
//! tree construction.

use blake2::{Blake2b512, Digest as BlakeDigest};
use ed25519_dalek::{Signature as Ed25519Signature, Verifier, VerifyingKey};
use secp256k1::{ecdsa::Signature as Secp256k1Signature, Message, PublicKey, Secp256k1};
use sha2::{Digest as ShaDigest, Sha256};

mod multi_device;
mod shadow;
pub use multi_device::{MultiDevice, DEFAULT_MIN_SPLIT};
pub use shadow::{ShadowStats, ShadowVerifier};

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
    #[error("backend {backend:?} has no kernel for {algorithm}")]
    KernelUnavailable {
        backend: BackendKind,
        algorithm: &'static str,
    },
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

    /// The secp256k1 verdicts, without the `Result` the trait needs for other backends:
    /// every malformed job is a `false`, so the CPU path has no error to report.
    pub fn secp256k1_verdicts(batch: &[Secp256k1VerifyJob]) -> Vec<bool> {
        batch
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
            .collect()
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
        Ok(Self::secp256k1_verdicts(batch))
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
                output.copy_from_slice(hash.as_bytes());
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
            // An empty leaf set has no root. Answering `[0u8; 32]` would hand a verifier a value
            // no hash produced, and a caller that compared it against a real root it never
            // computed would be able to accept an empty tree as a match (`AGENTS.md` §5: an
            // unknown must not become a success). The word "empty" is the answer, not a digest.
            return Err(AccelError::InvalidInput(
                "a merkle root needs at least one leaf; an empty tree has no root",
            ));
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

/// wgpu adapter-backed accelerator.
///
/// The backend is selected only when the `wgpu` feature is enabled and a wgpu
/// adapter/device can be initialized. Algorithms without kernels fail closed so
/// callers can count the accelerator fallback and keep CPU as consensus truth.
#[cfg(feature = "wgpu")]
pub struct WgpuBackend {
    inner: x3_accel_wgpu::WgpuBackend,
}

#[cfg(feature = "wgpu")]
impl WgpuBackend {
    pub fn try_new() -> Result<Self, AccelError> {
        x3_accel_wgpu::WgpuBackend::initialize()
            .map(|inner| Self { inner })
            .map_err(|_| AccelError::BackendUnavailable(BackendKind::Wgpu))
    }

    /// Use one specific hardware adapter (index into
    /// `x3_accel_wgpu::WgpuBackend::hardware_adapters`), e.g. one per GPU.
    pub fn from_adapter(index: usize) -> Result<Self, AccelError> {
        x3_accel_wgpu::WgpuBackend::initialize_adapter(index)
            .map(|inner| Self { inner })
            .map_err(|_| AccelError::BackendUnavailable(BackendKind::Wgpu))
    }
}

#[cfg(feature = "wgpu")]
fn wgpu_error(algorithm: &'static str) -> impl Fn(x3_accel_wgpu::WgpuAccelError) -> AccelError {
    move |err| match err {
        x3_accel_wgpu::WgpuAccelError::InvalidInput(message) => AccelError::InvalidInput(message),
        x3_accel_wgpu::WgpuAccelError::AdapterUnavailable
        | x3_accel_wgpu::WgpuAccelError::DeviceRequestFailed(_)
        | x3_accel_wgpu::WgpuAccelError::BufferMapFailed(_) => {
            AccelError::BackendUnavailable(BackendKind::Wgpu)
        }
        x3_accel_wgpu::WgpuAccelError::KernelUnavailable(_) => AccelError::KernelUnavailable {
            backend: BackendKind::Wgpu,
            algorithm,
        },
    }
}

/// Apply libsecp256k1's input rules on the host, with the same library the
/// CPU backend uses, and hand the GPU only well-formed jobs.
///
/// `None` means `CpuBackend` would return `false` before doing any curve
/// arithmetic: a non-canonical signature (r or s >= n), a high-S signature
/// (libsecp256k1's verify rejects these), r or s = 0, or a public key that does
/// not parse (bad length/prefix, coordinate >= p, off-curve, hybrid parity).
#[cfg(feature = "wgpu")]
fn prepare_secp256k1(job: &Secp256k1VerifyJob) -> Option<x3_accel_wgpu::Secp256k1Prepared> {
    let signature = Secp256k1Signature::from_compact(&job.signature).ok()?;
    let mut low_s = signature;
    low_s.normalize_s();
    if low_s != signature {
        return None;
    }
    let r: [u8; 32] = *job.signature.first_chunk::<32>()?;
    let s: [u8; 32] = *job.signature.last_chunk::<32>()?;
    if r == [0; 32] || s == [0; 32] {
        return None;
    }
    let point = PublicKey::from_slice(&job.public_key)
        .ok()?
        .serialize_uncompressed();
    Some(x3_accel_wgpu::Secp256k1Prepared {
        r,
        s,
        z: job.message_hash,
        qx: point[1..33].try_into().ok()?,
        qy: point[33..65].try_into().ok()?,
    })
}

#[cfg(feature = "wgpu")]
impl AccelBackend for WgpuBackend {
    fn name(&self) -> &'static str {
        self.inner.name()
    }

    fn verify_secp256k1_batch(
        &self,
        batch: &[Secp256k1VerifyJob],
    ) -> Result<Vec<bool>, AccelError> {
        let mut results = vec![false; batch.len()];
        let (slots, prepared): (Vec<usize>, Vec<_>) = batch
            .iter()
            .enumerate()
            .filter_map(|(slot, job)| prepare_secp256k1(job).map(|p| (slot, p)))
            .unzip();
        let verdicts = self
            .inner
            .secp256k1_verify_prepared(&prepared)
            .map_err(wgpu_error("secp256k1"))?;
        if verdicts.len() != slots.len() {
            return Err(AccelError::BackendUnavailable(BackendKind::Wgpu));
        }
        for (slot, verdict) in slots.into_iter().zip(verdicts) {
            results[slot] = verdict;
        }
        Ok(results)
    }

    fn verify_ed25519_batch(&self, _batch: &[Ed25519VerifyJob]) -> Result<Vec<bool>, AccelError> {
        Err(AccelError::KernelUnavailable {
            backend: BackendKind::Wgpu,
            algorithm: "ed25519",
        })
    }

    fn keccak256_batch(&self, inputs: &[Vec<u8>]) -> Result<Vec<[u8; 32]>, AccelError> {
        self.inner
            .keccak256_batch(inputs)
            .map_err(wgpu_error("keccak256"))
    }

    fn sha256_batch(&self, inputs: &[Vec<u8>]) -> Result<Vec<[u8; 32]>, AccelError> {
        self.inner
            .sha256_batch(inputs)
            .map_err(wgpu_error("sha256"))
    }

    fn blake2b256_batch(&self, _inputs: &[Vec<u8>]) -> Result<Vec<[u8; 32]>, AccelError> {
        Err(AccelError::KernelUnavailable {
            backend: BackendKind::Wgpu,
            algorithm: "blake2b256",
        })
    }

    fn build_merkle_root(&self, _leaves: &[[u8; 32]]) -> Result<[u8; 32], AccelError> {
        Err(AccelError::KernelUnavailable {
            backend: BackendKind::Wgpu,
            algorithm: "merkle_root",
        })
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
        BackendKind::Wgpu => select_wgpu_backend(strict),
        kind if strict => Box::new(UnavailableBackend::new(kind)),
        _ => Box::new(CpuBackend::new()),
    }
}

fn select_wgpu_backend(strict: bool) -> Box<dyn AccelBackend> {
    #[cfg(feature = "wgpu")]
    {
        if let Ok(backend) = WgpuBackend::try_new() {
            return Box::new(backend);
        }
    }

    if strict {
        Box::new(UnavailableBackend::new(BackendKind::Wgpu))
    } else {
        Box::new(CpuBackend::new())
    }
}

/// Execute a hash batch and compare accelerator output to CPU truth.
pub fn keccak256_with_parity<B: AccelBackend + ?Sized>(
    backend: &B,
    inputs: &[Vec<u8>],
) -> Result<Vec<[u8; 32]>, AccelError> {
    compare_with_cpu(backend.keccak256_batch(inputs)?, &inputs, |cpu, inputs| {
        cpu.keccak256_batch(inputs)
    })
}

/// Execute a SHA-256 batch and compare the accelerator output to CPU truth.
pub fn sha256_with_parity<B: AccelBackend + ?Sized>(
    backend: &B,
    inputs: &[Vec<u8>],
) -> Result<Vec<[u8; 32]>, AccelError> {
    compare_with_cpu(backend.sha256_batch(inputs)?, &inputs, |cpu, inputs| {
        cpu.sha256_batch(inputs)
    })
}

/// Execute a BLAKE2b-512 (truncated to 32 bytes) batch and compare it to CPU truth.
pub fn blake2b256_with_parity<B: AccelBackend + ?Sized>(
    backend: &B,
    inputs: &[Vec<u8>],
) -> Result<Vec<[u8; 32]>, AccelError> {
    compare_with_cpu(backend.blake2b256_batch(inputs)?, &inputs, |cpu, inputs| {
        cpu.blake2b256_batch(inputs)
    })
}

/// Execute an Ed25519 verification batch and compare it to CPU truth.
pub fn ed25519_with_parity<B: AccelBackend + ?Sized>(
    backend: &B,
    batch: &[Ed25519VerifyJob],
) -> Result<Vec<bool>, AccelError> {
    compare_with_cpu(
        backend.verify_ed25519_batch(batch)?,
        &batch,
        |cpu, batch| cpu.verify_ed25519_batch(batch),
    )
}

/// Execute a secp256k1 verification batch and compare it to CPU truth.
pub fn secp256k1_with_parity<B: AccelBackend + ?Sized>(
    backend: &B,
    batch: &[Secp256k1VerifyJob],
) -> Result<Vec<bool>, AccelError> {
    compare_with_cpu(
        backend.verify_secp256k1_batch(batch)?,
        &batch,
        |cpu, batch| cpu.verify_secp256k1_batch(batch),
    )
}

/// Build a Merkle root and compare it to the CPU reference.
pub fn merkle_root_with_parity<B: AccelBackend + ?Sized>(
    backend: &B,
    leaves: &[[u8; 32]],
) -> Result<[u8; 32], AccelError> {
    compare_with_cpu(
        backend.build_merkle_root(leaves)?,
        &leaves,
        |cpu, leaves| cpu.build_merkle_root(leaves),
    )
}

/// The one place an accelerator answer is allowed to become a result.
///
/// `AGENTS.md` §18: a signature or proof must never be accepted merely because the accelerator
/// returned success, and a disagreement with the canonical verifier is a hard failure, not a
/// preference. Every `*_with_parity` helper funnels through here, so the comparison cannot be
/// forgotten in one of them: the accelerated answer is discarded the moment it differs from the
/// CPU recomputation, and `ParityMismatch` is returned instead. There is deliberately no
/// "prefer the accelerator" or "log and continue" branch.
fn compare_with_cpu<T, A, F>(accelerated: T, argument: &A, cpu_op: F) -> Result<T, AccelError>
where
    T: PartialEq,
    F: FnOnce(&CpuBackend, &A) -> Result<T, AccelError>,
{
    let reference = cpu_op(&CpuBackend::new(), argument)?;
    if accelerated != reference {
        return Err(AccelError::ParityMismatch);
    }
    Ok(accelerated)
}

/// An `AccelBackend` decorator that refuses any batch whose answer differs from the CPU.
///
/// The free `*_with_parity` helpers protect the call sites that remember to use them. This wrapper
/// protects the ones that do not: hoist a backend once with `ParityChecked::new(...)` and every
/// method it implements is compared against `CpuBackend` before the caller sees it. A backend that
/// cannot be checked cannot be used at all, which is the point — an unverifiable accelerator is a
/// consensus hazard, not a performance option.
pub struct ParityChecked<B> {
    inner: B,
}

impl<B> ParityChecked<B> {
    pub fn new(inner: B) -> Self {
        Self { inner }
    }

    /// The wrapped backend, for reporting only. Do not call it directly to bypass the check.
    pub fn inner(&self) -> &B {
        &self.inner
    }
}

impl<B: AccelBackend> AccelBackend for ParityChecked<B> {
    fn name(&self) -> &'static str {
        self.inner.name()
    }

    fn verify_secp256k1_batch(
        &self,
        batch: &[Secp256k1VerifyJob],
    ) -> Result<Vec<bool>, AccelError> {
        secp256k1_with_parity(&self.inner, batch)
    }

    fn verify_ed25519_batch(&self, batch: &[Ed25519VerifyJob]) -> Result<Vec<bool>, AccelError> {
        ed25519_with_parity(&self.inner, batch)
    }

    fn keccak256_batch(&self, inputs: &[Vec<u8>]) -> Result<Vec<[u8; 32]>, AccelError> {
        keccak256_with_parity(&self.inner, inputs)
    }

    fn sha256_batch(&self, inputs: &[Vec<u8>]) -> Result<Vec<[u8; 32]>, AccelError> {
        sha256_with_parity(&self.inner, inputs)
    }

    fn blake2b256_batch(&self, inputs: &[Vec<u8>]) -> Result<Vec<[u8; 32]>, AccelError> {
        blake2b256_with_parity(&self.inner, inputs)
    }

    fn build_merkle_root(&self, leaves: &[[u8; 32]]) -> Result<[u8; 32], AccelError> {
        merkle_root_with_parity(&self.inner, leaves)
    }
}

/// What one accelerator backend can do on *this* host, measured rather than assumed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcceleratorProbe {
    pub kind: BackendKind,
    /// True only when a backend for `kind` initialized here. A backend that is absent is reported
    /// absent: `GPU_VALIDATOR_HONEST_AUDIT.md` forbids claiming a measurement that no device made.
    pub available: bool,
    pub detail: &'static str,
}

/// Probe every backend this build knows about and report which ones really run here.
///
/// The GPU rows in the feature matrix are scored on evidence, and the honest evidence for a host
/// with no compute device is the string "no device, not run" — never a pass. `backend_available`
/// is the machine-readable form of that answer so a release gate or a report can quote it.
pub fn probe_accelerators() -> Vec<AcceleratorProbe> {
    let mut probes = vec![AcceleratorProbe {
        kind: BackendKind::Cpu,
        available: true,
        detail: "CPU reference; the consensus truth source",
    }];

    #[cfg(feature = "wgpu")]
    {
        probes.push(match WgpuBackend::try_new() {
            Ok(backend) => AcceleratorProbe {
                kind: BackendKind::Wgpu,
                available: true,
                detail: backend.name(),
            },
            Err(_) => AcceleratorProbe {
                kind: BackendKind::Wgpu,
                available: false,
                detail: "no device, not run: no wgpu adapter/device answered",
            },
        });
    }
    #[cfg(not(feature = "wgpu"))]
    {
        probes.push(AcceleratorProbe {
            kind: BackendKind::Wgpu,
            available: false,
            detail: "no device, not run: built without the wgpu feature",
        });
    }

    for kind in [
        BackendKind::OpenCl,
        BackendKind::Vulkan,
        BackendKind::CudaOptional,
    ] {
        probes.push(AcceleratorProbe {
            kind,
            available: false,
            detail: "no device, not run: this crate has no backend for it",
        });
    }

    probes
}

pub fn backend_available(kind: BackendKind) -> bool {
    probe_accelerators()
        .into_iter()
        .any(|probe| probe.kind == kind && probe.available)
}

/// Published vectors for the CPU reference implementations.
///
/// The CPU recomputation in `compare_with_cpu` is only as trustworthy as the CPU implementation it
/// compares against, so every primitive the accelerator rows name is pinned to a value published
/// by someone other than this repository:
///
/// * **Keccak-256** — the Ethereum digests of the empty string and of `abc`. Keccak-256 is *not*
///   SHA3-256: they differ by one domain-separation byte in the padding, so `SHA3_256_ABC` is
///   included as the answer a kernel with the wrong padding produces.
/// * **SHA-256** — FIPS 180-4 examples: the empty string, `abc`, and the 448-bit two-block string.
/// * **BLAKE2b-512**, truncated to its first 32 bytes — RFC 7693 Appendix A.
/// * **Ed25519** — RFC 8032 §7.1 TEST 1 (empty message).
/// * **secp256k1** — the compressed generator point, i.e. the public key of secret key 1.
/// * **Merkle root** — the rule is written down in `CpuBackend::build_merkle_root` (leaf =
///   SHA-256 of the data, internal node = SHA-256 of `left || right`, an odd node hashes against
///   itself), and the pinned roots are that rule over `sha256("a")`, `sha256("b")`, `sha256("c")`,
///   reproducible with any SHA-256 tool.
#[cfg(test)]
pub mod vectors {
    /// Decode exactly 32 bytes of hex. A vector that will not decode is a broken test, not a
    /// runtime condition, so this panics rather than returning an `Option` a caller might ignore.
    pub fn to_32(hex: &str) -> [u8; 32] {
        assert_eq!(
            hex.len(),
            64,
            "a 32-byte vector is 64 hex characters; got {}",
            hex.len()
        );
        let mut out = [0u8; 32];
        for (index, byte) in out.iter_mut().enumerate() {
            *byte = u8::from_str_radix(&hex[index * 2..index * 2 + 2], 16)
                .expect("a vector must be hexadecimal");
        }
        out
    }

    /// Decode exactly 64 bytes of hex, for the Ed25519 signature vector.
    pub fn to_64(hex: &str) -> [u8; 64] {
        assert_eq!(
            hex.len(),
            128,
            "a 64-byte vector is 128 hex characters; got {}",
            hex.len()
        );
        let mut out = [0u8; 64];
        for (index, byte) in out.iter_mut().enumerate() {
            *byte = u8::from_str_radix(&hex[index * 2..index * 2 + 2], 16)
                .expect("a vector must be hexadecimal");
        }
        out
    }

    /// Decode exactly 33 bytes of hex, for a compressed secp256k1 public key.
    pub fn to_33(hex: &str) -> Vec<u8> {
        assert_eq!(
            hex.len(),
            66,
            "a 33-byte vector is 66 hex characters; got {}",
            hex.len()
        );
        let mut out = Vec::with_capacity(33);
        for index in 0..33 {
            out.push(
                u8::from_str_radix(&hex[index * 2..index * 2 + 2], 16)
                    .expect("a vector must be hexadecimal"),
            );
        }
        out
    }

    pub const KECCAK256_EMPTY: &str =
        "c5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470";
    pub const KECCAK256_ABC: &str =
        "4e03657aea45a94fc7d47ba826c8d667c0d1e6e33a64a036ec44f58fa12d6c45";
    /// `0x3a985da7…` is SHA3-256("abc"), not Keccak-256("abc").
    pub const SHA3_256_ABC: &str =
        "3a985da74fe225b2045c172d6bd390bd855f086e3e9d525b46bfe24511431532";

    pub const SHA256_EMPTY: &str =
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
    pub const SHA256_ABC: &str = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
    /// FIPS 180-4's 448-bit message: `abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq`.
    pub const SHA256_NIST_448BIT: &str =
        "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1";
    pub const SHA256_NIST_448BIT_MESSAGE: &[u8] =
        b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq";

    /// RFC 7693 Appendix A, first 32 of the 64 digest bytes.
    pub const BLAKE2B512_EMPTY_TRUNCATED: &str =
        "786a02f742015903c6c6fd852552d272912f4740e15847618a86e217f71f5419";
    pub const BLAKE2B512_ABC_TRUNCATED: &str =
        "ba80a53f981c4d0d6a2797b69f12f6e94c212f14685ac4b74b12bb6fdbffa2d1";

    /// RFC 8032 §7.1 TEST 1: secret key, public key and signature of the empty message.
    pub const ED25519_RFC8032_TEST1_PUBLIC_KEY: &str =
        "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a";
    pub const ED25519_RFC8032_TEST1_SIGNATURE: &str = "e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e065224901555fb8821590a33bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100b";

    /// Compressed public key of secp256k1 secret key 1: the curve generator point G.
    pub const SECP256K1_GENERATOR_COMPRESSED: &str =
        "0279be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798";

    /// Root over `[sha256("a"), sha256("b"), sha256("c")]`; the odd node hashes against itself.
    pub const MERKLE_ROOT_ABC: &str =
        "d31a37ef6ac14a2db1470c4316beb5592e6afd4465022339adafda76a18ffabe";
    /// Root over `[sha256("a"), sha256("b")]`.
    pub const MERKLE_ROOT_AB: &str =
        "e5a01fee14e0ed5c48714f22180f25ad8365b53f9779f79dc4a3d7e93963f94a";
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
    fn merkle_root_matches_the_published_rule_and_refuses_an_empty_tree() {
        let backend = CpuBackend::new();
        let leaves = backend
            .sha256_batch(&[b"a".to_vec(), b"b".to_vec(), b"c".to_vec()])
            .unwrap();

        // The three-leaf case is the odd level, so it pins both the pairing order and the
        // duplicate-last rule against a root anyone can recompute with a SHA-256 tool.
        assert_eq!(
            backend.build_merkle_root(&leaves).unwrap(),
            vectors::to_32(vectors::MERKLE_ROOT_ABC)
        );
        assert_eq!(
            backend.build_merkle_root(&leaves[..2]).unwrap(),
            vectors::to_32(vectors::MERKLE_ROOT_AB)
        );

        // An empty leaf set used to answer `[0u8; 32]`. That is a digest no hash produced, and a
        // verifier that compared it to a root it never computed would read it as a match.
        assert_eq!(
            backend.build_merkle_root(&[]).unwrap_err(),
            AccelError::InvalidInput(
                "a merkle root needs at least one leaf; an empty tree has no root"
            )
        );
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
    fn strict_wgpu_selection_fails_closed_when_unavailable_or_executes_sha256() {
        let backend = select_backend_with("wgpu", true);
        let result = backend.sha256_batch(&[b"x".to_vec()]);

        match result {
            Ok(outputs) => assert_eq!(outputs.len(), 1),
            Err(AccelError::BackendUnavailable(BackendKind::Wgpu))
            | Err(AccelError::KernelUnavailable {
                backend: BackendKind::Wgpu,
                algorithm: "sha256",
            }) => {}
            other => panic!("unexpected wgpu selection result: {other:?}"),
        }
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

    #[test]
    fn cpu_reference_matches_published_keccak256_vectors() {
        let backend = CpuBackend::new();
        let output = backend
            .keccak256_batch(&[Vec::new(), b"abc".to_vec()])
            .unwrap();

        assert_eq!(output[0], vectors::to_32(vectors::KECCAK256_EMPTY));
        assert_eq!(output[1], vectors::to_32(vectors::KECCAK256_ABC));
        // Keccak-256 and SHA3-256 differ by one domain byte in the padding. A kernel that
        // implements the SHA3 padding answers the second vector and must not be accepted.
        assert_ne!(output[1], vectors::to_32(vectors::SHA3_256_ABC));
    }

    #[test]
    fn cpu_reference_matches_published_sha256_vectors() {
        let backend = CpuBackend::new();
        let output = backend
            .sha256_batch(&[
                Vec::new(),
                b"abc".to_vec(),
                vectors::SHA256_NIST_448BIT_MESSAGE.to_vec(),
            ])
            .unwrap();

        assert_eq!(output[0], vectors::to_32(vectors::SHA256_EMPTY));
        assert_eq!(output[1], vectors::to_32(vectors::SHA256_ABC));
        assert_eq!(output[2], vectors::to_32(vectors::SHA256_NIST_448BIT));
    }

    #[test]
    fn cpu_reference_matches_published_blake2b_vectors() {
        let backend = CpuBackend::new();
        let output = backend
            .blake2b256_batch(&[Vec::new(), b"abc".to_vec()])
            .unwrap();

        assert_eq!(
            output[0],
            vectors::to_32(vectors::BLAKE2B512_EMPTY_TRUNCATED)
        );
        assert_eq!(output[1], vectors::to_32(vectors::BLAKE2B512_ABC_TRUNCATED));
    }

    #[test]
    fn cpu_reference_matches_the_rfc8032_ed25519_test_vector() {
        let backend = CpuBackend::new();
        let public_key = vectors::to_32(vectors::ED25519_RFC8032_TEST1_PUBLIC_KEY);
        let signature = vectors::to_64(vectors::ED25519_RFC8032_TEST1_SIGNATURE);

        let valid = Ed25519VerifyJob {
            message: Vec::new(),
            signature,
            public_key,
        };
        assert_eq!(
            backend
                .verify_ed25519_batch(std::slice::from_ref(&valid))
                .unwrap(),
            vec![true],
            "RFC 8032 TEST 1 must verify"
        );

        // The ugly path: a verifier that answered `true` unconditionally passes the line above.
        // Each field is changed on its own so the three checks cannot cancel out.
        let mut changed_message = valid.clone();
        changed_message.message = b"!".to_vec();
        let mut changed_signature = valid.clone();
        changed_signature.signature[0] ^= 0x01;
        let mut changed_key = valid.clone();
        changed_key.public_key[31] ^= 0x80;

        assert_eq!(
            backend
                .verify_ed25519_batch(&[changed_message, changed_signature, changed_key])
                .unwrap(),
            vec![false, false, false]
        );
    }

    #[test]
    fn cpu_reference_anchors_secp256k1_to_the_published_generator() {
        use secp256k1::SecretKey;

        let backend = CpuBackend::new();
        let secp = Secp256k1::new();
        // Secret key 1 has the curve generator as its public key, so this pins the group
        // parameters the whole crate verifies against — not just that *some* curve works.
        let secret = SecretKey::from_slice(&{
            let mut bytes = [0u8; 32];
            bytes[31] = 1;
            bytes
        })
        .unwrap();
        let public_key = PublicKey::from_secret_key(&secp, &secret);

        assert_eq!(
            public_key.serialize().to_vec(),
            vectors::to_33(vectors::SECP256K1_GENERATOR_COMPRESSED)
        );

        let message_hash = [9u8; 32];
        let message = Message::from_digest_slice(&message_hash).unwrap();
        let signature = secp.sign_ecdsa(&message, &secret).serialize_compact();
        let valid = Secp256k1VerifyJob {
            message_hash,
            signature,
            public_key: public_key.serialize().to_vec(),
        };
        assert_eq!(
            backend
                .verify_secp256k1_batch(std::slice::from_ref(&valid))
                .unwrap(),
            vec![true]
        );

        let mut short_hash = valid.clone();
        short_hash.message_hash[0] ^= 0x01;
        let mut bad_signature = valid.clone();
        bad_signature.signature[63] ^= 0x01;
        let mut bad_key = valid;
        bad_key.public_key[0] = 0x03;

        assert_eq!(
            backend
                .verify_secp256k1_batch(&[short_hash, bad_signature, bad_key])
                .unwrap(),
            vec![false, false, false]
        );
    }

    /// A backend that answers every batch with a plausible-looking constant.
    ///
    /// This is the shape of a real accelerator bug: not a crash, and not an empty result, but a
    /// wrong answer of exactly the right shape. Nothing about it is detectable at the call site,
    /// which is why the comparison has to live inside the wrapper.
    struct DivergentBackend;

    impl AccelBackend for DivergentBackend {
        fn name(&self) -> &'static str {
            "divergent"
        }

        fn verify_secp256k1_batch(
            &self,
            batch: &[Secp256k1VerifyJob],
        ) -> Result<Vec<bool>, AccelError> {
            Ok(vec![false; batch.len()])
        }

        fn verify_ed25519_batch(
            &self,
            batch: &[Ed25519VerifyJob],
        ) -> Result<Vec<bool>, AccelError> {
            Ok(vec![false; batch.len()])
        }

        fn keccak256_batch(&self, inputs: &[Vec<u8>]) -> Result<Vec<[u8; 32]>, AccelError> {
            Ok(vec![[0u8; 32]; inputs.len()])
        }

        fn sha256_batch(&self, inputs: &[Vec<u8>]) -> Result<Vec<[u8; 32]>, AccelError> {
            Ok(vec![[1u8; 32]; inputs.len()])
        }

        fn blake2b256_batch(&self, inputs: &[Vec<u8>]) -> Result<Vec<[u8; 32]>, AccelError> {
            Ok(vec![[2u8; 32]; inputs.len()])
        }

        fn build_merkle_root(&self, _leaves: &[[u8; 32]]) -> Result<[u8; 32], AccelError> {
            Ok([3u8; 32])
        }
    }

    fn rfc8032_job() -> Ed25519VerifyJob {
        Ed25519VerifyJob {
            message: Vec::new(),
            signature: vectors::to_64(vectors::ED25519_RFC8032_TEST1_SIGNATURE),
            public_key: vectors::to_32(vectors::ED25519_RFC8032_TEST1_PUBLIC_KEY),
        }
    }

    fn signed_secp_job() -> Secp256k1VerifyJob {
        use secp256k1::SecretKey;
        let secp = Secp256k1::new();
        let secret = SecretKey::from_slice(&[7u8; 32]).unwrap();
        let public_key = PublicKey::from_secret_key(&secp, &secret);
        let message_hash = [9u8; 32];
        let message = Message::from_digest_slice(&message_hash).unwrap();
        Secp256k1VerifyJob {
            message_hash,
            signature: secp.sign_ecdsa(&message, &secret).serialize_compact(),
            public_key: public_key.serialize().to_vec(),
        }
    }

    /// Every one of the five accelerator rows is covered here, because the failure mode is per
    /// primitive: a wrapper that checked keccak256 and forgot blake2b would pass a partial test.
    #[test]
    fn an_accelerator_that_disagrees_with_the_cpu_is_refused_on_every_primitive() {
        let divergent = DivergentBackend;

        assert_eq!(
            keccak256_with_parity(&divergent, &[b"abc".to_vec()]).unwrap_err(),
            AccelError::ParityMismatch
        );
        assert_eq!(
            sha256_with_parity(&divergent, &[b"abc".to_vec()]).unwrap_err(),
            AccelError::ParityMismatch
        );
        assert_eq!(
            blake2b256_with_parity(&divergent, &[b"abc".to_vec()]).unwrap_err(),
            AccelError::ParityMismatch
        );
        assert_eq!(
            merkle_root_with_parity(&divergent, &[[7u8; 32]]).unwrap_err(),
            AccelError::ParityMismatch
        );
        assert_eq!(
            ed25519_with_parity(&divergent, &[rfc8032_job()]).unwrap_err(),
            AccelError::ParityMismatch
        );
        assert_eq!(
            secp256k1_with_parity(&divergent, &[signed_secp_job()]).unwrap_err(),
            AccelError::ParityMismatch
        );
    }

    /// The wrapper is the enforcement point, so a caller that never heard of `*_with_parity` still
    /// cannot receive a divergent answer.
    #[test]
    fn the_parity_wrapper_refuses_a_divergent_backend_through_the_trait() {
        let backend: ParityChecked<DivergentBackend> = ParityChecked::new(DivergentBackend);

        assert_eq!(
            backend.keccak256_batch(&[b"abc".to_vec()]).unwrap_err(),
            AccelError::ParityMismatch
        );
        assert_eq!(
            backend.verify_ed25519_batch(&[rfc8032_job()]).unwrap_err(),
            AccelError::ParityMismatch
        );
        assert_eq!(
            backend.build_merkle_root(&[[7u8; 32]]).unwrap_err(),
            AccelError::ParityMismatch
        );
    }

    /// A correct backend still passes through, so the refusals above are not vacuous.
    #[test]
    fn the_parity_wrapper_passes_through_a_backend_that_agrees() {
        let backend = ParityChecked::new(CpuBackend::new());

        assert_eq!(
            backend.keccak256_batch(&[b"abc".to_vec()]).unwrap(),
            vec![vectors::to_32(vectors::KECCAK256_ABC)]
        );
        assert_eq!(
            backend.verify_ed25519_batch(&[rfc8032_job()]).unwrap(),
            vec![true]
        );
        assert_eq!(
            backend.build_merkle_root(&[[7u8; 32]]).unwrap(),
            CpuBackend::new().build_merkle_root(&[[7u8; 32]]).unwrap()
        );
    }

    /// "No device" must never read as a pass: the missing-device backend has to refuse every
    /// primitive, not answer an empty batch that a caller could mistake for success.
    #[test]
    fn an_absent_device_refuses_every_primitive_instead_of_answering() {
        let backend = UnavailableBackend::new(BackendKind::Wgpu);
        let unavailable = AccelError::BackendUnavailable(BackendKind::Wgpu);

        assert_eq!(
            backend.keccak256_batch(&[b"abc".to_vec()]).unwrap_err(),
            unavailable
        );
        assert_eq!(
            backend.sha256_batch(&[b"abc".to_vec()]).unwrap_err(),
            unavailable
        );
        assert_eq!(
            backend.blake2b256_batch(&[b"abc".to_vec()]).unwrap_err(),
            unavailable
        );
        assert_eq!(
            backend.build_merkle_root(&[[7u8; 32]]).unwrap_err(),
            unavailable
        );
        assert_eq!(
            backend.verify_ed25519_batch(&[rfc8032_job()]).unwrap_err(),
            unavailable
        );
        assert_eq!(
            backend
                .verify_secp256k1_batch(&[signed_secp_job()])
                .unwrap_err(),
            unavailable
        );
    }

    #[test]
    fn the_probe_reports_what_this_host_can_run() {
        let probes = probe_accelerators();

        let cpu = probes
            .iter()
            .find(|probe| probe.kind == BackendKind::Cpu)
            .expect("the CPU reference is always probed");
        assert!(cpu.available, "the CPU reference is the consensus truth");
        assert!(backend_available(BackendKind::Cpu));

        for probe in probes.iter().filter(|probe| !probe.available) {
            // An absent backend must say so in words, not merely carry `false`, because the
            // honest report for the GPU rows is the phrase "no device, not run".
            assert!(
                probe.detail.starts_with("no device, not run")
                    || probe.detail.starts_with("built without"),
                "an absent backend must say why it is absent: {:?}",
                probe
            );
            assert!(!backend_available(probe.kind));
        }
    }
}
