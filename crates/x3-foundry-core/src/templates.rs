use crate::types::DAppType;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::info;

/// A template for generating dApps.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Template {
    pub id: String,
    pub name: String,
    pub description: String,
    pub dapp_type: DAppType,
    pub version: String,
    pub author: String,
    pub license: String,
    pub tags: Vec<String>,
    pub required_contracts: Vec<String>,
    pub frontend_framework: String,
    pub chain_support: Vec<String>,
}

impl Template {
    pub fn new(
        id: &str,
        name: &str,
        description: &str,
        dapp_type: DAppType,
        version: &str,
        author: &str,
        license: &str,
        tags: Vec<String>,
        required_contracts: Vec<String>,
        frontend_framework: &str,
        chain_support: Vec<String>,
    ) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            description: description.to_string(),
            dapp_type,
            version: version.to_string(),
            author: author.to_string(),
            license: license.to_string(),
            tags,
            required_contracts,
            frontend_framework: frontend_framework.to_string(),
            chain_support,
        }
    }
}

/// TemplateRegistry manages all available dApp templates.
#[derive(Debug, Clone)]
pub struct TemplateRegistry {
    templates: HashMap<String, Template>,
}

impl TemplateRegistry {
    pub fn new() -> Self {
        Self {
            templates: HashMap::new(),
        }
    }

    /// Creates the default registry with all 14 template types pre-registered.
    pub fn default_registry() -> Self {
        let mut registry = Self::new();
        registry.register_default_templates();
        registry
    }

    /// Registers a template.
    pub fn register_template(&mut self, template: Template) {
        info!("Registering template: {} ({})", template.name, template.id);
        self.templates.insert(template.id.clone(), template);
    }

    /// Gets a template by ID.
    pub fn get_template(&self, id: &str) -> Option<&Template> {
        self.templates.get(id)
    }

    /// Lists all registered templates.
    pub fn list_templates(&self) -> Vec<&Template> {
        self.templates.values().collect()
    }

    /// Lists templates by dApp category.
    pub fn list_by_category(&self, dapp_type: &DAppType) -> Vec<&Template> {
        self.templates
            .values()
            .filter(|t| t.dapp_type == *dapp_type)
            .collect()
    }

    /// Lists templates that support a specific chain.
    pub fn list_by_chain(&self, chain: &str) -> Vec<&Template> {
        self.templates
            .values()
            .filter(|t| {
                t.chain_support
                    .iter()
                    .any(|c| c.to_lowercase() == chain.to_lowercase())
            })
            .collect()
    }

    /// Lists templates by tag.
    pub fn list_by_tag(&self, tag: &str) -> Vec<&Template> {
        self.templates
            .values()
            .filter(|t| {
                t.tags
                    .iter()
                    .any(|tg| tg.to_lowercase() == tag.to_lowercase())
            })
            .collect()
    }

    /// Removes a template by ID.
    pub fn remove_template(&mut self, id: &str) -> Option<Template> {
        self.templates.remove(id)
    }

    /// Gets the total number of registered templates.
    pub fn count(&self) -> usize {
        self.templates.len()
    }

