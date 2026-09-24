//! Sandbox Manager for DePIN GPU Marketplace
//!
//! Proposal: DEPIN-GPU-001
//!
//! Isolates external workloads in containers/VMs to prevent:
//! - Memory access violations
//! - GPU memory corruption
//! - Resource exhaustion attacks
//! - Side-channel attacks on other lanes
//!
//! ## Architecture
//!
//! ```text
//! ┌────────────────────────────────────────────────┐
//! │              Sandbox Manager                   │
//! │                                                │
//! │  ┌──────────┐  ┌──────────┐  ┌──────────────┐ │
//! │  │ Sandbox  │  │ Sandbox  │  │  Resource     │ │
//! │  │ (OCI)    │  │ (nsjail) │  │  Monitor      │ │
//! │  └──────┬───┘  └──────┬───┘  └──────┬───────┘ │
//! │         │              │             │         │
//! │    ┌────▼──────────────▼─────────────▼────┐    │
//! │    │      GPU MIG / MPS Partition          │   │
//! │    └───────────────────────────────────────┘   │
//! └────────────────────────────────────────────────┘
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

/// Unique sandbox identifier.
pub type SandboxId = String;

/// Sandbox isolation level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IsolationLevel {
    /// OCI container with GPU passthrough.
    Container,
    /// nsjail with fine-grained syscall filtering.
    Nsjail,
    /// Full VM with GPU VFIO passthrough.
    VirtualMachine,
    /// NVIDIA MIG partition (H100/A100).
    MigPartition,
}

/// Resource limits for a sandbox.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    /// Maximum GPU memory (MB).
    pub max_vram_mb: u32,
    /// Maximum system memory (MB).
    pub max_ram_mb: u32,
    /// CPU core limit (fractional).
    pub cpu_cores: f32,
    /// Maximum disk space (MB).
    pub max_disk_mb: u32,
    /// Network bandwidth limit (Mbps), 0 = no network.
    pub network_bandwidth_mbps: u32,
    /// Maximum execution time.
    pub timeout: Duration,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_vram_mb: 4096,
            max_ram_mb: 8192,
            cpu_cores: 2.0,
            max_disk_mb: 10240,
            network_bandwidth_mbps: 0, // No network by default
            timeout: Duration::from_secs(3600),
        }
    }
}

/// Status of a sandbox.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SandboxStatus {
    Creating,
    Running,
    Paused,
    Checkpointing,
    Stopped,
    Failed,
}

/// A sandbox instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sandbox {
    pub id: SandboxId,
    pub order_id: [u8; 16],
    pub isolation: IsolationLevel,
    pub limits: ResourceLimits,
    pub status: SandboxStatus,
    pub created_at: u64,
    pub gpu_utilization: f32,
    pub vram_used_mb: u32,
}

/// Manages sandbox lifecycle.
pub struct SandboxManager {
    sandboxes: HashMap<SandboxId, Sandbox>,
    max_sandboxes: usize,
}

impl SandboxManager {
    /// Create a new sandbox manager.
    pub fn new(max_sandboxes: usize) -> Self {
        Self {
            sandboxes: HashMap::new(),
            max_sandboxes,
        }
    }

    /// Create a new sandbox for a rental workload.
    pub fn create(
        &mut self,
        order_id: [u8; 16],
        isolation: IsolationLevel,
        limits: ResourceLimits,
    ) -> Result<SandboxId, SandboxError> {
        if self.sandboxes.len() >= self.max_sandboxes {
            return Err(SandboxError::AtCapacity);
        }

        let id = format!(
            "sb-{}",
            uuid::Uuid::new_v4()
                .to_string()
                .split('-')
                .next()
                .unwrap_or("0")
        );

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let sandbox = Sandbox {
            id: id.clone(),
            order_id,
            isolation,
            limits,
            status: SandboxStatus::Creating,
            created_at: now,
            gpu_utilization: 0.0,
            vram_used_mb: 0,
        };

        // In production: spawn OCI container / nsjail / VM
        self.sandboxes.insert(id.clone(), sandbox);

        Ok(id)
    }

    /// Start a sandbox.
    pub fn start(&mut self, sandbox_id: &str) -> Result<(), SandboxError> {
        let sandbox = self
            .sandboxes
            .get_mut(sandbox_id)
            .ok_or(SandboxError::NotFound)?;

        sandbox.status = SandboxStatus::Running;
        Ok(())
    }

