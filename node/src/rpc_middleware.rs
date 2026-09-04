/// RPC Rate Limiting and Security Middleware for X3 Chain
///
/// Provides per-connection and per-method rate limiting to prevent DoS attacks
/// and abuse of RPC endpoints.
///
/// Connection identity is injected by the RPC server layer and consumed by
/// handler-level checks to enforce per-connection method limits.
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

/// Rate limit configuration
#[derive(Clone, Debug)]
pub struct RateLimitConfig {
    /// Maximum requests per second per connection
    pub requests_per_second: u32,
    /// Maximum concurrent subscriptions per connection
    pub max_subscriptions_per_connection: u32,
    /// Burst allowance (requests can spike up to this)
    pub burst_size: u32,
    /// Per-method rate limits (method -> requests per minute)
    pub method_limits: HashMap<String, u32>,
    /// Default requests per minute for unlisted methods
    pub default_method_limit: u32,
    /// Ban duration after rate limit exceeded
    pub ban_duration: Duration,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        let mut method_limits = HashMap::new();
        // Heavy methods get stricter limits
        method_limits.insert("eth_call".to_string(), 100); // 100/min
        method_limits.insert("eth_estimateGas".to_string(), 60); // 60/min
        method_limits.insert("atlasKernel_getCanonicalBalance".to_string(), 300); // 300/min
        method_limits.insert("atlasKernel_getAssetMetadata".to_string(), 600); // 600/min
        method_limits.insert("atlasKernel_isAuthorized".to_string(), 600); // 600/min
        method_limits.insert("atlasKernel_getAuthorizedAccounts".to_string(), 120); // 120/min
        method_limits.insert("atlasKernel_getAuthorities".to_string(), 120); // 120/min
        method_limits.insert("x3Domains_getRecords".to_string(), 300); // 300/min
        method_limits.insert("x3Domains_getDomain".to_string(), 120); // 120/min
        method_limits.insert("x3Domains_listDomains".to_string(), 120); // 120/min
        method_limits.insert("atomicTrade_simulate".to_string(), 30); // 30/min (expensive)
        method_limits.insert("atomicTrade_estimateCost".to_string(), 120); // 120/min
        method_limits.insert("atomicTrade_getPriceData".to_string(), 300); // 300/min
        method_limits.insert("atomicTrade_getBatchStatus".to_string(), 300); // 300/min
        method_limits.insert("atomicTrade_isAuthorized".to_string(), 600); // 600/min
        method_limits.insert("x3_findBestPath".to_string(), 60); // 60/min
        method_limits.insert("evolutionCore_getParams".to_string(), 180); // 180/min
        method_limits.insert("evolutionCore_getStatus".to_string(), 180); // 180/min
        method_limits.insert("evolutionCore_getMetrics".to_string(), 120); // 120/min
        method_limits.insert("evolutionCore_getPendingProposals".to_string(), 120); // 120/min
        method_limits.insert("evolutionCore_isEnabled".to_string(), 600); // 600/min
        method_limits.insert("evolutionCore_isAiAgent".to_string(), 300); // 300/min
        method_limits.insert("x3Verifier_getStatus".to_string(), 300); // 300/min
        method_limits.insert("x3Verifier_getExecutor".to_string(), 300); // 300/min
        method_limits.insert("x3Verifier_getActiveExecutors".to_string(), 120); // 120/min
        method_limits.insert("x3Verifier_getJob".to_string(), 180); // 180/min
        method_limits.insert("x3Verifier_getPendingJobs".to_string(), 120); // 120/min
        method_limits.insert("x3Verifier_getReceipt".to_string(), 180); // 180/min
        method_limits.insert("x3Verifier_isEnabled".to_string(), 600); // 600/min
        method_limits.insert("x3Verifier_isExecutor".to_string(), 600); // 600/min
        method_limits.insert("eth_chainId".to_string(), 600); // 600/min
        method_limits.insert("eth_gasPrice".to_string(), 600); // 600/min
        method_limits.insert("eth_blockNumber".to_string(), 600); // 600/min
        method_limits.insert("x3Node_getRateLimitMetrics".to_string(), 120); // 120/min

