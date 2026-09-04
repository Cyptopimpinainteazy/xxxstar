//! Configuration loading and method classification.
//!
//! Loads chains.yaml, methods.yaml, and providers.yaml at startup.
//! Provides the canonical method classification and chain resolution.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use regex::RegexSet;
use serde::{Deserialize, Serialize};

/// Shared configuration handle.
pub type ArcConfig = Arc<AppConfig>;

// ── Chain Configuration ────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct ChainsFile {
    pub chains: HashMap<String, ChainConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ChainConfig {
    pub kind: ChainKind,
    pub chain_id: Option<u64>,
    pub network: String,
    pub finality: Option<String>,
    pub max_block_lag: u32,
    pub require_archive_for: Vec<String>,
    pub blocked_methods: Vec<String>,
    pub tx_broadcast: Option<TxBroadcastConfig>,
    pub quorum: Option<QuorumConfig>,
    pub cache_ttl_ms: Option<HashMap<String, u64>>,
    // Solana-specific
    pub max_slot_lag: Option<u32>,
    pub websocket_sticky: Option<bool>,
    pub ws_max_subscriptions_per_client: Option<u32>,
    pub ws_subscription_timeout_seconds: Option<u32>,
    pub require_private_for: Option<Vec<String>>,
    // Bitcoin-specific
    pub require_local_first: Option<bool>,
    pub health_method: Option<String>,
    pub max_header_lag: Option<u32>,
    pub safe_public: Option<Vec<String>>,
    // X3-specific
    pub require_quorum_for: Option<Vec<String>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ChainKind {
    Evm,
    Solana,
    Bitcoin,
    X3,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TxBroadcastConfig {
    pub mode: String,
    pub max_upstreams: u32,
    pub require_nonce_guard: Option<bool>,
    pub require_chain_id_check: Option<bool>,
    pub retry_delay_ms: Option<u64>,
    pub preflight: Option<bool>,
    pub max_retries: Option<u32>,
    pub require_fresh_blockhash: Option<bool>,
    pub blockhash_max_age_slots: Option<u32>,
    pub require_local_first: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct QuorumConfig {
    pub enabled: bool,
    pub min_agreement: u32,
    pub check_methods: Vec<String>,
}

// ── Methods Configuration ──────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct MethodsFile {
    pub evm: MethodGroup,
    pub solana: MethodGroup,
    pub bitcoin: MethodGroup,
    pub x3: MethodGroup,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MethodGroup {
    pub safe_public: Vec<String>,
    pub tx_methods: Vec<String>,
    #[serde(default)]
    pub archive_only: Vec<String>,
    #[serde(default)]
    pub blocked: Vec<String>,
    #[serde(default)]
    pub admin_authenticated: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MethodClass {
    SafePublic,
    TxMethod,
    ArchiveOnly,
    AdminAuthenticated,
    Blocked,
}

// ── Provider Configuration ─────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct ProvidersFile {
    pub providers: Vec<ProviderEntry>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProviderEntry {
    pub id: String,
    pub chain: String,
    pub kind: String,
    pub url: String,
    #[serde(default)]
    pub ws_url: Option<String>,
    pub tier: u8,
    pub priority: u8,
    #[serde(default)]
    pub capabilities: Vec<String>,
    #[serde(default)]
    pub rate_limit_rps: u32,
    #[serde(default)]
    pub labels: HashMap<String, String>,
    #[serde(default)]
    pub auth: Option<ProviderAuth>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProviderAuth {
    pub rpc_user: String,
    pub rpc_password: String,
}

// ── Full App Configuration ─────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub chains: HashMap<String, ChainConfig>,
    pub methods: MethodsFile,
    pub providers: Vec<ProviderEntry>,
    /// Index of chain name → providers sorted by (tier, priority)
    pub providers_by_chain: HashMap<String, Vec<ProviderEntry>>,
    /// Pre-built method classification lookups per chain
    method_matchers: HashMap<String, (RegexSet, Vec<MethodClass>)>,
    /// Method → chain mapping for all known methods
    method_chain_map: HashMap<String, String>,
}

impl AppConfig {
    /// Load configuration from the standard paths.
    pub fn load() -> anyhow::Result<Self> {
        let config_dir = std::env::var("X3_RPC_CONFIG_DIR")
            .unwrap_or_else(|_| "../".to_string());

        let chains_path = PathBuf::from(&config_dir).join("chains.yaml");
        let methods_path = PathBuf::from(&config_dir).join("methods.yaml");
        let providers_path = PathBuf::from(&config_dir).join("providers.yaml");

        // If providers.yaml doesn't exist, fall back to providers.yaml.example with stubs
        let providers_raw = if providers_path.exists() {
            fs::read_to_string(&providers_path)?
        } else {
            let example_path = PathBuf::from(&config_dir).join("providers.yaml.example");
            if example_path.exists() {
                fs::read_to_string(&example_path)?
            } else {
                "providers: []".to_string()
            }
        };

        let chains: ChainsFile = serde_yaml::from_str(&fs::read_to_string(&chains_path)?)?;
        let methods: MethodsFile = serde_yaml::from_str(&fs::read_to_string(&methods_path)?)?;
        let providers_raw_parsed: ProvidersFile = serde_yaml::from_str(&providers_raw)?;

        // Index providers by chain
        let mut providers_by_chain: HashMap<String, Vec<ProviderEntry>> = HashMap::new();
        for p in &providers_raw_parsed.providers {
            providers_by_chain
                .entry(p.chain.clone())
                .or_default()
                .push(p.clone());
        }
        // Sort each chain's providers by (tier, priority)
        for list in providers_by_chain.values_mut() {
            list.sort_by_key(|p| (p.tier, p.priority));
        }

        // Build method classification lookup
        let mut method_matchers = HashMap::new();
        let mut method_chain_map = HashMap::new();

        let chain_method_groups: Vec<(&str, &MethodGroup)> = vec![
            ("ethereum", &methods.evm),
            ("base", &methods.evm),
            ("arbitrum", &methods.evm),
            ("polygon", &methods.evm),
            ("bsc", &methods.evm),
            ("solana", &methods.solana),
            ("bitcoin", &methods.bitcoin),
            ("x3", &methods.x3),
        ];

        for (chain_name, group) in &chain_method_groups {
            let mut patterns = Vec::new();
            let mut classes = Vec::new();

            for method in &group.safe_public {
                patterns.push(wildcard_to_regex(method));
                classes.push(MethodClass::SafePublic);
                method_chain_map.insert(method.clone(), chain_name.to_string());
            }
            for method in &group.tx_methods {
                patterns.push(wildcard_to_regex(method));
                classes.push(MethodClass::TxMethod);
                method_chain_map.insert(method.clone(), chain_name.to_string());
            }
            for method in &group.archive_only {
                patterns.push(wildcard_to_regex(method));
                classes.push(MethodClass::ArchiveOnly);
                method_chain_map.insert(method.clone(), chain_name.to_string());
            }
            for method in &group.admin_authenticated {
                patterns.push(wildcard_to_regex(method));
                classes.push(MethodClass::AdminAuthenticated);
                method_chain_map.insert(method.clone(), chain_name.to_string());
            }
            for method in &group.blocked {
                patterns.push(wildcard_to_regex(method));
                classes.push(MethodClass::Blocked);
                method_chain_map.insert(method.clone(), chain_name.to_string());
            }

            if let Ok(set) = RegexSet::new(&patterns) {
                method_matchers.insert(chain_name.to_string(), (set, classes));
            }
        }

        Ok(Self {
            chains: chains.chains,
            methods,
            providers: providers_raw_parsed.providers,
            providers_by_chain,
            method_matchers,
            method_chain_map,
        })
    }

    /// Classify a method for a specific chain.
    pub fn classify_for_chain(&self, method: &str, chain: &str) -> MethodClass {
        if let Some((regex_set, classes)) = self.method_matchers.get(chain) {
            for idx in regex_set.matches(method) {
                return classes[idx].clone();
            }
        }
        // Default: block unknown methods
        MethodClass::Blocked
    }

    /// Classify a method across all chains (uses first matching chain).
    pub fn classify(&self, method: &str) -> MethodClass {
        for chain in self.method_matchers.keys() {
            let c = self.classify_for_chain(method, chain);
            if c != MethodClass::Blocked {
                return c;
            }
        }
        MethodClass::Blocked
    }

    /// Determine which chain a method belongs to.
    pub fn resolve_chain(&self, method: &str) -> ChainKind {
        // First check exact matches
        if let Some(chain_name) = self.method_chain_map.get(method) {
            if let Some(chain_cfg) = self.chains.get(chain_name) {
                return chain_cfg.kind;
            }
        }

        // Fall back to prefix-based resolution
        if method.starts_with("eth_") || method.starts_with("net_") || method.starts_with("web3_")
            || method.starts_with("trace_") || method.starts_with("txpool_")
        {
            return ChainKind::Evm;
        }
        if method.starts_with("get") || method.starts_with("send") || method.starts_with("simulate")
            || method.starts_with("is")
        {
            return ChainKind::Solana;
        }
        if method.starts_with("getblock") || method.starts_with("getraw")
            || method.starts_with("estimate") || method.starts_with("decoderaw")
            || method.starts_with("sendraw") || method.starts_with("testmempool")
        {
            return ChainKind::Bitcoin;
        }
        if method.starts_with("x3_") || method.starts_with("author_")
            || method.starts_with("offchain_")
        {
            return ChainKind::X3;
        }

        // Default to EVM — it's the most common RPC pattern
        ChainKind::Evm
    }

    /// Get providers for a chain, sorted by tier then priority.
    pub fn providers_for(&self, chain: &str) -> Vec<&ProviderEntry> {
        self.providers_by_chain
            .get(chain)
            .map(|v| v.iter().collect())
            .unwrap_or_default()
    }

    /// Is this method one that requires archive node capabilities?
    pub fn requires_archive(&self, method: &str, chain: &str) -> bool {
        if let Some(cfg) = self.chains.get(chain) {
            cfg.require_archive_for.iter().any(|pattern| {
                wildcard_matches(pattern, method)
            })
        } else {
            false
        }
    }

    /// Is this method one that requires quorum verification?
    pub fn requires_quorum(&self, method: &str, chain: &str) -> bool {
        if let Some(cfg) = self.chains.get(chain) {
            if let Some(ref quorum_for) = cfg.require_quorum_for {
                quorum_for.iter().any(|p| wildcard_matches(p, method))
            } else {
                false
            }
        } else {
            false
        }
    }

    /// Is this method a transaction-sending method?
    pub fn is_tx_method(&self, method: &str) -> bool {
        matches!(self.classify(method), MethodClass::TxMethod)
    }
}

// ── Helpers ────────────────────────────────────────────────────────────────

/// Convert a method pattern (which may include wildcards like `trace_*`) to a regex.
fn wildcard_to_regex(pattern: &str) -> String {
    format!("^{}$", regex::escape(pattern).replace(r"\*", ".*"))
}

/// Check if a method name matches a wildcard pattern.
fn wildcard_matches(pattern: &str, method: &str) -> bool {
    let re_str = wildcard_to_regex(pattern);
    if let Ok(re) = regex::Regex::new(&re_str) {
        re.is_match(method)
    } else {
        pattern == method
    }
}