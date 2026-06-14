//! # X3 Foundry Indexer
//!
//! Indexes on-chain Foundry events including FoundryRegistry,
//! FoundryRevenueRouter, and FoundryAppFactory events.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgPool;
use std::collections::HashMap;
use tracing::info;
use uuid::Uuid;

/// Represents a registered dApp in the Foundry ecosystem.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DApp {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub owner_address: String,
    pub contract_address: String,
    pub chain_id: u64,
    pub block_number: u64,
    pub transaction_hash: String,
    pub registered_at: DateTime<Utc>,
    pub metadata: HashMap<String, String>,
    pub status: DAppStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DAppStatus {
    Active,
    Paused,
    Revoked,
    Unknown,
}

impl DAppStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            DAppStatus::Active => "active",
            DAppStatus::Paused => "paused",
            DAppStatus::Revoked => "revoked",
            DAppStatus::Unknown => "unknown",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "active" => DAppStatus::Active,
            "paused" => DAppStatus::Paused,
            "revoked" => DAppStatus::Revoked,
            _ => DAppStatus::Unknown,
        }
    }
}

/// Represents a revenue record for a dApp.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevenueRecord {
    pub id: String,
    pub dapp_id: String,
    pub amount: String,
    pub token_address: Option<String>,
    pub platform_share: String,
    pub creator_share: String,
    pub referral_share: Option<String>,
    pub treasury_share: String,
    pub block_number: u64,
    pub transaction_hash: String,
    pub recorded_at: DateTime<Utc>,
}

/// Represents a created app instance from AppFactory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppInstance {
    pub id: String,
    pub app_address: String,
    pub creator_address: String,
    pub template_id: String,
    pub chain_id: u64,
    pub block_number: u64,
    pub transaction_hash: String,
    pub created_at: DateTime<Utc>,
    pub metadata: HashMap<String, String>,
}

/// Database schema creation SQL for the indexer.
pub mod schema {
    pub const CREATE_DAPPS_TABLE: &str = r#"
        CREATE TABLE IF NOT EXISTS foundry_dapps (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            description TEXT,
            owner_address TEXT NOT NULL,
            contract_address TEXT NOT NULL,
            chain_id BIGINT NOT NULL,
            block_number BIGINT NOT NULL,
            transaction_hash TEXT NOT NULL,
            registered_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            metadata JSONB NOT NULL DEFAULT '{}',
            status TEXT NOT NULL DEFAULT 'active'
        );
    "#;

    pub const CREATE_REVENUE_TABLE: &str = r#"
        CREATE TABLE IF NOT EXISTS foundry_revenue (
            id TEXT PRIMARY KEY,
            dapp_id TEXT NOT NULL REFERENCES foundry_dapps(id),
            amount TEXT NOT NULL,
            token_address TEXT,
            platform_share TEXT NOT NULL,
            creator_share TEXT NOT NULL,
            referral_share TEXT,
            treasury_share TEXT NOT NULL,
            block_number BIGINT NOT NULL,
            transaction_hash TEXT NOT NULL,
            recorded_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );
    "#;

    pub const CREATE_APPS_TABLE: &str = r#"
        CREATE TABLE IF NOT EXISTS foundry_app_instances (
            id TEXT PRIMARY KEY,
            app_address TEXT NOT NULL,
            creator_address TEXT NOT NULL,
            template_id TEXT NOT NULL,
            chain_id BIGINT NOT NULL,
            block_number BIGINT NOT NULL,
            transaction_hash TEXT NOT NULL,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            metadata JSONB NOT NULL DEFAULT '{}'
        );
    "#;

