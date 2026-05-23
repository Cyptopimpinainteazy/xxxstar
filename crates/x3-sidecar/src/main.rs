//! X3 Cross-VM Sidecar daemon — Phase 4 (complete).
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
//! | `X3_META_REFRESH_SECS`     | `300`                                    | How often to re-fetch ChainMeta (spec_version etc) |
//! | `RUST_LOG`                 | `info`                                   | Log filter                                         |
//!
//! # Signing
//! When `X3_SIGNER_SEED_HEX` is set the sidecar produces a proper SCALE V4
//! signed sr25519 extrinsic.  The signer account must hold council or sudo
//! authority (i.e. satisfy `ExternalExecutorOrigin = EnsureRootOrHalfCouncil`).
//! When the env var is absent, an **unsigned** extrinsic is emitted with a
//! clear warning — accepted only if the runtime is patched to allow it.
//!
//! # ChainMeta refresh
//! `spec_version` and `transaction_version` change on every runtime upgrade.
//! Extrinsics signed with stale values are rejected by the transaction pool.
//! The sidecar re-fetches `ChainMeta` every `X3_META_REFRESH_SECS` seconds
//! (default 300) and detects spec_version changes, logging a warning when
//! a runtime upgrade is detected mid-run.

use clap::Parser;
use serde::{Deserialize, Serialize};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::RwLock;
use zeroize::Zeroizing;

/// Solana confirmation threshold before treating an account as finality-confirmed.
pub const FINALITY_CONFIRMATIONS: u64 = 32;
/// Anchor account data layout: [8B discriminator][8B created_slot][...]
const ACCOUNT_DATA_SLOT_OFFSET: usize = 8;
/// Substrate signing context tag (must match runtime).
const SUBSTRATE_CTX: &[u8] = b"substrate";
/// Default ChainMeta refresh interval (seconds).
const DEFAULT_META_REFRESH_SECS: u64 = 300;

// ─── CLI ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Parser)]
#[command(name = "x3-sidecar", about = "X3 Cross-VM Sidecar daemon (Phase 4)")]
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

    /// Re-fetch ChainMeta (spec_version, tx_version, genesis) every N seconds.
    #[arg(long, env = "X3_META_REFRESH_SECS", default_value_t = DEFAULT_META_REFRESH_SECS)]
    meta_refresh_secs: u64,
}

// ─── Bridge config ────────────────────────────────────────────────────────────

pub struct BridgeConfig {
    pub solana_chain_id: u32,
    pub pallet_index: u8,
    pub call_index: u8,
}

impl BridgeConfig {
    pub fn from_env() -> Self {
        Self {
            solana_chain_id: env_u32("X3_SOLANA_CHAIN_ID", 2),
            pallet_index:    env_u8("X3_BRIDGE_PALLET_INDEX", 26),
            call_index:      env_u8("X3_BRIDGE_CALL_INDEX", 4),
        }
    }
}

pub fn env_u32(key: &str, default: u32) -> u32 {
    std::env::var(key).ok().and_then(|s| s.parse().ok()).unwrap_or(default)
}
pub fn env_u8(key: &str, default: u8) -> u8 {
    std::env::var(key).ok().and_then(|s| s.parse().ok()).unwrap_or(default)
}

// ─── ChainMeta + refresh ─────────────────────────────────────────────────────

/// Chain metadata required to build valid signed extrinsics.
/// Must be refreshed after every runtime upgrade (spec_version change).
#[derive(Clone)]
pub struct ChainMeta {
    pub spec_version: u32,
    pub tx_version:   u32,
    pub genesis_hash: [u8; 32],
}

/// Shared, periodically-refreshed chain metadata.
pub struct MetaCache {
    meta:         RwLock<ChainMeta>,
    last_refresh: RwLock<Instant>,
    refresh_secs: u64,
    x3_rpc:       String,
}

impl MetaCache {
    pub fn new(initial: ChainMeta, refresh_secs: u64, x3_rpc: String) -> Arc<Self> {
        Arc::new(Self {
            meta: RwLock::new(initial),
            last_refresh: RwLock::new(Instant::now()),
            refresh_secs,
            x3_rpc,
        })
    }

    /// Return the current meta, re-fetching in the background if the TTL has
    /// expired.  Never blocks the caller: if a refresh is in progress, returns
    /// the last known good value.
    pub async fn get(&self, client: &reqwest::Client, service_id: &str) -> ChainMeta {
        let age = self.last_refresh.read().await.elapsed().as_secs();
        if age >= self.refresh_secs {
            self.try_refresh(client, service_id).await;
        }
        self.meta.read().await.clone()
    }

