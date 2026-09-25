use serde::{Deserialize, Serialize};
use std::{fs, io::Read, net::SocketAddr, path::PathBuf, sync::Arc, thread};
use tiny_http::{Header, Method, Response, Server};
use tokio::sync::Mutex;

use rand::RngCore;

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce}; // AES-GCM
use argon2::Argon2;
use base64::{engine::general_purpose, Engine as _};
use dashmap::DashMap;
use dirs::config_dir;
use once_cell::sync::Lazy;
use sha2::Digest;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewardPoint {
    pub t: u64,
    pub rewards: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminState {
    pub enabled: bool,
    pub gpu_level: String, // e.g., low, medium, high or percentage
    pub schedule_on: Option<String>,
    pub schedule_off: Option<String>,
    pub rewards: f64,
    pub uptime_seconds: u64,
    pub score: u32,
    pub last_updated: u64,

    // new fields
    pub rewards_history: Vec<RewardPoint>,
    pub wallet_address: Option<String>,
}

impl Default for AdminState {
    fn default() -> Self {
        Self {
            enabled: true,
            gpu_level: "medium".to_string(),
            schedule_on: None,
            schedule_off: None,
            rewards: 0.0,
            uptime_seconds: 0,
            score: 100,
            last_updated: 0,
            rewards_history: Vec::new(),
            wallet_address: None,
        }
    }
}

pub type SharedState = Arc<Mutex<AdminState>>;

// In-memory session store (session_id -> expiry_ts)
static SESSIONS: Lazy<DashMap<String, u64>> = Lazy::new(|| DashMap::new());
const SESSION_TTL_SECS: u64 = 60 * 60; // 1 hour

fn create_session() -> (String, u64) {
    let id = Uuid::new_v4().to_string();
    let expires = chrono::Utc::now().timestamp() as u64 + SESSION_TTL_SECS;
    SESSIONS.insert(id.clone(), expires);
    (id, expires)
}

fn session_matches(provided: &str) -> bool {
    if let Some(exp) = SESSIONS.get(provided) {
        let now = chrono::Utc::now().timestamp() as u64;
        if *exp > now {
            return true;
        } else {
            SESSIONS.remove(provided);
        }
    }
    false
}

/// Ensure we have a config directory, return it
pub fn ensure_config_dir() -> PathBuf {
    let base = config_dir().unwrap_or_else(|| {
        let mut p = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        p.push(".config");
        p
    });
    let dir = base.join("gpu-swarm");
    if !dir.exists() {
        let _ = fs::create_dir_all(&dir);
    }
    dir
}

/// Ensure a local admin token file exists, return the token string
pub fn ensure_local_token() -> std::io::Result<String> {
    let dir = ensure_config_dir();
    let token_file = dir.join("admin.token");
    if token_file.exists() {
        let s = fs::read_to_string(token_file)?;
        return Ok(s.trim().to_string());
    }

    let mut rnd = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut rnd);
    let token = hex::encode(rnd);
    fs::write(&token_file, &token)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&token_file)?.permissions();
        perms.set_mode(0o600);
        fs::set_permissions(&token_file, perms)?;
    }

    Ok(token)
}

// Encrypt a 32-byte secret key using Argon2-derived key (from admin token) + AES-GCM
fn encrypt_private_key(secret: &[u8; 32], token: &str) -> anyhow::Result<String> {
    // Argon2 derive 32 byte key; prototype parameters
    let mut salt_bytes = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut salt_bytes);
    let mut out = [0u8; 32];
    Argon2::default()
        .hash_password_into(token.as_bytes(), &salt_bytes, &mut out)
        .map_err(|e| anyhow::anyhow!("argon2 failed: {}", e))?;

    let cipher =
        Aes256Gcm::new_from_slice(&out).map_err(|e| anyhow::anyhow!("aes init error: {}", e))?;

    let mut nonce_bytes = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ct = cipher
        .encrypt(nonce, secret.as_ref())
        .map_err(|e| anyhow::anyhow!("aes encrypt: {}", e))?;
    // store as base64: salt | nonce | ct
    let b = format!(
        "{}:{}:{}",
        general_purpose::STANDARD.encode(&salt_bytes),
        general_purpose::STANDARD.encode(&nonce_bytes),
        general_purpose::STANDARD.encode(&ct)
    );
    Ok(b)
}

