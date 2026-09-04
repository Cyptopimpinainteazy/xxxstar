use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// The type of dApp to generate.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum DAppType {
    TokenLaunchpad,
    NFTMarketplace,
    StakingPool,
    SubscriptionApp,
    EscrowApp,
    AiImageSaaS,
    TradingBotVault,
    YieldOptimizer,
    CrossChainPayout,
    DomainRegistry,
    PredictionMarket,
    AffiliateApp,
    DataMarketplace,
    Custom(String),
}

impl DAppType {
    /// Returns a human-readable label for the dApp type.
    pub fn label(&self) -> &str {
        match self {
            DAppType::TokenLaunchpad => "Token Launchpad",
            DAppType::NFTMarketplace => "NFT Marketplace",
            DAppType::StakingPool => "Staking Pool",
            DAppType::SubscriptionApp => "Subscription App",
            DAppType::EscrowApp => "Escrow App",
            DAppType::AiImageSaaS => "AI Image SaaS",
            DAppType::TradingBotVault => "Trading Bot Vault",
            DAppType::YieldOptimizer => "Yield Optimizer",
            DAppType::CrossChainPayout => "Cross-Chain Payout",
            DAppType::DomainRegistry => "Domain Registry",
            DAppType::PredictionMarket => "Prediction Market",
            DAppType::AffiliateApp => "Affiliate App",
            DAppType::DataMarketplace => "Data Marketplace",
            DAppType::Custom(_) => "Custom dApp",
        }
    }
}

impl std::fmt::Display for DAppType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

/// Fee distribution mode.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum FeeMode {
    GrossRevenue,
    NetProtocolFees,
    SubscriptionRevenue,
    TradingFeesOnly,
    MarketplaceSalesOnly,
    CreatorDefinedWithPlatformMinimum,
}

impl FeeMode {
    pub fn label(&self) -> &str {
        match self {
            FeeMode::GrossRevenue => "Gross Revenue",
            FeeMode::NetProtocolFees => "Net Protocol Fees",
            FeeMode::SubscriptionRevenue => "Subscription Revenue",
            FeeMode::TradingFeesOnly => "Trading Fees Only",
            FeeMode::MarketplaceSalesOnly => "Marketplace Sales Only",
            FeeMode::CreatorDefinedWithPlatformMinimum => "Creator Defined with Platform Minimum",
        }
    }
}

/// Revenue configuration for a dApp.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevenueConfig {
    pub platform_fee_bps: u16,
    pub creator_fee_bps: u16,
    pub ai_agent_fee_bps: Option<u16>,
    pub maintenance_fee_bps: Option<u16>,
    pub referral_fee_bps: Option<u16>,
    pub treasury_wallet: String,
    pub creator_wallet: String,
    pub maintenance_wallet: String,
    pub ai_agent_wallet: String,
    pub referral_wallet: String,
    pub fee_token: String,
    pub fee_mode: FeeMode,
}

impl Default for RevenueConfig {
    fn default() -> Self {
        Self {
            platform_fee_bps: 200,
            creator_fee_bps: 9700,
            ai_agent_fee_bps: Some(50),
            maintenance_fee_bps: Some(50),
            referral_fee_bps: Some(50),
            treasury_wallet: "0x0000000000000000000000000000000000000000".to_string(),
            creator_wallet: String::new(),
            maintenance_wallet: "0x0000000000000000000000000000000000000001".to_string(),
            ai_agent_wallet: "0x0000000000000000000000000000000000000002".to_string(),
            referral_wallet: "0x0000000000000000000000000000000000000003".to_string(),
            fee_token: "X3".to_string(),
            fee_mode: FeeMode::GrossRevenue,
        }
    }
}

/// Project state containing all metadata about a generated dApp.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectState {
    pub id: String,
    pub name: String,
    pub dapp_type: DAppType,
    pub description: String,
    pub creator_wallet: String,
    pub revenue_config: RevenueConfig,
    pub pricing_tier: PricingTier,
    pub features: Vec<String>,
    pub required_contracts: Vec<String>,
    pub frontend_framework: String,
    pub target_chains: Vec<String>,
    pub template_id: String,
    pub status: ProjectStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub version: String,
    pub tags: Vec<String>,
    pub license: String,
    pub repository_url: Option<String>,
    pub documentation_url: Option<String>,
    pub deployment_receipt: Option<DeploymentReceipt>,
}

impl ProjectState {
    pub fn new(name: String, dapp_type: DAppType, creator_wallet: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            dapp_type,
            description: String::new(),
            creator_wallet,
            revenue_config: RevenueConfig::default(),
            pricing_tier: PricingTier::Free,
            features: Vec::new(),
            required_contracts: Vec::new(),
            frontend_framework: "React + TypeScript".to_string(),
            target_chains: vec!["x3-testnet".to_string()],
            template_id: String::new(),
            status: ProjectStatus::Draft,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            version: "0.1.0".to_string(),
            tags: Vec::new(),
            license: "Apache-2.0".to_string(),
            repository_url: None,
            documentation_url: None,
            deployment_receipt: None,
        }
    }
}