        Self {
            requests_per_second: 50,
            max_subscriptions_per_connection: 10,
            burst_size: 100,
            method_limits,
            default_method_limit: 600, // 10/sec default
            ban_duration: Duration::from_secs(60),
        }
    }
}

/// Per-connection rate limit state
struct ConnectionState {
    /// Token bucket for overall rate limiting
    tokens: AtomicU64,
    /// Last token refill time
    last_refill: RwLock<Instant>,
    /// Per-method request counts (method -> (count, window_start))
    method_counts: RwLock<HashMap<String, (u32, Instant)>>,
    /// Active subscription count
    subscriptions: AtomicU64,
    /// Ban expiry time (if banned)
    banned_until: RwLock<Option<Instant>>,
}

impl ConnectionState {
    fn new(burst_size: u32) -> Self {
        Self {
            tokens: AtomicU64::new(burst_size as u64),
            last_refill: RwLock::new(Instant::now()),
            method_counts: RwLock::new(HashMap::new()),
            subscriptions: AtomicU64::new(0),
            banned_until: RwLock::new(None),
        }
    }
}

/// Rate limiter state shared across connections
pub struct RateLimiter {
    config: RateLimitConfig,
    connections: RwLock<HashMap<u64, Arc<ConnectionState>>>,
    /// Global request counter for metrics
    total_requests: AtomicU64,
    /// Global rejected counter for metrics
    total_rejected: AtomicU64,
}

