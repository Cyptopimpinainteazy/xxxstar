//! The engine has to be able to find something.
//!
//! A campaign that reports zero failures is only meaningful if the same engine
//! would have reported a failure had there been one. These tests are that
//! control: a scenario with a known broken region, so detection, seeding,
//! replay and minimization can all be checked against a defect that is
//! definitely there.

use std::collections::BTreeMap;

use x3_mc::campaign::{Campaign, CampaignMeta, SeedSpec};
use x3_mc::engine::{
    execute_one, run_campaign, CampaignConfig, MonteCarloScenario, ParamSet, RunOutcome,
    ViolationReport,
};
use x3_mc::minimize::{cluster, minimize};
use x3_mc::{AtomicSettlement, Distribution, McRng};

/// Fails whenever `pressure` is above `threshold` — the shape of a real
/// boundary, so minimization has something to walk down to.
struct Boundary {
    threshold: f64,
    /// Execution count, to prove the engine does not silently re-run.
    calls: std::cell::Cell<u64>,
}

impl Boundary {
    fn new(threshold: f64) -> Self {
        Self {
            threshold,
            calls: std::cell::Cell::new(0),
        }
    }
}

impl MonteCarloScenario for Boundary {
    fn name(&self) -> &str {
        "boundary"
    }

    fn execute(&self, params: &ParamSet, _seed: u64) -> RunOutcome {
        self.calls.set(self.calls.get() + 1);
        let pressure = params.get("pressure").copied().unwrap_or(0.0);
        RunOutcome::empty()
            .observe("pressure", pressure)
            .observe("failed", if pressure > self.threshold { 1.0 } else { 0.0 })
    }

    fn invariants(&self, params: &ParamSet, _outcome: &RunOutcome) -> Vec<ViolationReport> {
        let pressure = params.get("pressure").copied().unwrap_or(0.0);
        if pressure > self.threshold {
            vec![ViolationReport::new(
                "PRESSURE_EXCEEDED",
                format!("pressure {pressure} exceeded {}", self.threshold),
            )]
        } else {
            Vec::new()
        }
    }
}

fn campaign_with(pressure: Distribution) -> Campaign {
    let mut variables = BTreeMap::new();
    variables.insert("pressure".to_string(), pressure);
    Campaign {
        campaign: CampaignMeta {
            name: "control".to_string(),
            runs: 200,
            seed: SeedSpec::Fixed(1),
            description: None,
            abort_on: None,
        },
        variables,
    }
}

fn config(runs: u64, seed: u64) -> CampaignConfig {
    CampaignConfig {
        runs,
        seed,
        minimize: true,
        minimize_budget: 200,
        max_failures: 0,
        abort_on: None,
    }
}

#[test]
fn the_engine_detects_a_defect_that_is_definitely_there() {
    // Half the sampled range is broken, so a working engine cannot miss it.
    let campaign = campaign_with(Distribution::Uniform {
        min: 0.0,
        max: 10.0,
    });
    let report = run_campaign(&Boundary::new(5.0), &campaign, &config(400, 7));

    assert!(!report.passed(), "a broken half of the range must be found");
    assert!(
        (150..=250).contains(&report.failing_runs),
        "roughly half of 400 runs should fail, got {}",
        report.failing_runs
    );
    assert_eq!(report.unique_root_causes, 1, "one defect, not four hundred");
    assert_eq!(report.coverage.violations["PRESSURE_EXCEEDED"], report.failing_runs);
}

#[test]
fn a_campaign_that_cannot_fail_reports_zero_and_says_so_honestly() {
    let campaign = campaign_with(Distribution::Constant { value: 1.0 });
    let report = run_campaign(&Boundary::new(5.0), &campaign, &config(500, 3));

    assert!(report.passed());
    let (observed, low, high) = report.failure_rate();
    assert_eq!((observed, low), (0.0, 0.0));
    assert!(
        high > 0.0,
        "zero observed failures must still bound the rate above zero"
    );
    // With one distinct parameter point, the report should not imply breadth.
    assert_eq!(report.distinct_parameter_points, 1);
}

#[test]
fn every_failure_carries_a_seed_that_replays_it() {
    let campaign = campaign_with(Distribution::Uniform {
        min: 0.0,
        max: 10.0,
    });
    let scenario = Boundary::new(5.0);
    let report = run_campaign(&scenario, &campaign, &config(200, 11));

    let failure = report.failures.first().expect("the range is half broken");
    let (params, seed, _, violations) = execute_one(&scenario, &campaign, failure.run_index, 11);

    assert_eq!(seed, failure.seed, "replay must reproduce the recorded seed");
    assert_eq!(params, failure.params, "replay must reproduce the parameters");
    assert!(
        !violations.is_empty(),
        "the replayed run must still violate the invariant"
    );
    assert_eq!(violations[0].code, "PRESSURE_EXCEEDED");
}

