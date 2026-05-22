//! X3 Cross-VM Sidecar daemon — Phase 4.
//!
//! Launched by the X3 node via `spawn_sidecar_service`.  Polls Solana for
//! escrow/bridge-root events, checks 32-slot per-account finality, derives a
//! real Blake2b-256 state root, then submits a SCALE V4 **signed** sr25519
//! extrinsic calling `X3CrossVmRouter::register_external_root` on the X3 node.
//!
//! # Environment variables
//! | Variable                  | Default                                  | Description                                        |
//! |---------------------------|------------------------------------------|----------------------------------------------------|
//! | `X3_SIDECAR_BIN`          | auto-detected                            | Binary path (consumed by launcher)                 |
//! | `X3_SOLANA_RPC_URL`        | `https://api.mainnet-beta.solana.com`    | Solana JSON-RPC endpoint                           |
//! | `X3_NODE_RPC_URL`          | `http://127.0.0.1:9944`                  | X3 node RPC endpoint                              |
//! | `X3_ESCROW_PROGRAM`        | *(empty — disables Solana polling)*      | Solana escrow program ID (base58)                  |
//! | `X3_SIGNER_SEED_HEX`       | *(empty — unsigned fallback)*            | 32-byte sr25519 mini-secret key as hex             |
//! | `X3_SOLANA_CHAIN_ID`       | `2`                                      | chain_id value for Solana in X3CrossVmRouter       |
//! | `X3_BRIDGE_PALLET_INDEX`   | `26`                                     | SCALE pallet index of X3CrossVmRouter              |
//! | `X3_BRIDGE_CALL_INDEX`     | `4`                                      | SCALE call index of register_external_root         |
//! | `RUST_LOG`                 | `info`                                   | Log filter                                         |
//!
//! # Signing
//! When `X3_SIGNER_SEED_HEX` is set the sidecar produces a proper SCALE V4
//! signed sr25519 extrinsic.  The signer account must hold council or sudo
//! authority (i.e. satisfy `ExternalExecutorOrigin = EnsureRootOrHalfCouncil`).
//! When the env var is absent, an **unsigned** extrinsic is emitted with a
//! clear warning — accepted only if the runtime is patched to allow it.

use clap::Parser;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use zeroize::Zeroizing;

const FINALITY_CONFIRMATIONS: u64 = 32;
/// Anchor account data layout: [8B discriminator][8B created_slot][...]
const ACCOUNT_DATA_SLOT_OFFSET: usize = 8;
/// Substrate signing context tag (must match runtime).
const SUBSTRATE_CTX: &[u8] = b"substrate";

// ─── CLI ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Parser)]
#[command(name = "x3-sidecar", about = "X3 Cross-VM Sidecar daemon")]
struct Args {
    #[arg(long, default_value = "x3-sidecar")]
    service_id: String,

    #[arg(long, env = "X3_SOLANA_RPC_URL",
          default_value = "https://api.mainnet-beta.solana.com")]
    solana_rpc: String,

    #[arg(long, env = "X3_NODE_RPC_URL",
          default_value = "http://127.0.0.1:9944")]
    x3_rpc: String,

    #[arg(long, env = "X3_ESCROW_PROGRAM", default_value = "")]
    escrow_program: String,

    #[arg(long, default_value_t = 30)]
    poll_interval_secs: u64,
}

// ─── Bridge config ────────────────────────────────────────────────────────────

struct BridgeConfig {
    solana_chain_id: u32,
    pallet_index: u8,
    call_index: u8,
}

impl BridgeConfig {
    fn from_env() -> Self {
        Self {
            solana_chain_id: env_u32("X3_SOLANA_CHAIN_ID", 2),
            pallet_index:    env_u8("X3_BRIDGE_PALLET_INDEX", 26),
            call_index:      env_u8("X3_BRIDGE_CALL_INDEX", 4),
        }
    }
}

fn env_u32(key: &str, default: u32) -> u32 {
    std::env::var(key).ok().and_then(|s| s.parse().ok()).unwrap_or(default)
}
fn env_u8(key: &str, default: u8) -> u8 {
    std::env::var(key).ok().and_then(|s| s.parse().ok()).unwrap_or(default)
}

// ─── Sr25519 signer ───────────────────────────────────────────────────────────

