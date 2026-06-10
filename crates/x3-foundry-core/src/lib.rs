//! X3 Foundry Core - AI-powered dApp factory for Atlas Sphere.
//!
//! This crate provides the core engine for generating, auditing, simulating,
//! and deploying decentralized applications on the X3 Chain and other
//! blockchain networks.

pub mod deployer;
pub mod error;
pub mod generator;
pub mod revenue;
pub mod security;
pub mod simulator;
pub mod templates;
pub mod types;

use crate::deployer::{CrossChainDeployer, Deployer, DeploymentManifest};
use crate::error::FoundryError;
use crate::generator::{
    ArchitecturePlan, ComplianceReport, ContractOutput, DeploymentPackage, FrontendOutput,
    Generator, MarketplaceOutput, TokenomicsOutput,
};
use crate::revenue::{FeeConfig, RevenueTracker, TreasurySplit};
use crate::security::SecurityAuditor;
use crate::simulator::Simulator;
use crate::templates::TemplateRegistry;
use crate::types::{
    AppHealthScore, DAppType, DeploymentReceipt, FeeMode, MarketplaceListing, PricingTier,
    ProjectState, ProjectStatus, RevenueConfig, RevenueReport, SecurityReport, SimulationResult,
};
use chrono::Utc;
use std::collections::HashMap;
use tracing::{error, info};

/// The main FoundryEngine that orchestrates the full dApp generation pipeline.
///
/// Pipeline: generate -> audit -> simulate -> deploy
pub struct FoundryEngine {
    pub generator: Generator,
    pub auditor: SecurityAuditor,
    pub simulator: Simulator,
    pub deployer: Deployer,
    pub cross_chain_deployer: CrossChainDeployer,
    pub revenue_tracker: RevenueTracker,
    pub template_registry: TemplateRegistry,
}

impl FoundryEngine {
    /// Creates a new FoundryEngine with default configuration.
    pub fn new(deployer_key: String, auditor_key: String) -> Self {
        info!("Initializing FoundryEngine");
        let template_registry = TemplateRegistry::default_registry();
        let deployer = Deployer::new(deployer_key.clone(), "x3-testnet".to_string());
        let auditor = SecurityAuditor::new(auditor_key);
        let simulator = Simulator::new();
        let revenue_tracker = RevenueTracker::new(FeeConfig::default(), TreasurySplit::default());

        let mut cross_chain_deployer = CrossChainDeployer::new();
        cross_chain_deployer.add_chain("x3-testnet".to_string(), Deployer::new(deployer_key.clone(), "x3-testnet".to_string()));
        cross_chain_deployer.add_chain("x3-mainnet".to_string(), Deployer::new(deployer_key, "x3-mainnet".to_string()));

        Self {
            generator: Generator::new(),
            auditor,
            simulator,
            deployer,
            cross_chain_deployer,
            revenue_tracker,
            template_registry,
        }
    }

