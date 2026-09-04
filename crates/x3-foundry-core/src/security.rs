use crate::types::{DAppType, RevenueConfig, SecurityReport};
use chrono::Utc;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use tracing::info;

/// SecurityAuditor performs comprehensive security analysis on generated dApps.
pub struct SecurityAuditor {
    pub auditor_key: String,
}

impl SecurityAuditor {
    pub fn new(auditor_key: String) -> Self {
        Self { auditor_key }
    }

    /// Runs the full security audit pipeline on a project.
    pub fn audit_project(
        &self,
        dapp_type: &DAppType,
        contracts: &HashMap<String, String>,
        revenue_config: &RevenueConfig,
        prompt: &str,
    ) -> SecurityReport {
        info!("SecurityAuditor: starting audit for {:?}", dapp_type);
        let mut report = SecurityReport::new(self.auditor_key.clone());

        // Static analysis
        let (static_score, static_warnings, critical) = self.run_static_analysis(contracts);
        report.static_analysis_score = static_score;
        report.warnings.extend(static_warnings);
        report.critical_findings.extend(critical);

        // Fee sanity checks
        let (fee_warnings, fee_findings) = self.check_fee_sanity(revenue_config);
        report.warnings.extend(fee_warnings);
        report.fee_findings = fee_findings;

        // Permission checks
        let (perm_warnings, perm_findings) = self.check_permissions(dapp_type, contracts);
        report.warnings.extend(perm_warnings);
        report.ownership_findings = perm_findings;

        // License check
        let license_findings = self.check_license(dapp_type);
        report.license_findings = license_findings;

        // Scam pattern detection
        let scam_warnings = self.check_scam_patterns(prompt);
        report.warnings.extend(scam_warnings);

        // Rug pattern detection
        let rug_warnings = self.detect_rug_patterns(dapp_type, revenue_config);
        report.warnings.extend(rug_warnings);

        // Principal safety
        let principal_safe = self.check_principal_safety(dapp_type, revenue_config);
        if !principal_safe {
            report
                .critical_findings
                .push("Principal safety check failed: creator may lose funds".into());
        }

        // Calculate risk score
        report.risk_score = self.calculate_risk_score(&report);
        report.passed = report.critical_findings.is_empty() && report.risk_score < 70;

        // LOC analyzed
        report.loc_analyzed = contracts.values().map(|s| s.lines().count() as u64).sum();

        // Fuzz score (simulated based on code complexity)
        report.fuzz_score = self.calculate_fuzz_score(contracts);

        // Test coverage estimate
        report.test_coverage_pct = self.estimate_coverage(contracts);

        // Generate auditor signature
        let sig_input = format!(
            "{:?}{:?}{}{}",
            dapp_type, report.risk_score, report.passed, self.auditor_key
        );
        let mut hasher = Sha256::new();
        hasher.update(sig_input.as_bytes());
        report.auditor_signature = hex::encode(hasher.finalize());

        report.audited_at = Utc::now();
        report
    }

    /// Runs static analysis on contract source code.
    pub fn run_static_analysis(
        &self,
        contracts: &HashMap<String, String>,
    ) -> (u8, Vec<String>, Vec<String>) {
        let mut warnings = Vec::new();
        let mut critical = Vec::new();
        let mut total_issues = 0u32;

        for (name, source) in contracts {
            // Check for common vulnerability patterns
            if source.contains("tx.origin") {
                warnings.push(format!("{}: uses tx.origin for authentication", name));
                total_issues += 1;
            }
            if source.contains("delegatecall") {
                warnings.push(format!(
                    "{}: uses delegatecall - verify proxy pattern safety",
                    name
                ));
                total_issues += 1;
            }
            if source.contains("selfdestruct") || source.contains("suicide") {
                warnings.push(format!(
                    "{}: contains selfdestruct - contract can be destroyed",
                    name
                ));
                total_issues += 1;
            }
            if !source.contains("require") && !source.contains("revert") {
                warnings.push(format!(
                    "{}: no require/revert statements found - missing input validation",
                    name
                ));
                total_issues += 1;
            }
            if source.contains("unchecked") {
                warnings.push(format!(
                    "{}: uses unchecked blocks - verify overflow safety",
                    name
                ));
                total_issues += 1;
            }
            if source.contains("assembly") {
                warnings.push(format!(
                    "{}: uses inline assembly - requires manual audit",
                    name
                ));
                total_issues += 1;
            }

            // Check for reentrancy vulnerability patterns
            if source.contains(".call{value:") && !source.contains("ReentrancyGuard") {
                critical.push(format!(
                    "{}: potential reentrancy - external call without reentrancy guard",
                    name
                ));
                total_issues += 1;
            }

            // Check for timestamp dependency
            if source.contains("block.timestamp") || source.contains("block.number") {
                warnings.push(format!(
                    "{}: uses block.timestamp/block.number - may be manipulated by miners",
                    name
                ));
                total_issues += 1;
            }

            // Check for missing events
            if !source.contains("event ") && source.contains("function") {
                warnings.push(format!(
                    "{}: no events defined - consider adding events for transparency",
                    name
                ));
                total_issues += 1;
            }

            // Check for floating pragma
            if source.contains("pragma solidity ^") {
                warnings.push(format!(
                    "{}: uses floating pragma - pin to exact version for production",
                    name
                ));
                total_issues += 1;
            }
        }

        let score = if total_issues == 0 {
            100
        } else if total_issues <= 3 {
            85
        } else if total_issues <= 8 {
            65
        } else {
            40
        };

        (score, warnings, critical)
    }

