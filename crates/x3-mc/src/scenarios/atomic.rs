//! `MC-X3-ATOMIC-001` — atomic settlement under randomized conditions.
//!
//! This campaign does not model the coordinator. Every state transition in a
//! run comes from `x3_cross_vm_coordinator::SwapCoordinator`, through the same
//! `x3_sim::run_with` entry point the deterministic fixtures use. Monte Carlo
//! chooses where in the parameter space to stand; the simulator executes; the
//! coordinator supplies the semantics.
//!
//! # What this campaign varies, and what it does not
//!
//! The simulator's seams are the network (latency, jitter, loss), the fault
//! plan (partitions, crashes, lost writes) and the clock. Those are what the
//! campaign moves. The source document also lists proof timing, proof
//! freshness, per-VM execution latency, timeouts and RPC failures as axes;
//! `x3-sim` v0 has no seam for them yet, so they are **not** covered here and
//! are not claimed to be. Adding them means adding seams to the simulator, not
//! renaming a variable in this file.

use std::collections::BTreeMap;

use serde_json::json;
use x3_sim::{run_with, Scenario, SimConfig, SimParams};

use crate::engine::{
    as_bool, as_u32, as_u64, as_usize, MonteCarloScenario, ParamSet, RunOutcome, ViolationReport,
};

/// The lifecycle under test is the coordinator's; the fault shape is the
/// simulator's.
const BASE_SCENARIO: Scenario = Scenario::ClaimRefundRace;

#[derive(Debug, Clone)]
pub struct AtomicSettlement {
    /// Which x3-sim scenario shape the samples are laid over.
    pub base: Scenario,
}

impl Default for AtomicSettlement {
    fn default() -> Self {
        Self { base: BASE_SCENARIO }
    }
}

impl AtomicSettlement {
    pub fn new() -> Self {
        Self::default()
    }

    /// Turn one sampled point into a simulator run.
    pub fn params_for(&self, sample: &ParamSet) -> (SimConfig, SimParams) {
        let config = SimConfig {
            seed: as_u64(sample, "sim_seed", 0),
            scenario: self.base,
            sessions: as_usize(sample, "sessions", 8).clamp(1, 64),
            steps: as_usize(sample, "steps", 120).clamp(10, 600),
            nodes: as_usize(sample, "nodes", 4).clamp(2, 12),
        };
        let defaults = SimParams::for_scenario(self.base);
        let params = SimParams {
            latency_ms: as_u64(sample, "latency_ms", defaults.latency_ms).min(3_000),
            jitter_ms: as_u64(sample, "jitter_ms", defaults.jitter_ms).min(2_000),
            drop_percent: as_u32(sample, "drop_percent", defaults.drop_percent).min(60),
            duplicate_percent: as_u32(sample, "duplicate_percent", defaults.duplicate_percent)
                .min(100),
            clock_step_ms: as_u64(sample, "clock_step_ms", defaults.clock_step_ms).clamp(1, 10_000),
            clock_jitter_ms: as_u64(sample, "clock_jitter_ms", defaults.clock_jitter_ms)
                .min(10_000),
            partition_events: as_usize(sample, "partition_events", defaults.partition_events)
                .min(8),
            slow_link_events: as_usize(sample, "slow_link_events", defaults.slow_link_events)
                .min(4),
            crash_events: as_usize(sample, "crash_events", defaults.crash_events).min(6),
            crash_duration_ms: as_u64(sample, "crash_duration_ms", defaults.crash_duration_ms)
                .min(30_000),
            stale_write_events: as_usize(
                sample,
                "stale_write_events",
                defaults.stale_write_events,
            )
            .min(6),
        };
        (config, params)
    }
}

impl MonteCarloScenario for AtomicSettlement {
    fn name(&self) -> &str {
        "MC-X3-ATOMIC-001"
    }