    /// Runs the full pipeline: generate -> audit -> simulate -> deploy.
    ///
    /// Takes a user prompt and creator wallet address, and returns the complete
    /// deployment receipt if successful.
    pub fn run(&self, prompt: &str, creator_wallet: &str) -> Result<PipelineResult, FoundryError> {
        info!("FoundryEngine: starting full pipeline for prompt: {}", prompt);

        // Step 1: Generate
        info!("Step 1/4: Generating dApp from prompt...");
        let (plan, contracts, frontend, tokenomics, deployment_pkg, marketplace, compliance) =
            self.generator.generate_all(prompt, creator_wallet)?;

        if !compliance.passed {
            error!("Compliance check failed: {:?}", compliance.violations);
            return Err(FoundryError::ComplianceFailed(format!(
                "Compliance violations: {:?}",
                compliance.violations
            )));
        }

        info!("Generation complete: {} ({})", plan.name, plan.dapp_type.label());

        // Step 2: Audit
        info!("Step 2/4: Auditing dApp...");
        let security_report = self.auditor.audit_project(
            &plan.dapp_type,
            &contracts.contracts,
            &tokenomics.revenue_config,
            prompt,
        );

        if !security_report.passed {
            error!("Security audit failed with risk score: {}", security_report.risk_score);
            return Err(FoundryError::SecurityAuditFailed(format!(
                "Security audit failed. Risk score: {}. Critical findings: {:?}",
                security_report.risk_score, security_report.critical_findings
            )));
        }

        info!("Audit passed with risk score: {}", security_report.risk_score);

        // Step 3: Simulate
        info!("Step 3/4: Simulating dApp...");
        let user_base_estimate = self.estimate_user_base(prompt);
        let simulation_result = self.simulator.simulate_project(
            &plan.dapp_type,
            &tokenomics.revenue_config,
            user_base_estimate,
        )?;

        info!(
            "Simulation complete: expected daily volume: {}, confidence: {:.2}%",
            simulation_result.expected_volume,
            simulation_result.confidence_score * 100.0
        );

        // Step 4: Deploy
        info!("Step 4/4: Deploying dApp...");
        let target_chain = plan.target_chains.first().cloned().unwrap_or_else(|| "x3-testnet".to_string());
        let deployed_contracts = self.deployer.deploy_contracts(
            &contracts.contracts,
            &contracts.deployment_order,
            &target_chain,
        )?;

        let frontend_url = self.deployer.deploy_frontend(
            &plan.name,
            &plan.frontend_framework,
            &frontend.routes,
            &frontend.api_endpoints,
        )?;

        let metadata_uri = self.deployer.deploy_metadata(
            &plan.name,
            &plan.description,
            &plan.features,
        )?;

        let treasury_hooks = self.deployer.deploy_treasury_hooks(
            &tokenomics.revenue_config.treasury_wallet,
            tokenomics.revenue_config.platform_fee_bps,
            &target_chain,
        )?;

        let marketplace_listing_id = self.deployer.deploy_marketplace_listing(
            &marketplace.title,
            &marketplace.description,
            &marketplace.tags,
            &target_chain,
        )?;

        let analytics_endpoint = self.deployer.deploy_analytics(&plan.name, &target_chain)?;

        // Generate manifest
        let manifest = self.deployer.generate_manifest(
            &plan.name,
            &plan.dapp_type,
            &deployed_contracts,
            Some(frontend_url.clone()),
            Some(metadata_uri.clone()),
            treasury_hooks,
            Some(marketplace_listing_id.clone()),
            Some(analytics_endpoint.clone()),
            &target_chain,
        );

        // Build deployment receipt
        let mut contract_addresses = HashMap::new();
        let mut tx_hashes = Vec::new();
        let mut total_gas = 0u64;

        for contract in &deployed_contracts {
            contract_addresses.insert(contract.name.clone(), contract.address.clone());
            tx_hashes.push(contract.tx_hash.clone());
            total_gas += contract.gas_used;
        }

        let mut receipt = DeploymentReceipt {
            app_id: plan.name.clone(),
            app_name: plan.name.clone(),
            dapp_type: plan.dapp_type.clone(),
            chain: target_chain.clone(),
            contract_addresses,
            tx_hashes,
            frontend_url: Some(frontend_url),
            metadata_uri: Some(metadata_uri),
            marketplace_listing_id: Some(marketplace_listing_id),
            analytics_endpoint: Some(analytics_endpoint),
            deployed_at: Utc::now(),
            deployer_address: self.deployer.deployer_key.clone(),
            signature: String::new(),
            signed_at: Utc::now(),
            manifest_hash: manifest.manifest_hash.clone(),
            block_number: deployed_contracts.first().map(|c| c.block_number).unwrap_or(0),
            gas_used: total_gas,
        };

        self.deployer.sign_receipt(&mut receipt);

        info!("Pipeline complete! dApp deployed successfully.");
        info!("  Name: {}", receipt.app_name);
        info!("  Type: {}", receipt.dapp_type);
        info!("  Chain: {}", receipt.chain);
        info!("  Contracts: {}", receipt.contract_addresses.len());
        info!("  Frontend: {:?}", receipt.frontend_url);
        info!("  Receipt Signature: {}", receipt.signature);

        Ok(PipelineResult {
            plan,
            contracts,
            frontend,
            tokenomics,
            deployment_pkg,
            marketplace,
            compliance,
            security_report,
            simulation_result,
            receipt,
            manifest,
        })
    }

    /// Estimates the user base from the prompt.
    fn estimate_user_base(&self, prompt: &str) -> u64 {
        let prompt_lower = prompt.to_lowercase();
        // Try to extract a number from the prompt
        for word in prompt_lower.split_whitespace() {
            if let Ok(num) = word.parse::<u64>() {
                if num > 0 && num < 1_000_000_000 {
                    return num;
                }
            }
        }
        // Default estimates based on keywords
        if prompt_lower.contains("enterprise") || prompt_lower.contains("large") {
            100_000
        } else if prompt_lower.contains("medium") || prompt_lower.contains("growing") {
            10_000
        } else if prompt_lower.contains("small") || prompt_lower.contains("niche") {
            1_000
        } else {
            5_000 // Default moderate estimate
        }
    }