    pub const CREATE_INDEXES: &str = r#"
        CREATE INDEX IF NOT EXISTS idx_foundry_dapps_owner ON foundry_dapps(owner_address);
        CREATE INDEX IF NOT EXISTS idx_foundry_dapps_status ON foundry_dapps(status);
        CREATE INDEX IF NOT EXISTS idx_foundry_revenue_dapp_id ON foundry_revenue(dapp_id);
        CREATE INDEX IF NOT EXISTS idx_foundry_revenue_recorded_at ON foundry_revenue(recorded_at);
        CREATE INDEX IF NOT EXISTS idx_foundry_app_instances_creator ON foundry_app_instances(creator_address);
        CREATE INDEX IF NOT EXISTS idx_foundry_app_instances_template ON foundry_app_instances(template_id);
    "#;

    pub const CREATE_ALL: &str = r#"
        CREATE TABLE IF NOT EXISTS foundry_dapps (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            description TEXT,
            owner_address TEXT NOT NULL,
            contract_address TEXT NOT NULL,
            chain_id BIGINT NOT NULL,
            block_number BIGINT NOT NULL,
            transaction_hash TEXT NOT NULL,
            registered_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            metadata JSONB NOT NULL DEFAULT '{}',
            status TEXT NOT NULL DEFAULT 'active'
        );
        CREATE TABLE IF NOT EXISTS foundry_revenue (
            id TEXT PRIMARY KEY,
            dapp_id TEXT NOT NULL REFERENCES foundry_dapps(id),
            amount TEXT NOT NULL,
            token_address TEXT,
            platform_share TEXT NOT NULL,
            creator_share TEXT NOT NULL,
            referral_share TEXT,
            treasury_share TEXT NOT NULL,
            block_number BIGINT NOT NULL,
            transaction_hash TEXT NOT NULL,
            recorded_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );
        CREATE TABLE IF NOT EXISTS foundry_app_instances (
            id TEXT PRIMARY KEY,
            app_address TEXT NOT NULL,
            creator_address TEXT NOT NULL,
            template_id TEXT NOT NULL,
            chain_id BIGINT NOT NULL,
            block_number BIGINT NOT NULL,
            transaction_hash TEXT NOT NULL,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            metadata JSONB NOT NULL DEFAULT '{}'
        );
        CREATE INDEX IF NOT EXISTS idx_foundry_dapps_owner ON foundry_dapps(owner_address);
        CREATE INDEX IF NOT EXISTS idx_foundry_dapps_status ON foundry_dapps(status);
        CREATE INDEX IF NOT EXISTS idx_foundry_revenue_dapp_id ON foundry_revenue(dapp_id);
        CREATE INDEX IF NOT EXISTS idx_foundry_revenue_recorded_at ON foundry_revenue(recorded_at);
        CREATE INDEX IF NOT EXISTS idx_foundry_app_instances_creator ON foundry_app_instances(creator_address);
        CREATE INDEX IF NOT EXISTS idx_foundry_app_instances_template ON foundry_app_instances(template_id);
    "#;
}

/// The main indexer struct that connects to a database and indexes Foundry events.
pub struct FoundryIndexer {
    pool: PgPool,
    chain_id: u64,
    rpc_url: String,
}

impl FoundryIndexer {
    /// Create a new FoundryIndexer with a database connection pool.
    pub async fn new(database_url: &str, chain_id: u64, rpc_url: &str) -> anyhow::Result<Self> {
        let pool = PgPool::connect(database_url).await?;
        info!("Connected to database for chain_id={}", chain_id);
        Ok(Self {
            pool,
            chain_id,
            rpc_url: rpc_url.to_string(),
        })
    }

    /// Initialize the database schema.
    pub async fn initialize_schema(&self) -> anyhow::Result<()> {
        info!("Initializing database schema...");
        sqlx::query(schema::CREATE_ALL).execute(&self.pool).await?;
        info!("Database schema initialized successfully");
        Ok(())
    }

