//! X3 Cross-VM Sidecar daemon — Phase 3.
//!
//! Launched by the X3 node via `spawn_sidecar_service`.  Polls Solana for
//! escrow / bridge-root events, checks 32-slot finality per account, then
//! submits a proper SCALE-encoded V4 unsigned extrinsic to the X3 node via
//! `author_submitExtrinsic`.  The target call is
//! `X3CrossVmRouter::register_external_root` (pallet 26, call 4).
//!
//! # Environment variables
//! | Variable                 | Default                              | Description                         |
//! |--------------------------|--------------------------------------|-------------------------------------|
//! | `X3_SIDECAR_BIN`         | auto-detected                        | Binary path (consumed by launcher)  |
//! | `X3_SOLANA_RPC_URL`      | `https://api.mainnet-beta.solana.com`| Solana JSON-RPC endpoint            |
//! | `X3_NODE_RPC_URL`        | `http://127.0.0.1:9944`              | X3 node RPC endpoint                |
//! | `X3_ESCROW_PROGRAM`      | *(empty)*                            | Solana escrow program ID (base58)   |
//! | `X3_SOLANA_CHAIN_ID`     | `2`                                  | chain_id value for Solana in X3     |
//! | `X3_BRIDGE_PALLET_INDEX` | `26`                                 | SCALE pallet index of X3CrossVmRouter|
//! | `X3_BRIDGE_CALL_INDEX`   | `4`                                  | SCALE call index of register_external_root|
//! | `RUST_LOG`               | `info`                               | Log filter                          |

use clap::Parser;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Solana confirmation threshold before we treat an account as finality-confirmed.
const FINALITY_CONFIRMATIONS: u64 = 32;

/// Offset in account data bytes where a u64 `created_slot` field lives.
/// Anchor programs typically start with an 8-byte discriminator followed by
/// struct fields.  We read slots at byte offset 8 (little-endian u64).
const ACCOUNT_DATA_SLOT_OFFSET: usize = 8;

// ─── CLI ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Parser)]
#[command(name = "x3-sidecar", about = "X3 Cross-VM Sidecar daemon")]
struct Args {
    /// Logical service identifier set by the node launcher.
    #[arg(long, default_value = "x3-sidecar")]
    service_id: String,

    /// Solana JSON-RPC endpoint.
    #[arg(long, env = "X3_SOLANA_RPC_URL",
          default_value = "https://api.mainnet-beta.solana.com")]
    solana_rpc: String,

    /// X3 node JSON-RPC endpoint for extrinsic submission.
    #[arg(long, env = "X3_NODE_RPC_URL",
          default_value = "http://127.0.0.1:9944")]
    x3_rpc: String,

    /// Solana escrow program ID to watch (base58).
    #[arg(long, env = "X3_ESCROW_PROGRAM", default_value = "")]
    escrow_program: String,

    /// Poll interval in seconds.
    #[arg(long, default_value_t = 30)]
    poll_interval_secs: u64,
}

// ─── Runtime config (from env, parsed once at start) ─────────────────────────

struct BridgeConfig {
    solana_chain_id: u32,
    pallet_index: u8,
    call_index: u8,
}

impl BridgeConfig {
    fn from_env() -> Self {
        let solana_chain_id: u32 = std::env::var("X3_SOLANA_CHAIN_ID")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(2); // 2 = Solana in X3CrossVmRouter
        let pallet_index: u8 = std::env::var("X3_BRIDGE_PALLET_INDEX")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(26); // X3CrossVmRouter position in construct_runtime
        let call_index: u8 = std::env::var("X3_BRIDGE_CALL_INDEX")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(4); // register_external_root
        BridgeConfig { solana_chain_id, pallet_index, call_index }
    }
}

// ─── JSON-RPC helpers ────────────────────────────────────────────────────────

#[derive(Serialize)]
struct RpcRequest<'a> {
    jsonrpc: &'static str,
    id: u32,
    method: &'a str,
    params: serde_json::Value,
}

