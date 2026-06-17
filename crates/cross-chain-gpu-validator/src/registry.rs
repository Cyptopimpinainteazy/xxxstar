//! Redis-backed atomic swap registry for cross-chain orchestration

use crate::error::{Result, ValidatorError};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum SwapPhase {
    Pending,
    ValidatingEvm,
    ValidatingSvm,
    ReadyCommit,
    Committed,
    RolledBack,
    TimedOut,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtomicSwapRecord {
    pub swap_id: String,
    pub phase: SwapPhase,
    pub evm_block: u64,
    pub svm_slot: u64,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub evm_validation_ok: bool,
    pub svm_validation_ok: bool,
}

impl AtomicSwapRecord {
    pub fn new(swap_id: String, timeout_secs: u64, evm_block: u64, svm_slot: u64) -> Self {
        let now = Utc::now();
        Self {
            swap_id,
            phase: SwapPhase::Pending,
            evm_block,
            svm_slot,
            created_at: now,
            expires_at: now + Duration::seconds(timeout_secs as i64),
            evm_validation_ok: false,
            svm_validation_ok: false,
        }
    }

    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }
}

/// Redis-backed atomic registry for swap state synchronization
pub struct AtomicRegistry {
    redis_client: redis::Client,
    ttl_secs: usize,
}

impl AtomicRegistry {
    pub async fn new(redis_url: &str, ttl_secs: usize) -> Result<Self> {
        let client = redis::Client::open(redis_url)
            .map_err(|e| ValidatorError::RedisError(e.to_string()))?;

        Ok(Self {
            redis_client: client,
            ttl_secs,
        })
    }

    pub async fn register_swap(&self, record: &AtomicSwapRecord) -> Result<()> {
        let conn = self
            .redis_client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| ValidatorError::RedisError(e.to_string()))?;

        let mut conn = conn;
        let key = format!("swap:{}", record.swap_id);
        let value = serde_json::to_string(record)?;

        redis::cmd("SET")
            .arg(&key)
            .arg(&value)
            .arg("EX")
            .arg(self.ttl_secs)
            .query_async::<()>(&mut conn)
            .await
            .map_err(|e| ValidatorError::RedisError(e.to_string()))?;

        Ok(())
    }

    pub async fn get_swap(&self, swap_id: &str) -> Result<Option<AtomicSwapRecord>> {
        let conn = self
            .redis_client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| ValidatorError::RedisError(e.to_string()))?;

        let mut conn = conn;
        let key = format!("swap:{swap_id}");

        let value: Option<String> = redis::cmd("GET")
            .arg(&key)
            .query_async(&mut conn)
            .await
            .map_err(|e| ValidatorError::RedisError(e.to_string()))?;

        match value {
            Some(v) => Ok(Some(serde_json::from_str(&v)?)),
            None => Ok(None),
        }
    }

    /// Create an in-memory registry for tests that don't need Redis.
    #[cfg(test)]
    pub fn new_in_memory() -> Self {
        // Connect to a placeholder — tests that construct orchestrators
        // directly skip registry calls via the test-only constructor.
        let client = redis::Client::open("redis://127.0.0.1:0").unwrap();
        Self {
            redis_client: client,
            ttl_secs: 60,
        }
    }

    pub async fn update_phase(&self, swap_id: &str, phase: SwapPhase) -> Result<()> {
        let mut record = self
            .get_swap(swap_id)
            .await?
            .ok_or_else(|| ValidatorError::InvalidSwapState(format!("Swap {swap_id} not found")))?;

        record.phase = phase;
        self.register_swap(&record).await?;
        Ok(())
    }

    pub async fn mark_evm_validated(&self, swap_id: &str, valid: bool) -> Result<()> {
        let mut record = self
            .get_swap(swap_id)
            .await?
            .ok_or_else(|| ValidatorError::InvalidSwapState(format!("Swap {swap_id} not found")))?;

        record.evm_validation_ok = valid;
        self.register_swap(&record).await?;
        Ok(())
    }

    pub async fn mark_svm_validated(&self, swap_id: &str, valid: bool) -> Result<()> {
        let mut record = self
            .get_swap(swap_id)
            .await?
            .ok_or_else(|| ValidatorError::InvalidSwapState(format!("Swap {swap_id} not found")))?;

        record.svm_validation_ok = valid;
        self.register_swap(&record).await?;
        Ok(())
    }

    /// Scan Redis for all keys matching `swap:*` and return pending tasks.
    /// Uses the SCAN command with a glob pattern to avoid blocking the server.
    pub async fn list_pending_swaps(&self) -> Result<Vec<crate::PendingValidationTask>> {
        let conn = self
            .redis_client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| ValidatorError::RedisError(e.to_string()))?;

        let mut conn = conn;
        let mut cursor: u64 = 0;
        let mut tasks = Vec::new();

        loop {
            let (next_cursor, keys): (u64, Vec<String>) = redis::cmd("SCAN")
                .arg(cursor)
                .arg("MATCH")
                .arg("swap:*")
                .arg("COUNT")
                .arg(100)
                .query_async(&mut conn)
                .await
                .map_err(|e| ValidatorError::RedisError(e.to_string()))?;

            for key in &keys {
                let value: Option<String> = redis::cmd("GET")
                    .arg(key)
                    .query_async(&mut conn)
                    .await
                    .map_err(|e| ValidatorError::RedisError(e.to_string()))?;

                if let Some(v) = value {
                    if let Ok(record) = serde_json::from_str::<AtomicSwapRecord>(&v) {
                        if record.phase == SwapPhase::Pending {
                            tasks.push(crate::PendingValidationTask {
                                swap_id: record.swap_id,
                                evm_data: Vec::new(), // populated by caller with proof data
                                svm_data: Vec::new(), // populated by caller with proof data
                            });
                        }
                    }
                }
            }

            cursor = next_cursor;
            if cursor == 0 {
                break;
            }
        }

        Ok(tasks)
    }

    pub async fn delete_swap(&self, swap_id: &str) -> Result<()> {
        let conn = self
            .redis_client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| ValidatorError::RedisError(e.to_string()))?;

        let mut conn = conn;
        let key = format!("swap:{swap_id}");

        redis::cmd("DEL")
            .arg(&key)
            .query_async::<()>(&mut conn)
            .await
            .map_err(|e| ValidatorError::RedisError(e.to_string()))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_swap_record_creation() {
        let record = AtomicSwapRecord::new("swap-001".to_string(), 60, 1000, 500);
        assert_eq!(record.swap_id, "swap-001");
        assert_eq!(record.phase, SwapPhase::Pending);
        assert!(!record.is_expired());
    }

    #[test]
    fn test_swap_expiration() {
        let mut record = AtomicSwapRecord::new("swap-001".to_string(), 60, 1000, 500);
        record.expires_at = Utc::now() - Duration::seconds(1);
        assert!(record.is_expired());
    }
}
