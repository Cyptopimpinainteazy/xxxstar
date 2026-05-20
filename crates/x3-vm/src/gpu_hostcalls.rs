//! GPU Compute Hostcalls for X3 VM
//!
//! Provides hostcall handlers that dispatch real CUDA operations from X3 bytecode.
//! These bridge the X3 VM interpreter to the compiled CUDA shared libraries:
//!
//!   - `libsha256_batch.so`    → SHA-256 batch + PoH chain
//!   - `libed25519_batch.so`   → Ed25519 batch verification
//!   - `libstream_pipeline.so` → Stream-pipelined SHA-256 with pinned memory
//!
//! # Architecture
//!
//! ```text
//! X3 bytecode
//!   → Opcode::GpuSha256Batch (0xD0)
//!     → VM dispatch loop
//!       → hostcall_registry.invoke(0xD0, args)
//!         → GpuHostcalls::sha256_batch_handler
//!           → libsha256_batch.so::sha256_batch_host() [real CUDA]
//!             → GPU kernel launch (15,360 CUDA cores)
//!               → 68.9M hashes/sec
//! ```
//!
//! # Hostcall IDs (0xD0 - 0xDF)
//!
//! | ID   | Name                 | Args                        | Return         |
//! |------|----------------------|-----------------------------|----------------|
//! | 0xD0 | gpu_sha256_batch     | (inputs: Bytes, count: I64) | Bytes          |
//! | 0xD1 | gpu_ed25519_verify   | (sigs: Bytes, count: I64)   | Bytes (bitmap) |
//! | 0xD2 | gpu_poh_chain        | (seeds: Bytes, chains: I64, len: I64) | Bytes |
//! | 0xD3 | gpu_sha256_streamed  | (inputs: Bytes, count: I64, streams: I64) | Bytes |
//! | 0xD4 | gpu_device_count     | ()                          | I64            |
//! | 0xD5 | gpu_benchmark        | (count: I64, streams: I64)  | Bytes (JSON)   |

use std::path::PathBuf;
use std::sync::Arc;

use crate::error::{VMError, VMErrorKind, VMResult};
use crate::hostcall::HostcallRegistry;
use crate::vm::Value;

/// GPU hostcall ID range
pub mod gpu_hostcall_ids {
    pub const GPU_SHA256_BATCH: u8 = 0xD0;
    pub const GPU_ED25519_VERIFY: u8 = 0xD1;
    pub const GPU_POH_CHAIN: u8 = 0xD2;
    pub const GPU_SHA256_STREAMED: u8 = 0xD3;
    pub const GPU_DEVICE_COUNT: u8 = 0xD4;
    pub const GPU_BENCHMARK: u8 = 0xD5;
    pub const GPU_KECCAK256_BATCH: u8 = 0xD6;
    pub const GPU_SECP256K1_VERIFY: u8 = 0xD7;
    pub const GPU_ATOMIC_VERIFY: u8 = 0xD8;
    pub const GPU_ATOMIC_COMMIT: u8 = 0xD9;
}

// FFI function signatures matching our CUDA extern "C" exports
type Sha256BatchFn = unsafe extern "C" fn(*const u8, i32, *mut u8) -> i32;
type Sha256PohChainFn = unsafe extern "C" fn(*const u8, i32, i32, *mut u8) -> i32;
type Sha256BatchMultiGpuFn = unsafe extern "C" fn(*const u8, i32, *mut u8) -> i32;
type Ed25519VerifyBatchFn = unsafe extern "C" fn(*const u8, i32, *mut u8) -> i32;
type Ed25519VerifyMultiGpuFn = unsafe extern "C" fn(*const u8, i32, *mut u8) -> i32;
type Sha256BatchStreamedFn = unsafe extern "C" fn(*const u8, i32, *mut u8, i32) -> i32;
type PipelineBenchmarkFn =
    unsafe extern "C" fn(i32, i32, *mut f32, *mut f32, *mut f32, *mut f32, *mut f32) -> i32;
type PipelinePrintInfoFn = unsafe extern "C" fn();
type PipelineCleanupFn = unsafe extern "C" fn();
type Keccak256BatchFn = unsafe extern "C" fn(*const u8, i32, *mut u8) -> i32;
type Secp256k1VerifyFn = unsafe extern "C" fn(*const u8, *const u8, *const u8, i32, *mut u8) -> i32;
type Secp256k1VerifyMultiGpuFn =
    unsafe extern "C" fn(*const u8, *const u8, *const u8, i32, *mut u8) -> i32;
type AtomicVerifyFn = unsafe extern "C" fn(*const u8, *const u8, i32, *mut u8) -> i32;
type AtomicCommitFn = unsafe extern "C" fn(*const u8, *const u8, i32) -> i32;

/// Handle to a loaded CUDA shared library with resolved symbols.
struct CudaLib {
    // We keep the libloading::Library alive so symbols remain valid.
    // The raw function pointers are transmuted from libloading::Symbol
    // and are valid for the lifetime of the Library.
    _lib: libloading::Library,
}

/// Loaded SHA-256 library functions
#[allow(dead_code)]
struct Sha256Lib {
    batch: Sha256BatchFn,
    poh_chain: Sha256PohChainFn,
    multi_gpu: Sha256BatchMultiGpuFn,
    _cuda_lib: CudaLib,
}