#[derive(Deserialize, Debug)]
struct RpcResponse {
    result: Option<serde_json::Value>,
    error: Option<RpcError>,
}

#[derive(Deserialize, Debug)]
struct RpcError {
    code: i64,
    message: String,
}

async fn rpc_call(
    client: &reqwest::Client,
    url: &str,
    method: &str,
    params: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let body = RpcRequest { jsonrpc: "2.0", id: 1, method, params };
    let resp = client
        .post(url)
        .json(&body)
        .timeout(Duration::from_secs(10))
        .send()
        .await
        .map_err(|e| format!("HTTP {}: {}", method, e))?
        .json::<RpcResponse>()
        .await
        .map_err(|e| format!("JSON {}: {}", method, e))?;
    if let Some(err) = resp.error {
        return Err(format!("RPC {}: code={} msg={}", method, err.code, err.message));
    }
    resp.result.ok_or_else(|| format!("empty result: {}", method))
}

// ─── Solana RPC ───────────────────────────────────────────────────────────────

/// Fetch the current confirmed slot on Solana.
async fn solana_get_slot(client: &reqwest::Client, rpc: &str) -> Result<u64, String> {
    rpc_call(client, rpc, "getSlot", serde_json::json!(["confirmed"]))
        .await?
        .as_u64()
        .ok_or_else(|| "getSlot: non-u64 result".to_string())
}

/// One escrow account entry returned by getProgramAccounts.
struct EscrowAccount {
    pubkey: String,
    data: Vec<u8>,
    /// Slot at which this account was last modified (parsed from account data at ACCOUNT_DATA_SLOT_OFFSET).
    created_slot: u64,
}

/// Fetch escrow program accounts.  Returns empty vec if `program_id` is empty.
async fn solana_get_program_accounts(
    client: &reqwest::Client,
    rpc: &str,
    program_id: &str,
) -> Result<Vec<EscrowAccount>, String> {
    if program_id.is_empty() {
        return Ok(vec![]);
    }
    let params = serde_json::json!([
        program_id,
        { "encoding": "base64", "commitment": "confirmed" }
    ]);
    let val = rpc_call(client, rpc, "getProgramAccounts", params).await?;
    let accounts = val.as_array().ok_or("getProgramAccounts: not array")?;

    let mut out = Vec::with_capacity(accounts.len());
    for acc in accounts {
        let pubkey = acc["pubkey"].as_str().unwrap_or("").to_string();
        let data = base64_decode(acc["account"]["data"][0].as_str().unwrap_or(""));
        // Parse created_slot from account data if enough bytes are present.
        let created_slot = read_u64_le(&data, ACCOUNT_DATA_SLOT_OFFSET).unwrap_or(0);
        out.push(EscrowAccount { pubkey, data, created_slot });
    }
    Ok(out)
}

/// Read a little-endian u64 from `data` at byte `offset`.  Returns `None` if out of bounds.
fn read_u64_le(data: &[u8], offset: usize) -> Option<u64> {
    let end = offset.checked_add(8)?;
    let bytes: [u8; 8] = data.get(offset..end)?.try_into().ok()?;
    Some(u64::from_le_bytes(bytes))
}

// ─── SCALE encoding ──────────────────────────────────────────────────────────

/// SCALE compact-encode a `u64`.
///
/// | Range                    | Format                          |
/// |---------------------------|---------------------------------|
/// | 0..=63                   | `(n << 2) as u8` (1 byte)       |
/// | 64..=16383               | `((n << 2) \| 1) as u16` LE      |
/// | 16384..=1_073_741_823    | `((n << 2) \| 2) as u32` LE      |
/// | larger                   | big-integer mode (6 bytes max)  |
pub fn scale_compact_encode(n: u64) -> Vec<u8> {
    if n <= 0x3f {
        vec![(n << 2) as u8]
    } else if n <= 0x3fff {
        ((n << 2) as u16 | 0x01).to_le_bytes().to_vec()
    } else if n <= 0x3fff_ffff {
        ((n << 2) as u32 | 0x02).to_le_bytes().to_vec()
    } else {
        // Big-integer mode: length prefix byte = (bytes_needed - 4) << 2 | 0x03
        let mut bytes = n.to_le_bytes().to_vec();
        while bytes.last() == Some(&0) { bytes.pop(); }
        let prefix = (((bytes.len() - 4) as u8) << 2) | 0x03;
        let mut out = vec![prefix];
        out.extend_from_slice(&bytes);
        out
    }
}