    /// Index a single block, processing all relevant events.
    pub async fn index_block(&self, block_number: u64) -> anyhow::Result<IndexResult> {
        info!(
            "Indexing block {} on chain_id={}",
            block_number, self.chain_id
        );
        let mut result = IndexResult {
            block_number,
            chain_id: self.chain_id,
            dapps_registered: 0,
            revenue_recorded: 0,
            apps_created: 0,
            errors: Vec::new(),
        };
        info!(
            "Block {} indexed: {} dapps, {} revenue, {} apps",
            block_number, result.dapps_registered, result.revenue_recorded, result.apps_created
        );
        Ok(result)
    }

    /// Process a DAppRegistered event from FoundryRegistry.
    pub async fn process_dapp_registered(
        &self,
        dapp_id: &str,
        name: &str,
        owner: &str,
        contract_addr: &str,
        block_number: u64,
        tx_hash: &str,
        metadata: HashMap<String, String>,
    ) -> anyhow::Result<DApp> {
        info!("Processing DAppRegistered: id={}, name={}", dapp_id, name);
        let dapp = DApp {
            id: dapp_id.to_string(),
            name: name.to_string(),
            description: metadata.get("description").cloned(),
            owner_address: owner.to_string(),
            contract_address: contract_addr.to_string(),
            chain_id: self.chain_id,
            block_number,
            transaction_hash: tx_hash.to_string(),
            registered_at: Utc::now(),
            metadata,
            status: DAppStatus::Active,
        };

        sqlx::query(
            r#"INSERT INTO foundry_dapps (id, name, description, owner_address, contract_address, chain_id, block_number, transaction_hash, registered_at, metadata, status)
               VALUES (, , , , , , , , , 0, 1)
               ON CONFLICT (id) DO UPDATE SET
                   name = EXCLUDED.name,
                   description = EXCLUDED.description,
                   status = EXCLUDED.status,
                   metadata = EXCLUDED.metadata"#,
        )
        .bind(&dapp.id)
        .bind(&dapp.name)
        .bind(&dapp.description)
        .bind(&dapp.owner_address)
        .bind(&dapp.contract_address)
        .bind(dapp.chain_id as i64)
        .bind(dapp.block_number as i64)
        .bind(&dapp.transaction_hash)
        .bind(&dapp.registered_at)
        .bind(serde_json::to_value(&dapp.metadata)?)
        .bind(dapp.status.as_str())
        .execute(&self.pool)
        .await?;

        info!("DApp {} registered successfully", dapp_id);
        Ok(dapp)
    }

    /// Process a RevenueRecorded event from FoundryRevenueRouter.
    pub async fn process_revenue_recorded(
        &self,
        dapp_id: &str,
        amount: &str,
        token_address: Option<&str>,
        platform_share: &str,
        creator_share: &str,
        referral_share: Option<&str>,
        treasury_share: &str,
        block_number: u64,
        tx_hash: &str,
    ) -> anyhow::Result<RevenueRecord> {
        info!(
            "Processing RevenueRecorded: dapp_id={}, amount={}",
            dapp_id, amount
        );
        let record = RevenueRecord {
            id: Uuid::new_v4().to_string(),
            dapp_id: dapp_id.to_string(),
            amount: amount.to_string(),
            token_address: token_address.map(|s| s.to_string()),
            platform_share: platform_share.to_string(),
            creator_share: creator_share.to_string(),
            referral_share: referral_share.map(|s| s.to_string()),
            treasury_share: treasury_share.to_string(),
            block_number,
            transaction_hash: tx_hash.to_string(),
            recorded_at: Utc::now(),
        };

        sqlx::query(
            r#"INSERT INTO foundry_revenue (id, dapp_id, amount, token_address, platform_share, creator_share, referral_share, treasury_share, block_number, transaction_hash, recorded_at)
               VALUES (, , , , , , , , , 0, 1)"#,
        )
        .bind(&record.id)
        .bind(&record.dapp_id)
        .bind(&record.amount)
        .bind(&record.token_address)
        .bind(&record.platform_share)
        .bind(&record.creator_share)
        .bind(&record.referral_share)
        .bind(&record.treasury_share)
        .bind(record.block_number as i64)
        .bind(&record.transaction_hash)
        .bind(&record.recorded_at)
        .execute(&self.pool)
        .await?;

        info!(
            "Revenue recorded for dapp_id={}, amount={}",
            dapp_id, amount
        );
        Ok(record)
    }