/// Loaded Ed25519 library functions
#[allow(dead_code)]
struct Ed25519Lib {
    verify_batch: Ed25519VerifyBatchFn,
    verify_multi_gpu: Ed25519VerifyMultiGpuFn,
    _cuda_lib: CudaLib,
}

/// Loaded stream pipeline library functions
#[allow(dead_code)]
struct StreamPipelineLib {
    batch_streamed: Sha256BatchStreamedFn,
    benchmark: PipelineBenchmarkFn,
    print_info: PipelinePrintInfoFn,
    cleanup: PipelineCleanupFn,
    _cuda_lib: CudaLib,
}

/// Loaded Keccak-256 library functions
struct Keccak256Lib {
    batch: Keccak256BatchFn,
    _cuda_lib: CudaLib,
}

/// Loaded secp256k1 library functions
#[allow(dead_code)]
struct Secp256k1Lib {
    verify: Secp256k1VerifyFn,
    verify_multi_gpu: Secp256k1VerifyMultiGpuFn,
    _cuda_lib: CudaLib,
}

/// Loaded Atomic Swap library functions
struct AtomicLib {
    verify: AtomicVerifyFn,
    commit: AtomicCommitFn,
    _cuda_lib: CudaLib,
}

// Safety: The CUDA libraries are thread-safe for our use pattern
// (each call gets its own device memory allocation)
unsafe impl Send for Sha256Lib {}
unsafe impl Sync for Sha256Lib {}
unsafe impl Send for Ed25519Lib {}
unsafe impl Sync for Ed25519Lib {}
unsafe impl Send for StreamPipelineLib {}
unsafe impl Sync for StreamPipelineLib {}
unsafe impl Send for Keccak256Lib {}
unsafe impl Sync for Keccak256Lib {}
unsafe impl Send for Secp256k1Lib {}
unsafe impl Sync for Secp256k1Lib {}
unsafe impl Send for AtomicLib {}
unsafe impl Sync for AtomicLib {}

/// Search paths for CUDA shared libraries
fn lib_search_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    // 1. Environment variable override
    if let Ok(dir) = std::env::var("X3_CUDA_LIB_DIR") {
        paths.push(PathBuf::from(dir));
    }

    // 2. Relative to canonical validator crates in the workspace
    let workspace_paths = [
        "crates/cross-chain-gpu-validator/kernels/build",
        "../cross-chain-gpu-validator/kernels/build",
        "cross-chain-gpu-validator/kernels/build",
        "crates/x3-gpu-validator-swarm/kernels/build",
        "../x3-gpu-validator-swarm/kernels/build",
        "x3-gpu-validator-swarm/kernels/build",
        "cu_kernels/build",
    ];
    if let Ok(cwd) = std::env::current_dir() {
        for p in &workspace_paths {
            paths.push(cwd.join(p));
        }
    }

    // 3. Standard library paths
    paths.push(PathBuf::from("/usr/local/lib/x3-chain"));
    paths.push(PathBuf::from("/usr/lib/x3-chain"));

    paths
}

fn find_lib(name: &str) -> Option<PathBuf> {
    for dir in lib_search_paths() {
        let path = dir.join(name);
        if path.exists() {
            return Some(path);
        }
    }
    None
}

/// Load the SHA-256 CUDA library
fn load_sha256_lib() -> Result<Sha256Lib, String> {
    let path = find_lib("libsha256_batch.so")
        .ok_or_else(|| "libsha256_batch.so not found in search paths".to_string())?;

    // Safety: `path` is an operator-controlled library path and the requested
    // symbols are copied into owned function pointers while the `Library`
    // handle is retained in `CudaLib`, so the symbol lifetimes remain valid.
    unsafe {
        let lib = libloading::Library::new(&path)
            .map_err(|e| format!("Failed to load {}: {}", path.display(), e))?;

        let batch: libloading::Symbol<Sha256BatchFn> = lib
            .get(b"sha256_batch_host")
            .map_err(|e| format!("Symbol sha256_batch_host: {}", e))?;
        let poh_chain: libloading::Symbol<Sha256PohChainFn> = lib
            .get(b"sha256_poh_chain_host")
            .map_err(|e| format!("Symbol sha256_poh_chain_host: {}", e))?;
        let multi_gpu: libloading::Symbol<Sha256BatchMultiGpuFn> = lib
            .get(b"sha256_batch_multi_gpu")
            .map_err(|e| format!("Symbol sha256_batch_multi_gpu: {}", e))?;

        // Transmute to owned function pointers (valid while lib is alive)
        let batch_fn: Sha256BatchFn = *batch;
        let poh_fn: Sha256PohChainFn = *poh_chain;
        let multi_fn: Sha256BatchMultiGpuFn = *multi_gpu;

        Ok(Sha256Lib {
            batch: batch_fn,
            poh_chain: poh_fn,
            multi_gpu: multi_fn,
            _cuda_lib: CudaLib { _lib: lib },
        })
    }
}

