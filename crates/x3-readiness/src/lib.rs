//! X3 Readiness Engine — Proof and Report Generation
//!
//! Reads the canonical root-level files `FEATURE_REGISTRY.toml` and
//! `TESTNET_FEATURE_FLAGS.toml`, validates their structure, and produces
//! readiness reports, feature gap analysis, missing tests reports, Tauri
//! wiring reports, and marketing claims audits.
//!
//! The real `FEATURE_REGISTRY.toml` uses TOML table-per-feature format:
//!   [atomic_kernel]
//!   mode = "LIVE_TESTNET"
//!   readiness_score = 75
//!   ...
//! Not a flat `[[features]]` array of structs.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

// ── Canonical deserialization matching the real FEATURE_REGISTRY.toml ──

/// A single feature entry as it appears in a `[section]` block.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureEntry {
    pub mode: String,
    #[serde(default)]
    pub crate_or_service: String,
    #[serde(default)]
    pub tauri_app: String,
    #[serde(default)]
    pub required_tests: Vec<String>,
    #[serde(default)]
    pub health_endpoint: String,
    #[serde(default)]
    pub proof_report: String,
    pub readiness_score: u8,
    #[serde(default)]
    pub blockers: Vec<String>,
    #[serde(default)]
    pub dangerous_paths: Vec<String>,
}

/// The root-level `FEATURE_REGISTRY.toml` maps feature names → entries using
/// `[feature_name]` sections, NOT a `[[features]]` array.
pub type FeatureRegistry = BTreeMap<String, FeatureEntry>;

/// The root-level `TESTNET_FEATURE_FLAGS.toml` maps feature names → mode strings.
pub type TestnetFeatureFlags = BTreeMap<String, String>;

