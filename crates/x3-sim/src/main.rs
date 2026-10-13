//! `x3-sim` command line entry point.
//!
//! Exit codes follow the repository's fail-closed convention: `0` when every
//! invariant held, `1` when a run produced a violation, `2` for a usage error.
//!
//! When a run fails, the simulator builds a **failure packet**: the exact run,
//! the first step that broke an invariant, the state before and after it, the
//! faults that were active, and the coordinator symbols the invariant
//! implicates. `--minimize` (on by default for failing runs) then shrinks the
//! config until no dimension can be removed while the same violation fires,
//! and re-runs the smallest candidate to verify it. The packet's `root_cause`
//! command line is what hands the failure to an investigation agent next.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use x3_sim::{minimize, run, FailurePacket, Scenario, SimConfig, SimOutcome, DEFAULT_MAX_RUNS};

const USAGE: &str = "\
x3-sim — deterministic fault-injection simulator for X3 atomic swaps

USAGE:
    cargo run -p x3-sim -- [OPTIONS]

OPTIONS:
    --seed <u64>          Seed for the run (default 0). Replay uses this.
    --scenario <name>     happy-path | claim-refund-race | partition-storm | crash-recovery
    --sessions <n>        Atomic swap sessions in the ledger (default 8)
    --steps <n>           Scheduler steps (default 120)
    --nodes <n>           Node 0 is the coordinator, the rest are clients (default 4)
    --out <dir>           Write the evidence bundle (JSON + trace) here
    --packet <dir>        Write the failure packet (JSON + Markdown) here on failure
    --hunt <n>            Run n consecutive seeds from --seed; collect failures
    --max-failures <n>    With --hunt, stop after n failures (default 5)
    --root-cause          Dispatch every failure packet to the root-cause agent
    --no-minimize         Do not minimize a failing config automatically
    --json                Print the evidence bundle to stdout instead of a summary
    -h, --help            Show this help

EXIT CODES:
    0  every invariant held
    1  at least one invariant was violated
    2  usage, I/O, or evidence-writing error
";

fn main() -> ExitCode {
    let mut config = SimConfig::default();
    let mut out: Option<PathBuf> = None;
    let mut packet_dir: Option<PathBuf> = None;
    let mut hunt: Option<usize> = None;
    let mut max_failures = 5usize;
    let mut do_minimize = true;
    let mut root_cause = false;
    let mut json = false;

    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut index = 0;
    while index < args.len() {
        let arg = args[index].as_str();
        let mut value = || -> Option<String> {
            index += 1;
            args.get(index).cloned()
        };
        match arg {
            "-h" | "--help" => {
                print!("{USAGE}");
                return ExitCode::SUCCESS;
            }
            "--json" => json = true,
            "--no-minimize" => do_minimize = false,
            "--root-cause" => root_cause = true,
            "--seed" => match value().and_then(|raw| raw.parse::<u64>().ok()) {
                Some(parsed) => config.seed = parsed,
                None => return usage_error("--seed requires an unsigned integer"),
            },
            "--hunt" => match value().and_then(|raw| raw.parse::<usize>().ok()) {
                Some(parsed) if parsed > 0 => hunt = Some(parsed),
                _ => return usage_error("--hunt requires a positive integer"),
            },
            "--max-failures" => match value().and_then(|raw| raw.parse::<usize>().ok()) {
                Some(parsed) if parsed > 0 => max_failures = parsed,
                _ => return usage_error("--max-failures requires a positive integer"),
            },
            "--sessions" => match value().and_then(|raw| raw.parse::<usize>().ok()) {
                Some(parsed) => config.sessions = parsed,
                None => return usage_error("--sessions requires a positive integer"),
            },
            "--steps" => match value().and_then(|raw| raw.parse::<usize>().ok()) {
                Some(parsed) => config.steps = parsed,
                None => return usage_error("--steps requires a positive integer"),
            },
            "--nodes" => match value().and_then(|raw| raw.parse::<usize>().ok()) {
                Some(parsed) => config.nodes = parsed,
                None => return usage_error("--nodes requires an integer of at least 2"),
            },
            "--scenario" => match value().and_then(|raw| Scenario::parse(&raw)) {
                Some(parsed) => config.scenario = parsed,
                None => return usage_error("--scenario must be one of the documented names"),
            },
            "--out" => match value() {
                Some(path) => out = Some(PathBuf::from(path)),
                None => return usage_error("--out requires a directory"),
            },
            "--packet" => match value() {
                Some(path) => packet_dir = Some(PathBuf::from(path)),
                None => return usage_error("--packet requires a directory"),
            },
            other => return usage_error(&format!("unknown argument '{other}'")),
        }
        index += 1;
    }

    if let Some(count) = hunt {
        return hunt_seeds(
            &config,
            count,
            max_failures,
            do_minimize,
            root_cause,
            packet_dir.as_deref(),
            out.as_deref(),
            json,
        );
    }

    if let Some(dir) = &out {
        if let Err(error) = std::fs::create_dir_all(dir) {
            eprintln!("x3-sim: cannot create evidence directory: {error}");
            return ExitCode::from(2);
        }
    }

    let outcome = run(&config);

    if outcome.is_pass() {
        if json {
            println!("{}", outcome.to_evidence_json());
        } else {
            print_summary(&outcome);
        }
    }

    if let Some(dir) = &out {
        if let Err(error) = write_evidence(dir, &outcome) {
            eprintln!("x3-sim: cannot write evidence bundle: {error}");
            return ExitCode::from(2);
        }
        if !json {
            println!("evidence: {}", dir.display());
        }
    }

    if outcome.is_pass() {
        return ExitCode::SUCCESS;
    }

    if !json {
        // Failing, human mode: the summary, then the packet. In `--json` mode
        // the packet *is* the document, so `stdout` stays parseable as one.
        print_summary(&outcome);
    }

    match report_failure(
        &config,
        &outcome,
        do_minimize,
        root_cause,
        packet_dir.as_deref(),
        json,
    ) {
        Ok(()) => ExitCode::FAILURE,
        Err(error) => {
            eprintln!("x3-sim: failure packet could not be completed: {error}");
            ExitCode::from(2)
        }
    }
}