    /// Checkpoint a sandbox's state (for preemption).
    ///
    /// Must complete within the preemption budget (2ms default).
    pub fn checkpoint(&mut self, sandbox_id: &str) -> Result<Vec<u8>, SandboxError> {
        let sandbox = self
            .sandboxes
            .get_mut(sandbox_id)
            .ok_or(SandboxError::NotFound)?;

        sandbox.status = SandboxStatus::Checkpointing;

        // In production: CRIU checkpoint or GPU state dump
        let checkpoint_data = format!("checkpoint-{}", sandbox_id).into_bytes();

        sandbox.status = SandboxStatus::Paused;

        Ok(checkpoint_data)
    }

    /// Resume a sandbox from checkpoint.
    pub fn resume(&mut self, sandbox_id: &str, _checkpoint: &[u8]) -> Result<(), SandboxError> {
        let sandbox = self
            .sandboxes
            .get_mut(sandbox_id)
            .ok_or(SandboxError::NotFound)?;

        sandbox.status = SandboxStatus::Running;
        Ok(())
    }

    /// Stop and remove a sandbox.
    pub fn destroy(&mut self, sandbox_id: &str) -> Result<(), SandboxError> {
        self.sandboxes
            .remove(sandbox_id)
            .ok_or(SandboxError::NotFound)?;
        Ok(())
    }

    /// Get sandbox status.
    pub fn status(&self, sandbox_id: &str) -> Option<SandboxStatus> {
        self.sandboxes.get(sandbox_id).map(|s| s.status)
    }

    /// Number of active sandboxes.
    pub fn active_count(&self) -> usize {
        self.sandboxes
            .values()
            .filter(|s| matches!(s.status, SandboxStatus::Running | SandboxStatus::Paused))
            .count()
    }

    /// Total VRAM allocated across all sandboxes.
    pub fn total_vram_allocated(&self) -> u32 {
        self.sandboxes.values().map(|s| s.limits.max_vram_mb).sum()
    }
}

/// Errors from the sandbox manager.
#[derive(Debug, thiserror::Error)]
pub enum SandboxError {
    #[error("Sandbox not found")]
    NotFound,
    #[error("At maximum sandbox capacity")]
    AtCapacity,
    #[error("Checkpoint failed: {0}")]
    CheckpointFailed(String),
    #[error("Resource limit exceeded: {0}")]
    ResourceExceeded(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_and_start_sandbox() {
        let mut mgr = SandboxManager::new(4);

        let id = mgr
            .create(
                [0x01; 16],
                IsolationLevel::Container,
                ResourceLimits::default(),
            )
            .unwrap();

        assert_eq!(mgr.status(&id), Some(SandboxStatus::Creating));

        mgr.start(&id).unwrap();
        assert_eq!(mgr.status(&id), Some(SandboxStatus::Running));
    }

    #[test]
    fn checkpoint_and_resume() {
        let mut mgr = SandboxManager::new(4);

        let id = mgr
            .create(
                [0x01; 16],
                IsolationLevel::MigPartition,
                ResourceLimits::default(),
            )
            .unwrap();
        mgr.start(&id).unwrap();

        let checkpoint = mgr.checkpoint(&id).unwrap();
        assert_eq!(mgr.status(&id), Some(SandboxStatus::Paused));

        mgr.resume(&id, &checkpoint).unwrap();
        assert_eq!(mgr.status(&id), Some(SandboxStatus::Running));
    }

    #[test]
    fn capacity_enforced() {
        let mut mgr = SandboxManager::new(1);

        mgr.create(
            [0x01; 16],
            IsolationLevel::Container,
            ResourceLimits::default(),
        )
        .unwrap();

        let result = mgr.create(
            [0x02; 16],
            IsolationLevel::Container,
            ResourceLimits::default(),
        );
        assert!(matches!(result, Err(SandboxError::AtCapacity)));
    }

    #[test]
    fn destroy_removes_sandbox() {
        let mut mgr = SandboxManager::new(4);
        let id = mgr
            .create(
                [0x01; 16],
                IsolationLevel::Container,
                ResourceLimits::default(),
            )
            .unwrap();

        mgr.destroy(&id).unwrap();
        assert!(mgr.status(&id).is_none());
    }
}