    /// Attempt to refresh metadata.  Logs but never panics on failure.
    async fn try_refresh(&self, client: &reqwest::Client, service_id: &str) {
        // Take a write lock on last_refresh immediately to prevent concurrent refreshes.
        let mut ts = self.last_refresh.write().await;
        // Double-check: another task might have refreshed while we waited.
        if ts.elapsed().as_secs() < self.refresh_secs {
            return;
        }
        *ts = Instant::now();
        drop(ts);

        match fetch_chain_meta(client, &self.x3_rpc).await {
            Ok(fresh) => {
                let old_spec = self.meta.read().await.spec_version;
                if fresh.spec_version != old_spec {
                    log::warn!(
                        "[{}] ⚡ Runtime upgrade detected: spec_version {} → {}",
                        service_id, old_spec, fresh.spec_version
                    );
                }
                *self.meta.write().await = fresh;
                log::debug!("[{}] ChainMeta refreshed", service_id);
            }
            Err(e) => {
                log::warn!("[{}] ChainMeta refresh failed ({}); using cached value", service_id, e);
            }
        }
    }

    #[cfg(test)]
    pub async fn spec_version(&self) -> u32 {
        self.meta.read().await.spec_version
    }
}

// ─── Sr25519 signer ───────────────────────────────────────────────────────────

pub struct Sr25519Signer {
    keypair: schnorrkel::Keypair,
    pub public_key: [u8; 32],
}

impl Sr25519Signer {
    /// Load from a 32-byte mini-secret seed encoded as hex.
    pub fn from_seed_hex(hex_seed: &str) -> Result<Self, String> {
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

    /// Sign with Substrate sr25519 context.  Payloads > 256 bytes are Blake2b-256
    /// pre-hashed (matches Substrate extrinsic signing convention).
    pub fn sign(&self, message: &[u8]) -> [u8; 64] {
        let payload = if message.len() > 256 {
            blake2b_256(message).to_vec()
        } else {
            message.to_vec()
        };
        let ctx = schnorrkel::context::signing_context(SUBSTRATE_CTX);
        self.keypair.sign(ctx.bytes(&payload)).to_bytes()
    }

    /// Verify a signature against a message (used in tests).
    #[cfg(test)]
    pub fn verify(&self, message: &[u8], sig_bytes: &[u8; 64]) -> bool {
        let payload = if message.len() > 256 { blake2b_256(message).to_vec() } else { message.to_vec() };
        let ctx = schnorrkel::context::signing_context(SUBSTRATE_CTX);
        let sig = schnorrkel::Signature::from_bytes(sig_bytes).unwrap();
        self.keypair.public.verify(ctx.bytes(&payload), &sig).is_ok()
    }
}

// ─── Crypto ──────────────────────────────────────────────────────────────────

/// Blake2b-256 (same as `sp_crypto_hashing::blake2_256`).
pub fn blake2b_256(data: &[u8]) -> [u8; 32] {
    let mut params = blake2b_simd::Params::new();
    params.hash_length(32);
    let mut out = [0u8; 32];
    out.copy_from_slice(params.hash(data).as_bytes());
    out
}

/// Derive a 32-byte state root: Blake2b-256(account_data || slot.to_le_bytes()).
pub fn derive_root_hash(account_data: &[u8], slot: u64) -> [u8; 32] {
    let mut params = blake2b_simd::Params::new();
    params.hash_length(32);
    let mut state = params.to_state();
    state.update(account_data);
    state.update(&slot.to_le_bytes());
    let mut out = [0u8; 32];
    out.copy_from_slice(state.finalize().as_bytes());
    out
}

// ─── SCALE encoding ──────────────────────────────────────────────────────────

/// SCALE compact-encode a u64.
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

/// Build a SCALE V4 **signed** sr25519 extrinsic.
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

    let extra: Vec<u8> = {
        let mut v = vec![0x00u8]; // era: immortal
        v.extend(scale_compact(nonce));
        v.extend(scale_compact(0)); // tip = 0
        v
    };

    let additional: Vec<u8> = {
        let mut v = Vec::new();
        v.extend_from_slice(&meta.spec_version.to_le_bytes());
        v.extend_from_slice(&meta.tx_version.to_le_bytes());
        v.extend_from_slice(&meta.genesis_hash);
        v.extend_from_slice(&meta.genesis_hash); // block_hash = genesis (immortal)
        v
    };

    let mut signing_payload = Vec::new();
    signing_payload.extend_from_slice(&call_bytes);
    signing_payload.extend_from_slice(&extra);
    signing_payload.extend_from_slice(&additional);

    let signature = signer.sign(&signing_payload);

    let mut inner = Vec::new();
    inner.push(0x84u8);                          // V4 signed
    inner.push(0x00);                            // MultiAddress::Id
    inner.extend_from_slice(&signer.public_key);
    inner.push(0x01);                            // MultiSignature::Sr25519
    inner.extend_from_slice(&signature);
    inner.extend_from_slice(&extra);
    inner.extend_from_slice(&call_bytes);

    let mut xt = scale_compact(inner.len() as u64);
    xt.extend_from_slice(&inner);
    xt
}

/// Build a SCALE V4 **unsigned** extrinsic (fallback — devnet only).
pub fn build_unsigned_extrinsic(pallet: u8, call: u8, call_args: &[u8]) -> Vec<u8> {
    let mut inner = vec![0x04u8, pallet, call]; // 0x04 = V4 unsigned
    inner.extend_from_slice(call_args);
    let mut xt = scale_compact(inner.len() as u64);
    xt.extend_from_slice(&inner);
    xt
}

/// SCALE-encode `register_external_root(chain_id, root_hash, block_number, proof)`.
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

