//! X3 Relayer — Watches external chains for deposit events and submits proofs to X3.
//!
//! Also watches X3 for withdrawal events and submits release proofs to external gateways.
//!
//! # Architecture
//! - EVM Chain Watcher: polls gateway contracts for DepositLocked events
//! - X3 Chain Watcher: polls X3 for WithdrawalRequested events
//! - Proof Builder: constructs Merkle inclusion proofs from event data
//! - Submitter: sends proofs to X3/external gateway via RPC
//!
//! # Configuration
//! Config file path can be set via X3_RELAYER_CONFIG env var.
//! Default: crates/x3-relayer/relayer-config.testnet.yaml
//! Env vars in the config (${X3_RPC_URL}, ${EVM_SEPOLIA_RPC}, etc.) are
//! expanded at load time.

use log::debug;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use x3_cross_vm_bridge::CrossVmOperation;

// ── Configuration ───────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct SvmClusterConfig {
    pub cluster_name: String,
    pub rpc_endpoint: String,
    pub finality_threshold: u64,    // slots
    pub slot_poll_interval_ms: u64, // ms
    pub max_concurrent_requests: u32,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct RelayerConfig {
    /// X3 node RPC endpoint
    x3_rpc: String,
    /// External EVM chain RPC endpoints (chain_id -> url)
    evm_rpcs: HashMap<u64, String>,
    /// External gateway contract addresses (chain_id -> address)
    gateway_addresses: HashMap<u64, String>,
    /// X3 kernel bridge contract address (on X3 EVM)
    x3_bridge_address: String,
    /// Number of confirmations required per chain (chain_id -> count)
    confirmations: HashMap<u64, u64>,
    /// Poll interval in seconds
    poll_interval_secs: u64,
    /// Maximum retries for failed proof submissions
    max_retries: u32,
    /// Private key for submitting transactions
    relayer_private_key: String,
    /// Database path for tracking processed events
    db_path: String,
    /// SVM (Solana) cluster configurations
    svm_clusters: Vec<SvmClusterConfig>,
}

#[derive(Debug, Deserialize)]
struct TypedX3Config {
    #[serde(rename = "rpc_url")]
    rpc_url: String,
    #[serde(rename = "relayer_seed_phrase")]
    relayer_seed_phrase: Option<String>,
    #[serde(rename = "relayer_custody_key_id")]
    relayer_custody_key_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TypedSubmissionConfig {
    #[serde(rename = "timeout_secs")]
    timeout_secs: u64,
    #[serde(rename = "max_retries")]
    max_retries: u32,
}

#[derive(Debug, Deserialize)]
struct TypedEvmChainEntry {
    #[serde(rename = "chain_id")]
    chain_id: u64,
    #[serde(rename = "rpc_endpoint")]
    rpc_endpoint: String,
    #[serde(rename = "state_root_contract")]
    state_root_contract: String,
    #[serde(rename = "finality_threshold")]
    finality_threshold: u64,
}

#[derive(Debug, Deserialize)]
struct TypedSvmClusterEntry {
    #[serde(rename = "cluster_name")]
    cluster_name: String,
    #[serde(rename = "rpc_endpoint")]
    rpc_endpoint: String,
    #[serde(rename = "finality_threshold")]
    finality_threshold: Option<u64>,
    #[serde(rename = "slot_poll_interval_ms")]
    slot_poll_interval_ms: Option<u64>,
    #[serde(rename = "max_concurrent_requests")]
    max_concurrent_requests: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct TypedConfig {
    x3: TypedX3Config,
    submission: TypedSubmissionConfig,
    #[serde(rename = "evm_chains")]
    evm_chains: Vec<TypedEvmChainEntry>,
    #[serde(rename = "svm_clusters", default)]
    svm_clusters: Vec<TypedSvmClusterEntry>,
}

impl RelayerConfig {
    fn from_env() -> Self {
        // Try loading the typed config from YAML first; fall back to env vars.
        if let Ok(cfg) = Self::from_yaml() {
            return cfg;
        }
        Self::from_env_vars()
    }

    fn from_yaml() -> Result<Self, Box<dyn std::error::Error>> {
        let config_path = std::env::var("X3_RELAYER_CONFIG")
            .unwrap_or_else(|_| "crates/x3-relayer/relayer-config.testnet.yaml".into());
        let content = std::fs::read_to_string(&config_path)?;

        // Expand ${VAR} / $VAR in raw YAML before parsing
        let expanded = Self::expand_env_vars(&content);
        let cfg: TypedConfig = serde_yaml::from_str(&expanded)?;

        let x3_rpc = cfg.x3.rpc_url;
        let relayer_private_key = cfg
            .x3
            .relayer_seed_phrase
            .or(cfg.x3.relayer_custody_key_id)
            .unwrap_or_default();
        let poll_interval_secs = cfg.submission.timeout_secs;
        let max_retries = cfg.submission.max_retries;

        let mut evm_rpcs = HashMap::new();
        let mut gateway_addresses = HashMap::new();
        let mut confirmations = HashMap::new();
        for chain in &cfg.evm_chains {
            evm_rpcs.insert(chain.chain_id, chain.rpc_endpoint.clone());
            gateway_addresses.insert(chain.chain_id, chain.state_root_contract.clone());
            confirmations.insert(chain.chain_id, chain.finality_threshold);
        }

        let svm_clusters: Vec<SvmClusterConfig> = cfg
            .svm_clusters
            .iter()
            .map(|s| SvmClusterConfig {
                cluster_name: s.cluster_name.clone(),
                rpc_endpoint: s.rpc_endpoint.clone(),
                finality_threshold: s.finality_threshold.unwrap_or(32),
                slot_poll_interval_ms: s.slot_poll_interval_ms.unwrap_or(2000),
                max_concurrent_requests: s.max_concurrent_requests.unwrap_or(10),
            })
            .collect();

        Ok(Self {
            x3_rpc,
            evm_rpcs,
            gateway_addresses,
            x3_bridge_address: std::env::var("X3_BRIDGE").unwrap_or_default(),
            confirmations,
            poll_interval_secs,
            max_retries,
            relayer_private_key,
            db_path: std::env::var("DB_PATH").unwrap_or_else(|_| "/tmp/x3-relayer.db".into()),
            svm_clusters,
        })
    }

