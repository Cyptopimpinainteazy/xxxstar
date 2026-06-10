//! # X3 Foundry Auditor
//!
//! Automated security auditor for X3 Foundry dApps. Performs static analysis,
//! vulnerability pattern detection, fee transparency verification, and
//! compliance checks.

use chrono::{DateTime, Utc};
use regex::Regex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use tracing::info;

pub use sha2::Sha256;

pub const MAX_RISK_SCORE: u64 = 100;
pub const MIN_RISK_SCORE: u64 = 0;
pub const HIGH_RISK_THRESHOLD: u64 = 70;
pub const MEDIUM_RISK_THRESHOLD: u64 = 40;
pub const LOW_RISK_THRESHOLD: u64 = 10;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AuditorError {
    #[error("Invalid source code: {0}")]
    InvalidSource(String),
    #[error("Analysis failed: {0}")]
    AnalysisFailed(String),
    #[error("Pattern detection error: {0}")]
    PatternDetectionError(String),
    #[error("Risk scoring error: {0}")]
    RiskScoringError(String),
    #[error("License check failed: {0}")]
    LicenseCheckFailed(String),
}

impl From<AuditorError> for anyhow::Error {
    fn from(e: AuditorError) -> Self {
        anyhow::anyhow!("{}", e)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Severity {
    Critical,
    High,
    Medium,
    Low,
    Info,
}

impl Severity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Severity::Critical => "CRITICAL",
            Severity::High => "HIGH",
            Severity::Medium => "MEDIUM",
            Severity::Low => "LOW",
            Severity::Info => "INFO",
        }
    }

    pub fn from_score(score: u64) -> Self {
        if score >= 90 {
            Severity::Critical
        } else if score >= HIGH_RISK_THRESHOLD {
            Severity::High
        } else if score >= MEDIUM_RISK_THRESHOLD {
            Severity::Medium
        } else if score >= LOW_RISK_THRESHOLD {
            Severity::Low
        } else {
            Severity::Info
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    pub id: String,
    pub title: String,
    pub description: String,
    pub severity: Severity,
    pub category: FindingCategory,
    pub location: Option<SourceLocation>,
    pub remediation: Option<String>,
    pub score: u64,
    pub raw_snippet: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FindingCategory {
    Reentrancy,
    Ownership,
    AccessControl,
    FeeTransparency,
    LicenseCompliance,
    ScamPattern,
    StaticAnalysis,
    UncheckedExternalCall,
    IntegerOverflow,
    TimestampDependency,
    GasOptimization,
    CentralizationRisk,
    Other,
}

impl FindingCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            FindingCategory::Reentrancy => "Reentrancy",
            FindingCategory::Ownership => "Ownership",
            FindingCategory::AccessControl => "Access Control",
            FindingCategory::FeeTransparency => "Fee Transparency",
            FindingCategory::LicenseCompliance => "License Compliance",
            FindingCategory::ScamPattern => "Scam Pattern",
            FindingCategory::StaticAnalysis => "Static Analysis",
            FindingCategory::UncheckedExternalCall => "Unchecked External Call",
            FindingCategory::IntegerOverflow => "Integer Overflow",
            FindingCategory::TimestampDependency => "Timestamp Dependency",
            FindingCategory::GasOptimization => "Gas Optimization",
            FindingCategory::CentralizationRisk => "Centralization Risk",
            FindingCategory::Other => "Other",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceLocation {
    pub file: String,
    pub line: usize,
    pub column: Option<usize>,
    pub snippet: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditReport {
    pub report_id: String,
    pub project_name: String,
    pub project_version: Option<String>,
    pub source_hash: String,
    pub audited_at: DateTime<Utc>,
    pub findings: Vec<Finding>,
    pub summary: AuditSummary,
    pub risk_score: u64,
    pub risk_level: String,
    pub passed: bool,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditSummary {
    pub total_findings: usize,
    pub critical: usize,
    pub high: usize,
    pub medium: usize,
    pub low: usize,
    pub info: usize,
    pub passed_checks: usize,
    pub failed_checks: usize,
    pub score: u64,
}

impl AuditReport {
    pub fn new(project_name: &str, source_code: &str) -> Self {
        let source_hash = hex::encode(Sha256::digest(source_code.as_bytes()));
        Self {
            report_id: format!("audit-{}-{}", project_name, Utc::now().timestamp()),
            project_name: project_name.to_string(),
            project_version: None,
            source_hash,
            audited_at: Utc::now(),
            findings: Vec::new(),
            summary: AuditSummary {
                total_findings: 0,
                critical: 0,
                high: 0,
                medium: 0,
                low: 0,
                info: 0,
                passed_checks: 0,
                failed_checks: 0,
                score: 0,
            },
            risk_score: 0,
            risk_level: "UNKNOWN".into(),
            passed: true,
            metadata: HashMap::new(),
        }
    }

    pub fn add_finding(&mut self, finding: Finding) {
        match finding.severity {
            Severity::Critical => self.summary.critical += 1,
            Severity::High => self.summary.high += 1,
            Severity::Medium => self.summary.medium += 1,
            Severity::Low => self.summary.low += 1,
            Severity::Info => self.summary.info += 1,
        }
        self.findings.push(finding);
        self.summary.total_findings = self.findings.len();
    }

    pub fn finalize(&mut self) {
        self.summary.total_findings = self.findings.len();
        self.risk_score = RiskScorer::calculate(&self.findings);
        self.risk_level = RiskScorer::risk_level(self.risk_score);
        self.passed = self.risk_score < MEDIUM_RISK_THRESHOLD
            && self.summary.critical == 0
            && self.summary.high == 0;
        self.summary.score = self.risk_score;
    }

    pub fn set_metadata(&mut self, key: &str, value: &str) {
        self.metadata.insert(key.to_string(), value.to_string());
    }
}

pub struct RiskScorer;

impl RiskScorer {
    pub fn calculate(findings: &[Finding]) -> u64 {
        if findings.is_empty() {
            return 0;
        }
        let mut score: u64 = 0;
        for finding in findings {
            score = score.saturating_add(finding.score);
        }
        let avg = score / findings.len() as u64;
        let max = findings.iter().map(|f| f.score).max().unwrap_or(0);
        (avg.saturating_mul(3) + max.saturating_mul(2)) / 5
    }

    pub fn risk_level(score: u64) -> String {
        if score >= 90 {
            "CRITICAL".into()
        } else if score >= HIGH_RISK_THRESHOLD {
            "HIGH".into()
        } else if score >= MEDIUM_RISK_THRESHOLD {
            "MEDIUM".into()
        } else if score >= LOW_RISK_THRESHOLD {
            "LOW".into()
        } else {
            "PASS".into()
        }
    }

    pub fn severity_from_score(score: u64) -> Severity {
        Severity::from_score(score)
    }
}

pub struct PatternDetector;

impl PatternDetector {
    pub fn detect_reentrancy(source: &str) -> Vec<(usize, String)> {
        let mut findings = Vec::new();
        let patterns: [(&str, &str); 4] = [
            (r"\.call\s*\{[^}]*\}\s*\([^)]*\)", "Low-level .call() detected - potential reentrancy vector"),
            (r"\.delegatecall\s*\([^)]*\)", "delegatecall detected - potential reentrancy vector"),
            (r"\.send\s*\([^)]*\)", ".send() detected - consider using pull-over-push pattern"),
            (r"\.transfer\s*\([^)]*\)", ".transfer() detected - may be unsafe with gas changes"),
        ];
        for (pattern, desc) in &patterns {
            if let Ok(re) = Regex::new(pattern) {
                for cap in re.find_iter(source) {
                    findings.push((
                        source[..cap.start()].lines().count(),
                        format!("{}: '{}'", desc, cap.as_str()),
                    ));
                }
            }
        }
        findings
    }

    pub fn detect_ownership_patterns(source: &str) -> Vec<(usize, String)> {
        let mut findings = Vec::new();
        let patterns: [(&str, &str); 6] = [
            (r"onlyOwner", "onlyOwner modifier used - check for centralization risk"),
            (r"Ownable", "Ownable contract detected"),
            (r"transferOwnership", "transferOwnership function detected"),
            (r"renounceOwnership", "renounceOwnership function detected - can permanently lock contract"),
            (r"Ownable2Step", "Two-step ownership transfer detected (good practice)"),
            (r"OwnableUpgradeable", "Upgradeable Ownable detected"),
        ];
        for (pattern, desc) in &patterns {
            if let Ok(re) = Regex::new(pattern) {
                for cap in re.find_iter(source) {
                    findings.push((
                        source[..cap.start()].lines().count(),
                        format!("{}: '{}'", desc, cap.as_str()),
                    ));
                }
            }
        }
        findings
    }

    pub fn detect_fee_patterns(source: &str) -> Vec<(usize, String)> {
        let mut findings = Vec::new();
        let patterns: [(&str, &str); 6] = [
            (r"fee\s*[=:]\s*\d{4,}", "Hardcoded fee value detected - verify transparency"),
            (r"basis\s*points", "Basis points fee mechanism detected"),
            (r"platformFee|platform_fee", "Platform fee variable detected"),
            (r"creatorFee|creator_fee", "Creator fee variable detected"),
            (r"referralFee|referral_fee", "Referral fee variable detected"),
            (r"setFee|updateFee|changeFee", "Mutable fee function detected - check for owner-only access"),
        ];
        for (pattern, desc) in &patterns {
            if let Ok(re) = Regex::new(pattern) {
                for cap in re.find_iter(source) {
                    findings.push((
                        source[..cap.start()].lines().count(),
                        format!("{}: '{}'", desc, cap.as_str()),
                    ));
                }
            }
        }
        findings
    }

    pub fn detect_scam_patterns(source: &str) -> Vec<(usize, String)> {
        let mut findings = Vec::new();
        let patterns: [(&str, &str); 10] = [
            (r"honeypot", "Honeypot keyword detected in source"),
            (r"rug\s*pull", "Rug pull keyword detected"),
            (r"pump\s*and\s*dump", "Pump and dump keyword detected"),
            (r"unlimited\s*approval", "Unlimited approval pattern detected - potential risk"),
            (r"selfdestruct|self_destruct|suicide", "Selfdestruct detected - contract can be destroyed"),
            (r"block\.timestamp\s*[=<>]", "Timestamp dependency detected - potential manipulation"),
            (r"tx\.origin", "tx.origin detected - use msg.sender instead"),
            (r"gasleft\s*[<>=]", "Gasleft comparison detected - potential gas manipulation"),
            (r"assembly\s*\{", "Inline assembly detected - review for safety"),
            (r"delegatecall", "delegatecall detected - can lead to storage collisions"),
        ];
        for (pattern, desc) in &patterns {
            if let Ok(re) = Regex::new(pattern) {
                for cap in re.find_iter(source) {
                    findings.push((
                        source[..cap.start()].lines().count(),
                        format!("{}: '{}'", desc, cap.as_str()),
                    ));
                }
            }
        }
        findings
    }

    pub fn detect_license(source: &str) -> Vec<(usize, String)> {
        let mut findings = Vec::new();
        let patterns: [(&str, &str); 6] = [
            (r"SPDX-License-Identifier:\s*MIT", "MIT license detected"),
            (r"SPDX-License-Identifier:\s*GPL", "GPL license detected"),
            (r"SPDX-License-Identifier:\s*Apache", "Apache license detected"),
            (r"SPDX-License-Identifier:\s*BSD", "BSD license detected"),
            (r"SPDX-License-Identifier:\s*UNLICENSED", "UNLICENSED - no open source license"),
            (r"SPDX-License-Identifier:\s*BUSL", "Business Source License detected"),
        ];
        for (pattern, desc) in &patterns {
            if let Ok(re) = Regex::new(pattern) {
                for cap in re.find_iter(source) {
                    findings.push((
                        source[..cap.start()].lines().count(),
                        format!("{}: '{}'", desc, cap.as_str()),
                    ));
                }
            }
        }
        if findings.is_empty() {
            findings.push((0, "No SPDX license identifier found".into()));
        }
        findings
    }

    pub fn detect_unchecked_external_calls(source: &str) -> Vec<(usize, String)> {
        let mut findings = Vec::new();
        let patterns: [(&str, &str); 2] = [
            (r"\.call\s*\{[^}]*\}\s*\([^)]*\)\s*;", "Unchecked external call - result not validated"),
            (r"\.send\s*\([^)]*\)\s*;", "Unchecked .send() - returns bool that may be ignored"),
        ];
        for (pattern, desc) in &patterns {
            if let Ok(re) = Regex::new(pattern) {
                for cap in re.find_iter(source) {
                    findings.push((
                        source[..cap.start()].lines().count(),
                        format!("{}: '{}'", desc, cap.as_str()),
                    ));
                }
            }
        }
        findings
    }

    pub fn detect_integer_overflows(source: &str) -> Vec<(usize, String)> {
        let mut findings = Vec::new();
        let patterns: [(&str, &str); 6] = [
            (r"uint8\s", "uint8 detected - potential overflow below 256"),
            (r"uint16\s", "uint16 detected - potential overflow below 256"),
            (r"uint32\s", "uint32 detected - potential overflow below 256"),
            (r"uint64\s", "uint64 detected - potential overflow below 256"),
            (r"uint128\s", "uint128 detected - potential overflow below 256"),
            (r"unchecked\s*\{", "unchecked block detected - overflow protection disabled"),
        ];
        for (pattern, desc) in &patterns {
            if let Ok(re) = Regex::new(pattern) {
                for cap in re.find_iter(source) {
                    findings.push((
                        source[..cap.start()].lines().count(),
                        format!("{}: '{}'", desc, cap.as_str()),
                    ));
                }
            }
        }
        findings
    }
}

pub struct FeeTransparencyChecker;

impl FeeTransparencyChecker {
    pub fn verify(source: &str) -> FeeTransparencyResult {
        let mut result = FeeTransparencyResult {
            has_fee_declaration: false,
            has_mutable_fees: false,
            has_fee_cap: false,
            has_fee_events: false,
            has_owner_only_fee_changes: false,
            fee_functions: Vec::new(),
            issues: Vec::new(),
        };

        if let Ok(re) = Regex::new(r"(?i)(fee|platformFee|creatorFee|referralFee)") {
            result.has_fee_declaration = re.is_match(source);
        }
        if let Ok(re) = Regex::new(r"(?i)(setFee|updateFee|changeFee|modifyFee)") {
            for cap in re.find_iter(source) {
                result.fee_functions.push((
                    source[..cap.start()].lines().count(),
                    cap.as_str().to_string(),
                ));
                result.has_mutable_fees = true;
            }
        }
        if let Ok(re) = Regex::new(r"(?i)(maxFee|feeCap|feeLimit|MAX_FEE)") {
            result.has_fee_cap = re.is_match(source);
        }
        if let Ok(re) = Regex::new(r"(?i)(FeeChanged|FeeUpdated|FeeSet|emit\s+Fee)") {
            result.has_fee_events = re.is_match(source);
        }
        if let Ok(re) = Regex::new(r"(?i)(onlyOwner|require\(msg\.sender\s*==\s*owner\))") {
            result.has_owner_only_fee_changes = re.is_match(source) && result.has_mutable_fees;
        }

        if result.has_mutable_fees && !result.has_fee_cap {
            result.issues.push("Mutable fees without a maximum cap".into());
        }
        if result.has_mutable_fees && !result.has_fee_events {
            result.issues.push("Mutable fees without events for transparency".into());
        }
        if result.has_mutable_fees && !result.has_owner_only_fee_changes {
            result.issues.push("Mutable fees without owner-only access control".into());
        }
        if !result.has_fee_declaration {
            result.issues.push("No fee declaration found in source".into());
        }
        result
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeTransparencyResult {
    pub has_fee_declaration: bool,
    pub has_mutable_fees: bool,
    pub has_fee_cap: bool,
    pub has_fee_events: bool,
    pub has_owner_only_fee_changes: bool,
    pub fee_functions: Vec<(usize, String)>,
    pub issues: Vec<String>,
}

pub struct PrincipalSafetyChecker;

impl PrincipalSafetyChecker {
    pub fn verify(source: &str) -> PrincipalSafetyResult {
        let mut result = PrincipalSafetyResult {
            has_owner: false,
            has_pausable: false,
            has_emergency_stop: false,
            has_rate_limit: false,
            has_withdrawal_guard: false,
            has_multi_sig_requirement: false,
            has_timelock: false,
            issues: Vec::new(),
        };

        if let Ok(re) = Regex::new(r"(?i)(onlyOwner|Ownable|owner\s*[=:])") {
            result.has_owner = re.is_match(source);
        }
        if let Ok(re) = Regex::new(r"(?i)(Pausable|whenNotPaused|whenPaused|pause\s*\()") {
            result.has_pausable = re.is_match(source);
        }
        if let Ok(re) = Regex::new(r"(?i)(emergencyStop|emergency_stop|emergencyPause|circuitBreaker)") {
            result.has_emergency_stop = re.is_match(source);
        }
        if let Ok(re) = Regex::new(r"(?i)(rateLimit|rate_limit|throttle|cooldown)") {
            result.has_rate_limit = re.is_match(source);
        }
        if let Ok(re) = Regex::new(r"(?i)(withdrawGuard|withdrawal_guard|withdraw\s*onlyOwner)") {
            result.has_withdrawal_guard = re.is_match(source);
        }
        if let Ok(re) = Regex::new(r"(?i)(multiSig|multisig|MultiSig|Gnosis|Safe)") {
            result.has_multi_sig_requirement = re.is_match(source);
        }
        if let Ok(re) = Regex::new(r"(?i)(Timelock|timelock|timeLock|TimeLock)") {
            result.has_timelock = re.is_match(source);
        }

        if !result.has_owner {
            result.issues.push("No owner/ownership pattern detected".into());
        }
        if !result.has_pausable {
            result.issues.push("No pausable mechanism detected".into());
        }
        if !result.has_emergency_stop {
            result.issues.push("No emergency stop mechanism detected".into());
        }
        if !result.has_withdrawal_guard {
            result.issues.push("No withdrawal guard detected".into());
        }
        result
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrincipalSafetyResult {
    pub has_owner: bool,
    pub has_pausable: bool,
    pub has_emergency_stop: bool,
    pub has_rate_limit: bool,
    pub has_withdrawal_guard: bool,
    pub has_multi_sig_requirement: bool,
    pub has_timelock: bool,
    pub issues: Vec<String>,
}

pub struct FoundryAuditor {
    project_name: String,
    source_code: String,
    findings: Vec<Finding>,
    metadata: HashMap<String, String>,
}

impl FoundryAuditor {
    pub fn new(project_name: &str, source_code: &str) -> Self {
        Self {
            project_name: project_name.to_string(),
            source_code: source_code.to_string(),
            findings: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    pub fn add_metadata(&mut self, key: &str, value: &str) {
        self.metadata.insert(key.to_string(), value.to_string());
    }

    pub fn audit_project(&mut self) -> AuditReport {
        info!("Starting audit for project: {}", self.project_name);
        self.check_static_analysis();
        self.check_reentrancy_patterns();
        self.check_ownership_patterns();
        self.check_fee_transparency();
        self.check_license_compliance();
        self.check_scam_patterns();

        let mut report = AuditReport::new(&self.project_name, &self.source_code);
        for finding in self.findings.drain(..) {
            report.add_finding(finding);
        }
        for (k, v) in self.metadata.drain() {
            report.set_metadata(&k, &v);
        }
        report.finalize();
        info!(
            "Audit complete for {}: score={}, level={}",
            report.project_name, report.risk_score, report.risk_level
        );
        report
    }

    pub fn check_static_analysis(&mut self) {
        info!("Running static analysis...");
        for (line, desc) in PatternDetector::detect_integer_overflows(&self.source_code) {
            let score = if desc.contains("unchecked") { 60 } else { 30 };
            self.findings.push(Finding {
                id: format!("SA-{:04}", self.findings.len() + 1),
                title: "Integer overflow risk".into(),
                description: desc.clone(),
                severity: Severity::from_score(score),
                category: FindingCategory::IntegerOverflow,
                location: Some(SourceLocation {
                    file: "contract.sol".into(),
                    line,
                    column: None,
                    snippet: None,
                }),
                remediation: Some(
                    "Use SafeMath or Solidity 0.8+ built-in overflow checks".into(),
                ),
                score,
                raw_snippet: None,
            });
        }
        for (line, desc) in PatternDetector::detect_unchecked_external_calls(&self.source_code) {
            self.findings.push(Finding {
                id: format!("SA-{:04}", self.findings.len() + 1),
                title: "Unchecked external call".into(),
                description: desc.clone(),
                severity: Severity::High,
                category: FindingCategory::UncheckedExternalCall,
                location: Some(SourceLocation {
                    file: "contract.sol".into(),
                    line,
                    column: None,
                    snippet: None,
                }),
                remediation: Some(
                    "Always check the return value of external calls using require() or if statement"
                        .into(),
                ),
                score: 75,
                raw_snippet: None,
            });
        }
    }

    pub fn check_reentrancy_patterns(&mut self) {
        info!("Checking reentrancy patterns...");
        for (line, desc) in PatternDetector::detect_reentrancy(&self.source_code) {
            let score = if desc.contains("delegatecall") { 90 } else { 70 };
            self.findings.push(Finding {
                id: format!("RE-{:04}", self.findings.len() + 1),
                title: "Reentrancy vulnerability".into(),
                description: desc.clone(),
                severity: Severity::from_score(score),
                category: FindingCategory::Reentrancy,
                location: Some(SourceLocation {
                    file: "contract.sol".into(),
                    line,
                    column: None,
                    snippet: None,
                }),
                remediation: Some(
                    "Use Checks-Effects-Interactions pattern or ReentrancyGuard".into(),
                ),
                score,
                raw_snippet: None,
            });
        }
    }

    pub fn check_ownership_patterns(&mut self) {
        info!("Checking ownership patterns...");
        for (line, desc) in PatternDetector::detect_ownership_patterns(&self.source_code) {
            let score = if desc.contains("renounceOwnership") {
                80
            } else if desc.contains("onlyOwner") {
                40
            } else {
                20
            };
            let category = if desc.contains("renounceOwnership") {
                FindingCategory::CentralizationRisk
            } else {
                FindingCategory::Ownership
            };
            self.findings.push(Finding {
                id: format!("OW-{:04}", self.findings.len() + 1),
                title: "Ownership pattern detected".into(),
                description: desc.clone(),
                severity: Severity::from_score(score),
                category,
                location: Some(SourceLocation {
                    file: "contract.sol".into(),
                    line,
                    column: None,
                    snippet: None,
                }),
                remediation: Some(
                    "Consider using two-step ownership transfer and timelocks".into(),
                ),
                score,
                raw_snippet: None,
            });
        }
        for issue in &PrincipalSafetyChecker::verify(&self.source_code).issues {
            self.findings.push(Finding {
                id: format!("OW-{:04}", self.findings.len() + 1),
                title: "Principal safety concern".into(),
                description: issue.clone(),
                severity: Severity::Medium,
                category: FindingCategory::CentralizationRisk,
                location: None,
                remediation: Some(
                    "Implement missing safety mechanisms as appropriate".into(),
                ),
                score: 50,
                raw_snippet: None,
            });
        }
    }

    pub fn check_fee_transparency(&mut self) {
        info!("Checking fee transparency...");
        let fee_result = FeeTransparencyChecker::verify(&self.source_code);
        if !fee_result.has_fee_declaration {
            self.findings.push(Finding {
                id: format!("FT-{:04}", self.findings.len() + 1),
                title: "Missing fee declaration".into(),
                description:
                    "No fee declaration or fee-related variables found in the source code".into(),
                severity: Severity::High,
                category: FindingCategory::FeeTransparency,
                location: None,
                remediation: Some(
                    "Clearly declare all fees as named constants or state variables".into(),
                ),
                score: 75,
                raw_snippet: None,
            });
        }
        if fee_result.has_mutable_fees && !fee_result.has_fee_cap {
            self.findings.push(Finding {
                id: format!("FT-{:04}", self.findings.len() + 1),
                title: "Mutable fees without cap".into(),
                description: "Fee functions can be changed without a maximum cap".into(),
                severity: Severity::High,
                category: FindingCategory::FeeTransparency,
                location: None,
                remediation: Some("Add a MAX_FEE constant and enforce it in setter functions".into()),
                score: 70,
                raw_snippet: None,
            });
        }
        if fee_result.has_mutable_fees && !fee_result.has_fee_events {
            self.findings.push(Finding {
                id: format!("FT-{:04}", self.findings.len() + 1),
                title: "Missing fee change events".into(),
                description: "Fee changes do not emit events for transparency".into(),
                severity: Severity::Medium,
                category: FindingCategory::FeeTransparency,
                location: None,
                remediation: Some(
                    "Emit events when fees are changed for off-chain monitoring".into(),
                ),
                score: 50,
                raw_snippet: None,
            });
        }
        for (line, func) in &fee_result.fee_functions {
            self.findings.push(Finding {
                id: format!("FT-{:04}", self.findings.len() + 1),
                title: "Fee modification function".into(),
                description: format!("Fee function '{}' can modify fees", func),
                severity: Severity::Low,
                category: FindingCategory::FeeTransparency,
                location: Some(SourceLocation {
                    file: "contract.sol".into(),
                    line: *line,
                    column: None,
                    snippet: None,
                }),
                remediation: Some(
                    "Ensure fee changes are timelocked and emit events".into(),
                ),
                score: 30,
                raw_snippet: None,
            });
        }
    }

    pub fn check_license_compliance(&mut self) {
        info!("Checking license compliance...");
        let licenses = PatternDetector::detect_license(&self.source_code);
        if licenses.is_empty() || licenses.iter().any(|(_, d)| d.contains("No SPDX")) {
            self.findings.push(Finding {
                id: format!("LC-{:04}", self.findings.len() + 1),
                title: "Missing license identifier".into(),
                description: "No SPDX-License-Identifier found in source code".into(),
                severity: Severity::Medium,
                category: FindingCategory::LicenseCompliance,
                location: None,
                remediation: Some(
                    "Add a SPDX-License-Identifier comment at the top of the file".into(),
                ),
                score: 40,
                raw_snippet: None,
            });
        }
        for (line, desc) in &licenses {
            if desc.contains("UNLICENSED") {
                self.findings.push(Finding {
                    id: format!("LC-{:04}", self.findings.len() + 1),
                    title: "Unlicensed source code".into(),
                    description: desc.clone(),
                    severity: Severity::Low,
                    category: FindingCategory::LicenseCompliance,
                    location: Some(SourceLocation {
                        file: "contract.sol".into(),
                        line: *line,
                        column: None,
                        snippet: None,
                    }),
                    remediation: Some(
                        "Consider using an open source license like MIT".into(),
                    ),
                    score: 20,
                    raw_snippet: None,
                });
            }
        }
    }

    pub fn check_scam_patterns(&mut self) {
        info!("Checking scam patterns...");
        for (line, desc) in PatternDetector::detect_scam_patterns(&self.source_code) {
            let score = if desc.contains("selfdestruct") || desc.contains("delegatecall") {
                85
            } else if desc.contains("honeypot") || desc.contains("rug") {
                95
            } else if desc.contains("tx.origin") {
                70
            } else {
                50
            };
            self.findings.push(Finding {
                id: format!("SP-{:04}", self.findings.len() + 1),
                title: "Suspicious pattern detected".into(),
                description: desc.clone(),
                severity: Severity::from_score(score),
                category: FindingCategory::ScamPattern,
                location: Some(SourceLocation {
                    file: "contract.sol".into(),
                    line,
                    column: None,
                    snippet: None,
                }),
                remediation: Some("Review this pattern carefully for malicious intent".into()),
                score,
                raw_snippet: None,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_severity_from_score() {
        assert_eq!(Severity::from_score(95), Severity::Critical);
        assert_eq!(Severity::from_score(75), Severity::High);
        assert_eq!(Severity::from_score(50), Severity::Medium);
        assert_eq!(Severity::from_score(20), Severity::Low);
        assert_eq!(Severity::from_score(5), Severity::Info);
    }

    #[test]
    fn test_severity_as_str() {
        assert_eq!(Severity::Critical.as_str(), "CRITICAL");
        assert_eq!(Severity::High.as_str(), "HIGH");
    }

    #[test]
    fn test_finding_category_as_str() {
        assert_eq!(FindingCategory::Reentrancy.as_str(), "Reentrancy");
        assert_eq!(FindingCategory::ScamPattern.as_str(), "Scam Pattern");
    }

    #[test]
    fn test_audit_report_new() {
        let r = AuditReport::new("test", "contract {}");
        assert_eq!(r.project_name, "test");
        assert_eq!(r.source_hash.len(), 64);
    }

    #[test]
    fn test_audit_report_add_finding() {
        let mut r = AuditReport::new("test", "");
        r.add_finding(Finding {
            id: "F1".into(),
            title: "Test".into(),
            description: "Desc".into(),
            severity: Severity::High,
            category: FindingCategory::Other,
            location: None,
            remediation: None,
            score: 75,
            raw_snippet: None,
        });
        assert_eq!(r.summary.high, 1);
    }

    #[test]
    fn test_audit_report_finalize() {
        let mut r = AuditReport::new("test", "");
        r.add_finding(Finding {
            id: "F1".into(),
            title: "Test".into(),
            description: "Desc".into(),
            severity: Severity::High,
            category: FindingCategory::Other,
            location: None,
            remediation: None,
            score: 75,
            raw_snippet: None,
        });
        r.finalize();
        assert!(r.risk_score > 0);
        assert_eq!(r.passed, false);
    }

    #[test]
    fn test_risk_scorer_empty() {
        assert_eq!(RiskScorer::calculate(&[]), 0);
    }

    #[test]
    fn test_risk_scorer_single() {
        let f = Finding {
            id: "F1".into(),
            title: "".into(),
            description: "".into(),
            severity: Severity::High,
            category: FindingCategory::Other,
            location: None,
            remediation: None,
            score: 80,
            raw_snippet: None,
        };
        assert_eq!(RiskScorer::calculate(&[f]), 80);
    }

    #[test]
    fn test_risk_level() {
        assert_eq!(RiskScorer::risk_level(95), "CRITICAL");
        assert_eq!(RiskScorer::risk_level(75), "HIGH");
        assert_eq!(RiskScorer::risk_level(50), "MEDIUM");
        assert_eq!(RiskScorer::risk_level(20), "LOW");
        assert_eq!(RiskScorer::risk_level(5), "PASS");
    }

    #[test]
    fn test_pattern_detector_reentrancy() {
        let source = "function withdraw() { msg.sender.call{value: amount}(); }";
        let findings = PatternDetector::detect_reentrancy(source);
        assert!(!findings.is_empty());
    }

    #[test]
    fn test_pattern_detector_ownership() {
        let source = "contract MyContract is Ownable { function close() onlyOwner { } }";
        let findings = PatternDetector::detect_ownership_patterns(source);
        assert!(!findings.is_empty());
    }

    #[test]
    fn test_pattern_detector_scam() {
        let source = "function destroy() { selfdestruct(address(0)); }";
        let findings = PatternDetector::detect_scam_patterns(source);
        assert!(!findings.is_empty());
    }

    #[test]
    fn test_pattern_detector_license() {
        let source = "// SPDX-License-Identifier: MIT";
        let findings = PatternDetector::detect_license(source);
        assert!(!findings.is_empty());
        assert!(findings[0].1.contains("MIT"));
    }

    #[test]
    fn test_pattern_detector_no_license() {
        let source = "contract Foo { }";
        let findings = PatternDetector::detect_license(source);
        assert!(findings.iter().any(|(_, d)| d.contains("No SPDX")));
    }

    #[test]
    fn test_fee_transparency_checker() {
        let source = "uint256 public platformFee = 500; function setFee(uint256 _fee) onlyOwner { }";
        let result = FeeTransparencyChecker::verify(source);
        assert!(result.has_fee_declaration);
        assert!(result.has_mutable_fees);
    }

    #[test]
    fn test_principal_safety_checker() {
        let source = "contract Test is Ownable, Pausable { }";
        let result = PrincipalSafetyChecker::verify(source);
        assert!(result.has_owner);
        assert!(result.has_pausable);
    }

    #[test]
    fn test_foundry_auditor_new() {
        let auditor = FoundryAuditor::new("test-project", "contract Test { }");
        assert_eq!(auditor.project_name, "test-project");
    }

    #[test]
    fn test_foundry_auditor_audit_clean() {
        let mut auditor = FoundryAuditor::new("clean", "contract Clean { uint256 public x; }");
        let report = auditor.audit_project();
        assert_eq!(report.project_name, "clean");
        assert!(report.source_hash.len() == 64);
    }

    #[test]
    fn test_foundry_auditor_audit_with_issues() {
        let source = "contract Bad { function withdraw() { msg.sender.call{value: 0}(); } function kill() { selfdestruct(address(0)); } }";
        let mut auditor = FoundryAuditor::new("bad", source);
        let report = auditor.audit_project();
        assert!(!report.findings.is_empty());
        assert!(report.risk_score > 0);
    }

    #[test]
    fn test_foundry_auditor_metadata() {
        let mut auditor = FoundryAuditor::new("meta", "contract M { }");
        auditor.add_metadata("version", "1.0.0");
        let report = auditor.audit_project();
        assert_eq!(report.metadata.get("version").unwrap(), "1.0.0");
    }

    #[test]
    fn test_detect_fee_patterns() {
        let source = "uint256 public platformFee = 500;";
        let findings = PatternDetector::detect_fee_patterns(source);
        assert!(!findings.is_empty());
    }

    #[test]
    fn test_detect_integer_overflows() {
        let source = "uint8 public small;";
        let findings = PatternDetector::detect_integer_overflows(source);
        assert!(!findings.is_empty());
    }

    #[test]
    fn test_detect_unchecked_external_calls() {
        let source = "addr.call{value: amount}();";
        let findings = PatternDetector::detect_unchecked_external_calls(source);
        assert!(!findings.is_empty());
    }
}