/// Build (and optionally minimize, verify, and persist) the failure packet for
/// one failing run. Prints the packet and the next command either way.
fn report_failure(
    config: &SimConfig,
    outcome: &SimOutcome,
    do_minimize: bool,
    root_cause: bool,
    packet_dir: Option<&Path>,
    json: bool,
) -> Result<(), String> {
    let mut packet = FailurePacket::from_outcome(outcome)
        .ok_or_else(|| "a failing run produced no packet".to_string())?;

    if do_minimize {
        let code = packet.invariant.clone();
        if let Some(minimized) = minimize(config, Some(&code), DEFAULT_MAX_RUNS, x3_sim::run) {
            packet.attach_minimized(minimized.to_reproducer());
        }
    }

    let mut packet_json: Option<PathBuf> = None;
    let dir = packet_dir.or_else(|| root_cause.then(|| Path::new("target/x3-failure-packets")));
    if let Some(dir) = dir {
        let (json_path, md_path) = packet.write(dir).map_err(|error| error.to_string())?;
        if !json {
            println!("packet: {}", json_path.display());
            println!("packet: {}", md_path.display());
        }
        packet_json = Some(json_path);
    }

    if root_cause {
        // `root_cause` always supplies a packet directory, and a failed
        // packet write returned above, so the packet path exists here.
        if let Some(path) = &packet_json {
            dispatch_root_cause(path, json);
        }
    }

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&packet).unwrap_or_else(|_| "{}".to_string())
        );
    } else {
        println!();
        println!("{}", packet.to_markdown());
        match &packet_json {
            Some(path) => println!(
                "root_cause_command = python3 crates/x3-sim/scripts/root_cause.py {}",
                path.display()
            ),
            // Printing a command with a path that does not exist would invite
            // a failed run: say what to add instead.
            None => println!("{}", root_cause_hint()),
        }
    }
    Ok(())
}

/// What to run when no packet was persisted (so there is no path to name yet).
fn root_cause_hint() -> String {
    "root_cause: rerun with --packet <dir> (or --root-cause) to persist the packet, \
     then: python3 crates/x3-sim/scripts/root_cause.py <written packet.json>"
        .to_string()
}