    fn from_env_vars() -> Self {
        let svm_clusters = {
            let mut clusters = Vec::new();
            if let Ok(url) = std::env::var("SOLANA_RPC_URL") {
                clusters.push(SvmClusterConfig {
                    cluster_name: "solana-env".to_string(),
                    rpc_endpoint: url,
                    finality_threshold: 32,
                    slot_poll_interval_ms: 2000,
                    max_concurrent_requests: 10,
                });
            }
            clusters
        };

        Self {
            x3_rpc: std::env::var("X3_RPC").unwrap_or_else(|_| "ws://127.0.0.1:9944".into()),
            evm_rpcs: {
                let mut m = HashMap::new();
                if let Ok(url) = std::env::var("ETH_RPC") {
                    m.insert(1, url);
                }
                if let Ok(url) = std::env::var("BASE_RPC") {
                    m.insert(8453, url);
                }
                if let Ok(url) = std::env::var("ARB_RPC") {
                    m.insert(42161, url);
                }
                m
            },
            gateway_addresses: {
                let mut m = HashMap::new();
                if let Ok(addr) = std::env::var("ETH_GATEWAY") {
                    m.insert(1, addr);
                }
                if let Ok(addr) = std::env::var("BASE_GATEWAY") {
                    m.insert(8453, addr);
                }
                if let Ok(addr) = std::env::var("ARB_GATEWAY") {
                    m.insert(42161, addr);
                }
                m
            },
            x3_bridge_address: std::env::var("X3_BRIDGE").unwrap_or_default(),
            confirmations: {
                let mut m = HashMap::new();
                m.insert(1, 64);
                m.insert(8453, 32);
                m.insert(42161, 32);
                m
            },
            poll_interval_secs: std::env::var("POLL_INTERVAL")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(30),
            max_retries: std::env::var("MAX_RETRIES")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(10),
            relayer_private_key: std::env::var("RELAYER_KEY").unwrap_or_default(),
            db_path: std::env::var("DB_PATH").unwrap_or_else(|_| "/tmp/x3-relayer.db".into()),
            svm_clusters,
        }
    }

    /// Expand ${VAR} and $VAR shell-style references in the raw YAML string.
    fn expand_env_vars(raw: &str) -> String {
        let mut result = String::with_capacity(raw.len());
        let mut chars = raw.chars().peekable();

        while let Some(c) = chars.next() {
            if c == '$' {
                if chars.peek() == Some(&'{') {
                    chars.next(); // consume '{'
                    let mut var = String::new();
                    while let Some(&nc) = chars.peek() {
                        if nc == '}' {
                            chars.next();
                            break;
                        }
                        var.push(nc);
                        chars.next();
                    }
                    let val = std::env::var(&var).unwrap_or_default();
                    result.push_str(&val);
                } else {
                    let mut var = String::new();
                    while let Some(&nc) = chars.peek() {
                        if nc.is_alphanumeric() || nc == '_' {
                            var.push(nc);
                            chars.next();
                        } else {
                            break;
                        }
                    }
                    let val = std::env::var(&var).unwrap_or_default();
                    result.push_str(&val);
                }
            } else {
                result.push(c);
            }
        }
        result
    }
}

// ── Event Types ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct ExternalDepositEvent {
    message_id: [u8; 32],
    token_address: [u8; 20],
    depositor: [u8; 20],
    x3_recipient: Vec<u8>,
    amount: u128,
    nonce: u128,
    chain_id: u64,
    block_number: u64,
    tx_hash: [u8; 32],
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct SvmDepositEvent {
    pub source_address: [u8; 32],
    pub destination_address: [u8; 32],
    pub amount: u128,
    pub asset_id: [u8; 32],
    pub slot: u64,
    pub tx_hash: String,
    pub program_id: [u8; 32],
    pub cluster_name: String,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
enum WatchedDepositEvent {
    Evm(ExternalDepositEvent),
    Svm(SvmDepositEvent),
}

// ── Event signatures (keccak256) ────────────────────────────────────────────
// event DepositLocked(
//     bytes32 indexed messageId,
//     address indexed token,
//     address indexed depositor,
//     bytes x3Recipient,
//     uint256 amount,
//     uint256 nonce,
//     uint256 chainId
// );
const DEPOSIT_LOCKED_SIGNATURE: &str =
    "DepositLocked(bytes32,address,address,bytes,uint256,uint256,uint256)";

// event WithdrawalReleased(
//     bytes32 indexed withdrawalId,
//     address indexed recipient,
//     address indexed relayer,
//     uint256 amount
// );
const WITHDRAWAL_RELEASED_SIGNATURE: &str = "WithdrawalReleased(bytes32,address,address,uint256)";

const SIGNED_DEPOSIT_RELAY_MAGIC: &[u8] = b"X3DP1";

#[cfg(test)]
#[derive(Debug, Clone, PartialEq, Eq)]
struct SignedDepositRelayEnvelope {
    lock_proof: Vec<u8>,
    deposit_payload: Vec<u8>,
}

fn build_deposit_proof_payload(event: &ExternalDepositEvent) -> Vec<u8> {
    let mut proof_payload = Vec::new();

    proof_payload.extend_from_slice(&event.chain_id.to_le_bytes());
    proof_payload.extend_from_slice(&event.message_id);
    proof_payload.extend_from_slice(&event.token_address);
    proof_payload.extend_from_slice(&event.depositor);

    let recipient_len = event.x3_recipient.len() as u32;
    match recipient_len {
        0..=63 => proof_payload.push((recipient_len << 2) as u8),
        64..=16383 => {
            let v = (recipient_len << 2) | 1;
            proof_payload.extend_from_slice(&(v as u16).to_le_bytes());
        }
        _ => {
            let v = (recipient_len << 2) | 2;
            proof_payload.extend_from_slice(&(v).to_le_bytes());
        }
    }
    proof_payload.extend_from_slice(&event.x3_recipient);
    proof_payload.extend_from_slice(&event.amount.to_le_bytes());
    proof_payload.extend_from_slice(&event.nonce.to_le_bytes());
    proof_payload.extend_from_slice(&[0u8; 20]);
    proof_payload.extend_from_slice(&event.block_number.to_le_bytes());

    proof_payload
}

fn build_lock_proof(
    operation: &CrossVmOperation,
    signer_seed: &str,
) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    use codec::Encode;
    use sp_core::Pair as _;

    let pair = sp_core::sr25519::Pair::from_string(signer_seed, None)
        .map_err(|e| format!("load relay proof signer failed: {e:?}"))?;
    let operation_hash = sp_core::hashing::blake2_256(&operation.encode());
    let signature = pair.sign(&operation_hash);

    let mut proof = Vec::with_capacity(33 + 32 + 64);
    proof.extend_from_slice(&operation_hash);
    proof.push(1);
    proof.extend_from_slice(pair.public().as_ref());
    proof.extend_from_slice(signature.as_ref());
    Ok(proof)
}

fn build_signed_deposit_relay_envelope(
    event: &ExternalDepositEvent,
    signer_seed: &str,
) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    if event.x3_recipient.len() != 32 {
        return Err(format!(
            "x3 recipient must be 32 bytes, got {}",
            event.x3_recipient.len()
        )
        .into());
    }