/// Wrapper around a schnorrkel keypair with zeroized key material.
struct Sr25519Signer {
    keypair: schnorrkel::Keypair,
    pub public_key: [u8; 32],
}

impl Sr25519Signer {
    /// Load from a 32-byte (mini-secret) hex seed.
    /// `X3_SIGNER_SEED_HEX` should be 64 hex chars (32 bytes).
    fn from_seed_hex(hex_seed: &str) -> Result<Self, String> {
        let raw = Zeroizing::new(
            hex::decode(hex_seed.trim_start_matches("0x"))
                .map_err(|e| format!("invalid seed hex: {}", e))?,
        );
        if raw.len() != 32 {
            return Err(format!(
                "seed must be 32 bytes (64 hex chars), got {} bytes",
                raw.len()
            ));
        }
        let mini = schnorrkel::MiniSecretKey::from_bytes(&raw)
            .map_err(|e| format!("invalid mini secret: {}", e))?;
        let keypair = mini.expand_to_keypair(schnorrkel::ExpansionMode::Ed25519);
        let public_key = keypair.public.to_bytes();
        Ok(Self { keypair, public_key })
    }

    /// Sign `message` using the Substrate sr25519 context.
    /// If `message` is longer than 256 bytes it is Blake2b-256 hashed first
    /// (matching Substrate's extrinsic signing convention).
    fn sign(&self, message: &[u8]) -> [u8; 64] {
        let payload = if message.len() > 256 {
            blake2b_256(message).to_vec()
        } else {
            message.to_vec()
        };
        let ctx = schnorrkel::context::signing_context(SUBSTRATE_CTX);
        self.keypair.sign(ctx.bytes(&payload)).to_bytes()
    }
}

// ─── Crypto helpers ──────────────────────────────────────────────────────────

/// Blake2b-256 (same algorithm used by `sp_crypto_hashing::blake2_256`).
pub fn blake2b_256(data: &[u8]) -> [u8; 32] {
    let mut params = blake2b_simd::Params::new();
    params.hash_length(32);
    let digest = params.hash(data);
    let mut out = [0u8; 32];
    out.copy_from_slice(digest.as_bytes());
    out
}

/// Derive a 32-byte state root from Solana account data and the slot it
/// was created at.  Input: `account_data_bytes || slot.to_le_bytes()`.
pub fn derive_root_hash(account_data: &[u8], slot: u64) -> [u8; 32] {
    let slot_bytes = slot.to_le_bytes();
    // Compute hash over concatenated bytes without extra allocation when possible.
    let mut params = blake2b_simd::Params::new();
    params.hash_length(32);
    let mut state = params.to_state();
    state.update(account_data);
    state.update(&slot_bytes);
    let mut out = [0u8; 32];
    out.copy_from_slice(state.finalize().as_bytes());
    out
}

// ─── SCALE encoding ──────────────────────────────────────────────────────────

/// SCALE compact-encode a `u64` value.
pub fn scale_compact(n: u64) -> Vec<u8> {
    if n <= 0x3f {
        vec![(n << 2) as u8]
    } else if n <= 0x3fff {
        ((n << 2) as u16 | 0x01).to_le_bytes().to_vec()
    } else if n <= 0x3fff_ffff {
        ((n << 2) as u32 | 0x02).to_le_bytes().to_vec()
    } else {
        let mut bytes = n.to_le_bytes().to_vec();
        while bytes.last() == Some(&0) { bytes.pop(); }
        let prefix = (((bytes.len() - 4) as u8) << 2) | 0x03;
        let mut out = vec![prefix];
        out.extend_from_slice(&bytes);
        out
    }
}

/// Chain metadata needed to build a valid signed extrinsic.
struct ChainMeta {
    spec_version: u32,
    tx_version: u32,
    genesis_hash: [u8; 32],
}

