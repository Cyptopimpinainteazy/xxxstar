//! Data models for Analytics Service

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// =============================================================================
// Event Models
// =============================================================================

/// Event type enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    ComitSubmitted,
    ComitConfirmed,
    ComitFailed,
    WalletConnected,
    WalletDisconnected,
    TransactionSent,
    TransactionReceived,
    SwapInitiated,
    SwapCompleted,
    Error,
    Custom,
}

impl std::fmt::Display for EventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EventType::ComitSubmitted => write!(f, "comit_submitted"),
            EventType::ComitConfirmed => write!(f, "comit_confirmed"),
            EventType::ComitFailed => write!(f, "comit_failed"),
            EventType::WalletConnected => write!(f, "wallet_connected"),
            EventType::WalletDisconnected => write!(f, "wallet_disconnected"),
            EventType::TransactionSent => write!(f, "transaction_sent"),
            EventType::TransactionReceived => write!(f, "transaction_received"),
            EventType::SwapInitiated => write!(f, "swap_initiated"),
            EventType::SwapCompleted => write!(f, "swap_completed"),
            EventType::Error => write!(f, "error"),
            EventType::Custom => write!(f, "custom"),
        }
    }
}

impl From<&str> for EventType {
    fn from(s: &str) -> Self {
        match s {
            "comit_submitted" => EventType::ComitSubmitted,
            "comit_confirmed" => EventType::ComitConfirmed,
            "comit_failed" => EventType::ComitFailed,
            "wallet_connected" => EventType::WalletConnected,
            "wallet_disconnected" => EventType::WalletDisconnected,
            "transaction_sent" => EventType::TransactionSent,
            "transaction_received" => EventType::TransactionReceived,
            "swap_initiated" => EventType::SwapInitiated,
            "swap_completed" => EventType::SwapCompleted,
            "error" => EventType::Error,
            _ => EventType::Custom,
        }
    }
}

/// Analytics event record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: Uuid,
    pub event_type: EventType,
    pub account: Option<String>,
    pub comit_hash: Option<String>,
    pub block_number: Option<i64>,
    pub chain_type: Option<String>, // "evm", "svm", "dual"
    pub metadata: Option<serde_json::Value>,
    pub timestamp: DateTime<Utc>,
    pub session_id: Option<String>,
    pub user_agent: Option<String>,
    pub ip_hash: Option<String>, // Hashed for privacy
}

/// Request to create a new event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateEventRequest {
    pub event_type: EventType,
    pub account: Option<String>,
    pub comit_hash: Option<String>,
    pub block_number: Option<i64>,
    pub chain_type: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub session_id: Option<String>,
}

/// Query parameters for listing events
#[derive(Debug, Clone, Deserialize)]
pub struct EventQueryParams {
    pub event_type: Option<String>,
    pub account: Option<String>,
    pub chain_type: Option<String>,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

// =============================================================================
// Metrics Models
// =============================================================================

/// Summary metrics for the apps/dash-legacy-2-legacy-2board
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSummary {
    pub total_events: i64,
    pub total_comits: i64,
    pub successful_comits: i64,
    pub failed_comits: i64,
    pub unique_accounts: i64,
    pub evm_transactions: i64,
    pub svm_transactions: i64,
    pub dual_transactions: i64,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
}

/// Time-series data point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSeriesPoint {
    pub timestamp: DateTime<Utc>,
    pub value: i64,
    pub label: Option<String>,
}

/// Time-series query parameters
#[derive(Debug, Clone, Deserialize)]
pub struct TimeSeriesParams {
    pub metric: String,           // "events", "comits", "accounts"
    pub interval: Option<String>, // "hour", "day", "week"
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub event_type: Option<String>,
}

// =============================================================================
// Comit-Specific Models
// =============================================================================

/// Comit transaction statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComitStats {
    pub total_comits: i64,
    pub pending: i64,
    pub confirmed: i64,
    pub failed: i64,
    pub avg_confirmation_time_ms: Option<f64>,
    pub evm_only: i64,
    pub svm_only: i64,
    pub dual_vm: i64,
    pub total_gas_used: Option<i64>,
}

/// Comit record for account queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComitRecord {
    pub comit_hash: String,
    pub account: String,
    pub chain_type: String,
    pub status: String,
    pub block_number: Option<i64>,
    pub gas_used: Option<i64>,
    pub submitted_at: DateTime<Utc>,
    pub confirmed_at: Option<DateTime<Utc>>,
}

// =============================================================================
// Response Models
// =============================================================================

/// Paginated response wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
    pub has_more: bool,
}

impl<T> PaginatedResponse<T> {
    pub fn new(data: Vec<T>, total: i64, limit: i64, offset: i64) -> Self {
        let has_more = offset + (data.len() as i64) < total;
        Self {
            data,
            total,
            limit,
            offset,
            has_more,
        }
    }
}

/// Health check response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub timestamp: DateTime<Utc>,
    pub database: String,
}

/// Readiness check response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadinessResponse {
    pub ready: bool,
    pub checks: ReadinessChecks,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadinessChecks {
    pub database: bool,
}