    /// Registers all 14 default template types.
    fn register_default_templates(&mut self) {
        // 1. Token Launchpad
        self.register_template(Template::new(
            "token-launchpad-v1",
            "Token Launchpad",
            "Launch your own token with presale, vesting, and liquidity locking. Perfect for community-driven token projects.",
            DAppType::TokenLaunchpad,
            "1.0.0",
            "X3 Foundry",
            "Apache-2.0",
            vec!["token".into(), "launchpad".into(), "presale".into(), "defi".into()],
            vec!["TokenFactory".into(), "PresaleContract".into(), "VestingWallet".into(), "LiquidityLocker".into()],
            "React + TypeScript",
            vec!["x3-mainnet".into(), "x3-testnet".into(), "ethereum".into(), "bsc".into()],
        ));

        // 2. NFT Marketplace
        self.register_template(Template::new(
            "nft-marketplace-v1",
            "NFT Marketplace",
            "Full-featured NFT marketplace with minting, auctions, royalties, and collection verification.",
            DAppType::NFTMarketplace,
            "1.0.0",
            "X3 Foundry",
            "Apache-2.0",
            vec!["nft".into(), "marketplace".into(), "auction".into(), "collectible".into()],
            vec!["NFTFactory".into(), "Marketplace".into(), "AuctionHouse".into(), "RoyaltyRegistry".into()],
            "Next.js + TypeScript",
            vec!["x3-mainnet".into(), "x3-testnet".into(), "ethereum".into(), "polygon".into()],
        ));

        // 3. Staking Pool
        self.register_template(Template::new(
            "staking-pool-v1",
            "Staking Pool",
            "Create flexible staking pools with tiered rewards, lock periods, and compound staking support.",
            DAppType::StakingPool,
            "1.0.0",
            "X3 Foundry",
            "Apache-2.0",
            vec!["staking".into(), "rewards".into(), "defi".into(), "yield".into()],
            vec!["StakingPool".into(), "RewardDistributor".into(), "StakingToken".into()],
            "React + TypeScript",
            vec!["x3-mainnet".into(), "x3-testnet".into(), "ethereum".into(), "polygon".into(), "avalanche".into()],
        ));

        // 4. Subscription App
        self.register_template(Template::new(
            "subscription-app-v1",
            "Subscription App",
            "Build a subscription-based SaaS with tiered plans, recurring billing, and token-gated access.",
            DAppType::SubscriptionApp,
            "1.0.0",
            "X3 Foundry",
            "Apache-2.0",
            vec!["subscription".into(), "saas".into(), "billing".into(), "recurring".into()],
            vec!["SubscriptionManager".into(), "PaymentProcessor".into(), "AccessControl".into()],
            "Next.js + TypeScript",
            vec!["x3-mainnet".into(), "x3-testnet".into(), "ethereum".into()],
        ));

        // 5. Escrow App
        self.register_template(Template::new(
            "escrow-app-v1",
            "Escrow App",
            "Secure escrow service with milestone-based payments, dispute resolution, and multi-sig approvals.",
            DAppType::EscrowApp,
            "1.0.0",
            "X3 Foundry",
            "Apache-2.0",
            vec!["escrow".into(), "dispute".into(), "payment".into(), "milestone".into()],
            vec!["EscrowContract".into(), "DisputeResolver".into(), "MilestoneManager".into()],
            "React + TypeScript",
            vec!["x3-mainnet".into(), "x3-testnet".into(), "ethereum".into()],
        ));

        // 6. AI Image SaaS
        self.register_template(Template::new(
            "ai-image-saas-v1",
            "AI Image SaaS",
            "AI-powered image generation platform with GPU compute integration, pay-per-use billing, and NFT minting.",
            DAppType::AiImageSaaS,
            "1.0.0",
            "X3 Foundry",
            "Apache-2.0",
            vec!["ai".into(), "image".into(), "generation".into(), "gpu".into(), "saas".into()],
            vec!["ComputeManager".into(), "PaymentProcessor".into(), "ImageRegistry".into(), "NFTMinter".into()],
            "Next.js + TypeScript + Tailwind",
            vec!["x3-mainnet".into(), "x3-testnet".into()],
        ));

        // 7. Trading Bot Vault
        self.register_template(Template::new(
            "trading-bot-vault-v1",
            "Trading Bot Vault",
            "Automated trading strategy vault with investor onboarding, performance tracking, and profit sharing.",
            DAppType::TradingBotVault,
            "1.0.0",
            "X3 Foundry",
            "Apache-2.0",
            vec!["trading".into(), "bot".into(), "vault".into(), "strategy".into(), "defi".into()],
            vec!["VaultManager".into(), "StrategyExecutor".into(), "ProfitDistributor".into(), "InvestorRegistry".into()],
            "React + TypeScript + D3.js",
            vec!["x3-mainnet".into(), "x3-testnet".into()],
        ));

        // 8. Yield Optimizer
        self.register_template(Template::new(
            "yield-optimizer-v1",
            "Yield Optimizer",
            "Multi-pool yield aggregator with auto-compounding, risk scoring, and strategy backtesting.",
            DAppType::YieldOptimizer,
            "1.0.0",
            "X3 Foundry",
            "Apache-2.0",
            vec!["yield".into(), "optimizer".into(), "aggregator".into(), "defi".into(), "compound".into()],
            vec!["YieldVault".into(), "StrategyManager".into(), "RewardHarvester".into(), "RiskOracle".into()],
            "React + TypeScript + Chart.js",
            vec!["x3-mainnet".into(), "x3-testnet".into(), "ethereum".into(), "polygon".into()],
        ));

        // 9. Cross-Chain Payout
        self.register_template(Template::new(
            "cross-chain-payout-v1",
            "Cross-Chain Payout",
            "Multi-chain payout system with streaming payments, batch settlement, and cross-chain bridging.",
            DAppType::CrossChainPayout,
            "1.0.0",
            "X3 Foundry",
            "Apache-2.0",
            vec!["cross-chain".into(), "payout".into(), "bridge".into(), "streaming".into()],
            vec!["PayoutManager".into(), "StreamingContract".into(), "BridgeAdapter".into(), "BatchSettler".into()],
            "Next.js + TypeScript",
            vec!["x3-mainnet".into(), "x3-testnet".into(), "ethereum".into(), "polygon".into(), "avalanche".into(), "solana".into()],
        ));

        // 10. Domain Registry
        self.register_template(Template::new(
            "domain-registry-v1",
            "Domain Registry",
            "Decentralized domain name registry with auctions, trading, and DNS resolution.",
            DAppType::DomainRegistry,
            "1.0.0",
            "X3 Foundry",
            "Apache-2.0",
            vec![
                "domain".into(),
                "registry".into(),
                "dns".into(),
                "naming".into(),
            ],
            vec![
                "DomainRegistry".into(),
                "DomainAuction".into(),
                "DNSResolver".into(),
                "DomainTreasury".into(),
            ],
            "React + TypeScript",
            vec!["x3-mainnet".into(), "x3-testnet".into(), "ethereum".into()],
        ));

        // 11. Prediction Market
        self.register_template(Template::new(
            "prediction-market-v1",
            "Prediction Market",
            "Decentralized prediction market with automated market making, oracle resolution, and liquidity pools.",
            DAppType::PredictionMarket,
            "1.0.0",
            "X3 Foundry",
            "Apache-2.0",
            vec!["prediction".into(), "market".into(), "betting".into(), "oracle".into()],
            vec!["MarketFactory".into(), "MarketMaker".into(), "OracleConnector".into(), "LiquidityPool".into()],
            "React + TypeScript + Chart.js",
            vec!["x3-mainnet".into(), "x3-testnet".into(), "ethereum".into(), "polygon".into()],
        ));

        // 12. Affiliate App
        self.register_template(Template::new(
            "affiliate-app-v1",
            "Affiliate App",
            "Multi-level affiliate marketing platform with commission tracking, referral links, and automated payouts.",
            DAppType::AffiliateApp,
            "1.0.0",
            "X3 Foundry",
            "Apache-2.0",
            vec!["affiliate".into(), "referral".into(), "commission".into(), "marketing".into()],
            vec!["AffiliateManager".into(), "CommissionDistributor".into(), "ReferralTracker".into()],
            "React + TypeScript",
            vec!["x3-mainnet".into(), "x3-testnet".into(), "ethereum".into(), "bsc".into()],
        ));

        // 13. Data Marketplace
        self.register_template(Template::new(
            "data-marketplace-v1",
            "Data Marketplace",
            "Decentralized data marketplace with tokenized data assets, access control, and privacy-preserving computation.",
            DAppType::DataMarketplace,
            "1.0.0",
            "X3 Foundry",
            "Apache-2.0",
            vec!["data".into(), "marketplace".into(), "privacy".into(), "computation".into()],
            vec!["DataRegistry".into(), "AccessControl".into(), "PrivacyCompute".into(), "RevenueSharer".into()],
            "Next.js + TypeScript + Tailwind",
            vec!["x3-mainnet".into(), "x3-testnet".into(), "ethereum".into()],
        ));

        // 14. Custom App
        self.register_template(Template::new(
            "custom-app-v1",
            "Custom dApp",
            "Start with a flexible template for building any custom dApp. Configure contracts, frontend, and features as needed.",
            DAppType::Custom("Custom".to_string()),
            "1.0.0",
            "X3 Foundry",
            "Apache-2.0",
            vec!["custom".into(), "flexible".into(), "generic".into()],
            vec!["MainContract".into()],
            "React + TypeScript",
            vec!["x3-mainnet".into(), "x3-testnet".into(), "ethereum".into(), "polygon".into(), "bsc".into(), "avalanche".into(), "solana".into()],
        ));
    }
}

