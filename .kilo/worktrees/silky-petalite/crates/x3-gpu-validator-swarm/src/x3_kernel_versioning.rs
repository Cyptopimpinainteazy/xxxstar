//! X3 Kernel versioning and lifecycle management
//!
//! X3 kernels are GPU execution units that abstract away CUDA/OpenCL differences.
//! Versioning system allows hot updates without node restarts.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

/// X3 Kernel metadata
#[derive(Clone, Debug)]
pub struct X3KernelManifest {
    /// Kernel name (e.g., "matmul_v3.2", "conv2d_v1.0")
    pub name: String,
    /// Semantic version (major.minor.patch)
    pub version: String,
    /// Kernel type
    pub kernel_type: KernelType,
    /// SHA-256 hash of kernel binary
    pub binary_hash: [u8; 32],
    /// Minimum GPU compute capability (e.g., 7.0 for Turing)
    pub min_gpu_capability: String,
    /// Kernel binary size in bytes
    pub binary_size: u32,
    /// Registered at block height
    pub registered_height: u32,
    /// Is this kernel production-ready?
    pub approved: bool,
}

/// Kernel execution type
#[derive(Clone, Debug)]
pub enum KernelType {
    /// Matrix multiply: (M×K) × (K×N) → (M×N)
    MatMul,
    /// 2D convolution
    Conv2D,
    /// FFT computation
    FFT,
    /// Reduction (sum, max, etc)
    Reduce,
    /// Custom bytecode execution
    Custom(String),
}

/// Governance account for kernel approvals
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GovernanceAccount {
    /// Account ID
    pub id: String,
    /// Public key
    pub pubkey: [u8; 32],
    /// Role
    pub role: GovernanceRole,
}

/// Governance role
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum GovernanceRole {
    /// Kernel approver
    KernelApprover,
    /// Admin
    Admin,
}

/// Signature for multi-sig approval
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Signature {
    /// Signer account ID
    pub signer_id: String,
    /// Signature bytes
    pub signature: Vec<u8>,
}

/// Kernel approval record
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KernelApproval {
    /// Kernel name
    pub kernel_name: String,
    /// Version
    pub version: String,
    /// Approved by
    pub approved_by: String,
    /// Signatures
    pub signatures: Vec<Signature>,
    /// Timestamp
    pub timestamp: chrono::DateTime<Utc>,
}

/// Kernel registry (per-validator)
#[derive(Clone)]
pub struct X3KernelRegistry {
    /// kernel_name → Vec<manifest> (all versions)
    pub kernels: HashMap<String, Vec<X3KernelManifest>>,
    /// Current active kernel version per type
    pub active: HashMap<String, String>, // kernel_name → active_version
    /// Update history: (block_height, kernel_name, old_version → new_version)
    pub update_history: Vec<(u32, String, String, String)>,
    /// Approval log for audit
    pub approval_log: Vec<KernelApproval>,
    /// Required signatures for approval (2/3 of validators)
    pub required_approval_signatures: usize,
}

impl X3KernelRegistry {
    pub fn new() -> Self {
        Self {
            kernels: HashMap::new(),
            active: HashMap::new(),
            update_history: Vec::new(),
            approval_log: Vec::new(),
            required_approval_signatures: 2, // Default 2/3 multi-sig
        }
    }

    /// Verify governance authority
    fn verify_governance_authority(
        &self,
        approver: &GovernanceAccount,
        signatures: &[Signature],
    ) -> bool {
        // Verify approver has correct role
        if approver.role != GovernanceRole::KernelApprover && approver.role != GovernanceRole::Admin
        {
            return false;
        }

        // Verify minimum signatures
        if signatures.len() < self.required_approval_signatures {
            return false;
        }

        // In production: verify each signature cryptographically
        // For now: just check non-empty
        signatures.iter().all(|s| !s.signature.is_empty())
    }