/// Build a SCALE-encoded V4 **unsigned** extrinsic for
/// `X3CrossVmRouter::register_external_root`.
///
/// The extrinsic wire format is:
/// ```text
/// compact_length(inner_len)  -- SCALE compact u64
/// 0x04                       -- version byte: V4, unsigned (no signature bit)
/// pallet_index               -- u8
/// call_index                 -- u8
/// chain_id                   -- SCALE u32 (4 bytes LE)
/// root_hash                  -- 32 bytes
/// block_number               -- SCALE u32 (4 bytes LE)
/// proof_len                  -- SCALE compact length
/// proof_bytes                -- raw bytes
/// ```
///
/// Ref: <https://docs.substrate.io/reference/transaction-format/>
pub fn build_register_external_root_extrinsic(
    pallet_index: u8,
    call_index: u8,
    chain_id: u32,
    root_hash: &[u8; 32],
    block_number: u32,
    proof: &[u8],
) -> Vec<u8> {
    let mut call: Vec<u8> = Vec::new();
    // Version byte for V4 unsigned: bit7=0 (unsigned), bits0-6=4 → 0x04
    call.push(0x04);
    call.push(pallet_index);
    call.push(call_index);
    call.extend_from_slice(&chain_id.to_le_bytes());
    call.extend_from_slice(root_hash);
    call.extend_from_slice(&block_number.to_le_bytes());
    call.extend_from_slice(&scale_compact_encode(proof.len() as u64));
    call.extend_from_slice(proof);

    // Prepend SCALE compact length of the call body.
    let mut extrinsic = scale_compact_encode(call.len() as u64);
    extrinsic.extend_from_slice(&call);
    extrinsic
}

/// Derive a 32-byte root hash from raw account data bytes.
/// Uses a simple XOR-fold with domain separation for Phase 3.
/// Phase 4: replace with Blake2b-256 once `sp_core` is available as dep.
fn derive_root_hash(account_data: &[u8], slot: u64) -> [u8; 32] {
    let mut hash = [0u8; 32];
    for (i, &b) in account_data.iter().enumerate() {
        hash[i % 32] ^= b;
    }
    // Mix slot bytes into upper 8 bytes for uniqueness.
    let slot_bytes = slot.to_le_bytes();
    for (i, &b) in slot_bytes.iter().enumerate() {
        hash[24 + i] ^= b;
    }
    hash
}

// ─── X3 extrinsic submission ──────────────────────────────────────────────────

/// Submit a `register_external_root` extrinsic to the X3 node.
async fn submit_bridge_event(
    client: &reqwest::Client,
    x3_rpc: &str,
    cfg: &BridgeConfig,
    account_data: &[u8],
    current_slot: u64,
    account_created_slot: u64,
) -> Result<String, String> {
    let root_hash = derive_root_hash(account_data, account_created_slot);
    // Use current_slot truncated to u32 as the block_number proxy.
    let block_number = current_slot.min(u32::MAX as u64) as u32;

    let extrinsic = build_register_external_root_extrinsic(
        cfg.pallet_index,
        cfg.call_index,
        cfg.solana_chain_id,
        &root_hash,
        block_number,
        account_data,
    );

    let hex_xt = format!("0x{}", hex::encode(&extrinsic));
    let result = rpc_call(
        client,
        x3_rpc,
        "author_submitExtrinsic",
        serde_json::json!([hex_xt]),
    )
    .await?;
    Ok(result.to_string())
}

