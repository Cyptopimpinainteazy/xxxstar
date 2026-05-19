//! Database operations for Analytics Service

use chrono::{DateTime, Utc};
use deadpool_postgres::Pool;
use tokio_postgres::Row;
use uuid::Uuid;

use crate::error::ServiceError;
use crate::models::*;

// =============================================================================
// Migrations
// =============================================================================

/// Run database migrations
pub async fn run_migrations(pool: &Pool) -> Result<(), ServiceError> {
    let client = pool.get().await?;

    // Create events table
    client
        .execute(
            r#"
            CREATE TABLE IF NOT EXISTS events (
                id UUID PRIMARY KEY,
                event_type VARCHAR(50) NOT NULL,
                account VARCHAR(100),
                comit_hash VARCHAR(100),
                block_number BIGINT,
                chain_type VARCHAR(20),
                metadata JSONB,
                timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                session_id VARCHAR(100),
                user_agent TEXT,
                ip_hash VARCHAR(64),
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
            )
            "#,
            &[],
        )
        .await?;

    // Create comits tracking table
    client
        .execute(
            r#"
            CREATE TABLE IF NOT EXISTS comit_tracking (
                comit_hash VARCHAR(100) PRIMARY KEY,
                account VARCHAR(100) NOT NULL,
                chain_type VARCHAR(20) NOT NULL,
                status VARCHAR(20) NOT NULL DEFAULT 'pending',
                block_number BIGINT,
                gas_used BIGINT,
                submitted_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                confirmed_at TIMESTAMPTZ,
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
            )
            "#,
            &[],
        )
        .await?;

    // Create metrics aggregation table
    client
        .execute(
            r#"
            CREATE TABLE IF NOT EXISTS metrics_hourly (
                id SERIAL PRIMARY KEY,
                hour TIMESTAMPTZ NOT NULL,
                event_type VARCHAR(50),
                chain_type VARCHAR(20),
                count BIGINT NOT NULL DEFAULT 0,
                unique_accounts BIGINT NOT NULL DEFAULT 0,
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                UNIQUE(hour, event_type, chain_type)
            )
            "#,
            &[],
        )
        .await?;

    // Create indexes
    let indexes = [
        "CREATE INDEX IF NOT EXISTS idx_events_timestamp ON events(timestamp DESC)",
        "CREATE INDEX IF NOT EXISTS idx_events_event_type ON events(event_type)",
        "CREATE INDEX IF NOT EXISTS idx_events_account ON events(account)",
        "CREATE INDEX IF NOT EXISTS idx_events_chain_type ON events(chain_type)",
        "CREATE INDEX IF NOT EXISTS idx_events_comit_hash ON events(comit_hash)",
        "CREATE INDEX IF NOT EXISTS idx_comit_tracking_account ON comit_tracking(account)",
        "CREATE INDEX IF NOT EXISTS idx_comit_tracking_status ON comit_tracking(status)",
        "CREATE INDEX IF NOT EXISTS idx_metrics_hourly_hour ON metrics_hourly(hour DESC)",
    ];

    for index_sql in indexes {
        client.execute(index_sql, &[]).await?;
    }

    tracing::info!("Database migrations completed successfully");
    Ok(())
}

// =============================================================================
// Event Operations
// =============================================================================

/// Insert a new event
pub async fn insert_event(pool: &Pool, event: &Event) -> Result<Event, ServiceError> {
    let client = pool.get().await?;

    client
        .execute(
            r#"
            INSERT INTO events (id, event_type, account, comit_hash, block_number, 
                               chain_type, metadata, timestamp, session_id, user_agent, ip_hash)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            "#,
            &[
                &event.id,
                &event.event_type.to_string(),
                &event.account,
                &event.comit_hash,
                &event.block_number,
                &event.chain_type,
                &event.metadata,
                &event.timestamp,
                &event.session_id,
                &event.user_agent,
                &event.ip_hash,
            ],
        )
        .await?;

    // Update comit tracking if this is a comit event
    if let Some(comit_hash) = &event.comit_hash {
        if let Some(account) = &event.account {
            update_comit_tracking(
                pool,
                comit_hash,
                account,
                &event.event_type,
                event.block_number,
            )
            .await?;
        }
    }

    // Update hourly metrics
    update_hourly_metrics(
        pool,
        &event.event_type,
        event.chain_type.as_deref(),
        &event.account,
    )
    .await?;

    Ok(event.clone())
}