    let operation = CrossVmOperation::TransferToSvm {
        source: event.depositor,
        destination: event.x3_recipient.clone(),
        amount: event.amount,
    };
    let lock_proof = build_lock_proof(&operation, signer_seed)?;
    let deposit_payload = build_deposit_proof_payload(event);

    let proof_len: u32 = lock_proof
        .len()
        .try_into()
        .map_err(|_| "lock proof too large for deposit relay envelope")?;
    let mut envelope = Vec::with_capacity(
        SIGNED_DEPOSIT_RELAY_MAGIC.len() + 4 + lock_proof.len() + deposit_payload.len(),
    );
    envelope.extend_from_slice(SIGNED_DEPOSIT_RELAY_MAGIC);
    envelope.extend_from_slice(&proof_len.to_le_bytes());
    envelope.extend_from_slice(&lock_proof);
    envelope.extend_from_slice(&deposit_payload);
    Ok(envelope)
}

#[cfg(test)]
fn decode_signed_deposit_relay_envelope(
    bytes: &[u8],
) -> Result<SignedDepositRelayEnvelope, Box<dyn std::error::Error + Send + Sync>> {
    if !bytes.starts_with(SIGNED_DEPOSIT_RELAY_MAGIC) {
        return Err("signed deposit relay envelope missing X3DP1 magic".into());
    }
    let mut offset = SIGNED_DEPOSIT_RELAY_MAGIC.len();
    let proof_len_bytes = bytes
        .get(offset..offset + 4)
        .ok_or("signed deposit relay envelope missing proof length")?;
    let proof_len = u32::from_le_bytes(proof_len_bytes.try_into()?) as usize;
    offset += 4;
    let proof_end = offset
        .checked_add(proof_len)
        .ok_or("signed deposit relay envelope proof length overflow")?;
    let lock_proof = bytes
        .get(offset..proof_end)
        .ok_or("signed deposit relay envelope truncated lock proof")?
        .to_vec();
    let deposit_payload = bytes
        .get(proof_end..)
        .ok_or("signed deposit relay envelope missing deposit payload")?
        .to_vec();
    if lock_proof.is_empty() || deposit_payload.is_empty() {
        return Err("signed deposit relay envelope requires proof and deposit payload".into());
    }
    Ok(SignedDepositRelayEnvelope {
        lock_proof,
        deposit_payload,
    })
}

/// Compute the keccak256 topic hash for an event signature.
fn event_topic(sig: &str) -> String {
    use sha3::{Digest, Keccak256};
    let mut hasher = Keccak256::new();
    hasher.update(sig.as_bytes());
    let result = hasher.finalize();
    format!("0x{}", hex::encode(result))
}

// ── JSON-RPC helpers ─────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
struct JsonRpcRequest {
    jsonrpc: &'static str,
    id: u64,
    method: String,
    params: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct JsonRpcResponse {
    #[serde(default)]
    result: serde_json::Value,
    #[serde(default)]
    error: Option<serde_json::Value>,
}

fn rpc_call_blocking(
    endpoint: &str,
    method: &str,
    params: serde_json::Value,
    timeout_ms: u64,
) -> Result<JsonRpcResponse, Box<dyn std::error::Error + Send + Sync>> {
    let client = reqwest::blocking::Client::new();
    let req = JsonRpcRequest {
        jsonrpc: "2.0",
        id: 1,
        method: method.to_string(),
        params,
    };
    let resp = client
        .post(endpoint)
        .json(&req)
        .timeout(Duration::from_millis(timeout_ms))
        .send()?;
    let body: JsonRpcResponse = resp.json()?;
    Ok(body)
}

/// Read a hex string (with or without 0x prefix) into a fixed-size array.
fn hex_to_bytes<const N: usize>(s: &str) -> Option<[u8; N]> {
    let s = s.strip_prefix("0x").unwrap_or(s);
    let bytes = hex::decode(s).ok()?;
    if bytes.len() != N {
        return None;
    }
    let mut out = [0u8; N];
    out.copy_from_slice(&bytes);
    Some(out)
}

/// Read a hex string into a Vec<u8>
fn hex_to_vec(s: &str) -> Option<Vec<u8>> {
    let s = s.strip_prefix("0x").unwrap_or(s);
    hex::decode(s).ok()
}

// ── Relayer logic ───────────────────────────────────────────────────────────

struct Relayer {
    config: RelayerConfig,
    processed_deposits: HashMap<[u8; 32], bool>,
    processed_withdrawals: HashMap<[u8; 32], bool>,
    /// Checkpoint: last processed block per EVM chain (chain_id -> block)
    last_evm_block: HashMap<u64, u64>,
    /// Checkpoint: last processed withdrawal block per EVM chain
    last_evm_withdrawal_block: HashMap<u64, u64>,
    /// Checkpoint: last processed slot per SVM cluster (cluster_name -> slot)
    last_svm_slot: HashMap<String, u64>,
    /// Watchdog: consecutive empty polls per chain
    evm_stale_counter: HashMap<u64, u64>,
    svm_stale_counter: HashMap<String, u64>,
    /// Nonce tracker for gateway extrinsic submissions
    _submission_nonce: u32,
}

impl Relayer {
    fn new(config: RelayerConfig) -> Self {
        Self {
            config,
            processed_deposits: HashMap::new(),
            processed_withdrawals: HashMap::new(),
            last_evm_block: HashMap::new(),
            last_evm_withdrawal_block: HashMap::new(),
            last_svm_slot: HashMap::new(),
            evm_stale_counter: HashMap::new(),
            svm_stale_counter: HashMap::new(),
            _submission_nonce: 0,
        }
    }

    /// Log connectivity status on startup.
    fn log_startup_status(&self) {
        println!(
            "[relayer] Starting X3 Relayer (poll_interval={}s)...",
            self.config.poll_interval_secs
        );
        println!("[relayer] X3 RPC: {}", self.config.x3_rpc);
        let x3_ok = rpc_call_blocking(
            &self.config.x3_rpc,
            "system_health",
            serde_json::json!([]),
            10000,
        );
        match x3_ok {
            Ok(resp) => {
                if let Some(err) = &resp.error {
                    println!("[relayer] X3 RPC connectivity: FAIL (error: {})", err);
                } else {
                    println!("[relayer] X3 RPC connectivity: OK");
                }
            }
            Err(e) => {
                println!("[relayer] X3 RPC connectivity: FAIL ({})", e);
            }
        }

        for (&chain_id, rpc) in &self.config.evm_rpcs {
            let gw = self
                .config
                .gateway_addresses
                .get(&chain_id)
                .map(|s| s.as_str())
                .unwrap_or("N/A");
            let ok = rpc_call_blocking(rpc, "eth_blockNumber", serde_json::json!([]), 10000);
            let status = match ok {
                Ok(r) if r.error.is_none() => "OK",
                _ => "FAIL",
            };
            println!(
                "[relayer] EVM chain {}: rpc={}, gateway={} [{}]",
                chain_id, rpc, gw, status
            );
        }
        for svm in &self.config.svm_clusters {
            let ok = rpc_call_blocking(&svm.rpc_endpoint, "getSlot", serde_json::json!([]), 10000);
            let status = match ok {
                Ok(r) if r.error.is_none() => "OK",
                _ => "FAIL",
            };
            println!(
                "[relayer] SVM cluster '{}': rpc={}, finality={} slots [{}]",
                svm.cluster_name, svm.rpc_endpoint, svm.finality_threshold, status
            );
        }
    }

