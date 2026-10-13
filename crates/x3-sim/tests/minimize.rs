//! The CLI hunt: many seeds, one verdict, and no packet unless something failed.

use std::path::PathBuf;
use std::process::Command;

/// A per-test directory, removed first so a stale run cannot make the test
/// pass (or fail) on somebody else's leftovers.
fn fresh_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("x3-sim-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

fn simulator(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_x3-sim"))
        .args(args)
        .output()
        .expect("run the simulator binary")
}

#[test]
fn hunt_over_clean_seeds_exits_zero_and_writes_no_packet() {
    let dir = fresh_dir("hunt");
    let output = simulator(&[
        "--hunt",
        "4",
        "--seed",
        "1000",
        "--scenario",
        "happy-path",
        "--sessions",
        "2",
        "--steps",
        "60",
        "--nodes",
        "3",
        "--packet",
        &dir.to_string_lossy(),
        "--no-minimize",
    ]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "a clean hunt must exit 0: {stdout} {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(stdout.contains("passes = 4"), "stdout was: {stdout}");
    assert!(!dir.exists(), "no failure means no packet directory");
}

#[test]
fn a_json_hunt_prints_one_parseable_json_document() {
    let output = simulator(&[
        "--hunt",
        "3",
        "--seed",
        "1000",
        "--scenario",
        "happy-path",
        "--sessions",
        "2",
        "--steps",
        "60",
        "--nodes",
        "3",
        "--json",
    ]);
    assert!(output.status.success(), "a clean hunt exits 0");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("--json must print exactly one JSON document");
    assert_eq!(parsed["schema"], "x3-sim-hunt-summary-v1");
    assert_eq!(parsed["seeds_run"], 3);
    assert_eq!(parsed["failures"].as_array().map(Vec::len), Some(0));
}

#[test]
fn a_clean_json_hunt_with_root_cause_keeps_stdout_one_document() {
    // No reachable coordinator violation exists (#558 closed them), so the
    // dispatch path cannot be driven from the CLI today: this pins the
    // combination that *is* reachable (a clean `--json --root-cause` hunt
    // stays one document). On a failing hunt the dispatcher's child stdout is
    // redirected to null in quiet mode, so the summary printer is still the
    // only writer to stdout.
    let out = fresh_dir("hunt-out");
    let output = simulator(&[
        "--hunt",
        "3",
        "--seed",
        "1000",
        "--scenario",
        "happy-path",
        "--sessions",
        "2",
        "--steps",
        "60",
        "--nodes",
        "3",
        "--json",
        "--root-cause",
        "--out",
        &out.to_string_lossy(),
    ]);
    assert!(output.status.success(), "a clean hunt exits 0");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("--json must print exactly one JSON document");
    assert_eq!(parsed["schema"], "x3-sim-hunt-summary-v1");
    // `--out` with a directory that did not exist must still produce the
    // hunt summary: the run is allowed to create its own evidence directory.
    let summary = out.join("hunt-summary-happy-path-1000.json");
    assert!(
        summary.exists(),
        "the hunt summary was written to a fresh --out dir"
    );
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn a_usage_error_exits_2_and_writes_no_packet_directory() {
    // Negative control for the hunt test above: a rejected invocation must
    // exit 2 and still write nothing.
    let dir = fresh_dir("usage");
    let output = simulator(&["--hunt", "0", "--packet", &dir.to_string_lossy()]);
    assert_eq!(output.status.code(), Some(2), "a usage error is exit 2");
    assert!(!dir.exists());
}
