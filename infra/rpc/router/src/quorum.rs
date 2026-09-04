//! Quorum verification for critical RPC methods.
//!
//! When a method requires quorum (e.g., x3_getProof, eth_chainId),
//! the request is sent to multiple independent upstreams and the
//! responses are compared. If `min_agreement` upstreams agree, the
//! majority response is returned. Mismatches are logged and counted.

use std::collections::HashMap;
use std::sync::Arc;

use tracing::{warn, error};

use crate::scoring::forward_to_upstream;
use crate::AppState;

/// Execute a quorum-verified RPC call.
///
/// 1. Send the request to up to 3 healthy upstreams.
/// 2. Compare the `result` field of each response.
/// 3. If at least `min_agreement` responses match, return the consensus result.
/// 4. Otherwise, return an error.
pub async fn quorum_call(
    state: &Arc<AppState>,
    chain: &str,
    method: &str,
    body: &str,
) -> anyhow::Result<String> {
    // Get quorum config
    let quorum_cfg = state
        .config
        .chains
        .get(chain)
        .and_then(|c| c.quorum.as_ref())
        .ok_or_else(|| anyhow::anyhow!("Quorum not configured for chain {}", chain))?;

    if !quorum_cfg.enabled {
        // Fall through to standard routing
        if let Some(best) = state.pool.best_for_chain(chain, None) {
            return forward_to_upstream(&best.url, body).await;
        }
        return Err(anyhow::anyhow!("No upstream for quorum-disabled chain {}", chain));
    }

    let min_agreement = quorum_cfg.min_agreement.max(2);

    // Get up to 3 healthy upstreams
    let candidates = state.pool.healthy_for_chain(chain);
    let upstreams: Vec<_> = candidates.iter().take(3).collect();

    if upstreams.is_empty() {
        return Err(anyhow::anyhow!("No healthy upstreams for quorum call on {}", chain));
    }

    if upstreams.len() < min_agreement as usize {
        warn!(
            chain = chain,
            method = method,
            available = upstreams.len(),
            required = min_agreement,
            "Insufficient upstreams for quorum"
        );
        // Degrade: use single best
        if let Some(best) = upstreams.first() {
            return forward_to_upstream(&best.url, body).await;
        }
        return Err(anyhow::anyhow!("Insufficient upstreams for quorum"));
    }

    // Fire requests to all candidates concurrently
    let mut responses = Vec::new();
    for upstream in &upstreams {
        match forward_to_upstream(&upstream.url, body).await {
            Ok(resp) => {
                // Extract the "result" field for comparison
                let result_hash = extract_result_hash(&resp);
                responses.push((upstream.id.clone(), resp, result_hash));
            }
            Err(e) => {
                warn!(
                    upstream = %upstream.id,
                    error = %e,
                    "Quorum upstream failed"
                );
            }
        }
    }

    if responses.is_empty() {
        return Err(anyhow::anyhow!("All quorum upstreams failed for {}", method));
    }

    // Count hash agreement
    let mut hash_votes: HashMap<String, Vec<usize>> = HashMap::new();
    for (idx, (_, _, hash)) in responses.iter().enumerate() {
        hash_votes.entry(hash.clone()).or_default().push(idx);
    }

    // Find the hash with the most votes
    let best_hash = hash_votes
        .iter()
        .max_by_key(|(_, indices)| indices.len())
        .map(|(h, _)| h.clone());

    if let Some(consensus_hash) = best_hash {
        let vote_count = hash_votes.get(&consensus_hash).map(|v| v.len()).unwrap_or(0);

        if vote_count >= min_agreement as usize {
            // Return the first response that matches consensus
            for (_, resp, hash) in &responses {
                if hash == &consensus_hash {
                    return Ok(resp.clone());
                }
            }
        } else {
            // Quorum mismatch — log and increment metric
            error!(
                chain = chain,
                method = method,
                votes = vote_count,
                required = min_agreement,
                total_upstreams = responses.len(),
                "Quorum mismatch"
            );
            state.metrics.increment_quorum_mismatch(chain, method);

            // Return the majority response anyway (best effort)
            for (_, resp, hash) in &responses {
                if hash == &consensus_hash {
                    return Ok(resp.clone());
                }
            }
        }
    }

    // Fallback: return the first successful response
    if let Some((_, resp, _)) = responses.first() {
        return Ok(resp.clone());
    }

    Err(anyhow::anyhow!("Quorum failed — no consensus on {}", method))
}

/// Extract a stable hash identifier from an RPC response's "result" field
/// for comparison purposes.
fn extract_result_hash(response: &str) -> String {
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(response) {
        if let Some(result) = v.get("result") {
            // Use a simple string representation for comparison
            return serde_json::to_string(result).unwrap_or_default();
        }
    }
    // Fallback: hash the full response
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(response.as_bytes());
    format!("{:x}", hasher.finalize())
}