impl RateLimiter {
    /// Construct a shared rate limiter with the given configuration.
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            config,
            connections: RwLock::new(HashMap::new()),
            total_requests: AtomicU64::new(0),
            total_rejected: AtomicU64::new(0),
        }
    }

    /// Get or create connection state
    fn get_connection_state(
        &self,
        connection_id: u64,
    ) -> Result<Arc<ConnectionState>, RateLimitError> {
        let mut conns = self
            .connections
            .write()
            .map_err(|_| RateLimitError::InternalStateCorrupted)?;
        Ok(conns
            .entry(connection_id)
            .or_insert_with(|| Arc::new(ConnectionState::new(self.config.burst_size)))
            .clone())
    }

    /// Check if a request should be allowed
    pub fn check_request(&self, connection_id: u64, method: &str) -> Result<(), RateLimitError> {
        self.total_requests.fetch_add(1, Ordering::Relaxed);

        let state = self.get_connection_state(connection_id)?;

        // Check if banned
        let banned_until = *state
            .banned_until
            .read()
            .map_err(|_| RateLimitError::InternalStateCorrupted)?;
        if let Some(banned_until) = banned_until {
            if Instant::now() < banned_until {
                self.total_rejected.fetch_add(1, Ordering::Relaxed);
                return Err(RateLimitError::Banned);
            } else {
                *state
                    .banned_until
                    .write()
                    .map_err(|_| RateLimitError::InternalStateCorrupted)? = None;
            }
        }

        // Refill tokens based on elapsed time
        self.refill_tokens(&state)?;

        // Try to consume a token
        let tokens = state.tokens.load(Ordering::Relaxed);
        if tokens == 0 {
            // Rate limited - apply ban
            *state
                .banned_until
                .write()
                .map_err(|_| RateLimitError::InternalStateCorrupted)? =
                Some(Instant::now() + self.config.ban_duration);
            self.total_rejected.fetch_add(1, Ordering::Relaxed);
            return Err(RateLimitError::TooManyRequests);
        }
        state.tokens.fetch_sub(1, Ordering::Relaxed);

        // Check per-method limit
        self.check_method_limit(&state, method)?;

        Ok(())
    }

    /// Refill tokens based on elapsed time
    fn refill_tokens(&self, state: &ConnectionState) -> Result<(), RateLimitError> {
        let mut last_refill = state
            .last_refill
            .write()
            .map_err(|_| RateLimitError::InternalStateCorrupted)?;
        let now = Instant::now();
        let elapsed = now.duration_since(*last_refill);

        if elapsed >= Duration::from_millis(100) {
            // Refill tokens (rate per 100ms)
            let refill_rate = self.config.requests_per_second as u64 / 10;
            let periods = elapsed.as_millis() as u64 / 100;
            let new_tokens = periods * refill_rate;

            let current = state.tokens.load(Ordering::Relaxed);
            let max = self.config.burst_size as u64;
            state
                .tokens
                .store((current + new_tokens).min(max), Ordering::Relaxed);
            *last_refill = now;
        }

        Ok(())
    }

    /// Check per-method rate limit
    fn check_method_limit(
        &self,
        state: &ConnectionState,
        method: &str,
    ) -> Result<(), RateLimitError> {
        let limit = self
            .config
            .method_limits
            .get(method)
            .copied()
            .unwrap_or(self.config.default_method_limit);

        let mut counts = state
            .method_counts
            .write()
            .map_err(|_| RateLimitError::InternalStateCorrupted)?;
        let now = Instant::now();
        let window = Duration::from_secs(60);

        let (count, window_start) = counts.entry(method.to_string()).or_insert((0, now));

        // Reset window if expired
        if now.duration_since(*window_start) >= window {
            *count = 0;
            *window_start = now;
        }

        if *count >= limit {
            self.total_rejected.fetch_add(1, Ordering::Relaxed);
            return Err(RateLimitError::MethodLimitExceeded);
        }

        *count += 1;
        Ok(())
    }

    /// Track subscription count
    pub fn add_subscription(&self, connection_id: u64) -> Result<(), RateLimitError> {
        let state = self.get_connection_state(connection_id)?;
        let current = state.subscriptions.fetch_add(1, Ordering::Relaxed);

        if current >= self.config.max_subscriptions_per_connection as u64 {
            state.subscriptions.fetch_sub(1, Ordering::Relaxed);
            return Err(RateLimitError::TooManySubscriptions);
        }

        Ok(())
    }

    /// Remove subscription tracking
    pub fn remove_subscription(&self, connection_id: u64) {
        if let Ok(state) = self.get_connection_state(connection_id) {
            state.subscriptions.fetch_sub(1, Ordering::Relaxed);
        }
    }

    /// Get metrics snapshot
    pub fn metrics(&self) -> RateLimitMetrics {
        RateLimitMetrics {
            total_requests: self.total_requests.load(Ordering::Relaxed),
            total_rejected: self.total_rejected.load(Ordering::Relaxed),
            active_connections: self
                .connections
                .read()
                .map(|conns| conns.len())
                .unwrap_or(0),
        }
    }

    /// Cleanup stale connections (call periodically)
    pub fn cleanup_stale_connections(&self, max_age: Duration) {
        let Ok(mut conns) = self.connections.write() else {
            return;
        };
        let now = Instant::now();

        conns.retain(|_, state| {
            state
                .last_refill
                .read()
                .map(|last| now.duration_since(*last) < max_age)
                .unwrap_or(false)
        });
    }
}

/// Rate limit error types
#[derive(Debug, Clone)]
pub enum RateLimitError {
    /// Too many requests from a single source.
    TooManyRequests,
    /// Specific RPC method exceeded its rate cap.
    MethodLimitExceeded,
    /// Subscription count limit exceeded for the session.
    TooManySubscriptions,
    /// Temporarily banned due to repeated violations.
    Banned,
    /// Internal synchronization state was poisoned.
    InternalStateCorrupted,
}

impl std::fmt::Display for RateLimitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooManyRequests => write!(f, "Rate limit exceeded"),
            Self::MethodLimitExceeded => write!(f, "Method rate limit exceeded"),
            Self::TooManySubscriptions => write!(f, "Too many active subscriptions"),
            Self::Banned => write!(f, "Temporarily banned due to rate limit violation"),
            Self::InternalStateCorrupted => write!(f, "Rate limiter internal state corrupted"),
        }
    }
}

