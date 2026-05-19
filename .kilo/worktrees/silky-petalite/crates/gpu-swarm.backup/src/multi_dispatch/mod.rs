//! # GPU Multi-Device Dispatch Layer
//!
//! ## Overview
//!
//! The `MultiDeviceDispatcher` extends the existing `GpuExecutorManager` with:
//!
//! 1. **Multi-device enumeration** — discovers all CUDA/Vulkan/OpenCL devices across
//!    all registered backends and builds a unified device slot table.
//!
//! 2. **Weighted round-robin dispatch** — routes tasks to devices proportionally to
//!    their peak FP32 throughput (TFLOPS), so faster GPUs naturally absorb more work.
//!
//! 3. **Per-device attestation** — each device slot carries a `DeviceAttestation`
//!    proving it is what it claims to be. Attestations have TTLs; the dispatcher
//!    refuses to route to slots with expired attestations.
//!
//! 4. **k-redundancy execution** — for critical tasks (e.g., finality-adjacent sig
//!    verification), the dispatcher fans the same task to k independent devices and
//!    requires ≥ ⌊k/2⌋+1 identical outputs before accepting the result.
//!
//! 5. **Fault isolation** — a device slot that fails 3× in a row is quarantined and
//!    skipped until its attestation is refreshed.
//!
//! ## Attestation Format
//!
//! ```text
//! DeviceAttestation {
//!   device_id:    u32         — index within the physical device list
//!   backend:      GpuBackendType
//!   gpu_model:    String      — e.g. "NVIDIA H100 80GB SXM5"
//!   tflops_fp32:  f32         — peak throughput claim
//!   report_hash:  [u8;32]     — SHA-256 of device capabilities report
//!   issued_at_ms: u64         — Unix ms timestamp
//!   expires_at_ms: u64        — issued_at_ms + TTL
//! }
//! ```
//!
//! The `report_hash` in production comes from the NVIDIA Confidential Computing
//! attestation API. In simulation mode (the current default) it is SHA-256 of
//! `{device_id}||{backend}||{gpu_model}||{tflops_fp32 as u32 LE bytes}`.
//!
//! ## Audit Alignment
//!
//! Per the deep-research audit: "GPU swarm needs multi-device dispatch + per-device
//! attestation with cooldowns + redundancy for critical path tasks."

use crate::gpu_backends::{GpuBackendType, GpuDeviceInfo};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tracing::{error, info, warn};

// ─── Attestation ──────────────────────────────────────────────────────────────

/// Per-device attestation record.
///
/// In production: loaded from the NVIDIA CC attestation API or similar TEE report.
/// In simulation: computed deterministically from device capabilities.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DeviceAttestation {
    /// Physical device ID (within the backend).
    pub device_id: u32,
    /// GPU compute backend.
    pub backend: GpuBackendType,
    /// Human-readable model string.
    pub gpu_model: String,
    /// Claimed peak FP32 throughput in TFLOPS.
    pub tflops_fp32: f32,
    /// SHA-256 of device capability report (or CC attestation blob).
    pub report_hash: [u8; 32],
    /// Unix millisecond timestamp when attestation was issued.
    pub issued_at_ms: u64,
    /// Unix millisecond timestamp when attestation expires (issued + TTL).
    pub expires_at_ms: u64,
}

impl DeviceAttestation {
    /// Default TTL: 1 hour (3_600_000 ms).
    pub const DEFAULT_TTL_MS: u64 = 3_600_000;

    /// Create a simulation-mode attestation from device info.
    pub fn from_device_info_sim(info: &GpuDeviceInfo) -> Self {
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        // Deterministic report hash for simulation
        let mut h = Sha256::new();
        h.update(info.device_id.to_le_bytes());
        h.update(format!("{:?}", info.backend).as_bytes());
        h.update(info.name.as_bytes());
        h.update((info.peak_fp32_tflops as u32).to_le_bytes());
        let report_hash: [u8; 32] = h.finalize().into();

        Self {
            device_id: info.device_id,
            backend: info.backend,
            gpu_model: info.name.clone(),
            tflops_fp32: info.peak_fp32_tflops,
            report_hash,
            issued_at_ms: now_ms,
            expires_at_ms: now_ms + Self::DEFAULT_TTL_MS,
        }
    }

    /// Check if this attestation is currently valid.
    pub fn is_valid(&self) -> bool {
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        now_ms < self.expires_at_ms
    }