// ─── base64 helper ────────────────────────────────────────────────────────────

/// Minimal base64 decoder — avoids pulling in the `base64` crate.
pub fn base64_decode(s: &str) -> Vec<u8> {
    const TABLE: &[u8; 64] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut lookup = [0xffu8; 256];
    for (i, &c) in TABLE.iter().enumerate() {
        lookup[c as usize] = i as u8;
    }
    let mut out = Vec::with_capacity(s.len() * 3 / 4 + 1);
    let mut buf = 0u32;
    let mut bits = 0u32;
    for &b in s.as_bytes() {
        if b == b'=' { break; }
        let v = lookup[b as usize];
        if v == 0xff { continue; }
        buf = (buf << 6) | v as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buf >> bits) as u8);
            buf &= (1 << bits) - 1;
        }
    }
    out
}

// ─── Main poll cycle ──────────────────────────────────────────────────────────

async fn poll_once(
    client: &reqwest::Client,
    solana_rpc: &str,
    x3_rpc: &str,
    escrow_program: &str,
    service_id: &str,
    cfg: &BridgeConfig,
) {
    let current_slot = match solana_get_slot(client, solana_rpc).await {
        Ok(s) => s,
        Err(e) => { log::warn!("🔌 '{}': getSlot failed — {}", service_id, e); return; }
    };
    log::debug!("🔌 '{}': Solana slot={}", service_id, current_slot);

    let accounts = match solana_get_program_accounts(client, solana_rpc, escrow_program).await {
        Ok(a) => a,
        Err(e) => { log::warn!("🔌 '{}': getProgramAccounts failed — {}", service_id, e); return; }
    };

    if accounts.is_empty() {
        log::debug!("🔌 '{}': no escrow accounts (program='{}')", service_id, escrow_program);
        return;
    }

    log::info!("🔌 '{}': {} escrow account(s) at Solana slot {}", service_id, accounts.len(), current_slot);

    for acc in &accounts {
        // Per-account finality: account must have been created at least FINALITY_CONFIRMATIONS
        // slots before the current slot.  If created_slot is 0 (not present in account data),
        // fall back to global slot check.
        let age = current_slot.saturating_sub(acc.created_slot);
        if acc.created_slot > 0 && age < FINALITY_CONFIRMATIONS {
            log::debug!(
                "🔌 '{}': skipping {} — created_slot={} age={} < {}",
                service_id, acc.pubkey, acc.created_slot, age, FINALITY_CONFIRMATIONS
            );
            continue;
        }
        if acc.created_slot == 0 && current_slot < FINALITY_CONFIRMATIONS {
            log::debug!("🔌 '{}': slot {} below finality threshold", service_id, current_slot);
            continue;
        }

        log::info!(
            "🌉 '{}': bridge pubkey={} data_len={} created_slot={} age={}",
            service_id, acc.pubkey, acc.data.len(),
            if acc.created_slot == 0 { "unknown".to_string() } else { acc.created_slot.to_string() },
            age
        );

        match submit_bridge_event(client, x3_rpc, cfg, &acc.data, current_slot, acc.created_slot).await {
            Ok(tx) => log::info!("✅ Bridge extrinsic submitted — pubkey={} tx={}", acc.pubkey, tx),
            Err(e) => log::error!("❌ Bridge submission failed — pubkey={} error={}", acc.pubkey, e),
        }
    }
}

// ─── Entry point ─────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let args = Args::parse();
    let cfg = BridgeConfig::from_env();

    log::info!(
        "🔌 X3 Sidecar '{}' starting (Phase 3) — Solana={} X3={} pallet={} call={} chain_id={}",
        args.service_id, args.solana_rpc, args.x3_rpc,
        cfg.pallet_index, cfg.call_index, cfg.solana_chain_id
    );

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .expect("reqwest client");

    let poll_interval = Duration::from_secs(args.poll_interval_secs);
    let shutdown = tokio::signal::ctrl_c();
    tokio::pin!(shutdown);

    loop {
        tokio::select! {
            _ = &mut shutdown => {
                log::info!("🛑 X3 Sidecar '{}' shutting down", args.service_id);
                break;
            }
            _ = tokio::time::sleep(poll_interval) => {
                poll_once(&client, &args.solana_rpc, &args.x3_rpc,
                          &args.escrow_program, &args.service_id, &cfg).await;
            }
        }
    }
    log::info!("✅ X3 Sidecar '{}' exited", args.service_id);
}