/// Project status enum.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ProjectStatus {
    Draft,
    Generating,
    Auditing,
    Simulating,
    Deploying,
    Deployed,
    Failed,
    Archived,
}

/// Security report from the audit pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityReport {
    pub risk_score: u8,
    pub passed: bool,
    pub warnings: Vec<String>,
    pub critical_findings: Vec<String>,
    pub fee_findings: Vec<String>,
    pub ownership_findings: Vec<String>,
    pub license_findings: Vec<String>,
    pub simulation_receipt: Option<SimulationResult>,
    pub auditor_signature: String,
    pub static_analysis_score: u8,
    pub fuzz_score: u8,
    pub test_coverage_pct: f64,
    pub loc_analyzed: u64,
    pub audited_at: DateTime<Utc>,
}

impl SecurityReport {
    pub fn new(_auditor_key: String) -> Self {
        Self {
            risk_score: 0,
            passed: false,
            warnings: Vec::new(),
            critical_findings: Vec::new(),
            fee_findings: Vec::new(),
            ownership_findings: Vec::new(),
            license_findings: Vec::new(),
            simulation_receipt: None,
            auditor_signature: String::new(),
            static_analysis_score: 100,
            fuzz_score: 100,
            test_coverage_pct: 0.0,
            loc_analyzed: 0,
            audited_at: Utc::now(),
        }
    }
}

/// Break-even model from simulation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BreakEvenModel {
    pub days_to_break_even: u64,
    pub total_dev_cost: u128,
    pub monthly_op_cost: u128,
    pub daily_volume_required: u128,
    pub is_profitable: bool,
}

/// Monthly revenue projection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonthlyProjection {
    pub month: u32,
    pub volume: u128,
    pub revenue: u128,
    pub gas_cost: u128,
}

/// Simulation result from the simulator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationResult {
    pub expected_volume: u128,
    pub fee_revenue: u128,
    pub gas_cost: u128,
    pub break_even_model: BreakEvenModel,
    pub treasury_contribution: u128,
    pub creator_earnings: u128,
    pub monthly_revenue_projection: Vec<MonthlyProjection>,
    pub confidence_score: f64,
}

/// Deployment receipt containing deployment details.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentReceipt {
    pub app_id: String,
    pub app_name: String,
    pub dapp_type: DAppType,
    pub chain: String,
    pub contract_addresses: HashMap<String, String>,
    pub tx_hashes: Vec<String>,
    pub frontend_url: Option<String>,
    pub metadata_uri: Option<String>,
    pub marketplace_listing_id: Option<String>,
    pub analytics_endpoint: Option<String>,
    pub deployed_at: DateTime<Utc>,
    pub deployer_address: String,
    pub signature: String,
    pub signed_at: DateTime<Utc>,
    pub manifest_hash: String,
    pub block_number: u64,
    pub gas_used: u64,
}

impl DeploymentReceipt {
    pub fn new(
        app_id: String,
        app_name: String,
        dapp_type: DAppType,
        chain: String,
        deployer_address: String,
    ) -> Self {
        Self {
            app_id,
            app_name,
            dapp_type,
            chain,
            contract_addresses: HashMap::new(),
            tx_hashes: Vec::new(),
            frontend_url: None,
            metadata_uri: None,
            marketplace_listing_id: None,
            analytics_endpoint: None,
            deployed_at: Utc::now(),
            deployer_address,
            signature: String::new(),
            signed_at: Utc::now(),
            manifest_hash: String::new(),
            block_number: 0,
            gas_used: 0,
        }
    }
}

/// Marketplace listing for a dApp.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketplaceListing {
    pub id: String,
    pub app_id: String,
    pub title: String,
    pub description: String,
    pub tagline: String,
    pub dapp_type: DAppType,
    pub tags: Vec<String>,
    pub pricing_tier: PricingTier,
    pub price: u128,
    pub price_token: String,
    pub creator_wallet: String,
    pub documentation_url: Option<String>,
    pub demo_url: Option<String>,
    pub screenshot_urls: Vec<String>,
    pub rating: f64,
    pub review_count: u64,
    pub download_count: u64,
    pub listed_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub verified: bool,
    pub featured: bool,
}

