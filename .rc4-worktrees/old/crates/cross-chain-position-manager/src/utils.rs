//! Utility functions and helpers for cross-chain position management
//!
//! This module provides:
//! - Common utility functions
//! - Type conversions
//! - Validation helpers
//! - Math utilities
//! - Time utilities

use crate::error::{PositionManagerError, Result};
use crate::types::{PositionId, H160, H256, U256};
use sp_std::vec;
use sp_std::vec::Vec;

/// Math utilities for position calculations
pub mod math {
    use super::*;

    /// Calculate percentage of a value
    pub fn percentage_of(value: U256, percentage: f64) -> Result<U256> {
        let result = value
            .checked_mul(U256::from((percentage * 100.0) as u128))
            .ok_or_else(|| PositionManagerError::ArithmeticOverflow)?
            .checked_div(U256::from(10000))
            .ok_or_else(|| PositionManagerError::ArithmeticOverflow)?;
        Ok(result)
    }

    /// Calculate percentage difference between two values
    pub fn percentage_diff(value1: U256, value2: U256) -> Result<f64> {
        if value1.is_zero() && value2.is_zero() {
            return Ok(0.0);
        }

        let max_val = if value1 > value2 { value1 } else { value2 };
        let min_val = if value1 < value2 { value1 } else { value2 };

        let diff = max_val - min_val;
        let percentage = diff.as_u128() as f64 / max_val.as_u128() as f64;

        Ok(percentage)
    }

    /// Calculate compound interest
    pub fn compound_interest(principal: U256, rate: f64, periods: u64) -> Result<U256> {
        let rate_decimal = rate / 100.0;
        let compound_factor = (1.0 + rate_decimal).powi(periods as i32);
        let result = principal
            .checked_mul(U256::from((compound_factor * 1e18) as u128))
            .ok_or_else(|| PositionManagerError::ArithmeticOverflow)?
            .checked_div(U256::from(1e18 as u128))
            .ok_or_else(|| PositionManagerError::ArithmeticOverflow)?;
        Ok(result)
    }

    /// Calculate average of values
    pub fn average(values: &[U256]) -> Result<U256> {
        if values.is_empty() {
            return Ok(U256::zero());
        }

        let sum: U256 = values
            .iter()
            .fold(U256::zero(), |acc, &x| acc.saturating_add(x));
        let avg = sum
            .checked_div(U256::from(values.len()))
            .ok_or_else(|| PositionManagerError::ArithmeticOverflow)?;
        Ok(avg)
    }

    /// Calculate standard deviation
    pub fn standard_deviation(values: &[U256]) -> Result<f64> {
        if values.len() < 2 {
            return Ok(0.0);
        }

        let avg = average(values)?;
        let avg_f64 = avg.as_u128() as f64;

        let variance: f64 = values
            .iter()
            .map(|&x| {
                let diff = x.as_u128() as f64 - avg_f64;
                diff * diff
            })
            .sum::<f64>()
            / (values.len() - 1) as f64;

        Ok(variance.sqrt())
    }

    /// Calculate Sharpe ratio
    pub fn sharpe_ratio(returns: &[f64], risk_free_rate: f64) -> Result<f64> {
        if returns.is_empty() {
            return Ok(0.0);
        }

        let avg_return: f64 = returns.iter().sum::<f64>() / returns.len() as f64;
        let excess_return = avg_return - risk_free_rate;

        let variance: f64 = returns
            .iter()
            .map(|&r| {
                let diff = r - avg_return;
                diff * diff
            })
            .sum::<f64>()
            / returns.len() as f64;

        let std_dev = variance.sqrt();

        if std_dev == 0.0 {
            return Ok(0.0);
        }

        Ok(excess_return / std_dev)
    }

