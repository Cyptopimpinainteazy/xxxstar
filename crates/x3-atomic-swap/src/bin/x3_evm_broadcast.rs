//! Real Anvil (or any live EVM RPC) HTLC lifecycle broadcaster.
//!
//! Mirrors `x3-svm-broadcast`: this binary drives the `LiveEvmExecutor`
//! against a genuine EVM JSON-RPC endpoint (e.g. a local `anvil` instance) to
//! submit real, signed `createHTLC` / `claimHTLC` / `refundHTLC` transactions
//! to a deployed `AtlasHTLC` contract and print the resulting on-chain
//! transaction hash. Never fabricates a tx hash or receipt.
//!
//! Usage:
//!   x3-evm-broadcast --rpc <url> --chain-id <id> --contract <0x..> \
//!       --signer-key <hex> lock --recipient <0x..> --hashlock <hex32> \
//!       --timelock <unix_ts> --amount <wei>
//!   x3-evm-broadcast ... claim --id <hex32> --secret <hex32>
//!   x3-evm-broadcast ... refund --id <hex32>

use std::process::ExitCode;
use x3_atomic_swap::evm_live::LiveEvmExecutor;

fn pick(key: &str, a: &[String]) -> Result<String, String> {
    a.iter()
        .position(|x| x == key)
        .and_then(|i| a.get(i + 1))
        .cloned()
        .ok_or_else(|| format!("missing {key}"))
}

fn parse_hex32(name: &str, s: &str) -> Result<[u8; 32], String> {
    let s = s.strip_prefix("0x").unwrap_or(s);
    let bytes = hex::decode(s).map_err(|e| format!("{name}: bad hex ({e})"))?;
    let mut out = [0u8; 32];
    if bytes.len() != 32 {
        return Err(format!("{name}: must be 32 bytes, got {}", bytes.len()));
    }
    out.copy_from_slice(&bytes);
    Ok(out)
}

fn parse_hex20(name: &str, s: &str) -> Result<[u8; 20], String> {
    let s = s.strip_prefix("0x").unwrap_or(s);
    let bytes = hex::decode(s).map_err(|e| format!("{name}: bad hex ({e})"))?;
    let mut out = [0u8; 20];
    if bytes.len() != 20 {
        return Err(format!("{name}: must be 20 bytes, got {}", bytes.len()));
    }
    out.copy_from_slice(&bytes);
    Ok(out)
}

