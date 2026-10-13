//! Failure-packet and minimizer behaviour, against real runs of the real
//! coordinator.
//!
//! The simulator's positive control must not manufacture a packet, and the
//! minimizer must shrink real runs — with the final candidate verified by
//! another real run. The only synthetic ingredient here is the predicate: the
//! coordinator no longer has a reachable violation (that is what #558 fixed),
//! so the tests minimize a real, reproducible *near-failure* (a refused
//! operation) to exercise the machinery end to end.

use x3_sim::invariants::Violation;
use x3_sim::{minimize, run, FailurePacket, Scenario, SimConfig, DEFAULT_MAX_RUNS};

#[test]
fn a_passing_real_run_produces_no_packet() {
    let config = SimConfig {
        seed: 5,
        scenario: Scenario::HappyPath,
        ..SimConfig::default()
    };
    let outcome = run(&config);
    assert!(outcome.is_pass(), "the positive control must pass");
    assert!(
        FailurePacket::from_outcome(&outcome).is_none(),
        "a passing run must never produce a failure packet"
    );
    assert!(outcome.state_before.is_none());
    assert!(outcome.first_bad_op.is_none());
}

#[test]
fn the_minimizer_drives_real_runs_and_verifies_its_result() {
    let start = SimConfig {
        seed: 11,
        scenario: Scenario::ClaimRefundRace,
        sessions: 24,
        steps: 400,
        nodes: 8,
    };

    // A real predicate over real outcomes: the run contains at least one
    // refused operation, which is the observable shape a violation would have
    // had. The synthetic violation carries a predicate code, never a real
    // invariant name, so nothing here can be mistaken for production evidence.
    let runner = |config: &SimConfig| {
        let mut outcome = run(config);
        if outcome.rejected >= 1 {
            outcome.violations.push(Violation {
                code: "TEST_REFUSAL_PRESENT",
                session_id: "<predicate>".to_string(),
                detail: format!("rejected = {}", outcome.rejected),
            });
        }
        outcome
    };

    let start_outcome = runner(&start);
    assert!(
        !start_outcome.is_pass(),
        "the starting config must reproduce the predicate"
    );

    let minimized = minimize(&start, None, DEFAULT_MAX_RUNS, runner)
        .expect("the predicate survives minimization");
    assert!(
        minimized.verified,
        "the minimized run was re-run and reproduced"
    );
    assert!(
        minimized.config.sessions <= start.sessions
            && minimized.config.steps <= start.steps
            && minimized.config.nodes <= start.nodes,
        "minimization never grows the config"
    );
    // A strict `<` here would assert something about the coordinator's
    // refusal distribution, not about the minimizer: a correct minimizer
    // returns the start config when nothing smaller reproduces. The proof
    // that a reducible predicate shrinks lives in `minimize.rs`, against a
    // deterministic synthetic predicate; this integration test owns "real
    // runs, never grown, verified".
    assert!(
        minimized.outcome.rejected >= 1,
        "the minimized run really reproduces the predicate"
    );
}

#[test]
fn the_cli_replays_a_recorded_config_to_the_same_trace_digest_and_verdict() {
    // The replay command must be the one the CLI accepts, with every
    // dimension the failing run used, and the CLI run must reach the same
    // verdict as the library run.
    let config = SimConfig {
        seed: 7,
        scenario: Scenario::PartitionStorm,
        sessions: 3,
        steps: 40,
        nodes: 3,
    };
    let outcome = run(&config);
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_x3-sim"))
        .args(config.replay_args())
        .output()
        .expect("run the simulator binary");
    assert!(
        output.status.success(),
        "the replay of a passing config must exit 0: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains(&outcome.trace_digest),
        "the CLI replay must produce the same trace digest as the library run: {stdout}"
    );
}

#[test]
fn a_packet_replay_command_names_every_dimension_the_run_used() {
    let config = SimConfig {
        seed: 7,
        scenario: Scenario::PartitionStorm,
        sessions: 3,
        steps: 40,
        nodes: 3,
    };
    let mut outcome = run(&config);
    // No reachable coordinator violation exists (#558 closed them), so the
    // failing verdict here is a predicate-only code, never an invariant name.
    outcome.violations.push(Violation {
        code: "TEST_REFUSAL_PRESENT",
        session_id: "<predicate>".to_string(),
        detail: format!("rejected = {}", outcome.rejected),
    });
    let packet = FailurePacket::from_outcome(&outcome).expect("a failing run yields a packet");

    assert_eq!(packet.replay_command, config.replay_command());
    for expected in [
        "--seed 7",
        "--scenario partition-storm",
        "--sessions 3",
        "--steps 40",
        "--nodes 3",
    ] {
        assert!(
            packet.replay_command.contains(expected),
            "the replay must pin `{expected}`: {}",
            packet.replay_command
        );
    }
    assert_ne!(
        packet.commit, "unknown",
        "the packet resolves the checkout HEAD"
    );
    // `X3_COMMIT` overrides discovery and the contract accepts whatever it
    // names, so the full-hash shape only holds for the checkout path.
    let overridden = std::env::var("X3_COMMIT")
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false);
    if !overridden {
        assert_eq!(
            packet.commit.len(),
            40,
            "HEAD is a full commit hash: {}",
            packet.commit
        );
    }
    assert!(packet.to_markdown().contains("## All violations"));
}

#[test]
fn a_packet_records_the_checkout_branch_and_dirtiness() {
    let config = SimConfig {
        seed: 13,
        scenario: Scenario::CrashRecovery,
        sessions: 4,
        steps: 40,
        nodes: 3,
    };
    let mut outcome = run(&config);
    outcome.violations.push(Violation {
        code: "TEST_REFUSAL_PRESENT",
        session_id: "<predicate>".to_string(),
        detail: format!("rejected = {}", outcome.rejected),
    });
    let packet = FailurePacket::from_outcome(&outcome).expect("a failing run yields a packet");

    // The oracle is git itself, recomputed at assertion time: a worktree can
    // be dirty while the suite runs (this very test file is an edit), so the
    // packet and the checkout state must be compared as a pair.
    let git = |args: &[&str]| -> Option<String> {
        std::process::Command::new("git")
            .args(args)
            .output()
            .ok()
            .filter(|output| output.status.success())
            .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
    };
    let expected_branch = git(&["branch", "--show-current"])
        .filter(|branch| !branch.is_empty())
        .unwrap_or_else(|| "unknown".to_string());
    let expected_dirty = git(&["status", "--porcelain"]).map(|status| !status.is_empty());

    assert_eq!(
        packet.branch, expected_branch,
        "the packet records the branch it ran on"
    );
    assert_eq!(
        packet.worktree_dirty, expected_dirty,
        "the packet records whether the worktree was dirty"
    );
    let markdown = packet.to_markdown();
    let expected_dirty_label = match expected_dirty {
        Some(true) => "yes",
        Some(false) => "no",
        None => "unknown",
    };
    assert!(
        markdown.contains(&format!(
            "on `{expected_branch}` (dirty: {expected_dirty_label})"
        )),
        "the markdown pins the checkout state: {markdown}"
    );
}
