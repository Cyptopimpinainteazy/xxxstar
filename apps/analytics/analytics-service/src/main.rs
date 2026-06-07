#![allow(unused, dead_code, deprecated)]

//! Analytics Service for X3 Chain
//!
//! Production-ready analytics backend with:
//! - Event tracking (comit, wallet, error events)
//! - Metrics aggregation
//! - Time-series queries
//! - Health checks

use actix_cors::Cors;
use actix_web::{middleware, web, App, HttpResponse, HttpServer};
use chrono::{DateTime, Utc};
use deadpool_postgres::{Config, Pool, Runtime};
use serde::{Deserialize, Serialize};
use tokio_postgres::NoTls;
use tracing::{error, info, Level};
use tracing_subscriber::FmtSubscriber;
use uuid::Uuid;

mod db;
mod error;
mod handlers;
mod models;

use error::ServiceError;

// =============================================================================
// Configuration
// =============================================================================

#[derive(Debug, Clone, Deserialize)]
pub struct ServiceConfig {
    pub host: String,
    pub port: u16,
    pub database_url: String,
    pub database_pool_size: usize,
}

impl Default for ServiceConfig {
    fn default() -> Self {
        Self {
            host: std::env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string()),
            port: std::env::var("PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(8080),
            database_url: std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "postgres://postgres:password@localhost/analytics".to_string()),
            database_pool_size: std::env::var("DATABASE_POOL_SIZE")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(16),
        }
    }
}

// =============================================================================
// Application State
// =============================================================================

pub struct AppState {
    pub pool: Pool,
}

// =============================================================================
// Main Entry Point
// =============================================================================

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Load .env file if present
    let _ = dotenvy::dotenv();

    // Initialize tracing
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("Failed to set tracing subscriber");

    // Load configuration
    let config = ServiceConfig::default();

    info!("Starting X3 Chain Analytics Service");
    info!(
        "Database: {}",
        config.database_url.split('@').last().unwrap_or("***")
    );

    // Create database pool
    let pool = create_pool(&config)
        .await
        .expect("Failed to create database pool");

    // Run migrations
    db::run_migrations(&pool)
        .await
        .expect("Failed to run migrations");

    let app_state = web::Data::new(AppState { pool });

    let bind_addr = format!("{}:{}", config.host, config.port);
    info!("Listening on {}", bind_addr);

    HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header()
            .max_age(3600);

        App::new()
            .app_data(app_state.clone())
            .wrap(cors)
            .wrap(middleware::Logger::default())
            // Health endpoints
            .route("/health", web::get().to(handlers::health_check))
            .route("/ready", web::get().to(handlers::readiness_check))
            // Event endpoints
            .route("/api/v1/events", web::post().to(handlers::record_event))
            .route("/api/v1/events", web::get().to(handlers::get_events))
            .route(
                "/api/v1/events/{event_id}",
                web::get().to(handlers::get_event),
            )
            // Metrics endpoints
            .route(
                "/api/v1/metrics/summary",
                web::get().to(handlers::get_metrics_summary),
            )
            .route(
                "/api/v1/metrics/timeseries",
                web::get().to(handlers::get_timeseries),
            )
            // Comit-specific analytics
            .route(
                "/api/v1/comits/stats",
                web::get().to(handlers::get_comit_stats),
            )
            .route(
                "/api/v1/comits/by-account/{account}",
                web::get().to(handlers::get_comits_by_account),
            )
    })
    .bind(&bind_addr)?
    .run()
    .await
}

async fn create_pool(config: &ServiceConfig) -> Result<Pool, ServiceError> {
    let mut pg_config = Config::new();

    // Parse DATABASE_URL
    let url = &config.database_url;
    let parts: Vec<&str> = url
        .strip_prefix("postgres://")
        .or_else(|| url.strip_prefix("postgresql://"))
        .unwrap_or(url)
        .split('@')
        .collect();

    if parts.len() == 2 {
        let auth: Vec<&str> = parts[0].split(':').collect();
        let host_db: Vec<&str> = parts[1].split('/').collect();

        if auth.len() >= 1 {
            pg_config.user = Some(auth[0].to_string());
        }
        if auth.len() >= 2 {
            pg_config.password = Some(auth[1].to_string());
        }
        if host_db.len() >= 1 {
            let host_port: Vec<&str> = host_db[0].split(':').collect();
            pg_config.host = Some(host_port[0].to_string());
            if host_port.len() >= 2 {
                pg_config.port = host_port[1].parse().ok();
            }
        }
        if host_db.len() >= 2 {
            pg_config.dbname = Some(host_db[1].to_string());
        }
    }

    pg_config.pool = Some(deadpool_postgres::PoolConfig::new(
        config.database_pool_size,
    ));

    pg_config
        .create_pool(Some(Runtime::Tokio1), NoTls)
        .map_err(|e| ServiceError::Database(format!("Pool creation failed: {}", e)))
}
