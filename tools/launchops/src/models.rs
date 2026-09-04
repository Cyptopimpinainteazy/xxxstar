use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LaunchOpsConfig {
    #[serde(default)]
    pub scan: ScanConfig,
    #[serde(default)]
    pub weights: WeightsConfig,
    #[serde(default)]
    pub required_gates: RequiredGates,
    #[serde(default)]
    pub commands: BTreeMap<String, String>,
    #[serde(default)]
    pub gates: BTreeMap<String, GateConfig>,
    #[serde(default)]
    pub audit: AuditConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanConfig {
    #[serde(default = "default_include")]
    pub include: Vec<String>,
    #[serde(default = "default_exclude")]
    pub exclude: Vec<String>,
}

impl Default for ScanConfig {
    fn default() -> Self {
        Self {
            include: default_include(),
            exclude: default_exclude(),
        }
    }
}

fn default_include() -> Vec<String> {
    vec![
        "README.md".to_string(),
        "docs/**/*.md".to_string(),
        "crates/**/*.md".to_string(),
        "runtime/**/*.md".to_string(),
        "node/**/*.md".to_string(),
        "testnet/**/*.md".to_string(),
    ]
}

fn default_exclude() -> Vec<String> {
    vec![
        "target/**".to_string(),
        "node_modules/**".to_string(),
        ".git/**".to_string(),
        "dist/**".to_string(),
        "build/**".to_string(),
    ]
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeightsConfig {
    #[serde(default = "d20")]
    pub consensus: u32,
    #[serde(default = "d15")]
    pub cross_vm: u32,
    #[serde(default = "d15")]
    pub bridge: u32,
    #[serde(default = "d10")]
    pub universal_asset_kernel: u32,
    #[serde(default = "d10")]
    pub dex: u32,
    #[serde(default = "d15")]
    pub security: u32,
    #[serde(default = "d5")]
    pub wallet_explorer: u32,
    #[serde(default = "d5")]
    pub ops: u32,
    #[serde(default = "d5")]
    pub docs: u32,
}

impl Default for WeightsConfig {
    fn default() -> Self {
        Self {
            consensus: 20,
            cross_vm: 15,
            bridge: 15,
            universal_asset_kernel: 10,
            dex: 10,
            security: 15,
            wallet_explorer: 5,
            ops: 5,
            docs: 5,
        }
    }
}

const fn d20() -> u32 {
    20
}
const fn d15() -> u32 {
    15
}
const fn d10() -> u32 {
    10
}
const fn d5() -> u32 {
    5
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RequiredGates {
    #[serde(default)]
    pub cargo_check: bool,
    #[serde(default)]
    pub cargo_test: bool,
    #[serde(default)]
    pub clippy: bool,
    #[serde(default)]
    pub fmt: bool,
    #[serde(default)]
    pub no_p0_blockers: bool,
    #[serde(default)]
    pub no_critical_stubs: bool,
    #[serde(default)]
    pub cross_vm_tests: bool,
    #[serde(default)]
    pub bridge_replay_tests: bool,
    #[serde(default)]
    pub dex_liquidity_lock_tests: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GateConfig {
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub module: Option<String>,
    #[serde(default)]
    pub keywords: Vec<String>,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub weight: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditConfig {
    #[serde(default = "default_baseline")]
    pub baseline_branch: String,
    #[serde(default)]
    pub fail_on_critical: bool,
    #[serde(default)]
    pub fail_on_test_weakening: bool,
    #[serde(default)]
    pub fail_on_consensus_without_invariants: bool,
    #[serde(default)]
    pub fail_on_bridge_without_replay_tests: bool,
    #[serde(default)]
    pub fail_on_mainnet_config_without_review: bool,
    #[serde(default = "default_stale_days")]
    pub stale_doc_days: u64,
    #[serde(default)]
    pub paths: AuditPathConfig,
}

impl Default for AuditConfig {
    fn default() -> Self {
        Self {
            baseline_branch: default_baseline(),
            fail_on_critical: true,
            fail_on_test_weakening: true,
            fail_on_consensus_without_invariants: true,
            fail_on_bridge_without_replay_tests: true,
            fail_on_mainnet_config_without_review: true,
            stale_doc_days: 30,
            paths: AuditPathConfig::default(),
        }
    }
}

fn default_baseline() -> String {
    "main".to_string()
}

const fn default_stale_days() -> u64 {
    30
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditPathConfig {
    #[serde(default = "default_audit_docs")]
    pub docs: Vec<String>,
    #[serde(default = "default_prod_code")]
    pub production_code: Vec<String>,
    #[serde(default = "default_test_paths")]
    pub tests: Vec<String>,
    #[serde(default = "default_consensus_paths")]
    pub consensus: Vec<String>,
    #[serde(default = "default_bridge_paths")]
    pub bridge: Vec<String>,
    #[serde(default = "default_mainnet_paths")]
    pub mainnet_config: Vec<String>,
}

impl Default for AuditPathConfig {
    fn default() -> Self {
        Self {
            docs: default_audit_docs(),
            production_code: default_prod_code(),
            tests: default_test_paths(),
            consensus: default_consensus_paths(),
            bridge: default_bridge_paths(),
            mainnet_config: default_mainnet_paths(),
        }
    }
}

fn default_audit_docs() -> Vec<String> {
    vec![
        "README.md".to_string(),
        "docs/**/*.md".to_string(),
        "crates/**/*.md".to_string(),
    ]
}

fn default_prod_code() -> Vec<String> {
    vec![
        "crates/**/src/**/*.rs".to_string(),
        "runtime/**/*.rs".to_string(),
        "node/**/*.rs".to_string(),
    ]
}

fn default_test_paths() -> Vec<String> {
    vec![
        "tests/**/*.rs".to_string(),
        "crates/**/tests/**/*.rs".to_string(),
        "crates/**/src/**/*test*.rs".to_string(),
    ]
}

fn default_consensus_paths() -> Vec<String> {
    vec![
        "runtime/src/**/*.rs".to_string(),
        "node/src/service.rs".to_string(),
        "crates/**/consensus*/**/*.rs".to_string(),
        "crates/**/finality*/**/*.rs".to_string(),
        "crates/**/poh*/**/*.rs".to_string(),
        "crates/**/parallel-proposer/**/*.rs".to_string(),
        "crates/**/contention-predictor/**/*.rs".to_string(),
    ]
}

fn default_bridge_paths() -> Vec<String> {
    vec![
        "crates/**/bridge*/**/*.rs".to_string(),
        "crates/**/*bridge*/**/*.rs".to_string(),
        "crates/**/cross-chain*/**/*.rs".to_string(),
        "crates/**/external-liquidity*/**/*.rs".to_string(),
        "crates/**/relayer*/**/*.rs".to_string(),
        "crates/**/*relayer*/**/*.rs".to_string(),
        "crates/**/adapter*/**/*.rs".to_string(),
    ]
}

fn default_mainnet_paths() -> Vec<String> {
    vec![
        "chain-spec.json".to_string(),
        "genesis.json".to_string(),
        "**/mainnet*.json".to_string(),
        "**/testnet*.json".to_string(),
        "node/src/chain_spec.rs".to_string(),
        "node/src/service.rs".to_string(),
        "runtime/src/lib.rs".to_string(),
    ]
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RequirementStatus {
    Complete,
    Incomplete,
    Partial,
    Blocker,
    NeedsTest,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Module {
    Consensus,
    CrossVm,
    Bridge,
    UniversalAssetKernel,
    Dex,
    GpuValidator,
    WalletExplorer,
    LaunchOps,
    Security,
    Ops,
    Docs,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Requirement {
    pub id: String,
    pub text: String,
    pub source_file: String,
    pub line: usize,
    pub status: RequirementStatus,
    pub tags: Vec<String>,
    pub module: Module,
    pub risk: RiskLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FeatureStatus {
    Missing,
    SpecifiedOnly,
    Partial,
    Implemented,
    Tested,
    Verified,
    Blocked,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureMatrixItem {
    pub feature: String,
    pub module: Module,
    pub source_file: String,
    pub status: FeatureStatus,
    pub risk: RiskLevel,
    pub code_evidence: Vec<String>,
    pub test_evidence: Vec<String>,
    pub score: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockerItem {
    pub severity: DriftSeverity,
    pub id: String,
    pub module: Module,
    pub source_file: String,
    pub line: usize,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadinessOutput {
    pub generated_at: String,
    pub scan_score: f64,
    pub final_readiness: f64,
    pub status: String,
    pub module_breakdown: BTreeMap<String, f64>,
    pub totals: ScanStats,
    pub multipliers: ReadinessMultipliers,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ScanStats {
    pub total_md_files: usize,
    pub total_requirements: usize,
    pub complete_items: usize,
    pub partial_items: usize,
    pub blocked_items: usize,
    pub needs_test_items: usize,
    #[serde(default)]
    pub raw_requirements: usize,
    #[serde(default)]
    pub duplicates_collapsed: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadinessMultipliers {
    pub command_multiplier: f64,
    pub gate_multiplier: f64,
    pub red_flag_multiplier: f64,
    pub audit_multiplier: f64,
}

impl Default for ReadinessMultipliers {
    fn default() -> Self {
        Self {
            command_multiplier: 1.0,
            gate_multiplier: 1.0,
            red_flag_multiplier: 1.0,
            audit_multiplier: 1.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandResult {
    pub name: String,
    pub command: String,
    pub status: CommandStatus,
    pub exit_code: Option<i32>,
    pub duration_ms: u128,
    pub stdout_excerpt: String,
    pub stderr_excerpt: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CommandStatus {
    Passed,
    Failed,
    Skipped,
    MissingTool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateResult {
    pub id: String,
    pub name: String,
    pub status: GateStatus,
    pub required: bool,
    pub weight: u32,
    pub source: String,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GateStatus {
    Pass,
    Fail,
    Warn,
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyReport {
    pub overall_status: String,
    pub command_results: Vec<CommandResult>,
    pub gates: Vec<GateResult>,
    pub red_flags: Vec<RedFlag>,
    pub command_multiplier: f64,
    pub gate_multiplier: f64,
    pub red_flag_multiplier: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedFlag {
    pub severity: RedFlagSeverity,
    pub file: String,
    pub line: usize,
    pub pattern: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum RedFlagSeverity {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChangedFileSet {
    pub docs: Vec<String>,
    pub code: Vec<String>,
    pub tests: Vec<String>,
    pub consensus: Vec<String>,
    pub bridge: Vec<String>,
    pub cross_vm: Vec<String>,
    pub dex: Vec<String>,
    pub gpu: Vec<String>,
    pub ops: Vec<String>,
    pub mainnet_config: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriftFlag {
    pub severity: DriftSeverity,
    pub flag_type: String,
    pub files: Vec<String>,
    pub reason: String,
    pub required_evidence: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum DriftSeverity {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequirementConflict {
    pub severity: DriftSeverity,
    pub conflict_type: String,
    pub requirement_a: RequirementRef,
    pub requirement_b: RequirementRef,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequirementRef {
    pub file: String,
    pub line: usize,
    pub text: String,
    pub status: String,
    pub module: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StaleDoc {
    pub file: String,
    pub linked_code: Vec<String>,
    pub severity: DriftSeverity,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriftReport {
    pub baseline_branch: String,
    pub changed_files: ChangedFileSet,
    pub drift_flags: Vec<DriftFlag>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditReport {
    pub overall_status: String,
    pub drift_report: DriftReport,
    pub deep_red_flags: Vec<DriftFlag>,
    pub requirement_conflicts: Vec<RequirementConflict>,
    pub stale_docs: Vec<StaleDoc>,
    pub audit_multiplier: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeRpcInventory {
    pub generated_at: String,
    pub runtime_source_file: String,
    pub rpc_source_file: String,
    pub runtime_traits: Vec<RuntimeApiTraitInventory>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeApiTraitInventory {
    pub trait_name: String,
    pub source_file: String,
    pub impl_line: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cfg_guard: Option<String>,
    pub methods: Vec<RuntimeApiMethodInventory>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeApiMethodInventory {
    pub name: String,
    pub line: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcContractMatrix {
    pub generated_at: String,
    pub rpc_source_file: String,
    pub runtime_backed_count: usize,
    pub node_local_adapter_count: usize,
    pub placeholder_count: usize,
    pub duplicate_registration_count: usize,
    pub bucket_drift_count: usize,
    pub flags: Vec<RpcContractFlag>,
    pub methods: Vec<RpcMethodContract>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcConsumerContracts {
    pub generated_at: String,
    pub source_matrix_file: String,
    pub frontend_safe_count: usize,
    pub sidecar_only_count: usize,
    pub mock_only_count: usize,
    pub frontend_safe_methods: Vec<RpcConsumerContractEntry>,
    pub sidecar_only_methods: Vec<RpcConsumerContractEntry>,
    pub mock_only_methods: Vec<RpcConsumerContractEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcConsumerContractEntry {
    pub method: String,
    pub registration_count: usize,
    pub bucket: RpcContractBucket,
    pub frontend_consumer_mode: String,
    pub sidecar_consumer_mode: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ownership_note: Option<String>,
    pub runtime_trait_hints: Vec<String>,
    pub node_local_signals: Vec<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrontendRouteAllowlist {
    pub generated_at: String,
    pub source_consumer_contracts_file: String,
    pub routes: Vec<FrontendRouteAllowlistEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrontendRouteAllowlistEntry {
    pub route_id: String,
    pub route_label: String,
    pub rationale: String,
    pub allowed_methods: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SidecarAdapterBacklog {
    pub generated_at: String,
    pub source_consumer_contracts_file: String,
    pub backlog: Vec<SidecarAdapterBacklogEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SidecarAdapterBacklogEntry {
    pub route_id: String,
    pub route_label: String,
    pub method: String,
    pub backlog_reason: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ownership_note: Option<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcMethodContract {
    pub method: String,
    pub source_file: String,
    pub line: usize,
    pub bucket: RpcContractBucket,
    pub runtime_calls: Vec<String>,
    pub runtime_trait_hints: Vec<String>,
    pub node_local_signals: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub placeholder_reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ownership_note: Option<String>,
    pub frontend_consumer_mode: String,
    pub sidecar_consumer_mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcContractFlag {
    pub category: RpcContractFlagCategory,
    pub severity: RpcContractFlagSeverity,
    pub method: String,
    pub line_refs: Vec<String>,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RpcContractFlagCategory {
    DuplicateRegistration,
    BucketDrift,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RpcContractFlagSeverity {
    Warn,
    High,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RpcContractBucket {
    RuntimeBacked,
    NodeLocalAdapter,
    Placeholder,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactManifest {
    pub schema_version: String,
    pub generated_at: String,
    pub artifacts: Vec<ArtifactDescriptor>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactDescriptor {
    pub file: String,
    pub kind: String,
    pub format: String,
    pub schema_version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema_ref: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactSchemaIndex {
    pub schema_version: String,
    pub generated_at: String,
    pub artifacts: Vec<ArtifactSchemaDoc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactSchemaDoc {
    pub file: String,
    pub kind: String,
    pub format: String,
    pub shape: String,
    pub fields: Vec<ArtifactFieldDoc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactFieldDoc {
    pub name: String,
    pub type_name: String,
    pub required: bool,
    pub description: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub enum_values: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub nested_fields: Vec<ArtifactFieldDoc>,
}