    /// Remaining validity in milliseconds (0 if expired).
    pub fn remaining_ms(&self) -> u64 {
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        self.expires_at_ms.saturating_sub(now_ms)
    }
}

// ─── Device Slot ─────────────────────────────────────────────────────────────

/// A single slot in the dispatcher's device table.
#[derive(Debug, Clone)]
pub struct DeviceSlot {
    /// Stable index in the dispatcher's slot table.
    pub slot_id: usize,
    /// Device capabilities (from the backend).
    pub info: GpuDeviceInfo,
    /// Current attestation for this slot.
    pub attestation: DeviceAttestation,
    /// Dispatch weight — proportional to `tflops_fp32`.
    /// Used by weighted round-robin.
    pub weight: u32,
    /// Current health status.
    pub status: SlotStatus,
    /// Consecutive failure count. Quarantine threshold: 3.
    pub fail_streak: u32,
    /// Total tasks dispatched to this slot.
    pub tasks_dispatched: u64,
    /// Total tasks that succeeded.
    pub tasks_succeeded: u64,
}

/// Health status of a device slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SlotStatus {
    /// Ready to accept tasks.
    Healthy,
    /// Temporarily degraded but still accepting tasks.
    Degraded,
    /// Quarantined — too many consecutive failures. Skipped by dispatcher.
    Quarantined,
    /// Attestation expired — must re-attest before use.
    AttestationExpired,
}

impl DeviceSlot {
    pub fn success_rate(&self) -> f64 {
        if self.tasks_dispatched == 0 {
            1.0
        } else {
            self.tasks_succeeded as f64 / self.tasks_dispatched as f64
        }
    }
}

// ─── Dispatch Strategy ────────────────────────────────────────────────────────

/// Controls how tasks are routed across device slots.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DispatchStrategy {
    /// Weighted round-robin by TFLOPS. Default.
    WeightedRoundRobin,
    /// Always pick the slot with the lowest tasks_dispatched.
    LeastLoaded,
    /// Random uniform (for testing only).
    #[cfg(test)]
    Random,
}

// ─── Dispatch Config ──────────────────────────────────────────────────────────

/// Configuration for the multi-device dispatcher.
#[derive(Debug, Clone)]
pub struct DispatchConfig {
    /// Dispatch strategy for normal tasks.
    pub strategy: DispatchStrategy,
    /// Number of redundant executors for critical tasks (k-redundancy).
    /// Minimum redundant copies needed to agree = ⌊k/2⌋ + 1.
    pub redundancy_k: usize,
    /// Number of consecutive failures before a slot is quarantined.
    pub quarantine_threshold: u32,
    /// Attestation validity required for dispatch (ms remaining).
    /// Slots with less than this much time left are treated as expired.
    pub min_attestation_remaining_ms: u64,
}

impl Default for DispatchConfig {
    fn default() -> Self {
        Self {
            strategy: DispatchStrategy::WeightedRoundRobin,
            redundancy_k: 3,
            quarantine_threshold: 3,
            // 5 minutes — we refresh if less than this remains
            min_attestation_remaining_ms: 300_000,
        }
    }
}

// ─── Dispatch Result ──────────────────────────────────────────────────────────

/// Result of a multi-device dispatch operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DispatchResult {
    /// Chosen slot ID.
    pub slot_id: usize,
    /// Device that executed the task.
    pub device_id: u32,
    /// Backend used.
    pub backend: GpuBackendType,
    /// Whether k-redundancy was used.
    pub redundancy_used: bool,
    /// Number of redundant copies that agreed (if redundancy_used).
    pub redundancy_agreement: usize,
    /// Attestation hash of the executing device.
    pub attestation_hash: [u8; 32],
}

// ─── Multi-Device Dispatcher ─────────────────────────────────────────────────

/// Multi-GPU dispatcher with attestation, weighted routing, and k-redundancy.
pub struct MultiDeviceDispatcher {
    config: DispatchConfig,
    /// All registered device slots.
    slots: Arc<RwLock<Vec<DeviceSlot>>>,
    /// Weighted round-robin cursor (atomic for lock-free advance).
    rr_cursor: Arc<AtomicU64>,
    /// Total dispatch weight (sum of all slot weights).
    total_weight: Arc<RwLock<u32>>,
}