/// Load the Ed25519 CUDA library
fn load_ed25519_lib() -> Result<Ed25519Lib, String> {
    let path = find_lib("libed25519_batch.so")
        .ok_or_else(|| "libed25519_batch.so not found in search paths".to_string())?;

    // Safety: same invariant as `load_sha256_lib`; the library handle is kept
    // alive for the lifetime of the copied symbols.
    unsafe {
        let lib = libloading::Library::new(&path)
            .map_err(|e| format!("Failed to load {}: {}", path.display(), e))?;

        let verify_batch: libloading::Symbol<Ed25519VerifyBatchFn> = lib
            .get(b"ed25519_verify_batch_host")
            .map_err(|e| format!("Symbol ed25519_verify_batch_host: {}", e))?;
        let verify_multi_gpu: libloading::Symbol<Ed25519VerifyMultiGpuFn> = lib
            .get(b"ed25519_verify_batch_multi_gpu")
            .map_err(|e| format!("Symbol ed25519_verify_batch_multi_gpu: {}", e))?;

        let verify_fn: Ed25519VerifyBatchFn = *verify_batch;
        let multi_fn: Ed25519VerifyMultiGpuFn = *verify_multi_gpu;

        Ok(Ed25519Lib {
            verify_batch: verify_fn,
            verify_multi_gpu: multi_fn,
            _cuda_lib: CudaLib { _lib: lib },
        })
    }
}

/// Load the stream pipeline CUDA library
fn load_pipeline_lib() -> Result<StreamPipelineLib, String> {
    let path = find_lib("libstream_pipeline.so")
        .ok_or_else(|| "libstream_pipeline.so not found in search paths".to_string())?;

    unsafe {
        let lib = libloading::Library::new(&path)
            .map_err(|e| format!("Failed to load {}: {}", path.display(), e))?;

        let batch_streamed: libloading::Symbol<Sha256BatchStreamedFn> = lib
            .get(b"sha256_batch_streamed")
            .map_err(|e| format!("Symbol sha256_batch_streamed: {}", e))?;
        let benchmark: libloading::Symbol<PipelineBenchmarkFn> = lib
            .get(b"sha256_pipeline_benchmark")
            .map_err(|e| format!("Symbol sha256_pipeline_benchmark: {}", e))?;
        let print_info: libloading::Symbol<PipelinePrintInfoFn> =
            lib.get(b"pipeline_print_info")
                .map_err(|e| format!("Symbol pipeline_print_info: {}", e))?;
        let cleanup: libloading::Symbol<PipelineCleanupFn> = lib
            .get(b"pipeline_cleanup")
            .map_err(|e| format!("Symbol pipeline_cleanup: {}", e))?;

        let streamed_fn: Sha256BatchStreamedFn = *batch_streamed;
        let bench_fn: PipelineBenchmarkFn = *benchmark;
        let info_fn: PipelinePrintInfoFn = *print_info;
        let cleanup_fn: PipelineCleanupFn = *cleanup;

        Ok(StreamPipelineLib {
            batch_streamed: streamed_fn,
            benchmark: bench_fn,
            print_info: info_fn,
            cleanup: cleanup_fn,
            _cuda_lib: CudaLib { _lib: lib },
        })
    }
}

/// GPU Hostcall manager — holds loaded CUDA libraries and registers hostcalls
pub struct GpuHostcalls {
    sha256: Option<Arc<Sha256Lib>>,
    ed25519: Option<Arc<Ed25519Lib>>,
    pipeline: Option<Arc<StreamPipelineLib>>,
    keccak256: Option<Arc<Keccak256Lib>>,
    secp256k1: Option<Arc<Secp256k1Lib>>,
    atomic: Option<Arc<AtomicLib>>,
}

/// Load the Keccak-256 CUDA library
fn load_keccak256_lib() -> Result<Keccak256Lib, String> {
    let path = find_lib("libkeccak256_batch.so")
        .ok_or_else(|| "libkeccak256_batch.so not found in search paths".to_string())?;
    unsafe {
        let lib = libloading::Library::new(&path)
            .map_err(|e| format!("Failed to load {}: {}", path.display(), e))?;
        let batch: libloading::Symbol<Keccak256BatchFn> = lib
            .get(b"keccak256_batch_host")
            .map_err(|e| format!("Symbol keccak256_batch_host: {}", e))?;
        let batch_fn: Keccak256BatchFn = *batch;
        Ok(Keccak256Lib {
            batch: batch_fn,
            _cuda_lib: CudaLib { _lib: lib },
        })
    }
}

/// Load the secp256k1 CUDA library (optimized Jacobian + Shamir kernel)
fn load_secp256k1_lib() -> Result<Secp256k1Lib, String> {
    let path = find_lib("libsecp256k1_batch.so")
        .ok_or_else(|| "libsecp256k1_batch.so not found in search paths".to_string())?;
    unsafe {
        let lib = libloading::Library::new(&path)
            .map_err(|e| format!("Failed to load {}: {}", path.display(), e))?;
        let verify: libloading::Symbol<Secp256k1VerifyFn> = lib
            .get(b"secp256k1_ecdsa_verify_host")
            .map_err(|e| format!("Symbol secp256k1_ecdsa_verify_host: {}", e))?;
        let multi: libloading::Symbol<Secp256k1VerifyMultiGpuFn> = lib
            .get(b"secp256k1_ecdsa_verify_multi_gpu")
            .map_err(|e| format!("Symbol secp256k1_ecdsa_verify_multi_gpu: {}", e))?;
        let verify_fn: Secp256k1VerifyFn = *verify;
        let multi_fn: Secp256k1VerifyMultiGpuFn = *multi;
        Ok(Secp256k1Lib {
            verify: verify_fn,
            verify_multi_gpu: multi_fn,
            _cuda_lib: CudaLib { _lib: lib },
        })
    }
}