// Decrypt private key given the token
fn decrypt_private_key(enc: &str, token: &str) -> anyhow::Result<[u8; 32]> {
    let parts: Vec<&str> = enc.split(':').collect();
    if parts.len() != 3 {
        return Err(anyhow::anyhow!("invalid format"));
    }
    let salt_b64 = parts[0];
    let nonce_b64 = parts[1];
    let ct_b64 = parts[2];

    // decode salt
    let salt_bytes = general_purpose::STANDARD.decode(salt_b64)?;
    // derive key via Argon2 into 32 bytes
    let mut out = [0u8; 32];
    Argon2::default()
        .hash_password_into(token.as_bytes(), &salt_bytes, &mut out)
        .map_err(|e| anyhow::anyhow!("argon2 failed: {}", e))?;

    let cipher =
        Aes256Gcm::new_from_slice(&out).map_err(|e| anyhow::anyhow!("aes init error: {}", e))?;
    let nonce_bytes = general_purpose::STANDARD.decode(nonce_b64)?;
    let ct = general_purpose::STANDARD.decode(ct_b64)?;
    let nonce = Nonce::from_slice(&nonce_bytes);
    let pt = cipher
        .decrypt(nonce, ct.as_ref())
        .map_err(|e| anyhow::anyhow!("aes decrypt: {}", e))?;
    let arr: [u8; 32] = pt
        .try_into()
        .map_err(|_| anyhow::anyhow!("invalid plaintext length"))?;
    Ok(arr)
}

fn token_matches(provided: &str) -> bool {
    if let Ok(local) = ensure_local_token() {
        local.trim() == provided.trim()
    } else {
        false
    }
}

fn config_path() -> PathBuf {
    ensure_config_dir().join("node-config.toml")
}

fn save_admin_config(s: &AdminState) -> anyhow::Result<()> {
    let path = config_path();
    let tmp = path.with_extension("tmp");
    let toml_str = toml::to_string(&s)?;
    fs::write(&tmp, toml_str)?;
    fs::rename(tmp, &path)?;
    Ok(())
}

fn load_admin_config() -> Option<AdminState> {
    let path = config_path();
    if path.exists() {
        if let Ok(contents) = fs::read_to_string(path) {
            if let Ok(s) = toml::from_str::<AdminState>(&contents) {
                return Some(s);
            }
        }
    }
    None
}