impl MultiDeviceDispatcher {
    pub fn new(config: DispatchConfig) -> Self {
        Self {
            config,
            slots: Arc::new(RwLock::new(Vec::new())),
            rr_cursor: Arc::new(AtomicU64::new(0)),
            total_weight: Arc::new(RwLock::new(0)),
        }
    }

    // ── Device Registration ───────────────────────────────────────────────────

    /// Register a GPU device with a simulation-mode attestation.
    /// In production: pass the real CC attestation instead.
    pub async fn register_device_sim(&self, info: GpuDeviceInfo) -> usize {
        let attestation = DeviceAttestation::from_device_info_sim(&info);
        self.register_device(info, attestation).await
    }

    /// Register a GPU device with an explicit attestation.
    pub async fn register_device(
        &self,
        info: GpuDeviceInfo,
        attestation: DeviceAttestation,
    ) -> usize {
        let mut slots = self.slots.write().await;
        let slot_id = slots.len();

        // Weight = TFLOPS × 10 rounded to u32 (gives a natural spread)
        let weight = ((info.peak_fp32_tflops * 10.0).round() as u32).max(1);

        let slot = DeviceSlot {
            slot_id,
            info,
            attestation,
            weight,
            status: SlotStatus::Healthy,
            fail_streak: 0,
            tasks_dispatched: 0,
            tasks_succeeded: 0,
        };

        info!(
            "[MultiDispatch] Registered slot {} — device {} ({:?}) {:.1} TFLOPS weight={}",
            slot_id, slot.info.device_id, slot.info.backend, slot.info.peak_fp32_tflops, weight
        );

        slots.push(slot);

        // Update total weight
        let mut tw = self.total_weight.write().await;
        *tw += weight;

        slot_id
    }

    // ── Routing ───────────────────────────────────────────────────────────────

    /// Select the best healthy slot for a normal task.
    ///
    /// Returns `None` if all slots are quarantined or have expired attestations.
    pub async fn select_slot(&self) -> Option<usize> {
        let slots = self.slots.read().await;
        let healthy: Vec<usize> = slots
            .iter()
            .enumerate()
            .filter(|(_, s)| s.status == SlotStatus::Healthy || s.status == SlotStatus::Degraded)
            .filter(|(_, s)| {
                s.attestation.remaining_ms() >= self.config.min_attestation_remaining_ms
            })
            .map(|(i, _)| i)
            .collect();

        if healthy.is_empty() {
            warn!("[MultiDispatch] No healthy slots available!");
            return None;
        }

        match self.config.strategy {
            DispatchStrategy::WeightedRoundRobin => {
                // Advance cursor and find slot whose cumulative-weight bucket contains cursor % total_weight
                let total: u32 = healthy.iter().map(|&i| slots[i].weight).sum();
                if total == 0 {
                    return healthy.first().copied();
                }
                let cursor = self.rr_cursor.fetch_add(1, Ordering::Relaxed) as u32;
                let target = cursor % total;
                let mut acc = 0u32;
                for &i in &healthy {
                    acc += slots[i].weight;
                    if acc > target {
                        return Some(i);
                    }
                }
                healthy.last().copied()
            }
            DispatchStrategy::LeastLoaded => healthy
                .into_iter()
                .min_by_key(|&i| slots[i].tasks_dispatched),
            #[cfg(test)]
            DispatchStrategy::Random => {
                use std::time::{SystemTime, UNIX_EPOCH};
                let t = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_nanos();
                Some(healthy[t as usize % healthy.len()])
            }
        }
    }

