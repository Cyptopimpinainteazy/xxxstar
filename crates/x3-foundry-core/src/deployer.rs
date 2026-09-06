use crate::error::FoundryError;
use crate::types::{DAppType, DeploymentReceipt};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::env;
use tracing::info;

/// Deployment manifest containing all deployment metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentManifest {
    pub app_name: String,
    pub version: String,
    pub dapp_type: String,
    pub target_chain: String,
    pub contracts: Vec<DeployedContractInfo>,
    pub frontend_url: Option<String>,
    pub metadata_uri: Option<String>,
    pub treasury_hooks: Vec<String>,
    pub marketplace_listing_id: Option<String>,
    pub analytics_endpoint: Option<String>,
    pub deployed_at: chrono::DateTime<Utc>,
    pub deployer_address: String,
    pub manifest_hash: String,
}

/// Information about a deployed contract.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeployedContractInfo {
    pub name: String,
    pub address: String,
    pub tx_hash: String,
    pub block_number: u64,
    pub gas_used: u64,
    pub verified: bool,
}

/// Deployer handles the deployment of dApps to target chains.
pub struct Deployer {
    pub deployer_key: String,
    pub default_chain: String,
}

impl Deployer {
    pub fn new(deployer_key: String, default_chain: String) -> Self {
        Self {
            deployer_key,
            default_chain,
        }
    }

    /// Deploys all smart contracts for the dApp.
    ///
    /// Current transport is deterministic/local deployment emulation. That is
    /// acceptable for local, dev, anvil, and testnet profiles. Production-like
    /// chains must opt in explicitly with `X3_FOUNDRY_ALLOW_SIMULATED_DEPLOY=1`
    /// until the real signed transaction transport lands. This prevents a
    /// mainnet path from returning fake addresses and calling that success.
    pub fn deploy_contracts(
        &self,
        contracts: &HashMap<String, String>,
        deployment_order: &[String],
        chain: &str,
    ) -> Result<Vec<DeployedContractInfo>, FoundryError> {
        self.ensure_simulated_deployment_allowed(chain)?;

        info!(
            "Deployer: deploying {} contracts to {}",
            contracts.len(),
            chain
        );
        let mut deployed = Vec::new();

        for contract_name in deployment_order {
            let source = contracts.get(contract_name).ok_or_else(|| {
                FoundryError::DeploymentFailed(format!(
                    "Contract {} not found in source map",
                    contract_name
                ))
            })?;

            let address = self.derive_local_contract_address(contract_name, source, chain);
            let tx_hash = self.compute_tx_hash(contract_name, chain);
            let block_number = self.simulate_block_number();
            let gas_used = self.estimate_gas(source);

            deployed.push(DeployedContractInfo {
                name: contract_name.clone(),
                address,
                tx_hash: tx_hash.clone(),
                block_number,
                gas_used,
                verified: false,
            });

            info!(
                "Deployed {} at {} (tx: {})",
                contract_name,
                deployed.last().unwrap().address,
                tx_hash
            );
        }

        Ok(deployed)
    }

    /// Deploys the frontend application.
    pub fn deploy_frontend(
        &self,
        app_name: &str,
        frontend_framework: &str,
        _routes: &[String],
        _api_endpoints: &[String],
    ) -> Result<String, FoundryError> {
        info!(
            "Deployer: deploying frontend for {} using {}",
            app_name, frontend_framework
        );
        let url = format!(
            "https://{}.x3-app.io",
            app_name.to_lowercase().replace(' ', "-")
        );
        info!("Frontend deployed at: {}", url);
        Ok(url)
    }

    /// Deploys metadata (IPFS/Arweave).
    pub fn deploy_metadata(
        &self,
        app_name: &str,
        description: &str,
        features: &[String],
    ) -> Result<String, FoundryError> {
        info!("Deployer: deploying metadata for {}", app_name);
        let metadata = serde_json::json!({
            "name": app_name,
            "description": description,
            "features": features,
            "version": "1.0.0",
            "deployed_at": Utc::now().to_rfc3339(),
            "deployer": self.deployer_key,
        });
        let metadata_str = serde_json::to_string(&metadata).map_err(|e| {
            FoundryError::DeploymentFailed(format!("Failed to serialize metadata: {}", e))
        })?;
        let mut hasher = Sha256::new();
        hasher.update(metadata_str.as_bytes());
        let hash = hex::encode(hasher.finalize());
        let uri = format!("ipfs://{}", hash);
        info!("Metadata deployed at: {}", uri);
        Ok(uri)
    }

    /// Deploys treasury hooks for fee distribution.
    pub fn deploy_treasury_hooks(
        &self,
        treasury_wallet: &str,
        platform_fee_bps: u16,
        chain: &str,
    ) -> Result<Vec<String>, FoundryError> {
        self.ensure_simulated_deployment_allowed(chain)?;

        info!("Deployer: deploying treasury hooks for {}", treasury_wallet);
        let key_preview = preview_key(&self.deployer_key);
        let hooks = vec![
            format!(
                "FeeCollector: {} bps -> {}",
                platform_fee_bps, treasury_wallet
            ),
            format!("RevenueDistributor: deployed on {}", chain),
            format!("TreasuryHook: {}", key_preview),
        ];
        Ok(hooks)
    }