/// Load the Atomic Swap CUDA library
fn load_atomic_lib() -> Result<AtomicLib, String> {
    let path = find_lib("libatomic_swap.so")
        .ok_or_else(|| "libatomic_swap.so not found in search paths".to_string())?;
    unsafe {
        let lib = libloading::Library::new(&path)
            .map_err(|e| format!("Failed to load {}: {}", path.display(), e))?;
        let verify: libloading::Symbol<AtomicVerifyFn> = lib
            .get(b"atomic_verify_host")
            .map_err(|e| format!("Symbol atomic_verify_host: {}", e))?;
        let commit: libloading::Symbol<AtomicCommitFn> = lib
            .get(b"atomic_commit_host")
            .map_err(|e| format!("Symbol atomic_commit_host: {}", e))?;
        Ok(AtomicLib {
            verify: *verify,
            commit: *commit,
            _cuda_lib: CudaLib { _lib: lib },
        })
    }
}

impl GpuHostcalls {
    /// Create a new GPU hostcall manager, loading all available CUDA libraries.
    /// Libraries that fail to load are silently skipped (hostcalls will return errors).
    pub fn new() -> Self {
        let sha256 = match load_sha256_lib() {
            Ok(lib) => {
                log::info!("[X3-GPU] Loaded libsha256_batch.so");
                Some(Arc::new(lib))
            }
            Err(e) => {
                log::warn!("[X3-GPU] SHA-256 GPU unavailable: {}", e);
                None
            }
        };

        let ed25519 = match load_ed25519_lib() {
            Ok(lib) => {
                log::info!("[X3-GPU] Loaded libed25519_batch.so");
                Some(Arc::new(lib))
            }
            Err(e) => {
                log::warn!("[X3-GPU] Ed25519 GPU unavailable: {}", e);
                None
            }
        };

        let pipeline = match load_pipeline_lib() {
            Ok(lib) => {
                log::info!("[X3-GPU] Loaded libstream_pipeline.so");
                Some(Arc::new(lib))
            }
            Err(e) => {
                log::warn!("[X3-GPU] Stream pipeline GPU unavailable: {}", e);
                None
            }
        };

        Self {
            sha256,
            ed25519,
            pipeline,
            keccak256: match load_keccak256_lib() {
                Ok(lib) => {
                    log::info!("[X3-GPU] Loaded libkeccak256_batch.so");
                    Some(Arc::new(lib))
                }
                Err(e) => {
                    log::warn!("[X3-GPU] Keccak-256 GPU unavailable: {}", e);
                    None
                }
            },
            secp256k1: match load_secp256k1_lib() {
                Ok(lib) => {
                    log::info!("[X3-GPU] Loaded libsecp256k1_batch.so (optimized)");
                    Some(Arc::new(lib))
                }
                Err(e) => {
                    log::warn!("[X3-GPU] secp256k1 GPU unavailable: {}", e);
                    None
                }
            },
            atomic: match load_atomic_lib() {
                Ok(lib) => {
                    log::info!("[X3-GPU] Loaded libatomic_swap.so");
                    Some(Arc::new(lib))
                }
                Err(e) => {
                    log::warn!("[X3-GPU] Atomic Swap GPU unavailable: {}", e);
                    None
                }
            },
        }
    }

    /// Check if any GPU library is loaded
    pub fn is_available(&self) -> bool {
        self.sha256.is_some()
            || self.ed25519.is_some()
            || self.pipeline.is_some()
            || self.keccak256.is_some()
            || self.secp256k1.is_some()
            || self.atomic.is_some()
    }