/// Get event by ID
pub async fn get_event_by_id(pool: &Pool, event_id: Uuid) -> Result<Option<Event>, ServiceError> {
    let client = pool.get().await?;

    let row = client
        .query_opt(
            r#"
            SELECT id, event_type, account, comit_hash, block_number, chain_type,
                   metadata, timestamp, session_id, user_agent, ip_hash
            FROM events
            WHERE id = $1
            "#,
            &[&event_id],
        )
        .await?;

    Ok(row.map(row_to_event))
}

/// Query events with filters
pub async fn query_events(
    pool: &Pool,
    params: &EventQueryParams,
) -> Result<(Vec<Event>, i64), ServiceError> {
    let client = pool.get().await?;

    let limit = params.limit.unwrap_or(100).min(1000);
    let offset = params.offset.unwrap_or(0);

    // Build WHERE clause dynamically
    let mut conditions = Vec::new();
    let mut param_values: Vec<Box<dyn tokio_postgres::types::ToSql + Sync + Send>> = Vec::new();
    let mut param_idx = 1;

    if let Some(event_type) = &params.event_type {
        conditions.push(format!("event_type = ${}", param_idx));
        param_values.push(Box::new(event_type.clone()));
        param_idx += 1;
    }
    if let Some(account) = &params.account {
        conditions.push(format!("account = ${}", param_idx));
        param_values.push(Box::new(account.clone()));
        param_idx += 1;
    }
    if let Some(chain_type) = &params.chain_type {
        conditions.push(format!("chain_type = ${}", param_idx));
        param_values.push(Box::new(chain_type.clone()));
        param_idx += 1;
    }
    if let Some(start_time) = &params.start_time {
        conditions.push(format!("timestamp >= ${}", param_idx));
        param_values.push(Box::new(*start_time));
        param_idx += 1;
    }
    if let Some(end_time) = &params.end_time {
        conditions.push(format!("timestamp <= ${}", param_idx));
        param_values.push(Box::new(*end_time));
        param_idx += 1;
    }

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };

    // Get total count
    let count_sql = format!("SELECT COUNT(*) FROM events {}", where_clause);
    let params_slice: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = param_values
        .iter()
        .map(|v| v.as_ref() as &(dyn tokio_postgres::types::ToSql + Sync))
        .collect();

    let count_row = client.query_one(&count_sql, &params_slice).await?;
    let total: i64 = count_row.get(0);

    // Get events
    let query_sql = format!(
        r#"
        SELECT id, event_type, account, comit_hash, block_number, chain_type,
               metadata, timestamp, session_id, user_agent, ip_hash
        FROM events
        {}
        ORDER BY timestamp DESC
        LIMIT {} OFFSET {}
        "#,
        where_clause, limit, offset
    );

    let rows = client.query(&query_sql, &params_slice).await?;
    let events: Vec<Event> = rows.into_iter().map(row_to_event).collect();

    Ok((events, total))
}

fn row_to_event(row: Row) -> Event {
    let event_type_str: String = row.get("event_type");
    Event {
        id: row.get("id"),
        event_type: EventType::from(event_type_str.as_str()),
        account: row.get("account"),
        comit_hash: row.get("comit_hash"),
        block_number: row.get("block_number"),
        chain_type: row.get("chain_type"),
        metadata: row.get("metadata"),
        timestamp: row.get("timestamp"),
        session_id: row.get("session_id"),
        user_agent: row.get("user_agent"),
        ip_hash: row.get("ip_hash"),
    }
}

// =============================================================================
// Comit Tracking Operations
// =============================================================================

