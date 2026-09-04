//! Axum REST API for the X3 solvency sidecar.
//!
//! ## Routes
//!
//! | Method | Path | Description |
//! |--------|------|-------------|
//! | `GET` | `/api/v1/health` | Liveness check; always returns 200 |
//! | `GET` | `/api/v1/dashboard` | Full `SolvencyDashboard` JSON |
//! | `GET` | `/api/v1/vaults` | All vault summaries |
//! | `GET` | `/api/v1/vaults/:id` | Single vault by hex ID, or 404 |
//! | `GET` | `/api/v1/lanes` | All lane summaries |
//! | `GET` | `/api/v1/lanes/frozen` | Frozen lanes only |
//! | `GET` | `/api/v1/partners` | All partner summaries |
//! | `GET` | `/api/v1/alerts` | Last 100 alerts (ring-buffer) |
//!
//! All handlers acquire a read lock on the shared dashboard and return stale
//! data when the subscriber has not yet connected, guaranteeing that the API
//! is reachable even when the node is unreachable.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use serde::Serialize;

use crate::state::{
    Alert, LaneSummary, PartnerSummary, SharedDashboard, SolvencyDashboard, VaultSummary,
};

// ---------------------------------------------------------------------------
// Router constructor
// ---------------------------------------------------------------------------

/// Build the full API router with `dashboard` as shared state.
///
/// The returned `Router` can be served with `axum::serve` on any
/// `tokio::net::TcpListener`.
pub fn router(dashboard: SharedDashboard) -> Router {
    Router::new()
        .route("/api/v1/health", get(health))
        .route("/api/v1/dashboard", get(dashboard_handler))
        .route("/api/v1/vaults", get(vaults))
        .route("/api/v1/vaults/:id", get(vault_by_id))
        .route("/api/v1/lanes", get(lanes))
        .route("/api/v1/lanes/frozen", get(frozen_lanes))
        .route("/api/v1/partners", get(partners))
        .route("/api/v1/alerts", get(alerts))
        .with_state(dashboard)
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// `GET /api/v1/health`
///
/// Returns HTTP 200 with a JSON body containing the service status and the
/// most recently processed block number.  Never returns a non-2xx status so
/// that load-balancers can always route to this sidecar.
async fn health(State(dashboard): State<SharedDashboard>) -> Json<HealthResponse> {
    let last_block = read_dashboard(&dashboard, |d| d.last_updated_block);
    Json(HealthResponse {
        status: "ok",
        last_block,
    })
}

/// `GET /api/v1/dashboard`
///
/// Returns the full `SolvencyDashboard` snapshot.
async fn dashboard_handler(
    State(dashboard): State<SharedDashboard>,
) -> Json<SolvencyDashboard> {
    let snap = read_dashboard(&dashboard, |d| d.clone());
    Json(snap)
}

/// `GET /api/v1/vaults`
///
/// Returns the list of all known vault summaries.
async fn vaults(State(dashboard): State<SharedDashboard>) -> Json<Vec<VaultSummary>> {
    let v = read_dashboard(&dashboard, |d| d.vaults.clone());
    Json(v)
}

/// `GET /api/v1/vaults/:id`
///
/// Returns the vault whose `vault_id` hex string equals `:id`, or HTTP 404.
async fn vault_by_id(
    State(dashboard): State<SharedDashboard>,
    Path(id): Path<String>,
) -> Response {
    let found = read_dashboard(&dashboard, |d| {
        d.vaults.iter().find(|v| v.vault_id == id).cloned()
    });
    match found {
        Some(vault) => Json(vault).into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

/// `GET /api/v1/lanes`
///
/// Returns all lane summaries.
async fn lanes(State(dashboard): State<SharedDashboard>) -> Json<Vec<LaneSummary>> {
    let l = read_dashboard(&dashboard, |d| d.lanes.clone());
    Json(l)
}

/// `GET /api/v1/lanes/frozen`
///
/// Returns only the subset of lanes whose `status` equals `"Frozen"`.
async fn frozen_lanes(State(dashboard): State<SharedDashboard>) -> Json<Vec<LaneSummary>> {
    let l = read_dashboard(&dashboard, |d| {
        d.lanes
            .iter()
            .filter(|lane| lane.status == "Frozen")
            .cloned()
            .collect()
    });
    Json(l)
}

/// `GET /api/v1/partners`
///
/// Returns all partner summaries.
async fn partners(State(dashboard): State<SharedDashboard>) -> Json<Vec<PartnerSummary>> {
    let p = read_dashboard(&dashboard, |d| d.partners.clone());
    Json(p)
}

/// `GET /api/v1/alerts`
///
/// Returns the last 100 alerts from the ring-buffer (most recent at the end).
async fn alerts(State(dashboard): State<SharedDashboard>) -> Json<Vec<Alert>> {
    let a = read_dashboard(&dashboard, |d| d.recent_alerts.clone());
    Json(a)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Acquire a read lock on `dashboard`, apply `f`, and return the result.
///
/// Uses poison recovery (`into_inner`) so a panicking writer does not
/// permanently disable the API.
fn read_dashboard<T>(dashboard: &SharedDashboard, f: impl FnOnce(&SolvencyDashboard) -> T) -> T {
    match dashboard.read() {
        Ok(guard) => f(&guard),
        Err(poisoned) => f(&poisoned.into_inner()),
    }
}

/// Response body for `GET /api/v1/health`.
#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    last_block: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::new_dashboard;

    #[test]
    fn read_dashboard_works_with_empty_state() {
        let d = new_dashboard();
        let block = read_dashboard(&d, |dash| dash.last_updated_block);
        assert_eq!(block, 0);
    }

    #[tokio::test]
    async fn health_route_returns_ok() {
        use axum::body::Body;
        use axum::http::Request;
        use tower::util::ServiceExt;

        let d = new_dashboard();
        let app = router(d);
        let req = Request::builder()
            .uri("/api/v1/health")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn vault_not_found_returns_404() {
        use axum::body::Body;
        use axum::http::Request;
        use tower::util::ServiceExt;

        let d = new_dashboard();
        let app = router(d);
        let req = Request::builder()
            .uri("/api/v1/vaults/nonexistent")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }
}
