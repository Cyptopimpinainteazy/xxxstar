//! `x3-mc` — run, replay, minimize and report Monte Carlo campaigns.
//!
//! ```text
//! x3-mc run     <campaign.json> [--runs N] [--seed N] [--out DIR] [--json]
//! x3-mc replay  <campaign.json> --index N [--master-seed N] [--json]
//! x3-mc minimize <campaign.json> --index N [--master-seed N] [--json]
//! x3-mc report  <directory>
//! ```
//!
//! Exit codes match the rest of the X3 tooling: `0` every invariant held,
//! `1` a violation was found, `2` usage or environment error.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use x3_mc::engine::{execute_one, run_campaign, CampaignConfig};
use x3_mc::evidence::EvidenceBundle;
use x3_mc::minimize::minimize;
use x3_mc::{AtomicSettlement, Campaign};

const USAGE: &str = "\
x3-mc — probabilistic adversarial simulation engine

USAGE:
    x3-mc run      <campaign.json> [--runs N] [--seed N] [--out DIR] [--no-minimize] [--json]
    x3-mc replay   <campaign.json> --index N [--master-seed N] [--json]
    x3-mc minimize <campaign.json> --index N [--master-seed N] [--json]
    x3-mc report   <directory>

The scenario is chosen by --scenario (default: atomic).";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() || args[0] == "--help" || args[0] == "-h" {
        println!("{USAGE}");
        return ExitCode::from(if args.is_empty() { 2 } else { 0 });
    }

    let command = args[0].clone();
    let rest = &args[1..];
    let result = match command.as_str() {
        "run" => command_run(rest),
        "replay" => command_replay(rest),
        "minimize" => command_minimize(rest),
        "report" => command_report(rest),
        other => Err(format!("unknown command `{other}`\n\n{USAGE}")),
    };

    match result {
        Ok(code) => code,
        Err(message) => {
            eprintln!("x3-mc: {message}");
            ExitCode::from(2)
        }
    }
}

/// A deliberately small flag reader: the CLI surface is four commands and
/// pulling in an argument parser would be more dependency than interface.
struct Flags {
    positional: Vec<String>,
    values: BTreeMap<String, String>,
    switches: Vec<String>,
}

impl Flags {
    fn parse(args: &[String]) -> Self {
        let mut positional = Vec::new();
        let mut values = BTreeMap::new();
        let mut switches = Vec::new();
        let mut index = 0;
        while index < args.len() {
            let arg = &args[index];
            if let Some(name) = arg.strip_prefix("--") {
                if let Some(inline) = name.split_once('=') {
                    values.insert(inline.0.to_string(), inline.1.to_string());
                } else if index + 1 < args.len() && !args[index + 1].starts_with("--") {
                    values.insert(name.to_string(), args[index + 1].clone());
                    index += 1;
                } else {
                    switches.push(name.to_string());
                }
            } else {
                positional.push(arg.clone());
            }
            index += 1;
        }
        Self {
            positional,
            values,
            switches,
        }
    }

    fn get(&self, key: &str) -> Option<&str> {
        self.values.get(key).map(String::as_str)
    }

    fn has(&self, key: &str) -> bool {
        self.switches.iter().any(|switch| switch == key)
    }

    fn number(&self, key: &str) -> Result<Option<u64>, String> {
        match self.get(key) {
            None => Ok(None),
            Some(text) => text
                .parse::<u64>()
                .map(Some)
                .map_err(|_| format!("--{key} expects an integer, found `{text}`")),
        }
    }

    fn campaign_path(&self) -> Result<PathBuf, String> {
        self.positional
            .first()
            .map(PathBuf::from)
            .ok_or_else(|| format!("a campaign file is required\n\n{USAGE}"))
    }
}

fn scenario_named(name: &str) -> Result<AtomicSettlement, String> {
    match name {
        "atomic" | "MC-X3-ATOMIC-001" => Ok(AtomicSettlement::new()),
        other => Err(format!(
            "scenario `{other}` is not implemented; available: atomic"
        )),
    }
}

/// Entropy for `seed: auto`. Read here, not inside the engine, and always
/// recorded, so an auto-seeded campaign is still replayable afterwards.
fn entropy() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_nanos() as u64)
        .unwrap_or(0)
}