    /// Register all GPU hostcalls into a hostcall registry.
    ///
    /// Prefer `register_on_vm()` when you have a `&mut VM` available.
    /// This method is kept for contexts that use a standalone `HostcallRegistry`.
    pub fn register_all(&self, registry: &mut HostcallRegistry) {
        let sha256 = self.sha256.clone();
        let ed25519 = self.ed25519.clone();
        let pipeline = self.pipeline.clone();
        let keccak256 = self.keccak256.clone();
        let secp256k1 = self.secp256k1.clone();
        let atomic = self.atomic.clone();

        {
            let lib = sha256.clone();
            registry.register(
                gpu_hostcall_ids::GPU_SHA256_BATCH,
                "gpu_sha256_batch",
                2,
                move |args| Self::handle_sha256_batch(&lib, args),
            );
        }
        {
            let lib = ed25519.clone();
            registry.register(
                gpu_hostcall_ids::GPU_ED25519_VERIFY,
                "gpu_ed25519_verify",
                2,
                move |args| Self::handle_ed25519_verify(&lib, args),
            );
        }
        {
            let lib = sha256.clone();
            registry.register(
                gpu_hostcall_ids::GPU_POH_CHAIN,
                "gpu_poh_chain",
                3,
                move |args| Self::handle_poh_chain(&lib, args),
            );
        }
        {
            let lib = pipeline.clone();
            registry.register(
                gpu_hostcall_ids::GPU_SHA256_STREAMED,
                "gpu_sha256_streamed",
                3,
                move |args| Self::handle_sha256_streamed(&lib, args),
            );
        }
        {
            let gpu_lib_count = [
                sha256.is_some(),
                ed25519.is_some(),
                pipeline.is_some(),
                keccak256.is_some(),
                secp256k1.is_some(),
            ]
            .iter()
            .filter(|&&present| present)
            .count() as i64;
            registry.register(
                gpu_hostcall_ids::GPU_DEVICE_COUNT,
                "gpu_device_count",
                0,
                move |_args| Ok(Some(Value::I64(gpu_lib_count))),
            );
        }
        {
            let lib = pipeline.clone();
            registry.register(
                gpu_hostcall_ids::GPU_BENCHMARK,
                "gpu_benchmark",
                2,
                move |args| Self::handle_benchmark(&lib, args),
            );
        }
        {
            let lib = keccak256.clone();
            registry.register(
                gpu_hostcall_ids::GPU_KECCAK256_BATCH,
                "gpu_keccak256_batch",
                2,
                move |args| Self::handle_keccak256_batch(&lib, args),
            );
        }
        {
            let lib = secp256k1.clone();
            registry.register(
                gpu_hostcall_ids::GPU_SECP256K1_VERIFY,
                "gpu_secp256k1_verify",
                2,
                move |args| Self::handle_secp256k1_verify(&lib, args),
            );
        }
        {
            let lib = atomic.clone();
            registry.register(
                gpu_hostcall_ids::GPU_ATOMIC_VERIFY,
                "gpu_atomic_verify",
                2,
                move |args| Self::handle_atomic_verify(&lib, args),
            );
        }
        {
            let lib = atomic.clone();
            registry.register(
                gpu_hostcall_ids::GPU_ATOMIC_COMMIT,
                "gpu_atomic_commit",
                2,
                move |args| Self::handle_atomic_commit(&lib, args),
            );
        }
    }

    /// Register all GPU hostcalls directly on a VM instance
    pub fn register_on_vm(&self, vm: &mut crate::vm::VM) {
        // We build a temporary registry, but actually we register
        // via the VM's public API for each hostcall.
        let sha256 = self.sha256.clone();
        let ed25519 = self.ed25519.clone();
        let pipeline = self.pipeline.clone();
        let keccak256 = self.keccak256.clone();
        let secp256k1 = self.secp256k1.clone();

        // 0xD0: gpu_sha256_batch
        {
            let lib = sha256.clone();
            vm.register_hostcall(
                gpu_hostcall_ids::GPU_SHA256_BATCH,
                "gpu_sha256_batch",
                2,
                move |args| Self::handle_sha256_batch(&lib, args),
            );
        }

        // 0xD1: gpu_ed25519_verify
        {
            let lib = ed25519.clone();
            vm.register_hostcall(
                gpu_hostcall_ids::GPU_ED25519_VERIFY,
                "gpu_ed25519_verify",
                2,
                move |args| Self::handle_ed25519_verify(&lib, args),
            );
        }

        // 0xD2: gpu_poh_chain
        {
            let lib = sha256.clone();
            vm.register_hostcall(
                gpu_hostcall_ids::GPU_POH_CHAIN,
                "gpu_poh_chain",
                3,
                move |args| Self::handle_poh_chain(&lib, args),
            );
        }

        // 0xD3: gpu_sha256_streamed
        {
            let lib = pipeline.clone();
            vm.register_hostcall(
                gpu_hostcall_ids::GPU_SHA256_STREAMED,
                "gpu_sha256_streamed",
                3,
                move |args| Self::handle_sha256_streamed(&lib, args),
            );
        }

        // 0xD4: gpu_device_count
        // Count must match register_all() which checks sha256, ed25519, pipeline,
        // keccak256, and secp256k1.
        {
            let gpu_lib_count = [
                sha256.is_some(),
                ed25519.is_some(),
                pipeline.is_some(),
                keccak256.is_some(),
                secp256k1.is_some(),
            ]
            .iter()
            .filter(|&&present| present)
            .count() as i64;
            vm.register_hostcall(
                gpu_hostcall_ids::GPU_DEVICE_COUNT,
                "gpu_device_count",
                0,
                move |_args| Ok(Some(Value::I64(gpu_lib_count))),
            );
        }

        // 0xD5: gpu_benchmark
        {
            let lib = pipeline.clone();
            vm.register_hostcall(
                gpu_hostcall_ids::GPU_BENCHMARK,
                "gpu_benchmark",
                2,
                move |args| Self::handle_benchmark(&lib, args),
            );
        }

        // 0xD6: gpu_keccak256_batch
        {
            let lib = keccak256.clone();
            vm.register_hostcall(
                gpu_hostcall_ids::GPU_KECCAK256_BATCH,
                "gpu_keccak256_batch",
                2,
                move |args| Self::handle_keccak256_batch(&lib, args),
            );
        }

        // 0xD7: gpu_secp256k1_verify
        {
            let lib = secp256k1.clone();
            vm.register_hostcall(
                gpu_hostcall_ids::GPU_SECP256K1_VERIFY,
                "gpu_secp256k1_verify",
                2,
                move |args| Self::handle_secp256k1_verify(&lib, args),
            );
        }

        // 0xD8: gpu_atomic_verify
        {
            let lib = self.atomic.clone();
            vm.register_hostcall(
                gpu_hostcall_ids::GPU_ATOMIC_VERIFY,
                "gpu_atomic_verify",
                2,
                move |args| Self::handle_atomic_verify(&lib, args),
            );
        }

        // 0xD9: gpu_atomic_commit
        {
            let lib = self.atomic.clone();
            vm.register_hostcall(
                gpu_hostcall_ids::GPU_ATOMIC_COMMIT,
                "gpu_atomic_commit",
                2,
                move |args| Self::handle_atomic_commit(&lib, args),
            );
        }
    }