    /// Register a new kernel version
    pub fn register_kernel(
        &mut self,
        manifest: X3KernelManifest,
        _block_height: u32,
    ) -> Result<(), String> {
        if manifest.version.is_empty() {
            return Err("Version cannot be empty".to_string());
        }

        let name = manifest.name.clone();
        let version = manifest.version.clone();

        // Check: version doesn't already exist
        if let Some(versions) = self.kernels.get(&name) {
            if versions.iter().any(|k| k.version == version) {
                return Err(format!(
                    "Kernel {} version {} already exists",
                    name, version
                ));
            }
        }

        // Add to registry
        self.kernels
            .entry(name.clone())
            .or_insert_with(Vec::new)
            .push(manifest);

        // Auto-activate if first version
        if !self.active.contains_key(&name) {
            self.active.insert(name, version);
        }

        Ok(())
    }

    /// Get active kernel version
    pub fn get_active_kernel(&self, kernel_name: &str) -> Option<X3KernelManifest> {
        let active_version = self.active.get(kernel_name)?;
        self.kernels
            .get(kernel_name)?
            .iter()
            .find(|k| k.version == *active_version)
            .cloned()
    }

    /// Switch to a different kernel version
    pub fn activate_kernel(
        &mut self,
        kernel_name: String,
        version: String,
        block_height: u32,
    ) -> Result<(), String> {
        // Verify version exists
        let kernel = self
            .kernels
            .get(&kernel_name)
            .and_then(|versions| versions.iter().find(|k| k.version == version))
            .ok_or_else(|| "Kernel version not found".to_string())?;

        // Verify kernel is approved
        if !kernel.approved {
            return Err("Kernel not approved for production".to_string());
        }

        let old_version = self.active.get(&kernel_name).cloned().unwrap_or_default();

        // Activate
        self.active.insert(kernel_name.clone(), version.clone());

        // Log update
        self.update_history
            .push((block_height, kernel_name, old_version, version));

        Ok(())
    }

    /// Get all versions of a kernel
    pub fn get_kernel_versions(&self, kernel_name: &str) -> Vec<X3KernelManifest> {
        self.kernels.get(kernel_name).cloned().unwrap_or_default()
    }

    /// Approve a kernel for production use (governance action)
    pub fn approve_kernel(
        &mut self,
        kernel_name: &str,
        version: &str,
        approver: &GovernanceAccount,
        signatures: Vec<Signature>,
    ) -> Result<(), String> {
        // Verify governance authorization
        if !self.verify_governance_authority(approver, &signatures) {
            return Err("Unauthorized: kernel approval requires governance vote".to_string());
        }

        // Verify minimum signatures (e.g., 2/3 of validators)
        if signatures.len() < self.required_approval_signatures {
            return Err(format!(
                "Insufficient signatures: {} required, {} provided",
                self.required_approval_signatures,
                signatures.len()
            ));
        }

        if let Some(versions) = self.kernels.get_mut(kernel_name) {
            if let Some(kernel) = versions.iter_mut().find(|k| k.version == version) {
                // Log approval for audit
                self.approval_log.push(KernelApproval {
                    kernel_name: kernel_name.to_string(),
                    version: version.to_string(),
                    approved_by: approver.id.clone(),
                    signatures,
                    timestamp: Utc::now(),
                });

                kernel.approved = true;
                return Ok(());
            }
        }
        Err("Kernel not found".to_string())
    }

    /// Get update timeline for a kernel
    pub fn get_update_history(&self, kernel_name: &str) -> Vec<(u32, String, String)> {
        self.update_history
            .iter()
            .filter(|(_, name, _, _)| name == kernel_name)
            .map(|(height, _, old_v, new_v)| (*height, old_v.clone(), new_v.clone()))
            .collect()
    }

