//! X3 LaunchOps — truth engine for mainnet readiness.
//!
//! Commands:
//!   scan    — parse markdown, map evidence, write readiness.
//!   verify  — run configured commands, gates, red-flag scan.
//!   audit   — git drift detection, test weakening, conflicts, stale docs.
//!   full    — scan + verify + audit + final report.
//!   inventory-contracts — generate runtime/RPC inventory and contract matrix from live code.

mod audit;
mod classifier;
mod commands;
mod conflict;
mod contract;
mod drift;
mod evidence;
mod gates;
mod gitdiff;
mod inventory;
mod models;
mod parser;
mod report;
mod risk_rules;
mod scanner;
mod scoring;
mod stale_docs;
mod test_weaken;
mod verify;

use anyhow::{Context, Result};
use chrono::Utc;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::classifier::{classify_module, classify_risk};
use crate::evidence::{
    base_status_score, build_caches, derive_feature_status, find_evidence, risk_multiplier,
};
use crate::models::{
    ArtifactDescriptor, ArtifactFieldDoc, ArtifactManifest, ArtifactSchemaDoc, ArtifactSchemaIndex,
    AuditReport, BlockerItem, DriftSeverity, FeatureMatrixItem, FeatureStatus, GateResult,
    LaunchOpsConfig, Module, ReadinessMultipliers, Requirement, RequirementStatus, ScanStats,
    VerifyReport,
};
use crate::parser::parse_requirements_from_markdown;
use crate::report::{audit_report_md, final_report_md, scan_report_md, verify_report_md};
use crate::scanner::scan_markdown;
use crate::scoring::{
    build_readiness, command_multiplier_from, compute_scan_score, gate_multiplier_from,
    red_flag_multiplier_from,
};

const OUT_DIR: &str = ".launchops";
const ARTIFACT_SCHEMA_VERSION: &str = "launchops-artifacts/v1";
const ARTIFACT_FIELDS_SCHEMA_VERSION: &str = "launchops-artifact-fields/v1";