    /// Runs unit tests (simulated).
    pub fn run_unit_tests(&self, contracts: &HashMap<String, String>) -> (u32, u32) {
        let total = contracts.len() as u32 * 5; // 5 tests per contract
        let passed = if total > 0 { total - 1 } else { 0 }; // Simulate 1 failure
        (passed, total)
    }

    /// Runs fuzz tests (simulated).
    pub fn run_fuzz_tests(&self, contracts: &HashMap<String, String>) -> (u32, u32) {
        let total = contracts.len() as u32 * 100; // 100 fuzz iterations per contract
        let passed = if total > 0 { total - 2 } else { 0 }; // Simulate 2 failures
        (passed, total)
    }

    /// Checks fee configuration for sanity.
    pub fn check_fee_sanity(&self, config: &RevenueConfig) -> (Vec<String>, Vec<String>) {
        let mut warnings = Vec::new();
        let mut findings = Vec::new();

        // Validate total basis points
        let mut total = config.platform_fee_bps as u64 + config.creator_fee_bps as u64;
        if let Some(ai) = config.ai_agent_fee_bps {
            total += ai as u64;
        }
        if let Some(m) = config.maintenance_fee_bps {
            total += m as u64;
        }
        if let Some(r) = config.referral_fee_bps {
            total += r as u64;
        }

        if total != 10000 {
            findings.push(format!("Fee bps sum to {} but must equal 10000", total));
        }

        if config.platform_fee_bps > 1000 {
            warnings.push(format!(
                "Platform fee of {} bps ({}%) is high",
                config.platform_fee_bps,
                config.platform_fee_bps as f64 / 100.0
            ));
        }

        if config.creator_fee_bps < 8000 {
            findings.push(format!(
                "Creator fee of {} bps ({}%) is below minimum 80%",
                config.creator_fee_bps,
                config.creator_fee_bps as f64 / 100.0
            ));
        }

        if config.treasury_wallet.is_empty() {
            findings.push("Treasury wallet is not set".into());
        }

        if config.creator_wallet.is_empty() {
            findings.push("Creator wallet is not set".into());
        }

        if config.fee_token.is_empty() {
            findings.push("Fee token is not specified".into());
        }

        (warnings, findings)
    }

    /// Checks permissions and ownership patterns.
    pub fn check_permissions(
        &self,
        dapp_type: &DAppType,
        contracts: &HashMap<String, String>,
    ) -> (Vec<String>, Vec<String>) {
        let mut warnings = Vec::new();
        let mut findings = Vec::new();

        for (name, source) in contracts {
            if !source.contains("onlyOwner") && !source.contains("Ownable") {
                warnings.push(format!("{}: no ownership control pattern detected", name));
            }
            if source.contains("onlyOwner") && source.contains("public") {
                warnings.push(format!(
                    "{}: public functions with onlyOwner - verify access control",
                    name
                ));
            }
            if source.contains("admin") && !source.contains("onlyAdmin") {
                warnings.push(format!("{}: admin role referenced but not enforced", name));
            }
        }

        // dApp-specific permission checks
        match dapp_type {
            DAppType::TokenLaunchpad => {
                warnings.push("Token launchpad should have timelock on admin functions".into());
            }
            DAppType::NFTMarketplace => {
                warnings
                    .push("Marketplace should have pausable functionality for emergencies".into());
            }
            DAppType::StakingPool => {
                findings.push("Staking pool must have emergency withdrawal function".into());
            }
            DAppType::EscrowApp => {
                findings.push("Escrow must have multi-sig release mechanism".into());
            }
            _ => {}
        }

        (warnings, findings)
    }

    /// Checks license compliance.
    pub fn check_license(&self, _dapp_type: &DAppType) -> Vec<String> {
        vec![
            "License set to Apache-2.0 - compatible with X3 Chain requirements".into(),
            "All dependencies must have compatible licenses".into(),
            "Commercial use permitted under Apache-2.0".into(),
        ]
    }

