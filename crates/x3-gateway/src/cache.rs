//! Optional Redis-backed response cache layered in front of the Postgres
//! data source.
//!
//! Every operation is best-effort: callers (the REST layer) already fall
//! back to the database and bump `cache_metrics.fallbacks` when a cache read
//! or write fails, so a Redis outage degrades performance, never correctness.

use crate::db::ChainStats;
use crate::error::{GatewayError, Result};
use redis::aio::MultiplexedConnection;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::debug;

/// Namespaced cache keys shared by the REST layer.
pub const KEY_CHAIN_STATS: &str = "x3-gateway:chain-stats";
const DEFAULT_STATS_TTL_SECS: u64 = 5;

/// A thread-safe handle to a redis async multiplexed connection.
#[derive(Clone)]
pub struct RedisCache {
    inner: Arc<Mutex<MultiplexedConnection>>,
}

impl RedisCache {
    /// Open a Redis client and establish a managed connection. Returns an
    /// error if the URL is invalid or the initial connection fails so the
    /// caller knows up-front whether caching is available.
    pub async fn connect(url: &str) -> Result<Self> {
        let client = redis::Client::open(url)
            .map_err(|e| GatewayError::Internal(format!("invalid redis url: {e}")))?;
        let conn = client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| GatewayError::Internal(format!("connect redis cache: {e}")))?;
        Ok(Self {
            inner: Arc::new(Mutex::new(conn)),
        })
    }

    /// Read a strongly-typed JSON value previously stored with [`Self::set_json`].
    pub async fn get_json<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>> {
        let mut conn = self.inner.lock().await;
        let raw: Option<String> = redis::cmd("GET")
            .arg(key)
            .query_async(&mut *conn)
            .await
            .map_err(cache_err)?;
        match raw {
            None => Ok(None),
            Some(json) => serde_json::from_str(&json)
                .map(Some)
                .map_err(|e| GatewayError::Internal(format!("corrupt cache value: {e}"))),
        }
    }

    /// Store a strongly-typed JSON value with a TTL in seconds.
    pub async fn set_json<T: Serialize>(&self, key: &str, value: &T, ttl_secs: u64) -> Result<()> {
        let json = serde_json::to_string(value)
            .map_err(|e| GatewayError::Internal(format!("serialize cache value: {e}")))?;
        let mut conn = self.inner.lock().await;
        redis::cmd("SETEX")
            .arg(key)
            .arg(ttl_secs)
            .arg(json)
            .query_async::<()>(&mut *conn)
            .await
            .map_err(cache_err)?;
        Ok(())
    }

    /// Read the aggregated chain stats, if cached.
    pub async fn get_chain_stats(&self) -> Result<Option<ChainStats>> {
        self.get_json::<ChainStats>(KEY_CHAIN_STATS).await
    }

    /// Cache aggregated chain stats.
    pub async fn set_chain_stats(&self, stats: &ChainStats) -> Result<()> {
        debug!("caching chain stats");
        self.set_json(KEY_CHAIN_STATS, stats, DEFAULT_STATS_TTL_SECS)
            .await
    }
}

fn cache_err(err: redis::RedisError) -> GatewayError {
    GatewayError::Internal(format!("redis cache error: {err}"))
}