    /// Calculate maximum drawdown
    pub fn max_drawdown(values: &[U256]) -> Result<f64> {
        if values.len() < 2 {
            return Ok(0.0);
        }

        let mut max_drawdown = 0.0;
        let mut peak = values[0].as_u128() as f64;

        for &value in values.iter().skip(1) {
            let current = value.as_u128() as f64;
            if current > peak {
                peak = current;
            } else {
                let drawdown = (peak - current) / peak;
                if drawdown > max_drawdown {
                    max_drawdown = drawdown;
                }
            }
        }

        Ok(max_drawdown)
    }

    /// Calculate weighted average
    pub fn weighted_average(values: &[(U256, U256)]) -> Result<U256> {
        if values.is_empty() {
            return Ok(U256::zero());
        }

        let mut total_weight = U256::zero();
        let mut weighted_sum = U256::zero();

        for &(value, weight) in values {
            weighted_sum = weighted_sum
                .checked_add(value.checked_mul(weight).unwrap_or(U256::zero()))
                .ok_or_else(|| PositionManagerError::ArithmeticOverflow)?;
            total_weight = total_weight
                .checked_add(weight)
                .ok_or_else(|| PositionManagerError::ArithmeticOverflow)?;
        }

        if total_weight.is_zero() {
            return Ok(U256::zero());
        }

        weighted_sum
            .checked_div(total_weight)
            .ok_or_else(|| PositionManagerError::ArithmeticOverflow)
    }
}

/// Time utilities for position management
pub mod time {
    use super::*;

    /// Get current timestamp in milliseconds
    pub fn current_timestamp_ms() -> u64 {
        sp_io::offchain::timestamp().unix_millis()
    }

    /// Get current timestamp in seconds
    pub fn current_timestamp_secs() -> u64 {
        sp_io::offchain::timestamp().unix_millis() / 1000
    }

    /// Check if a timestamp is expired
    pub fn is_expired(timestamp: u64, ttl_ms: u64) -> bool {
        let now = current_timestamp_ms();
        now > timestamp + ttl_ms
    }

    /// Calculate time remaining until expiration
    pub fn time_until_expiration(timestamp: u64, ttl_ms: u64) -> Option<u64> {
        let now = current_timestamp_ms();
        let expiration = timestamp + ttl_ms;
        if now >= expiration {
            None
        } else {
            Some(expiration - now)
        }
    }

    /// Format duration as human-readable string
    pub fn format_duration(ms: u64) -> String {
        if ms < 1000 {
            format!("{}ms", ms)
        } else if ms < 60_000 {
            format!("{:.1}s", ms as f64 / 1000.0)
        } else if ms < 3600_000 {
            format!("{:.1}m", ms as f64 / 60_000.0)
        } else {
            format!("{:.1}h", ms as f64 / 3_600_000.0)
        }
    }

    /// Get timestamp for start of day
    pub fn start_of_day(timestamp: u64) -> u64 {
        let secs = timestamp / 1000;
        let days = secs / 86400;
        days * 86400 * 1000
    }

    /// Get timestamp for end of day
    pub fn end_of_day(timestamp: u64) -> u64 {
        start_of_day(timestamp) + 86400 * 1000 - 1
    }
}

/// Validation utilities
pub mod validation {
    use super::*;

    /// Validate address is not zero
    pub fn validate_non_zero_address(address: &H160) -> Result<()> {
        if address == &H160::zero() {
            return Err(PositionManagerError::InvalidAddress(
                "Address cannot be zero".to_string(),
            ));
        }
        Ok(())
    }

    /// Validate amount is positive
    pub fn validate_positive_amount(amount: &U256) -> Result<()> {
        if amount.is_zero() {
            return Err(PositionManagerError::InvalidAmount(
                "Amount must be positive".to_string(),
            ));
        }
        Ok(())
    }

    /// Validate amount is within bounds
    pub fn validate_amount_bounds(
        amount: &U256,
        min: Option<&U256>,
        max: Option<&U256>,
    ) -> Result<()> {
        if let Some(min_val) = min {
            if amount < min_val {
                return Err(PositionManagerError::InvalidAmount(format!(
                    "Amount {} is below minimum {}",
                    amount, min_val
                )));
            }
        }

        if let Some(max_val) = max {
            if amount > max_val {
                return Err(PositionManagerError::InvalidAmount(format!(
                    "Amount {} exceeds maximum {}",
                    amount, max_val
                )));
            }
        }

        Ok(())
    }