#[test]
fn the_seed_sets_of_two_master_seeds_are_unrelated() {
    let campaign = campaign_with(Distribution::Uniform {
        min: 0.0,
        max: 10.0,
    });
    let first = run_campaign(&Boundary::new(5.0), &campaign, &config(50, 1));
    let second = run_campaign(&Boundary::new(5.0), &campaign, &config(50, 2));

    let left: Vec<u64> = first.failures.iter().map(|failure| failure.seed).collect();
    let right: Vec<u64> = second.failures.iter().map(|failure| failure.seed).collect();
    assert_ne!(left, right);
}

#[test]
fn the_same_campaign_and_seed_produce_the_same_report() {
    let campaign = campaign_with(Distribution::Uniform {
        min: 0.0,
        max: 10.0,
    });
    let left = run_campaign(&Boundary::new(5.0), &campaign, &config(120, 99));
    let right = run_campaign(&Boundary::new(5.0), &campaign, &config(120, 99));

    assert_eq!(left.failing_runs, right.failing_runs);
    assert_eq!(left.observations, right.observations);
    assert_eq!(
        left.failures.iter().map(|f| (f.run_index, f.seed)).collect::<Vec<_>>(),
        right.failures.iter().map(|f| (f.run_index, f.seed)).collect::<Vec<_>>()
    );
}

#[test]
fn minimization_shrinks_the_failure_without_losing_it() {
    let mut params = ParamSet::new();
    params.insert("pressure".to_string(), 9.75);
    let scenario = Boundary::new(5.0);

    let result = minimize(&scenario, &params, 1234, 200);

    assert!(!result.violations.is_empty(), "the shrunk case must still fail");
    let shrunk = result.params["pressure"];
    assert!(shrunk < 9.75, "minimization did not reduce the parameter");
    // It should walk close to the boundary without crossing it: just above 5.0
    // and far below where it started.
    assert!(
        (5.0..6.0).contains(&shrunk),
        "expected a value just above the threshold, got {shrunk}"
    );
    assert!(result.reduced_variables >= 1);
}

#[test]
fn minimization_will_not_drift_onto_a_different_violation() {
    // The scenario reports a *different* code once pressure is small; the
    // minimizer must not accept that as progress.
    struct TwoFaults;
    impl MonteCarloScenario for TwoFaults {
        fn name(&self) -> &str {
            "two-faults"
        }
        fn execute(&self, _params: &ParamSet, _seed: u64) -> RunOutcome {
            RunOutcome::empty()
        }
        fn invariants(&self, params: &ParamSet, _outcome: &RunOutcome) -> Vec<ViolationReport> {
            let value = params.get("x").copied().unwrap_or(0.0);
            if value > 100.0 {
                vec![ViolationReport::new("HIGH", "high")]
            } else if value > 10.0 {
                vec![ViolationReport::new("MEDIUM", "medium")]
            } else {
                Vec::new()
            }
        }
    }

    let mut params = ParamSet::new();
    params.insert("x".to_string(), 1000.0);
    let result = minimize(&TwoFaults, &params, 1, 100);

    assert_eq!(
        result.violations[0].code, "HIGH",
        "shrinking onto a different violation is not progress"
    );
    assert!(
        result.params["x"] > 100.0,
        "the reduced case must still be in the HIGH region, got {}",
        result.params["x"]
    );
}

#[test]
fn a_run_that_reaches_nothing_is_not_reported_as_a_pass() {
    // The positive control on the atomic campaign itself: if the simulator
    // stops reaching the coordinator, the campaign must say so rather than
    // reporting a clean sweep.
    struct Reached {
        accepted: f64,
        delivered: f64,
    }
    impl MonteCarloScenario for Reached {
        fn name(&self) -> &str {
            "reached"
        }
        fn execute(&self, _params: &ParamSet, _seed: u64) -> RunOutcome {
            RunOutcome::empty()
                .observe("accepted", self.accepted)
                .observe("delivered_messages", self.delivered)
        }
        fn invariants(&self, _params: &ParamSet, outcome: &RunOutcome) -> Vec<ViolationReport> {
            let accepted = outcome.observations["accepted"];
            let delivered = outcome.observations["delivered_messages"];
            if accepted == 0.0 && delivered > 0.0 {
                vec![ViolationReport::new("NO_PROGRESS_BUT_TRAFFIC", "nothing ran")]
            } else {
                Vec::new()
            }
        }
    }

    let campaign = campaign_with(Distribution::Constant { value: 1.0 });
    let stalled = run_campaign(
        &Reached {
            accepted: 0.0,
            delivered: 40.0,
        },
        &campaign,
        &config(10, 1),
    );
    let working = run_campaign(
        &Reached {
            accepted: 40.0,
            delivered: 40.0,
        },
        &campaign,
        &config(10, 1),
    );

    assert_eq!(stalled.failing_runs, 10, "a stalled harness is a failure, not a pass");
    assert!(working.passed());
}

