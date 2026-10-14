//! Tunable run parameters.
//!
//! The scenarios in [`crate::Scenario`] fix one point in the parameter space:
//! `PartitionStorm` always cuts the link four times, `CrashRecovery` always
//! crashes twice, and message loss is a constant. That is what you want from a
//! regression fixture and it is the wrong shape for a search, where the whole
//! question is what happens at packet loss 3% versus 27%.
//!
//! `SimParams` is the seam. [`SimParams::for_scenario`] returns exactly the
//! values that were previously hardcoded, so `run()` behaves identically and
//! every existing fixture stays valid; a caller that wants to explore passes
//! any other point in the space to [`crate::sim::run_with`].

use crate::Scenario;

/// One point in the simulator's parameter space.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimParams {
    /// One-way link latency before jitter.
    pub latency_ms: u64,
    /// Half-width of the uniform jitter added to each link.
    pub jitter_ms: u64,
    /// Baseline message drop rate, percent.
    pub drop_percent: u32,
    /// Chance a prepared intent is delivered a second time, percent.
    pub duplicate_percent: u32,
    /// Base clock advance per scheduler step.
    pub clock_step_ms: u64,
    /// Extra uniform advance per step, so steps are not evenly spaced.
    pub clock_jitter_ms: u64,
    /// How many partitions the fault plan schedules.
    pub partition_events: usize,
    /// How many slow-link degradations the fault plan schedules.
    pub slow_link_events: usize,
    /// How many crash-and-restart events the fault plan schedules.
    pub crash_events: usize,
    /// Duration of the first crash; later crashes are shorter.
    pub crash_duration_ms: u64,
    /// How many persisted writes the fault plan makes disappear.
    pub stale_write_events: usize,
}

impl SimParams {
    /// The values each scenario used before they were parameters.
    ///
    /// Changing anything here changes what the existing fixtures exercise, so
    /// it is deliberately the least interesting point in the space.
    pub fn for_scenario(scenario: Scenario) -> Self {
        match scenario {
            Scenario::HappyPath => Self {
                latency_ms: 25,
                jitter_ms: 0,
                drop_percent: 0,
                duplicate_percent: 0,
                clock_step_ms: 250,
                clock_jitter_ms: 500,
                partition_events: 0,
                slow_link_events: 0,
                crash_events: 0,
                crash_duration_ms: 4_000,
                stale_write_events: 0,
            },
            Scenario::ClaimRefundRace => Self {
                latency_ms: 25,
                jitter_ms: 40,
                drop_percent: 2,
                duplicate_percent: 15,
                clock_step_ms: 250,
                clock_jitter_ms: 500,
                partition_events: 1,
                slow_link_events: 0,
                crash_events: 0,
                crash_duration_ms: 4_000,
                stale_write_events: 0,
            },
            Scenario::PartitionStorm => Self {
                latency_ms: 25,
                jitter_ms: 40,
                drop_percent: 2,
                duplicate_percent: 25,
                clock_step_ms: 250,
                clock_jitter_ms: 500,
                partition_events: 4,
                slow_link_events: 2,
                crash_events: 0,
                crash_duration_ms: 4_000,
                stale_write_events: 0,
            },
            Scenario::CrashRecovery => Self {
                latency_ms: 25,
                jitter_ms: 40,
                drop_percent: 2,
                duplicate_percent: 20,
                clock_step_ms: 250,
                clock_jitter_ms: 500,
                partition_events: 0,
                slow_link_events: 0,
                crash_events: 2,
                crash_duration_ms: 4_000,
                stale_write_events: 1,
            },
        }
    }

    /// A description short enough for a log line and complete enough to read
    /// a failure by.
    pub fn summary(&self) -> String {
        format!(
            "lat={}ms jit={}ms drop={}% dup={}% step={}+{}ms parts={} slow={} crashes={}@{}ms stale={}",
            self.latency_ms,
            self.jitter_ms,
            self.drop_percent,
            self.duplicate_percent,
            self.clock_step_ms,
            self.clock_jitter_ms,
            self.partition_events,
            self.slow_link_events,
            self.crash_events,
            self.crash_duration_ms,
            self.stale_write_events,
        )
    }
}

impl Default for SimParams {
    fn default() -> Self {
        Self::for_scenario(Scenario::HappyPath)
    }
}
