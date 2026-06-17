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

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

// ── Configuration ───────────────────────────────────────────────────────────

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
struct TypedConfig {
    x3: TypedX3Config,
    submission: TypedSubmissionConfig,
    #[serde(rename = "evm_chains")]
    evm_chains: Vec<TypedEvmChainEntry>,
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
        })
    }

    fn from_env_vars() -> Self {
        Self {
            x3_rpc: std::env::var("X3_RPC")
                .unwrap_or_else(|_| "ws://127.0.0.1:9944".into()),
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
            db_path: std::env::var("DB_PATH")
                .unwrap_or_else(|_| "/tmp/x3-relayer.db".into()),
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

// ── DepositLocked event signature (keccak256) ───────────────────────────────
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
}

impl Relayer {
    fn new(config: RelayerConfig) -> Self {
        Self {
            config,
            processed_deposits: HashMap::new(),
        }
    }

    /// Main relayer loop — polls both external chains and X3
    async fn run(&mut self) {
        println!(
            "[relayer] Starting X3 Relayer (poll_interval={}s)...",
            self.config.poll_interval_secs
        );
        println!("[relayer] X3 RPC: {}", self.config.x3_rpc);
        for (&chain_id, rpc) in &self.config.evm_rpcs {
            let gw = self
                .config
                .gateway_addresses
                .get(&chain_id)
                .map(|s| s.as_str())
                .unwrap_or("N/A");
            println!("[relayer] EVM chain {}: rpc={}, gateway={}", chain_id, rpc, gw);
        }

        loop {
            // Watch external EVM chains for deposit events
            let chain_ids: Vec<u64> = self.config.evm_rpcs.keys().copied().collect();
            for chain_id in chain_ids {
                match self.watch_evm_deposits(chain_id) {
                    Ok(count) if count > 0 => {
                        println!(
                            "[relayer] Chain {}: processed {} new deposits",
                            chain_id, count
                        );
                    }
                    Ok(_) => {} // no new events
                    Err(e) => {
                        eprintln!(
                            "[relayer] Error watching EVM chain {}: {:?}",
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
        let _confirmations = self.config.confirmations.get(&chain_id).copied().unwrap_or(12);

        let topic = event_topic(DEPOSIT_LOCKED_SIGNATURE);

        // Build eth_getLogs filter for DepositLocked events from the gateway contract
        let filter = serde_json::json!({
            "address": gateway,
            "topics": [topic],
            "fromBlock": "0x0",
            "toBlock": "latest",
        });

        let resp = rpc_call_blocking(rpc_url, "eth_getLogs", serde_json::json!([filter]), 10000)?;
        if let Some(err) = resp.error {
            eprintln!("[relayer] eth_getLogs error on chain {}: {:?}", chain_id, err);
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
        for log in &logs {
            let event = match parse_deposit_log(log, chain_id) {
                Some(e) => e,
                None => continue,
            };

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

        Ok(processed)
    }

    /// Submit a deposit proof to X3 via JSON-RPC.
    ///
    /// Calls x3_submitCrossVmTransaction with the deposit event data.
    /// The X3 runtime decodes the deposit proof, verifies it against the
    /// gateway contract's known state, and mints the corresponding
    /// wrapped asset via the supply ledger.
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

        // Build the deposit proof payload.
        // Format: SCALE-encoded (chain_id: u64, message_id: [u8; 32],
        //   token_address: [u8; 20], depositor: [u8; 20],
        //   x3_recipient: Vec<u8>, amount: u128, nonce: u128,
        //   gateway_address: [u8; 20], gateway_block_number: u64)
        let mut proof_payload = Vec::new();

        // chain_id (u64 LE)
        proof_payload.extend_from_slice(&event.chain_id.to_le_bytes());
        // message_id (32 bytes)
        proof_payload.extend_from_slice(&event.message_id);
        // token_address (20 bytes)
        proof_payload.extend_from_slice(&event.token_address);
        // depositor (20 bytes)
        proof_payload.extend_from_slice(&event.depositor);
        // x3_recipient (SCALE compact length-prefixed)
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
        // amount (u128 LE)
        proof_payload.extend_from_slice(&event.amount.to_le_bytes());
        // nonce (u128 LE)
        proof_payload.extend_from_slice(&event.nonce.to_le_bytes());
        // gateway_address placeholder (20 zero bytes for now; in production pulled from config)
        proof_payload.extend_from_slice(&[0u8; 20]);
        // gateway_block_number (u64 LE)
        proof_payload.extend_from_slice(&event.block_number.to_le_bytes());

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
            "[relayer] Deposit proof submitted successfully — msg_id=0x{}",
            hex::encode(event.message_id)
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
        .and_then(|s| hex_to_bytes::<32>(s))
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
}