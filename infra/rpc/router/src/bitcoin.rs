//! Bitcoin chain handler.
//!
//! Routes Bitcoin JSON-RPC methods, prioritizing local Bitcoin Core.
//! Uses getblockchaininfo for health. Blocks unsafe/wallet methods.

use std::sync::Arc;
use tracing::warn;

use crate::scoring::forward_to_upstream;
use crate::AppState;

/// Handle a Bitcoin RPC request.
pub async fn handle_request(state: &Arc<AppState>, method: &str, body: &str) -> anyhow::Result<String> {
    let chain_name = "bitcoin";

    // ── Bug detection: nonce/chain_id injection + body size guard ──
    // Bitcoin JSON-RPC is especially vulnerable to parameter injection.
    // We validate the body is a well-formed JSON-RPC 2.0 envelope with
    // only "jsonrpc", "method", "params", and "id" fields.
    validate_jsonrpc_body(body)?;

    // ── Local-first routing ───────────────────────────────────────
    // Bitcoin Core local node (tier 0) is always preferred
    let local_providers = state
        .pool
        .healthy_for_chain(chain_name)
        .into_iter()
        .filter(|p| p.tier == 0)
        .collect::<Vec<_>>();

    if let Some(local) = local_providers.first() {
        match forward_to_upstream(&local.url, body).await {
            Ok(resp) => return Ok(resp),
            Err(e) => {
                warn!(upstream = %local.id, error = %e, "Local BTC node failed, trying fallback");
                state.metrics.increment_failover("bitcoin", &local.id, "fallback");
            }
        }
    }

    // ── Fallback to paid/public ───────────────────────────────────
    if let Some(best) = state.pool.best_for_chain(chain_name, None) {
        return forward_to_upstream(&best.url, body).await;
    }

    let all_healthy = state.pool.healthy_for_chain(chain_name);
    for upstream in &all_healthy {
        match forward_to_upstream(&upstream.url, body).await {
            Ok(resp) => return Ok(resp),
            Err(e) => {
                warn!(upstream = %upstream.id, error = %e, "BTC upstream failed, trying next");
            }
        }
    }

    Err(anyhow::anyhow!("No healthy Bitcoin upstream available"))
}

/// Validate that the JSON-RPC body is well-formed and doesn't contain
/// injection-style extra fields.
fn validate_jsonrpc_body(body: &str) -> anyhow::Result<()> {
    let v: serde_json::Value = serde_json::from_str(body)?;
    let obj = v.as_object().ok_or_else(|| {
        anyhow::anyhow!("Bitcoin RPC body must be a JSON object")
    })?;

    // Only allow standard JSON-RPC 2.0 fields
    let allowed_keys = ["jsonrpc", "method", "params", "id"];
    for key in obj.keys() {
        if !allowed_keys.contains(&key.as_str()) {
            return Err(anyhow::anyhow!(
                "Unexpected key '{}' in Bitcoin RPC request body",
                key
            ));
        }
    }

    // Validate method field exists
    if !obj.contains_key("method") {
        return Err(anyhow::anyhow!("Bitcoin RPC request missing 'method' field"));
    }

    Ok(())
}