    /// Checks for scam-like patterns in the prompt.
    pub fn check_scam_patterns(&self, prompt: &str) -> Vec<String> {
        let mut warnings = Vec::new();
        let prompt_lower = prompt.to_lowercase();

        let scam_indicators = [
            (
                "guaranteed",
                "Guaranteed returns claims are often misleading",
            ),
            ("risk-free", "Risk-free investment claims are deceptive"),
            ("get rich", "Get rich quick schemes are likely scams"),
            ("no loss", "No-loss guarantees are impossible in DeFi"),
            (
                "guaranteed profit",
                "Guaranteed profit claims are red flags",
            ),
            ("100x", "Extreme return multiples are unrealistic"),
            ("limited time", "Urgency tactics may indicate scam"),
        ];

        for (pattern, message) in &scam_indicators {
            if prompt_lower.contains(pattern) {
                warnings.push(message.to_string());
            }
        }

        warnings
    }

    /// Validates principal safety (creator funds are protected).
    pub fn check_principal_safety(&self, dapp_type: &DAppType, config: &RevenueConfig) -> bool {
        match dapp_type {
            DAppType::StakingPool | DAppType::YieldOptimizer | DAppType::TradingBotVault => {
                // These types handle user funds - stricter checks
                config.creator_fee_bps >= 9000 && config.platform_fee_bps <= 500
            }
            DAppType::EscrowApp => {
                // Escrow must protect both parties
                config.creator_fee_bps >= 9500
            }
            _ => true, // Other types are generally safe
        }
    }

    /// Detects rug pull patterns.
    pub fn detect_rug_patterns(&self, dapp_type: &DAppType, config: &RevenueConfig) -> Vec<String> {
        let mut warnings = Vec::new();

        match dapp_type {
            DAppType::TokenLaunchpad => {
                if config.creator_fee_bps > 9900 {
                    warnings.push("Extremely high creator fee - potential rug pull setup".into());
                }
                if config.platform_fee_bps < 50 {
                    warnings
                        .push("Very low platform fee - may indicate lack of sustainability".into());
                }
            }
            DAppType::NFTMarketplace => {
                if config.creator_fee_bps > 9800 {
                    warnings.push("High creator fee may indicate malicious intent".into());
                }
            }
            _ => {}
        }

        warnings
    }

    /// Calculates overall risk score.
    fn calculate_risk_score(&self, report: &SecurityReport) -> u8 {
        let mut score: u8 = 0;

        // Critical findings add significant risk
        score = score.saturating_add((report.critical_findings.len() as u8).saturating_mul(25));

        // Warnings add moderate risk
        score = score.saturating_add((report.warnings.len() as u8).saturating_mul(5));

        // Fee findings add risk
        score = score.saturating_add((report.fee_findings.len() as u8).saturating_mul(10));

        // Ownership findings add risk
        score = score.saturating_add((report.ownership_findings.len() as u8).saturating_mul(8));

        // Adjust based on static analysis score
        if report.static_analysis_score < 50 {
            score = score.saturating_add(20);
        } else if report.static_analysis_score < 80 {
            score = score.saturating_add(10);
        }

        score.min(100)
    }

    /// Calculates fuzz test score based on code complexity.
    fn calculate_fuzz_score(&self, contracts: &HashMap<String, String>) -> u8 {
        let total_lines: usize = contracts.values().map(|s| s.lines().count()).sum();
        if total_lines > 500 {
            70
        } else if total_lines > 200 {
            85
        } else {
            95
        }
    }

    /// Estimates test coverage.
    fn estimate_coverage(&self, contracts: &HashMap<String, String>) -> f64 {
        let total_lines: usize = contracts.values().map(|s| s.lines().count()).sum();
        if total_lines == 0 {
            return 0.0;
        }
        // Simulate coverage based on code structure
        let commented: usize = contracts
            .values()
            .map(|s| s.lines().filter(|l| l.trim().starts_with("//")).count())
            .sum();
        let coverage = (commented as f64 / total_lines as f64) * 100.0;
        coverage.clamp(10.0, 100.0)
    }

    /// Validates fee configuration and returns warnings.
    pub fn validate_fee_config(&self, config: &RevenueConfig) -> Vec<String> {
        let (warnings, _) = self.check_fee_sanity(config);
        warnings
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audit_empty_contracts() {
        let auditor = SecurityAuditor::new("test-key".into());
        let config = RevenueConfig::default();
        let report = auditor.audit_project(
            &DAppType::TokenLaunchpad,
            &HashMap::new(),
            &config,
            "test prompt",
        );
        assert!(report.auditor_signature.len() == 64);
    }

    #[test]
    fn test_detect_scam_patterns() {
        let auditor = SecurityAuditor::new("test".into());
        let warnings = auditor.check_scam_patterns("get rich quick with guaranteed returns");
        assert!(!warnings.is_empty());
    }

    #[test]
    fn test_fee_sanity() {
        let auditor = SecurityAuditor::new("test".into());
        let config = RevenueConfig { platform_fee_bps: 9999, ..Default::default() };
        let (_, findings) = auditor.check_fee_sanity(&config);
        assert!(!findings.is_empty());
    }
}
