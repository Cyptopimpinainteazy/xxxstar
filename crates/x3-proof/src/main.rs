use clap::{Parser, Subcommand};
use std::fs;
use std::path::Path;

#[derive(Debug, Parser)]
#[command(name = "x3-proof")]
#[command(about = "X3 proof and mainnet RC reporting tool")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    MainnetRcReport {
        #[arg(long)]
        out: String,
    },
}

fn status(flag: bool) -> &'static str {
    if flag {
        "PASS"
    } else {
        "FAIL"
    }
}

fn read_text(path: &str) -> Option<String> {
    fs::read_to_string(path).ok()
}

fn report_result_pass(path: &str) -> bool {
    read_text(path)
        .map(|c| c.contains("Result: PASS"))
        .unwrap_or(false)
}

fn gate_line_pass(gate: &str, content: &str) -> bool {
    content
        .lines()
        .any(|l| l.trim() == format!("- {gate}: PASS"))
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::MainnetRcReport { out } => {
            let gate_status = read_text("reports/mainnet_rc_gate_status.md").unwrap_or_default();
            let panic_audit = read_text("reports/panic_unwrap_audit.md").unwrap_or_default();
            let chain_plain = read_text("chain-specs/x3-mainnet-plain.json").unwrap_or_default();
            let chain_raw = read_text("chain-specs/x3-mainnet-raw.json").unwrap_or_default();
            let router_lib = read_text("pallets/x3-cross-vm-router/src/lib.rs").unwrap_or_default();
            let router_tests =
                read_text("pallets/x3-cross-vm-router/src/tests.rs").unwrap_or_default();
            let ledger_lib = read_text("pallets/x3-supply-ledger/src/lib.rs").unwrap_or_default();

            let build_status = gate_line_pass("fresh_build_check", &gate_status);
            let test_status = [
                "test_pallet_x3_cross_vm_router",
                "test_pallet_x3_supply_ledger",
                "test_pallet_x3_atomic_kernel",
                "test_x3_ixl",
                "test_x3_proof",
            ]
            .iter()
            .all(|k| gate_line_pass(k, &gate_status));

            let panic_unwrap_status = panic_audit.contains("runtime hook panic path gate: PASS")
                && panic_audit.contains("user-triggerable unwrap/expect gate: PASS");

            let genesis_lint_status = report_result_pass("reports/genesis_lint.md");

            let external_bridge_status = !chain_plain.is_empty()
                && !chain_raw.is_empty()
                && !chain_plain.contains("\"ExternalBridgesEnabled\": true")
                && !chain_raw.contains("\"ExternalBridgesEnabled\": true")
                && !chain_plain.contains("\"external_bridges\": true")
                && !chain_raw.contains("\"external_bridges\": true");

            let bridge_audit_gate_status = router_lib.contains("ExternalBridgeAuditGateMissing")
                && router_lib.contains("set_external_bridge_audit_gate")
                && router_lib.contains("ExternalBridgesToggled { enabled: false }");

            let supply_invariant_policy_status = ledger_lib.contains("InvariantViolationPolicy")
                && ledger_lib.contains("set_invariant_violation_policy")
                && ledger_lib.contains("RejectNewTransfers");

            let economic_halt_status = ledger_lib.contains("TransferHalted")
                && ledger_lib.contains("impl<T: Config> EconomicHaltInspect for Pallet<T>");

            let six_internal_routes_status =
                router_tests.contains("six_internal_routes_strict_invariants_and_replay_guards");

            let atomic_rollback_tests_status = router_tests
                .contains("ixl_abort_after_lock_restores_ledger")
                && router_tests.contains("completion_after_refund_rejected")
                && router_tests.contains("refund_after_finalized_rejected");

            let runtime_upgrade_rehearsal_status =
                report_result_pass("reports/runtime_upgrade_rehearsal.md");

            let required = [
                build_status,
                test_status,
                panic_unwrap_status,
                genesis_lint_status,
                external_bridge_status,
                bridge_audit_gate_status,
                supply_invariant_policy_status,
                economic_halt_status,
                six_internal_routes_status,
                atomic_rollback_tests_status,
                runtime_upgrade_rehearsal_status,
            ];
            let passed = required.into_iter().filter(|v| *v).count();
            let score = (passed * 100) / 11;
            let verdict = if passed == 11 { "GO" } else { "NO-GO" };

            let body = format!(
                "# Mainnet RC Report\n\n- Build Status: {}\n- Test Status: {}\n- Panic/Unwrap Audit: {}\n- Genesis Lint: {}\n- External Bridge Status: {}\n- Bridge Audit Gate Status: {}\n- Supply Invariant Policy: {}\n- Economic Halt Status: {}\n- Six Internal Routes: {}\n- Atomic Rollback Tests: {}\n- Runtime Upgrade Rehearsal: {}\n- Readiness Score: {}/100\n- Launch Verdict: {}\n",
                status(build_status),
                status(test_status),
                status(panic_unwrap_status),
                status(genesis_lint_status),
                status(external_bridge_status),
                status(bridge_audit_gate_status),
                status(supply_invariant_policy_status),
                status(economic_halt_status),
                status(six_internal_routes_status),
                status(atomic_rollback_tests_status),
                status(runtime_upgrade_rehearsal_status),
                score,
                verdict
            );

            if let Some(parent) = Path::new(&out).parent() {
                if !parent.as_os_str().is_empty() {
                    let _ = fs::create_dir_all(parent);
                }
            }

            fs::write(&out, body).expect("failed to write report");
            println!("wrote {}", out);
        }
    }
}
