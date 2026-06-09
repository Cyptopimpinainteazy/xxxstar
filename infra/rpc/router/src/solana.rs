//! Solana chain handler.
//!
//! Routes Solana JSON-RPC methods to the best upstream, applying:
//! - WebSocket sticky routing for subscriptions
//! - Heavy method filtering (getProgramAccounts → private/paid only)
//! - Transaction broadcast guard
//! - Slot freshness checks

use std::sync::Arc;
use tracing::warn;

use crate::config::MethodClass;
use crate::scoring::forward_to_upstream;
use crate::AppState;

/// Handle a Solana RPC request.
pub async fn handle_request(state: &Arc<AppState>, method: &str, body: &str) -> anyhow::Result<String> {
    let chain_name = "solana";

    // ── Heavy methods → private/paid upstreams only ────────────────
    let heavy_methods = ["getProgramAccounts", "getMultipleAccounts", "getSignaturesForAddress"];
    if heavy_methods.contains(&method) {
        // Require private tier (tier 0) or paid tier (tier 1)
        let providers = state.pool.healthy_for_chain(chain_name);
        let private_or_paid = providers
            .iter()
            .filter(|p| p.tier <= 1)
            .collect::<Vec<_>>();

        if let Some(upstream) = private_or_paid.first() {
            return forward_to_upstream(&upstream.url, body).await;
        }
        return Err(anyhow::anyhow!(
            "No private/paid Solana upstream available for heavy method {}",
            method
        ));
    }

    // ── Quorum check for critical methods ─────────────────────────
    if state.config.requires_quorum(method, chain_name) {
        return crate::quorum::quorum_call(state, chain_name, method, body).await;
    }

    // ── Transaction broadcast ─────────────────────────────────────
    if method == "sendTransaction" {
        return crate::tx_broadcast::controlled_broadcast(
            state,
            chain_name,
            method,
            body,
        )
        .await;
    }

    // ── Standard routing ──────────────────────────────────────────
    if let Some(best) = state.pool.best_for_chain(chain_name, None) {
        return forward_to_upstream(&best.url, body).await;
    }

    let all_healthy = state.pool.healthy_for_chain(chain_name);
    for upstream in &all_healthy {
        match forward_to_upstream(&upstream.url, body).await {
            Ok(resp) => return Ok(resp),
            Err(e) => {
                warn!(upstream = %upstream.id, error = %e, "Solana upstream failed, trying next");
            }
        }
    }

    Err(anyhow::anyhow!("No healthy Solana upstream available"))
}

/// Handle a WebSocket subscription forward. Sticky routing is maintained
/// by the main WebSocket handler — this function just forwards a single
/// message to the best WebSocket-capable upstream.
pub async fn ws_forward(state: &Arc<AppState>, _method: &str, body: &str) -> anyhow::Result<String> {
    let chain_name = "solana";

    // For WebSocket, we need a websocket-capable upstream
    if let Some(best_ws) = state.pool.best_for_chain(chain_name, Some("websocket")) {
        if let Some(ref ws_url) = best_ws.ws_url {
            return forward_to_upstream(ws_url, body).await;
        }
    }

    // Fallback: try any upstream that might handle WS through HTTP upgrade
    if let Some(best) = state.pool.best_for_chain(chain_name, None) {
        if let Some(ref ws_url) = best.ws_url {
            return forward_to_upstream(ws_url, body).await;
        }
        // Try HTTP URL for methods that work over HTTP
        return forward_to_upstream(&best.url, body).await;
    }

    Err(anyhow::anyhow!("No WebSocket-capable Solana upstream available"))
}