    /// Select up to `k` distinct healthy slots for redundant execution.
    pub async fn select_redundant_slots(&self, k: usize) -> Vec<usize> {
        let slots = self.slots.read().await;
        let mut healthy: Vec<usize> = slots
            .iter()
            .enumerate()
            .filter(|(_, s)| {
                (s.status == SlotStatus::Healthy || s.status == SlotStatus::Degraded)
                    && s.attestation.remaining_ms() >= self.config.min_attestation_remaining_ms
            })
            .map(|(i, _)| i)
            .collect();

        // Sort by success_rate DESC for quality-first selection
        healthy.sort_by(|&a, &b| {
            slots[b]
                .success_rate()
                .partial_cmp(&slots[a].success_rate())
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        healthy.truncate(k);
        healthy
    }

    // ── Accounting ────────────────────────────────────────────────────────────

    /// Record a successful execution on a slot.
    pub async fn record_success(&self, slot_id: usize) {
        let mut slots = self.slots.write().await;
        if let Some(slot) = slots.get_mut(slot_id) {
            slot.tasks_dispatched += 1;
            slot.tasks_succeeded += 1;
            slot.fail_streak = 0;
            if slot.status == SlotStatus::Degraded {
                slot.status = SlotStatus::Healthy;
                info!("[MultiDispatch] Slot {} recovered to Healthy", slot_id);
            }
        }
    }

    /// Record a failed execution on a slot.
    ///
    /// After `quarantine_threshold` consecutive failures, the slot is quarantined.
    pub async fn record_failure(&self, slot_id: usize) {
        let mut slots = self.slots.write().await;
        if let Some(slot) = slots.get_mut(slot_id) {
            slot.tasks_dispatched += 1;
            slot.fail_streak += 1;

            if slot.fail_streak >= self.config.quarantine_threshold {
                slot.status = SlotStatus::Quarantined;
                error!(
                    "[MultiDispatch] Slot {} QUARANTINED after {} consecutive failures",
                    slot_id, slot.fail_streak
                );
            } else {
                slot.status = SlotStatus::Degraded;
                warn!(
                    "[MultiDispatch] Slot {} degraded ({}/{} failures)",
                    slot_id, slot.fail_streak, self.config.quarantine_threshold
                );
            }
        }
    }

    // ── Attestation Management ────────────────────────────────────────────────

    /// Refresh the attestation for a slot (e.g., after TTL expiry).
    pub async fn refresh_attestation(&self, slot_id: usize, new_attestation: DeviceAttestation) {
        let mut slots = self.slots.write().await;
        if let Some(slot) = slots.get_mut(slot_id) {
            slot.attestation = new_attestation;
            // Unquarantine if it was only expired
            if slot.status == SlotStatus::AttestationExpired {
                slot.status = SlotStatus::Healthy;
                slot.fail_streak = 0;
            }
            info!("[MultiDispatch] Attestation refreshed for slot {}", slot_id);
        }
    }

    /// Scan all slots and mark those with expired attestations.
    pub async fn sweep_expired_attestations(&self) {
        let mut slots = self.slots.write().await;
        for slot in slots.iter_mut() {
            if !slot.attestation.is_valid()
                && slot.status != SlotStatus::Quarantined
                && slot.status != SlotStatus::AttestationExpired
            {
                slot.status = SlotStatus::AttestationExpired;
                warn!(
                    "[MultiDispatch] Slot {} attestation EXPIRED — device={} backend={:?}",
                    slot.slot_id, slot.info.device_id, slot.info.backend
                );
            }
        }
    }

    // ── Redundancy ────────────────────────────────────────────────────────────

    /// Check if k outputs agree (quorum = ⌊k/2⌋ + 1).
    ///
    /// Returns `(agreed, winning_output)` where `agreed` is the number of
    /// copies that returned the winning output.
    pub fn check_redundancy_agreement<T: PartialEq>(
        &self,
        outputs: &[T],
    ) -> (usize, Option<usize>) {
        if outputs.is_empty() {
            return (0, None);
        }
        let quorum = outputs.len() / 2 + 1;

        // Count occurrences of each unique output by index comparison
        let mut best_count = 0usize;
        let mut best_idx = 0usize;
        for (i, out) in outputs.iter().enumerate() {
            let count = outputs.iter().filter(|o| *o == out).count();
            if count > best_count {
                best_count = count;
                best_idx = i;
            }
        }

        if best_count >= quorum {
            (best_count, Some(best_idx))
        } else {
            (best_count, None)
        }
    }

    // ── Observability ─────────────────────────────────────────────────────────

    /// Get a snapshot of all slot states for monitoring dashboards.
    pub async fn slot_states(&self) -> Vec<SlotState> {
        let slots = self.slots.read().await;
        slots
            .iter()
            .map(|s| SlotState {
                slot_id: s.slot_id,
                device_id: s.info.device_id,
                backend: s.info.backend,
                gpu_model: s.info.name.clone(),
                tflops_fp32: s.info.peak_fp32_tflops,
                status: s.status,
                fail_streak: s.fail_streak,
                tasks_dispatched: s.tasks_dispatched,
                success_rate: s.success_rate(),
                attestation_remaining_ms: s.attestation.remaining_ms(),
                attestation_valid: s.attestation.is_valid(),
            })
            .collect()
    }

    /// Total number of registered device slots.
    pub async fn slot_count(&self) -> usize {
        self.slots.read().await.len()
    }

    /// Number of currently healthy (and not expired) slots.
    pub async fn healthy_slot_count(&self) -> usize {
        let slots = self.slots.read().await;
        slots
            .iter()
            .filter(|s| {
                (s.status == SlotStatus::Healthy || s.status == SlotStatus::Degraded)
                    && s.attestation.is_valid()
            })
            .count()
    }
}

/// Snapshot of a single device slot for monitoring.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlotState {
    pub slot_id: usize,
    pub device_id: u32,
    pub backend: GpuBackendType,
    pub gpu_model: String,
    pub tflops_fp32: f32,
    pub status: SlotStatus,
    pub fail_streak: u32,
    pub tasks_dispatched: u64,
    pub success_rate: f64,
    pub attestation_remaining_ms: u64,
    pub attestation_valid: bool,
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gpu_backends::GpuDeviceInfo;

    fn make_device(id: u32, tflops: f32) -> GpuDeviceInfo {
        GpuDeviceInfo {
            device_id: id,
            name: format!("GPU-{}", id),
            compute_capability: "8.9".to_string(),
            total_memory: 80 * 1024 * 1024 * 1024,
            available_memory: 70 * 1024 * 1024 * 1024,
            backend: GpuBackendType::CUDA,
            clock_speed_mhz: 2000,
            memory_bandwidth_gbs: 3350.0,
            peak_fp32_tflops: tflops,
            is_available: true,
        }
    }

    fn make_dispatcher() -> MultiDeviceDispatcher {
        MultiDeviceDispatcher::new(DispatchConfig::default())
    }

    #[tokio::test]
    async fn test_register_and_count() {
        let d = make_dispatcher();
        assert_eq!(d.slot_count().await, 0);

        d.register_device_sim(make_device(0, 67.0)).await;
        d.register_device_sim(make_device(1, 67.0)).await;
        d.register_device_sim(make_device(2, 67.0)).await;

        assert_eq!(d.slot_count().await, 3);
        assert_eq!(d.healthy_slot_count().await, 3);
    }

    #[tokio::test]
    async fn test_weighted_rr_selects_all_slots() {
        let d = make_dispatcher();
        d.register_device_sim(make_device(0, 10.0)).await;
        d.register_device_sim(make_device(1, 20.0)).await;
        d.register_device_sim(make_device(2, 30.0)).await;

        let mut seen = std::collections::HashSet::new();
        // With 3 slots and weights 100/200/300, total weight is 600.
        // It takes exactly 600 iterations to cycle through all possibilities once.
        for _ in 0..600 {
            if let Some(slot) = d.select_slot().await {
                seen.insert(slot);
            }
        }
        assert!(seen.contains(&0), "Slot 0 never selected");
        assert!(seen.contains(&1), "Slot 1 never selected");
        assert!(seen.contains(&2), "Slot 2 never selected");
    }

    #[tokio::test]
    async fn test_quarantine_after_threshold_failures() {
        let d = make_dispatcher();
        d.register_device_sim(make_device(0, 67.0)).await;

        // 3 consecutive failures → quarantine
        d.record_failure(0).await;
        d.record_failure(0).await;
        d.record_failure(0).await;

        let states = d.slot_states().await;
        assert_eq!(states[0].status, SlotStatus::Quarantined);

        // Quarantined slot should not be selected
        let sel = d.select_slot().await;
        assert!(sel.is_none(), "Quarantined slot must not be selected");
    }

    #[tokio::test]
    async fn test_recovery_after_success() {
        let d = make_dispatcher();
        d.register_device_sim(make_device(0, 67.0)).await;

        // 2 failures → degraded
        d.record_failure(0).await;
        d.record_failure(0).await;

        let states = d.slot_states().await;
        assert_eq!(states[0].status, SlotStatus::Degraded);

        // success → healthy
        d.record_success(0).await;
        let states = d.slot_states().await;
        assert_eq!(states[0].status, SlotStatus::Healthy);
        assert_eq!(states[0].fail_streak, 0);
    }

    #[tokio::test]
    async fn test_attestation_refresh_unquarantines_expired_slot() {
        let d = make_dispatcher();
        d.register_device_sim(make_device(0, 67.0)).await;

        // Manually expire the attestation
        {
            let mut slots = d.slots.write().await;
            slots[0].status = SlotStatus::AttestationExpired;
            slots[0].attestation.expires_at_ms = 0; // already expired
        }

        // Refresh with a new valid attestation
        let new_att = DeviceAttestation::from_device_info_sim(&make_device(0, 67.0));
        d.refresh_attestation(0, new_att).await;

        let states = d.slot_states().await;
        assert_eq!(states[0].status, SlotStatus::Healthy);
        assert!(states[0].attestation_valid);
    }

    #[tokio::test]
    async fn test_redundancy_agreement_quorum_met() {
        let d = make_dispatcher();
        // 3 copies, 2 agree → quorum met (⌊3/2⌋+1 = 2)
        let outputs = vec!["root_A", "root_A", "root_B"];
        let (agreed, winner) = d.check_redundancy_agreement(&outputs);
        assert_eq!(agreed, 2);
        assert!(winner.is_some());
    }

    #[tokio::test]
    async fn test_redundancy_agreement_no_quorum() {
        let d = make_dispatcher();
        // All different → no quorum
        let outputs = vec!["root_A", "root_B", "root_C"];
        let (agreed, winner) = d.check_redundancy_agreement(&outputs);
        assert_eq!(agreed, 1);
        assert!(winner.is_none());
    }

    #[tokio::test]
    async fn test_redundant_slots_select_up_to_k() {
        let d = make_dispatcher();
        d.register_device_sim(make_device(0, 67.0)).await;
        d.register_device_sim(make_device(1, 80.0)).await;
        d.register_device_sim(make_device(2, 67.0)).await;
        d.register_device_sim(make_device(3, 50.0)).await;

        let slots = d.select_redundant_slots(3).await;
        // Should select exactly 3 (k=3), and they must all be healthy
        assert_eq!(slots.len(), 3);
        // All must be distinct
        let unique: std::collections::HashSet<_> = slots.iter().collect();
        assert_eq!(unique.len(), 3, "Redundant slots must be distinct");
    }

    #[tokio::test]
    async fn test_attestation_is_valid_by_default() {
        let info = make_device(0, 67.0);
        let att = DeviceAttestation::from_device_info_sim(&info);
        assert!(att.is_valid());
        assert!(att.remaining_ms() > 3_599_000); // at least 1 hour - 1s
    }

    #[tokio::test]
    async fn test_no_crash_with_zero_slots() {
        let d = make_dispatcher();
        let sel = d.select_slot().await;
        assert!(sel.is_none());

        let redundant = d.select_redundant_slots(3).await;
        assert!(redundant.is_empty());
    }

    #[tokio::test]
    async fn test_success_rate_tracking() {
        let d = make_dispatcher();
        d.register_device_sim(make_device(0, 67.0)).await;

        d.record_success(0).await;
        d.record_success(0).await;
        d.record_failure(0).await;

        let states = d.slot_states().await;
        // 2 successes / 3 total = 0.666...
        assert!((states[0].success_rate - 2.0 / 3.0).abs() < 1e-6);
    }

    #[tokio::test]
    async fn test_sweep_marks_expired_attestation() {
        let d = make_dispatcher();
        d.register_device_sim(make_device(0, 67.0)).await;

        // Expire the attestation manually
        {
            let mut slots = d.slots.write().await;
            slots[0].attestation.expires_at_ms = 0;
        }

        d.sweep_expired_attestations().await;

        let states = d.slot_states().await;
        assert_eq!(states[0].status, SlotStatus::AttestationExpired);
    }

    #[tokio::test]
    async fn test_least_loaded_strategy() {
        let d = MultiDeviceDispatcher::new(DispatchConfig {
            strategy: DispatchStrategy::LeastLoaded,
            ..DispatchConfig::default()
        });

        d.register_device_sim(make_device(0, 67.0)).await;
        d.register_device_sim(make_device(1, 67.0)).await;

        // Load up slot 0 with "dispatches"
        d.record_success(0).await;
        d.record_success(0).await;
        d.record_success(0).await;

        // Next pick should be slot 1 (least loaded at 0 dispatches)
        let sel = d.select_slot().await;
        assert_eq!(sel, Some(1), "Should pick slot 1 (least loaded)");
    }
}