async fn update_comit_tracking(
    pool: &Pool,
    comit_hash: &str,
    account: &str,
    event_type: &EventType,
    block_number: Option<i64>,
) -> Result<(), ServiceError> {
    let client = pool.get().await?;

    let (status, chain_type) = match event_type {
        EventType::ComitSubmitted => ("pending", "dual"),
        EventType::ComitConfirmed => ("confirmed", "dual"),
        EventType::ComitFailed => ("failed", "dual"),
        _ => return Ok(()), // Not a comit event
    };

    // Upsert comit tracking
    client
        .execute(
            r#"
            INSERT INTO comit_tracking (comit_hash, account, chain_type, status, block_number, submitted_at)
            VALUES ($1, $2, $3, $4, $5, NOW())
            ON CONFLICT (comit_hash) DO UPDATE SET
                status = EXCLUDED.status,
                block_number = COALESCE(EXCLUDED.block_number, comit_tracking.block_number),
                confirmed_at = CASE WHEN EXCLUDED.status = 'confirmed' THEN NOW() ELSE comit_tracking.confirmed_at END,
                updated_at = NOW()
            "#,
            &[&comit_hash, &account, &chain_type, &status, &block_number],
        )
        .await?;

    Ok(())
}

/// Get comits by account
pub async fn get_comits_by_account(
    pool: &Pool,
    account: &str,
    limit: i64,
    offset: i64,
) -> Result<(Vec<ComitRecord>, i64), ServiceError> {
    let client = pool.get().await?;

    // Get total count
    let count_row = client
        .query_one(
            "SELECT COUNT(*) FROM comit_tracking WHERE account = $1",
            &[&account],
        )
        .await?;
    let total: i64 = count_row.get(0);

    // Get records
    let rows = client
        .query(
            r#"
            SELECT comit_hash, account, chain_type, status, block_number, 
                   gas_used, submitted_at, confirmed_at
            FROM comit_tracking
            WHERE account = $1
            ORDER BY submitted_at DESC
            LIMIT $2 OFFSET $3
            "#,
            &[&account, &limit, &offset],
        )
        .await?;

    let records: Vec<ComitRecord> = rows
        .into_iter()
        .map(|row| ComitRecord {
            comit_hash: row.get("comit_hash"),
            account: row.get("account"),
            chain_type: row.get("chain_type"),
            status: row.get("status"),
            block_number: row.get("block_number"),
            gas_used: row.get("gas_used"),
            submitted_at: row.get("submitted_at"),
            confirmed_at: row.get("confirmed_at"),
        })
        .collect();

    Ok((records, total))
}

/// Get comit statistics
pub async fn get_comit_stats(pool: &Pool) -> Result<ComitStats, ServiceError> {
    let client = pool.get().await?;

    let row = client
        .query_one(
            r#"
            SELECT
                COUNT(*) as total_comits,
                COUNT(*) FILTER (WHERE status = 'pending') as pending,
                COUNT(*) FILTER (WHERE status = 'confirmed') as confirmed,
                COUNT(*) FILTER (WHERE status = 'failed') as failed,
                AVG(EXTRACT(EPOCH FROM (confirmed_at - submitted_at)) * 1000) 
                    FILTER (WHERE confirmed_at IS NOT NULL) as avg_confirmation_time_ms,
                COUNT(*) FILTER (WHERE chain_type = 'evm') as evm_only,
                COUNT(*) FILTER (WHERE chain_type = 'svm') as svm_only,
                COUNT(*) FILTER (WHERE chain_type = 'dual') as dual_vm,
                SUM(gas_used) as total_gas_used
            FROM comit_tracking
            "#,
            &[],
        )
        .await?;

    Ok(ComitStats {
        total_comits: row.get::<_, i64>("total_comits"),
        pending: row.get::<_, i64>("pending"),
        confirmed: row.get::<_, i64>("confirmed"),
        failed: row.get::<_, i64>("failed"),
        avg_confirmation_time_ms: row.get("avg_confirmation_time_ms"),
        evm_only: row.get::<_, i64>("evm_only"),
        svm_only: row.get::<_, i64>("svm_only"),
        dual_vm: row.get::<_, i64>("dual_vm"),
        total_gas_used: row.get("total_gas_used"),
    })
}

// =============================================================================
// Metrics Operations
// =============================================================================