    /// Validate chain ID is supported
    pub fn validate_chain_id(chain_id: u64, supported_chains: &[u64]) -> Result<()> {
        if !supported_chains.contains(&chain_id) {
            return Err(PositionManagerError::UnsupportedChain(chain_id));
        }
        Ok(())
    }

    /// Validate percentage is in range [0, 100]
    pub fn validate_percentage(percentage: f64) -> Result<()> {
        if percentage < 0.0 || percentage > 100.0 {
            return Err(PositionManagerError::InvalidPercentage(percentage));
        }
        Ok(())
    }

    /// Validate slippage is in range [0, 1]
    pub fn validate_slippage(slippage: f64) -> Result<()> {
        if slippage < 0.0 || slippage > 1.0 {
            return Err(PositionManagerError::InvalidSlippage(slippage));
        }
        Ok(())
    }

    /// Validate deadline is in the future
    pub fn validate_deadline(deadline: u64) -> Result<()> {
        let now = time::current_timestamp_ms();
        if deadline <= now {
            return Err(PositionManagerError::DeadlineExpired);
        }
        Ok(())
    }
}

/// Conversion utilities
pub mod conversion {
    use super::*;

    /// Convert U256 to f64 with decimal precision
    pub fn u256_to_f64(value: &U256, decimals: u32) -> Result<f64> {
        let divisor = U256::from(10).pow(U256::from(decimals));
        let integer_part = value
            .checked_div(divisor)
            .ok_or_else(|| PositionManagerError::ArithmeticOverflow)?;
        let remainder = value
            .checked_rem(divisor)
            .ok_or_else(|| PositionManagerError::ArithmeticOverflow)?;

        let integer_f64 = integer_part.as_u128() as f64;
        let fractional_f64 = remainder.as_u128() as f64 / divisor.as_u128() as f64;

        Ok(integer_f64 + fractional_f64)
    }

    /// Convert f64 to U256 with decimal precision
    pub fn f64_to_u256(value: f64, decimals: u32) -> Result<U256> {
        let multiplier = U256::from(10).pow(U256::from(decimals));
        let scaled = value * multiplier.as_u128() as f64;
        Ok(U256::from(scaled as u128))
    }

    /// Convert bytes to hex string
    pub fn bytes_to_hex(bytes: &[u8]) -> String {
        hex::encode(bytes)
    }

    /// Convert hex string to bytes
    pub fn hex_to_bytes(hex: &str) -> Result<Vec<u8>> {
        hex::decode(hex)
            .map_err(|e| PositionManagerError::ConversionError(format!("Invalid hex: {}", e)))
    }

    /// Convert H256 to hex string
    pub fn h256_to_hex(hash: &H256) -> String {
        format!("{:?}", hash)
    }

    /// Convert H160 to hex string
    pub fn h160_to_hex(address: &H160) -> String {
        format!("{:?}", address)
    }
}

/// Hash utilities
pub mod hash {
    use super::*;
    use sp_core::Hasher;
    use sp_runtime::traits::BlakeTwo256;

    /// Hash multiple values together
    pub fn hash_values(values: &[&[u8]]) -> H256 {
        let mut hasher = BlakeTwo256::default();
        for value in values {
            hasher.hash(value);
        }
        H256::from_slice(hasher.finish().as_ref())
    }

    /// Hash a string
    pub fn hash_string(s: &str) -> H256 {
        hash_values(&[s.as_bytes()])
    }

    /// Hash a position ID with a timestamp
    pub fn hash_position_with_time(position_id: &PositionId, timestamp: u64) -> H256 {
        hash_values(&[position_id.as_bytes(), &timestamp.to_le_bytes()])
    }