/// Run the dispatcher on a stored packet. Dispatch failing is not a reason to
/// hide the violation: the run still reports its failure and exit code, and
/// the packet remains on disk either way.
fn dispatch_root_cause(packet_path: &Path, quiet: bool) {
    let note = |message: String| {
        if quiet {
            eprintln!("{message}");
        } else {
            println!("{message}");
        }
    };
    let script = Path::new("crates/x3-sim/scripts/root_cause.py");
    if !script.exists() {
        note(format!(
            "root_cause: dispatcher not found at {}; packet kept at {}",
            script.display(),
            packet_path.display()
        ));
        return;
    }
    note(format!("root_cause: dispatching {}", packet_path.display()));
    let mut command = std::process::Command::new("python3");
    command.arg(script).arg(packet_path);
    if quiet {
        // stdout is the packet JSON in this mode; the child gets its own.
        command.stdout(std::process::Stdio::null());
    }
    match command.status() {
        Ok(status) if status.success() => {}
        Ok(status) => note(format!(
            "root_cause: dispatcher exited with {status}; packet kept"
        )),
        Err(error) => note(format!(
            "root_cause: could not run python3 ({error}); packet kept"
        )),
    }
}

/// Run `count` consecutive seeds and report every distinct failure.
#[allow(clippy::too_many_arguments)]
fn hunt_seeds(
    base: &SimConfig,
    count: usize,
    max_failures: usize,
    do_minimize: bool,
    root_cause: bool,
    packet_dir: Option<&Path>,
    out: Option<&Path>,
    json: bool,
) -> ExitCode {
    let report = x3_sim::hunt(base, count, max_failures, do_minimize, run);
    // Dispatch needs the packet on disk, so `--root-cause` implies a default
    // packet directory even when `--packet` is not given.
    let dir = packet_dir.or_else(|| root_cause.then(|| Path::new("target/x3-failure-packets")));

    // Duplicates get no packet and no dispatch, but the run itself is
    // evidence: `--out` must not silently drop it. An evidence-write error is
    // recorded, not returned yet: the failure packets are the part a
    // root-cause agent needs, so they are still written and dispatched.
    let mut evidence_error: Option<String> = None;
    for failure in report.failures.iter().chain(report.duplicates.iter()) {
        if let Some(dir) = out {
            if let Err(error) = write_evidence(dir, &failure.outcome) {
                if evidence_error.is_none() {
                    evidence_error = Some(format!("cannot write evidence bundle: {error}"));
                }
            }
        }
    }
    for failure in &report.failures {
        if let Some(dir) = dir {
            if let Err(error) = failure.packet.write(dir) {
                eprintln!("x3-sim: cannot write failure packet: {error}");
                return ExitCode::from(2);
            }
        }
        if root_cause {
            if let Some(path) =
                dir.map(|dir| dir.join(format!("{}.json", failure.packet.file_stem())))
            {
                dispatch_root_cause(&path, json);
            }
        }
    }
    if let Some(error) = evidence_error {
        eprintln!("x3-sim: {error}");
        return ExitCode::from(2);
    }

    let summary = serde_json::json!({
        "schema": "x3-sim-hunt-summary-v1",
        "scenario": base.scenario.as_str(),
        "first_seed": base.seed,
        "seeds_requested": count,
        "seeds_run": report.seeds_run,
        "passes": report.passes,
        "duplicates": report.duplicates_seen,
        "duplicate_evidence_samples": report.duplicates.len(),
        "failures": report.failures.iter().map(|failure| serde_json::json!({
            "seed": failure.config.seed,
            "invariant": failure.packet.invariant,
            "failure_id": failure.packet.failure_id,
            "violations": failure.outcome.violations.len(),
            "packet_stem": failure.packet.file_stem(),
            "replay": failure.packet.replay_command,
        })).collect::<Vec<_>>(),
        "stopped_at_max_failures": report.stopped_at_max_failures,
    });

    if let Some(dir) = out {
        let path = dir.join(format!(
            "hunt-summary-{}-{}.json",
            base.scenario.as_str(),
            base.seed
        ));
        if let Err(error) = std::fs::create_dir_all(dir).and_then(|_| {
            std::fs::write(
                &path,
                serde_json::to_string_pretty(&summary).unwrap_or_default() + "\n",
            )
        }) {
            eprintln!("x3-sim: cannot write hunt summary: {error}");
            return ExitCode::from(2);
        }
    }

    // `--json` means "one parseable JSON document on stdout", for a single
    // run and for a hunt alike.
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&summary).unwrap_or_else(|_| "{}".to_string())
        );
        return if report.is_clean() {
            ExitCode::SUCCESS
        } else {
            ExitCode::FAILURE
        };
    }

    println!(
        "hunt: seeds {}..{}  run = {}  passes = {}  failures = {} duplicates = {}{}",
        base.seed,
        base.seed.wrapping_add(count as u64),
        report.seeds_run,
        report.passes,
        report.failures.len(),
        report.duplicates_seen,
        if report.stopped_at_max_failures {
            format!(" (stopped at --max-failures {max_failures})")
        } else {
            String::new()
        }
    );
    if report.is_clean() {
        println!("no invariant violations in {} runs", count);
        return ExitCode::SUCCESS;
    }
    for failure in &report.failures {
        let packet = &failure.packet;
        println!(
            "  seed={} invariant={} session={} replay=`{}`",
            failure.outcome.seed,
            packet.invariant,
            packet.session,
            failure.outcome.replay_command()
        );
        if let Some(minimized) = &packet.minimized {
            println!(
                "    minimized (verified={}): {}",
                minimized.verified, minimized.replay_command
            );
        }
        println!(
            "    packet: {}  id={}",
            packet.file_stem(),
            packet.failure_id
        );
        match dir {
            Some(dir) => println!(
                "    root cause: python3 crates/x3-sim/scripts/root_cause.py {}",
                dir.join(format!("{}.json", packet.file_stem())).display()
            ),
            None => println!("    {}", root_cause_hint()),
        }
    }
    ExitCode::FAILURE
}