    /// Main relayer loop — polls both external chains and X3
    async fn run(&mut self) {
        self.log_startup_status();

        loop {
            // Watch external EVM chains for deposit events
            let chain_ids: Vec<u64> = self.config.evm_rpcs.keys().copied().collect();
            for &chain_id in &chain_ids {
                match self.watch_evm_deposits(chain_id) {
                    Ok(count) if count > 0 => {
                        println!(
                            "[relayer] Chain {}: processed {} new deposits",
                            chain_id, count
                        );
                        self.evm_stale_counter.insert(chain_id, 0);
                    }
                    Ok(_) => {
                        // Watchdog: no new events
                        let stale = self.evm_stale_counter.entry(chain_id).or_insert(0);
                        *stale = stale.saturating_add(1);
                        if *stale >= 10 {
                            eprintln!(
                                "[relayer] WATCHDOG: EVM chain {} has had no new deposits for {} consecutive polls",
                                chain_id, *stale
                            );
                        }
                    }
                    Err(e) => {
                        eprintln!("[relayer] Error watching EVM chain {}: {:?}", chain_id, e);
                        let stale = self.evm_stale_counter.entry(chain_id).or_insert(0);
                        *stale = stale.saturating_add(1);
                        if *stale >= 5 {
                            eprintln!(
                                "[relayer] WATCHDOG: EVM chain {} has failed {} consecutive polls",
                                chain_id, *stale
                            );
                        }
                    }
                }
            }

            // Watch SVM (Solana) clusters for deposit events
            let svm_configs: Vec<SvmClusterConfig> = self.config.svm_clusters.clone();
            for svm_config in &svm_configs {
                match self.watch_svm_deposits(svm_config) {
                    Ok(count) if count > 0 => {
                        println!(
                            "[relayer] SVM cluster '{}': processed {} new deposits",
                            svm_config.cluster_name, count
                        );
                        self.svm_stale_counter
                            .insert(svm_config.cluster_name.clone(), 0);
                    }
                    Ok(_) => {
                        let stale = self
                            .svm_stale_counter
                            .entry(svm_config.cluster_name.clone())
                            .or_insert(0);
                        *stale = stale.saturating_add(1);
                        if *stale >= 10 {
                            eprintln!(
                                "[relayer] WATCHDOG: SVM cluster '{}' has had no new deposits for {} consecutive polls",
                                svm_config.cluster_name, *stale
                            );
                        }
                    }
                    Err(e) => {
                        eprintln!(
                            "[relayer] Error watching SVM cluster '{}': {:?}",
                            svm_config.cluster_name, e
                        );
                        let stale = self
                            .svm_stale_counter
                            .entry(svm_config.cluster_name.clone())
                            .or_insert(0);
                        *stale = stale.saturating_add(1);
                        if *stale >= 5 {
                            eprintln!(
                                "[relayer] WATCHDOG: SVM cluster '{}' has failed {} consecutive polls",
                                svm_config.cluster_name, *stale
                            );
                        }
                    }
                }
            }

            // Watch external EVM chains for WithdrawalReleased events
            for &chain_id in &chain_ids {
                match self.watch_evm_withdrawals(chain_id) {
                    Ok(count) if count > 0 => {
                        println!(
                            "[relayer] Chain {}: processed {} new withdrawal releases",
                            chain_id, count
                        );
                    }
                    Ok(_) => {}
                    Err(e) => {
                        eprintln!(
                            "[relayer] Error watching EVM withdrawals on chain {}: {:?}",
                            chain_id, e
                        );
                    }
                }
            }

            tokio::time::sleep(Duration::from_secs(self.config.poll_interval_secs)).await;
        }
    }

    /// Poll an external EVM gateway contract for new DepositLocked events.
    /// Returns the number of new deposits successfully processed.
    fn watch_evm_deposits(
        &mut self,
        chain_id: u64,
    ) -> Result<usize, Box<dyn std::error::Error + Send + Sync>> {
        let rpc_url = self
            .config
            .evm_rpcs
            .get(&chain_id)
            .ok_or("No RPC configured for chain")?;
        let gateway = self
            .config
            .gateway_addresses
            .get(&chain_id)
            .ok_or("No gateway configured for chain")?;
        let _confirmations = self
            .config
            .confirmations
            .get(&chain_id)
            .copied()
            .unwrap_or(12);

        let topic = event_topic(DEPOSIT_LOCKED_SIGNATURE);

        // Determine starting block for scanning — use last checkpoint or 0
        let from_block = self.last_evm_block.get(&chain_id).copied().unwrap_or(0);
        let from_block_hex = format!("0x{:x}", from_block);

        // Build eth_getLogs filter for DepositLocked events from the gateway contract
        let filter = serde_json::json!({
            "address": gateway,
            "topics": [topic],
            "fromBlock": from_block_hex,
            "toBlock": "latest",
        });

        let resp = rpc_call_blocking(rpc_url, "eth_getLogs", serde_json::json!([filter]), 10000)?;
        if let Some(err) = resp.error {
            eprintln!(
                "[relayer] eth_getLogs error on chain {}: {:?}",
                chain_id, err
            );
            return Ok(0);
        }

        let logs = match resp.result {
            serde_json::Value::Array(arr) => arr,
            _ => {
                eprintln!(
                    "[relayer] eth_getLogs unexpected result type on chain {}",
                    chain_id
                );
                return Ok(0);
            }
        };

        let mut processed = 0;
        let mut max_block_seen = from_block;
        for log in &logs {
            let event = match parse_deposit_log(log, chain_id) {
                Some(e) => e,
                None => continue,
            };

            // Track the highest block number seen for checkpoint persistence
            if event.block_number > max_block_seen {
                max_block_seen = event.block_number;
            }

            // Skip already-processed events
            if self.processed_deposits.contains_key(&event.message_id) {
                continue;
            }

            println!(
                "[relayer] New DepositLocked event: chain={}, msg_id=0x{}",
                chain_id,
                hex::encode(event.message_id)
            );

            // Submit proof to X3
            match self.submit_deposit_proof_to_x3(&event) {
                Ok(()) => {
                    self.processed_deposits.insert(event.message_id, true);
                    processed += 1;
                }
                Err(e) => {
                    eprintln!(
                        "[relayer] Failed to submit proof for msg_id 0x{}: {}",
                        hex::encode(event.message_id),
                        e
                    );
                }
            }
        }

        // Persist checkpoint: update last scanned block to avoid re-scanning older blocks
        if max_block_seen > from_block {
            self.last_evm_block.insert(chain_id, max_block_seen);
        }

        Ok(processed)
    }

