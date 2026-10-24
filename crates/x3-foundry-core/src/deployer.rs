use crate::error::FoundryError;
use crate::evm_deploy;
use crate::types::{DAppType, DeploymentReceipt};
use chrono::Utc;
use ethers::signers::{LocalWallet, Signer};
use ethers::utils::hash_message;
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

    fn signing_wallet(&self) -> Result<LocalWallet, FoundryError> {
        self.deployer_key
            .parse()
            .map_err(|_| FoundryError::DeploymentFailed("invalid EVM deployer signing key".into()))
    }

    pub fn deployer_address(&self) -> Result<String, FoundryError> {
        Ok(format!("{:?}", self.signing_wallet()?.address()))
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

    /// Refuse the full pipeline before signing or sending any chain transaction.
    /// Publication adapters are not implemented; none of these operations may
    /// manufacture evidence that a remote deployment exists.
    pub(crate) fn ensure_publication_backends(&self) -> Result<(), FoundryError> {
        Err(Self::unavailable_backend("frontend publishing"))
    }

    fn unavailable_backend(name: &str) -> FoundryError {
        FoundryError::DeploymentFailed(format!("{name} backend is not configured"))
    }

    pub fn deploy_frontend(
        &self,
        _app_name: &str,
        _frontend_framework: &str,
        _routes: &[String],
        _api_endpoints: &[String],
    ) -> Result<String, FoundryError> {
        Err(Self::unavailable_backend("frontend publishing"))
    }

    pub fn deploy_metadata(
        &self,
        _app_name: &str,
        _description: &str,
        _features: &[String],
    ) -> Result<String, FoundryError> {
        Err(Self::unavailable_backend("metadata publishing"))
    }

    pub fn deploy_treasury_hooks(
        &self,
        _treasury_wallet: &str,
        _platform_fee_bps: u16,
        _chain: &str,
    ) -> Result<Vec<String>, FoundryError> {
        Err(Self::unavailable_backend("treasury contract deployment"))
    }

    pub fn deploy_marketplace_listing(
        &self,
        _title: &str,
        _description: &str,
        _tags: &[String],
        _chain: &str,
    ) -> Result<String, FoundryError> {
        Err(Self::unavailable_backend("marketplace publishing"))
    }

    pub fn deploy_analytics(&self, _app_name: &str, _chain: &str) -> Result<String, FoundryError> {
        Err(Self::unavailable_backend("analytics registration"))
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
    ) -> Result<DeploymentManifest, FoundryError> {
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
            deployer_address: self.deployer_address()?,
            manifest_hash: String::new(), // Will be computed
        };

        // Compute manifest hash
        let manifest_json = serde_json::to_string(&manifest).map_err(|_| {
            FoundryError::DeploymentFailed("could not serialize deployment manifest".into())
        })?;
        let mut hasher = Sha256::new();
        hasher.update(manifest_json.as_bytes());
        let hash = hex::encode(hasher.finalize());

        Ok(DeploymentManifest {
            manifest_hash: hash,
            ..manifest
        })
    }

    /// Sign an EIP-191 personal message containing the complete receipt as UTF-8
    /// JSON with recursively sorted object keys and an empty signature field.
    /// The public signer address and signing timestamp are part of that message.
    pub fn sign_receipt(&self, receipt: &mut DeploymentReceipt) -> Result<(), FoundryError> {
        let wallet = self.signing_wallet()?;
        // Prepare a copy so an invalid key or signing failure cannot partially
        // rewrite a previously valid receipt.
        let mut signed = receipt.clone();
        signed.deployer_address = format!("{:?}", wallet.address());
        signed.signature.clear();
        signed.signed_at = Utc::now();
        let mut value = serde_json::to_value(&signed).map_err(|_| {
            FoundryError::DeploymentFailed("could not serialize deployment receipt".into())
        })?;
        // Canonicalize nested maps, including contract_addresses, independently
        // of HashMap iteration order and serde_json's preserve_order feature.
        value.sort_all_objects();
        let payload = serde_json::to_vec(&value).map_err(|_| {
            FoundryError::DeploymentFailed("could not serialize deployment receipt".into())
        })?;
        let signature = wallet.sign_hash(hash_message(payload)).map_err(|_| {
            FoundryError::DeploymentFailed("could not sign deployment receipt".into())
        })?;
        signed.signature = format!("0x{signature}");
        *receipt = signed;
        Ok(())
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
    ) -> Result<HashMap<String, DeploymentManifest>, FoundryError> {
        let mut manifests = HashMap::new();
        for (chain, contracts) in chain_results {
            let deployer = self.deployers.get(chain).ok_or_else(|| {
                FoundryError::DeploymentFailed("no deployer configured for manifest chain".into())
            })?;
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
            )?;
            manifests.insert(chain.clone(), manifest);
        }
        Ok(manifests)
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

    #[test]
    fn manifest_never_publishes_the_signing_key() {
        use ethers::signers::{LocalWallet, Signer};
        let wallet = LocalWallet::new(&mut ethers::core::rand::thread_rng());
        let key = hex::encode(wallet.signer().to_bytes());
        let expected_address = format!("{:?}", wallet.address());
        let deployer = Deployer::new(key.clone(), "local-test".into());
        let manifest = deployer
            .generate_manifest(
                "identity-regression",
                &DAppType::TokenLaunchpad,
                &[],
                None,
                None,
                vec![],
                None,
                None,
                "local-test",
            )
            .expect("valid local signing key");
        let serialized = serde_json::to_string(&manifest).unwrap();
        assert!(
            !serialized.contains(&key),
            "public manifest contains signing material"
        );
        assert_eq!(manifest.deployer_address, expected_address);
        assert!(matches!(
            deployer.deploy_treasury_hooks(&expected_address, 200, "local-test"),
            Err(FoundryError::DeploymentFailed(_))
        ));
    }

    #[test]
    fn invalid_short_signing_key_is_rejected_without_a_treasury_hook_panic() {
        let deployer = Deployer::new("x".into(), "local-test".into());
        let result = std::panic::catch_unwind(|| {
            deployer.deploy_treasury_hooks("treasury", 200, "local-test")
        });
        assert!(result.is_ok(), "untrusted key length must not panic");
        assert!(result.unwrap().is_err());
    }

    #[test]
    fn receipt_signature_is_publicly_recoverable_and_binds_its_manifest() {
        use ethers::signers::{LocalWallet, Signer};
        use ethers::types::Signature;
        let wallet = LocalWallet::new(&mut ethers::core::rand::thread_rng());
        let expected_address = wallet.address();
        let deployer = Deployer::new(hex::encode(wallet.signer().to_bytes()), "local-test".into());
        let mut receipt = DeploymentReceipt::new(
            "id".into(),
            "name".into(),
            DAppType::TokenLaunchpad,
            "local-test".into(),
            format!("{:?}", expected_address),
        );
        receipt.manifest_hash = "manifest-one".into();
        deployer
            .sign_receipt(&mut receipt)
            .expect("valid local signing key");
        let signature: Signature = receipt
            .signature
            .parse()
            .expect("receipt requires a recoverable signature");
        let payload = |receipt: &DeploymentReceipt| {
            let mut value = serde_json::to_value(receipt).unwrap();
            value["signature"] = serde_json::Value::String(String::new());
            value.sort_all_objects();
            serde_json::to_vec(&value).unwrap()
        };
        assert_eq!(
            signature.recover(payload(&receipt).as_slice()).unwrap(),
            expected_address
        );
        receipt.manifest_hash = "manifest-two".into();
        assert_ne!(
            signature.recover(payload(&receipt).as_slice()).unwrap(),
            expected_address
        );
    }

    #[test]
    fn signing_failure_preserves_the_original_receipt() {
        let wallet = LocalWallet::new(&mut ethers::core::rand::thread_rng());
        let deployer = Deployer::new(hex::encode(wallet.signer().to_bytes()), "local-test".into());
        let mut receipt = DeploymentReceipt::new(
            "id".into(),
            "name".into(),
            DAppType::TokenLaunchpad,
            "local-test".into(),
            deployer.deployer_address().unwrap(),
        );
        deployer.sign_receipt(&mut receipt).unwrap();
        let original = serde_json::to_value(&receipt).unwrap();
        let invalid = Deployer::new("invalid-test-key".into(), "local-test".into());
        assert!(invalid.sign_receipt(&mut receipt).is_err());
        assert_eq!(serde_json::to_value(&receipt).unwrap(), original);
        assert!(invalid
            .generate_manifest(
                "app",
                &DAppType::TokenLaunchpad,
                &[],
                None,
                None,
                vec![],
                None,
                None,
                "local-test"
            )
            .is_err());
        let mut chains = HashMap::new();
        chains.insert("unconfigured-chain".into(), vec![]);
        assert!(CrossChainDeployer::new()
            .generate_cross_chain_manifest("app", &DAppType::TokenLaunchpad, &chains)
            .is_err());
    }

    #[test]
    fn missing_publication_backends_cannot_report_deployment_success() {
        let wallet = LocalWallet::new(&mut ethers::core::rand::thread_rng());
        let deployer = Deployer::new(hex::encode(wallet.signer().to_bytes()), "local-test".into());
        let outcomes = [
            deployer.deploy_frontend("app", "React", &[], &[]),
            deployer.deploy_metadata("app", "description", &[]),
            deployer.deploy_marketplace_listing("app", "description", &[], "local-test"),
            deployer.deploy_analytics("app", "local-test"),
        ];
        for outcome in outcomes {
            assert!(
                matches!(outcome, Err(FoundryError::DeploymentFailed(_))),
                "unconfigured publisher must not fabricate deployment evidence"
            );
        }
    }

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
    fn test_deploy_frontend_refuses_unconfigured_publisher() {
        let deployer = Deployer::new("test".into(), "x3".into());
        let url = deployer.deploy_frontend("MyApp", "React", &["/".into()], &["/api".into()]);
        assert!(matches!(url, Err(FoundryError::DeploymentFailed(_))));
        assert!(url
            .unwrap_err()
            .to_string()
            .contains("frontend publishing backend"));
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
