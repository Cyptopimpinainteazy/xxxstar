//! Tuning Module - Runtime tuning and optimization

use crate::RuntimeParameters;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

/// Tuning profile
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TuningProfile {
    /// Default balanced profile
    Default,
    /// High throughput profile
    HighThroughput,
    /// Low latency profile
    LowLatency,
    /// Archival profile
    Archival,
}

/// Runtime tuner for dynamic parameter adjustment
pub struct RuntimeTuner {
    profile: AtomicU64,
    auto_tune_enabled: AtomicBool,
    params: std::sync::Arc<parking_lot::RwLock<RuntimeParameters>>,
    // Tuning thresholds
    target_tps: AtomicU64,
    target_latency_ms: AtomicU64,
}

impl RuntimeTuner {
    /// Create new tuner
    pub fn new(params: RuntimeParameters) -> Self {
        Self {
            profile: AtomicU64::new(TuningProfile::Default as u64),
            auto_tune_enabled: AtomicBool::new(false),
            params: std::sync::Arc::new(parking_lot::RwLock::new(params)),
            target_tps: AtomicU64::new(65000),
            target_latency_ms: AtomicU64::new(1000),
        }
    }

    /// Apply tuning parameters
    pub fn apply_tuning(&self, params: &RuntimeParameters) {
        *self.params.write() = params.clone();
    }

    /// Set tuning profile
    pub fn set_profile(&self, profile: TuningProfile) {
        self.profile.store(profile as u64, Ordering::SeqCst);
        
        let params = match profile {
            TuningProfile::Default => RuntimeParameters::default(),
            TuningProfile::HighThroughput => RuntimeParameters::high_throughput(),
            TuningProfile::LowLatency => RuntimeParameters::low_latency(),
            TuningProfile::Archival => RuntimeParameters::archival(),
        };
        
        self.apply_tuning(&params);
    }

    /// Enable auto-tuning
    pub fn enable_auto_tune(&self) {
        self.auto_tune_enabled.store(true, Ordering::SeqCst);
    }

    /// Disable auto-tuning
    pub fn disable_auto_tune(&self) {
        self.auto_tune_enabled.store(false, Ordering::SeqCst);
    }

    /// Check if auto-tuning is enabled
    pub fn is_auto_tune_enabled(&self) -> bool {
        self.auto_tune_enabled.load(Ordering::SeqCst)
    }

    /// Get current profile
    pub fn get_profile(&self) -> TuningProfile {
        match self.profile.load(Ordering::SeqCst) {
            0 => TuningProfile::Default,
            1 => TuningProfile::HighThroughput,
            2 => TuningProfile::LowLatency,
            3 => TuningProfile::Archival,
            _ => TuningProfile::Default,
        }
    }

    /// Set target TPS
    pub fn set_target_tps(&self, tps: u64) {
        self.target_tps.store(tps, Ordering::SeqCst);
    }

    /// Set target latency
    pub fn set_target_latency(&self, latency_ms: u64) {
        self.target_latency_ms.store(latency_ms, Ordering::SeqCst);
    }

    /// Get current parameters
    pub fn get_params(&self) -> RuntimeParameters {
        self.params.read().clone()
    }
}