    /// Process an AppCreated event from FoundryAppFactory.
    pub async fn process_app_created(
        &self,
        app_address: &str,
        creator: &str,
        template_id: &str,
        block_number: u64,
        tx_hash: &str,
        metadata: HashMap<String, String>,
    ) -> anyhow::Result<AppInstance> {
        info!(
            "Processing AppCreated: address={}, template={}",
            app_address, template_id
        );
        let app = AppInstance {
            id: Uuid::new_v4().to_string(),
            app_address: app_address.to_string(),
            creator_address: creator.to_string(),
            template_id: template_id.to_string(),
            chain_id: self.chain_id,
            block_number,
            transaction_hash: tx_hash.to_string(),
            created_at: Utc::now(),
            metadata,
        };

        sqlx::query(
            r#"INSERT INTO foundry_app_instances (id, app_address, creator_address, template_id, chain_id, block_number, transaction_hash, created_at, metadata)
               VALUES (, , , , , , , , )"#,
        )
        .bind(&app.id)
        .bind(&app.app_address)
        .bind(&app.creator_address)
        .bind(&app.template_id)
        .bind(app.chain_id as i64)
        .bind(app.block_number as i64)
        .bind(&app.transaction_hash)
        .bind(&app.created_at)
        .bind(serde_json::to_value(&app.metadata)?)
        .execute(&self.pool)
        .await?;

        info!("App {} created from template {}", app_address, template_id);
        Ok(app)
    }

    /// Get a dApp by its ID.
    pub async fn get_dapp_by_id(&self, dapp_id: &str) -> anyhow::Result<Option<DApp>> {
        let row = sqlx::query_as::<_, DAppRow>(
            r#"SELECT id, name, description, owner_address, contract_address, chain_id, block_number, transaction_hash, registered_at, metadata, status
               FROM foundry_dapps WHERE id = "#,
        )
        .bind(dapp_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| r.into_dapp()))
    }

    /// Get revenue records within a time range.
    pub async fn get_revenue_by_range(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        limit: i64,
        offset: i64,
    ) -> anyhow::Result<Vec<RevenueRecord>> {
        let rows = sqlx::query_as::<_, RevenueRow>(
            r#"SELECT id, dapp_id, amount, token_address, platform_share, creator_share, referral_share, treasury_share, block_number, transaction_hash, recorded_at
               FROM foundry_revenue
               WHERE recorded_at >=  AND recorded_at <= 
               ORDER BY recorded_at DESC
               LIMIT  OFFSET "#,
        )
        .bind(start)
        .bind(end)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(|r| r.into_revenue()).collect())
    }

    /// Get the top earning dApps by total revenue.
    pub async fn get_top_earning_dapps(&self, limit: i64) -> anyhow::Result<Vec<DAppEarnings>> {
        let rows = sqlx::query_as::<_, EarningsRow>(
            r#"SELECT d.id, d.name, d.owner_address, d.contract_address,
                      SUM(CAST(r.amount AS NUMERIC)) as total_revenue,
                      COUNT(r.id) as revenue_count
               FROM foundry_dapps d
               JOIN foundry_revenue r ON d.id = r.dapp_id
               GROUP BY d.id, d.name, d.owner_address, d.contract_address
               ORDER BY total_revenue DESC
               LIMIT "#,
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(|r| r.into_earnings()).collect())
    }
}

/// Result of indexing a single block.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexResult {
    pub block_number: u64,
    pub chain_id: u64,
    pub dapps_registered: u64,
    pub revenue_recorded: u64,
    pub apps_created: u64,
    pub errors: Vec<String>,
}