/// Fetch the `HTLCCreated(bytes32 indexed id, ...)` event's `id` topic from a
/// mined transaction's receipt via a raw JSON-RPC call. The id is the
/// contract's own keccak256 derivation and is not independently
/// recomputable client-side without duplicating internal contract state
/// (htlcCount), so this is the only honest way to retrieve it.
fn fetch_htlc_created_id(rpc_url: &str, tx_hash: &str) -> Result<[u8; 32], String> {
    use sha3::{Digest, Keccak256};
    let topic0 = format!(
        "0x{}",
        hex::encode(Keccak256::digest(
            b"HTLCCreated(bytes32,address,address,address,uint256,bytes32,uint256)"
        ))
    );
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "eth_getTransactionReceipt",
        "params": [tx_hash],
    });
    let response_body = ureq::post(rpc_url)
        .set("Content-Type", "application/json")
        .send_string(&body.to_string())
        .map_err(|e| format!("eth_getTransactionReceipt failed: {e}"))?
        .into_string()
        .map_err(|e| format!("failed to read response body: {e}"))?;
    let response: serde_json::Value =
        serde_json::from_str(&response_body).map_err(|e| format!("bad JSON response: {e}"))?;
    let logs = response["result"]["logs"]
        .as_array()
        .ok_or_else(|| "receipt has no logs".to_string())?;
    for log in logs {
        if let Some(topics) = log["topics"].as_array() {
            if topics.first().and_then(|v| v.as_str()) == Some(topic0.as_str()) {
                let id_hex = topics
                    .get(1)
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "HTLCCreated log missing id topic".to_string())?;
                return parse_hex32("htlc_id", id_hex);
            }
        }
    }
    Err("no HTLCCreated event found in receipt".to_string())
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let json = args.iter().any(|arg| arg == "--json");

    let run = || -> Result<serde_json::Value, String> {
        let rpc = pick("--rpc", &args)?;
        let chain_id: u64 = pick("--chain-id", &args)?
            .parse()
            .map_err(|e| format!("--chain-id: {e}"))?;
        let contract = parse_hex20("--contract", &pick("--contract", &args)?)?;
        let signer_key = pick("--signer-key", &args)?;

        let mut exec = LiveEvmExecutor::new(&rpc, chain_id, contract, &signer_key)
            .map_err(|e| format!("executor init failed: {e}"))?;

        let action = args
            .iter()
            .find(|a| matches!(a.as_str(), "lock" | "claim" | "refund"))
            .cloned()
            .ok_or_else(|| "missing action (lock|claim|refund)".to_string())?;

        match action.as_str() {
            "lock" => {
                let recipient = parse_hex20("--recipient", &pick("--recipient", &args)?)?;
                let hashlock = parse_hex32("--hashlock", &pick("--hashlock", &args)?)?;
                let timelock: u64 = pick("--timelock", &args)?
                    .parse()
                    .map_err(|e| format!("--timelock: {e}"))?;
                let amount: u128 = pick("--amount", &args)?
                    .parse()
                    .map_err(|e| format!("--amount: {e}"))?;
                let timeout_ms: u64 = pick("--timeout-ms", &args)
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(60_000);

                let proof = exec
                    .execute_lock(
                        "evm-live",
                        recipient,
                        hashlock,
                        timelock,
                        [0u8; 20], // native ETH
                        amount,
                        timeout_ms,
                    )
                    .map_err(|e| format!("lock failed: {e}"))?;
                // AtlasHTLC derives `id` inside the contract
                // (keccak256(sender,recipient,hashLock,htlcCount)); the only
                // authoritative source for it is the mined HTLCCreated event,
                // so we independently fetch the receipt via raw RPC rather
                // than fabricating/recomputing it client-side.
                let htlc_id = fetch_htlc_created_id(&rpc, &proof.tx_id)?;
                Ok(serde_json::json!({
                    "action": "lock",
                    "status": "submitted",
                    "tx_hash": proof.tx_id,
                    "block_number": proof.block_number,
                    "block_hash": proof.block_hash,
                    "contract": format!("0x{}", hex::encode(contract)),
                    "htlc_id": format!("0x{}", hex::encode(htlc_id)),
                }))
            }
            "claim" => {
                let id = parse_hex32("--id", &pick("--id", &args)?)?;
                let secret = parse_hex32("--secret", &pick("--secret", &args)?)?;
                let timeout_ms: u64 = pick("--timeout-ms", &args)
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(60_000);
                let proof = exec
                    .execute_claim("evm-live", id, 0, secret, timeout_ms)
                    .map_err(|e| format!("claim failed: {e}"))?;
                Ok(serde_json::json!({
                    "action": "claim",
                    "status": "submitted",
                    "tx_hash": proof.tx_id,
                    "block_number": proof.block_number,
                    "block_hash": proof.block_hash,
                }))
            }
            "refund" => {
                let id = parse_hex32("--id", &pick("--id", &args)?)?;
                let timeout_ms: u64 = pick("--timeout-ms", &args)
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(60_000);
                let proof = exec
                    .execute_refund("evm-live", id, 0, timeout_ms)
                    .map_err(|e| format!("refund failed: {e}"))?;
                Ok(serde_json::json!({
                    "action": "refund",
                    "status": "submitted",
                    "tx_hash": proof.tx_id,
                    "block_number": proof.block_number,
                    "block_hash": proof.block_hash,
                }))
            }
            other => Err(format!("unknown action {other}")),
        }
    };

    match run() {
        Ok(value) => {
            if json {
                println!("{value}");
            } else {
                println!(
                    "{}_SUBMITTED tx_hash={} block={}",
                    value["action"].as_str().unwrap_or("?").to_ascii_uppercase(),
                    value["tx_hash"].as_str().unwrap_or("?"),
                    value["block_number"]
                );
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            if json {
                println!("{}", serde_json::json!({"status": "error", "error": e}));
            } else {
                eprintln!("error: {e}");
            }
            ExitCode::FAILURE
        }
    }
}