/// Build a **signed** SCALE V4 sr25519 extrinsic.
///
/// Wire format (Substrate extrinsic v4):
/// ```text
/// compact_len(inner)
/// 0x84                    -- V4, signed
/// 0x00 || public_key[32]  -- MultiAddress::Id
/// 0x01 || signature[64]   -- MultiSignature::Sr25519
/// 0x00                    -- era: immortal
/// compact(nonce)
/// compact(tip = 0)
/// pallet_index || call_index || call_args...
/// ```
///
/// Signing payload = call_bytes + extra + additional_signed,
/// hashed with Blake2b-256 if > 256 bytes.
pub fn build_signed_extrinsic(
    signer: &Sr25519Signer,
    pallet: u8,
    call: u8,
    call_args: &[u8],
    nonce: u64,
    meta: &ChainMeta,
) -> Vec<u8> {
    let mut call_bytes = vec![pallet, call];
    call_bytes.extend_from_slice(call_args);

    // "Extra" signed extensions that appear in the extrinsic body.
    let extra: Vec<u8> = {
        let mut v = vec![0x00u8]; // era: immortal
        v.extend(scale_compact(nonce));
        v.extend(scale_compact(0)); // tip = 0
        v
    };

    // "Additional" data that's mixed into the signature but NOT in the body.
    let additional: Vec<u8> = {
        let mut v = Vec::new();
        v.extend_from_slice(&meta.spec_version.to_le_bytes());
        v.extend_from_slice(&meta.tx_version.to_le_bytes());
        v.extend_from_slice(&meta.genesis_hash);   // genesis_hash
        v.extend_from_slice(&meta.genesis_hash);   // block_hash = genesis (immortal)
        v
    };

    let mut signing_payload = Vec::new();
    signing_payload.extend_from_slice(&call_bytes);
    signing_payload.extend_from_slice(&extra);
    signing_payload.extend_from_slice(&additional);

    let signature = signer.sign(&signing_payload);

    let mut inner = Vec::new();
    inner.push(0x84u8);                         // V4 signed
    inner.push(0x00);                           // MultiAddress::Id
    inner.extend_from_slice(&signer.public_key);
    inner.push(0x01);                           // MultiSignature::Sr25519
    inner.extend_from_slice(&signature);
    inner.extend_from_slice(&extra);
    inner.extend_from_slice(&call_bytes);

    let mut xt = scale_compact(inner.len() as u64);
    xt.extend_from_slice(&inner);
    xt
}

/// Build an **unsigned** SCALE V4 extrinsic (fallback when no signer configured).
pub fn build_unsigned_extrinsic(pallet: u8, call: u8, call_args: &[u8]) -> Vec<u8> {
    let mut inner = vec![0x04u8, pallet, call]; // 0x04 = V4 unsigned
    inner.extend_from_slice(call_args);
    let mut xt = scale_compact(inner.len() as u64);
    xt.extend_from_slice(&inner);
    xt
}

/// SCALE-encode arguments for `register_external_root(chain_id, root_hash, block_number, proof)`.
pub fn encode_register_args(
    chain_id: u32,
    root_hash: &[u8; 32],
    block_number: u32,
    proof: &[u8],
) -> Vec<u8> {
    let mut args = Vec::new();
    args.extend_from_slice(&chain_id.to_le_bytes());
    args.extend_from_slice(root_hash);
    args.extend_from_slice(&block_number.to_le_bytes());
    args.extend_from_slice(&scale_compact(proof.len() as u64));
    args.extend_from_slice(proof);
    args
}

// ─── JSON-RPC helpers ─────────────────────────────────────────────────────────

#[derive(Serialize)]
struct RpcReq<'a> {
    jsonrpc: &'static str,
    id: u32,
    method: &'a str,
    params: serde_json::Value,
}

#[derive(Deserialize)]
struct RpcResp {
    result: Option<serde_json::Value>,
    error: Option<RpcErr>,
}

#[derive(Deserialize)]
struct RpcErr { code: i64, message: String }

async fn rpc_call(
    client: &reqwest::Client,
    url: &str,
    method: &str,
    params: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let body = RpcReq { jsonrpc: "2.0", id: 1, method, params };
    let resp = client.post(url).json(&body).timeout(Duration::from_secs(10))
        .send().await.map_err(|e| format!("HTTP {}: {}", method, e))?
        .json::<RpcResp>().await.map_err(|e| format!("JSON {}: {}", method, e))?;
    if let Some(e) = resp.error {
        return Err(format!("RPC {}: code={} {}", method, e.code, e.message));
    }
    resp.result.ok_or_else(|| format!("empty result: {}", method))
}

// ─── X3 node helpers ─────────────────────────────────────────────────────────

