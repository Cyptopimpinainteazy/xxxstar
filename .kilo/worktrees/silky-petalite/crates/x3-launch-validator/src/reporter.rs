//! Human-readable checklist reporter.

use crate::checklist::{CheckPhase, CheckResult, LaunchChecklist};

pub struct ChecklistReporter;

impl ChecklistReporter {
    /// Print a full report to stdout.
    pub fn print(checklist: &LaunchChecklist) {
        println!();
        println!("╔══════════════════════════════════════════════════════════════════╗");
        println!("║     X3 ADVERSARIAL WORLD LAUNCH CHECKLIST — vΩ-1.0              ║");
        println!("╠══════════════════════════════════════════════════════════════════╣");

        for phase in [
            CheckPhase::PreLaunch,
            CheckPhase::LaunchDay,
            CheckPhase::PostLaunch30Days,
            CheckPhase::FailureConditions,
        ] {
            let items = checklist.phase(&phase);
            if items.is_empty() {
                continue;
            }

            println!("║                                                                  ║");
            println!("║  [ {} ]", phase);

            for item in &items {
                let status = match &item.result {
                    Some(CheckResult::Pass) => "[PASS]",
                    Some(CheckResult::Fail(_)) => "[FAIL]",
                    Some(CheckResult::Skipped(_)) => "[SKIP]",
                    None => "[----]",
                };
                let blocking = if item.blocking { "!" } else { " " };
                println!("║  {blocking}{status} {:7} {}", item.id, item.description);

                if let Some(CheckResult::Fail(msg)) = &item.result {
                    println!("║         => {msg}");
                }
                if let Some(CheckResult::Skipped(msg)) = &item.result {
                    println!("║         ~> {msg}");
                }
            }
        }

        let (pass, fail, skip) = checklist.summary();
        println!("║                                                                  ║");
        println!("╠══════════════════════════════════════════════════════════════════╣");
        println!(
            "║  RESULTS  Pass: {pass:3}   Fail: {fail:3}   Skip/Pending: {skip:3}            ║"
        );

        if checklist.any_blocking_failed() {
            println!("║  STATUS: LAUNCH BLOCKED — resolve all '!' failures before launch ║");
        } else if checklist.all_blocking_passed() {
            println!("║  STATUS: ALL BLOCKING CHECKS PASSED — system may proceed          ║");
        } else {
            println!("║  STATUS: INCOMPLETE — pending checks require live environment     ║");
        }
        println!("╚══════════════════════════════════════════════════════════════════╝");
        println!();
    }
}