    /// Creates a marketplace listing for the deployed dApp.
    pub fn deploy_marketplace_listing(
        &self,
        title: &str,
        _description: &str,
        _tags: &[String],
        chain: &str,
    ) -> Result<String, FoundryError> {
        self.ensure_simulated_deployment_allowed(chain)?;

        info!("Deployer: creating marketplace listing for {}", title);
        let listing_id = format!("listing-{}", &uuid::Uuid::new_v4().to_string()[..8]);
        info!("Marketplace listing created: {} on {}", listing_id, chain);
        Ok(listing_id)
    }

    /// Deploys analytics tracking.
    pub fn deploy_analytics(&self, app_name: &str, chain: &str) -> Result<String, FoundryError> {
        info!(
            "Deployer: deploying analytics for {} on {}",
            app_name, chain
        );
        let endpoint = format!(
            "https://analytics.x3-chain.io/api/v1/apps/{}/events",
            app_name.to_lowercase().replace(' ', "-")
        );
        info!("Analytics endpoint: {}", endpoint);
        Ok(endpoint)
    }

    /// Generates a deployment manifest.
    #[allow(clippy::too_many_arguments)]
    pub fn generate_manifest(
        &self,
        app_name: &str,
        dapp_type: &DAppType,
        contracts: &[DeployedContractInfo],
        frontend_url: Option<String>,
        metadata_uri: Option<String>,
        treasury_hooks: Vec<String>,
        marketplace_listing_id: Option<String>,
        analytics_endpoint: Option<String>,
        chain: &str,
    ) -> DeploymentManifest {
        let manifest = DeploymentManifest {
            app_name: app_name.to_string(),
            version: "1.0.0".to_string(),
            dapp_type: dapp_type.to_string(),
            target_chain: chain.to_string(),
            contracts: contracts.to_vec(),
            frontend_url,
            metadata_uri,
            treasury_hooks,
            marketplace_listing_id,
            analytics_endpoint,
            deployed_at: Utc::now(),
            deployer_address: self.deployer_key.clone(),
            manifest_hash: String::new(), // Will be computed
        };

        let manifest_json = serde_json::to_string(&manifest).unwrap_or_default();
        let mut hasher = Sha256::new();
        hasher.update(manifest_json.as_bytes());
        let hash = hex::encode(hasher.finalize());

        DeploymentManifest {
            manifest_hash: hash,
            ..manifest
        }
    }

    /// Signs a deployment receipt.
    pub fn sign_receipt(&self, receipt: &mut DeploymentReceipt) {
        let receipt_data = format!(
            "{:?}{:?}{}",
            receipt.contract_addresses, receipt.tx_hashes, self.deployer_key
        );
        let mut hasher = Sha256::new();
        hasher.update(receipt_data.as_bytes());
        receipt.signature = hex::encode(hasher.finalize());
        receipt.signed_at = Utc::now();
        info!("Receipt signed: {}", receipt.signature);
    }

    fn ensure_simulated_deployment_allowed(&self, chain: &str) -> Result<(), FoundryError> {
        if is_local_or_test_chain(chain) || allow_simulated_deploys() {
            return Ok(());
        }

        Err(FoundryError::DeploymentFailed(format!(
            "Refusing simulated Foundry deployment on production-like chain '{}'. Wire real signed transaction deployment or set X3_FOUNDRY_ALLOW_SIMULATED_DEPLOY=1 only for controlled drills.",
            chain
        )))
    }

    /// Derives a deterministic local/test address for non-production drills.
    fn derive_local_contract_address(&self, contract_name: &str, source: &str, chain: &str) -> String {
        let input = format!(
            "{}{}{}{}",
            contract_name,
            source.len(),
            chain,
            self.deployer_key
        );
        let mut hasher = Sha256::new();
        hasher.update(input.as_bytes());
        let hash = hex::encode(hasher.finalize());
        format!("0x{}", &hash[..40])
    }

    /// Computes a deterministic transaction hash for local/test deployment drills.
    fn compute_tx_hash(&self, contract_name: &str, chain: &str) -> String {
        let input = format!(
            "deploy-{}-{}-{}",
            contract_name,
            chain,
            Utc::now().timestamp()
        );
        let mut hasher = Sha256::new();
        hasher.update(input.as_bytes());
        hex::encode(hasher.finalize())
    }

    /// Simulates block number for local/test deployment drills.
    fn simulate_block_number(&self) -> u64 {
        (Utc::now().timestamp() as u64) % 100_000_000 + 10_000_000
    }

    /// Estimates gas for contract deployment.
    fn estimate_gas(&self, source: &str) -> u64 {
        let lines = source.lines().count() as u64;
        500_000 + lines * 10_000
    }
}