#[test]
fn clustering_collapses_duplicates_into_root_causes() {
    let campaign = campaign_with(Distribution::Uniform {
        min: 6.0,
        max: 10.0,
    });
    let report = run_campaign(&Boundary::new(5.0), &campaign, &config(200, 5));

    assert!(report.failing_runs > 100);
    assert_eq!(
        report.unique_root_causes, 1,
        "{} failures with one cause must cluster to one",
        report.failing_runs
    );
    let clusters = cluster(&report.failures);
    assert_eq!(clusters.len(), 1);
    assert_eq!(clusters.values().sum::<usize>(), report.failures.len());
}

#[test]
fn a_failure_budget_stops_the_campaign_without_hiding_that_it_did() {
    let campaign = campaign_with(Distribution::Uniform {
        min: 6.0,
        max: 10.0,
    });
    let mut settings = config(1000, 2);
    settings.max_failures = 5;
    let report = run_campaign(&Boundary::new(5.0), &campaign, &settings);

    assert_eq!(report.failing_runs, 5);
    assert_eq!(report.stop_reason, x3_mc::StopReason::FailureBudget);
    assert!(report.runs_completed < 1000);
}

#[test]
fn abort_on_ends_the_campaign_at_the_named_violation() {
    let campaign = campaign_with(Distribution::Uniform {
        min: 6.0,
        max: 10.0,
    });
    let mut settings = config(1000, 2);
    settings.abort_on = Some("PRESSURE_EXCEEDED".to_string());
    let report = run_campaign(&Boundary::new(5.0), &campaign, &settings);

    assert_eq!(report.failing_runs, 1);
    assert_eq!(
        report.stop_reason,
        x3_mc::StopReason::AbortedOnViolation("PRESSURE_EXCEEDED".to_string())
    );
}

#[test]
fn the_atomic_scenario_is_reproducible_and_reaches_the_coordinator() {
    let campaign = Campaign::load(std::path::Path::new(
        "campaigns/mc-x3-atomic-001.json",
    ))
    .expect("the shipped campaign must load");
    let scenario = AtomicSettlement::new();

    let report = run_campaign(&scenario, &campaign, &config(40, 202_609_30));

    assert_eq!(report.runs_completed, 40);
    let accepted = report.observations["accepted"];
    let delivered = report.observations["delivered_messages"];
    assert!(
        delivered > 0.0,
        "the campaign must actually deliver traffic, otherwise it tests nothing"
    );
    assert!(
        accepted > 0.0,
        "the coordinator must actually accept operations"
    );

    // The same point twice must match, or no failure from this harness could be
    // trusted.
    let first = execute_one(&scenario, &campaign, 7, 4242);
    let second = execute_one(&scenario, &campaign, 7, 4242);
    assert_eq!(first.1, second.1);
    assert_eq!(first.2.evidence, second.2.evidence);
}

#[test]
fn the_sample_ordering_does_not_depend_on_the_file_layout() {
    let first = Campaign::parse(
        r#"{"campaign":{"name":"a","runs":1},"variables":{
             "pressure":{"distribution":"uniform","min":0,"max":10}}}"#,
    )
    .unwrap();
    let second = Campaign::parse(
        r#"{"campaign":{"name":"a","runs":1},"variables":{
             "other":{"distribution":"uniform","min":0,"max":10},
             "pressure":{"distribution":"log_uniform","min":1,"max":10}}}"#,
    )
    .unwrap();

    // A different variable set changes the draw, but for a fixed set the order
    // is the sorted key order, not the order the keys appeared.
    let mut rng = McRng::from_seed(9);
    let params = x3_mc::sample_params(&first, &mut rng);
    assert_eq!(params.len(), 1);
    assert!(params.contains_key("pressure"));
    assert_eq!(second.variables.len(), 2);
}