/// Metrics for monitoring
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RateLimitMetrics {
    /// Total RPC requests evaluated by the rate limiter.
    pub total_requests: u64,
    /// RPC requests rejected due to rate limiting.
    pub total_rejected: u64,
    /// Current number of active RPC connections.
    pub active_connections: usize,
}

/// CORS configuration for RPC server
#[derive(Clone, Debug)]
pub struct CorsConfig {
    /// Allowed origins (None = no CORS, Some(vec![]) = block all, Some(origins) = allow listed)
    pub allowed_origins: Option<Vec<String>>,
    /// Allow credentials
    pub allow_credentials: bool,
    /// Max age for preflight cache
    pub max_age: Option<u32>,
}

impl Default for CorsConfig {
    fn default() -> Self {
        Self {
            // Default: only allow localhost for development
            allowed_origins: Some(vec![
                "http://localhost:3000".to_string(),
                "http://127.0.0.1:3000".to_string(),
                "https://explorer.x3-chain.io".to_string(),
                "https://dex.x3-chain.io".to_string(),
            ]),
            allow_credentials: false,
            max_age: Some(3600),
        }
    }
}

impl CorsConfig {
    /// Production config with strict origins
    pub fn production(allowed_origins: Vec<String>) -> Self {
        Self {
            allowed_origins: Some(allowed_origins),
            allow_credentials: false,
            max_age: Some(86400),
        }
    }

    /// Development config allowing localhost
    pub fn development() -> Self {
        Self::default()
    }

    /// Check if origin is allowed
    pub fn is_origin_allowed(&self, origin: &str) -> bool {
        match &self.allowed_origins {
            None => true,                                 // No CORS restrictions
            Some(origins) if origins.is_empty() => false, // Block all
            Some(origins) => origins.iter().any(|o| o == origin || o == "*"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limiter_basic() {
        let limiter = RateLimiter::new(RateLimitConfig {
            requests_per_second: 10,
            burst_size: 5,
            ..Default::default()
        });

        let connection_id = 42_u64;

        // First 5 requests should succeed (burst)
        for _ in 0..5 {
            assert!(limiter.check_request(connection_id, "test_method").is_ok());
        }

        // 6th should fail (exceeded burst)
        assert!(limiter.check_request(connection_id, "test_method").is_err());
    }

    #[test]
    fn test_cors_config() {
        let cors = CorsConfig::default();
        assert!(cors.is_origin_allowed("http://localhost:3000"));
        assert!(!cors.is_origin_allowed("http://evil.com"));
    }

    #[test]
    fn test_subscription_limits() {
        let limiter = RateLimiter::new(RateLimitConfig {
            max_subscriptions_per_connection: 2,
            ..Default::default()
        });

        let connection_id = 42_u64;

        assert!(limiter.add_subscription(connection_id).is_ok());
        assert!(limiter.add_subscription(connection_id).is_ok());
        assert!(limiter.add_subscription(connection_id).is_err()); // 3rd should fail

        limiter.remove_subscription(connection_id);
        assert!(limiter.add_subscription(connection_id).is_ok()); // Now should work
    }

    #[test]
    fn test_connection_isolation_for_rate_limit() {
        let limiter = RateLimiter::new(RateLimitConfig {
            requests_per_second: 1,
            burst_size: 1,
            ban_duration: Duration::from_secs(60),
            ..Default::default()
        });

        let connection_a = 1_u64;
        let connection_b = 2_u64;

        assert!(limiter
            .check_request(connection_a, "atomicTrade_estimateCost")
            .is_ok());
        assert!(limiter
            .check_request(connection_a, "atomicTrade_estimateCost")
            .is_err());

        // Second connection must remain independent.
        assert!(limiter
            .check_request(connection_b, "atomicTrade_estimateCost")
            .is_ok());
    }
}