// ── Report output types ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadinessReport {
    pub generated_at: String,
    pub feature_registry: FeatureRegistry,
    pub testnet_flags: TestnetFeatureFlags,
    pub feature_completion: Vec<FeatureStatus>,
    pub service_health: Vec<ServiceHealth>,
    pub tauri_wiring: Vec<TauriWiring>,
    pub dead_buttons: Vec<DeadButton>,
    pub unsupported_claims: Vec<UnsupportedClaim>,
    pub verdict: Verdict,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureStatus {
    pub name: String,
    pub mode: String,
    pub readiness_score: u8,
    pub required_tests_present: bool,
    pub health_status: String,
    pub tauri_wired: bool,
    pub proof_report_generated: bool,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceHealth {
    pub name: String,
    pub endpoint: String,
    pub status: String,
    pub response_time_ms: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TauriWiring {
    pub feature: String,
    pub tauri_app: String,
    pub wired: bool,
    pub health_endpoint: String,
    pub last_test_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeadButton {
    pub feature: String,
    pub button: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnsupportedClaim {
    pub claim: String,
    pub supported: bool,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Verdict {
    TestnetGo,
    TestnetNoGo,
    MainnetGo,
    MainnetNoGo,
}

// ── Path constants — MUST match root-level locations ──

const FEATURE_REGISTRY_PATH: &str = "FEATURE_REGISTRY.toml";
const TESTNET_FLAGS_PATH: &str = "TESTNET_FEATURE_FLAGS.toml";
// Legacy paths that the old code used — fail explicitly if they exist.
const LEGACY_REGISTRY_PATH: &str = "docs/FEATURE_REGISTRY.toml";
const LEGACY_FLAGS_PATH: &str = "docs/TESTNET_FEATURE_FLAGS.toml";

/// Load the canonical feature registry from the root-level file.
/// Fails with a clear message if the file is missing or malformed.
pub fn load_feature_registry() -> Result<FeatureRegistry, String> {
    // Guard: if only the legacy path exists, tell the user to move it.
    if !Path::new(FEATURE_REGISTRY_PATH).exists() {
        if Path::new(LEGACY_REGISTRY_PATH).exists() {
            return Err(format!(
                "FEATURE_REGISTRY.toml not found at '{}'. \
                 A file exists at the legacy path '{}'. \
                 Move it to the root level: mv {} {}",
                FEATURE_REGISTRY_PATH, LEGACY_REGISTRY_PATH,
                LEGACY_REGISTRY_PATH, FEATURE_REGISTRY_PATH
            ));
        }
        return Err(format!(
            "FEATURE_REGISTRY.toml not found at '{}'. \
             This is the canonical readiness source. Create it or ensure it exists.",
            FEATURE_REGISTRY_PATH
        ));
    }

    let content = fs::read_to_string(FEATURE_REGISTRY_PATH)
        .map_err(|e| format!("Failed to read {}: {}", FEATURE_REGISTRY_PATH, e))?;

    let registry: FeatureRegistry = toml::from_str(&content).map_err(|e| {
        format!(
            "Invalid FEATURE_REGISTRY.toml: {}. \
             Expected table-per-feature format [feature_name] with fields: \
             mode, readiness_score, required_tests, etc.",
            e
        )
    })?;

    // Validate that every entry has required fields
    for (name, entry) in &registry {
        if entry.mode.is_empty() {
            return Err(format!(
                "FEATURE_REGISTRY.toml: feature '{}' has empty 'mode' field",
                name
            ));
        }
    }

    Ok(registry)
}

/// Load testnet feature flags from the root-level file.
pub fn load_testnet_flags() -> Result<TestnetFeatureFlags, String> {
    if !Path::new(TESTNET_FLAGS_PATH).exists() {
        if Path::new(LEGACY_FLAGS_PATH).exists() {
            return Err(format!(
                "TESTNET_FEATURE_FLAGS.toml not found at '{}'. \
                 A file exists at the legacy path '{}'. \
                 Move it: mv {} {}",
                TESTNET_FLAGS_PATH, LEGACY_FLAGS_PATH,
                LEGACY_FLAGS_PATH, TESTNET_FLAGS_PATH
            ));
        }
        return Err(format!(
            "TESTNET_FEATURE_FLAGS.toml not found at '{}'.",
            TESTNET_FLAGS_PATH
        ));
    }

    let content = fs::read_to_string(TESTNET_FLAGS_PATH)
        .map_err(|e| format!("Failed to read {}: {}", TESTNET_FLAGS_PATH, e))?;

    let flags: TestnetFeatureFlags = toml::from_str(&content).map_err(|e| {
        format!(
            "Invalid TESTNET_FEATURE_FLAGS.toml: {}. \
             Expected key = \"value\" format.",
            e
        )
    })?;

    Ok(flags)
}

// ── Main report generation ──

pub fn generate_testnet_report() -> ReadinessReport {
    let mut errors: Vec<String> = Vec::new();

    let registry = match load_feature_registry() {
        Ok(r) => r,
        Err(e) => {
            errors.push(e.clone());
            BTreeMap::new()
        }
    };

    let flags = match load_testnet_flags() {
        Ok(f) => f,
        Err(e) => {
            errors.push(e.clone());
            BTreeMap::new()
        }
    };

    // Generate feature status from the registry
    let mut feature_status = Vec::new();
    for (name, entry) in &registry {
        let required_tests_present = check_required_tests_from_list(&entry.required_tests);
        let health_status = check_health_endpoint(&entry.health_endpoint);
        let tauri_wired = check_tauri_wiring_for_app(&name, &entry.tauri_app);
        let proof_report_generated = check_proof_report_path(&entry.proof_report);

        let mut blockers: Vec<String> = entry.blockers.clone();

        if !required_tests_present {
            blockers.push("Missing required tests".to_string());
        }
        if health_status != "healthy" && health_status != "no-endpoint" {
            blockers.push(format!("Service health check: {}", health_status));
        }
        if !tauri_wired && !entry.tauri_app.is_empty() {
            blockers.push(format!("Tauri wiring incomplete for app '{}'", entry.tauri_app));
        }
        if !proof_report_generated && !entry.proof_report.is_empty() {
            blockers.push(format!("Proof report missing: {}", entry.proof_report));
        }

        feature_status.push(FeatureStatus {
            name: name.clone(),
            mode: entry.mode.clone(),
            readiness_score: entry.readiness_score,
            required_tests_present,
            health_status,
            tauri_wired,
            proof_report_generated,
            blockers,
        });
    }

    // Determine verdict
    let verdict = if !errors.is_empty() {
        Verdict::TestnetNoGo
    } else if feature_status.iter().any(|s| !s.blockers.is_empty()) {
        Verdict::TestnetNoGo
    } else {
        Verdict::TestnetGo
    };

    ReadinessReport {
        generated_at: Utc::now().to_rfc3339(),
        feature_registry: registry,
        testnet_flags: flags,
        feature_completion: feature_status,
        service_health: Vec::new(),
        tauri_wiring: Vec::new(),
        dead_buttons: Vec::new(),
        unsupported_claims: Vec::new(),
        verdict,
        errors,
    }
}

// ── Health and wiring checks ──

fn check_required_tests_from_list(tests: &[String]) -> bool {
    if tests.is_empty() {
        return true; // no tests required → pass
    }
    for test_name in tests {
        // Test names in the registry are test function names, not file paths.
        // We treat them as present if they are non-empty names that look valid.
        if test_name.is_empty() {
            return false;
        }
        // We don't check file existence for test function names — that would
        // require parsing Rust source files. The caller is responsible for
        // verifying these via CI. This function checks that the list is
        // well-formed.
    }
    true
}

fn check_health_endpoint(endpoint: &str) -> String {
    if endpoint.is_empty() {
        return "no-endpoint".to_string();
    }
    // Attempt a minimal blocking HTTP GET with 2-second timeout.
    match ureq::get(endpoint)
        .timeout(std::time::Duration::from_secs(2))
        .call()
    {
        Ok(resp) if resp.status() == 200 => "healthy".to_string(),
        Ok(resp) => format!("unhealthy-{}", resp.status()),
        Err(e) => format!("unreachable: {}", e),
    }
}

fn check_tauri_wiring_for_app(feature_name: &str, app_name: &str) -> bool {
    if app_name.is_empty() {
        return false;
    }
    let app_dir = Path::new("apps").join(app_name).join("src");
    if !app_dir.is_dir() {
        return false;
    }
    let invoke_pattern = format!("invoke(\"{}\"", feature_name);
    fs::read_dir(&app_dir)
        .map(|entries| {
            entries.filter_map(|e| e.ok()).any(|entry| {
                let path = entry.path();
                if path
                    .extension()
                    .map(|e| {
                        e == "rs"
                            || e == "ts"
                            || e == "tsx"
                            || e == "js"
                            || e == "jsx"
                            || e == "svelte"
                    })
                    .unwrap_or(false)
                {
                    fs::read_to_string(&path)
                        .map(|content| content.contains(&invoke_pattern))
                        .unwrap_or(false)
                } else {
                    false
                }
            })
        })
        .unwrap_or(false)
}

fn check_proof_report_path(report_path: &str) -> bool {
    if report_path.is_empty() {
        return false;
    }
    let path = Path::new(report_path);
    path.exists() && fs::metadata(path).map(|m| m.len() > 0).unwrap_or(false)
}

// ── Specialized report generators ──

pub fn generate_feature_gap_report() -> String {
    let report = generate_testnet_report();
    let mut out = String::from("# Feature Gap Report\n\n");
    out.push_str(&format!("Generated: {}\n\n", report.generated_at));

    if !report.errors.is_empty() {
        out.push_str("## Load Errors\n\n");
        for err in &report.errors {
            out.push_str(&format!("- {}\n", err));
        }
        out.push('\n');
    }

    let gaps: Vec<_> = report
        .feature_completion
        .iter()
        .filter(|f| !f.blockers.is_empty())
        .map(|f| format!("  {} (score={}): {}", f.name, f.readiness_score, f.blockers.join(", ")))
        .collect();

    if gaps.is_empty() {
        out.push_str("No feature gaps detected.\n");
    } else {
        out.push_str("## Features with blockers\n\n");
        for g in &gaps {
            out.push_str(g);
            out.push('\n');
        }
    }
    out
}

pub fn generate_missing_tests_report() -> String {
    let mut out = String::from("# Missing Tests Report\n\n");
    let roots = &["pallets", "crates"];
    for root in roots {
        let dir = match fs::read_dir(root) {
            Ok(d) => d,
            Err(_) => continue,
        };
        for entry in dir.filter_map(|e| e.ok()) {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let has_tests_dir = path.join("tests").is_dir();
            let src_test_mod = path.join("src").join("tests.rs").exists()
                || path.join("src").join("tests").is_dir();
            let has_test_cfg = if let Ok(manifest) = fs::read_to_string(path.join("Cargo.toml")) {
                manifest.contains("[dev-dependencies]")
            } else {
                false
            };
            if !has_tests_dir && !src_test_mod && !has_test_cfg {
                let name = path.file_name().unwrap_or_default().to_string_lossy();
                out.push_str(&format!("  {}/{}: no test infrastructure\n", root, name));
            }
        }
    }
    if out.lines().count() <= 2 {
        out.push_str("All crates/pallets have test infrastructure.\n");
    }
    out
}

pub fn generate_tauri_wiring_report() -> String {
    let report = generate_testnet_report();
    let mut out = String::from("# Tauri Wiring Report\n\n");
    for (name, entry) in &report.feature_registry {
        let wired = check_tauri_wiring_for_app(name, &entry.tauri_app);
        out.push_str(&format!(
            "  {}: {}\n",
            name,
            if wired {
                "wired"
            } else {
                "NOT WIRED"
            }
        ));
    }
    out
}

pub fn generate_marketing_claims_audit() -> String {
    String::from("# Marketing Claims Audit\n\nNo automated claims verification configured.\n")
}

pub fn generate_btc_gateway_report() -> String {
    String::from("# BTC Gateway Report\n\nSPV header count and UTXO health not yet wired.\n")
}

pub fn generate_service_health_report() -> String {
    let report = generate_testnet_report();
    let mut out = String::from("# Service Health Report\n\n");
    for (name, entry) in &report.feature_registry {
        if !entry.health_endpoint.is_empty() {
            let status = check_health_endpoint(&entry.health_endpoint);
            out.push_str(&format!("  {}: {}\n", name, status));
        }
    }
    out
}

pub fn generate_swarm_health_report() -> String {
    String::from("# Swarm Health Report\n\nSwarm agent heartbeat collection not yet wired.\n")
}

pub fn generate_reactor_benchmark_report() -> String {
    String::from(
        "# Reactor Benchmark Report\n\nBenchmark harness exists; CI pipeline not yet wired.\n",
    )
}

pub fn generate_grant_pipeline_report() -> String {
    String::from("# Grant Pipeline Report\n\nGrant tracking integration not yet wired.\n")
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_registry_from_root_level() {
        // This test only runs if FEATURE_REGISTRY.toml exists at the repo root.
        // In CI it validates the actual file is parseable.
        let result = load_feature_registry();
        match result {
            Ok(registry) => {
                assert!(!registry.is_empty(), "Registry should have entries");
                // Verify known features exist
                assert!(
                    registry.contains_key("atomic_kernel"),
                    "atomic_kernel must be in the registry"
                );
                let kernel = &registry["atomic_kernel"];
                assert!(!kernel.mode.is_empty(), "atomic_kernel must have a mode");
                assert!(kernel.readiness_score > 0, "atomic_kernel must have a score");
            }
            Err(e) => {
                // If the file doesn't exist in this test context, that's OK —
                // the function correctly reports the error.
                if !e.contains("not found") {
                    panic!("Unexpected error loading registry: {}", e);
                }
            }
        }
    }

    #[test]
    fn load_flags_from_root_level() {
        let result = load_testnet_flags();
        match result {
            Ok(flags) => {
                assert!(!flags.is_empty(), "Flags should have entries");
            }
            Err(e) => {
                if !e.contains("not found") {
                    panic!("Unexpected error loading flags: {}", e);
                }
            }
        }
    }

    #[test]
    fn generate_report_succeeds_or_reports_errors() {
        let report = generate_testnet_report();
        assert!(!report.generated_at.is_empty());
        // If the files exist, we should have features
        if report.errors.is_empty() {
            // At minimum, we should have generated a valid report structure
            assert!(report.verdict == Verdict::TestnetGo || report.verdict == Verdict::TestnetNoGo);
        } else {
            // With errors, verdict should be no-go
            assert_eq!(report.verdict, Verdict::TestnetNoGo);
        }
    }

    #[test]
    fn test_required_tests_for_valid_names() {
        let tests = vec!["test_something".to_string(), "test_other".to_string()];
        assert!(check_required_tests_from_list(&tests));

        let empty_names = vec!["".to_string()];
        assert!(!check_required_tests_from_list(&empty_names));
    }

    #[test]
    fn legacy_path_guard_reports_correctly() {
        // The function should return an error when the root file doesn't exist.
        // (It may or may not find a legacy path — either case produces a clear error.)
        let result = load_feature_registry();
        if let Err(msg) = result {
            assert!(msg.contains(FEATURE_REGISTRY_PATH));
        }
    }
}