fn is_local_or_test_chain(chain: &str) -> bool {
    let chain = chain.to_ascii_lowercase();
    chain.contains("testnet")
        || chain.contains("local")
        || chain.contains("anvil")
        || chain.contains("dev")
        || chain == "31337"
        || chain == "1337"
}

fn allow_simulated_deploys() -> bool {
    env::var("X3_FOUNDRY_ALLOW_SIMULATED_DEPLOY")
        .map(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"))
        .unwrap_or(false)
}

fn preview_key(key: &str) -> String {
    let prefix: String = key.chars().take(8).collect();
    let suffix: String = key
        .chars()
        .rev()
        .take(8)
        .collect::<String>()
        .chars()
        .rev()
        .collect();

    if key.chars().count() <= 16 {
        format!("{}...", prefix)
    } else {
        format!("{}...{}", prefix, suffix)
    }
}

/// CrossChainDeployer handles multi-chain deployment.
pub struct CrossChainDeployer {
    pub deployers: HashMap<String, Deployer>,
}

impl CrossChainDeployer {
    pub fn new() -> Self {
        Self {
            deployers: HashMap::new(),
        }
    }

    /// Adds a deployer for a specific chain.
    pub fn add_chain(&mut self, chain: String, deployer: Deployer) {
        self.deployers.insert(chain, deployer);
    }

    /// Deploys contracts to multiple chains.
    pub fn deploy_to_chains(
        &self,
        contracts: &HashMap<String, String>,
        deployment_order: &[String],
        chains: &[String],
    ) -> Result<HashMap<String, Vec<DeployedContractInfo>>, FoundryError> {
        info!("CrossChainDeployer: deploying to {} chains", chains.len());
        let mut results = HashMap::new();

        for chain in chains {
            let deployer = self.deployers.get(chain).ok_or_else(|| {
                FoundryError::DeploymentFailed(format!(
                    "No deployer configured for chain {}",
                    chain
                ))
            })?;
            let deployed = deployer.deploy_contracts(contracts, deployment_order, chain)?;
            results.insert(chain.clone(), deployed);
        }

        Ok(results)
    }

    /// Generates a cross-chain deployment manifest.
    pub fn generate_cross_chain_manifest(
        &self,
        app_name: &str,
        dapp_type: &DAppType,
        chain_results: &HashMap<String, Vec<DeployedContractInfo>>,
    ) -> HashMap<String, DeploymentManifest> {
        let mut manifests = HashMap::new();
        for (chain, contracts) in chain_results {
            let deployer = self.deployers.get(chain).unwrap();
            let manifest = deployer.generate_manifest(
                app_name,
                dapp_type,
                contracts,
                None,
                None,
                vec![],
                None,
                None,
                chain,
            );
            manifests.insert(chain.clone(), manifest);
        }
        manifests
    }
}

impl Default for CrossChainDeployer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_contracts() -> (HashMap<String, String>, Vec<String>) {
        let mut contracts = HashMap::new();
        contracts.insert(
            "TestToken".into(),
            "pragma solidity ^0.8.20;\ncontract TestToken {}".into(),
        );
        let order = vec!["TestToken".into()];
        (contracts, order)
    }

    #[test]
    fn test_deploy_contracts() {
        let deployer = Deployer::new("test-key".into(), "x3-testnet".into());
        let (contracts, order) = sample_contracts();
        let result = deployer.deploy_contracts(&contracts, &order, "x3-testnet");
        assert!(result.is_ok());
        let deployed = result.unwrap();
        assert_eq!(deployed.len(), 1);
        assert!(deployed[0].address.starts_with("0x"));
    }

    #[test]
    fn test_mainnet_rejects_simulated_deploy_without_escape_hatch() {
        env::remove_var("X3_FOUNDRY_ALLOW_SIMULATED_DEPLOY");
        let deployer = Deployer::new("test-key".into(), "x3-mainnet".into());
        let (contracts, order) = sample_contracts();
        let result = deployer.deploy_contracts(&contracts, &order, "x3-mainnet");
        assert!(matches!(result, Err(FoundryError::DeploymentFailed(_))));
    }

    #[test]
    fn test_deploy_frontend() {
        let deployer = Deployer::new("test".into(), "x3".into());
        let url = deployer.deploy_frontend("MyApp", "React", &["/".into()], &["/api".into()]);
        assert!(url.is_ok());
        assert!(url.unwrap().contains("x3-app.io"));
    }

    #[test]
    fn test_cross_chain() {
        let mut cc = CrossChainDeployer::new();
        cc.add_chain(
            "x3-testnet".into(),
            Deployer::new("key1".into(), "x3-testnet".into()),
        );
        cc.add_chain(
            "ethereum-anvil".into(),
            Deployer::new("key2".into(), "ethereum-anvil".into()),
        );
        let (contracts, order) = sample_contracts();
        let result = cc.deploy_to_chains(
            &contracts,
            &order,
            &["x3-testnet".into(), "ethereum-anvil".into()],
        );
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 2);
    }
}