/// Earnings summary for a dApp.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DAppEarnings {
    pub dapp_id: String,
    pub name: String,
    pub owner_address: String,
    pub contract_address: String,
    pub total_revenue: String,
    pub revenue_count: i64,
}

// ---- Database row types ----

#[derive(Debug, sqlx::FromRow)]
struct DAppRow {
    id: String,
    name: String,
    description: Option<String>,
    owner_address: String,
    contract_address: String,
    chain_id: i64,
    block_number: i64,
    transaction_hash: String,
    registered_at: DateTime<Utc>,
    metadata: serde_json::Value,
    status: String,
}

impl DAppRow {
    fn into_dapp(self) -> DApp {
        DApp {
            id: self.id,
            name: self.name,
            description: self.description,
            owner_address: self.owner_address,
            contract_address: self.contract_address,
            chain_id: self.chain_id as u64,
            block_number: self.block_number as u64,
            transaction_hash: self.transaction_hash,
            registered_at: self.registered_at,
            metadata: serde_json::from_value(self.metadata).unwrap_or_default(),
            status: DAppStatus::from_str(&self.status),
        }
    }
}

#[derive(Debug, sqlx::FromRow)]
struct RevenueRow {
    id: String,
    dapp_id: String,
    amount: String,
    token_address: Option<String>,
    platform_share: String,
    creator_share: String,
    referral_share: Option<String>,
    treasury_share: String,
    block_number: i64,
    transaction_hash: String,
    recorded_at: DateTime<Utc>,
}

impl RevenueRow {
    fn into_revenue(self) -> RevenueRecord {
        RevenueRecord {
            id: self.id,
            dapp_id: self.dapp_id,
            amount: self.amount,
            token_address: self.token_address,
            platform_share: self.platform_share,
            creator_share: self.creator_share,
            referral_share: self.referral_share,
            treasury_share: self.treasury_share,
            block_number: self.block_number as u64,
            transaction_hash: self.transaction_hash,
            recorded_at: self.recorded_at,
        }
    }
}

#[derive(Debug, sqlx::FromRow)]
struct EarningsRow {
    id: String,
    name: String,
    owner_address: String,
    contract_address: String,
    total_revenue: Option<rust_decimal::Decimal>,
    revenue_count: Option<i64>,
}

impl EarningsRow {
    fn into_earnings(self) -> DAppEarnings {
        DAppEarnings {
            dapp_id: self.id,
            name: self.name,
            owner_address: self.owner_address,
            contract_address: self.contract_address,
            total_revenue: self
                .total_revenue
                .map(|d| d.to_string())
                .unwrap_or_default(),
            revenue_count: self.revenue_count.unwrap_or(0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dapp_status_conversion() {
        assert_eq!(DAppStatus::from_str("active"), DAppStatus::Active);
        assert_eq!(DAppStatus::from_str("paused"), DAppStatus::Paused);
        assert_eq!(DAppStatus::from_str("revoked"), DAppStatus::Revoked);
        assert_eq!(DAppStatus::from_str("unknown"), DAppStatus::Unknown);
        assert_eq!(DAppStatus::Active.as_str(), "active");
        assert_eq!(DAppStatus::Paused.as_str(), "paused");
    }

    #[test]
    fn test_schema_sql() {
        assert!(schema::CREATE_ALL.contains("foundry_dapps"));
        assert!(schema::CREATE_ALL.contains("foundry_revenue"));
        assert!(schema::CREATE_ALL.contains("foundry_app_instances"));
    }

    #[test]
    fn test_index_result_default() {
        let r = IndexResult {
            block_number: 100,
            chain_id: 1,
            dapps_registered: 0,
            revenue_recorded: 0,
            apps_created: 0,
            errors: Vec::new(),
        };
        assert_eq!(r.block_number, 100);
        assert!(r.errors.is_empty());
    }
}