    /// Poll an external EVM gateway contract for new WithdrawalReleased events.
    /// Returns the number of new withdrawal events successfully processed.
    fn watch_evm_withdrawals(
        &mut self,
        chain_id: u64,
    ) -> Result<usize, Box<dyn std::error::Error + Send + Sync>> {
        let rpc_url = self
            .config
            .evm_rpcs
            .get(&chain_id)
            .ok_or("No RPC configured for chain")?;
        let gateway = self
            .config
            .gateway_addresses
            .get(&chain_id)
            .ok_or("No gateway configured for chain")?;

        let topic = event_topic(WITHDRAWAL_RELEASED_SIGNATURE);

        let from_block = self
            .last_evm_withdrawal_block
            .get(&chain_id)
            .copied()
            .unwrap_or(0);
        let from_block_hex = format!("0x{:x}", from_block);

        let filter = serde_json::json!({
            "address": gateway,
            "topics": [topic],
            "fromBlock": from_block_hex,
            "toBlock": "latest",
        });

        let resp = rpc_call_blocking(rpc_url, "eth_getLogs", serde_json::json!([filter]), 10000)?;
        if let Some(err) = resp.error {
            eprintln!(
                "[relayer] eth_getLogs (withdrawal) error on chain {}: {:?}",
                chain_id, err
            );
            return Ok(0);
        }

        let logs = match resp.result {
            serde_json::Value::Array(arr) => arr,
            _ => {
                eprintln!(
                    "[relayer] eth_getLogs (withdrawal) unexpected result on chain {}",
                    chain_id
                );
                return Ok(0);
            }
        };

        let mut processed = 0;
        let mut max_block_seen = from_block;
        for log in &logs {
            let event = match parse_withdrawal_released_log(log, chain_id) {
                Some(e) => e,
                None => continue,
            };

            if event.block_number > max_block_seen {
                max_block_seen = event.block_number;
            }

            if self
                .processed_withdrawals
                .contains_key(&event.withdrawal_id)
            {
                continue;
            }

            println!(
                "[relayer] New WithdrawalReleased event: chain={}, withdrawal_id=0x{}",
                chain_id,
                hex::encode(event.withdrawal_id)
            );

            // Submit release proof to X3 gateway pallet
            match self.submit_release_proof_to_gateway(&event) {
                Ok(()) => {
                    self.processed_withdrawals.insert(event.withdrawal_id, true);
                    processed += 1;
                }
                Err(e) => {
                    eprintln!(
                        "[relayer] Failed to submit release proof for withdrawal 0x{}: {}",
                        hex::encode(event.withdrawal_id),
                        e
                    );
                }
            }
        }

        if max_block_seen > from_block {
            self.last_evm_withdrawal_block
                .insert(chain_id, max_block_seen);
        }

        Ok(processed)
    }

    /// Poll an SVM (Solana) cluster for new deposit program accounts.
    /// Uses getProgramAccounts to query the HTLC program for owned accounts.
    fn watch_svm_deposits(
        &mut self,
        svm_config: &SvmClusterConfig,
    ) -> Result<usize, Box<dyn std::error::Error + Send + Sync>> {
        // Default HTLC program ID for Solana testnet
        // This can be overridden via env var SVM_HTLC_PROGRAM_ID
        let program_id = std::env::var("SVM_HTLC_PROGRAM_ID")
            .unwrap_or_else(|_| "8uL5NJGjg9oKPAfM5K7xJcJQ5hCQ5pCfQ3oCpByHvKJF".to_string());

        // Build getProgramAccounts filter — accounts owned by the HTLC program
        // with a data size filter (HTLC accounts are typically 200+ bytes)
        let params = serde_json::json!([
            program_id,
            {
                "filters": [
                    {
                        "dataSize": 200
                    }
                ]
            }
        ]);

        let resp = rpc_call_blocking(
            &svm_config.rpc_endpoint,
            "getProgramAccounts",
            params,
            svm_config.slot_poll_interval_ms,
        )?;
        if let Some(err) = resp.error {
            eprintln!(
                "[relayer] getProgramAccounts error on cluster '{}': {:?}",
                svm_config.cluster_name, err
            );
            return Ok(0);
        }

        let accounts = match resp.result {
            serde_json::Value::Array(arr) => arr,
            serde_json::Value::Null => {
                // No accounts returned — this is normal, not an error
                debug!(
                    "No HTLC program accounts found on cluster '{}'",
                    svm_config.cluster_name
                );
                return Ok(0);
            }
            _ => {
                eprintln!(
                    "[relayer] getProgramAccounts unexpected result type on cluster '{}'",
                    svm_config.cluster_name
                );
                return Ok(0);
            }
        };

        let mut processed = 0;
        let mut max_slot_seen = self
            .last_svm_slot
            .get(&svm_config.cluster_name)
            .copied()
            .unwrap_or(0);

        if accounts.is_empty() {
            debug!(
                "Empty accounts list on cluster '{}' (normal if no HTLC deposits exist)",
                svm_config.cluster_name
            );
            return Ok(0);
        }

        for account in &accounts {
            let pubkey = match account.get("pubkey").and_then(|v| v.as_str()) {
                Some(pk) => pk.to_string(),
                None => continue,
            };

            // Build a deterministic message_id from pubkey + slot
            let message_id = sp_core::hashing::blake2_256(pubkey.as_bytes());

            // Skip already-processed accounts
            if self.processed_deposits.contains_key(&message_id) {
                continue;
            }

            // Try to parse account data for deposit info
            let account_data = account
                .get("account")
                .and_then(|a| a.get("data"))
                .and_then(|d| d.as_array())
                .and_then(|arr| arr.first())
                .and_then(|v| v.as_str());

            // Parse slot and amount from account info
            let slot = account
                .get("account")
                .and_then(|a| a.get("slot"))
                .and_then(|v| v.as_u64())
                .unwrap_or(0);

            // Track the highest slot seen for checkpoint persistence
            if slot > max_slot_seen {
                max_slot_seen = slot;
            }

            let _data_bytes = account_data.and_then(hex_to_vec).unwrap_or_default();

            // For simplicity, use a default amount — in production this would
            // be decoded from the account's data layout based on the HTLC program.
            // The SVM HTLC account layout typically stores: [depositor(32), recipient(32), amount(8), asset_id(32)]
            let mut source_address = [0u8; 32];
            let mut destination_address = [0u8; 32];
            let mut asset_id = [0u8; 32];

            if _data_bytes.len() >= 104 {
                source_address.copy_from_slice(&_data_bytes[0..32]);
                destination_address.copy_from_slice(&_data_bytes[32..64]);
                if _data_bytes.len() >= 72 {
                    asset_id.copy_from_slice(&_data_bytes[64..96]);
                }
            }

            // Parse amount from bytes 96..104 (little-endian u64)
            let amount = if _data_bytes.len() >= 104 {
                let mut amt_bytes = [0u8; 8];
                amt_bytes.copy_from_slice(&_data_bytes[96..104]);
                u128::from(u64::from_le_bytes(amt_bytes))
            } else {
                0
            };

            let svm_event = SvmDepositEvent {
                source_address,
                destination_address,
                amount,
                asset_id,
                slot,
                tx_hash: pubkey.clone(),
                program_id: {
                    let mut pid = [0u8; 32];
                    let pk_bytes = hex::decode(pubkey.strip_prefix("0x").unwrap_or(&pubkey))
                        .unwrap_or_default();
                    if pk_bytes.len() == 32 {
                        pid.copy_from_slice(&pk_bytes);
                    }
                    pid
                },
                cluster_name: svm_config.cluster_name.clone(),
            };

            println!(
                "[relayer] New SVM deposit event: cluster='{}', source=0x{}, amount={}",
                svm_config.cluster_name,
                hex::encode(source_address),
                amount
            );

            // Submit proof to X3
            match self.submit_svm_deposit_proof_to_x3(&svm_event) {
                Ok(()) => {
                    self.processed_deposits.insert(message_id, true);
                    processed += 1;
                }
                Err(e) => {
                    eprintln!(
                        "[relayer] Failed to submit SVM proof for account 0x{}: {}",
                        hex::encode(message_id),
                        e
                    );
                }
            }
        }

        // Persist checkpoint: update last scanned slot to avoid re-scanning old accounts
        if max_slot_seen > 0 {
            self.last_svm_slot
                .insert(svm_config.cluster_name.clone(), max_slot_seen);
        }

        Ok(processed)
    }