pub async fn run_admin(state: SharedState, listen_addr: SocketAddr) {
    // ensure token exists at startup
    match ensure_local_token() {
        Ok(path) => tracing::info!("Local admin token ready (keep secret): {}", path),
        Err(e) => tracing::warn!("Unable to ensure admin token: {}", e),
    }

    // If we have a persisted config, load it into state
    if let Some(cfg) = load_admin_config() {
        let mut s = state.lock().await;
        *s = cfg;
    }

    let addr = listen_addr;
    // Capture runtime handle so the thread can use the existing Tokio runtime
    let runtime_handle = tokio::runtime::Handle::current();
    thread::spawn(move || {
        let server = match Server::http(addr) {
            Ok(s) => s,
            Err(e) => {
                tracing::error!("Failed to start admin server: {}", e);
                return;
            }
        };
        for mut request in server.incoming_requests() {
            let url = request.url().to_string();
            let method = request.method().clone();

            // Serve SPA
            if url == "/" && method == Method::Get {
                let html = include_str!("../static/index.html");
                let response = Response::from_string(html).with_header(
                    Header::from_bytes(&b"Content-Type"[..], &b"text/html; charset=utf-8"[..])
                        .unwrap(),
                );
                let _ = request.respond(response);
                continue;
            }

            // Static assets (very small set)
            if url.starts_with("/static/") && method == Method::Get {
                let path = url.trim_start_matches('/');
                let data = match path {
                    "static/app.js" => include_str!("../static/app.js").to_string(),
                    "static/style.css" => include_str!("../static/style.css").to_string(),
                    _ => "".to_string(),
                };
                if !data.is_empty() {
                    let ct = if path.ends_with(".js") {
                        "application/javascript"
                    } else if path.ends_with(".css") {
                        "text/css"
                    } else {
                        "text/plain"
                    };
                    let response = Response::from_string(data).with_header(
                        Header::from_bytes(&b"Content-Type"[..], ct.as_bytes()).unwrap(),
                    );
                    let _ = request.respond(response);
                    continue;
                }
            }

            // API: GET /api/state
            if url == "/api/state" && method == Method::Get {
                let s = runtime_fetch_state(&state);
                let body = serde_json::to_string(&s).unwrap_or_else(|_| "{}".to_string());
                let response = Response::from_string(body).with_header(
                    Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap(),
                );
                let _ = request.respond(response);
                continue;
            }

            // API: POST /api/login  { token }
            if url == "/api/login" && method == Method::Post {
                let mut content = String::new();
                let reader = request.as_reader();
                let _ = reader.read_to_string(&mut content);
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(t) = v.get("token").and_then(|t| t.as_str()) {
                        if token_matches(t) {
                            // create a session token with expiry
                            let (session_id, expires) = create_session();
                            let body = serde_json::json!({"ok": true, "session": session_id, "expires": expires}).to_string();
                            let response = Response::from_string(body).with_header(
                                Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..])
                                    .unwrap(),
                            );
                            let _ = request.respond(response);
                            continue;
                        }
                    }
                }
                let response = Response::from_string("{\"ok\":false}")
                    .with_status_code(401)
                    .with_header(
                        Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap(),
                    );
                let _ = request.respond(response);
                continue;
            }

            // API: POST /api/register  -> generates BIP39 mnemonic + address + encrypted private key
            if url == "/api/register" && method == Method::Post {
                // Use BIP39 generator
                let mnemonic = crate::bip39::generate_mnemonic_12();
                let seed = crate::bip39::mnemonic_to_seed(&mnemonic, "");
                let mut sk_bytes = [0u8; 32];
                sk_bytes.copy_from_slice(&seed[0..32]);
                // For prototype derive an address deterministically from the seed (sha256)
                let address = hex::encode(sha2::Sha256::digest(&seed));

                // Encrypt private key with local admin token (best-effort)
                match ensure_local_token() {
                    Ok(tok) => {
                        if let Ok(enc) = encrypt_private_key(&sk_bytes, &tok) {
                            let _ = fs::write(ensure_config_dir().join("wallet.key"), enc);
                        }
                    }
                    Err(e) => tracing::warn!("Unable to ensure token for encryption: {}", e),
                }

                // Persist wallet address
                let address_for_cfg = address.clone();
                let cloned = state.clone();
                runtime_handle.block_on(async move {
                    let mut s = cloned.lock().await;
                    s.wallet_address = Some(address_for_cfg.clone());
                    s.last_updated = chrono::Utc::now().timestamp() as u64;
                    let _ = save_admin_config(&s);
                });

                let response_body = serde_json::json!({
                    "mnemonic": mnemonic,
                    "address": address,
                })
                .to_string();
                let response = Response::from_string(response_body).with_header(
                    Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap(),
                );
                let _ = request.respond(response);
                continue;
            }

            // API: POST /api/state  (requires Authorization: Bearer <token>)
            if url == "/api/state" && method == Method::Post {
                // auth
                let auth_header = request
                    .headers()
                    .iter()
                    .find(|h| h.field.equiv("Authorization"));
                let mut authorized = false;
                if let Some(h) = auth_header {
                    if let Ok(txt) = std::str::from_utf8(h.value.as_ref()) {
                        if txt.starts_with("Bearer ") {
                            let provided = txt.trim_start_matches("Bearer ").trim();
                            if token_matches(provided) || session_matches(provided) {
                                authorized = true;
                            }
                        }
                    }
                }

                if !authorized {
                    let response =
                        Response::from_string("{\"ok\":false,\"error\":\"unauthorized\"}")
                            .with_status_code(401)
                            .with_header(
                                Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..])
                                    .unwrap(),
                            );
                    let _ = request.respond(response);
                    continue;
                }

                let mut content = String::new();
                let reader = request.as_reader();
                let _ = reader.read_to_string(&mut content);
                if let Ok(payload) = serde_json::from_str::<AdminState>(&content) {
                    // clone for safe reuse
                    let payload_clone = payload.clone();
                    // update shared state
                    let cloned = state.clone();
                    runtime_handle.block_on(async move {
                        let mut s = cloned.lock().await;
                        s.enabled = payload.enabled;
                        s.gpu_level = payload.gpu_level.clone();
                        s.schedule_on = payload.schedule_on.clone();
                        s.schedule_off = payload.schedule_off.clone();
                        s.last_updated = chrono::Utc::now().timestamp() as u64;
                        // save each update
                        let _ = save_admin_config(&s);
                    });
                    let body =
                        serde_json::to_string(&payload_clone).unwrap_or_else(|_| "{}".to_string());
                    let response = Response::from_string(body).with_header(
                        Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap(),
                    );
                    let _ = request.respond(response);
                    continue;
                } else {
                    let response = Response::from_string("invalid json").with_header(
                        Header::from_bytes(&b"Content-Type"[..], &b"text/plain"[..]).unwrap(),
                    );
                    let _ = request.respond(response);
                    continue;
                }
            }

            // default 404
            let response = Response::from_string("Not Found").with_status_code(404);
            let _ = request.respond(response);
        }
    });
}

