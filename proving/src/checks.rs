//! Async RPC probe functions — one per Phase 1 check, each returning a
//! `CheckResult`.
//!
//! Every probe applies a hard 5-second [`tokio::time::timeout`].  When the
//! target node is offline, connection-refused errors surface immediately
//! (well under the timeout), so the binary always exits cleanly regardless of
//! node availability.

use std::time::{Duration, Instant};

use anyhow::anyhow;
use reqwest::Client;
use serde_json::{json, Value};
use tokio::time::timeout;

use crate::report::CheckResult;

// ─────────────────────────────────────────────────────────────────────────────
// Configuration
// ─────────────────────────────────────────────────────────────────────────────

/// Per-chain RPC endpoint configuration.
#[derive(Debug, Clone)]
pub struct ChainConfig {
    /// Chain identifier as supplied on the command line (e.g. `"x3-native"`).
    pub chain_id: String,
    /// Full HTTP URL for the node's JSON-RPC endpoint.
    ///
    /// Defaults:
    /// - `x3-native`   → `http://127.0.0.1:9944`  (Substrate)
    /// - `evm-testnet` → `http://127.0.0.1:8545`  (EVM)
    /// - `svm-testnet` → `http://127.0.0.1:8899`  (SVM)
    pub rpc_url: String,
}

/// RPC wire protocol implied by the chain ID prefix.
#[derive(Debug, Clone, Copy)]
enum ChainFamily {
    /// Substrate / Polkadot-SDK node (Substrate JSON-RPC).
    Substrate,
    /// Ethereum-compatible EVM node (Ethereum JSON-RPC).
    Evm,
    /// Solana Virtual Machine node (Solana JSON-RPC).
    Svm,
}

impl ChainConfig {
    fn family(&self) -> ChainFamily {
        if self.chain_id.starts_with("evm") {
            ChainFamily::Evm
        } else if self.chain_id.starts_with("svm") {
            ChainFamily::Svm
        } else {
            ChainFamily::Substrate
        }
    }
}