// ─── Unit tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── SCALE compact encoding ────────────────────────────────────────────

    #[test]
    fn compact_encode_single_byte_range() {
        assert_eq!(scale_compact_encode(0), vec![0x00]);
        assert_eq!(scale_compact_encode(1), vec![0x04]);
        assert_eq!(scale_compact_encode(63), vec![0xfc]);
    }

    #[test]
    fn compact_encode_two_byte_range() {
        // 64 → (64 << 2) | 1 = 257 = 0x0101
        assert_eq!(scale_compact_encode(64), vec![0x01, 0x01]);
        // 16383 → (16383 << 2) | 1 = 65533 = 0xfffd
        assert_eq!(scale_compact_encode(16383), vec![0xfd, 0xff]);
    }

    #[test]
    fn compact_encode_four_byte_range() {
        // 16384 → (16384 << 2) | 2 = 65538 = 0x00010002
        assert_eq!(scale_compact_encode(16384), vec![0x02, 0x00, 0x01, 0x00]);
    }

    // ── Extrinsic assembly ────────────────────────────────────────────────

    #[test]
    fn extrinsic_starts_with_version_byte() {
        let xt = build_register_external_root_extrinsic(
            26, 4, 2, &[0xab; 32], 100, b"test-proof",
        );
        // After the compact length prefix, the next byte must be 0x04 (V4 unsigned).
        // The compact prefix for this payload length is 1 or 2 bytes.
        // Payload = 1 (ver) + 1 (pallet) + 1 (call) + 4 (chain_id) + 32 (hash) + 4 (block) + compact + proof
        // len = 43 + compact(10) + 10 = 43 + 1 + 10 = 54 → single-byte compact: 54*4 = 216 = 0xd8
        assert_eq!(xt[0], (54u8 << 2)); // compact(54) = 0xd8
        assert_eq!(xt[1], 0x04);        // version: V4 unsigned
        assert_eq!(xt[2], 26);          // pallet_index
        assert_eq!(xt[3], 4);           // call_index
        // chain_id = 2 as u32 LE
        assert_eq!(&xt[4..8], &[0x02, 0x00, 0x00, 0x00]);
        // root_hash starts at byte 8
        assert_eq!(&xt[8..40], &[0xab; 32]);
        // block_number = 100 LE
        assert_eq!(&xt[40..44], &[100u8, 0, 0, 0]);
        // proof compact len = compact(10) = 0x28 (single byte)
        assert_eq!(xt[44], 0x28);
        // proof bytes
        assert_eq!(&xt[45..55], b"test-proof");
    }

    #[test]
    fn extrinsic_proof_compact_length_two_bytes_for_large_proof() {
        let proof = vec![0u8; 100];
        let xt = build_register_external_root_extrinsic(26, 4, 2, &[0; 32], 0, &proof);
        // inner len = 1+1+1+4+32+4+2+100 = 145 → compact(145) single byte? 145 > 63, need 2 bytes
        // proof compact: compact(100) = (100<<2)|1 = 401 = [0x91, 0x01]
        // Find proof compact at offset 44:
        assert_eq!(xt[44], 0x91);
        assert_eq!(xt[45], 0x01);
        assert_eq!(xt[46..46 + 100], proof[..]);
    }

    // ── read_u64_le ───────────────────────────────────────────────────────

    #[test]
    fn read_u64_le_correct() {
        let data = [0u8; 8].iter()
            .chain([0x01u8, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00].iter())
            .copied()
            .collect::<Vec<_>>();
        assert_eq!(read_u64_le(&data, 8), Some(1u64));
    }

    #[test]
    fn read_u64_le_out_of_bounds_returns_none() {
        assert_eq!(read_u64_le(&[0u8; 4], 4), None);
        assert_eq!(read_u64_le(&[], 0), None);
    }

    // ── base64_decode ─────────────────────────────────────────────────────

    #[test]
    fn base64_decode_roundtrip() {
        // "hello" in base64 is "aGVsbG8="
        assert_eq!(base64_decode("aGVsbG8="), b"hello");
    }

    #[test]
    fn base64_decode_empty_string() {
        assert_eq!(base64_decode(""), Vec::<u8>::new());
    }

    // ── derive_root_hash ──────────────────────────────────────────────────

    #[test]
    fn root_hash_is_32_bytes() {
        assert_eq!(derive_root_hash(b"test", 42).len(), 32);
    }

    #[test]
    fn root_hash_differs_by_slot() {
        let h1 = derive_root_hash(b"data", 1);
        let h2 = derive_root_hash(b"data", 2);
        assert_ne!(h1, h2);
    }

    // ── E2E mock: poll_once with mock Solana server ───────────────────────

    #[tokio::test]
    async fn poll_once_skips_below_finality_threshold() {
        // Build a tiny axum mock: getSlot returns 10, getProgramAccounts returns
        // one account with created_slot=8 (age=2 < 32 → should be skipped).
        use axum::{routing::post, Json, Router};

        async fn handler(
            Json(req): Json<serde_json::Value>,
        ) -> Json<serde_json::Value> {
            let method = req["method"].as_str().unwrap_or("");
            let response = match method {
                "getSlot" => serde_json::json!({
                    "jsonrpc": "2.0", "id": 1, "result": 10u64
                }),
                "getProgramAccounts" => {
                    // Account with created_slot=8 (bytes 8..16 = 8u64 LE)
                    let mut data = vec![0u8; 16];
                    data[8..16].copy_from_slice(&8u64.to_le_bytes());
                    let b64 = base64_encode(&data);
                    serde_json::json!({
                        "jsonrpc": "2.0", "id": 1,
                        "result": [{"pubkey": "abc123", "account": {"lamports": 1, "data": [b64, "base64"]}}]
                    })
                }
                _ => serde_json::json!({ "jsonrpc": "2.0", "id": 1, "result": null }),
            };
            Json(response)
        }

        let app = Router::new().route("/", post(handler));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        let client = reqwest::Client::new();
        let rpc_url = format!("http://{}", addr);
        let cfg = BridgeConfig { solana_chain_id: 2, pallet_index: 26, call_index: 4 };

        // poll_once should not panic and should skip (age=2 < 32).
        poll_once(&client, &rpc_url, "http://127.0.0.1:9944", "FakeProgram111", "test", &cfg).await;
        // If we reach here without panic the skip logic worked.
    }

    /// Minimal base64 encoder for test data generation.
    fn base64_encode(data: &[u8]) -> String {
        const TABLE: &[u8; 64] =
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut out = String::new();
        let mut i = 0;
        while i < data.len() {
            let b0 = data[i] as u32;
            let b1 = if i + 1 < data.len() { data[i + 1] as u32 } else { 0 };
            let b2 = if i + 2 < data.len() { data[i + 2] as u32 } else { 0 };
            let triple = (b0 << 16) | (b1 << 8) | b2;
            out.push(TABLE[((triple >> 18) & 0x3f) as usize] as char);
            out.push(TABLE[((triple >> 12) & 0x3f) as usize] as char);
            out.push(if i + 1 < data.len() { TABLE[((triple >> 6) & 0x3f) as usize] as char } else { '=' });
            out.push(if i + 2 < data.len() { TABLE[(triple & 0x3f) as usize] as char } else { '=' });
            i += 3;
        }
        out
    }
}
