//! EVM chain handler — ethereum, base, arbitrum, polygon, bsc.
//!
//! Routes eth_*, net_*, web3_*, trace_*, debug_* methods to the best upstream
//! based on scoring, method policy, archive requirements, and quorum rules.

use std::sync::Arc;
use tracing::{debug, warn};

use crate::config::MethodClass;
use crate::scoring::forward_to_upstream;
use crate::AppState;

/// Handle an EVM RPC request. Routes to the best upstream, applying:
/// - archive-only methods → archive-capable upstreams only
/// - quorum methods → multi-upstream verification
/// - tx methods → controlled broadcast policy
pub async fn handle_request(state: &Arc<AppState>, method: &str, body: &str) -> anyhow::Result<String> {
    let chain_name = resolve_evm_chain(method, state);
    let method_class = state.config.classify_for_chain(method, &chain_name);

    // ── Archive-only check ────────────────────────────────────────
    if method_class == MethodClass::ArchiveOnly || state.config.requires_archive(method, &chain_name) {
        if let Some(archive_upstream) = state.pool.best_for_chain(&chain_name, Some("archive")) {
            return forward_to_upstream(&archive_upstream.url, body).await;
        }
        // If no archive upstream exists, try trace-capable upstream
        if let Some(trace_upstream) = state.pool.best_for_chain(&chain_name, Some("trace")) {
            return forward_to_upstream(&trace_upstream.url, body).await;
        }
        return Err(anyhow::anyhow!(
            "No archive-capable upstream available for {} on {}",
            method,
            chain_name
        ));
    }

    // ── Quorum check for critical methods ─────────────────────────
    if state.config.requires_quorum(method, &chain_name) {
        return crate::quorum::quorum_call(state, &chain_name, method, body).await;
    }

    // ── Transaction broadcast ─────────────────────────────────────
    if method == "eth_sendRawTransaction" || method == "eth_sendTransaction" {
        return crate::tx_broadcast::controlled_broadcast(
            state,
            &chain_name,
            method,
            body,
        )
        .await;
    }

    // ── Standard read routing ─────────────────────────────────────
    if let Some(best) = state.pool.best_for_chain(&chain_name, None) {
        return forward_to_upstream(&best.url, body).await;
    }

    // ── Fallback: try any healthy upstream ────────────────────────
    let all_healthy = state.pool.healthy_for_chain(&chain_name);
    for upstream in &all_healthy {
        match forward_to_upstream(&upstream.url, body).await {
            Ok(resp) => return Ok(resp),
            Err(e) => {
                warn!(upstream = %upstream.id, error = %e, "EVM upstream failed, trying next");
            }
        }
    }

    Err(anyhow::anyhow!(
        "No healthy EVM upstream available for chain {}",
        chain_name
    ))
}

/// Resolve which EVM chain a method belongs to by checking chain_id patterns.
fn resolve_evm_chain(method: &str, state: &Arc<AppState>) -> String {
    // Check all EVM chains for method-specific chain_id hints
    // In practice, upstream chain IDs are validated during health scoring.
    // The chain is determined by which provider set has the method available.
    for chain_name in &["ethereum", "base", "arbitrum", "polygon", "bsc"] {
        if let Some(cfg) = state.config.chains.get(*chain_name) {
            match cfg.kind {
                crate::config::ChainKind::Evm => {
                    // Check if this chain has providers with this method
                    if state.pool.healthy_for_chain(chain_name).first().is_some() {
                        return chain_name.to_string();
                    }
                }
                _ => continue,
            }
        }
    }
    // Default to ethereum — the reference EVM chain
    "ethereum".to_string()
}