impl Default for TemplateRegistry {
    fn default() -> Self {
        Self::default_registry()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_registry_has_all_templates() {
        let registry = TemplateRegistry::default_registry();
        assert_eq!(registry.count(), 14);
    }

    #[test]
    fn test_get_template() {
        let registry = TemplateRegistry::default_registry();
        let tpl = registry.get_template("token-launchpad-v1");
        assert!(tpl.is_some());
        assert_eq!(tpl.unwrap().name, "Token Launchpad");
    }

    #[test]
    fn test_list_by_category() {
        let registry = TemplateRegistry::default_registry();
        let nft_templates = registry.list_by_category(&DAppType::NFTMarketplace);
        assert_eq!(nft_templates.len(), 1);
    }

    #[test]
    fn test_list_by_chain() {
        let registry = TemplateRegistry::default_registry();
        let eth_templates = registry.list_by_chain("ethereum");
        assert!(eth_templates.len() > 5);
    }

    #[test]
    fn test_list_by_tag() {
        let registry = TemplateRegistry::default_registry();
        let defi_templates = registry.list_by_tag("defi");
        assert!(defi_templates.len() >= 3);
    }

    #[test]
    fn test_register_and_remove() {
        let mut registry = TemplateRegistry::new();
        assert_eq!(registry.count(), 0);
        registry.register_template(Template::new(
            "test",
            "Test",
            "Test template",
            DAppType::Custom("Test".into()),
            "1.0",
            "Test",
            "MIT",
            vec![],
            vec![],
            "React",
            vec!["x3".into()],
        ));
        assert_eq!(registry.count(), 1);
        let removed = registry.remove_template("test");
        assert!(removed.is_some());
        assert_eq!(registry.count(), 0);
    }
}