    // ── Static dispatch handlers ────────────────────────────────────────

    fn handle_sha256_batch(
        lib: &Option<Arc<Sha256Lib>>,
        args: &[Value],
    ) -> VMResult<Option<Value>> {
        let count = args[1].as_i64()? as i32;
        if count <= 0 {
            return Ok(Some(Value::Bytes(vec![])));
        }

        let lib = lib.as_ref().ok_or_else(|| {
            VMError::without_ip(VMErrorKind::HostcallError(
                "GPU SHA-256 library not loaded".into(),
            ))
        })?;

        let inputs = match &args[0] {
            Value::Bytes(b) => b,
            _ => {
                return Err(VMError::without_ip(VMErrorKind::TypeMismatch(
                    "Bytes".into(),
                    format!("{:?}", args[0]),
                )))
            }
        };

        let expected_len = count as usize * 32;
        if inputs.len() < expected_len {
            return Err(VMError::without_ip(VMErrorKind::HostcallError(format!(
                "gpu_sha256_batch: input {} bytes, expected {}",
                inputs.len(),
                expected_len
            ))));
        }

        let mut output = vec![0u8; expected_len];
        let ret = unsafe { (lib.multi_gpu)(inputs.as_ptr(), count, output.as_mut_ptr()) };

        if ret != 0 {
            return Err(VMError::without_ip(VMErrorKind::HostcallError(format!(
                "gpu_sha256_batch: CUDA error code {}",
                ret
            ))));
        }

        Ok(Some(Value::Bytes(output)))
    }

    fn handle_ed25519_verify(
        lib: &Option<Arc<Ed25519Lib>>,
        args: &[Value],
    ) -> VMResult<Option<Value>> {
        let count = args[1].as_i64()? as i32;
        if count <= 0 {
            return Ok(Some(Value::Bytes(vec![])));
        }

        let lib = lib.as_ref().ok_or_else(|| {
            VMError::without_ip(VMErrorKind::HostcallError(
                "GPU Ed25519 library not loaded".into(),
            ))
        })?;

        let sigs = match &args[0] {
            Value::Bytes(b) => b,
            _ => {
                return Err(VMError::without_ip(VMErrorKind::TypeMismatch(
                    "Bytes".into(),
                    format!("{:?}", args[0]),
                )))
            }
        };

        let expected_len = count as usize * 128;
        if sigs.len() < expected_len {
            return Err(VMError::without_ip(VMErrorKind::HostcallError(format!(
                "gpu_ed25519_verify: input {} bytes, expected {}",
                sigs.len(),
                expected_len
            ))));
        }

        let mut results = vec![0u8; count as usize];
        let ret = unsafe { (lib.verify_multi_gpu)(sigs.as_ptr(), count, results.as_mut_ptr()) };

        if ret != 0 {
            return Err(VMError::without_ip(VMErrorKind::HostcallError(format!(
                "gpu_ed25519_verify: CUDA error code {}",
                ret
            ))));
        }

        Ok(Some(Value::Bytes(results)))
    }

    fn handle_poh_chain(lib: &Option<Arc<Sha256Lib>>, args: &[Value]) -> VMResult<Option<Value>> {
        let num_chains = args[1].as_i64()? as i32;
        let chain_length = args[2].as_i64()? as i32;
        if num_chains <= 0 || chain_length <= 0 {
            return Ok(Some(Value::Bytes(vec![])));
        }

        let lib = lib.as_ref().ok_or_else(|| {
            VMError::without_ip(VMErrorKind::HostcallError(
                "GPU SHA-256 library not loaded (needed for PoH)".into(),
            ))
        })?;

        let seeds = match &args[0] {
            Value::Bytes(b) => b,
            _ => {
                return Err(VMError::without_ip(VMErrorKind::TypeMismatch(
                    "Bytes".into(),
                    format!("{:?}", args[0]),
                )))
            }
        };

        let expected_len = num_chains as usize * 32;
        if seeds.len() < expected_len {
            return Err(VMError::without_ip(VMErrorKind::HostcallError(format!(
                "gpu_poh_chain: seed input {} bytes, expected {}",
                seeds.len(),
                expected_len
            ))));
        }

        let mut results = vec![0u8; expected_len];
        let ret = unsafe {
            (lib.poh_chain)(
                seeds.as_ptr(),
                num_chains,
                chain_length,
                results.as_mut_ptr(),
            )
        };

        if ret != 0 {
            return Err(VMError::without_ip(VMErrorKind::HostcallError(format!(
                "gpu_poh_chain: CUDA error code {}",
                ret
            ))));
        }

        Ok(Some(Value::Bytes(results)))
    }