fn environment() -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    map.insert(
        "profiles".to_string(),
        std::env::var("X3_MC_PROFILE").unwrap_or_else(|_| "local".to_string()),
    );
    map
}

fn command_run(args: &[String]) -> Result<ExitCode, String> {
    let flags = Flags::parse(args);
    let path = flags.campaign_path()?;
    let campaign = Campaign::load(&path)?;
    let scenario = scenario_named(flags.get("scenario").unwrap_or("atomic"))?;

    let resolved_seed = match flags.number("seed")? {
        Some(value) => value,
        None => campaign.seed(entropy())?,
    };
    let runs = flags
        .number("runs")?
        .unwrap_or(campaign.campaign.runs)
        .max(1);
    let config = CampaignConfig {
        runs,
        seed: resolved_seed,
        minimize: !flags.has("no-minimize"),
        minimize_budget: flags.number("minimize-budget")?.unwrap_or(200),
        max_failures: flags.number("max-failures")?.unwrap_or(0) as usize,
        abort_on: campaign.campaign.abort_on.clone(),
    };

    let started = Instant::now();
    let report = run_campaign(&scenario, &campaign, &config);
    let elapsed = started.elapsed();

    if let Some(directory) = flags.get("out") {
        let bundle = EvidenceBundle::create(&PathBuf::from(directory))?;
        bundle.write_all(&campaign, &report, resolved_seed, &environment())?;
    }

    if flags.has("json") {
        println!(
            "{}",
            serde_json::to_string_pretty(&report).unwrap_or_else(|_| "{}".to_string())
        );
    } else {
        print_report(&report, elapsed);
    }

    Ok(if report.passed() {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    })
}

fn print_report(report: &x3_mc::CampaignReport, elapsed: std::time::Duration) {
    let (observed, low, high) = report.failure_rate();
    println!("X3 MONTE CARLO — {}", report.campaign);
    println!("scenario                  {}", report.scenario);
    println!("master seed               {}", report.master_seed);
    println!(
        "runs                      {} of {} completed",
        report.runs_completed, report.runs_requested
    );
    println!(
        "distinct parameter points {}",
        report.distinct_parameter_points
    );
    println!("invariant violations      {}", report.failing_runs);
    println!("unique root causes        {}", report.unique_root_causes);
    println!("stop reason               {:?}", report.stop_reason);
    println!("elapsed                   {:.1}s", elapsed.as_secs_f64());
    println!(
        "observed failure rate     {observed:.6}  (95% interval {low:.6} .. {high:.6})"
    );

    if !report.coverage.violations.is_empty() {
        println!("\nviolations by code:");
        for (code, count) in &report.coverage.violations {
            println!("  {code:32} {count}");
        }
    }

    println!("\nparameter coverage (buckets hit of {}):", {
        x3_mc::engine::COVERAGE_BUCKETS
    });
    for (name, coverage) in &report.coverage.variables {
        println!(
            "  {name:24} {:>12.3} .. {:<12.3} mean {:>10.3}  {}/{}",
            coverage.observed_min,
            coverage.observed_max,
            coverage.mean,
            coverage.buckets_hit,
            coverage.buckets
        );
    }

    if !report.observations.is_empty() {
        // Proof the scenario actually happened: a campaign whose faults never
        // fired would also report zero violations.
        println!("\nmean observation per run:");
        for (name, mean) in &report.observations {
            println!("  {name:24} {mean:>12.4}");
        }
    }

    if !report.failures.is_empty() {
        println!("\nfirst failures:");
        for failure in report.failures.iter().take(5) {
            let code = failure
                .violations
                .first()
                .map(|violation| violation.code.as_str())
                .unwrap_or("UNKNOWN");
            println!(
                "  run {} seed {} {} :: {}",
                failure.run_index,
                failure.seed,
                code,
                failure
                    .violations
                    .first()
                    .map(|violation| violation.detail.as_str())
                    .unwrap_or("")
            );
            if let Some(minimized) = &failure.minimized {
                println!(
                    "    minimized in {} executions ({} reductions): {}",
                    minimized.executions,
                    minimized.reduced_variables,
                    serde_json::to_string(&minimized.params).unwrap_or_default()
                );
            }
        }
    }

    println!(
        "\nThis bounds the failure rate for the sampled distributions. It is not proof \
         that the rate is zero."
    );
}

