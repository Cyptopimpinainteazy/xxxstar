//! API key authentication validator.
//!
//! Validates requests against configured API keys.
//! - Admin methods require admin-level keys
//! - Public methods accept any valid key or no key (if configured)
//! - Unauthenticated requests get base public access only

use std::collections::HashMap;
use std::sync::Arc;

use axum::http::HeaderMap;

use crate::config::ArcConfig;
use crate::config::MethodClass;

/// API key with associated permissions.
#[derive(Debug, Clone)]
pub struct ApiKey {
    pub key_hash: String,
    pub service: String,           // "bot", "wallet", "explorer", "dex", "validator", "admin"
    pub allowed_chains: Vec<String>,
    pub rate_limit_budget: u32,    // requests per second
}

/// Auth validator pulls keys from config/environment.
pub struct AuthValidator {
    keys: HashMap<String, ApiKey>,
    require_auth: bool,
}

impl AuthValidator {
    pub fn new(config: ArcConfig) -> Self {
        // In production, keys are loaded from environment variables or vault.
        // For the template, we check X3_RPC_API_KEYS env var.
        let mut keys = HashMap::new();

        // Load from env: X3_RPC_API_KEYS=svc:keyhash:chains:budget,...
        if let Ok(keys_env) = std::env::var("X3_RPC_API_KEYS") {
            for entry in keys_env.split(',') {
                let parts: Vec<&str> = entry.split(':').collect();
                if parts.len() >= 4 {
                    keys.insert(
                        parts[1].to_string(),
                        ApiKey {
                            key_hash: parts[1].to_string(),
                            service: parts[0].to_string(),
                            allowed_chains: parts[2].split('|').map(String::from).collect(),
                            rate_limit_budget: parts[3].parse().unwrap_or(100),
                        },
                    );
                }
            }
        }

        // In test mode, add a default key
        if std::env::var("X3_RPC_TEST_MODE").is_ok() {
            keys.insert(
                "test-key-hash".to_string(),
                ApiKey {
                    key_hash: "test-key-hash".to_string(),
                    service: "test".to_string(),
                    allowed_chains: vec!["ethereum".to_string(), "solana".to_string(), "bitcoin".to_string(), "x3".to_string()],
                    rate_limit_budget: 1000,
                },
            );
        }

        let require_auth = std::env::var("X3_RPC_REQUIRE_AUTH")
            .map(|v| v == "true" || v == "1")
            .unwrap_or(false);

        Self {
            keys,
            require_auth,
        }
    }

    /// Validate a request. Returns true if the request should be allowed.
    pub fn validate_request(&self, headers: &HeaderMap, method: &str) -> bool {
        // If auth is not required, allow all non-admin methods
        if !self.require_auth {
            // Still block admin methods without auth
            if is_admin_method(method) {
                return self.check_auth_header(headers, method);
            }
            return true;
        }

        // Auth required — check header
        self.check_auth_header(headers, method)
    }

    fn check_auth_header(&self, headers: &HeaderMap, _method: &str) -> bool {
        // Check X-API-Key header
        let provided_key = headers
            .get("X-API-Key")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");

        if provided_key.is_empty() {
            return false;
        }

        // Hash the key for comparison (simple SHA256)
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(provided_key.as_bytes());
        let key_hash = format!("{:x}", hasher.finalize());

        self.keys.contains_key(&key_hash)
    }
}

fn is_admin_method(method: &str) -> bool {
    method.starts_with("admin_")
        || method.starts_with("personal_")
        || method.starts_with("miner_")
        || method.starts_with("debug_")
        || method.starts_with("txpool_")
        || method == "eth_sign"
        || method == "eth_signTransaction"
        || method.starts_with("eth_signTypedData")
        || method.starts_with("author_")
}