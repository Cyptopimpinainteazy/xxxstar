//! WebSocket subscriber for the X3 node.
//!
//! Connects to the node's WebSocket RPC endpoint, subscribes to new block
//! heads via the standard Substrate JSON-RPC method `chain_subscribeNewHeads`,
//! and on every block:
//!
//! 1. Updates `last_updated_block` in the shared dashboard.
//! 2. Sends a `system_health` JSON-RPC call to verify the node is alive.
//! 3. Refreshes all Prometheus gauges via [`crate::metrics::update_metrics`].
//! 4. If `alert_webhook` is configured and a vault has *transitioned* into the
//!    `"Frozen"` state (or `frozen_lane_count` has moved from 0 to >0), fires
//!    a HTTP POST to the webhook URL with the alert payload.
//!
//! Reconnection uses capped exponential back-off:
//! `1 s → 2 s → 4 s → 8 s → 30 s (cap)`.
//!
//! This task never panics; every error is logged with [`tracing::error!`] and
//! results in a reconnect attempt after the back-off delay.

use std::collections::HashSet;

use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::tungstenite::Message;

use crate::metrics;
use crate::state::{push_alert, Alert, AlertLevel, SharedDashboard, now_rfc3339};

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Spawn-and-forget entry point for the subscriber task.
///
/// The function loops forever: it attempts to establish a WebSocket connection
/// to `node_ws`, runs the subscription session until the connection breaks,
/// then waits a back-off delay before reconnecting.
///
/// # Arguments
///
/// * `node_ws`          – WebSocket URL of the Substrate node (e.g. `"ws://127.0.0.1:9944"`).
/// * `dashboard`        – Shared dashboard state updated on every block.
/// * `alert_webhook`    – If `Some`, the URL to POST alert JSON to when frozen transitions occur.
/// * `poll_interval_ms` – Reserved for future use (subscription-driven, not polled).
pub async fn run_subscriber(
    node_ws: String,
    dashboard: SharedDashboard,
    alert_webhook: Option<String>,
    poll_interval_ms: u64,
) {
    let _ = poll_interval_ms; // subscription-driven; kept for API compatibility

    let mut attempt: u32 = 0;
    loop {
        tracing::info!(
            url = %node_ws,
            attempt,
            "Attempting WebSocket connection to X3 node"
        );

        match run_session(&node_ws, &dashboard, &alert_webhook).await {
            Ok(()) => {
                tracing::warn!("Subscriber session ended cleanly — reconnecting immediately");
                attempt = 0;
            }
            Err(e) => {
                let delay = backoff_delay(attempt);
                tracing::error!(
                    error = %e,
                    attempt,
                    delay_secs = delay.as_secs(),
                    "Subscriber session failed; will reconnect after back-off"
                );
                tokio::time::sleep(delay).await;
                attempt = attempt.saturating_add(1);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Internals
// ---------------------------------------------------------------------------

/// Capped exponential back-off: 1 s, 2 s, 4 s, 8 s, then 30 s forever.
fn backoff_delay(attempt: u32) -> std::time::Duration {
    let secs = if attempt >= 5 { 30 } else { 1u64 << attempt };
    std::time::Duration::from_secs(secs)
}

/// Run one connection session.  Returns `Ok(())` only when the connection
/// closes cleanly.  Returns `Err(_)` on any protocol or IO failure.
async fn run_session(
    node_ws: &str,
    dashboard: &SharedDashboard,
    alert_webhook: &Option<String>,
) -> anyhow::Result<()> {
    // Connect.
    let (ws, _resp) = tokio_tungstenite::connect_async(node_ws)
        .await
        .map_err(|e| anyhow::anyhow!("WebSocket connect failed: {e}"))?;

    tracing::info!(url = %node_ws, "WebSocket connected");

    let (mut sink, mut stream) = ws.split();

    // Subscribe to new block heads.
    let sub_req = serde_json::json!({
        "id": 1,
        "jsonrpc": "2.0",
        "method": "chain_subscribeNewHeads",
        "params": []
    });
    sink.send(Message::Text(sub_req.to_string()))
        .await
        .map_err(|e| anyhow::anyhow!("Subscribe send failed: {e}"))?;

    // State for this session.
    let mut next_rpc_id: u64 = 2;
    let mut pending_health_id: Option<u64> = None;
    let mut prev_frozen_vaults: HashSet<String> = HashSet::new();
    let mut prev_frozen_lane_count: u32 = 0;
    let http_client = reqwest::Client::new();

    loop {
        // A 120-second timeout prevents the task from hanging silently when
        // the node stops sending pings/data.
        let msg = tokio::time::timeout(
            std::time::Duration::from_secs(120),
            stream.next(),
        )
        .await
        .map_err(|_| anyhow::anyhow!("WebSocket read timed out after 120 s"))?
        .ok_or_else(|| anyhow::anyhow!("WebSocket stream closed by peer"))?
        .map_err(|e| anyhow::anyhow!("WebSocket message error: {e}"))?;

        match msg {
            Message::Text(text) => {
                handle_text_message(
                    &text,
                    &mut sink,
                    dashboard,
                    alert_webhook,
                    &http_client,
                    &mut next_rpc_id,
                    &mut pending_health_id,
                    &mut prev_frozen_vaults,
                    &mut prev_frozen_lane_count,
                )
                .await;
            }

            // Respond to server pings to keep the connection alive.
            Message::Ping(data) => {
                if let Err(e) = sink.send(Message::Pong(data)).await {
                    return Err(anyhow::anyhow!("Pong send failed: {e}"));
                }
            }

            Message::Close(_) => {
                tracing::info!("WebSocket close frame received");
                return Ok(());
            }

            // Binary, Pong, Frame — ignore silently.
            _ => {}
        }
    }
}

/// Dispatch a single text-frame JSON-RPC message.
#[allow(clippy::too_many_arguments)]
async fn handle_text_message(
    text: &str,
    sink: &mut (impl SinkExt<Message, Error = tokio_tungstenite::tungstenite::Error> + Unpin),
    dashboard: &SharedDashboard,
    alert_webhook: &Option<String>,
    http_client: &reqwest::Client,
    next_rpc_id: &mut u64,
    pending_health_id: &mut Option<u64>,
    prev_frozen_vaults: &mut HashSet<String>,
    prev_frozen_lane_count: &mut u32,
) {
    let v: serde_json::Value = match serde_json::from_str(text) {
        Ok(v) => v,
        Err(e) => {
            tracing::error!("JSON parse error in WS message: {e}  raw={text}");
            return;
        }
    };

    // --- Subscription confirmation (id == 1) ---
    if v.get("id") == Some(&serde_json::json!(1)) {
        match v["result"].as_str() {
            Some(sub_id) => {
                tracing::info!(sub_id, "chain_subscribeNewHeads confirmed");
            }
            None => {
                tracing::error!("Subscription confirmation missing 'result' field: {text}");
            }
        }
        return;
    }

    // --- system_health response ---
    if let Some(hid) = *pending_health_id {
        if v.get("id") == Some(&serde_json::json!(hid)) {
            *pending_health_id = None;
            let peers = v["result"]["peers"].as_u64().unwrap_or(0);
            let is_syncing = v["result"]["isSyncing"].as_bool().unwrap_or(false);
            if is_syncing || peers == 0 {
                tracing::warn!(peers, is_syncing, "Node health check: node may be degraded");
            } else {
                tracing::debug!(peers, is_syncing, "Node health check OK");
            }
            return;
        }
    }

    // --- New block head notification ---
    if v.get("method") == Some(&serde_json::json!("chain_newHead")) {
        let block_num = v["params"]["result"]["number"]
            .as_str()
            .and_then(|hex| {
                u64::from_str_radix(hex.trim_start_matches("0x"), 16).ok()
            })
            .unwrap_or(0);

        tracing::debug!(block = block_num, "New block head received");

        // Update last_updated_block in the dashboard.
        {
            match dashboard.write() {
                Ok(mut dash) => {
                    dash.last_updated_block = block_num;
                }
                Err(poisoned) => {
                    let mut dash = poisoned.into_inner();
                    dash.last_updated_block = block_num;
                }
            }
        }

        // Sync Prometheus metrics.
        {
            let snap = match dashboard.read() {
                Ok(d) => d.clone(),
                Err(poisoned) => poisoned.into_inner().clone(),
            };
            metrics::update_metrics(&snap);
        }

        // Fire a system_health check on this block's connection.
        let health_id = *next_rpc_id;
        *next_rpc_id = next_rpc_id.wrapping_add(1);
        *pending_health_id = Some(health_id);

        let health_req = serde_json::json!({
            "id": health_id,
            "jsonrpc": "2.0",
            "method": "system_health",
            "params": []
        });
        if let Err(e) = sink.send(Message::Text(health_req.to_string())).await {
            tracing::error!("Failed to send system_health RPC call: {e}");
        }

        // Check for frozen-state transitions and fire the webhook if needed.
        maybe_send_webhook(
            dashboard,
            alert_webhook,
            http_client,
            prev_frozen_vaults,
            prev_frozen_lane_count,
            block_num,
        )
        .await;
    }
}

/// Detect frozen-state transitions and POST to the webhook when they occur.
///
/// A transition is defined as:
/// - At least one vault ID appearing in `"Frozen"` status that was not frozen
///   on the previous call, **or**
/// - `frozen_lane_count` moving from `0` to a positive value.
///
/// The function updates `prev_frozen_vaults` and `prev_frozen_lane_count` on
/// every call so the next block can detect further transitions.
async fn maybe_send_webhook(
    dashboard: &SharedDashboard,
    alert_webhook: &Option<String>,
    client: &reqwest::Client,
    prev_frozen_vaults: &mut HashSet<String>,
    prev_frozen_lane_count: &mut u32,
    block_num: u64,
) {
    let Some(webhook_url) = alert_webhook else {
        return;
    };

    // Snapshot current state under the lock.
    let (frozen_vaults, frozen_lane_count, recent_alerts) = {
        let dash = match dashboard.read() {
            Ok(d) => d.clone(),
            Err(poisoned) => poisoned.into_inner().clone(),
        };
        let frozen: HashSet<String> = dash
            .vaults
            .iter()
            .filter(|v| v.status == "Frozen")
            .map(|v| v.vault_id.clone())
            .collect();
        (frozen, dash.frozen_lane_count, dash.recent_alerts.clone())
    };

    // Compute transitions.
    let new_frozen_vaults: Vec<String> = frozen_vaults
        .difference(prev_frozen_vaults)
        .cloned()
        .collect();
    let lane_transition = frozen_lane_count > 0 && *prev_frozen_lane_count == 0;

    if !new_frozen_vaults.is_empty() || lane_transition {
        // Record alerts in the dashboard ring-buffer for the REST API to serve.
        let ts = now_rfc3339();
        for vault_id in &new_frozen_vaults {
            push_alert(
                dashboard,
                Alert {
                    level: AlertLevel::Critical,
                    message: format!("Vault {vault_id} transitioned to Frozen"),
                    block: block_num,
                    timestamp: ts.clone(),
                },
            );
        }
        if lane_transition {
            push_alert(
                dashboard,
                Alert {
                    level: AlertLevel::Critical,
                    message: format!(
                        "frozen_lane_count rose to {frozen_lane_count} at block {block_num}"
                    ),
                    block: block_num,
                    timestamp: ts.clone(),
                },
            );
        }

        let payload = serde_json::json!({
            "block": block_num,
            "new_frozen_vaults": new_frozen_vaults,
            "frozen_lane_count": frozen_lane_count,
            "recent_alerts": recent_alerts,
        });

        match client.post(webhook_url).json(&payload).send().await {
            Ok(resp) => {
                tracing::info!(
                    status = resp.status().as_u16(),
                    block = block_num,
                    "Alert webhook delivered"
                );
            }
            Err(e) => {
                tracing::error!("Alert webhook POST failed: {e}");
            }
        }
    }

    *prev_frozen_vaults = frozen_vaults;
    *prev_frozen_lane_count = frozen_lane_count;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backoff_delay_sequence() {
        // 1 s, 2 s, 4 s, 8 s, 16 s, then caps at 30 s.
        assert_eq!(backoff_delay(0).as_secs(), 1);
        assert_eq!(backoff_delay(1).as_secs(), 2);
        assert_eq!(backoff_delay(2).as_secs(), 4);
        assert_eq!(backoff_delay(3).as_secs(), 8);
        assert_eq!(backoff_delay(4).as_secs(), 16);
        assert_eq!(backoff_delay(5).as_secs(), 30);
        assert_eq!(backoff_delay(10).as_secs(), 30);
        assert_eq!(backoff_delay(u32::MAX).as_secs(), 30);
    }

    #[test]
    fn saturating_add_does_not_overflow() {
        let mut attempt: u32 = u32::MAX;
        attempt = attempt.saturating_add(1);
        assert_eq!(attempt, u32::MAX);
        assert_eq!(backoff_delay(attempt).as_secs(), 30);
    }
}