async fn update_hourly_metrics(
    pool: &Pool,
    event_type: &EventType,
    chain_type: Option<&str>,
    account: &Option<String>,
) -> Result<(), ServiceError> {
    let client = pool.get().await?;

    let event_type_str = event_type.to_string();
    let chain_type_str = chain_type.unwrap_or("unknown");

    // Upsert hourly metrics
    client
        .execute(
            r#"
            INSERT INTO metrics_hourly (hour, event_type, chain_type, count, unique_accounts)
            VALUES (date_trunc('hour', NOW()), $1, $2, 1, CASE WHEN $3::text IS NOT NULL THEN 1 ELSE 0 END)
            ON CONFLICT (hour, event_type, chain_type) DO UPDATE SET
                count = metrics_hourly.count + 1,
                unique_accounts = metrics_hourly.unique_accounts + 
                    CASE WHEN $3::text IS NOT NULL THEN 1 ELSE 0 END
            "#,
            &[&event_type_str, &chain_type_str, account],
        )
        .await?;

    Ok(())
}

/// Get metrics summary
pub async fn get_metrics_summary(
    pool: &Pool,
    start_time: Option<DateTime<Utc>>,
    end_time: Option<DateTime<Utc>>,
) -> Result<MetricsSummary, ServiceError> {
    let client = pool.get().await?;

    let start = start_time.unwrap_or_else(|| Utc::now() - chrono::Duration::days(30));
    let end = end_time.unwrap_or_else(Utc::now);

    let row = client
        .query_one(
            r#"
            SELECT
                COUNT(*) as total_events,
                COUNT(*) FILTER (WHERE event_type LIKE 'comit_%') as total_comits,
                COUNT(*) FILTER (WHERE event_type = 'comit_confirmed') as successful_comits,
                COUNT(*) FILTER (WHERE event_type = 'comit_failed') as failed_comits,
                COUNT(DISTINCT account) FILTER (WHERE account IS NOT NULL) as unique_accounts,
                COUNT(*) FILTER (WHERE chain_type = 'evm') as evm_transactions,
                COUNT(*) FILTER (WHERE chain_type = 'svm') as svm_transactions,
                COUNT(*) FILTER (WHERE chain_type = 'dual') as dual_transactions
            FROM events
            WHERE timestamp BETWEEN $1 AND $2
            "#,
            &[&start, &end],
        )
        .await?;

    Ok(MetricsSummary {
        total_events: row.get("total_events"),
        total_comits: row.get("total_comits"),
        successful_comits: row.get("successful_comits"),
        failed_comits: row.get("failed_comits"),
        unique_accounts: row.get("unique_accounts"),
        evm_transactions: row.get("evm_transactions"),
        svm_transactions: row.get("svm_transactions"),
        dual_transactions: row.get("dual_transactions"),
        period_start: start,
        period_end: end,
    })
}

/// Get time-series data
pub async fn get_timeseries(
    pool: &Pool,
    params: &TimeSeriesParams,
) -> Result<Vec<TimeSeriesPoint>, ServiceError> {
    let client = pool.get().await?;

    let interval = params.interval.as_deref().unwrap_or("hour");
    let start = params
        .start_time
        .unwrap_or_else(|| Utc::now() - chrono::Duration::days(7));
    let end = params.end_time.unwrap_or_else(Utc::now);

    let interval_sql = match interval {
        "hour" => "date_trunc('hour', timestamp)",
        "day" => "date_trunc('day', timestamp)",
        "week" => "date_trunc('week', timestamp)",
        _ => "date_trunc('hour', timestamp)",
    };

    let mut query = format!(
        r#"
        SELECT {} as ts, COUNT(*) as value
        FROM events
        WHERE timestamp BETWEEN $1 AND $2
        "#,
        interval_sql
    );

    if let Some(event_type) = &params.event_type {
        query.push_str(&format!(" AND event_type = '{}'", event_type));
    }

    query.push_str(" GROUP BY ts ORDER BY ts");

    let rows = client.query(&query, &[&start, &end]).await?;

    let points: Vec<TimeSeriesPoint> = rows
        .into_iter()
        .map(|row| TimeSeriesPoint {
            timestamp: row.get("ts"),
            value: row.get("value"),
            label: None,
        })
        .collect();

    Ok(points)
}

/// Check database health
pub async fn check_health(pool: &Pool) -> bool {
    match pool.get().await {
        Ok(client) => client.query_one("SELECT 1", &[]).await.is_ok(),
        Err(_) => false,
    }
}