fn command_replay(args: &[String]) -> Result<ExitCode, String> {
    let flags = Flags::parse(args);
    let path = flags.campaign_path()?;
    let campaign = Campaign::load(&path)?;
    let scenario = scenario_named(flags.get("scenario").unwrap_or("atomic"))?;
    let index = flags
        .number("index")?
        .ok_or("replay needs --index <run number>")?;
    let master = match flags.number("master-seed")? {
        Some(value) => value,
        None => campaign.seed(entropy())?,
    };

    let (params, seed, outcome, violations) = execute_one(&scenario, &campaign, index, master);
    if flags.has("json") {
        let body = serde_json::json!({
            "run_index": index,
            "master_seed": master,
            "run_seed": seed,
            "params": params,
            "violations": violations,
            "evidence": outcome.evidence,
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&body).unwrap_or_else(|_| "{}".to_string())
        );
    } else {
        println!("run {index} of {} (master seed {master})", campaign.campaign.name);
        println!("run seed  {seed}");
        println!("params    {}", serde_json::to_string(&params).unwrap_or_default());
        println!("outcome   {}", if violations.is_empty() { "held" } else { "VIOLATED" });
        for violation in &violations {
            println!("  {} :: {}", violation.code, violation.detail);
        }
        if let Some(replay) = outcome.evidence.get("replay").and_then(|v| v.as_str()) {
            println!("reproduce with the simulator directly:\n  {replay}");
        }
    }

    Ok(if violations.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    })
}

fn command_minimize(args: &[String]) -> Result<ExitCode, String> {
    let flags = Flags::parse(args);
    let path = flags.campaign_path()?;
    let campaign = Campaign::load(&path)?;
    let scenario = scenario_named(flags.get("scenario").unwrap_or("atomic"))?;
    let index = flags
        .number("index")?
        .ok_or("minimize needs --index <run number>")?;
    let master = match flags.number("master-seed")? {
        Some(value) => value,
        None => campaign.seed(entropy())?,
    };
    let budget = flags.number("minimize-budget")?.unwrap_or(400);

    let (params, seed, _, violations) = execute_one(&scenario, &campaign, index, master);
    if violations.is_empty() {
        return Err(format!(
            "run {index} did not violate an invariant, so there is nothing to minimize"
        ));
    }

    let result = minimize(&scenario, &params, seed, budget);
    if flags.has("json") {
        println!(
            "{}",
            serde_json::to_string_pretty(&result).unwrap_or_else(|_| "{}".to_string())
        );
    } else {
        println!("original   {}", serde_json::to_string(&params).unwrap_or_default());
        println!(
            "minimized  {}",
            serde_json::to_string(&result.params).unwrap_or_default()
        );
        println!(
            "executions {}   reductions {}",
            result.executions, result.reduced_variables
        );
        for violation in &result.violations {
            println!("  {} :: {}", violation.code, violation.detail);
        }
    }
    Ok(ExitCode::from(1))
}

fn command_report(args: &[String]) -> Result<ExitCode, String> {
    let flags = Flags::parse(args);
    let directory = flags
        .positional
        .first()
        .map(PathBuf::from)
        .ok_or("report needs a bundle directory")?;
    let path = directory.join("summary.json");
    let text = std::fs::read_to_string(&path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    let value: serde_json::Value =
        serde_json::from_str(&text).map_err(|error| format!("summary did not parse: {error}"))?;

    let get = |key: &str| value.get(key).cloned().unwrap_or(serde_json::Value::Null);
    println!("campaign        {}", get("campaign"));
    println!("runs completed  {}", get("runs_completed"));
    println!("failing runs    {}", get("failing_runs"));
    println!("root causes     {}", get("unique_root_causes"));
    println!("stop reason     {}", get("stop_reason"));
    println!("observed rate   {}", get("observed_failure_rate"));
    println!("95% interval    {}", get("failure_rate_95_interval"));
    println!("not covered     {}", get("interpretation")["not_covered_here"]);

    let failures = value
        .get("failures")
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or_default();
    Ok(if failures.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    })
}
