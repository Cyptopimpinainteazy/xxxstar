//! Optional Redis-backed cache helpers.

use crate::db::ChainStats;
use crate::error::{GatewayError, Result};
use redis::AsyncCommands;
use serde::{de::DeserializeOwned, Serialize};

#[derive(Clone)]
pub struct RedisCache {
    client: redis::Client,
    stats_ttl_secs: u64,
}

impl RedisCache {
    pub fn new(url: &str, stats_ttl_secs: u64) -> Result<Self> {
        let client = redis::Client::open(url)
            .map_err(|e| GatewayError::Config(format!("invalid redis url: {e}")))?;
        Ok(Self {
            client,
            stats_ttl_secs,
        })
    }

    pub async fn get_chain_stats(&self) -> Result<Option<ChainStats>> {
        self.get_json("x3-gateway:chain-stats").await
    }

    pub async fn set_chain_stats(&self, stats: &ChainStats) -> Result<()> {
        self.set_json("x3-gateway:chain-stats", stats, self.stats_ttl_secs)
            .await
    }

    pub async fn get_json<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>> {
        let mut conn = self
            .client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| GatewayError::Internal(format!("redis connection failed: {e}")))?;
        let raw: Option<String> = conn
            .get(key)
            .await
            .map_err(|e| GatewayError::Internal(format!("redis read failed: {e}")))?;
        match raw {
            Some(payload) => {
                let parsed = serde_json::from_str::<T>(&payload)
                    .map_err(|e| GatewayError::Internal(format!("redis payload invalid: {e}")))?;
                Ok(Some(parsed))
            }
            None => Ok(None),
        }
    }

    pub async fn set_json<T: Serialize>(&self, key: &str, value: &T, ttl_secs: u64) -> Result<()> {
        let mut conn = self
            .client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| GatewayError::Internal(format!("redis connection failed: {e}")))?;
        let payload = serde_json::to_string(value)?;
        conn.set_ex::<_, _, ()>(key, payload, ttl_secs)
            .await
            .map_err(|e| GatewayError::Internal(format!("redis write failed: {e}")))?;
        Ok(())
    }
}