impl MarketplaceListing {
    pub fn new(title: String, app_id: String, dapp_type: DAppType, creator_wallet: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            app_id,
            title,
            description: String::new(),
            tagline: String::new(),
            dapp_type,
            tags: Vec::new(),
            pricing_tier: PricingTier::Free,
            price: 0,
            price_token: "X3".to_string(),
            creator_wallet,
            documentation_url: None,
            demo_url: None,
            screenshot_urls: Vec::new(),
            rating: 0.0,
            review_count: 0,
            download_count: 0,
            listed_at: Utc::now(),
            updated_at: Utc::now(),
            verified: false,
            featured: false,
        }
    }
}

/// Revenue report for an app or creator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevenueReport {
    pub app_id: String,
    pub app_name: String,
    pub total_volume: u128,
    pub total_fees_collected: u128,
    pub platform_revenue: u128,
    pub creator_revenue: u128,
    pub ai_agent_revenue: u128,
    pub maintenance_revenue: u128,
    pub referral_payouts: u128,
    pub transaction_count: u64,
    pub unique_users: u64,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub generated_at: DateTime<Utc>,
}

/// App health score for monitoring.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppHealthScore {
    pub app_id: String,
    pub overall_score: f64,
    pub uptime_score: f64,
    pub transaction_success_rate: f64,
    pub gas_efficiency: f64,
    pub security_score: f64,
    pub revenue_health: f64,
    pub user_satisfaction: f64,
    pub last_checked: DateTime<Utc>,
    pub warnings: Vec<String>,
    pub critical_issues: Vec<String>,
}

impl AppHealthScore {
    pub fn new(app_id: String) -> Self {
        Self {
            app_id,
            overall_score: 100.0,
            uptime_score: 100.0,
            transaction_success_rate: 100.0,
            gas_efficiency: 100.0,
            security_score: 100.0,
            revenue_health: 100.0,
            user_satisfaction: 100.0,
            last_checked: Utc::now(),
            warnings: Vec::new(),
            critical_issues: Vec::new(),
        }
    }
}

/// Fork lineage for tracking template forks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForkLineage {
    pub original_template_id: String,
    pub fork_id: String,
    pub forked_from: Option<String>,
    pub fork_depth: u32,
    pub modifications: Vec<String>,
    pub forked_at: DateTime<Utc>,
    pub forked_by: String,
}

/// Pricing tier for dApps.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PricingTier {
    Free,
    Builder,
    Pro,
    Enterprise,
}

impl PricingTier {
    pub fn label(&self) -> &str {
        match self {
            PricingTier::Free => "Free",
            PricingTier::Builder => "Builder",
            PricingTier::Pro => "Pro",
            PricingTier::Enterprise => "Enterprise",
        }
    }

    pub fn monthly_price_usd(&self) -> u64 {
        match self {
            PricingTier::Free => 0,
            PricingTier::Builder => 29,
            PricingTier::Pro => 99,
            PricingTier::Enterprise => 499,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dapp_type_label() {
        assert_eq!(DAppType::TokenLaunchpad.label(), "Token Launchpad");
        assert_eq!(DAppType::Custom("Test".into()).label(), "Custom dApp");
    }

    #[test]
    fn test_revenue_config_default() {
        let config = RevenueConfig::default();
        assert_eq!(config.platform_fee_bps, 200);
        assert_eq!(config.creator_fee_bps, 9700);
    }

    #[test]
    fn test_project_state_new() {
        let state = ProjectState::new(
            "TestApp".into(),
            DAppType::NFTMarketplace,
            "0xCreator".into(),
        );
        assert_eq!(state.name, "TestApp");
        assert_eq!(state.status, ProjectStatus::Draft);
    }

    #[test]
    fn test_pricing_tier() {
        assert_eq!(PricingTier::Free.label(), "Free");
        assert_eq!(PricingTier::Enterprise.monthly_price_usd(), 499);
    }

    #[test]
    fn test_deployment_receipt_new() {
        let receipt = DeploymentReceipt::new(
            "app-1".into(),
            "Test".into(),
            DAppType::StakingPool,
            "x3-mainnet".into(),
            "0xDeployer".into(),
        );
        assert_eq!(receipt.app_name, "Test");
        assert!(receipt.contract_addresses.is_empty());
    }

    #[test]
    fn test_marketplace_listing_new() {
        let listing = MarketplaceListing::new(
            "MyApp".into(),
            "app-1".into(),
            DAppType::AiImageSaaS,
            "0xCreator".into(),
        );
        assert_eq!(listing.title, "MyApp");
        assert_eq!(listing.rating, 0.0);
    }

    #[test]
    fn test_app_health_score() {
        let health = AppHealthScore::new("app-1".into());
        assert_eq!(health.overall_score, 100.0);
    }

    #[test]
    fn test_fee_mode_label() {
        assert_eq!(FeeMode::GrossRevenue.label(), "Gross Revenue");
    }

    #[test]
    fn test_security_report_new() {
        let report = SecurityReport::new("auditor-key".into());
        assert!(!report.passed);
        assert!(report.warnings.is_empty());
    }
}
