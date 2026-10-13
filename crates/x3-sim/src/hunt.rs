//! Multi-seed hunting.
//!
//! One seed proves one schedule; hunting runs a range and keeps one failure
//! packet per distinct failure. The loop lives here rather than in the CLI so
//! its stopping rule — stop once `max_failures` packets exist, and never run
//! another seed after that — is covered by a test with a synthetic runner
//! instead of only by a real hunt that has to find a real violation first.

use crate::minimize::{minimize, DEFAULT_MAX_RUNS};
use crate::packet::{violation_signature, FailurePacket};
use crate::sim::{SimConfig, SimOutcome};

/// How many duplicate runs keep their full outcome and packet as evidence.
///
/// A systematic defect can be found by every remaining seed in a large hunt;
/// retaining all of them would make memory grow with the hunt size while the
/// thousandth copy adds nothing a sample does not. `duplicates_seen` still
/// counts every rediscovery.
pub const MAX_DUPLICATE_SAMPLES: usize = 32;

/// One failure, with the packet and the config that produced it.
#[derive(Debug, Clone)]
pub struct HuntFailure {
    pub config: SimConfig,
    pub outcome: SimOutcome,
    pub packet: FailurePacket,
}

/// The result of a hunt, whether or not it found anything.
#[derive(Debug, Clone)]
pub struct HuntReport {
    pub seeds_requested: usize,
    pub seeds_run: usize,
    pub passes: usize,
    pub failures: Vec<HuntFailure>,
    /// Failing seeds whose violation was already collected under an earlier
    /// seed: the same defect, and each run is evidence of it, so the first
    /// `MAX_DUPLICATE_SAMPLES` are kept whole (outcome + packet). They do not
    /// consume `--max-failures` and get no second packet.
    pub duplicates: Vec<HuntFailure>,
    /// Every rediscovery, including the ones beyond the retained samples.
    pub duplicates_seen: usize,
    /// True when the hunt stopped because `max_failures` was reached rather
    /// than because the seed range was exhausted.
    pub stopped_at_max_failures: bool,
}

impl HuntReport {
    pub fn is_clean(&self) -> bool {
        self.failures.is_empty()
    }
}