fn runtime_fetch_state(state: &SharedState) -> AdminState {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    rt.block_on(async {
        let s = state.lock().await.clone();
        s
    })
}

/// Simple prototype mnemonic generator: picks 12 words from a built-in small wordlist.
fn generate_mnemonic() -> String {
    // Note: this is a prototype and NOT full BIP39-compliant. Replace with a proper BIP39
    // implementation for production use.
    const WORDS: &[&str] = &[
        "abandon", "ability", "able", "about", "above", "absent", "absorb", "abstract", "absurd",
        "abuse", "access", "accident", "account", "accuse", "achieve", "acid", "acoustic",
        "acquire", "across", "act", "action", "actor", "actress", "actual", "adapt", "add",
        "addict", "address", "adjust", "admit", "adult", "advance", "advice", "aerobic", "affair",
        "afford", "afraid", "again", "age", "agent", "agree", "ahead", "aim", "air", "airport",
        "aisle", "alarm", "album", "alcohol", "alert", "alien", "all", "alley", "allow", "almost",
        "alone", "alpha", "already", "also", "alter", "always", "amateur", "amazing", "among",
        "amount", "amused", "analyst", "anchor", "ancient", "anger", "angle", "angry", "animal",
        "ankle", "announce", "annual", "another", "answer", "antenna", "antique", "anxiety", "any",
        "apart", "apology", "appear", "apple", "approve", "april", "arch", "arctic", "area",
        "arena", "argue", "arm", "armed", "armor", "army", "around", "arrange", "arrest", "arrive",
        "arrow", "art", "artefact", "artist", "artwork", "ask", "aspect", "assault", "asset",
        "assist", "assume", "asthma", "athlete", "atom", "attack", "attend", "attitude", "attract",
        "auction", "audit", "august", "aunt", "author", "auto", "autumn", "average", "avocado",
        "avoid", "awake", "aware", "away", "awesome", "awful", "awkward", "axis", "baby",
        "bachelor", "bacon", "badge", "bag", "balance", "balcony", "ball", "bamboo", "banana",
        "banner", "bar", "barely", "bargain", "barrel", "base", "basic", "basket", "battle",
        "beach", "bean", "beauty", "because", "become", "beef", "before", "begin", "behave",
        "behind", "believe", "below", "belt", "bench", "benefit", "best", "betray", "better",
        "between", "beyond", "bicycle", "bid", "bike", "bind", "biology", "bird", "birth",
        "bitter", "black", "blade", "blame", "blanket", "blast", "bleak", "bless", "blind",
        "blood", "blossom", "blouse", "blue", "blur", "blush", "board", "boat", "body", "boil",
        "bomb", "bone", "bonus", "book", "boost", "border", "boring", "borrow", "boss", "bottom",
        "bounce", "box", "boy", "bracket", "brain", "brand", "brass", "brave", "bread", "breeze",
        "brick", "bridge", "brief", "bright", "bring", "brisk", "broccoli", "broken", "bronze",
        "broom", "brother", "brown", "brush", "bubble", "buddy", "budget", "buffalo", "build",
        "bulb", "bulk", "bullet", "bundle", "bunker", "burden", "burger", "burst", "bus",
        "business", "busy", "butter", "buyer", "buzz", "cabbage", "cabin", "cable", "cactus",
        "cage", "cake", "call", "calm", "camera", "camp", "can", "canal", "cancel", "candy",
        "cannon", "canoe", "canvas", "canyon", "cap", "capable", "capital", "captain", "car",
        "carbon", "card", "cargo", "carpet", "carry", "cart", "case", "cash", "casino", "castle",
        "casual", "cat", "catalog",
    ];

    let mut rng = rand::thread_rng();
    let mut words: Vec<String> = Vec::with_capacity(12);
    for _ in 0..12 {
        let idx = (rng.next_u32() as usize) % WORDS.len();
        words.push(WORDS[idx].to_string());
    }
    words.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mnemonic_has_12_words() {
        let m = generate_mnemonic();
        assert_eq!(m.split_whitespace().count(), 12);
    }
}