    fn handle_sha256_streamed(
        lib: &Option<Arc<StreamPipelineLib>>,
        args: &[Value],
    ) -> VMResult<Option<Value>> {
        let lib = lib.as_ref().ok_or_else(|| {
            VMError::without_ip(VMErrorKind::HostcallError(
                "GPU stream pipeline library not loaded".into(),
            ))
        })?;

        let inputs = match &args[0] {
            Value::Bytes(b) => b,
            _ => {
                return Err(VMError::without_ip(VMErrorKind::TypeMismatch(
                    "Bytes".into(),
                    format!("{:?}", args[0]),
                )))
            }
        };
        let count = args[1].as_i64()? as i32;
        let num_streams = args[2].as_i64()? as i32;

        if count <= 0 {
            return Ok(Some(Value::Bytes(vec![])));
        }

        let expected_len = count as usize * 32;
        if inputs.len() < expected_len {
            return Err(VMError::without_ip(VMErrorKind::HostcallError(format!(
                "gpu_sha256_streamed: input {} bytes, expected {}",
                inputs.len(),
                expected_len
            ))));
        }

        let mut output = vec![0u8; expected_len];
        let ret = unsafe {
            (lib.batch_streamed)(inputs.as_ptr(), count, output.as_mut_ptr(), num_streams)
        };

        if ret != 0 {
            return Err(VMError::without_ip(VMErrorKind::HostcallError(format!(
                "gpu_sha256_streamed: CUDA error code {}",
                ret
            ))));
        }

        Ok(Some(Value::Bytes(output)))
    }

    fn handle_benchmark(
        lib: &Option<Arc<StreamPipelineLib>>,
        args: &[Value],
    ) -> VMResult<Option<Value>> {
        let lib = lib.as_ref().ok_or_else(|| {
            VMError::without_ip(VMErrorKind::HostcallError(
                "GPU pipeline library not loaded".into(),
            ))
        })?;

        let count = args[0].as_i64()? as i32;
        let streams = args[1].as_i64()? as i32;

        let mut total_ms: f32 = 0.0;
        let mut h2d_ms: f32 = 0.0;
        let mut compute_ms: f32 = 0.0;
        let mut d2h_ms: f32 = 0.0;
        let mut throughput: f32 = 0.0;

        let ret = unsafe {
            (lib.benchmark)(
                count,
                streams,
                &mut total_ms,
                &mut h2d_ms,
                &mut compute_ms,
                &mut d2h_ms,
                &mut throughput,
            )
        };

        if ret != 0 {
            return Err(VMError::without_ip(VMErrorKind::HostcallError(
                "gpu_benchmark: CUDA error".into(),
            )));
        }

        let json = format!(
            r#"{{"total_ms":{:.3},"h2d_ms":{:.3},"compute_ms":{:.3},"d2h_ms":{:.3},"throughput_mhps":{:.2}}}"#,
            total_ms, h2d_ms, compute_ms, d2h_ms, throughput
        );
        Ok(Some(Value::Bytes(json.into_bytes())))
    }

    // ── Keccak-256 handler ──────────────────────────────────────────────

    fn handle_keccak256_batch(
        lib: &Option<Arc<Keccak256Lib>>,
        args: &[Value],
    ) -> VMResult<Option<Value>> {
        let lib = lib.as_ref().ok_or_else(|| {
            VMError::without_ip(VMErrorKind::HostcallError(
                "GPU Keccak-256 library not loaded".into(),
            ))
        })?;

        let inputs = match &args[0] {
            Value::Bytes(b) => b,
            _ => {
                return Err(VMError::without_ip(VMErrorKind::TypeMismatch(
                    "Bytes".into(),
                    format!("{:?}", args[0]),
                )))
            }
        };
        let count = args[1].as_i64()? as i32;

        if count <= 0 {
            return Ok(Some(Value::Bytes(vec![])));
        }

        let expected_len = count as usize * 32;
        if inputs.len() < expected_len {
            return Err(VMError::without_ip(VMErrorKind::HostcallError(format!(
                "gpu_keccak256_batch: input {} bytes, expected {}",
                inputs.len(),
                expected_len
            ))));
        }

        let mut output = vec![0u8; expected_len];
        let ret = unsafe { (lib.batch)(inputs.as_ptr(), count, output.as_mut_ptr()) };
        if ret != 0 {
            return Err(VMError::without_ip(VMErrorKind::HostcallError(format!(
                "gpu_keccak256_batch: CUDA error code {}",
                ret
            ))));
        }
        Ok(Some(Value::Bytes(output)))
    }

    // ── secp256k1 handler ───────────────────────────────────────────────