    /// Validate kernel binary (checksum verification)
    pub fn verify_kernel(
        &self,
        kernel_name: &str,
        version: &str,
        binary: &[u8],
        provided_hash: &[u8; 32],
    ) -> bool {
        // Get the manifest
        let manifest = match self
            .kernels
            .get(kernel_name)
            .and_then(|v| v.iter().find(|k| k.version == version))
        {
            Some(m) => m,
            None => return false,
        };

        // Verify binary size
        if manifest.binary_size as usize != binary.len() {
            return false;
        }

        // Actually compute hash of the binary
        let mut hasher = Sha256::new();
        hasher.update(binary);
        let computed_hash = hasher.finalize();
        let mut computed_hash_array = [0u8; 32];
        computed_hash_array.copy_from_slice(&computed_hash);

        // Verify both:
        // 1. Computed hash matches the manifest
        // 2. Provided hash matches the manifest (for compatibility)
        manifest.binary_hash == computed_hash_array && manifest.binary_hash == *provided_hash
    }
}

/// Kernel runtime: loads and executes active kernels
#[derive(Clone)]
pub struct X3KernelRuntime {
    registry: X3KernelRegistry,
    /// Cached loaded kernels (name → binary)
    kernel_cache: HashMap<String, Vec<u8>>,
    /// Kernel execution stats
    pub total_executions: u64,
    pub total_errors: u64,
}

impl X3KernelRuntime {
    pub fn new(registry: X3KernelRegistry) -> Self {
        Self {
            registry,
            kernel_cache: HashMap::new(),
            total_executions: 0,
            total_errors: 0,
        }
    }

    /// Load a kernel for execution
    pub fn load_kernel(&mut self, kernel_name: &str) -> Result<Vec<u8>, String> {
        // Check cache first
        if let Some(binary) = self.kernel_cache.get(kernel_name) {
            return Ok(binary.clone());
        }

        // Get active kernel
        let kernel = self
            .registry
            .get_active_kernel(kernel_name)
            .ok_or_else(|| "Kernel not found".to_string())?;

        // In production: load from disk or remote registry
        // For now: return empty binary
        let binary = vec![0u8; kernel.binary_size as usize];

        // Cache it
        self.kernel_cache
            .insert(kernel_name.to_string(), binary.clone());

        Ok(binary)
    }

    /// Execute a kernel
    pub fn execute_kernel(&mut self, kernel_name: &str, _args: &[u8]) -> Result<Vec<u8>, String> {
        let _binary = self.load_kernel(kernel_name)?;

        self.total_executions += 1;

        // In production: call native code / GPU
        // For now: return mock result
        match kernel_name {
            "matmul" => Ok(vec![1u8; 32]),
            "conv2d" => Ok(vec![2u8; 32]),
            _ => {
                self.total_errors += 1;
                Err("Unknown kernel".to_string())
            }
        }
    }

    /// Hot-reload: switch kernel version mid-execution
    pub fn hot_reload_kernel(
        &mut self,
        kernel_name: String,
        new_version: String,
    ) -> Result<(), String> {
        // Invalidate cache
        self.kernel_cache.remove(&kernel_name);

        // Activate new version (in production: governance votes first)
        self.registry.activate_kernel(kernel_name, new_version, 0)?;

        Ok(())
    }