/// Run `count` consecutive seeds from `base`, keeping at most `max_failures`
/// distinct failures. When `minimize_failures` is set, each kept failure is
/// minimized and verified before the packet is returned.
pub fn hunt<F>(
    base: &SimConfig,
    count: usize,
    max_failures: usize,
    minimize_failures: bool,
    mut runner: F,
) -> HuntReport
where
    F: FnMut(&SimConfig) -> SimOutcome,
{
    let mut report = HuntReport {
        seeds_requested: count,
        seeds_run: 0,
        passes: 0,
        failures: Vec::new(),
        duplicates: Vec::new(),
        duplicates_seen: 0,
        stopped_at_max_failures: false,
    };

    for offset in 0..count {
        if report.failures.len() >= max_failures {
            report.stopped_at_max_failures = true;
            break;
        }
        let mut config = base.clone();
        config.seed = base.seed.wrapping_add(offset as u64);
        report.seeds_run += 1;
        let outcome = runner(&config);
        if outcome.is_pass() {
            report.passes += 1;
            continue;
        }
        // `--max-failures` counts distinct defects, not seeds: the same
        // violation found again must not consume the budget (or stop the
        // hunt) before a different one is reached. The signature is computed
        // from the outcome, before any packet exists, so a duplicate beyond
        // the sample cap costs a comparison — not a packet (whose build runs
        // `git rev-parse`).
        let signature = violation_signature(&outcome.violations);
        if report
            .failures
            .iter()
            .any(|failure| failure.packet.violation_signature() == signature)
        {
            report.duplicates_seen += 1;
            if report.duplicates.len() < MAX_DUPLICATE_SAMPLES {
                if let Some(packet) = FailurePacket::from_outcome(&outcome) {
                    report.duplicates.push(HuntFailure {
                        config: config.clone(),
                        outcome,
                        packet,
                    });
                }
            }
            continue;
        }
        let mut packet = match FailurePacket::from_outcome(&outcome) {
            Some(packet) => packet,
            None => continue,
        };
        if minimize_failures {
            let code = packet.invariant.clone();
            if let Some(minimized) = minimize(&config, Some(&code), DEFAULT_MAX_RUNS, |candidate| {
                runner(candidate)
            }) {
                packet.attach_minimized(minimized.to_reproducer());
            }
        }
        report.failures.push(HuntFailure {
            config,
            outcome,
            packet,
        });
    }

    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::invariants::Violation;
    use crate::network::NetworkStats;
    use crate::sim::Scenario;

    fn outcome_for(config: &SimConfig, failing: bool) -> SimOutcome {
        let mut outcome = SimOutcome {
            seed: config.seed,
            scenario: config.scenario.as_str().to_string(),
            sessions: config.sessions,
            steps: config.steps,
            nodes: config.nodes,
            accepted: 1,
            rejected: 0,
            restarts: 0,
            stale_writes: 0,
            partitions: 0,
            sessions_completed: 0,
            sessions_refunded: 0,
            network: NetworkStats::default(),
            trace_digest: "00".to_string(),
            state_digest: "00".to_string(),
            first_bad_step: Some(0),
            first_bad_step_label: "step 0".to_string(),
            first_bad_op: Some("refund".to_string()),
            active_faults: Vec::new(),
            state_before: None,
            state_after: None,
            violations: Vec::new(),
            trace: Vec::new(),
        };
        if failing {
            outcome.violations.push(Violation {
                code: "CLAIM_REFUND_MIX",
                session_id: "s".to_string(),
                // Seed-specific, so each failing seed is a distinct defect
                // unless a test deliberately reuses a signature.
                detail: format!("detail seed={}", config.seed),
            });
        }
        outcome
    }

    #[test]
    fn hunting_stops_at_max_failures_and_counts_what_it_ran() {
        let base = SimConfig {
            seed: 10,
            scenario: Scenario::ClaimRefundRace,
            sessions: 4,
            steps: 40,
            nodes: 3,
        };
        // Every third seed fails.
        let report = hunt(&base, 30, 2, false, |config| {
            outcome_for(config, config.seed % 3 == 0)
        });
        assert!(report.stopped_at_max_failures, "the limit stops the hunt");
        assert_eq!(report.failures.len(), 2);
        assert_eq!(
            report.seeds_run, 6,
            "seeds 10,11,12,13,14,15: two failures then stop"
        );
        assert_eq!(report.passes, 4);
    }

    /// A failure whose signature does not depend on the seed: the same
    /// defect, rediscovered.
    fn outcome_with_fixed_violation(config: &SimConfig) -> SimOutcome {
        let mut outcome = outcome_for(config, false);
        outcome.violations.push(Violation {
            code: "CLAIM_REFUND_MIX",
            session_id: "s".to_string(),
            detail: "identical detail".to_string(),
        });
        outcome
    }

    #[test]
    fn repeated_failures_are_one_defect_and_do_not_consume_max_failures() {
        let base = SimConfig {
            seed: 0,
            scenario: Scenario::ClaimRefundRace,
            sessions: 3,
            steps: 20,
            nodes: 3,
        };
        let report = hunt(&base, 6, 2, false, outcome_with_fixed_violation);
        assert_eq!(
            report.failures.len(),
            1,
            "one defect, however many seeds found it"
        );
        assert_eq!(
            report.duplicates.len(),
            5,
            "each rediscovery is kept as evidence"
        );
        assert_eq!(
            report.seeds_run, 6,
            "the hunt keeps looking past the duplicate"
        );
        assert!(!report.stopped_at_max_failures);
    }

    #[test]
    fn seed_derived_session_ids_do_not_defeat_deduplication() {
        let base = SimConfig {
            seed: 0,
            scenario: Scenario::ClaimRefundRace,
            sessions: 3,
            steps: 20,
            nodes: 3,
        };
        let report = hunt(&base, 5, 2, false, |config| {
            let mut outcome = outcome_for(config, false);
            // The real simulator derives session ids from a seed-derived
            // secret, so the same defect never carries the same id twice.
            outcome.violations.push(Violation {
                code: "REFUND_AFTER_CLAIM",
                session_id: format!("sim-{:016x}", config.seed),
                detail: "journal records a claim and then a refund".to_string(),
            });
            outcome
        });
        assert_eq!(report.failures.len(), 1, "one defect, not one per seed");
        assert_eq!(report.duplicates.len(), 4);
    }

    #[test]
    fn duplicate_samples_are_bounded_but_every_rediscovery_is_counted() {
        let base = SimConfig {
            seed: 0,
            scenario: Scenario::ClaimRefundRace,
            sessions: 3,
            steps: 20,
            nodes: 3,
        };
        let count = MAX_DUPLICATE_SAMPLES + 11;
        let report = hunt(&base, count, 2, false, outcome_with_fixed_violation);
        assert_eq!(report.failures.len(), 1, "still one defect");
        assert_eq!(
            report.duplicates_seen,
            count - 1,
            "a systematic defect cannot be hidden by the sample cap"
        );
        assert_eq!(
            report.duplicates.len(),
            MAX_DUPLICATE_SAMPLES,
            "memory stays bounded on a long hunt"
        );
        assert_eq!(report.seeds_run, count);
    }

    #[test]
    fn a_clean_hunt_runs_the_whole_range() {
        let base = SimConfig {
            seed: 0,
            scenario: Scenario::HappyPath,
            sessions: 2,
            steps: 10,
            nodes: 3,
        };
        let report = hunt(&base, 7, 3, true, |config| outcome_for(config, false));
        assert!(report.is_clean());
        assert_eq!(report.seeds_run, 7);
        assert!(report.duplicates.is_empty());
        assert!(!report.stopped_at_max_failures);
    }
}