    /// Submit an SVM deposit proof to X3 via JSON-RPC.
    fn submit_svm_deposit_proof_to_x3(
        &self,
        event: &SvmDepositEvent,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        println!(
            "[relayer] Submitting SVM deposit proof: cluster='{}', amount={}",
            event.cluster_name, event.amount,
        );

        let proof_signer =
            std::env::var("X3_RELAY_PROOF_SIGNER").unwrap_or_else(|_| "//Alice".to_string());

        // Build the CrossVmOperation for SVM deposit — source is 32-byte SVM address
        let operation = CrossVmOperation::TransferToSvm {
            source: [0u8; 20], // placeholder — X3 chain expects 20-byte EVM compat source
            destination: event.destination_address.to_vec(),
            amount: event.amount,
        };
        let lock_proof = build_lock_proof(&operation, &proof_signer)?;

        // Build deposit payload using SVM event fields
        let mut deposit_payload = Vec::new();
        deposit_payload.extend_from_slice(&event.source_address);
        deposit_payload.extend_from_slice(&event.destination_address);
        deposit_payload.extend_from_slice(&event.amount.to_le_bytes());
        deposit_payload.extend_from_slice(&event.asset_id);
        deposit_payload.extend_from_slice(&event.slot.to_le_bytes());
        deposit_payload.extend_from_slice(event.cluster_name.as_bytes());

        let proof_len: u32 = lock_proof
            .len()
            .try_into()
            .map_err(|_| "lock proof too large for SVM deposit relay envelope")?;
        let mut envelope = Vec::with_capacity(
            SIGNED_DEPOSIT_RELAY_MAGIC.len() + 4 + lock_proof.len() + deposit_payload.len(),
        );
        envelope.extend_from_slice(SIGNED_DEPOSIT_RELAY_MAGIC);
        envelope.extend_from_slice(&proof_len.to_le_bytes());
        envelope.extend_from_slice(&lock_proof);
        envelope.extend_from_slice(&deposit_payload);

        let proof_hex = format!("0x{}", hex::encode(&envelope));

        let resp = rpc_call_blocking(
            &self.config.x3_rpc,
            "x3_submitCrossVmTransaction",
            serde_json::json!([proof_hex]),
            30000,
        )?;

        if let Some(err) = resp.error {
            return Err(format!("X3 RPC error: {:?}", err).into());
        }

        println!(
            "[relayer] SVM deposit proof submitted successfully — cluster='{}'",
            event.cluster_name
        );
        Ok(())
    }

    /// Submit a deposit proof to X3 via the gateway pallet.
    ///
    /// Builds a signed deposit relay envelope and submits it via
    /// the `x3_submitCrossVmTransaction` RPC. The runtime verifies
    /// the proof against the route's verifier and creates a GatewayTransfer.
    fn submit_deposit_proof_to_x3(
        &self,
        event: &ExternalDepositEvent,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        println!(
            "[relayer] Submitting deposit proof: chain={}, amount={}, msg_id=0x{}",
            event.chain_id,
            event.amount,
            hex::encode(event.message_id)
        );

        let proof_signer =
            std::env::var("X3_RELAY_PROOF_SIGNER").unwrap_or_else(|_| "//Alice".to_string());
        let proof_envelope = build_signed_deposit_relay_envelope(event, &proof_signer)?;
        let proof_hex = format!("0x{}", hex::encode(&proof_envelope));

        // Submit via x3_submitCrossVmTransaction RPC
        let resp = rpc_call_blocking(
            &self.config.x3_rpc,
            "x3_submitCrossVmTransaction",
            serde_json::json!([proof_hex]),
            30000,
        )?;

        if let Some(err) = resp.error {
            return Err(format!("X3 RPC error: {:?}", err).into());
        }

        println!(
            "[relayer] Deposit proof submitted successfully — msg_id=0x{}",
            hex::encode(event.message_id)
        );
        Ok(())
    }