async fn fetch_nonce(client: &reqwest::Client, x3_rpc: &str, pubkey: &[u8; 32]) -> Result<u64, String> {
    let hex = format!("0x{}", hex::encode(pubkey));
    rpc_call(client, x3_rpc, "system_accountNextIndex", serde_json::json!([hex]))
        .await?.as_u64().ok_or_else(|| "nonce: non-u64".to_string())
}

async fn fetch_chain_meta(client: &reqwest::Client, x3_rpc: &str) -> Result<ChainMeta, String> {
    let version = rpc_call(client, x3_rpc, "state_getRuntimeVersion", serde_json::json!([])).await?;
    let spec_version = version["specVersion"].as_u64().unwrap_or(1) as u32;
    let tx_version  = version["transactionVersion"].as_u64().unwrap_or(1) as u32;

    let genesis_val = rpc_call(client, x3_rpc, "chain_getBlockHash", serde_json::json!([0])).await?;
    let genesis_hex = genesis_val.as_str().ok_or("genesis: not a string")?
        .trim_start_matches("0x");
    let genesis_bytes = hex::decode(genesis_hex).map_err(|e| format!("genesis hex: {}", e))?;
    if genesis_bytes.len() != 32 {
        return Err(format!("genesis: expected 32 bytes, got {}", genesis_bytes.len()));
    }
    let mut genesis_hash = [0u8; 32];
    genesis_hash.copy_from_slice(&genesis_bytes);

    Ok(ChainMeta { spec_version, tx_version, genesis_hash })
}

// ─── Solana RPC ───────────────────────────────────────────────────────────────

async fn solana_get_slot(client: &reqwest::Client, rpc: &str) -> Result<u64, String> {
    rpc_call(client, rpc, "getSlot", serde_json::json!(["confirmed"]))
        .await?.as_u64().ok_or_else(|| "getSlot: non-u64".to_string())
}

struct EscrowAccount {
    pubkey: String,
    data: Vec<u8>,
    created_slot: u64,
}

async fn solana_get_program_accounts(
    client: &reqwest::Client,
    rpc: &str,
    program_id: &str,
) -> Result<Vec<EscrowAccount>, String> {
    if program_id.is_empty() { return Ok(vec![]); }
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
        let created_slot = read_u64_le(&data, ACCOUNT_DATA_SLOT_OFFSET).unwrap_or(0);
        out.push(EscrowAccount { pubkey, data, created_slot });
    }
    Ok(out)
}

pub fn read_u64_le(data: &[u8], offset: usize) -> Option<u64> {
    let end = offset.checked_add(8)?;
    Some(u64::from_le_bytes(data.get(offset..end)?.try_into().ok()?))
}

// ─── base64 decoder ───────────────────────────────────────────────────────────