fn usage_error(message: &str) -> ExitCode {
    eprintln!("x3-sim: {message}\n\n{USAGE}");
    ExitCode::from(2)
}

fn print_summary(outcome: &SimOutcome) {
    println!("seed = {}", outcome.seed);
    println!("scenario = {}", outcome.scenario);
    println!(
        "steps = {}  accepted = {}  rejected = {}",
        outcome.steps, outcome.accepted, outcome.rejected
    );
    println!(
        "restarts = {}  stale_writes = {}  partitions = {}",
        outcome.restarts, outcome.stale_writes, outcome.partitions
    );
    println!(
        "network: sent = {}  delivered = {}  dropped = {}  partitioned = {}  reordered = {}",
        outcome.network.sent,
        outcome.network.delivered,
        outcome.network.dropped,
        outcome.network.partitioned,
        outcome.network.reordered
    );
    println!(
        "sessions: completed = {}  refunded = {}",
        outcome.sessions_completed, outcome.sessions_refunded
    );
    println!("trace_digest = {}", outcome.trace_digest);
    println!("state_digest = {}", outcome.state_digest);

    if outcome.is_pass() {
        println!("PASS");
    } else {
        println!("FAIL");
        println!("violations:");
        for violation in &outcome.violations {
            println!(
                "  {} session={} {}",
                violation.code, violation.session_id, violation.detail
            );
        }
        println!("Replay:");
        println!("  {}", outcome.replay_command());
    }
}

fn write_evidence(dir: &std::path::Path, outcome: &SimOutcome) -> std::io::Result<()> {
    // `--hunt --out <new dir>` reaches this before anything else creates the
    // directory, so the evidence path owns its own parent.
    std::fs::create_dir_all(dir)?;
    let stem = format!("x3-sim-{}-{}", outcome.scenario, outcome.seed);
    std::fs::write(dir.join(format!("{stem}.json")), outcome.to_evidence_json())?;
    std::fs::write(
        dir.join(format!("{stem}.trace.log")),
        outcome.trace.join("\n"),
    )?;
    Ok(())
}