    /// Get execution statistics
    pub fn get_stats(&self) -> (u64, u64) {
        (self.total_executions, self.total_errors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kernel_registry_creation() {
        let registry = X3KernelRegistry::new();
        assert!(registry.kernels.is_empty());
    }

    #[test]
    fn test_register_kernel() {
        let mut registry = X3KernelRegistry::new();

        let manifest = X3KernelManifest {
            name: "matmul".to_string(),
            version: "1.0.0".to_string(),
            kernel_type: KernelType::MatMul,
            binary_hash: [0u8; 32],
            min_gpu_capability: "7.0".to_string(),
            binary_size: 1024,
            registered_height: 100,
            approved: true,
        };

        assert!(registry.register_kernel(manifest, 100).is_ok());
    }

    #[test]
    fn test_activate_kernel() {
        let mut registry = X3KernelRegistry::new();

        let manifest = X3KernelManifest {
            name: "matmul".to_string(),
            version: "1.0.0".to_string(),
            kernel_type: KernelType::MatMul,
            binary_hash: [0u8; 32],
            min_gpu_capability: "7.0".to_string(),
            binary_size: 1024,
            registered_height: 100,
            approved: true,
        };

        registry.register_kernel(manifest, 100).ok();

        let active = registry.get_active_kernel("matmul");
        assert!(active.is_some());
    }

    #[test]
    fn test_kernel_versioning() {
        let mut registry = X3KernelRegistry::new();

        for v in 1..=3 {
            let manifest = X3KernelManifest {
                name: "conv2d".to_string(),
                version: format!("1.{}.0", v),
                kernel_type: KernelType::Conv2D,
                binary_hash: [v as u8; 32],
                min_gpu_capability: "7.0".to_string(),
                binary_size: 2048,
                registered_height: 100 + v as u32,
                approved: true,
            };

            registry.register_kernel(manifest, 100 + v as u32).ok();
        }

        let versions = registry.get_kernel_versions("conv2d");
        assert_eq!(versions.len(), 3);
    }

    #[test]
    fn test_kernel_approval() {
        let mut registry = X3KernelRegistry::new();

        let mut manifest = X3KernelManifest {
            name: "fft".to_string(),
            version: "1.0.0".to_string(),
            kernel_type: KernelType::FFT,
            binary_hash: [1u8; 32],
            min_gpu_capability: "8.0".to_string(),
            binary_size: 4096,
            registered_height: 150,
            approved: false, // Not approved yet
        };

        registry.register_kernel(manifest.clone(), 150).ok();

        // Can't activate unapproved
        let result = registry.activate_kernel("fft".to_string(), "1.0.0".to_string(), 160);
        assert!(result.is_err());

        // Approve it
        let approver = GovernanceAccount {
            id: "gov-1".to_string(),
            pubkey: [7u8; 32],
            role: GovernanceRole::KernelApprover,
        };
        let signatures = vec![
            Signature {
                signer_id: "sig-1".to_string(),
                signature: vec![1, 2, 3],
            },
            Signature {
                signer_id: "sig-2".to_string(),
                signature: vec![4, 5, 6],
            },
        ];
        registry
            .approve_kernel("fft", "1.0.0", &approver, signatures)
            .ok();

        // Now can activate
        let result = registry.activate_kernel("fft".to_string(), "1.0.0".to_string(), 160);
        assert!(result.is_ok());
    }

    #[test]
    fn test_kernel_runtime_execution() {
        let registry = X3KernelRegistry::new();
        let mut runtime = X3KernelRuntime::new(registry);

        // Mock execution
        let _result = runtime.execute_kernel("matmul", &[]);
        assert_eq!(runtime.total_executions, 1);
    }

    #[test]
    fn test_kernel_hot_reload() {
        let mut registry = X3KernelRegistry::new();

        let m1 = X3KernelManifest {
            name: "reduce".to_string(),
            version: "1.0.0".to_string(),
            kernel_type: KernelType::Reduce,
            binary_hash: [1u8; 32],
            min_gpu_capability: "7.0".to_string(),
            binary_size: 512,
            registered_height: 100,
            approved: true,
        };

        let m2 = X3KernelManifest {
            name: "reduce".to_string(),
            version: "2.0.0".to_string(),
            kernel_type: KernelType::Reduce,
            binary_hash: [2u8; 32],
            min_gpu_capability: "7.0".to_string(),
            binary_size: 768,
            registered_height: 200,
            approved: true,
        };

        registry.register_kernel(m1, 100).ok();
        registry.register_kernel(m2, 200).ok();

        let mut runtime = X3KernelRuntime::new(registry);

        runtime
            .hot_reload_kernel("reduce".to_string(), "2.0.0".to_string())
            .ok();

        let active = runtime.registry.get_active_kernel("reduce");
        assert_eq!(active.unwrap().version, "2.0.0");
    }
}