pub async fn rpc_call(
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

pub async fn fetch_nonce(client: &reqwest::Client, x3_rpc: &str, pubkey: &[u8; 32]) -> Result<u64, String> {
    let hex = format!("0x{}", hex::encode(pubkey));
    rpc_call(client, x3_rpc, "system_accountNextIndex", serde_json::json!([hex]))
        .await?.as_u64().ok_or_else(|| "nonce: non-u64".to_string())
}

pub async fn fetch_chain_meta(client: &reqwest::Client, x3_rpc: &str) -> Result<ChainMeta, String> {
    let version = rpc_call(client, x3_rpc, "state_getRuntimeVersion", serde_json::json!([])).await?;
    let spec_version = version["specVersion"].as_u64().unwrap_or(1) as u32;
    let tx_version   = version["transactionVersion"].as_u64().unwrap_or(1) as u32;

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

pub async fn solana_get_slot(client: &reqwest::Client, rpc: &str) -> Result<u64, String> {
    rpc_call(client, rpc, "getSlot", serde_json::json!(["confirmed"]))
        .await?.as_u64().ok_or_else(|| "getSlot: non-u64".to_string())
}

pub struct EscrowAccount {
    pub pubkey: String,
    pub data: Vec<u8>,
    pub created_slot: u64,
}

pub async fn solana_get_program_accounts(
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

// ─── Finality check ───────────────────────────────────────────────────────────

/// Returns `true` when an account with `created_slot` is finality-confirmed
/// as of `current_slot`.
///
/// Rules:
/// - If `created_slot` is known (> 0): `age = current_slot − created_slot ≥ FINALITY_CONFIRMATIONS`
/// - If `created_slot` is unknown (== 0): `current_slot ≥ FINALITY_CONFIRMATIONS` (global proxy)
/// - Saturating subtraction prevents underflow when current_slot < created_slot (clock skew).
pub fn is_finality_confirmed(current_slot: u64, created_slot: u64) -> bool {
    if created_slot > 0 {
        current_slot.saturating_sub(created_slot) >= FINALITY_CONFIRMATIONS
    } else {
        current_slot >= FINALITY_CONFIRMATIONS
    }
}

// ─── string utilities ─────────────────────────────────────────────────────────

/// Return the first `max_chars` Unicode characters from `s` as an owned String.
/// This is char-boundary-safe: byte-slicing at an arbitrary byte offset panics
/// when multi-byte UTF-8 sequences straddle the cut point.  A hostile RPC can
/// return arbitrary UTF-8 in JSON string fields, so all log-prefix truncations
/// must go through this helper.
pub fn safe_str_prefix(s: &str, max_chars: usize) -> String {
    s.chars().take(max_chars).collect()
}

// ─── base64 ───────────────────────────────────────────────────────────────────

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

/// Groups the connection-oriented params for `poll_once`, keeping argument count
/// within clippy's `too_many_arguments` limit.
pub struct PollerContext<'a> {
    pub solana_rpc:     &'a str,
    pub x3_rpc:         &'a str,
    pub escrow_program: &'a str,
    pub service_id:     &'a str,
    pub cfg:            &'a BridgeConfig,
}

pub async fn poll_once(
    client:     &reqwest::Client,
    ctx:        &PollerContext<'_>,
    signer:     Option<&Sr25519Signer>,
    meta_cache: &MetaCache,
) {
    let current_slot = match solana_get_slot(client, ctx.solana_rpc).await {
        Ok(s) => s,
        Err(e) => { log::warn!("[{}] getSlot: {}", ctx.service_id, e); return; }
    };

    let accounts = match solana_get_program_accounts(client, ctx.solana_rpc, ctx.escrow_program).await {
        Ok(a) => a,
        Err(e) => { log::warn!("[{}] getProgramAccounts: {}", ctx.service_id, e); return; }
    };

    if accounts.is_empty() {
        log::debug!("[{}] no escrow accounts (slot={})", ctx.service_id, current_slot);
        return;
    }
    log::info!("[{}] {} account(s) @ Solana slot {}", ctx.service_id, accounts.len(), current_slot);

    // Fetch (possibly refreshed) chain metadata once per poll cycle.
    let meta = meta_cache.get(client, ctx.service_id).await;

    for acc in &accounts {
        if !is_finality_confirmed(current_slot, acc.created_slot) {
            let age = current_slot.saturating_sub(acc.created_slot);
            log::debug!("[{}] skip {} — age {} < {}", ctx.service_id, acc.pubkey, age, FINALITY_CONFIRMATIONS);
            continue;
        }

        let root_hash = derive_root_hash(&acc.data, acc.created_slot);
        // Clamp slot to u32 for the block_number field — Substrate block numbers
        // are u32 and Solana slot numbers can exceed u32::MAX in the far future.
        let block_number = current_slot.min(u32::MAX as u64) as u32;
        let call_args = encode_register_args(ctx.cfg.solana_chain_id, &root_hash, block_number, &acc.data);

        let xt = match signer {
            Some(s) => {
                let nonce = match fetch_nonce(client, ctx.x3_rpc, &s.public_key).await {
                    Ok(n) => n,
                    Err(e) => { log::error!("[{}] nonce fetch: {}", ctx.service_id, e); continue; }
                };
                build_signed_extrinsic(s, ctx.cfg.pallet_index, ctx.cfg.call_index, &call_args, nonce, &meta)
            }
            None => {
                log::warn!("[{}] X3_SIGNER_SEED_HEX not set — unsigned extrinsic (devnet only)", ctx.service_id);
                build_unsigned_extrinsic(ctx.cfg.pallet_index, ctx.cfg.call_index, &call_args)
            }
        };

        let hex_xt = format!("0x{}", hex::encode(&xt));
        // Use char-boundary-safe truncation: a hostile RPC could return a pubkey
        // string with multi-byte UTF-8 chars; byte-slicing at position 8 would
        // panic if that byte is inside a multi-byte sequence.
        let pk_prefix = safe_str_prefix(&acc.pubkey, 8);
        log::info!("[{}] 🌉 pubkey={}… root={} block={}", ctx.service_id, pk_prefix, hex::encode(&root_hash[..8]), block_number);

        match rpc_call(client, ctx.x3_rpc, "author_submitExtrinsic", serde_json::json!([hex_xt])).await {
            Ok(tx) => log::info!("[{}] ✅ tx={}", ctx.service_id, tx),
            Err(e) => log::error!("[{}] ❌ submit failed: {}", ctx.service_id, e),
        }
    }
}

// ─── Entry point ──────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let args = Args::parse();
    let cfg  = BridgeConfig::from_env();

    let signer: Option<Sr25519Signer> = match std::env::var("X3_SIGNER_SEED_HEX") {
        Ok(seed) => match Sr25519Signer::from_seed_hex(&seed) {
            Ok(s) => {
                log::info!("[{}] sr25519 signer loaded (pubkey={})", args.service_id, hex::encode(s.public_key));
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

    let bootstrap_client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            log::error!("failed to build HTTP client (TLS init?): {e}");
            std::process::exit(1);
        }
    };

    let initial_meta = match fetch_chain_meta(&bootstrap_client, &args.x3_rpc).await {
        Ok(m) => {
            log::info!("[{}] chain meta: spec_version={} tx_version={} genesis={}",
                args.service_id, m.spec_version, m.tx_version, hex::encode(m.genesis_hash));
            m
        }
        Err(e) => {
            log::warn!("[{}] chain meta unavailable ({}); using defaults — ensure node is up", args.service_id, e);
            ChainMeta { spec_version: 1, tx_version: 1, genesis_hash: [0u8; 32] }
        }
    };

    let meta_cache = MetaCache::new(initial_meta, args.meta_refresh_secs, args.x3_rpc.clone());

    log::info!("[{}] Phase 4 — Solana={} X3={} pallet={} call={} chain_id={} meta_refresh={}s",
        args.service_id, args.solana_rpc, args.x3_rpc,
        cfg.pallet_index, cfg.call_index, cfg.solana_chain_id, args.meta_refresh_secs);

    let client = match reqwest::Client::builder().timeout(Duration::from_secs(15)).build() {
        Ok(c) => c,
        Err(e) => {
            log::error!("failed to build polling HTTP client: {e}");
            std::process::exit(1);
        }
    };
    let poll_interval = Duration::from_secs(args.poll_interval_secs);
    let shutdown      = tokio::signal::ctrl_c();
    tokio::pin!(shutdown);

    loop {
        tokio::select! {
            _ = &mut shutdown => {
                log::info!("[{}] shutdown signal — exiting cleanly", args.service_id);
                break;
            }
            _ = tokio::time::sleep(poll_interval) => {
                let ctx = PollerContext {
                    solana_rpc:     &args.solana_rpc,
                    x3_rpc:         &args.x3_rpc,
                    escrow_program: &args.escrow_program,
                    service_id:     &args.service_id,
                    cfg:            &cfg,
                };
                poll_once(&client, &ctx, signer.as_ref(), &meta_cache).await;
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
    fn blake2b_256_length_always_32() {
        assert_eq!(blake2b_256(b"").len(), 32);
        assert_eq!(blake2b_256(&vec![0xffu8; 10_000]).len(), 32);
    }

    #[test]
    fn blake2b_256_known_empty_vector() {
        // Verified reference: https://www.blake2.net/ — Blake2b-256("")
        let got = blake2b_256(b"");
        assert_eq!(&got[..4], &[0x0e, 0x57, 0x51, 0xc0]);
    }

    #[test]
    fn blake2b_256_collision_resistance() {
        assert_ne!(blake2b_256(b"a"), blake2b_256(b"b"));
        assert_ne!(blake2b_256(b""), blake2b_256(b"\x00"));
    }

    #[test]
    fn derive_root_hash_is_32_bytes() {
        assert_eq!(derive_root_hash(b"", 0).len(), 32);
    }

    #[test]
    fn derive_root_hash_slot_mixing() {
        assert_ne!(derive_root_hash(b"data", 1), derive_root_hash(b"data", 2));
    }

    #[test]
    fn derive_root_hash_data_mixing() {
        assert_ne!(derive_root_hash(b"data1", 42), derive_root_hash(b"data2", 42));
    }

    #[test]
    fn derive_root_hash_empty_data_no_panic() {
        let h = derive_root_hash(b"", 0);
        assert_eq!(h.len(), 32);
        // Empty data with slot 0 must not be all-zeros (Blake2b always produces non-zero).
        assert_ne!(h, [0u8; 32]);
    }

    // ── SCALE compact ─────────────────────────────────────────────────────

    #[test]
    fn scale_compact_boundaries() {
        assert_eq!(scale_compact(0), vec![0x00]);
        assert_eq!(scale_compact(1), vec![0x04]);
        assert_eq!(scale_compact(63), vec![0xfc]);
        // 64 first two-byte value
        assert_eq!(scale_compact(64), vec![0x01, 0x01]);
        // 16383 last two-byte value
        assert_eq!(scale_compact(16383), vec![0xfd, 0xff]);
        // 16384 first four-byte value
        assert_eq!(scale_compact(16384), vec![0x02, 0x00, 0x01, 0x00]);
        // 2^30 - 1 last four-byte value
        assert_eq!(scale_compact(0x3fff_ffff), vec![0xfe, 0xff, 0xff, 0xff]);
    }

    #[test]
    fn scale_compact_zero_proof_length() {
        // Empty proof → compact(0) = 0x00
        assert_eq!(scale_compact(0), vec![0x00]);
    }

    // ── encode_register_args ──────────────────────────────────────────────

    #[test]
    fn encode_args_empty_proof() {
        let args = encode_register_args(2, &[0xab; 32], 100, b"");
        // 4 + 32 + 4 + 1(compact 0) = 41 bytes
        assert_eq!(args.len(), 41);
        assert_eq!(args[40], 0x00); // compact(0)
    }

    #[test]
    fn encode_args_layout_with_proof() {
        let proof = b"hello";
        let args = encode_register_args(3, &[0xbb; 32], 999, proof);
        assert_eq!(&args[0..4], &[3, 0, 0, 0]);          // chain_id LE
        assert_eq!(&args[4..36], &[0xbb; 32]);            // root_hash
        assert_eq!(&args[36..40], &[231, 3, 0, 0]);       // 999 LE
        assert_eq!(args[40], (5 << 2) as u8);             // compact(5)
        assert_eq!(&args[41..46], proof);
    }

    // ── is_finality_confirmed ─────────────────────────────────────────────

    #[test]
    fn finality_at_exact_threshold_passes() {
        // age == FINALITY_CONFIRMATIONS (32) → should confirm
        assert!(is_finality_confirmed(100, 68)); // age = 32
    }

    #[test]
    fn finality_one_below_threshold_fails() {
        // age == 31 → not yet confirmed
        assert!(!is_finality_confirmed(100, 69)); // age = 31
    }

    #[test]
    fn finality_zero_created_slot_uses_global() {
        assert!(is_finality_confirmed(32, 0));   // current ≥ 32 → ok
        assert!(!is_finality_confirmed(31, 0));  // current < 32 → not yet
    }

    #[test]
    fn finality_no_underflow_on_clock_skew() {
        // created_slot > current_slot (validator clock skew / reorg)
        assert!(!is_finality_confirmed(5, 100)); // saturating: age=0 < 32
    }

    #[test]
    fn finality_u64_max_slot_no_overflow() {
        // Extreme values must not panic.
        assert!(is_finality_confirmed(u64::MAX, u64::MAX - 100)); // age=100 ≥ 32
        assert!(!is_finality_confirmed(u64::MAX, u64::MAX));      // age=0 < 32
    }

    // ── block_number clamping ─────────────────────────────────────────────

    #[test]
    fn block_number_clamped_at_u32_max() {
        let slot = u64::from(u32::MAX) + 1_000_000;
        let block_number = slot.min(u32::MAX as u64) as u32;
        assert_eq!(block_number, u32::MAX);
    }

    // ── unsigned extrinsic ────────────────────────────────────────────────

    #[test]
    fn unsigned_extrinsic_version_and_call_bytes() {
        let xt = build_unsigned_extrinsic(26, 4, &[1u8, 2, 3]);
        // inner = [0x04, 26, 4, 1, 2, 3] = 6 bytes → compact(6) = 24 = 0x18
        assert_eq!(xt[0], 6 << 2);
        assert_eq!(xt[1], 0x04); // V4 unsigned
        assert_eq!(xt[2], 26);
        assert_eq!(xt[3], 4);
        assert_eq!(&xt[4..7], &[1u8, 2, 3]);
    }

    #[test]
    fn unsigned_extrinsic_empty_call_args() {
        let xt = build_unsigned_extrinsic(0, 0, &[]);
        assert_eq!(xt.len(), 4); // compact(3) + 3 bytes
    }

    // ── signed extrinsic ─────────────────────────────────────────────────

    fn test_meta() -> ChainMeta {
        ChainMeta { spec_version: 1, tx_version: 1, genesis_hash: [0u8; 32] }
    }

    #[test]
    fn signed_extrinsic_field_layout() {
        let signer = Sr25519Signer::from_seed_hex(&"00".repeat(32)).unwrap();
        let xt = build_signed_extrinsic(&signer, 26, 4, &[0xffu8; 4], 0, &test_meta());

        let prefix_len = scale_compact(decode_compact_val(&xt) as u64).len();
        let inner = &xt[prefix_len..];

        assert_eq!(inner[0], 0x84, "version V4 signed");
        assert_eq!(inner[1], 0x00, "MultiAddress::Id prefix");
        assert_eq!(&inner[2..34], &signer.public_key, "public key");
        assert_eq!(inner[34], 0x01, "MultiSignature::Sr25519");
        assert_eq!(inner[35..99].len(), 64, "64 byte signature");
        assert_eq!(inner[99], 0x00, "immortal era");
        assert_eq!(inner[100], 0x00, "nonce compact(0)");
        assert_eq!(inner[101], 0x00, "tip compact(0)");
        assert_eq!(inner[102], 26, "pallet");
        assert_eq!(inner[103], 4, "call");
    }

    #[test]
    fn signed_extrinsic_different_nonces_differ() {
        let signer = Sr25519Signer::from_seed_hex(&"01".repeat(32)).unwrap();
        let m = test_meta();
        let xt0 = build_signed_extrinsic(&signer, 26, 4, &[], 0, &m);
        let xt1 = build_signed_extrinsic(&signer, 26, 4, &[], 1, &m);
        assert_ne!(xt0, xt1);
    }

    // ── sr25519 signer ────────────────────────────────────────────────────

    #[test]
    fn sr25519_from_zero_seed_produces_public_key() {
        let s = Sr25519Signer::from_seed_hex(&"00".repeat(32)).unwrap();
        assert_eq!(s.public_key.len(), 32);
        assert_ne!(s.public_key, [0u8; 32], "public key must not be all-zero");
    }

    #[test]
    fn sr25519_from_invalid_hex_errors() {
        assert!(Sr25519Signer::from_seed_hex("not-hex").is_err());
    }

    #[test]
    fn sr25519_from_short_seed_errors() {
        assert!(Sr25519Signer::from_seed_hex("0102").is_err());
    }

    #[test]
    fn sr25519_sign_produces_64_bytes() {
        let s = Sr25519Signer::from_seed_hex(&"ab".repeat(32)).unwrap();
        assert_eq!(s.sign(b"hello world").len(), 64);
    }

    #[test]
    fn sr25519_signature_is_verifiable() {
        let s = Sr25519Signer::from_seed_hex(&"cd".repeat(32)).unwrap();
        let msg = b"test message";
        let sig = s.sign(msg);
        assert!(s.verify(msg, &sig), "signature must verify against same key");
    }

    #[test]
    fn sr25519_large_payload_pre_hashed() {
        let s = Sr25519Signer::from_seed_hex(&"ef".repeat(32)).unwrap();
        let msg = vec![0u8; 300]; // > 256 bytes → Blake2b pre-hash path
        let sig = s.sign(&msg);
        assert_eq!(sig.len(), 64);
        assert!(s.verify(&msg, &sig));
    }

    #[test]
    fn sr25519_different_messages_different_sigs() {
        let s = Sr25519Signer::from_seed_hex(&"ff".repeat(32)).unwrap();
        let s1 = s.sign(b"msg1");
        let s2 = s.sign(b"msg2");
        assert_ne!(s1, s2);
    }

    // ── read_u64_le ───────────────────────────────────────────────────────

    #[test]
    fn read_u64_le_at_offset_8() {
        let mut data = vec![0u8; 16];
        data[8..16].copy_from_slice(&999u64.to_le_bytes());
        assert_eq!(read_u64_le(&data, 8), Some(999));
    }

    #[test]
    fn read_u64_le_at_offset_0() {
        let data = 12345u64.to_le_bytes().to_vec();
        assert_eq!(read_u64_le(&data, 0), Some(12345));
    }

    #[test]
    fn read_u64_le_oob_returns_none() {
        assert_eq!(read_u64_le(&[0u8; 4], 4), None);
        assert_eq!(read_u64_le(&[], 0), None);
        assert_eq!(read_u64_le(&[0u8; 7], 0), None); // 7 < 8
    }

    // ── base64_decode ─────────────────────────────────────────────────────

    #[test]
    fn base64_decode_known_vector() {
        assert_eq!(base64_decode("aGVsbG8="), b"hello");
    }

    #[test]
    fn base64_decode_empty() {
        assert_eq!(base64_decode(""), b"" as &[u8]);
    }

    #[test]
    fn base64_decode_no_padding() {
        // "Man" → "TWFu" (no padding needed for 3 bytes)
        assert_eq!(base64_decode("TWFu"), b"Man");
    }

    // ── safe_str_prefix — panic regression for todo-4 ────────────────────
    // These tests reproduce the exact byte-boundary patterns that would have
    // caused a panic with the old `&acc.pubkey[..8.min(acc.pubkey.len())]`
    // approach when a hostile Solana RPC returns non-ASCII UTF-8 pubkeys.

    #[test]
    fn safe_str_prefix_pure_ascii_longer_than_max() {
        // Normal Solana base58 pubkey — always ASCII, always safe.
        let s = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf8Ss623VQ5DA";
        assert_eq!(safe_str_prefix(s, 8), "Tokenkeg");
    }

    #[test]
    fn safe_str_prefix_pure_ascii_shorter_than_max() {
        assert_eq!(safe_str_prefix("abc", 8), "abc");
    }

    #[test]
    fn safe_str_prefix_empty_string() {
        assert_eq!(safe_str_prefix("", 8), "");
    }

    #[test]
    fn safe_str_prefix_multibyte_straddles_byte8_case1() {
        // "aaaaaa\u{1234}x": U+1234 = 3-byte seq at bytes 6-8; byte 8 = 0x88 (cont).
        // The old byte-slice would panic; safe_str_prefix must return 8 chars.
        let s = "aaaaaa\u{1234}x";
        let got = safe_str_prefix(s, 8);
        assert_eq!(got.chars().count(), 8);
        assert_eq!(got, "aaaaaa\u{1234}x");
    }

    #[test]
    fn safe_str_prefix_multibyte_straddles_byte8_case2() {
        // "aaaaaaa\u{1234}": U+1234 starts at byte 7; byte 8 = 0xe1 (lead byte).
        // The old byte-slice would panic; safe_str_prefix must return 8 chars.
        let s = "aaaaaaa\u{1234}x";
        let got = safe_str_prefix(s, 8);
        assert_eq!(got.chars().count(), 8);
        assert_eq!(got, "aaaaaaa\u{1234}");
    }

    #[test]
    fn safe_str_prefix_four_byte_emoji_at_boundary() {
        // "abcd🦀cde": 4 ASCII + 1 emoji (4 bytes) + 3 ASCII = 8 chars, 11 bytes.
        // The emoji occupies bytes 4-7; the old byte-slice at byte 8 would land
        // on 'c' (ASCII, safe), but any byte-based cut inside the 4-byte emoji
        // would have panicked.  Verify char-count and content are correct.
        let s = "abcd\u{1F980}cde";  // 8 chars, 11 bytes
        let got = safe_str_prefix(s, 8);
        assert_eq!(got.chars().count(), 8);
        assert_eq!(got, "abcd\u{1F980}cde");
    }

    #[test]
    fn safe_str_prefix_max_chars_zero() {
        assert_eq!(safe_str_prefix("hello", 0), "");
    }

    #[test]
    fn safe_str_prefix_exact_length() {
        assert_eq!(safe_str_prefix("exactly8", 8), "exactly8");
    }

    // ── MetaCache refresh ─────────────────────────────────────────────────

    #[tokio::test]
    async fn meta_cache_returns_initial_value() {
        let meta = ChainMeta { spec_version: 42, tx_version: 7, genesis_hash: [1u8; 32] };
        let cache = MetaCache::new(meta, 300, "http://127.0.0.1:19944".to_string());
        // Should return spec_version=42 without hitting any network.
        // Set last_refresh to "now" so it won't try to refresh.
        let client = reqwest::Client::new();
        // Force TTL not expired (default 300s hasn't elapsed in 1ms).
        assert_eq!(cache.spec_version().await, 42);
    }

    // ── E2E poll_once tests ───────────────────────────────────────────────

    #[tokio::test]
    async fn poll_once_skips_account_below_finality() {
        use axum::{routing::post, Json, Router};

        async fn handler(Json(req): Json<serde_json::Value>) -> Json<serde_json::Value> {
            Json(match req["method"].as_str().unwrap_or("") {
                "getSlot" => serde_json::json!({"jsonrpc":"2.0","id":1,"result":31u64}),
                "getProgramAccounts" => {
                    let mut d = vec![0u8; 16];
                    d[8..16].copy_from_slice(&0u64.to_le_bytes()); // created_slot=0
                    // Global check: current_slot(31) < 32 → skip
                    serde_json::json!({"jsonrpc":"2.0","id":1,"result":[{
                        "pubkey":"Skip1111","account":{"lamports":1,"data":[b64_enc(&d),"base64"]}
                    }]})
                }
                _ => serde_json::json!({"jsonrpc":"2.0","id":1,"result":null}),
            })
        }

        let addr = spawn_mock_server(handler).await;
        let client = reqwest::Client::new();
        let rpc = format!("http://{}", addr);
        let cfg = BridgeConfig { solana_chain_id: 2, pallet_index: 26, call_index: 4 };
        let meta = ChainMeta { spec_version: 1, tx_version: 1, genesis_hash: [0u8; 32] };
        let cache = MetaCache::new(meta, 9999, rpc.clone());
        let ctx = PollerContext { solana_rpc: &rpc, x3_rpc: "http://127.0.0.1:19944", escrow_program: "FakeProg", service_id: "t", cfg: &cfg };
        poll_once(&client, &ctx, None, &cache).await;
        // No panic = pass; submission not reached
    }

    #[tokio::test]
    async fn poll_once_submits_mature_account_unsigned() {
        use axum::{extract::State, routing::post, Json, Router};
        use std::sync::{Arc, Mutex};

        let calls: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(vec![]));
        let calls_clone = calls.clone();

        async fn handler(
            State(calls): State<Arc<Mutex<Vec<String>>>>,
            Json(req): Json<serde_json::Value>,
        ) -> Json<serde_json::Value> {
            let m = req["method"].as_str().unwrap_or("").to_string();
            calls.lock().unwrap().push(m.clone());
            Json(match m.as_str() {
                "getSlot" => serde_json::json!({"jsonrpc":"2.0","id":1,"result":200u64}),
                "getProgramAccounts" => {
                    let mut d = vec![0u8; 16];
                    d[8..16].copy_from_slice(&10u64.to_le_bytes()); // age = 190 ≥ 32
                    serde_json::json!({"jsonrpc":"2.0","id":1,"result":[{
                        "pubkey":"MaturePubkey11","account":{"lamports":1,"data":[b64_enc(&d),"base64"]}
                    }]})
                }
                "author_submitExtrinsic" => {
                    serde_json::json!({"jsonrpc":"2.0","id":1,"result":"0xdeadbeef"})
                }
                _ => serde_json::json!({"jsonrpc":"2.0","id":1,"result":null}),
            })
        }

        let app = Router::new().route("/", post(handler)).with_state(calls_clone);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

        let rpc = format!("http://{}", addr);
        let client = reqwest::Client::new();
        let cfg  = BridgeConfig { solana_chain_id: 2, pallet_index: 26, call_index: 4 };
        let meta = ChainMeta { spec_version: 1, tx_version: 1, genesis_hash: [0u8; 32] };
        let cache = MetaCache::new(meta, 9999, rpc.clone());
        let ctx = PollerContext { solana_rpc: &rpc, x3_rpc: &rpc, escrow_program: "FakeProg", service_id: "t", cfg: &cfg };
        poll_once(&client, &ctx, None, &cache).await;

        let seen = calls.lock().unwrap();
        assert!(seen.contains(&"author_submitExtrinsic".to_string()),
            "expected author_submitExtrinsic; got {:?}", *seen);
    }

    #[tokio::test]
    async fn poll_once_handles_solana_rpc_failure_gracefully() {
        // Point at a port that's not listening — should log a warning, not panic.
        let client = reqwest::Client::builder().timeout(Duration::from_millis(200)).build().unwrap();
        let cfg  = BridgeConfig { solana_chain_id: 2, pallet_index: 26, call_index: 4 };
        let meta = ChainMeta { spec_version: 1, tx_version: 1, genesis_hash: [0u8; 32] };
        let cache = MetaCache::new(meta, 9999, "http://127.0.0.1:19944".to_string());
        let ctx = PollerContext {
            solana_rpc: "http://127.0.0.1:19944", x3_rpc: "http://127.0.0.1:19944",
            escrow_program: "FakeProg", service_id: "t", cfg: &cfg,
        };
        // Must not panic or unwrap-fail.
        poll_once(&client, &ctx, None, &cache).await;
    }

    /// Live Solana devnet integration test.
    /// Run: `X3_SOLANA_RPC_URL=https://api.devnet.solana.com cargo test live_solana -- --ignored`
    #[tokio::test]
    #[ignore = "requires live Solana devnet (set X3_SOLANA_RPC_URL)"]
    async fn live_solana_get_slot_returns_nonzero() {
        let rpc = std::env::var("X3_SOLANA_RPC_URL")
            .unwrap_or_else(|_| "https://api.devnet.solana.com".to_string());
        let client = reqwest::Client::new();
        let slot = solana_get_slot(&client, &rpc).await.expect("getSlot must succeed");
        assert!(slot > 0, "slot must be non-zero on live devnet");
    }

    // ── Helpers ───────────────────────────────────────────────────────────

    fn decode_compact_val(bytes: &[u8]) -> usize {
        match bytes[0] & 0b11 {
            0 => (bytes[0] >> 2) as usize,
            1 => (u16::from_le_bytes([bytes[0], bytes[1]]) >> 2) as usize,
            2 => (u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) >> 2) as usize,
            _ => panic!("big-integer compact in test"),
        }
    }

    async fn spawn_mock_server<F, Fut>(handler: F) -> std::net::SocketAddr
    where
        F: Fn(axum::extract::Json<serde_json::Value>) -> Fut + Clone + Send + 'static,
        Fut: std::future::Future<Output = axum::extract::Json<serde_json::Value>> + Send,
    {
        use axum::{routing::post, Router};
        let app = Router::new().route("/", post(handler));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        addr
    }

    pub fn b64_enc(data: &[u8]) -> String {
        const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut o = String::new();
        let mut i = 0;
        while i < data.len() {
            let a = data[i] as u32;
            let b = if i+1 < data.len() { data[i+1] as u32 } else { 0 };
            let c = if i+2 < data.len() { data[i+2] as u32 } else { 0 };
            let t = (a << 16) | (b << 8) | c;
            o.push(T[((t >> 18) & 63) as usize] as char);
            o.push(T[((t >> 12) & 63) as usize] as char);
            o.push(if i+1 < data.len() { T[((t >> 6) & 63) as usize] as char } else { '=' });
            o.push(if i+2 < data.len() { T[(t & 63) as usize] as char } else { '=' });
            i += 3;
        }
        o
    }
}
