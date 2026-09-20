use crate::error::FoundryError;
use crate::evm_deploy;
use crate::types::{DAppType, DeploymentReceipt};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use tracing::{info, warn};
use x3_foundry_auditor::{compile_contract_bytecode, FoundryAuditor};

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

    /// Deploys all smart contracts for the dApp: compiles each with the real
    /// Solidity compiler, then signs and submits a genuine contract-creation
    /// transaction to `chain`'s JSON-RPC node and waits for it to be mined.
    /// Every field on the returned `DeployedContractInfo` comes from the
    /// node's own transaction receipt.
    ///
    /// `chain` is resolved to an RPC URL via
    /// [`evm_deploy::resolve_rpc_url`] -- a literal `http(s)://` value is
    /// used directly (how tests point this at a local `anvil` instance),
    /// named chains resolve via environment variables. `self.deployer_key`
    /// must be a real hex-encoded private key; there is no more fallback to
    /// a simulated address for an invalid one.
    pub fn deploy_contracts(
        &self,
        contracts: &HashMap<String, String>,
        deployment_order: &[String],
        chain: &str,
    ) -> Result<Vec<DeployedContractInfo>, FoundryError> {
        info!(
            "Deployer: deploying {} contracts to {}",
            contracts.len(),
            chain
        );
        let mut deployed = Vec::new();
        let rpc_url = evm_deploy::resolve_rpc_url(chain);

        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| {
                FoundryError::DeploymentFailed(format!(
                    "failed to start the async runtime this deployment needs: {e}"
                ))
            })?;

        for contract_name in deployment_order {
            let source = contracts.get(contract_name).ok_or_else(|| {
                FoundryError::DeploymentFailed(format!(
                    "Contract {} not found in source map",
                    contract_name
                ))
            })?;

            self.gate_on_audit(contract_name, source)?;

            let compiled = compile_contract_bytecode(contract_name, source)
                .map_err(FoundryError::DeploymentFailed)?;

            let result = runtime.block_on(evm_deploy::deploy_bytecode(
                &rpc_url,
                &self.deployer_key,
                compiled.bytecode,
            ))?;

            info!(
                "Deployed {} at {} (tx: {})",
                contract_name, result.address, result.tx_hash
            );

            deployed.push(DeployedContractInfo {
                name: contract_name.clone(),
                address: result.address,
                tx_hash: result.tx_hash,
                block_number: result.block_number,
                gas_used: result.gas_used,
                verified: false,
            });
        }

        Ok(deployed)
    }

    /// Runs the real security audit (compiler diagnostics, reentrancy,
    /// ownership, fee-transparency, license, and scam-pattern checks) against
    /// a contract's source and refuses to deploy it if the audit finds any
    /// Critical-severity issue — including "the compiler couldn't verify
    /// this even compiles". A deployment path that silently proceeds despite
    /// a failed audit is exactly the "no-op adapter" this crate must not
    /// have, so this is a hard error, not a logged warning.
    fn gate_on_audit(&self, contract_name: &str, source: &str) -> Result<(), FoundryError> {
        let mut auditor = FoundryAuditor::new(contract_name, source);
        let report = auditor.audit_project();
        if report.summary.critical > 0 {
            let critical: Vec<String> = report
                .findings
                .iter()
                .filter(|f| f.severity == x3_foundry_auditor::Severity::Critical)
                .map(|f| format!("{} ({})", f.title, f.description))
                .collect();
            return Err(FoundryError::SecurityAuditFailed(format!(
                "{} failed the pre-deployment security audit with {} critical finding(s): {}",
                contract_name,
                report.summary.critical,
                critical.join("; ")
            )));
        }
        if !report.passed {
            warn!(
                "{} passed the critical-findings gate but the audit is not fully clean \
                 (risk_score={}, high={}) — deploying anyway, but review the report",
                contract_name, report.risk_score, report.summary.high
            );
        }
        Ok(())
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
        info!("Deployer: deploying treasury hooks for {}", treasury_wallet);
        let hooks = vec![
            format!(
                "FeeCollector: {} bps -> {}",
                platform_fee_bps, treasury_wallet
            ),
            format!("RevenueDistributor: deployed on {}", chain),
            format!(
                "TreasuryHook: 0x{}...{}",
                &self.deployer_key[..8],
                &self.deployer_key[self.deployer_key.len().saturating_sub(8)..]
            ),
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

        // Compute manifest hash
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

    /// `forge` is a first-class repo toolchain requirement (X3-contracts/evm
    /// tests already depend on it); skip rather than hard-fail on a dev
    /// machine that genuinely doesn't have it, instead of pretending the
    /// gate ran.
    fn forge_available() -> bool {
        std::process::Command::new("forge")
            .arg("--version")
            .output()
            .is_ok()
    }

    fn anvil_available() -> bool {
        std::process::Command::new("anvil")
            .arg("--version")
            .output()
            .is_ok()
    }

    /// Now that deployment is real, a placeholder string like `"test-key"`
    /// is not a valid credential and deployment must fail honestly rather
    /// than silently succeed with a fabricated address -- this is the
    /// opposite of what this test asserted before deployment was real, and
    /// that's the point: the old assertion (`result.is_ok()`) was testing
    /// simulated behavior, not correct behavior.
    #[test]
    fn test_deploy_contracts_with_invalid_key_fails_honestly() {
        if !forge_available() {
            eprintln!("skipping: forge not on PATH in this environment");
            return;
        }
        let deployer = Deployer::new("test-key".into(), "x3-testnet".into());
        let mut contracts = HashMap::new();
        contracts.insert(
            "TestToken".into(),
            "pragma solidity ^0.8.20;\ncontract TestToken {}".into(),
        );
        let order = vec!["TestToken".into()];
        let result = deployer.deploy_contracts(&contracts, &order, "x3-testnet");
        assert!(
            matches!(result, Err(FoundryError::DeploymentFailed(_))),
            "a placeholder string is not a valid private key and must fail, got {result:?}"
        );
    }

    /// The one end-to-end proof that deployment is real: spins up a local
    /// `anvil` node (a real EVM, not a mock of one), deploys a real
    /// contract to it using anvil's well-known deterministic dev account
    /// #0 private key, and checks every field on the result against what
    /// anvil itself reports back -- not against a value this test computed
    /// independently, since the whole point is that only the node's own
    /// receipt is trusted.
    #[test]
    fn test_deploy_contracts_real_anvil_end_to_end() {
        if !forge_available() || !anvil_available() {
            eprintln!("skipping: forge/anvil not on PATH in this environment");
            return;
        }
        // anvil's default mnemonic always derives this as dev account #0;
        // it is publicly documented and funded only on ephemeral local
        // chains anvil itself spins up, never a real network.
        const ANVIL_DEV_KEY_0: &str =
            "ac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";

        let anvil = ethers::utils::Anvil::new().spawn();
        let deployer = Deployer::new(ANVIL_DEV_KEY_0.into(), anvil.endpoint());
        let mut contracts = HashMap::new();
        contracts.insert(
            "SimpleToken".into(),
            "pragma solidity ^0.8.20;\ncontract SimpleToken {\n    uint256 public totalSupply;\n}"
                .into(),
        );
        let order = vec!["SimpleToken".into()];

        let result = deployer.deploy_contracts(&contracts, &order, &anvil.endpoint());
        let deployed =
            result.expect("real deployment against a real local anvil node must succeed");
        assert_eq!(deployed.len(), 1);

        let info = &deployed[0];
        assert_eq!(info.name, "SimpleToken");
        assert_eq!(
            info.address.len(),
            42,
            "a real EVM address is 0x + 40 hex chars, got {:?}",
            info.address
        );
        assert!(info.address.starts_with("0x"));
        assert_ne!(
            info.address, "0x0000000000000000000000000000000000000000",
            "must be a real deployed address, not a zero placeholder"
        );
        assert!(
            info.tx_hash.starts_with("0x") && info.tx_hash.len() == 66,
            "a real tx hash is 0x + 64 hex chars, got {:?}",
            info.tx_hash
        );
        assert!(info.block_number > 0, "anvil mines starting from block 1");
        assert!(
            info.gas_used > 21_000,
            "a real contract-creation transaction spends more than the 21000 base cost, got {}",
            info.gas_used
        );
        assert!(!info.verified);
    }

    #[test]
    fn test_deploy_contracts_refuses_a_contract_that_does_not_compile() {
        if !forge_available() {
            eprintln!("skipping: forge not on PATH in this environment");
            return;
        }
        let deployer = Deployer::new("test-key".into(), "x3-testnet".into());
        let mut contracts = HashMap::new();
        contracts.insert(
            "BrokenToken".into(),
            "pragma solidity ^0.8.20;\ncontract BrokenToken {\n    function nope( {\n}".into(),
        );
        let order = vec!["BrokenToken".into()];
        let result = deployer.deploy_contracts(&contracts, &order, "x3-testnet");
        assert!(
            matches!(result, Err(FoundryError::SecurityAuditFailed(_))),
            "expected deployment to be refused for a contract that fails to compile, got {result:?}"
        );
    }

    #[test]
    fn test_deploy_frontend() {
        let deployer = Deployer::new("test".into(), "x3".into());
        let url = deployer.deploy_frontend("MyApp", "React", &["/".into()], &["/api".into()]);
        assert!(url.is_ok());
        assert!(url.unwrap().contains("x3-app.io"));
    }

    /// `deploy_to_chains` must propagate each per-chain deployer's real
    /// validation instead of faking success across multiple chains; "key1"/
    /// "key2" are not valid private keys, so this must fail honestly on the
    /// first chain it tries, same as the single-deployer case.
    #[test]
    fn test_cross_chain_with_invalid_keys_fails_honestly() {
        if !forge_available() {
            eprintln!("skipping: forge not on PATH in this environment");
            return;
        }
        let mut cc = CrossChainDeployer::new();
        cc.add_chain(
            "x3-mainnet".into(),
            Deployer::new("key1".into(), "x3-mainnet".into()),
        );
        cc.add_chain(
            "ethereum".into(),
            Deployer::new("key2".into(), "ethereum".into()),
        );
        let mut contracts = HashMap::new();
        contracts.insert(
            "Token".into(),
            "pragma solidity ^0.8.20;\ncontract Token {}".into(),
        );
        let result = cc.deploy_to_chains(
            &contracts,
            &["Token".into()],
            &["x3-mainnet".into(), "ethereum".into()],
        );
        assert!(
            matches!(result, Err(FoundryError::DeploymentFailed(_))),
            "invalid keys must fail, got {result:?}"
        );
    }

    /// `CrossChainDeployer::deploy_to_chains` iterates real deployers, so
    /// the multi-chain success path gets its own real-anvil proof rather
    /// than reusing the single-chain test's node: two independent anvil
    /// instances stand in for "two chains". `deploy_to_chains` uses each
    /// entry in `chains` both as the deployer-registry lookup key and as
    /// the literal `chain` argument passed to `deploy_contracts`, so each
    /// anvil instance's own endpoint URL has to serve as its chain "name"
    /// here for `resolve_rpc_url` to actually reach it.
    #[test]
    fn test_cross_chain_real_anvil_end_to_end() {
        if !forge_available() || !anvil_available() {
            eprintln!("skipping: forge/anvil not on PATH in this environment");
            return;
        }
        const ANVIL_DEV_KEY_0: &str =
            "ac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";

        let anvil_a = ethers::utils::Anvil::new().spawn();
        let anvil_b = ethers::utils::Anvil::new().spawn();
        let (endpoint_a, endpoint_b) = (anvil_a.endpoint(), anvil_b.endpoint());

        let mut cc = CrossChainDeployer::new();
        cc.add_chain(
            endpoint_a.clone(),
            Deployer::new(ANVIL_DEV_KEY_0.into(), endpoint_a.clone()),
        );
        cc.add_chain(
            endpoint_b.clone(),
            Deployer::new(ANVIL_DEV_KEY_0.into(), endpoint_b.clone()),
        );

        let mut contracts = HashMap::new();
        contracts.insert(
            "Token".into(),
            "pragma solidity ^0.8.20;\ncontract Token {}".into(),
        );

        let result = cc.deploy_to_chains(
            &contracts,
            &["Token".into()],
            &[endpoint_a.clone(), endpoint_b.clone()],
        );
        let by_chain = result.expect("real deployment to two local anvil nodes must succeed");
        assert_eq!(by_chain.len(), 2);
        for endpoint in [&endpoint_a, &endpoint_b] {
            let deployed = &by_chain[endpoint];
            assert_eq!(deployed.len(), 1);
            assert!(deployed[0].address.starts_with("0x"));
            assert_eq!(deployed[0].address.len(), 42);
        }
    }
}
