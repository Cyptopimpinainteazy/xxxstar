//! X3 chain handler (Substrate-based).
//!
//! Routes x3_* and author_* methods to the best upstream, applying:
//! - Quorum verification for proof/finality/atomic methods
//! - Transaction broadcast guard
//! - Finalized-state routing (reject un-finalized nodes)

use std::sync::Arc;
use tracing::warn;

use crate::scoring::forward_to_upstream;
use crate::AppState;

/// Handle an X3 RPC request.
pub async fn handle_request(state: &Arc<AppState>, method: &str, body: &str) -> anyhow::Result<String> {
    let chain_name = "x3";

    // ── Quorum-required methods ────────────────────────────────────
    let quorum_methods = [
        "x3_getAtomicRoute",
        "x3_getProof",
        "x3_getFinalizedState",
        "x3_getSettlementStatus",
        "x3_verifyBridgeDeposit",
    ];

    if quorum_methods.contains(&method) || state.config.requires_quorum(method, chain_name) {
        return crate::quorum::quorum_call(state, chain_name, method, body).await;
    }

    // ── Transaction broadcast ─────────────────────────────────────
    if method == "x3_submitExtrinsic" {
        return crate::tx_broadcast::controlled_broadcast(state, chain_name, method, body).await;
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
                warn!(upstream = %upstream.id, error = %e, "X3 upstream failed, trying next");
            }
        }
    }

    Err(anyhow::anyhow!("No healthy X3 upstream available"))
}

/// Handle WebSocket forward for X3 (Substrate JSON-RPC over WS).
pub async fn ws_forward(state: &Arc<AppState>, _method: &str, body: &str) -> anyhow::Result<String> {
    let chain_name = "x3";

    // X3 Substrate uses WS URLs natively
    let providers = state.pool.healthy_for_chain(chain_name);
    for upstream in &providers {
        // Substrate nodes use ws:// URLs
        if upstream.url.starts_with("ws://") || upstream.url.starts_with("wss://") {
            return forward_to_upstream(&upstream.url, body).await;
        }
    }

    // Fallback to any URL
    if let Some(best) = state.pool.best_for_chain(chain_name, None) {
        return forward_to_upstream(&best.url, body).await;
    }

    Err(anyhow::anyhow!("No WebSocket-capable X3 upstream available"))
}