    /// Generate a unique ID
    pub fn generate_unique_id(prefix: &str, timestamp: u64) -> H256 {
        hash_values(&[
            prefix.as_bytes(),
            &timestamp.to_le_bytes(),
            &sp_io::offchain::random_seed(),
        ])
    }
}

/// Collection utilities
pub mod collections {
    use super::*;

    /// Group items by a key
    pub fn group_by<K, V, F>(
        items: Vec<V>,
        key_fn: F,
    ) -> sp_std::collections::btree_map::BTreeMap<K, Vec<V>>
    where
        K: Ord,
        F: Fn(&V) -> K,
    {
        let mut map = sp_std::collections::btree_map::BTreeMap::new();
        for item in items {
            let key = key_fn(&item);
            map.entry(key).or_insert_with(Vec::new).push(item);
        }
        map
    }

    /// Flatten nested collections
    pub fn flatten<T>(nested: Vec<Vec<T>>) -> Vec<T> {
        nested.into_iter().flatten().collect()
    }

    /// Remove duplicates while preserving order
    pub fn dedup<T: Clone + Ord>(items: &[T]) -> Vec<T> {
        let mut seen = sp_std::collections::btree_set::BTreeSet::new();
        let mut result = Vec::new();
        for item in items {
            if seen.insert(item.clone()) {
                result.push(item.clone());
            }
        }
        result
    }

    /// Chunk a collection into smaller pieces
    pub fn chunk<T: Clone>(items: &[T], size: usize) -> Vec<Vec<T>> {
        items.chunks(size).map(|chunk| chunk.to_vec()).collect()
    }
}

/// Retry utilities
pub mod retry {
    use super::*;

    /// Retry configuration
    #[derive(Debug, Clone)]
    pub struct RetryConfig {
        pub max_attempts: u32,
        pub initial_delay_ms: u64,
        pub max_delay_ms: u64,
        pub exponential_backoff: bool,
    }

    impl Default for RetryConfig {
        fn default() -> Self {
            Self {
                max_attempts: 3,
                initial_delay_ms: 1000,
                max_delay_ms: 30000,
                exponential_backoff: true,
            }
        }
    }

    /// Retry result
    pub enum RetryResult<T> {
        Success(T),
        Retry(u32, PositionManagerError),
        Failed(PositionManagerError),
    }

    /// Execute a function with retry logic
    pub async fn with_retry<F, Fut, T>(config: &RetryConfig, mut f: F) -> Result<T>
    where
        F: FnMut() -> Fut,
        Fut: core::future::Future<Output = RetryResult<T>>,
    {
        let mut attempt = 0;

        loop {
            attempt += 1;

            match f().await {
                RetryResult::Success(value) => return Ok(value),
                RetryResult::Failed(error) => return Err(error),
                RetryResult::Retry(current_attempt, error) => {
                    if current_attempt >= config.max_attempts {
                        return Err(error);
                    }

                    let delay = if config.exponential_backoff {
                        let exp_delay = config.initial_delay_ms * 2u64.pow(current_attempt - 1);
                        exp_delay.min(config.max_delay_ms)
                    } else {
                        config.initial_delay_ms
                    };

                    // In a real implementation, we'd use tokio::time::sleep
                    // For now, we just continue
                }
            }
        }
    }
}

/// Logging utilities
pub mod logging {
    use super::*;

    /// Log level
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    pub enum LogLevel {
        Error,
        Warn,
        Info,
        Debug,
        Trace,
    }

    /// Log a message
    pub fn log(level: LogLevel, message: &str, context: Option<&str>) {
        let timestamp = time::current_timestamp_ms();
        let prefix = match level {
            LogLevel::Error => "ERROR",
            LogLevel::Warn => "WARN",
            LogLevel::Info => "INFO",
            LogLevel::Debug => "DEBUG",
            LogLevel::Trace => "TRACE",
        };

        let context_str = context.map(|c| format!(" [{}]", c)).unwrap_or_default();
        tracing::log!(
            match level {
                LogLevel::Error => tracing::Level::ERROR,
                LogLevel::Warn => tracing::Level::WARN,
                LogLevel::Info => tracing::Level::INFO,
                LogLevel::Debug => tracing::Level::DEBUG,
                LogLevel::Trace => tracing::Level::TRACE,
            },
            "[{}] {}{} - {}",
            timestamp,
            prefix,
            context_str,
            message
        );
    }

