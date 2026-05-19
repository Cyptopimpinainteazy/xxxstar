//! HTTP handlers for Analytics Service

use actix_web::{web, HttpRequest, HttpResponse};
use chrono::Utc;
use uuid::Uuid;

use crate::db;
use crate::error::ServiceError;
use crate::models::*;
use crate::AppState;

// =============================================================================
// Health Endpoints
// =============================================================================

/// GET /health - Basic health check
pub async fn health_check() -> HttpResponse {
    HttpResponse::Ok().json(HealthResponse {
        status: "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        timestamp: Utc::now(),
        database: "connected".to_string(),
    })
}

/// GET /ready - Readiness check with database validation
pub async fn readiness_check(state: web::Data<AppState>) -> HttpResponse {
    let db_healthy = db::check_health(&state.pool).await;

    let response = ReadinessResponse {
        ready: db_healthy,
        checks: ReadinessChecks {
            database: db_healthy,
        },
    };

    if db_healthy {
        HttpResponse::Ok().json(response)
    } else {
        HttpResponse::ServiceUnavailable().json(response)
    }
}

// =============================================================================
// Event Endpoints
// =============================================================================

/// POST /api/v1/events - Record a new event
pub async fn record_event(
    state: web::Data<AppState>,
    req: HttpRequest,
    body: web::Json<CreateEventRequest>,
) -> Result<HttpResponse, ServiceError> {
    // Extract user agent and IP for analytics
    let user_agent = req
        .headers()
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let ip_hash = req.connection_info().realip_remote_addr().map(|ip| {
        // Hash IP for privacy
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        ip.hash(&mut hasher);
        format!("{:016x}", hasher.finish())
    });

    let event = Event {
        id: Uuid::new_v4(),
        event_type: body.event_type.clone(),
        account: body.account.clone(),
        comit_hash: body.comit_hash.clone(),
        block_number: body.block_number,
        chain_type: body.chain_type.clone(),
        metadata: body.metadata.clone(),
        timestamp: Utc::now(),
        session_id: body.session_id.clone(),
        user_agent,
        ip_hash,
    };

    let created = db::insert_event(&state.pool, &event).await?;

    tracing::info!(
        event_id = %created.id,
        event_type = %created.event_type,
        "Event recorded"
    );

    Ok(HttpResponse::Created().json(created))
}

/// GET /api/v1/events - List events with filters
pub async fn get_events(
    state: web::Data<AppState>,
    query: web::Query<EventQueryParams>,
) -> Result<HttpResponse, ServiceError> {
    let (events, total) = db::query_events(&state.pool, &query).await?;

    let limit = query.limit.unwrap_or(100);
    let offset = query.offset.unwrap_or(0);

    Ok(HttpResponse::Ok().json(PaginatedResponse::new(events, total, limit, offset)))
}

/// GET /api/v1/events/{event_id} - Get single event by ID
pub async fn get_event(
    state: web::Data<AppState>,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, ServiceError> {
    let event_id = path.into_inner();

    match db::get_event_by_id(&state.pool, event_id).await? {
        Some(event) => Ok(HttpResponse::Ok().json(event)),
        None => Err(ServiceError::NotFound(format!(
            "Event {} not found",
            event_id
        ))),
    }
}

// =============================================================================
// Metrics Endpoints
// =============================================================================

/// Query parameters for metrics summary
#[derive(Debug, serde::Deserialize)]
pub struct MetricsSummaryParams {
    pub start_time: Option<chrono::DateTime<Utc>>,
    pub end_time: Option<chrono::DateTime<Utc>>,
}

/// GET /api/v1/metrics/summary - Get aggregated metrics
pub async fn get_metrics_summary(
    state: web::Data<AppState>,
    query: web::Query<MetricsSummaryParams>,
) -> Result<HttpResponse, ServiceError> {
    let summary = db::get_metrics_summary(&state.pool, query.start_time, query.end_time).await?;
    Ok(HttpResponse::Ok().json(summary))
}

/// GET /api/v1/metrics/timeseries - Get time-series data
pub async fn get_timeseries(
    state: web::Data<AppState>,
    query: web::Query<TimeSeriesParams>,
) -> Result<HttpResponse, ServiceError> {
    let data = db::get_timeseries(&state.pool, &query).await?;
    Ok(HttpResponse::Ok().json(data))
}

// =============================================================================
// Comit-Specific Endpoints
// =============================================================================

/// GET /api/v1/comits/stats - Get comit transaction statistics
pub async fn get_comit_stats(state: web::Data<AppState>) -> Result<HttpResponse, ServiceError> {
    let stats = db::get_comit_stats(&state.pool).await?;
    Ok(HttpResponse::Ok().json(stats))
}

/// Query parameters for account comits
#[derive(Debug, serde::Deserialize)]
pub struct AccountComitsParams {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// GET /api/v1/comits/by-account/{account} - Get comits for a specific account
pub async fn get_comits_by_account(
    state: web::Data<AppState>,
    path: web::Path<String>,
    query: web::Query<AccountComitsParams>,
) -> Result<HttpResponse, ServiceError> {
    let account = path.into_inner();
    let limit = query.limit.unwrap_or(100).min(1000);
    let offset = query.offset.unwrap_or(0);

    let (records, total) = db::get_comits_by_account(&state.pool, &account, limit, offset).await?;

    Ok(HttpResponse::Ok().json(PaginatedResponse::new(records, total, limit, offset)))
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_type_serialization() {
        let event_type = EventType::ComitSubmitted;
        assert_eq!(event_type.to_string(), "comit_submitted");

        let parsed = EventType::from("comit_confirmed");
        assert_eq!(parsed, EventType::ComitConfirmed);
    }

    #[test]
    fn test_paginated_response() {
        let data = vec![1, 2, 3];
        let response = PaginatedResponse::new(data, 10, 3, 0);
        assert!(response.has_more);
        assert_eq!(response.total, 10);

        let data2 = vec![1, 2, 3];
        let response2 = PaginatedResponse::new(data2, 3, 3, 0);
        assert!(!response2.has_more);
    }
}
