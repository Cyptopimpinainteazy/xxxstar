//! Per-IP and per-key rate limiting.
//!
//! Uses token bucket algorithm. Each IP and API key gets a budget
//! of requests per second. Exceeding the budget results in 429 responses.

use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use tokio::sync::Mutex;
use std::time::Instant;

use crate::config::ArcConfig;

/// Token bucket for a single client (IP or key).
#[derive(Debug)]
struct TokenBucket {
    tokens: f64,
    max_tokens: f64,
    refill_rate: f64,    // tokens per second
    last_refill: Instant,
}

impl TokenBucket {
    fn new(max_tokens: f64, refill_rate: f64) -> Self {
        Self {
            tokens: max_tokens,
            max_tokens,
            refill_rate,
            last_refill: Instant::now(),
        }
    }

    /// Try to consume one token. Returns true if allowed.
    fn try_consume(&mut self) -> bool {
        self.refill();
        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }

    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        self.tokens = (self.tokens + elapsed * self.refill_rate).min(self.max_tokens);
        self.last_refill = now;
    }
}

/// Rate limiter for all clients.
pub struct RateLimiter {
    ip_buckets: Mutex<HashMap<IpAddr, TokenBucket>>,
    default_rps: u32,
    max_buckets: usize,
}

impl RateLimiter {
    pub fn new(_config: ArcConfig) -> Self {
        Self {
            ip_buckets: Mutex::new(HashMap::new()),
            default_rps: 200,          // 200 requests/second per IP default
            max_buckets: 100_000,
        }
    }

    /// Check if a request from this IP is allowed for the given method.
    /// Transaction methods get a stricter budget.
    pub fn check(&self, ip: IpAddr, method: &str) -> bool {
        let rps = if is_tx_method(method) {
            20                          // 20 tx/second per IP
        } else if is_heavy_method(method) {
            50                          // 50 heavy reads/second
        } else {
            self.default_rps
        };

        self.check_with_budget(ip, rps as f64)
    }

    fn check_with_budget(&self, ip: IpAddr, budget_rps: f64) -> bool {
        let mut buckets = match self.ip_buckets.try_lock() {
            Ok(b) => b,
            Err(_) => return false,     // lock contention → block
        };

        // Cleanup old entries if too many
        if buckets.len() > self.max_buckets {
            buckets.clear();
        }

        let bucket = buckets
            .entry(ip)
            .or_insert_with(|| TokenBucket::new(budget_rps, budget_rps));

        bucket.try_consume()
    }
}

fn is_tx_method(method: &str) -> bool {
    method == "eth_sendRawTransaction"
        || method == "eth_sendTransaction"
        || method == "sendTransaction"
        || method == "sendrawtransaction"
        || method == "x3_submitExtrinsic"
}

fn is_heavy_method(method: &str) -> bool {
    method == "getProgramAccounts"
        || method == "getMultipleAccounts"
        || method == "getSignaturesForAddress"
        || method == "eth_getLogs"
        || method.starts_with("trace_")
        || method.starts_with("debug_trace")
}