    /// Submit a release proof to X3's gateway pallet.
    ///
    /// Builds a signed proof payload for the WithdrawalReleased event
    /// and submits it via `x3_submitCrossVmTransaction`.
    fn submit_release_proof_to_gateway(
        &self,
        event: &WithdrawalReleasedEvent,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        println!(
            "[relayer] Submitting release proof: chain={}, withdrawal_id=0x{}",
            event.chain_id,
            hex::encode(event.withdrawal_id)
        );

        // Encode the release proof payload.
        // The runtime's submit_release_proof extrinsic expects the EVM
        // receipt proof payload containing the WithdrawalReleased event data.
        let mut proof_payload = Vec::new();
        proof_payload.extend_from_slice(&event.withdrawal_id);
        proof_payload.extend_from_slice(&event.recipient);
        proof_payload.extend_from_slice(&event.amount.to_le_bytes());
        let proof_hex = format!("0x{}", hex::encode(&proof_payload));

        // Submit via x3_submitCrossVmTransaction RPC
        let resp = rpc_call_blocking(
            &self.config.x3_rpc,
            "x3_submitCrossVmTransaction",
            serde_json::json!([proof_hex]),
            30000,
        )?;

        if let Some(err) = resp.error {
            return Err(format!("X3 RPC error: {:?}", err).into());
        }

        println!(
            "[relayer] Release proof submitted successfully — withdrawal=0x{}",
            hex::encode(event.withdrawal_id)
        );
        Ok(())
    }
}

// ── Log parsing ─────────────────────────────────────────────────────────────

/// Parse a DepositLocked event from an eth_getLogs result entry.
///
/// Topic layout:
///   topics[0] = keccak256(DepositLocked(bytes32,address,address,bytes,uint256,uint256,uint256))
///   topics[1] = bytes32 messageId
///   topics[2] = address token (left-padded to 32 bytes)
///   topics[3] = address depositor (left-padded to 32 bytes)
///   data = ABI-encoded (bytes x3Recipient, uint256 amount, uint256 nonce, uint256 chainId)
fn parse_deposit_log(log: &serde_json::Value, chain_id: u64) -> Option<ExternalDepositEvent> {
    let topics = log.get("topics")?.as_array()?;
    let data_hex = log.get("data")?.as_str()?;

    // topics[1] = messageId (32 bytes)
    let message_id_str = topics.get(1)?.as_str()?;
    let message_id = hex_to_bytes::<32>(message_id_str)?;

    // topics[2] = token address (left-padded to 32 bytes, last 20 carry the address)
    let token_str = topics.get(2)?.as_str()?;
    let token_full = hex_to_vec(token_str)?;
    let token_address: [u8; 20] = token_full[12..32].try_into().ok()?;

    // topics[3] = depositor address (left-padded to 32 bytes, last 20 carry the address)
    let depositor_str = topics.get(3)?.as_str()?;
    let depositor_full = hex_to_vec(depositor_str)?;
    let depositor: [u8; 20] = depositor_full[12..32].try_into().ok()?;

    // data = ABI-encoded: (bytes offset=0x80, amount=u256, nonce=u256, chainId=u256)
    let data_bytes = hex_to_vec(data_hex)?;

    // amount is at bytes 32..64 (second u256 in data, after the "offset" for bytes)
    // Actually the ABI layout for (bytes, uint256, uint256, uint256) is:
    //   [0..32]   = offset to bytes start (usually 0x80)
    //   [32..64]  = amount
    //   [64..96]  = nonce
    //   [96..128] = chainId
    //   [128..]   = bytes length + data
    let amount = data_bytes
        .get(32..64)
        .map(|b| {
            let s = hex::encode(b);
            u128::from_str_radix(&s, 16).unwrap_or(0)
        })
        .unwrap_or(0);

    let nonce = data_bytes
        .get(64..96)
        .map(|b| {
            let s = hex::encode(b);
            u128::from_str_radix(&s, 16).unwrap_or(0)
        })
        .unwrap_or(0);

    // x3Recipient is the `bytes` field in the ABI data
    let recipient_start = 128;
    let recipient_len = data_bytes
        .get(recipient_start..recipient_start + 32)
        .map(|b| {
            let s = hex::encode(b);
            u64::from_str_radix(&s, 16).unwrap_or(0) as usize
        })
        .unwrap_or(0);

    let x3_recipient = if recipient_len > 0 && recipient_len <= 1024 {
        data_bytes
            .get(recipient_start + 32..recipient_start + 32 + recipient_len)
            .map(|b| b.to_vec())
            .unwrap_or_default()
    } else {
        Vec::new()
    };

    let block_number = log
        .get("blockNumber")
        .and_then(|v| v.as_str())
        .map(|s| {
            let s = s.strip_prefix("0x").unwrap_or(s);
            u64::from_str_radix(s, 16).unwrap_or(0)
        })
        .unwrap_or(0);

    let tx_hash = log
        .get("transactionHash")
        .and_then(|v| v.as_str())
        .and_then(hex_to_bytes::<32>)
        .unwrap_or([0u8; 32]);

    Some(ExternalDepositEvent {
        message_id,
        token_address,
        depositor,
        x3_recipient,
        amount,
        nonce,
        chain_id,
        block_number,
        tx_hash,
    })
}

/// Parse a WithdrawalReleased event from an eth_getLogs result entry.
///
/// Topic layout:
///   topics[0] = keccak256(WithdrawalReleased(bytes32,address,address,uint256))
///   topics[1] = bytes32 withdrawalId
///   topics[2] = address recipient (left-padded to 32 bytes)
///   topics[3] = address relayer (left-padded to 32 bytes)
///   data = ABI-encoded (uint256 amount)
#[derive(Debug, Clone)]
struct WithdrawalReleasedEvent {
    withdrawal_id: [u8; 32],
    recipient: [u8; 20],
    amount: u128,
    chain_id: u64,
    block_number: u64,
    _tx_hash: [u8; 32],
}

fn parse_withdrawal_released_log(
    log: &serde_json::Value,
    chain_id: u64,
) -> Option<WithdrawalReleasedEvent> {
    let topics = log.get("topics")?.as_array()?;
    let data_hex = log.get("data")?.as_str()?;

    // topics[1] = withdrawalId (32 bytes)
    let withdrawal_id_str = topics.get(1)?.as_str()?;
    let withdrawal_id = hex_to_bytes::<32>(withdrawal_id_str)?;

    // topics[2] = recipient address (left-padded to 32 bytes)
    let recipient_str = topics.get(2)?.as_str()?;
    let recipient_full = hex_to_vec(recipient_str)?;
    let recipient: [u8; 20] = recipient_full[12..32].try_into().ok()?;

    // data = ABI-encoded (uint256 amount) — the amount is the only data field
    let data_bytes = hex_to_vec(data_hex)?;
    let amount = if data_bytes.len() >= 32 {
        data_bytes
            .get(0..32)
            .map(|b| {
                let s = hex::encode(b);
                u128::from_str_radix(&s, 16).unwrap_or(0)
            })
            .unwrap_or(0)
    } else {
        0
    };

    let block_number = log
        .get("blockNumber")
        .and_then(|v| v.as_str())
        .map(|s| {
            let s = s.strip_prefix("0x").unwrap_or(s);
            u64::from_str_radix(s, 16).unwrap_or(0)
        })
        .unwrap_or(0);

    let tx_hash = log
        .get("transactionHash")
        .and_then(|v| v.as_str())
        .and_then(hex_to_bytes::<32>)
        .unwrap_or([0u8; 32]);

    Some(WithdrawalReleasedEvent {
        withdrawal_id,
        recipient,
        amount,
        chain_id,
        block_number,
        _tx_hash: tx_hash,
    })
}

// ── Entry point ─────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    let config = RelayerConfig::from_env();
    let mut relayer = Relayer::new(config);
    relayer.run().await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_topic_computes_keccak() {
        let topic = event_topic(DEPOSIT_LOCKED_SIGNATURE);
        assert!(topic.starts_with("0x"));
        assert_eq!(topic.len(), 66); // 0x + 64 hex chars
    }