    fn handle_secp256k1_verify(
        lib: &Option<Arc<Secp256k1Lib>>,
        args: &[Value],
    ) -> VMResult<Option<Value>> {
        let lib = lib.as_ref().ok_or_else(|| {
            VMError::without_ip(VMErrorKind::HostcallError(
                "GPU secp256k1 library not loaded".into(),
            ))
        })?;

        // Input layout: packed [u1(32) | u2(32) | pubkey(64)] × count = 128 bytes/sig
        let sigs = match &args[0] {
            Value::Bytes(b) => b,
            _ => {
                return Err(VMError::without_ip(VMErrorKind::TypeMismatch(
                    "Bytes".into(),
                    format!("{:?}", args[0]),
                )))
            }
        };
        let count = args[1].as_i64()? as i32;

        if count <= 0 {
            return Ok(Some(Value::Bytes(vec![])));
        }

        let expected_len = count as usize * 128;
        if sigs.len() < expected_len {
            return Err(VMError::without_ip(VMErrorKind::HostcallError(format!(
                "gpu_secp256k1_verify: input {} bytes, expected {}",
                sigs.len(),
                expected_len
            ))));
        }

        // Unpack: separate u1, u2, pubkey arrays for the CUDA API
        let n = count as usize;
        let mut u1 = vec![0u8; n * 32];
        let mut u2 = vec![0u8; n * 32];
        let mut pk = vec![0u8; n * 64];
        for i in 0..n {
            let base = i * 128;
            u1[i * 32..(i + 1) * 32].copy_from_slice(&sigs[base..base + 32]);
            u2[i * 32..(i + 1) * 32].copy_from_slice(&sigs[base + 32..base + 64]);
            pk[i * 64..(i + 1) * 64].copy_from_slice(&sigs[base + 64..base + 128]);
        }

        let mut output = vec![0u8; n * 32];
        let ret = unsafe {
            (lib.verify_multi_gpu)(
                u1.as_ptr(),
                u2.as_ptr(),
                pk.as_ptr(),
                count,
                output.as_mut_ptr(),
            )
        };
        if ret != 0 {
            return Err(VMError::without_ip(VMErrorKind::HostcallError(format!(
                "gpu_secp256k1_verify: CUDA error code {}",
                ret
            ))));
        }
        Ok(Some(Value::Bytes(output)))
    }

    // ── Atomic Swap handlers ───────────────────────────────────────────

    fn handle_atomic_verify(
        lib: &Option<Arc<AtomicLib>>,
        args: &[Value],
    ) -> VMResult<Option<Value>> {
        let lib = lib.as_ref().ok_or_else(|| {
            VMError::without_ip(VMErrorKind::HostcallError(
                "GPU Atomic Swap library not loaded".into(),
            ))
        })?;

        let svm_data = match &args[0] {
            Value::Bytes(b) => b,
            _ => {
                return Err(VMError::without_ip(VMErrorKind::TypeMismatch(
                    "Bytes".into(),
                    format!("{:?}", args[0]),
                )))
            }
        };
        let evm_data = match &args[1] {
            Value::Bytes(b) => b,
            _ => {
                return Err(VMError::without_ip(VMErrorKind::TypeMismatch(
                    "Bytes".into(),
                    format!("{:?}", args[1]),
                )))
            }
        };

        let mut status = vec![0u8; 1];
        let ret =
            unsafe { (lib.verify)(svm_data.as_ptr(), evm_data.as_ptr(), 1, status.as_mut_ptr()) };

        if ret != 0 {
            return Err(VMError::without_ip(VMErrorKind::HostcallError(format!(
                "gpu_atomic_verify: CUDA error code {}",
                ret
            ))));
        }

        Ok(Some(Value::Bool(status[0] == 1)))
    }

    fn handle_atomic_commit(
        lib: &Option<Arc<AtomicLib>>,
        args: &[Value],
    ) -> VMResult<Option<Value>> {
        let lib = lib.as_ref().ok_or_else(|| {
            VMError::without_ip(VMErrorKind::HostcallError(
                "GPU Atomic Swap library not loaded".into(),
            ))
        })?;

        let svm_data = match &args[0] {
            Value::Bytes(b) => b,
            _ => {
                return Err(VMError::without_ip(VMErrorKind::TypeMismatch(
                    "Bytes".into(),
                    format!("{:?}", args[0]),
                )))
            }
        };
        let evm_data = match &args[1] {
            Value::Bytes(b) => b,
            _ => {
                return Err(VMError::without_ip(VMErrorKind::TypeMismatch(
                    "Bytes".into(),
                    format!("{:?}", args[1]),
                )))
            }
        };

        let ret = unsafe { (lib.commit)(svm_data.as_ptr(), evm_data.as_ptr(), 1) };

        if ret <= 0 {
            return Err(VMError::without_ip(VMErrorKind::HostcallError(format!(
                "gpu_atomic_commit: commit failed (returned {})",
                ret
            ))));
        }

        Ok(Some(Value::Bool(true)))
    }
}

impl Default for GpuHostcalls {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for GpuHostcalls {
    fn drop(&mut self) {
        // Call pipeline cleanup if loaded
        if let Some(ref pipeline) = self.pipeline {
            unsafe { (pipeline.cleanup)() };
        }
    }
}

/// Configuration for GPU hostcall registration
#[derive(Clone, Debug)]
pub struct GpuConfig {
    /// Enable GPU hostcalls
    pub enable_gpu: bool,
    /// Prefer multi-GPU dispatch over single GPU
    pub multi_gpu: bool,
    /// Default number of CUDA streams for pipelined operations
    pub default_streams: i32,
    /// Custom library search directory
    pub lib_dir: Option<PathBuf>,
}

impl Default for GpuConfig {
    fn default() -> Self {
        Self {
            enable_gpu: true,
            multi_gpu: true,
            default_streams: 4,
            lib_dir: None,
        }
    }
}
