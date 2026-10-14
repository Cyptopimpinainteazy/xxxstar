//! Parameterization must not move the fixtures.
//!
//! `SimParams::for_scenario` was introduced so a search can explore the run
//! space without a second simulator. The risk is that the refactor silently
//! re-tunes the fixed scenarios: every seeded run would still pass, and every
//! seeded run would mean something different. These tests pin the plans that
//! the hardcoded scenarios produced before parameters existed.

use x3_sim::faults::{FaultKind, FaultPlan};
use x3_sim::params::SimParams;
use x3_sim::{run, run_with, Scenario, SimConfig, SimRng, START_UNIX_MS};

const STEPS: usize = 120;
const SESSIONS: usize = 8;
const CLIENTS: usize = 3;

fn plan(scenario: Scenario) -> FaultPlan {
    let mut rng = SimRng::from_seed(1);
    let horizon = (STEPS as u64) * 500;
    FaultPlan::generate(scenario, &mut rng, CLIENTS, SESSIONS, START_UNIX_MS, horizon)
}

fn kinds(scenario: Scenario) -> Vec<(&'static str, u64)> {
    plan(scenario)
        .events()
        .iter()
        .map(|event| {
            let name = match event.kind {
                FaultKind::Partition { .. } => "partition",
                FaultKind::Heal => "heal",
                FaultKind::CrashAndRestart { .. } => "crash",
                FaultKind::SlowLink { .. } => "slow-link",
                FaultKind::StaleWrite { .. } => "stale-write",
            };
            (name, event.at_ms - START_UNIX_MS)
        })
        .collect()
}

#[test]
fn the_happy_path_schedules_no_faults() {
    assert!(plan(Scenario::HappyPath).is_empty());
}

#[test]
fn the_claim_refund_race_still_cuts_the_link_once_at_forty_percent() {
    // horizon = 120 * 500 = 60_000ms, so one percent is 600ms.
    assert_eq!(
        kinds(Scenario::ClaimRefundRace),
        vec![("partition", 24_000), ("heal", 30_000)]
    );
}

#[test]
fn the_partition_storm_still_cuts_the_link_four_times() {
    assert_eq!(
        kinds(Scenario::PartitionStorm),
        vec![
            ("partition", 7_200),
            ("heal", 10_200),
            ("slow-link", 12_000),
            ("partition", 18_000),
            ("heal", 21_007),
            ("partition", 31_200),
            ("heal", 34_214),
            ("slow-link", 39_000),
            ("partition", 44_400),
            ("heal", 47_421),
        ]
    );
}

#[test]
fn the_crash_recovery_scenario_still_crashes_twice_and_loses_one_write() {
    let events = plan(Scenario::CrashRecovery);

    assert_eq!(
        kinds(Scenario::CrashRecovery),
        vec![
            ("crash", 15_000),
            ("stale-write", 27_000),
            ("crash", 36_000),
        ]
    );
    // The first crash is long, the second is half as long. Read them off the
    // plan rather than by index, since the plan is ordered by time.
    let durations: Vec<u64> = events
        .events()
        .iter()
        .filter_map(|event| match event.kind {
            FaultKind::CrashAndRestart { duration_ms } => Some(duration_ms),
            _ => None,
        })
        .collect();
    assert_eq!(durations, vec![4_000, 2_000]);
}

#[test]
fn the_default_point_reproduces_the_scenario_output_exactly() {
    for scenario in Scenario::all() {
        let config = SimConfig {
            seed: 948_218_671,
            scenario,
            sessions: SESSIONS,
            steps: STEPS,
            nodes: 4,
        };
        let baseline = run(&config);
        let explicit = run_with(&config, &SimParams::for_scenario(scenario));

        assert_eq!(baseline.trace_digest, explicit.trace_digest);
        assert_eq!(baseline.state_digest, explicit.state_digest);
        assert_eq!(baseline.violations, explicit.violations);
    }
}

#[test]
fn parameters_actually_reach_the_run() {
    // A calm run must not look like a stormy one, or the seam is inert.
    let config = SimConfig {
        seed: 42,
        scenario: Scenario::PartitionStorm,
        sessions: SESSIONS,
        steps: STEPS,
        nodes: 4,
    };
    let stormy = run(&config);
    let calm = run_with(
        &config,
        &SimParams {
            partition_events: 0,
            slow_link_events: 0,
            ..SimParams::for_scenario(Scenario::PartitionStorm)
        },
    );

    assert_eq!(stormy.partitions, 4);
    assert_eq!(calm.partitions, 0);
    assert_ne!(stormy.trace_digest, calm.trace_digest);
}

#[test]
fn a_zero_jitter_clock_is_still_deterministic() {
    let config = SimConfig {
        seed: 7,
        scenario: Scenario::ClaimRefundRace,
        sessions: 4,
        steps: 40,
        nodes: 3,
    };
    let params = SimParams {
        clock_jitter_ms: 0,
        ..SimParams::for_scenario(Scenario::ClaimRefundRace)
    };

    let first = run_with(&config, &params);
    let second = run_with(&config, &params);

    assert_eq!(first.trace_digest, second.trace_digest);
}

#[test]
fn crash_and_stale_write_parameters_reach_a_non_crash_scenario() {
    // The `ClaimRefundRace` shape schedules no crashes by default. A campaign
    // that adds crashes to it must get crashes: an earlier version of the plan
    // ignored those counts outside `CrashRecovery`, so the search sampled a
    // variable, injected nothing, and reported a clean sweep over a fault it
    // never delivered.
    let config = SimConfig {
        seed: 3,
        scenario: Scenario::ClaimRefundRace,
        sessions: SESSIONS,
        steps: STEPS,
        nodes: 4,
    };
    let defaults = SimParams::for_scenario(Scenario::ClaimRefundRace);
    assert_eq!(
        (defaults.crash_events, defaults.stale_write_events, defaults.slow_link_events),
        (0, 0, 0),
        "the race scenario is calm by default"
    );

    let faulted_params = SimParams {
        crash_events: 2,
        stale_write_events: 1,
        slow_link_events: 1,
        ..defaults.clone()
    };
    let calm = run_with(&config, &defaults);
    let faulted = run_with(&config, &faulted_params);

    assert_eq!(calm.restarts, 0);
    assert!(
        faulted.restarts > 0,
        "a requested crash must actually restart the coordinator"
    );
    assert!(
        faulted.stale_writes > 0,
        "a requested lost write must actually be lost"
    );
    assert_ne!(
        calm.state_digest, faulted.state_digest,
        "injecting faults must change the run, or the parameters are decorative"
    );
}