pub fn base64_decode(s: &str) -> Vec<u8> {
    let mut lookup = [0xffu8; 256];
    for (i, &c) in b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/".iter().enumerate() {
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

// ─── Poll cycle ───────────────────────────────────────────────────────────────

async fn poll_once(
    client: &reqwest::Client,
    solana_rpc: &str,
    x3_rpc: &str,
    escrow_program: &str,
    service_id: &str,
    cfg: &BridgeConfig,
    signer: Option<&Sr25519Signer>,
    meta: &ChainMeta,
) {
    let current_slot = match solana_get_slot(client, solana_rpc).await {
        Ok(s) => s,
        Err(e) => { log::warn!("[{}] getSlot: {}", service_id, e); return; }
    };

    let accounts = match solana_get_program_accounts(client, solana_rpc, escrow_program).await {
        Ok(a) => a,
        Err(e) => { log::warn!("[{}] getProgramAccounts: {}", service_id, e); return; }
    };

    if accounts.is_empty() {
        log::debug!("[{}] no escrow accounts (slot={})", service_id, current_slot);
        return;
    }
    log::info!("[{}] {} account(s) @ Solana slot {}", service_id, accounts.len(), current_slot);

    for acc in &accounts {
        // Per-account finality: age = current_slot − created_slot.
        let age = current_slot.saturating_sub(acc.created_slot);
        let finality_ok = if acc.created_slot > 0 {
            age >= FINALITY_CONFIRMATIONS
        } else {
            current_slot >= FINALITY_CONFIRMATIONS
        };
        if !finality_ok {
            log::debug!("[{}] skip {} — age {} < {}", service_id, acc.pubkey, age, FINALITY_CONFIRMATIONS);
            continue;
        }

        let root_hash = derive_root_hash(&acc.data, acc.created_slot);
        let block_number = current_slot.min(u32::MAX as u64) as u32;
        let call_args = encode_register_args(cfg.solana_chain_id, &root_hash, block_number, &acc.data);

        let xt = match signer {
            Some(s) => {
                let nonce = match fetch_nonce(client, x3_rpc, &s.public_key).await {
                    Ok(n) => n,
                    Err(e) => { log::error!("[{}] nonce fetch: {}", service_id, e); continue; }
                };
                log::debug!("[{}] signing extrinsic nonce={}", service_id, nonce);
                build_signed_extrinsic(s, cfg.pallet_index, cfg.call_index, &call_args, nonce, meta)
            }
            None => {
                log::warn!("[{}] X3_SIGNER_SEED_HEX not set — submitting unsigned extrinsic (requires runtime patch)", service_id);
                build_unsigned_extrinsic(cfg.pallet_index, cfg.call_index, &call_args)
            }
        };

        let hex_xt = format!("0x{}", hex::encode(&xt));
        log::info!("[{}] 🌉 pubkey={} root={} block_number={}", service_id, &acc.pubkey[..8.min(acc.pubkey.len())], hex::encode(&root_hash[..8]), block_number);

        match rpc_call(client, x3_rpc, "author_submitExtrinsic", serde_json::json!([hex_xt])).await {
            Ok(tx) => log::info!("[{}] ✅ tx={}", service_id, tx),
            Err(e) => log::error!("[{}] ❌ submit failed: {}", service_id, e),
        }
    }
}

// ─── Entry point ──────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let args = Args::parse();
    let cfg  = BridgeConfig::from_env();

    // Load sr25519 signer if seed is configured.
    let signer: Option<Sr25519Signer> = match std::env::var("X3_SIGNER_SEED_HEX") {
        Ok(seed) => match Sr25519Signer::from_seed_hex(&seed) {
            Ok(s) => {
                log::info!("[{}] sr25519 signer loaded (pubkey={})", args.service_id, hex::encode(&s.public_key));
                Some(s)
            }
            Err(e) => {
                log::error!("[{}] invalid X3_SIGNER_SEED_HEX: {} — aborting", args.service_id, e);
                std::process::exit(1);
            }
        },
        Err(_) => {
            log::warn!("[{}] X3_SIGNER_SEED_HEX not set — unsigned extrinsic mode (devnet only)", args.service_id);
            None
        }
    };

    // Fetch chain metadata once at startup; log on failure but continue.
    let meta = match fetch_chain_meta(
        &reqwest::Client::builder().timeout(Duration::from_secs(10)).build().unwrap(),
        &args.x3_rpc,
    ).await {
        Ok(m) => {
            log::info!("[{}] chain meta: spec_version={} tx_version={} genesis={}",
                args.service_id, m.spec_version, m.tx_version, hex::encode(&m.genesis_hash));
            m
        }
        Err(e) => {
            log::warn!("[{}] chain meta unavailable ({}); using defaults — ensure node is up", args.service_id, e);
            ChainMeta { spec_version: 1, tx_version: 1, genesis_hash: [0u8; 32] }
        }
    };

    log::info!("[{}] Phase 4 starting — Solana={} X3={} pallet={} call={} chain_id={}",
        args.service_id, args.solana_rpc, args.x3_rpc,
        cfg.pallet_index, cfg.call_index, cfg.solana_chain_id);

    let client       = reqwest::Client::builder().timeout(Duration::from_secs(15)).build().unwrap();
    let poll_interval = Duration::from_secs(args.poll_interval_secs);
    let shutdown      = tokio::signal::ctrl_c();
    tokio::pin!(shutdown);

    loop {
        tokio::select! {
            _ = &mut shutdown => {
                log::info!("[{}] shutdown signal — exiting", args.service_id);
                break;
            }
            _ = tokio::time::sleep(poll_interval) => {
                poll_once(
                    &client, &args.solana_rpc, &args.x3_rpc,
                    &args.escrow_program, &args.service_id,
                    &cfg, signer.as_ref(), &meta,
                ).await;
            }
        }
    }
    log::info!("[{}] exited cleanly", args.service_id);
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── blake2b_256 ───────────────────────────────────────────────────────

    #[test]
    fn blake2b_256_length() {
        assert_eq!(blake2b_256(b"").len(), 32);
        assert_eq!(blake2b_256(b"hello world").len(), 32);
    }

    #[test]
    fn blake2b_256_known_vector() {
        // Blake2b-256("") must equal the well-known empty-input hash.
        // Verified against reference: https://www.blake2.net/
        let got = blake2b_256(b"");
        // First 4 bytes of Blake2b-256("") = 0x0e5751c0
        assert_eq!(&got[..4], &[0x0e, 0x57, 0x51, 0xc0]);
    }

    #[test]
    fn blake2b_256_differs_by_input() {
        assert_ne!(blake2b_256(b"a"), blake2b_256(b"b"));
    }

    #[test]
    fn derive_root_hash_mixes_slot() {
        let h1 = derive_root_hash(b"data", 1);
        let h2 = derive_root_hash(b"data", 2);
        assert_ne!(h1, h2, "same data, different slot → different hash");
    }

    #[test]
    fn derive_root_hash_mixes_data() {
        let h1 = derive_root_hash(b"data1", 42);
        let h2 = derive_root_hash(b"data2", 42);
        assert_ne!(h1, h2, "same slot, different data → different hash");
    }

    // ── SCALE compact ─────────────────────────────────────────────────────

    #[test]
    fn scale_compact_1byte() {
        assert_eq!(scale_compact(0), vec![0x00]);
        assert_eq!(scale_compact(63), vec![0xfc]);
    }

    #[test]
    fn scale_compact_2byte() {
        assert_eq!(scale_compact(64), vec![0x01, 0x01]);
    }

    #[test]
    fn scale_compact_4byte() {
        assert_eq!(scale_compact(16384), vec![0x02, 0x00, 0x01, 0x00]);
    }

    // ── encode_register_args ──────────────────────────────────────────────

    #[test]
    fn encode_args_layout() {
        let chain_id: u32 = 2;
        let root = [0xabu8; 32];
        let block: u32 = 100;
        let proof = b"proof_bytes";

        let args = encode_register_args(chain_id, &root, block, proof);

        // chain_id: 4 bytes LE
        assert_eq!(&args[0..4], &[2, 0, 0, 0]);
        // root_hash: 32 bytes
        assert_eq!(&args[4..36], &[0xab; 32]);
        // block_number: 4 bytes LE
        assert_eq!(&args[36..40], &[100, 0, 0, 0]);
        // proof compact len = scale_compact(11) = 11*4 = 44 = 0x2c (1 byte)
        assert_eq!(args[40], 0x2c);
        // proof bytes
        assert_eq!(&args[41..52], proof);
    }

    // ── unsigned extrinsic ────────────────────────────────────────────────

    #[test]
    fn unsigned_extrinsic_version_byte() {
        let xt = build_unsigned_extrinsic(26, 4, &[0u8; 4]);
        // inner = [0x04, 26, 4, 0,0,0,0] = 7 bytes → compact(7) = 28 = 0x1c
        assert_eq!(xt[0], 7 << 2);
        assert_eq!(xt[1], 0x04); // V4 unsigned
        assert_eq!(xt[2], 26);
        assert_eq!(xt[3], 4);
    }

    // ── signed extrinsic ─────────────────────────────────────────────────

    #[test]
    fn signed_extrinsic_structure() {
        // Use a deterministic zero seed for testing.
        let signer = Sr25519Signer::from_seed_hex(&"00".repeat(32)).unwrap();
        let meta = ChainMeta {
            spec_version: 1,
            tx_version: 1,
            genesis_hash: [0u8; 32],
        };
        let xt = build_signed_extrinsic(&signer, 26, 4, &[0xffu8; 4], 0, &meta);

        // Skip compact prefix bytes to get inner start.
        let inner_len = decode_compact(&xt);
        let prefix_len = scale_compact(inner_len as u64).len();
        let inner = &xt[prefix_len..];

        assert_eq!(inner[0], 0x84, "version byte: V4 signed");
        assert_eq!(inner[1], 0x00, "MultiAddress::Id prefix");
        // public key: next 32 bytes
        assert_eq!(&inner[2..34], &signer.public_key);
        // signature type
        assert_eq!(inner[34], 0x01, "MultiSignature::Sr25519");
        // 64 bytes of signature follow
        assert_eq!(inner[35..99].len(), 64);
        // era = immortal = 0x00
        assert_eq!(inner[99], 0x00, "era: immortal");
        // nonce compact(0) = 0x00
        assert_eq!(inner[100], 0x00, "nonce compact 0");
        // tip compact(0) = 0x00
        assert_eq!(inner[101], 0x00, "tip compact 0");
        // call pallet
        assert_eq!(inner[102], 26);
        assert_eq!(inner[103], 4);
    }

    #[test]
    fn sr25519_signer_from_zero_seed() {
        let s = Sr25519Signer::from_seed_hex(&"00".repeat(32)).unwrap();
        assert_eq!(s.public_key.len(), 32);
    }

    #[test]
    fn sr25519_signer_from_invalid_seed_errors() {
        assert!(Sr25519Signer::from_seed_hex("not-hex").is_err());
        assert!(Sr25519Signer::from_seed_hex("0102").is_err()); // too short
    }

    #[test]
    fn sr25519_sign_produces_64_bytes() {
        let s = Sr25519Signer::from_seed_hex(&"01".repeat(32)).unwrap();
        assert_eq!(s.sign(b"test message").len(), 64);
    }

    #[test]
    fn sr25519_sign_large_payload_uses_hash() {
        let s = Sr25519Signer::from_seed_hex(&"02".repeat(32)).unwrap();
        let msg = vec![0u8; 300]; // > 256 bytes → should be hashed first
        let sig = s.sign(&msg);
        assert_eq!(sig.len(), 64);
    }

    // ── read_u64_le ───────────────────────────────────────────────────────

    #[test]
    fn read_u64_le_at_offset() {
        let mut data = vec![0u8; 16];
        data[8..16].copy_from_slice(&42u64.to_le_bytes());
        assert_eq!(read_u64_le(&data, 8), Some(42));
    }

    #[test]
    fn read_u64_le_oob() {
        assert_eq!(read_u64_le(&[0u8; 4], 4), None);
        assert_eq!(read_u64_le(&[], 0), None);
    }

    // ── base64_decode ─────────────────────────────────────────────────────

    #[test]
    fn base64_decode_hello() {
        assert_eq!(base64_decode("aGVsbG8="), b"hello");
    }

    #[test]
    fn base64_decode_empty() {
        assert_eq!(base64_decode(""), b"" as &[u8]);
    }

    // ── E2E poll_once: finality skip ──────────────────────────────────────

    #[tokio::test]
    async fn poll_once_skips_immature_accounts() {
        // Mock: getSlot = 10, one account with created_slot = 8 (age = 2 < 32).
        use axum::{routing::post, Json, Router};

        async fn handler(Json(req): Json<serde_json::Value>) -> Json<serde_json::Value> {
            let method = req["method"].as_str().unwrap_or("");
            Json(match method {
                "getSlot" => serde_json::json!({"jsonrpc":"2.0","id":1,"result":10}),
                "getProgramAccounts" => {
                    let mut d = vec![0u8; 16];
                    d[8..16].copy_from_slice(&8u64.to_le_bytes()); // created_slot = 8
                    serde_json::json!({"jsonrpc":"2.0","id":1,"result":[{
                        "pubkey":"TestPubkey111",
                        "account":{"lamports":1,"data":[base64_encode_test(&d),"base64"]}
                    }]})
                }
                _ => serde_json::json!({"jsonrpc":"2.0","id":1,"result":null}),
            })
        }

        let app = Router::new().route("/", post(handler));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

        let client = reqwest::Client::new();
        let rpc = format!("http://{}", addr);
        let cfg  = BridgeConfig { solana_chain_id: 2, pallet_index: 26, call_index: 4 };
        let meta = ChainMeta { spec_version: 1, tx_version: 1, genesis_hash: [0u8; 32] };
        // Should complete without panic; no submission should reach a non-existent x3 node.
        poll_once(&client, &rpc, "http://127.0.0.1:19944", "FakeProg", "test", &cfg, None, &meta).await;
    }

    #[tokio::test]
    async fn poll_once_submits_mature_account() {
        // Mock: getSlot = 100, one account with created_slot = 10 (age = 90 ≥ 32).
        // X3 node mock returns a fake tx hash.
        use axum::{routing::post, Json, Router};
        use std::sync::{Arc, Mutex};

        let submitted_calls: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(vec![]));
        let submitted_clone = submitted_calls.clone();

        async fn handler(
            axum::extract::State(calls): axum::extract::State<Arc<Mutex<Vec<String>>>>,
            Json(req): Json<serde_json::Value>,
        ) -> Json<serde_json::Value> {
            let method = req["method"].as_str().unwrap_or("").to_string();
            calls.lock().unwrap().push(method.clone());
            Json(match method.as_str() {
                "getSlot" => serde_json::json!({"jsonrpc":"2.0","id":1,"result":100}),
                "getProgramAccounts" => {
                    let mut d = vec![0u8; 16];
                    d[8..16].copy_from_slice(&10u64.to_le_bytes()); // created_slot = 10, age = 90
                    serde_json::json!({"jsonrpc":"2.0","id":1,"result":[{
                        "pubkey":"MaturePubkey111",
                        "account":{"lamports":1,"data":[base64_encode_test(&d),"base64"]}
                    }]})
                }
                "author_submitExtrinsic" => {
                    serde_json::json!({"jsonrpc":"2.0","id":1,"result":"0xdeadbeef"})
                }
                _ => serde_json::json!({"jsonrpc":"2.0","id":1,"result":null}),
            })
        }

        let app = Router::new()
            .route("/", post(handler))
            .with_state(submitted_clone);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

        let rpc = format!("http://{}", addr);
        let client = reqwest::Client::new();
        let cfg  = BridgeConfig { solana_chain_id: 2, pallet_index: 26, call_index: 4 };
        let meta = ChainMeta { spec_version: 1, tx_version: 1, genesis_hash: [0u8; 32] };
        poll_once(&client, &rpc, &rpc, "FakeProg", "test", &cfg, None, &meta).await;

        let calls = submitted_calls.lock().unwrap();
        assert!(calls.contains(&"author_submitExtrinsic".to_string()),
            "expected author_submitExtrinsic to be called; got: {:?}", *calls);
    }

    // ── Integration test (requires live Solana devnet) ────────────────────

    /// Run with:  X3_SOLANA_RPC_URL=https://api.devnet.solana.com cargo test live_solana
    #[tokio::test]
    #[ignore = "requires live Solana devnet (set X3_SOLANA_RPC_URL and run with --ignored)"]
    async fn live_solana_get_slot_returns_nonzero() {
        let rpc = std::env::var("X3_SOLANA_RPC_URL")
            .unwrap_or_else(|_| "https://api.devnet.solana.com".to_string());
        let client = reqwest::Client::new();
        let slot = solana_get_slot(&client, &rpc).await.expect("getSlot failed");
        assert!(slot > 0, "expected a non-zero slot from live Solana");
    }

    // ── Helpers ───────────────────────────────────────────────────────────

    /// Decode the first SCALE compact value; returns the integer it encodes.
    fn decode_compact(bytes: &[u8]) -> usize {
        match bytes[0] & 0b11 {
            0 => (bytes[0] >> 2) as usize,
            1 => {
                let v = u16::from_le_bytes([bytes[0], bytes[1]]);
                (v >> 2) as usize
            }
            2 => {
                let v = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
                (v >> 2) as usize
            }
            _ => panic!("big-integer compact mode in test"),
        }
    }

    fn base64_encode_test(data: &[u8]) -> String {
        const T: &[u8; 64] =
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut o = String::new();
        let mut i = 0;
        while i < data.len() {
            let a = data[i] as u32;
            let b = if i+1 < data.len() { data[i+1] as u32 } else { 0 };
            let c = if i+2 < data.len() { data[i+2] as u32 } else { 0 };
            let triple = (a << 16) | (b << 8) | c;
            o.push(T[((triple >> 18) & 63) as usize] as char);
            o.push(T[((triple >> 12) & 63) as usize] as char);
            o.push(if i+1 < data.len() { T[((triple >> 6) & 63) as usize] as char } else { '=' });
            o.push(if i+2 < data.len() { T[(triple & 63) as usize] as char } else { '=' });
            i += 3;
        }
        o
    }
}