    fn execute(&self, sample: &ParamSet, seed: u64) -> RunOutcome {
        // `sim_seed` is part of the sampled point rather than taken from the run
        // seed, so a minimizer shrinking the scenario cannot change the
        // simulator's own schedule at the same time and call the result a
        // smaller reproducer.
        let mut point = sample.clone();
        point.insert("sim_seed".to_string(), seed as f64);
        let (config, params) = self.params_for(&point);
        let outcome = run_with(&config, &params);

        let mut observations = BTreeMap::new();
        let counts = [
            ("accepted", outcome.accepted),
            ("rejected", outcome.rejected),
            ("restarts", outcome.restarts),
            ("stale_writes", outcome.stale_writes),
            ("partitions", outcome.partitions),
            ("sessions_completed", outcome.sessions_completed as u64),
            ("sessions_refunded", outcome.sessions_refunded as u64),
            ("dropped_messages", outcome.network.dropped),
            ("delivered_messages", outcome.network.delivered),
        ];
        for (key, value) in counts {
            observations.insert(key.to_string(), value as f64);
        }
        let attempted = outcome.accepted + outcome.rejected;
        observations.insert(
            "rejection_rate".to_string(),
            if attempted == 0 {
                0.0
            } else {
                outcome.rejected as f64 / attempted as f64
            },
        );

        let replay = format!(
            "cargo run -p x3-sim -- --seed {} --scenario {} --sessions {} --steps {} --nodes {}",
            config.seed,
            config.scenario.as_str(),
            config.sessions,
            config.steps,
            config.nodes
        );
        let sim = serde_json::to_value(&outcome).unwrap_or(serde_json::Value::Null);
        let shape = json!({
            "scenario": config.scenario.as_str(),
            "sessions": config.sessions,
            "steps": config.steps,
            "nodes": config.nodes,
        });
        let evidence = json!({
            "sim": sim,
            "sim_params": params.summary(),
            "run": shape,
            "replay": replay,
        });

        RunOutcome {
            observations,
            evidence,
        }
    }

    fn invariants(&self, sample: &ParamSet, outcome: &RunOutcome) -> Vec<ViolationReport> {
        let mut found: Vec<ViolationReport> = Vec::new();

        // The coordinator's own checker. These properties are the ones the
        // simulator's fixtures prove fire on a hand-built broken state, so a
        // finding here is a real transition rather than a heuristic.
        let sim = &outcome.evidence["sim"];
        if let Some(violations) = sim.get("violations").and_then(|value| value.as_array()) {
            for violation in violations {
                let text = |key: &str| {
                    violation
                        .get(key)
                        .and_then(|value| value.as_str())
                        .unwrap_or("")
                };
                found.push(ViolationReport::new(
                    text("code"),
                    format!("session {}: {}", text("session_id"), text("detail")),
                ));
            }
        }

        let accepted = outcome.observations.get("accepted").copied().unwrap_or(0.0);
        let delivered = outcome
            .observations
            .get("delivered_messages")
            .copied()
            .unwrap_or(0.0);
        let stale = outcome
            .observations
            .get("stale_writes")
            .copied()
            .unwrap_or(0.0);

        // A campaign that stopped reaching the coordinator would otherwise look
        // like a clean sweep. A run that accepted nothing while traffic was
        // delivered did not test the lifecycle.
        if accepted == 0.0 && delivered > 0.0 {
            found.push(ViolationReport::new(
                "NO_PROGRESS_BUT_TRAFFIC",
                format!("{delivered} messages delivered and the coordinator accepted none"),
            ));
        }

        // A stale-write fault is a real persisted write disappearing. If it
        // fired in a run where nothing else happened, the parameters describe a
        // weaker scenario than they claim.
        if stale > 0.0 && accepted == 0.0 && delivered == 0.0 {
            found.push(ViolationReport::new(
                "FAULT_WITHOUT_EFFECT",
                "a stale-write fault fired in a run where nothing else happened",
            ));
        }

        // Determinism, sampled rather than assumed. The same point must produce
        // the same state digest; if it does not, nothing this harness reports
        // can be replayed and every other finding is suspect.
        if as_bool(sample, "verify_determinism", false) {
            // The seed actually used is read back from the evidence. Rebuilding
            // it from `sample` would omit `sim_seed` and compare two different
            // runs, which would report a determinism failure that is really a
            // bug in this check.
            let used = sim.get("seed").and_then(|value| value.as_u64()).unwrap_or(0);
            let mut point = sample.clone();
            point.insert("sim_seed".to_string(), used as f64);
            let (config, params) = self.params_for(&point);
            let replay = run_with(&config, &params);
            let first = sim
                .get("state_digest")
                .and_then(|value| value.as_str())
                .unwrap_or_default();
            if first != replay.state_digest {
                found.push(ViolationReport::new(
                    "NONDETERMINISTIC_REPLAY",
                    format!(
                        "state digest changed between identical runs: {first} vs {}",
                        replay.state_digest
                    ),
                ));
            }
        }

        found
    }
}