    #[test]
    fn test_parse_deposit_log_roundtrip() {
        // Simulate a DepositLocked event log
        let message_id = [0xabu8; 32];
        let token: [u8; 20] = {
            let mut a = [0u8; 20];
            a[19] = 0xff;
            a
        };
        let depositor: [u8; 20] = {
            let mut a = [0u8; 20];
            a[19] = 0x11;
            a
        };

        // Build ABI-encoded data for a DepositLocked event
        // bytes x3Recipient (len=4, data="abcd"), uint256 amount, uint256 nonce, uint256 chainId
        // offset for bytes = 0x80 (128)
        let mut data = Vec::new();
        // offset for x3Recipient bytes: 0x80
        data.extend_from_slice(&[0u8; 31]);
        data.push(0x80);
        // amount = 1000
        let mut amount_bytes = [0u8; 32];
        amount_bytes[31] = 0x03;
        amount_bytes[30] = 0xe8; // 0x3e8 = 1000
        data.extend_from_slice(&amount_bytes);
        // nonce = 1
        let mut nonce_bytes = [0u8; 32];
        nonce_bytes[31] = 0x01;
        data.extend_from_slice(&nonce_bytes);
        // chainId field (internal to event, not the chain_id param)
        let mut chain_id_bytes = [0u8; 32];
        chain_id_bytes[31] = 0x01;
        data.extend_from_slice(&chain_id_bytes);
        // bytes length for x3Recipient = 4
        let mut len_bytes = [0u8; 32];
        len_bytes[31] = 0x04;
        data.extend_from_slice(&len_bytes);
        // x3Recipient data: "abcd"
        data.extend_from_slice(&[0x61, 0x62, 0x63, 0x64]);
        // pad to 32 bytes
        data.extend_from_slice(&[0u8; 28]);

        let log = serde_json::json!({
            "topics": [
                event_topic(DEPOSIT_LOCKED_SIGNATURE),
                format!("0x{}", hex::encode(message_id)),
                format!("0x000000000000000000000000{}", hex::encode(token)),
                format!("0x000000000000000000000000{}", hex::encode(depositor)),
            ],
            "data": format!("0x{}", hex::encode(&data)),
            "blockNumber": "0x10",
            "transactionHash": format!("0x{}", hex::encode([0x22u8; 32])),
        });

        let event = parse_deposit_log(&log, 1).expect("should parse deposit log");
        assert_eq!(event.message_id, message_id);
        assert_eq!(event.token_address, token);
        assert_eq!(event.depositor, depositor);
        assert_eq!(event.block_number, 16);
        assert_eq!(event.nonce, 1);
        assert_eq!(event.chain_id, 1);
        // x3Recipient should be 4 bytes "abcd"
        assert_eq!(event.x3_recipient, vec![0x61, 0x62, 0x63, 0x64]);
    }

    #[test]
    fn test_config_env_var_expansion() {
        // set up an env var to test expansion
        std::env::set_var("TEST_VAR", "expanded_value");
        let raw = "rpc: ${TEST_VAR}";
        let result = RelayerConfig::expand_env_vars(raw);
        assert_eq!(result, "rpc: expanded_value");
    }

    #[test]
    fn signed_deposit_relay_envelope_contains_verifier_lock_proof() {
        use codec::Encode;
        use sp_core::Pair as _;
        use x3_cross_vm_bridge::CrossVmOperation;

        let event = ExternalDepositEvent {
            message_id: [0x11; 32],
            token_address: [0x22; 20],
            depositor: [0x33; 20],
            x3_recipient: vec![0x44; 32],
            amount: 1_000,
            nonce: 7,
            chain_id: 1_337,
            block_number: 42,
            tx_hash: [0x55; 32],
        };

        let envelope =
            build_signed_deposit_relay_envelope(&event, "//Alice").expect("signed envelope");
        let decoded =
            decode_signed_deposit_relay_envelope(&envelope).expect("decode signed envelope");

        let operation = CrossVmOperation::TransferToSvm {
            source: event.depositor,
            destination: event.x3_recipient.clone(),
            amount: event.amount,
        };
        let expected_hash = sp_core::hashing::blake2_256(&operation.encode());
        assert_eq!(&decoded.lock_proof[0..32], expected_hash.as_slice());
        assert_eq!(decoded.lock_proof[32], 1);

        let alice = sp_core::sr25519::Pair::from_string("//Alice", None).unwrap();
        let alice_public = alice.public();
        let alice_public_bytes: &[u8] = alice_public.as_ref();
        assert_eq!(&decoded.lock_proof[33..65], alice_public_bytes);
        let signature =
            sp_core::sr25519::Signature::from_raw(decoded.lock_proof[65..129].try_into().unwrap());
        assert!(sp_core::sr25519::Pair::verify(
            &signature,
            expected_hash,
            &alice.public()
        ));
        assert_eq!(decoded.deposit_payload, build_deposit_proof_payload(&event));
    }

    #[test]
    fn test_parse_withdrawal_released_log() {
        let withdrawal_id = [0xaa; 32];
        let recipient: [u8; 20] = {
            let mut a = [0u8; 20];
            a[19] = 0xbb;
            a
        };
        let amount = 500u128;

        // ABI-encode amount as uint256 (32 bytes, big-endian)
        let mut data = vec![0u8; 32];
        let amount_bytes = amount.to_be_bytes();
        data[16..32].copy_from_slice(&amount_bytes);

        let log = serde_json::json!({
            "topics": [
                event_topic(WITHDRAWAL_RELEASED_SIGNATURE),
                format!("0x{}", hex::encode(withdrawal_id)),
                format!("0x000000000000000000000000{}", hex::encode(recipient)),
                format!("0x000000000000000000000000{}", hex::encode([0xccu8; 20])),
            ],
            "data": format!("0x{}", hex::encode(&data)),
            "blockNumber": "0x20",
            "transactionHash": format!("0x{}", hex::encode([0xddu8; 32])),
        });

        let event = parse_withdrawal_released_log(&log, 1).expect("should parse withdrawal log");
        assert_eq!(event.withdrawal_id, withdrawal_id);
        assert_eq!(event.recipient, recipient);
        assert_eq!(event.amount, 500);
        assert_eq!(event.block_number, 32);
        assert_eq!(event.chain_id, 1);
    }
}
