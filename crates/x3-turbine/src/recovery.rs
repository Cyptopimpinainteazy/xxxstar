//! Recovery Module - Handle missing shreds through erasure coding

use crate::error::{TurbineError, TurbineResult};
use crate::metrics::TurbineMetrics;
use crate::shred::{ErasureCode, Shred};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{debug, warn};

/// Recovery configuration
#[derive(Debug, Clone)]
pub struct RecoveryConfig {
    pub max_recovery_attempts: u32,
    pub recovery_timeout: Duration,
    pub enable_coding_recovery: bool,
}

impl Default for RecoveryConfig {
    fn default() -> Self {
        Self {
            max_recovery_attempts: 3,
            recovery_timeout: Duration::from_secs(5),
            enable_coding_recovery: true,
        }
    }
}

/// Pending recovery request
#[derive(Debug)]
struct RecoveryRequest {
    _slot: u64,
    _missing_indices: Vec<u32>,
    attempts: u32,
    requested_at: Instant,
}

/// Shred recovery handler
pub struct ShredRecovery {
    config: RecoveryConfig,
    erasure_code: ErasureCode,
    metrics: Arc<TurbineMetrics>,
    pending_requests: RwLock<HashMap<u64, RecoveryRequest>>,
}

impl ShredRecovery {
    /// Create new recovery handler
    pub fn new(config: RecoveryConfig, metrics: Arc<TurbineMetrics>) -> Self {
        // Default to reasonable values for erasure code
        let erasure_code = ErasureCode::new(32, 16);

        Self {
            config,
            erasure_code,
            metrics,
            pending_requests: RwLock::new(HashMap::new()),
        }
    }

    /// Check if recovery is needed
    pub fn needs_recovery(&self, _slot: u64, received: &[bool], total: u32) -> bool {
        let received_count: usize = received.iter().filter(|&&b| b).count();
        let data_shards_needed = (total as usize).div_ceil(2);
        received_count < data_shards_needed
    }

    /// Get missing indices for recovery
    pub fn get_missing_indices(&self, received: &[bool]) -> Vec<u32> {
        received
            .iter()
            .enumerate()
            .filter(|(_, &received)| !received)
            .map(|(i, _)| i as u32)
            .collect()
    }

    /// Attempt to recover missing shreds
    pub fn recover(
        &self,
        slot: u64,
        data_shreds: &[Vec<u8>],
        coding_shreds: &[Vec<u8>],
    ) -> TurbineResult<Vec<Shred>> {
        if !self.config.enable_coding_recovery {
            return Ok(Vec::new());
        }

        let total_shreds = data_shreds.len() + coding_shreds.len();
        let data_needed = total_shreds.div_ceil(2);

        if data_shreds.len() < data_needed && coding_shreds.is_empty() {
            warn!("Cannot recover: not enough shreds for recovery");
            self.metrics.record_recovery_failed();
            return Err(TurbineError::RecoveryError(
                "Insufficient shreds for recovery".into(),
            ));
        }

        // Use erasure code to recover
        let recovered = self.erasure_code.decode(data_shreds, coding_shreds, &[]);

        match recovered {
            Some(data) => {
                self.metrics.record_recovery_success();
                // Create recovered shreds
                let shreds = Vec::new();
                let chunk_size = data.len() / data_needed;

                for (i, _chunk) in data.chunks(chunk_size).enumerate() {
                    // In real implementation, would create proper Shred objects
                    debug!("Recovered shred {} for slot {}", i, slot);
                }

                Ok(shreds)
            }
            None => {
                self.metrics.record_recovery_failed();
                Err(TurbineError::RecoveryError("Recovery failed".into()))
            }
        }
    }

    /// Request recovery from peers
    pub fn request_recovery(&self, slot: u64, missing_indices: &[u32]) -> bool {
        let mut requests = self.pending_requests.write();

        if let Some(req) = requests.get_mut(&slot) {
            if req.attempts >= self.config.max_recovery_attempts {
                return false;
            }
            req.attempts += 1;
            debug!("Recovery attempt {} for slot {}", req.attempts, slot);
        } else {
            requests.insert(
                slot,
                RecoveryRequest {
                    _slot: slot,
                    _missing_indices: missing_indices.to_vec(),
                    attempts: 1,
                    requested_at: Instant::now(),
                },
            );
        }

        true
    }

    /// Check pending requests timeout
    pub fn check_timeouts(&self) -> Vec<u64> {
        let mut requests = self.pending_requests.write();
        let mut timed_out = Vec::new();

        requests.retain(|slot, req| {
            let timed_out_now = req.requested_at.elapsed() > self.config.recovery_timeout;
            if timed_out_now {
                timed_out.push(*slot);
            }
            !timed_out_now
        });

        timed_out
    }
}