    /// Creates a marketplace listing for a deployed dApp.
    pub fn create_marketplace_listing(
        &self,
        title: String,
        description: String,
        dapp_type: DAppType,
        creator_wallet: String,
        pricing_tier: PricingTier,
    ) -> MarketplaceListing {
        let mut listing = MarketplaceListing::new(title, uuid::Uuid::new_v4().to_string(), dapp_type, creator_wallet);
        listing.description = description;
        listing.pricing_tier = pricing_tier;
        listing
    }

    /// Generates a revenue report for an app.
    pub fn generate_revenue_report(&self, app_id: &str) -> RevenueReport {
        let summary = self.revenue_tracker.get_app_revenue_summary(app_id);
        RevenueReport {
            app_id: app_id.to_string(),
            app_name: app_id.to_string(),
            total_volume: summary.total_volume,
            total_fees_collected: summary.total_fees,
            platform_revenue: summary.platform_revenue,
            creator_revenue: summary.creator_revenue,
            ai_agent_revenue: 0,
            maintenance_revenue: 0,
            referral_payouts: 0,
            transaction_count: summary.transaction_count,
            unique_users: 0,
            period_start: Utc::now(),
            period_end: Utc::now(),
            generated_at: Utc::now(),
        }
    }

    /// Calculates the health score for an app.
    pub fn calculate_app_health(&self, app_id: &str) -> AppHealthScore {
        let mut health = AppHealthScore::new(app_id.to_string());
        let summary = self.revenue_tracker.get_app_revenue_summary(app_id);

        // Calculate revenue health
        if summary.total_volume > 0 {
            health.revenue_health = 85.0;
        } else {
            health.revenue_health = 50.0;
            health.warnings.push("No revenue generated yet".into());
        }

        // Calculate overall score
        health.overall_score = (health.uptime_score
            + health.transaction_success_rate
            + health.gas_efficiency
            + health.security_score
            + health.revenue_health
            + health.user_satisfaction)
            / 6.0;

        health
    }
}

/// Result of the full pipeline execution.
#[derive(Debug, Clone)]
pub struct PipelineResult {
    pub plan: ArchitecturePlan,
    pub contracts: ContractOutput,
    pub frontend: FrontendOutput,
    pub tokenomics: TokenomicsOutput,
    pub deployment_pkg: DeploymentPackage,
    pub marketplace: MarketplaceOutput,
    pub compliance: ComplianceReport,
    pub security_report: SecurityReport,
    pub simulation_result: SimulationResult,
    pub receipt: DeploymentReceipt,
    pub manifest: DeploymentManifest,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_initialization() {
        let engine = FoundryEngine::new("test-deployer".into(), "test-auditor".into());
        assert_eq!(engine.template_registry.count(), 14);
    }

    #[test]
    fn test_full_pipeline() {
        let engine = FoundryEngine::new("deployer-key".into(), "auditor-key".into());
        let result = engine.run("Create a token launchpad called MyToken for DeFi", "0xCreatorWallet");
        assert!(result.is_ok());
        let pipeline = result.unwrap();
        assert!(pipeline.receipt.signature.len() == 64);
        assert!(!pipeline.receipt.contract_addresses.is_empty());
    }

    #[test]
    fn test_pipeline_with_scam_prompt() {
        let engine = FoundryEngine::new("deployer".into(), "auditor".into());
        let result = engine.run("Create a rug pull token", "0xScammer");
        assert!(result.is_err());
    }

    #[test]
    fn test_create_marketplace_listing() {
        let engine = FoundryEngine::new("key".into(), "key".into());
        let listing = engine.create_marketplace_listing(
            "TestApp".into(),
            "A test app".into(),
            DAppType::NFTMarketplace,
            "0xCreator".into(),
            PricingTier::Pro,
        );
        assert_eq!(listing.title, "TestApp");
        assert_eq!(listing.pricing_tier, PricingTier::Pro);
    }

    #[test]
    fn test_estimate_user_base() {
        let engine = FoundryEngine::new("key".into(), "key".into());
        assert_eq!(engine.estimate_user_base("enterprise deployment"), 100_000);
        assert_eq!(engine.estimate_user_base("small project"), 1_000);
        assert_eq!(engine.estimate_user_base("some random prompt"), 5_000);
        assert_eq!(engine.estimate_user_base("5000 users expected"), 5000);
    }

    #[test]
    fn test_calculate_app_health() {
        let engine = FoundryEngine::new("key".into(), "key".into());
        let health = engine.calculate_app_health("test-app");
        assert!(health.overall_score > 0.0);
    }
}