    /// Log error
    pub fn error(message: &str, context: Option<&str>) {
        log(LogLevel::Error, message, context);
    }

    /// Log warning
    pub fn warn(message: &str, context: Option<&str>) {
        log(LogLevel::Warn, message, context);
    }

    /// Log info
    pub fn info(message: &str, context: Option<&str>) {
        log(LogLevel::Info, message, context);
    }

    /// Log debug
    pub fn debug(message: &str, context: Option<&str>) {
        log(LogLevel::Debug, message, context);
    }

    /// Log trace
    pub fn trace(message: &str, context: Option<&str>) {
        log(LogLevel::Trace, message, context);
    }
}

/// Rate limiting utilities
pub mod rate_limit {
    use super::*;

    /// Rate limiter
    #[derive(Debug, Clone)]
    pub struct RateLimiter {
        pub max_requests: u32,
        pub window_ms: u64,
        pub requests: Vec<u64>,
    }

    impl RateLimiter {
        /// Create a new rate limiter
        pub fn new(max_requests: u32, window_ms: u64) -> Self {
            Self {
                max_requests,
                window_ms,
                requests: Vec::new(),
            }
        }

        /// Check if a request is allowed
        pub fn allow_request(&mut self) -> bool {
            let now = time::current_timestamp_ms();
            let window_start = now - self.window_ms;

            // Remove old requests
            self.requests.retain(|&timestamp| timestamp > window_start);

            // Check if we can allow this request
            if self.requests.len() < self.max_requests as usize {
                self.requests.push(now);
                true
            } else {
                false
            }
        }

        /// Get remaining requests in current window
        pub fn remaining_requests(&self) -> u32 {
            let now = time::current_timestamp_ms();
            let window_start = now - self.window_ms;

            let current_requests = self
                .requests
                .iter()
                .filter(|&&timestamp| timestamp > window_start)
                .count();

            self.max_requests.saturating_sub(current_requests as u32)
        }

        /// Get time until next request is allowed
        pub fn time_until_allowed(&self) -> Option<u64> {
            if self.remaining_requests() > 0 {
                return Some(0);
            }

            let now = time::current_timestamp_ms();
            let window_start = now - self.window_ms;

            self.requests
                .iter()
                .filter(|&&timestamp| timestamp > window_start)
                .min()
                .map(|&oldest| {
                    let time_since_oldest = now - oldest;
                    if time_since_oldest < self.window_ms {
                        self.window_ms - time_since_oldest
                    } else {
                        0
                    }
                })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_percentage_of() {
        let value = U256::from(1000);
        let result = math::percentage_of(value, 10.0).unwrap();
        assert_eq!(result, U256::from(100));
    }

    #[test]
    fn test_percentage_diff() {
        let value1 = U256::from(1000);
        let value2 = U256::from(1100);
        let result = math::percentage_diff(value1, value2).unwrap();
        assert!((result - 0.09090909090909091).abs() < 1e-10);
    }

    #[test]
    fn test_is_expired() {
        let timestamp = time::current_timestamp_ms() - 1000;
        assert!(time::is_expired(timestamp, 500));
        assert!(!time::is_expired(timestamp, 2000));
    }

    #[test]
    fn test_validate_non_zero_address() {
        let zero = H160::zero();
        let non_zero = H160::random();

        assert!(validation::validate_non_zero_address(&zero).is_err());
        assert!(validation::validate_non_zero_address(&non_zero).is_ok());
    }

    #[test]
    fn test_rate_limiter() {
        let mut limiter = rate_limit::RateLimiter::new(2, 1000);

        assert!(limiter.allow_request());
        assert!(limiter.allow_request());
        assert!(!limiter.allow_request());
    }
}