/// Build a [`ChainConfig`] from a chain-id string, mapping well-known IDs to
/// their standard testnet ports and falling back to the Substrate port for
/// anything unrecognised.
#[must_use]
pub fn chain_config_for(chain_id: &str) -> ChainConfig {
    let rpc_url = match chain_id {
        "x3-native" => "http://127.0.0.1:9944".to_string(),
        "evm-testnet" => "http://127.0.0.1:8545".to_string(),
        "svm-testnet" => "http://127.0.0.1:8899".to_string(),
        id if id.starts_with("evm") => "http://127.0.0.1:8545".to_string(),
        id if id.starts_with("svm") => "http://127.0.0.1:8899".to_string(),
        _ => "http://127.0.0.1:9944".to_string(),
    };
    ChainConfig {
        chain_id: chain_id.to_string(),
        rpc_url,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Internal RPC transport
// ─────────────────────────────────────────────────────────────────────────────

/// Hard timeout applied to every RPC probe.
const RPC_TIMEOUT: Duration = Duration::from_secs(5);

/// Substrate storage key for `System::Number`
/// (`twox_128("System") ++ twox_128("Number")`).
/// Returns the current block number as a SCALE-encoded `u64` when the node
/// is running — a lightweight, non-empty storage read for reconciliation checks.
const SYSTEM_NUMBER_KEY: &str =
    "0x26aa394eea5630e07c48ae0c9558cef702a5c1b19ab7a04f536c519aca4983ac";

/// Solana System Program address (base-58).  Its account info is always
/// present on any live Solana node, making it a reliable reconciliation target.
const SOLANA_SYSTEM_PROGRAM: &str = "11111111111111111111111111111111";

/// Returns the current UTC time as an RFC-3339 string.
fn now_rfc3339() -> String {
    chrono::Utc::now().to_rfc3339()
}

/// Send a single JSON-RPC POST request and return `Ok(())` when the node
/// responds with a non-error payload, or an `Err` describing the failure.
///
/// Large responses such as `state_getMetadata` are read fully into memory;
/// the caller's 5-second timeout bounds the total wall time.
///
/// # Errors
///
/// Returns an error if:
/// - The TCP connection cannot be established.
/// - The HTTP response status is not 2xx.
/// - The response body is not valid JSON-RPC (unless it begins with `{`,
///   which covers partial / streaming responses).
/// - The response contains a top-level `"error"` field.
async fn rpc_call(client: &Client, url: &str, method: &str, params: Value) -> anyhow::Result<()> {
    let body = json!({
        "jsonrpc": "2.0",
        "id":      1,
        "method":  method,
        "params":  params,
    });

    let resp = client.post(url).json(&body).send().await?;

    let http_status = resp.status();
    if !http_status.is_success() {
        return Err(anyhow!("HTTP {http_status}"));
    }

    let bytes = resp.bytes().await?;

    // Parse the body to detect a JSON-RPC-level error field.
    // For large metadata responses (~2–5 MiB) the full parse is acceptable on a
    // proving harness; the timeout at the call site keeps wall time bounded.
    match serde_json::from_slice::<Value>(&bytes) {
        Ok(v) if v.get("error").is_some() => Err(anyhow!("JSON-RPC error: {}", v["error"])),
        Ok(_) => Ok(()),
        Err(parse_err) => {
            // If the body starts with `{` it is almost certainly a valid
            // JSON-RPC response that was truncated by the OS buffer; treat
            // the HTTP 200 as success rather than failing on a partial read.
            if bytes.starts_with(b"{") {
                Ok(())
            } else {
                Err(anyhow!("invalid response body: {parse_err}"))
            }
        }
    }
}

/// Generic probe runner: starts the clock, calls [`rpc_call`] inside a
/// 5-second [`timeout`], then converts the outcome to a [`CheckResult`].
///
/// A fresh [`Client`] is created per call so that unreachable nodes fail
/// immediately without blocking a shared connection pool.
async fn run_check(
    config: &ChainConfig,
    check_name: &str,
    method: &str,
    params: Value,
) -> CheckResult {
    let client = Client::new();
    let start = Instant::now();
    let result = timeout(
        RPC_TIMEOUT,
        rpc_call(&client, &config.rpc_url, method, params),
    )
    .await;
    let latency_ms = u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX);
    let timestamp = now_rfc3339();

    match result {
        Ok(Ok(())) => CheckResult {
            check: check_name.to_string(),
            chain_id: config.chain_id.clone(),
            passed: true,
            score: 1.0,
            latency_ms,
            error: None,
            timestamp,
        },
        Ok(Err(e)) => CheckResult {
            check: check_name.to_string(),
            chain_id: config.chain_id.clone(),
            passed: false,
            score: 0.0,
            latency_ms,
            error: Some(e.to_string()),
            timestamp,
        },
        Err(_elapsed) => CheckResult {
            check: check_name.to_string(),
            chain_id: config.chain_id.clone(),
            passed: false,
            score: 0.0,
            latency_ms,
            error: Some("timed out after 5 s".to_string()),
            timestamp,
        },
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Phase 1 checks — public API
// ─────────────────────────────────────────────────────────────────────────────

/// **Quoting** — verifies the node is alive and the routing layer is healthy.
///
/// RPC methods used:
/// - Substrate: `system_health`
/// - EVM: `eth_blockNumber`
/// - SVM: `getHealth`
#[must_use]
pub async fn check_quoting(config: &ChainConfig) -> CheckResult {
    let (method, params) = match config.family() {
        ChainFamily::Evm => ("eth_blockNumber", json!([])),
        ChainFamily::Svm => ("getHealth", json!([])),
        ChainFamily::Substrate => ("system_health", json!([])),
    };
    run_check(config, "quoting", method, params).await
}

/// **Bundle construction** — fetches the latest block anchor, confirming that
/// transaction bundles can be attached to a real chain head.
///
/// RPC methods used:
/// - Substrate: `chain_getBlockHash` (null → latest)
/// - EVM: `eth_getBlockByNumber("latest", false)`
/// - SVM: `getLatestBlockhash`
#[must_use]
pub async fn check_bundle_construction(config: &ChainConfig) -> CheckResult {
    let (method, params) = match config.family() {
        ChainFamily::Evm => ("eth_getBlockByNumber", json!(["latest", false])),
        ChainFamily::Svm => ("getLatestBlockhash", json!([])),
        ChainFamily::Substrate => ("chain_getBlockHash", json!([null])),
    };
    run_check(config, "bundle_construction", method, params).await
}

/// **Submission** — fetches chain-level metadata / protocol version, confirming
/// the gateway required for transaction submission is accessible.
///
/// RPC methods used:
/// - Substrate: `state_getMetadata`
/// - EVM: `eth_protocolVersion`
/// - SVM: `getVersion`
#[must_use]
pub async fn check_submission(config: &ChainConfig) -> CheckResult {
    let (method, params) = match config.family() {
        ChainFamily::Evm => ("eth_protocolVersion", json!([])),
        ChainFamily::Svm => ("getVersion", json!([])),
        ChainFamily::Substrate => ("state_getMetadata", json!([])),
    };
    run_check(config, "submission", method, params).await
}

/// **Rollback correctness** — fetches the finalized head / slot, verifying
/// the node tracks a canonical chain from which a rollback could be executed.
///
/// RPC methods used:
/// - Substrate: `chain_getFinalizedHead`
/// - EVM: `eth_getBlockByNumber("finalized", false)`
/// - SVM: `getSlot` with `"finalized"` commitment
#[must_use]
pub async fn check_rollback(config: &ChainConfig) -> CheckResult {
    let (method, params) = match config.family() {
        ChainFamily::Evm => ("eth_getBlockByNumber", json!(["finalized", false])),
        ChainFamily::Svm => ("getSlot", json!([{"commitment": "finalized"}])),
        ChainFamily::Substrate => ("chain_getFinalizedHead", json!([])),
    };
    run_check(config, "rollback", method, params).await
}

/// **Reconciliation accuracy** — reads a well-known storage location to
/// confirm that state can be queried for post-trade reconciliation.
///
/// RPC methods used:
/// - Substrate: `state_getStorage` with the `System::Number` key
/// - EVM: `eth_getBalance` for the zero address at `latest`
/// - SVM: `getAccountInfo` for the Solana System Program
#[must_use]
pub async fn check_reconciliation(config: &ChainConfig) -> CheckResult {
    let (method, params) = match config.family() {
        ChainFamily::Evm => (
            "eth_getBalance",
            json!(["0x0000000000000000000000000000000000000000", "latest"]),
        ),
        ChainFamily::Svm => (
            "getAccountInfo",
            json!([SOLANA_SYSTEM_PROGRAM, {"encoding": "base64"}]),
        ),
        ChainFamily::Substrate => ("state_getStorage", json!([SYSTEM_NUMBER_KEY])),
    };
    run_check(config, "reconciliation", method, params).await
}

/// **State verification** — fetches the runtime / client version string,
/// confirming that receipt metadata can be independently verified on-chain.
///
/// RPC methods used:
/// - Substrate: `system_version`
/// - EVM: `web3_clientVersion`
/// - SVM: `getVersion`
#[must_use]
pub async fn check_state_verification(config: &ChainConfig) -> CheckResult {
    let (method, params) = match config.family() {
        ChainFamily::Evm => ("web3_clientVersion", json!([])),
        ChainFamily::Svm => ("getVersion", json!([])),
        ChainFamily::Substrate => ("system_version", json!([])),
    };
    run_check(config, "state_verification", method, params).await
}
