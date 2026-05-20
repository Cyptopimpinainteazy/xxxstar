//! X3 Launch Checklist Validator CLI
//!
//! Usage:
//!   x3-launch-check --check all
//!   x3-launch-check --check pre-launch
//!   x3-launch-check --check launch-day
//!   x3-launch-check --check post-launch
//!   x3-launch-check --check failure-conditions
//!   x3-launch-check --json          (emit JSON report to stdout)
//!   x3-launch-check --allow-incomplete-blocking

use x3_launch_validator::{
    checklist::{CheckPhase, LaunchChecklist},
    checks::run_all,
    reporter::ChecklistReporter,
};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let json_mode = args.iter().any(|a| a == "--json");
    let allow_incomplete_blocking = args.iter().any(|a| a == "--allow-incomplete-blocking");
    let phase_filter = args
        .windows(2)
        .find(|w| w[0] == "--check")
        .map(|w| w[1].as_str());

    let mut checklist = LaunchChecklist::canonical();

    // Run all checks (phase filtering is applied at reporting time)
    run_all(&mut checklist);

    if json_mode {
        // Output machine-readable JSON
        let json = serde_json::to_string_pretty(&checklist.items).expect("serialization failed");
        println!("{json}");

        // Fail closed by default: any blocking check that is not PASS is launch-blocking.
        // Use --allow-incomplete-blocking only for local advisory runs.
        if (!allow_incomplete_blocking && checklist.any_blocking_unmet())
            || (allow_incomplete_blocking && checklist.any_blocking_failed())
        {
            std::process::exit(1);
        }
        return;
    }

    // Apply optional phase filter for human output
    if let Some(filter) = phase_filter {
        if filter != "all" {
            let phase = match filter {
                "pre-launch" => Some(CheckPhase::PreLaunch),
                "launch-day" => Some(CheckPhase::LaunchDay),
                "post-launch" => Some(CheckPhase::PostLaunch30Days),
                "failure-conditions" | "failure" => Some(CheckPhase::FailureConditions),
                _ => {
                    eprintln!("Unknown phase filter: {filter}");
                    eprintln!("Valid phases: all, pre-launch, launch-day, post-launch, failure-conditions");
                    std::process::exit(2);
                }
            };
            if let Some(p) = phase {
                // Null-out items not in this phase for cleaner output
                for item in checklist.items.iter_mut() {
                    if item.phase != p {
                        item.result = None;
                    }
                }
            }
        }
    }

    ChecklistReporter::print(&checklist);

    if (!allow_incomplete_blocking && checklist.any_blocking_unmet())
        || (allow_incomplete_blocking && checklist.any_blocking_failed())
    {
        std::process::exit(1);
    }
}