fn repo_root() -> PathBuf {
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

fn load_config(root: &Path) -> Result<LaunchOpsConfig> {
    let path = root.join(OUT_DIR).join("config.toml");
    if !path.exists() {
        return Ok(LaunchOpsConfig::default());
    }
    let raw =
        std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    let cfg: LaunchOpsConfig =
        toml::from_str(&raw).with_context(|| format!("parsing {}", path.display()))?;
    Ok(cfg)
}

fn ensure_out_dir(root: &Path) -> Result<PathBuf> {
    let out = root.join(OUT_DIR);
    std::fs::create_dir_all(&out).with_context(|| format!("creating {}", out.display()))?;
    Ok(out)
}

fn write_json<T: serde::Serialize>(path: &Path, value: &T) -> Result<()> {
    let data = serde_json::to_string_pretty(value)? + "\n";
    std::fs::write(path, data).with_context(|| format!("writing {}", path.display()))?;
    Ok(())
}

fn write_text(path: &Path, value: &str) -> Result<()> {
    std::fs::write(path, value).with_context(|| format!("writing {}", path.display()))?;
    Ok(())
}

fn write_artifact_manifest(root: &Path) -> Result<()> {
    let out_dir = root.join(OUT_DIR);
    let schema_file = "artifact_schemas.json";
    let known = [
        ("readiness.json", "readiness", "json"),
        ("blockers.json", "blockers", "json"),
        ("feature_matrix.json", "feature_matrix", "json"),
        ("report.md", "scan_report", "markdown"),
        ("test_results.json", "test_results", "json"),
        ("gates.json", "gates", "json"),
        ("red_flags.json", "red_flags", "json"),
        ("verify_report.md", "verify_report", "markdown"),
        ("drift_report.json", "drift_report", "json"),
        ("deep_red_flags.json", "deep_red_flags", "json"),
        (
            "requirement_conflicts.json",
            "requirement_conflicts",
            "json",
        ),
        ("stale_docs.json", "stale_docs", "json"),
        (
            "runtime_rpc_inventory.json",
            "runtime_rpc_inventory",
            "json",
        ),
        ("rpc_contract_matrix.json", "rpc_contract_matrix", "json"),
        (
            "rpc_contract_matrix.md",
            "rpc_contract_matrix_report",
            "markdown",
        ),
        (
            "rpc_consumer_contracts.json",
            "rpc_consumer_contracts",
            "json",
        ),
        (
            "rpc_consumer_contracts.md",
            "rpc_consumer_contracts_report",
            "markdown",
        ),
        (
            "frontend_route_allowlist.json",
            "frontend_route_allowlist",
            "json",
        ),
        (
            "frontend_route_allowlist.md",
            "frontend_route_allowlist_report",
            "markdown",
        ),
        (
            "sidecar_adapter_backlog.json",
            "sidecar_adapter_backlog",
            "json",
        ),
        (
            "sidecar_adapter_backlog.md",
            "sidecar_adapter_backlog_report",
            "markdown",
        ),
        ("audit_report.md", "audit_report", "markdown"),
        ("final_report.md", "final_report", "markdown"),
    ];

    let mut artifacts = Vec::new();
    for (file, kind, format) in known {
        if out_dir.join(file).exists() {
            artifacts.push(ArtifactDescriptor {
                file: file.to_string(),
                kind: kind.to_string(),
                format: format.to_string(),
                schema_version: ARTIFACT_SCHEMA_VERSION.to_string(),
                schema_ref: Some(format!("{}#{}", schema_file, kind)),
            });
        }
    }
    if out_dir.join(schema_file).exists() {
        artifacts.push(ArtifactDescriptor {
            file: schema_file.to_string(),
            kind: "artifact_schemas".to_string(),
            format: "json".to_string(),
            schema_version: ARTIFACT_FIELDS_SCHEMA_VERSION.to_string(),
            schema_ref: None,
        });
    }
    artifacts.sort_by(|a, b| a.file.cmp(&b.file));

    let manifest = ArtifactManifest {
        schema_version: ARTIFACT_SCHEMA_VERSION.to_string(),
        generated_at: Utc::now().to_rfc3339(),
        artifacts,
    };
    write_json(&out_dir.join("artifact_manifest.json"), &manifest)
}

fn field(name: &str, type_name: &str, required: bool, description: &str) -> ArtifactFieldDoc {
    ArtifactFieldDoc {
        name: name.to_string(),
        type_name: type_name.to_string(),
        required,
        description: description.to_string(),
        enum_values: Vec::new(),
        nested_fields: Vec::new(),
    }
}

fn enum_field(
    name: &str,
    type_name: &str,
    required: bool,
    description: &str,
    enum_values: &[&str],
) -> ArtifactFieldDoc {
    ArtifactFieldDoc {
        name: name.to_string(),
        type_name: type_name.to_string(),
        required,
        description: description.to_string(),
        enum_values: enum_values.iter().map(|v| (*v).to_string()).collect(),
        nested_fields: Vec::new(),
    }
}

fn object_field(
    name: &str,
    type_name: &str,
    required: bool,
    description: &str,
    nested_fields: Vec<ArtifactFieldDoc>,
) -> ArtifactFieldDoc {
    ArtifactFieldDoc {
        name: name.to_string(),
        type_name: type_name.to_string(),
        required,
        description: description.to_string(),
        enum_values: Vec::new(),
        nested_fields,
    }
}

fn artifact_schema_docs() -> Vec<ArtifactSchemaDoc> {
    let drift_levels = &["low", "medium", "high", "critical"];
    let modules = &[
        "consensus",
        "cross_vm",
        "bridge",
        "universal_asset_kernel",
        "dex",
        "gpu_validator",
        "wallet_explorer",
        "launch_ops",
        "security",
        "ops",
        "docs",
        "unknown",
    ];
    let feature_statuses = &[
        "missing",
        "specified_only",
        "partial",
        "implemented",
        "tested",
        "verified",
        "blocked",
    ];
    let risk_levels = &["low", "medium", "high", "critical"];
    let command_statuses = &["passed", "failed", "skipped", "missing_tool"];
    let gate_statuses = &["pass", "fail", "warn", "skipped"];

    let mut docs = vec![
        ArtifactSchemaDoc {
            file: "report.md".to_string(),
            kind: "scan_report".to_string(),
            format: "markdown".to_string(),
            shape: "text/markdown".to_string(),
            fields: vec![],
        },
        ArtifactSchemaDoc {
            file: "verify_report.md".to_string(),
            kind: "verify_report".to_string(),
            format: "markdown".to_string(),
            shape: "text/markdown".to_string(),
            fields: vec![],
        },
        ArtifactSchemaDoc {
            file: "audit_report.md".to_string(),
            kind: "audit_report".to_string(),
            format: "markdown".to_string(),
            shape: "text/markdown".to_string(),
            fields: vec![],
        },
        ArtifactSchemaDoc {
            file: "final_report.md".to_string(),
            kind: "final_report".to_string(),
            format: "markdown".to_string(),
            shape: "text/markdown".to_string(),
            fields: vec![],
        },
        ArtifactSchemaDoc {
            file: "rpc_contract_matrix.md".to_string(),
            kind: "rpc_contract_matrix_report".to_string(),
            format: "markdown".to_string(),
            shape: "text/markdown".to_string(),
            fields: vec![],
        },
        ArtifactSchemaDoc {
            file: "rpc_consumer_contracts.md".to_string(),
            kind: "rpc_consumer_contracts_report".to_string(),
            format: "markdown".to_string(),
            shape: "text/markdown".to_string(),
            fields: vec![],
        },
        ArtifactSchemaDoc {
            file: "frontend_route_allowlist.md".to_string(),
            kind: "frontend_route_allowlist_report".to_string(),
            format: "markdown".to_string(),
            shape: "text/markdown".to_string(),
            fields: vec![],
        },
        ArtifactSchemaDoc {
            file: "sidecar_adapter_backlog.md".to_string(),
            kind: "sidecar_adapter_backlog_report".to_string(),
            format: "markdown".to_string(),
            shape: "text/markdown".to_string(),
            fields: vec![],
        },
        ArtifactSchemaDoc {
            file: "readiness.json".to_string(),
            kind: "readiness".to_string(),
            format: "json".to_string(),
            shape: "object".to_string(),
            fields: vec![
                field("generated_at", "string(datetime)", true, "UTC timestamp when readiness was computed."),
                field("scan_score", "number", true, "Readiness score from markdown and evidence scanning before runtime multipliers."),
                field("final_readiness", "number", true, "Final readiness score after verify and audit multipliers are applied."),
                field("status", "string", true, "Overall readiness label such as BLOCKED or READY-like status."),
                field("module_breakdown", "object<string,number>", true, "Per-module readiness percentages."),
                object_field(
                    "totals",
                    "object",
                    true,
                    "Aggregated markdown and requirement counts from scan.",
                    vec![
                        field("total_md_files", "integer", true, "Markdown files scanned after include/exclude filtering."),
                        field("total_requirements", "integer", true, "Deduplicated requirement count used in scoring."),
                        field("complete_items", "integer", true, "Count of requirements marked complete."),
                        field("partial_items", "integer", true, "Count of requirements marked partial."),
                        field("blocked_items", "integer", true, "Count of requirements explicitly marked blocker."),
                        field("needs_test_items", "integer", true, "Count of requirements marked as needing tests."),
                        field("raw_requirements", "integer", true, "Pre-dedup total requirement count parsed from markdown."),
                        field("duplicates_collapsed", "integer", true, "Number of duplicate requirements removed during deduplication."),
                    ],
                ),
                object_field(
                    "multipliers",
                    "object",
                    true,
                    "Verify and audit multiplier inputs used to derive final readiness.",
                    vec![
                        field("command_multiplier", "number", true, "Penalty derived from configured command execution results."),
                        field("gate_multiplier", "number", true, "Penalty derived from required gate failures."),
                        field("red_flag_multiplier", "number", true, "Penalty derived from red-flag severity counts."),
                        field("audit_multiplier", "number", true, "Penalty derived from drift and audit findings."),
                    ],
                ),
            ],
        },
        ArtifactSchemaDoc {
            file: "blockers.json".to_string(),
            kind: "blockers".to_string(),
            format: "json".to_string(),
            shape: "array<object>".to_string(),
            fields: vec![
                enum_field("severity", "string", true, "Blocker severity level.", drift_levels),
                field("id", "string", true, "Normalized requirement identifier."),
                enum_field("module", "string", true, "Classified module for the blocker.", modules),
                field("source_file", "string", true, "Markdown source file for the blocker requirement."),
                field("line", "integer", true, "1-based line number in the source file."),
                field("reason", "string", true, "Human-readable blocker explanation."),
            ],
        },
        ArtifactSchemaDoc {
            file: "feature_matrix.json".to_string(),
            kind: "feature_matrix".to_string(),
            format: "json".to_string(),
            shape: "array<object>".to_string(),
            fields: vec![
                field("feature", "string", true, "Normalized feature or requirement id."),
                enum_field("module", "string", true, "Classified module for this feature.", modules),
                field("source_file", "string", true, "Markdown source file where the feature requirement was parsed."),
                enum_field("status", "string", true, "Derived feature implementation status.", feature_statuses),
                enum_field("risk", "string", true, "Assigned risk level for the feature.", risk_levels),
                field("code_evidence", "array<string>", true, "Matching code file paths or snippets supporting implementation evidence."),
                field("test_evidence", "array<string>", true, "Matching test file paths or snippets supporting validation evidence."),
                field("score", "integer", true, "Risk-adjusted feature score."),
            ],
        },
        ArtifactSchemaDoc {
            file: "test_results.json".to_string(),
            kind: "test_results".to_string(),
            format: "json".to_string(),
            shape: "array<object>".to_string(),
            fields: vec![
                field("name", "string", true, "Configured command id."),
                field("command", "string", true, "Shell command executed."),
                enum_field("status", "string", true, "Execution outcome for the command.", command_statuses),
                field("exit_code", "integer|null", true, "Exit code when the command was executed."),
                field("duration_ms", "integer", true, "Execution duration in milliseconds."),
                field("stdout_excerpt", "string", true, "Bounded excerpt of stdout for diagnostics."),
                field("stderr_excerpt", "string", true, "Bounded excerpt of stderr for diagnostics."),
            ],
        },
        ArtifactSchemaDoc {
            file: "gates.json".to_string(),
            kind: "gates".to_string(),
            format: "json".to_string(),
            shape: "array<object>".to_string(),
            fields: vec![
                field("id", "string", true, "Stable gate identifier."),
                field("name", "string", true, "Human-readable gate name."),
                enum_field("status", "string", true, "Gate evaluation result.", gate_statuses),
                field("required", "boolean", true, "Whether this gate blocks readiness when failing."),
                field("weight", "integer", true, "Configured gate weight."),
                field("source", "string", true, "Primary evaluation source such as a command or artifact name."),
                field("reason", "string|null", true, "Optional failure or warning explanation."),
            ],
        },
        ArtifactSchemaDoc {
            file: "red_flags.json".to_string(),
            kind: "red_flags".to_string(),
            format: "json".to_string(),
            shape: "array<object>".to_string(),
            fields: vec![
                enum_field("severity", "string", true, "Red-flag severity level.", risk_levels),
                field("file", "string", true, "File containing the risky pattern."),
                field("line", "integer", true, "1-based line number containing the match."),
                field("pattern", "string", true, "Detected risky token or expression."),
                field("reason", "string", true, "Human-readable explanation of why the pattern matters."),
            ],
        },
        ArtifactSchemaDoc {
            file: "drift_report.json".to_string(),
            kind: "drift_report".to_string(),
            format: "json".to_string(),
            shape: "object".to_string(),
            fields: vec![
                field("baseline_branch", "string", true, "Git baseline branch used for drift comparison."),
                object_field(
                    "changed_files",
                    "object",
                    true,
                    "Changed file sets grouped by category.",
                    vec![
                        field("docs", "array<string>", true, "Changed markdown and documentation files."),
                        field("code", "array<string>", true, "Changed production code files."),
                        field("tests", "array<string>", true, "Changed test files."),
                        field("consensus", "array<string>", true, "Changed consensus-critical files."),
                        field("bridge", "array<string>", true, "Changed bridge or relayer files."),
                        field("cross_vm", "array<string>", true, "Changed cross-VM files."),
                        field("dex", "array<string>", true, "Changed DEX files."),
                        field("gpu", "array<string>", true, "Changed GPU validator files."),
                        field("ops", "array<string>", true, "Changed operational or launch files."),
                        field("mainnet_config", "array<string>", true, "Changed genesis, chain spec, or mainnet config files."),
                    ],
                ),
                field("drift_flags", "array<object>", true, "Detected drift findings from the git diff and evidence rules."),
            ],
        },
        ArtifactSchemaDoc {
            file: "deep_red_flags.json".to_string(),
            kind: "deep_red_flags".to_string(),
            format: "json".to_string(),
            shape: "array<object>".to_string(),
            fields: vec![
                enum_field("severity", "string", true, "Deep audit severity level.", drift_levels),
                field("flag_type", "string", true, "Stable drift or audit flag type identifier."),
                field("files", "array<string>", true, "Files implicated by the audit finding."),
                field("reason", "string", true, "Human-readable explanation of the audit finding."),
                field("required_evidence", "array<string>", true, "Evidence expected to clear or justify the finding."),
            ],
        },
        ArtifactSchemaDoc {
            file: "requirement_conflicts.json".to_string(),
            kind: "requirement_conflicts".to_string(),
            format: "json".to_string(),
            shape: "array<object>".to_string(),
            fields: vec![
                enum_field("severity", "string", true, "Conflict severity level.", drift_levels),
                field("conflict_type", "string", true, "Conflict classifier such as conflicting_requirement."),
                object_field(
                    "requirement_a",
                    "object",
                    true,
                    "First requirement involved in the conflict.",
                    vec![
                        field("file", "string", true, "Source markdown file for the first requirement."),
                        field("line", "integer", true, "1-based line number for the first requirement."),
                        field("text", "string", true, "Original markdown requirement text."),
                        field("status", "string", true, "Serialized requirement status label."),
                        field("module", "string", true, "Serialized module label for the first requirement."),
                    ],
                ),
                object_field(
                    "requirement_b",
                    "object",
                    true,
                    "Second requirement involved in the conflict.",
                    vec![
                        field("file", "string", true, "Source markdown file for the second requirement."),
                        field("line", "integer", true, "1-based line number for the second requirement."),
                        field("text", "string", true, "Original markdown requirement text."),
                        field("status", "string", true, "Serialized requirement status label."),
                        field("module", "string", true, "Serialized module label for the second requirement."),
                    ],
                ),
                field("reason", "string", true, "Human-readable conflict explanation."),
            ],
        },
        ArtifactSchemaDoc {
            file: "stale_docs.json".to_string(),
            kind: "stale_docs".to_string(),
            format: "json".to_string(),
            shape: "array<object>".to_string(),
            fields: vec![
                field("file", "string", true, "Markdown file considered stale."),
                field("linked_code", "array<string>", true, "Code files linked to the document by evidence search."),
                enum_field("severity", "string", true, "Staleness severity level.", drift_levels),
                field("reason", "string", true, "Human-readable explanation of why the doc is stale."),
            ],
        },
        ArtifactSchemaDoc {
            file: "runtime_rpc_inventory.json".to_string(),
            kind: "runtime_rpc_inventory".to_string(),
            format: "json".to_string(),
            shape: "object".to_string(),
            fields: vec![
                field("generated_at", "string(datetime)", true, "UTC timestamp when the inventory was generated."),
                field("runtime_source_file", "string", true, "Runtime source file parsed for impl_runtime_apis traits."),
                field("rpc_source_file", "string", true, "Node RPC source file paired with the runtime inventory."),
                object_field(
                    "runtime_traits",
                    "array<object>",
                    true,
                    "Runtime API trait impls discovered in live code.",
                    vec![
                        field("trait_name", "string", true, "Fully qualified runtime API trait name as parsed from the impl block."),
                        field("source_file", "string", true, "Relative source file containing the impl block."),
                        field("impl_line", "integer", true, "1-based line number where the impl block starts."),
                        field("cfg_guard", "string|null", false, "Optional cfg guard directly attached to the impl block."),
                        object_field(
                            "methods",
                            "array<object>",
                            true,
                            "Methods discovered inside the runtime API impl block.",
                            vec![
                                field("name", "string", true, "Method name."),
                                field("line", "integer", true, "1-based line number where the method starts."),
                            ],
                        ),
                    ],
                ),
            ],
        },
        ArtifactSchemaDoc {
            file: "rpc_contract_matrix.json".to_string(),
            kind: "rpc_contract_matrix".to_string(),
            format: "json".to_string(),
            shape: "object".to_string(),
            fields: vec![
                field("generated_at", "string(datetime)", true, "UTC timestamp when the RPC contract matrix was generated."),
                field("rpc_source_file", "string", true, "Node RPC source file parsed for register_method calls."),
                field("runtime_backed_count", "integer", true, "Number of RPC methods classified as runtime-backed."),
                field("node_local_adapter_count", "integer", true, "Number of RPC methods classified as node-local adapters."),
                field("placeholder_count", "integer", true, "Number of RPC methods classified as placeholders."),
                field("duplicate_registration_count", "integer", true, "Number of duplicate RPC registration flags in the matrix."),
                field("bucket_drift_count", "integer", true, "Number of bucket drift flags where runtime-backed behavior is mixed with node-local ownership."),
                object_field(
                    "flags",
                    "array<object>",
                    true,
                    "Matrix-level anomaly flags such as duplicate registrations or bucket drift.",
                    vec![
                        enum_field(
                            "category",
                            "string",
                            true,
                            "Flag category.",
                            &["duplicate_registration", "bucket_drift"],
                        ),
                        enum_field(
                            "severity",
                            "string",
                            true,
                            "Flag severity.",
                            &["warn", "high"],
                        ),
                        field("method", "string", true, "RPC method name attached to the flag."),
                        field("line_refs", "array<string>", true, "Source file and line references related to the flag."),
                        field("reason", "string", true, "Human-readable explanation of the anomaly."),
                    ],
                ),
                object_field(
                    "methods",
                    "array<object>",
                    true,
                    "Per-method contract classification for frontend and sidecar consumers.",
                    vec![
                        field("method", "string", true, "RPC method name."),
                        field("source_file", "string", true, "Relative source file containing the method registration."),
                        field("line", "integer", true, "1-based line number of the register_method call."),
                        enum_field(
                            "bucket",
                            "string",
                            true,
                            "Contract ownership bucket.",
                            &["runtime_backed", "node_local_adapter", "placeholder"],
                        ),
                        field("runtime_calls", "array<string>", true, "Runtime API method calls detected inside the RPC handler."),
                        field("runtime_trait_hints", "array<string>", true, "Runtime API trait impls that own the detected runtime calls."),
                        field("node_local_signals", "array<string>", true, "Detected node-local services or orchestration signals used by the handler."),
                        field("placeholder_reason", "string|null", false, "Reason the handler is considered a placeholder when applicable."),
                        field("ownership_note", "string|null", false, "Optional note when mixed ownership is expected rather than anomalous."),
                        field("frontend_consumer_mode", "string", true, "Frontend integration guidance derived from the bucket and method shape."),
                        field("sidecar_consumer_mode", "string", true, "Sidecar integration guidance derived from the bucket and method shape."),
                    ],
                ),
            ],
        },
        ArtifactSchemaDoc {
            file: "rpc_consumer_contracts.json".to_string(),
            kind: "rpc_consumer_contracts".to_string(),
            format: "json".to_string(),
            shape: "object".to_string(),
            fields: vec![
                field("generated_at", "string(datetime)", true, "UTC timestamp when the consumer contract split was generated."),
                field("source_matrix_file", "string", true, "Source LaunchOps matrix artifact used to derive the consumer split."),
                field("frontend_safe_count", "integer", true, "Number of methods considered safe for direct frontend reads."),
                field("sidecar_only_count", "integer", true, "Number of methods that should remain behind sidecar or adapter ownership."),
                field("mock_only_count", "integer", true, "Number of methods that remain mock-only or deferred."),
                object_field(
                    "frontend_safe_methods",
                    "array<object>",
                    true,
                    "Methods currently considered safe for direct frontend reads.",
                    vec![
                        field("method", "string", true, "RPC method name."),
                        field("registration_count", "integer", true, "Number of raw register_method entries collapsed into this consumer contract entry."),
                        enum_field(
                            "bucket",
                            "string",
                            true,
                            "Most conservative ownership bucket after duplicate collapse.",
                            &["runtime_backed", "node_local_adapter", "placeholder"],
                        ),
                        field("frontend_consumer_mode", "string", true, "Frontend integration mode."),
                        field("sidecar_consumer_mode", "string", true, "Sidecar integration mode."),
                        field("ownership_note", "string|null", false, "Optional note when mixed ownership is expected rather than anomalous."),
                        field("runtime_trait_hints", "array<string>", true, "Runtime API trait hints preserved after duplicate collapse."),
                        field("node_local_signals", "array<string>", true, "Node-local signals preserved after duplicate collapse."),
                        field("notes", "array<string>", true, "Conservative notes attached to the consumer contract entry."),
                    ],
                ),
                object_field(
                    "sidecar_only_methods",
                    "array<object>",
                    true,
                    "Methods that should remain behind sidecar or adapter ownership.",
                    vec![
                        field("method", "string", true, "RPC method name."),
                        field("registration_count", "integer", true, "Number of raw register_method entries collapsed into this consumer contract entry."),
                        enum_field(
                            "bucket",
                            "string",
                            true,
                            "Most conservative ownership bucket after duplicate collapse.",
                            &["runtime_backed", "node_local_adapter", "placeholder"],
                        ),
                        field("frontend_consumer_mode", "string", true, "Frontend integration mode."),
                        field("sidecar_consumer_mode", "string", true, "Sidecar integration mode."),
                        field("ownership_note", "string|null", false, "Optional note when mixed ownership is expected rather than anomalous."),
                        field("runtime_trait_hints", "array<string>", true, "Runtime API trait hints preserved after duplicate collapse."),
                        field("node_local_signals", "array<string>", true, "Node-local signals preserved after duplicate collapse."),
                        field("notes", "array<string>", true, "Conservative notes attached to the consumer contract entry."),
                    ],
                ),
                object_field(
                    "mock_only_methods",
                    "array<object>",
                    true,
                    "Methods that remain mock-only or deferred in the current build.",
                    vec![
                        field("method", "string", true, "RPC method name."),
                        field("registration_count", "integer", true, "Number of raw register_method entries collapsed into this consumer contract entry."),
                        enum_field(
                            "bucket",
                            "string",
                            true,
                            "Most conservative ownership bucket after duplicate collapse.",
                            &["runtime_backed", "node_local_adapter", "placeholder"],
                        ),
                        field("frontend_consumer_mode", "string", true, "Frontend integration mode."),
                        field("sidecar_consumer_mode", "string", true, "Sidecar integration mode."),
                        field("ownership_note", "string|null", false, "Optional note when mixed ownership is expected rather than anomalous."),
                        field("runtime_trait_hints", "array<string>", true, "Runtime API trait hints preserved after duplicate collapse."),
                        field("node_local_signals", "array<string>", true, "Node-local signals preserved after duplicate collapse."),
                        field("notes", "array<string>", true, "Conservative notes attached to the consumer contract entry."),
                    ],
                ),
            ],
        },
        ArtifactSchemaDoc {
            file: "frontend_route_allowlist.json".to_string(),
            kind: "frontend_route_allowlist".to_string(),
            format: "json".to_string(),
            shape: "object".to_string(),
            fields: vec![
                field("generated_at", "string(datetime)", true, "UTC timestamp when the route allowlist was generated."),
                field("source_consumer_contracts_file", "string", true, "Source consumer contracts artifact used to derive the allowlist."),
                object_field(
                    "routes",
                    "array<object>",
                    true,
                    "Per-route direct-read RPC allowlist for the disposable frontend shell.",
                    vec![
                        field("route_id", "string", true, "Frontend shell route id."),
                        field("route_label", "string", true, "Frontend shell route label."),
                        field("rationale", "string", true, "Explanation for why the allowlist is scoped the way it is."),
                        field("allowed_methods", "array<string>", true, "Direct-read RPC methods allowed for that route."),
                    ],
                ),
            ],
        },
        ArtifactSchemaDoc {
            file: "sidecar_adapter_backlog.json".to_string(),
            kind: "sidecar_adapter_backlog".to_string(),
            format: "json".to_string(),
            shape: "object".to_string(),
            fields: vec![
                field("generated_at", "string(datetime)", true, "UTC timestamp when the sidecar adapter backlog was generated."),
                field("source_consumer_contracts_file", "string", true, "Source consumer contracts artifact used to derive the backlog."),
                object_field(
                    "backlog",
                    "array<object>",
                    true,
                    "Sidecar-owned RPC methods grouped against the frontend route they block or support.",
                    vec![
                        field("route_id", "string", true, "Frontend shell route id most directly affected by the missing sidecar adapter."),
                        field("route_label", "string", true, "Frontend shell route label."),
                        field("method", "string", true, "RPC method that remains sidecar-owned."),
                        field("backlog_reason", "string", true, "Human-readable reason this method belongs in the sidecar backlog."),
                        field("ownership_note", "string|null", false, "Optional note when mixed ownership is expected rather than anomalous."),
                        field("notes", "array<string>", true, "Additional carry-forward notes from the consumer contract entry."),
                    ],
                ),
            ],
        },
    ];
    docs.sort_by(|a, b| a.file.cmp(&b.file));
    docs
}

fn write_artifact_schemas(root: &Path) -> Result<()> {
    let out_dir = root.join(OUT_DIR);
    let index = ArtifactSchemaIndex {
        schema_version: ARTIFACT_FIELDS_SCHEMA_VERSION.to_string(),
        generated_at: Utc::now().to_rfc3339(),
        artifacts: artifact_schema_docs(),
    };
    write_json(&out_dir.join("artifact_schemas.json"), &index)
}

fn write_artifact_contract(root: &Path) -> Result<()> {
    write_artifact_schemas(root)?;
    write_artifact_manifest(root)
}

fn do_inventory_contracts(root: &Path) -> Result<()> {
    let out_dir = ensure_out_dir(root)?;
    let generated_at = Utc::now().to_rfc3339();
    let outputs = crate::inventory::generate_inventory(root, &generated_at)?;

    write_json(
        &out_dir.join("runtime_rpc_inventory.json"),
        &outputs.runtime_inventory,
    )?;
    write_json(
        &out_dir.join("rpc_contract_matrix.json"),
        &outputs.contract_matrix,
    )?;
    write_text(
        &out_dir.join("rpc_contract_matrix.md"),
        &outputs.contract_matrix_md,
    )?;
    write_json(
        &out_dir.join("rpc_consumer_contracts.json"),
        &outputs.consumer_contracts,
    )?;
    write_text(
        &out_dir.join("rpc_consumer_contracts.md"),
        &outputs.consumer_contracts_md,
    )?;
    write_json(
        &out_dir.join("frontend_route_allowlist.json"),
        &outputs.frontend_route_allowlist,
    )?;
    write_text(
        &out_dir.join("frontend_route_allowlist.md"),
        &outputs.frontend_route_allowlist_md,
    )?;
    write_json(
        &out_dir.join("sidecar_adapter_backlog.json"),
        &outputs.sidecar_adapter_backlog,
    )?;
    write_text(
        &out_dir.join("sidecar_adapter_backlog.md"),
        &outputs.sidecar_adapter_backlog_md,
    )?;
    write_artifact_contract(root)?;

    println!(
        "[inventory-contracts] runtime_traits={} rpc_methods={} runtime_backed={} node_local_adapter={} placeholder={} frontend_safe={} sidecar_only={} mock_only={} duplicate_registrations={} bucket_drift={} routes={} backlog={}",
        outputs.runtime_inventory.runtime_traits.len(),
        outputs.contract_matrix.methods.len(),
        outputs.contract_matrix.runtime_backed_count,
        outputs.contract_matrix.node_local_adapter_count,
        outputs.contract_matrix.placeholder_count,
        outputs.consumer_contracts.frontend_safe_count,
        outputs.consumer_contracts.sidecar_only_count,
        outputs.consumer_contracts.mock_only_count,
        outputs.contract_matrix.duplicate_registration_count,
        outputs.contract_matrix.bucket_drift_count,
        outputs.frontend_route_allowlist.routes.len(),
        outputs.sidecar_adapter_backlog.backlog.len(),
    );

    Ok(())
}

/// Parse all markdown into requirements and produce scan-time stats.
fn collect_requirements(
    root: &Path,
    cfg: &LaunchOpsConfig,
) -> Result<(Vec<Requirement>, ScanStats)> {
    let inputs = scan_markdown(root, &cfg.scan)?;
    let mut out: Vec<Requirement> = Vec::new();
    let mut stats = ScanStats::default();
    stats.total_md_files = inputs.files.len();

    for path in inputs.files {
        let rel = match path.strip_prefix(root) {
            Ok(p) => p.to_string_lossy().into_owned(),
            Err(_) => path.to_string_lossy().into_owned(),
        };
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let mut reqs = parse_requirements_from_markdown(&rel, &content);
        for r in reqs.iter_mut() {
            r.module = classify_module(&r.text, &r.tags);
            r.risk = classify_risk(r);
        }
        out.extend(reqs);
    }

    stats.total_requirements = out.len();
    // Dedup requirements by id — many docs duplicate checklists across the repo.
    // Keep the "worst" status (most pressing) and merge tags.
    let raw_count = out.len();
    if raw_count > 0 {
        use std::collections::HashMap;
        fn status_severity(s: &RequirementStatus) -> u8 {
            match s {
                RequirementStatus::Complete => 0,
                RequirementStatus::Partial => 1,
                RequirementStatus::Incomplete => 2,
                RequirementStatus::NeedsTest => 3,
                RequirementStatus::Blocker => 4,
            }
        }
        let mut by_id: HashMap<String, Requirement> = HashMap::with_capacity(raw_count);
        for r in out.drain(..) {
            match by_id.get_mut(&r.id) {
                None => {
                    by_id.insert(r.id.clone(), r);
                }
                Some(existing) => {
                    if status_severity(&r.status) > status_severity(&existing.status) {
                        existing.status = r.status.clone();
                        existing.source_file = r.source_file.clone();
                        existing.line = r.line;
                    }
                    for t in r.tags {
                        if !existing.tags.contains(&t) {
                            existing.tags.push(t);
                        }
                    }
                }
            }
        }
        out = by_id.into_values().collect();
        out.sort_by(|a, b| a.id.cmp(&b.id));
    }
    stats.raw_requirements = raw_count;
    stats.duplicates_collapsed = raw_count.saturating_sub(out.len());
    stats.total_requirements = out.len();
    // Recompute status totals after dedup.
    stats.complete_items = 0;
    stats.partial_items = 0;
    stats.blocked_items = 0;
    stats.needs_test_items = 0;
    for r in &out {
        match r.status {
            RequirementStatus::Complete => stats.complete_items += 1,
            RequirementStatus::Partial => stats.partial_items += 1,
            RequirementStatus::Blocker => stats.blocked_items += 1,
            RequirementStatus::NeedsTest => stats.needs_test_items += 1,
            RequirementStatus::Incomplete => {}
        }
    }
    Ok((out, stats))
}

fn build_feature_matrix(
    root: &Path,
    cfg: &LaunchOpsConfig,
    requirements: &[Requirement],
) -> Result<Vec<FeatureMatrixItem>> {
    let (code_cache, test_cache) = build_caches(root, &cfg.audit.paths)?;
    let mut items: Vec<FeatureMatrixItem> = Vec::new();
    for r in requirements {
        let ev = find_evidence(root, r, &cfg.audit.paths, &code_cache, &test_cache)?;
        let status = derive_feature_status(r, &ev);
        let base = base_status_score(&status) as f64;
        let rm = risk_multiplier(&r.risk);
        let score = (base * rm).round() as u32;
        // Cap evidence arrays emitted in the feature matrix to keep JSON size bounded.
        let mut code_ev = ev.code_evidence;
        let mut test_ev = ev.test_evidence;
        code_ev.truncate(5);
        test_ev.truncate(5);
        items.push(FeatureMatrixItem {
            feature: r.id.clone(),
            module: r.module.clone(),
            source_file: r.source_file.clone(),
            status,
            risk: r.risk.clone(),
            code_evidence: code_ev,
            test_evidence: test_ev,
            score,
        });
    }
    items.sort_by(|a, b| a.feature.cmp(&b.feature));
    Ok(items)
}

fn build_blockers(
    requirements: &[Requirement],
    features: &[FeatureMatrixItem],
) -> Vec<BlockerItem> {
    use std::collections::HashMap;
    let feature_by_id: HashMap<&str, &FeatureMatrixItem> =
        features.iter().map(|f| (f.feature.as_str(), f)).collect();

    let mut out = Vec::new();
    for r in requirements {
        // Explicit blocker marker
        if matches!(r.status, RequirementStatus::Blocker) {
            let sev = if r
                .tags
                .iter()
                .any(|t| t == "MAINNET" || t == "SECURITY" || t == "CONSENSUS" || t == "BRIDGE")
            {
                DriftSeverity::Critical
            } else {
                DriftSeverity::High
            };
            out.push(BlockerItem {
                severity: sev,
                id: r.id.clone(),
                module: r.module.clone(),
                source_file: r.source_file.clone(),
                line: r.line,
                reason: format!("Explicit [!] blocker: {}", r.text),
            });
            continue;
        }
        // Needs-test marker with no test evidence
        let feature = feature_by_id.get(r.id.as_str());
        if matches!(r.status, RequirementStatus::NeedsTest) {
            let no_tests = feature.map(|f| f.test_evidence.is_empty()).unwrap_or(true);
            if no_tests {
                out.push(BlockerItem {
                    severity: DriftSeverity::High,
                    id: r.id.clone(),
                    module: r.module.clone(),
                    source_file: r.source_file.clone(),
                    line: r.line,
                    reason: format!("[#] needs_test but no test evidence: {}", r.text),
                });
            }
        }
        // MAINNET/SECURITY tag with no evidence
        let needs_evidence = r.tags.iter().any(|t| t == "MAINNET" || t == "SECURITY");
        if needs_evidence {
            if let Some(f) = feature {
                if matches!(
                    f.status,
                    FeatureStatus::Missing | FeatureStatus::SpecifiedOnly
                ) {
                    out.push(BlockerItem {
                        severity: DriftSeverity::Critical,
                        id: r.id.clone(),
                        module: r.module.clone(),
                        source_file: r.source_file.clone(),
                        line: r.line,
                        reason: format!("Tagged {:?} without evidence: {}", r.tags, r.text),
                    });
                }
            }
        }
    }
    out.sort_by(|a, b| {
        severity_rank(&a.severity)
            .cmp(&severity_rank(&b.severity))
            .reverse()
            .then(a.id.cmp(&b.id))
    });
    out
}

fn severity_rank(s: &DriftSeverity) -> u8 {
    match s {
        DriftSeverity::Low => 0,
        DriftSeverity::Medium => 1,
        DriftSeverity::High => 2,
        DriftSeverity::Critical => 3,
    }
}

/// High-level results computed by `scan` and reused by `verify`/`audit`/`full`.
struct ScanOutputs {
    cfg: LaunchOpsConfig,
    requirements: Vec<Requirement>,
    features: Vec<FeatureMatrixItem>,
    blockers: Vec<BlockerItem>,
    stats: ScanStats,
    module_breakdown: BTreeMap<String, f64>,
    scan_score: f64,
}

fn do_scan(root: &Path) -> Result<ScanOutputs> {
    let cfg = load_config(root)?;
    let out_dir = ensure_out_dir(root)?;

    let (requirements, stats) = collect_requirements(root, &cfg)?;
    let features = build_feature_matrix(root, &cfg, &requirements)?;
    let blockers = build_blockers(&requirements, &features);
    let scoring = compute_scan_score(&features, &cfg.weights);

    let readiness = build_readiness(
        scoring.scan_score,
        scoring.module_breakdown.clone(),
        stats.clone(),
        ReadinessMultipliers::default(),
        blockers.len(),
    );

    write_json(&out_dir.join("readiness.json"), &readiness)?;
    write_json(&out_dir.join("blockers.json"), &blockers)?;
    write_json(&out_dir.join("feature_matrix.json"), &features)?;
    write_text(
        &out_dir.join("report.md"),
        &scan_report_md(&readiness, &blockers, &features),
    )?;
    do_inventory_contracts(root)?;
    write_artifact_contract(root)?;

    println!(
        "[scan] md_files={} requirements={} (raw={} deduped={}) scan_score={:.2}% blockers={}",
        stats.total_md_files,
        stats.total_requirements,
        stats.raw_requirements,
        stats.duplicates_collapsed,
        scoring.scan_score,
        blockers.len()
    );

    Ok(ScanOutputs {
        cfg,
        requirements,
        features,
        blockers,
        stats,
        module_breakdown: scoring.module_breakdown,
        scan_score: scoring.scan_score,
    })
}

fn do_verify(root: &Path, scan: &ScanOutputs) -> Result<(VerifyReport, Vec<GateResult>)> {
    let out_dir = ensure_out_dir(root)?;
    let command_results = crate::commands::run_commands(root, &scan.cfg.commands)?;
    let red_flags = crate::verify::scan_red_flags(root, &scan.cfg.audit.paths)?;
    let gate_results = crate::gates::evaluate_gates(
        &scan.cfg.gates,
        &command_results,
        &scan.features,
        &scan.blockers,
        &red_flags,
    );

    let passed = command_results
        .iter()
        .filter(|c| matches!(c.status, crate::models::CommandStatus::Passed))
        .count();
    let failed = command_results
        .iter()
        .filter(|c| matches!(c.status, crate::models::CommandStatus::Failed))
        .count();
    let ran = passed + failed;
    let command_multiplier = command_multiplier_from(passed, failed, ran);

    let required_total = gate_results.iter().filter(|g| g.required).count();
    let required_failed = gate_results
        .iter()
        .filter(|g| g.required && matches!(g.status, crate::models::GateStatus::Fail))
        .count();
    let gate_multiplier = gate_multiplier_from(required_failed, required_total);

    let red_counts = crate::verify::count_red_flags(&red_flags);
    let red_flag_multiplier = red_flag_multiplier_from(&red_counts);

    let overall_status = if failed > 0 || required_failed > 0 || red_counts.critical > 0 {
        "BLOCKED".to_string()
    } else {
        "NOT_BLOCKED".to_string()
    };
    let report = VerifyReport {
        overall_status,
        command_results: command_results.clone(),
        gates: gate_results.clone(),
        red_flags: red_flags.clone(),
        command_multiplier,
        gate_multiplier,
        red_flag_multiplier,
    };

    write_json(&out_dir.join("test_results.json"), &command_results)?;
    write_json(&out_dir.join("gates.json"), &gate_results)?;
    write_json(&out_dir.join("red_flags.json"), &red_flags)?;
    write_text(
        &out_dir.join("verify_report.md"),
        &verify_report_md(&report),
    )?;
    write_artifact_contract(root)?;

    println!(
        "[verify] commands_ran={} failed={} required_gates_failed={}/{} red_flags={} (crit={})",
        ran,
        failed,
        required_failed,
        required_total,
        red_flags.len(),
        red_counts.critical
    );
    Ok((report, gate_results))
}

fn do_audit(root: &Path, scan: &ScanOutputs) -> Result<AuditReport> {
    let out_dir = ensure_out_dir(root)?;
    let outcome =
        crate::audit::run_audit(root, &scan.cfg.audit, &scan.requirements, &scan.features)?;

    write_json(
        &out_dir.join("drift_report.json"),
        &outcome.report.drift_report,
    )?;
    write_json(
        &out_dir.join("deep_red_flags.json"),
        &outcome.report.deep_red_flags,
    )?;
    write_json(
        &out_dir.join("requirement_conflicts.json"),
        &outcome.report.requirement_conflicts,
    )?;
    write_json(&out_dir.join("stale_docs.json"), &outcome.report.stale_docs)?;
    write_text(
        &out_dir.join("audit_report.md"),
        &audit_report_md(&outcome.report),
    )?;
    write_artifact_contract(root)?;

    println!(
        "[audit] drift_flags={} conflicts={} stale_docs={} audit_multiplier={:.2}",
        outcome.report.drift_report.drift_flags.len(),
        outcome.report.requirement_conflicts.len(),
        outcome.report.stale_docs.len(),
        outcome.multiplier
    );
    Ok(outcome.report)
}

fn write_final(
    root: &Path,
    scan: &ScanOutputs,
    verify: Option<&VerifyReport>,
    audit: Option<&AuditReport>,
    gates: &[GateResult],
) -> Result<()> {
    let out_dir = ensure_out_dir(root)?;
    let mults = ReadinessMultipliers {
        command_multiplier: verify.map(|v| v.command_multiplier).unwrap_or(1.0),
        gate_multiplier: verify.map(|v| v.gate_multiplier).unwrap_or(1.0),
        red_flag_multiplier: verify.map(|v| v.red_flag_multiplier).unwrap_or(1.0),
        audit_multiplier: audit.map(|a| a.audit_multiplier).unwrap_or(1.0),
    };
    let readiness = build_readiness(
        scan.scan_score,
        scan.module_breakdown.clone(),
        scan.stats.clone(),
        mults,
        scan.blockers.len(),
    );
    write_json(&out_dir.join("readiness.json"), &readiness)?;
    write_text(
        &out_dir.join("final_report.md"),
        &final_report_md(&readiness, verify, audit, gates),
    )?;
    write_artifact_contract(root)?;
    println!(
        "[final] readiness={:.2}% status={}",
        readiness.final_readiness, readiness.status
    );
    Ok(())
}

fn usage() -> ! {
    eprintln!("usage: launchops <scan|verify|audit|full|validate-contract|inventory-contracts>");
    std::process::exit(2)
}

fn main() -> Result<()> {
    let _ = Module::Unknown; // keep Module in scope for main binary
    let args: Vec<String> = std::env::args().collect();
    let cmd = args.get(1).map(|s| s.as_str()).unwrap_or("scan");
    let root = repo_root();
    match cmd {
        "scan" => {
            do_scan(&root)?;
        }
        "verify" => {
            let scan = do_scan(&root)?;
            let (v, gates) = do_verify(&root, &scan)?;
            write_final(&root, &scan, Some(&v), None, &gates)?;
        }
        "audit" => {
            let scan = do_scan(&root)?;
            let a = do_audit(&root, &scan)?;
            write_final(&root, &scan, None, Some(&a), &[])?;
        }
        "full" => {
            let scan = do_scan(&root)?;
            let (v, gates) = do_verify(&root, &scan)?;
            let a = do_audit(&root, &scan)?;
            write_final(&root, &scan, Some(&v), Some(&a), &gates)?;
        }
        "validate-contract" => {
            let validated = crate::contract::validate_emitted_contract(&root)?;
            println!("[validate-contract] artifacts_validated={}", validated);
        }
        "inventory-contracts" => {
            do_inventory_contracts(&root)?;
        }
        "-h" | "--help" | "help" => {
            println!(
                "usage: launchops <scan|verify|audit|full|validate-contract|inventory-contracts>"
            );
        }
        _ => usage(),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn artifact_manifest_includes_generated_inventory_artifacts_when_present() {
        let root = std::env::temp_dir().join(format!(
            "launchops-manifest-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("time")
                .as_nanos()
        ));
        let out_dir = root.join(OUT_DIR);
        std::fs::create_dir_all(&out_dir).expect("create .launchops");

        std::fs::write(out_dir.join("artifact_schemas.json"), "{}\n").expect("schema file");
        std::fs::write(out_dir.join("runtime_rpc_inventory.json"), "{}\n").expect("inventory json");
        std::fs::write(out_dir.join("rpc_contract_matrix.json"), "{}\n").expect("matrix json");
        std::fs::write(out_dir.join("rpc_contract_matrix.md"), "# generated\n").expect("matrix md");
        std::fs::write(out_dir.join("rpc_consumer_contracts.json"), "{}\n").expect("consumer json");
        std::fs::write(out_dir.join("rpc_consumer_contracts.md"), "# generated\n")
            .expect("consumer md");
        std::fs::write(out_dir.join("frontend_route_allowlist.json"), "{}\n")
            .expect("route allowlist json");
        std::fs::write(out_dir.join("frontend_route_allowlist.md"), "# generated\n")
            .expect("route allowlist md");
        std::fs::write(out_dir.join("sidecar_adapter_backlog.json"), "{}\n").expect("backlog json");
        std::fs::write(out_dir.join("sidecar_adapter_backlog.md"), "# generated\n")
            .expect("backlog md");

        write_artifact_manifest(&root).expect("write manifest");
        let manifest_raw =
            std::fs::read_to_string(out_dir.join("artifact_manifest.json")).expect("read manifest");
        let manifest: ArtifactManifest =
            serde_json::from_str(&manifest_raw).expect("parse manifest");

        let has_runtime_inventory = manifest.artifacts.iter().any(|artifact| {
            artifact.file == "runtime_rpc_inventory.json"
                && artifact.kind == "runtime_rpc_inventory"
                && artifact.schema_ref.as_deref()
                    == Some("artifact_schemas.json#runtime_rpc_inventory")
        });
        let has_matrix_json = manifest.artifacts.iter().any(|artifact| {
            artifact.file == "rpc_contract_matrix.json"
                && artifact.kind == "rpc_contract_matrix"
                && artifact.schema_ref.as_deref()
                    == Some("artifact_schemas.json#rpc_contract_matrix")
        });
        let has_matrix_report = manifest.artifacts.iter().any(|artifact| {
            artifact.file == "rpc_contract_matrix.md"
                && artifact.kind == "rpc_contract_matrix_report"
                && artifact.schema_ref.as_deref()
                    == Some("artifact_schemas.json#rpc_contract_matrix_report")
        });
        let has_consumer_json = manifest.artifacts.iter().any(|artifact| {
            artifact.file == "rpc_consumer_contracts.json"
                && artifact.kind == "rpc_consumer_contracts"
                && artifact.schema_ref.as_deref()
                    == Some("artifact_schemas.json#rpc_consumer_contracts")
        });
        let has_consumer_report = manifest.artifacts.iter().any(|artifact| {
            artifact.file == "rpc_consumer_contracts.md"
                && artifact.kind == "rpc_consumer_contracts_report"
                && artifact.schema_ref.as_deref()
                    == Some("artifact_schemas.json#rpc_consumer_contracts_report")
        });
        let has_route_allowlist = manifest.artifacts.iter().any(|artifact| {
            artifact.file == "frontend_route_allowlist.json"
                && artifact.kind == "frontend_route_allowlist"
                && artifact.schema_ref.as_deref()
                    == Some("artifact_schemas.json#frontend_route_allowlist")
        });
        let has_backlog = manifest.artifacts.iter().any(|artifact| {
            artifact.file == "sidecar_adapter_backlog.json"
                && artifact.kind == "sidecar_adapter_backlog"
                && artifact.schema_ref.as_deref()
                    == Some("artifact_schemas.json#sidecar_adapter_backlog")
        });

        assert!(has_runtime_inventory);
        assert!(has_matrix_json);
        assert!(has_matrix_report);
        assert!(has_consumer_json);
        assert!(has_consumer_report);
        assert!(has_route_allowlist);
        assert!(has_backlog);
    }

    #[test]
    fn artifact_schema_docs_include_generated_inventory_contracts() {
        let docs = artifact_schema_docs();

        let runtime_inventory = docs
            .iter()
            .find(|doc| doc.kind == "runtime_rpc_inventory")
            .expect("runtime inventory schema");
        let matrix_json = docs
            .iter()
            .find(|doc| doc.kind == "rpc_contract_matrix")
            .expect("rpc matrix schema");
        let matrix_report = docs
            .iter()
            .find(|doc| doc.kind == "rpc_contract_matrix_report")
            .expect("rpc matrix markdown schema");
        let consumer_json = docs
            .iter()
            .find(|doc| doc.kind == "rpc_consumer_contracts")
            .expect("rpc consumer schema");
        let consumer_report = docs
            .iter()
            .find(|doc| doc.kind == "rpc_consumer_contracts_report")
            .expect("rpc consumer markdown schema");
        let route_allowlist = docs
            .iter()
            .find(|doc| doc.kind == "frontend_route_allowlist")
            .expect("frontend route allowlist schema");
        let sidecar_backlog = docs
            .iter()
            .find(|doc| doc.kind == "sidecar_adapter_backlog")
            .expect("sidecar backlog schema");

        assert_eq!(runtime_inventory.file, "runtime_rpc_inventory.json");
        assert_eq!(runtime_inventory.format, "json");
        assert!(runtime_inventory
            .fields
            .iter()
            .any(|field| field.name == "runtime_traits" && field.type_name == "array<object>"));

        assert_eq!(matrix_json.file, "rpc_contract_matrix.json");
        assert_eq!(matrix_json.format, "json");
        assert!(matrix_json
            .fields
            .iter()
            .any(|field| field.name == "methods" && field.type_name == "array<object>"));

        assert_eq!(matrix_report.file, "rpc_contract_matrix.md");
        assert_eq!(matrix_report.format, "markdown");
        assert_eq!(matrix_report.shape, "text/markdown");

        assert_eq!(consumer_json.file, "rpc_consumer_contracts.json");
        assert_eq!(consumer_json.format, "json");
        assert!(consumer_json.fields.iter().any(
            |field| field.name == "frontend_safe_methods" && field.type_name == "array<object>"
        ));

        assert_eq!(consumer_report.file, "rpc_consumer_contracts.md");
        assert_eq!(consumer_report.format, "markdown");
        assert_eq!(consumer_report.shape, "text/markdown");

        assert_eq!(route_allowlist.file, "frontend_route_allowlist.json");
        assert!(route_allowlist
            .fields
            .iter()
            .any(|field| field.name == "routes" && field.type_name == "array<object>"));

        assert_eq!(sidecar_backlog.file, "sidecar_adapter_backlog.json");
        assert!(sidecar_backlog
            .fields
            .iter()
            .any(|field| field.name == "backlog" && field.type_name == "array<object>"));
    }
}
