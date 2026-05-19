//! Relayer/Watcher — monitors chain events and relays secrets between VMs.
//!
//! The relayer is the off-chain component that:
//! 1. Watches HTLC events on all chains
//! 2. When a claim reveals the secret on one chain, immediately claims on the other
//! 3. Monitors block confirmations
//! 4. Triggers refunds when timelocks expire
//! 5. Resubmits stuck transactions with higher gas/priority fees

use crate::htlc::HtlcChainAdapter;
use crate::types::*;
use std::sync::Arc;
use tracing::{error, info, warn};

/// Event emitted when an HTLC is claimed (secret revealed).
#[derive(Debug, Clone)]
pub struct HtlcClaimEvent {
    pub htlc_id: HtlcId,
    pub secret: HtlcSecret,
    pub vm: VmTarget,
    pub tx_hash: Vec<u8>,
    pub block_number: u64,
}

/// Event emitted when an HTLC is created.
#[derive(Debug, Clone)]
pub struct HtlcCreateEvent {
    pub htlc_id: HtlcId,
    pub hash_lock: HtlcHash,
    pub vm: VmTarget,
    pub amount: u128,
    pub timelock: u64,
}

/// Relayer configuration.
#[derive(Debug, Clone)]
pub struct RelayerConfig {
    /// Polling interval for event queries (milliseconds).
    pub poll_interval_ms: u64,
    /// Maximum time to wait for a claim relay (milliseconds).
    pub max_relay_wait_ms: u64,
    /// Gas/priority fee bump factor for stuck transactions.
    pub fee_bump_factor: f64,
    /// Maximum retries for relaying a claim.
    pub max_retries: u32,
}

impl Default for RelayerConfig {
    fn default() -> Self {
        Self {
            poll_interval_ms: 500,     // 500ms polling
            max_relay_wait_ms: 30_000, // 30s max relay time
            fee_bump_factor: 1.25,     // 25% fee bump
            max_retries: 5,
        }
    }
}

/// Cross-chain relayer that watches events and relays secrets.
pub struct CrossChainRelayer {
    config: RelayerConfig,
    fast_adapter: Arc<dyn HtlcChainAdapter>,
    slow_adapter: Arc<dyn HtlcChainAdapter>,
}

impl CrossChainRelayer {
    pub fn new(
        config: RelayerConfig,
        fast_adapter: Arc<dyn HtlcChainAdapter>,
        slow_adapter: Arc<dyn HtlcChainAdapter>,
    ) -> Self {
        Self {
            config,
            fast_adapter,
            slow_adapter,
        }
    }

    /// Relay a secret from the fast chain to claim on the slow chain.
    ///
    /// This is the critical path: when a claim event is observed on the
    /// fast chain, the relayer must immediately claim on the slow chain
    /// before the slow chain's timelock expires.
    pub async fn relay_secret_fast_to_slow(
        &self,
        slow_htlc_id: &HtlcId,
        secret: &HtlcSecret,
    ) -> Result<Vec<u8>, CoordinatorError> {
        info!(
            slow_htlc = %slow_htlc_id.to_hex(),
            slow_vm = %self.slow_adapter.vm_target(),
            "Relaying secret from fast chain to slow chain"
        );

        let mut retries = 0;
        loop {
            match self.slow_adapter.claim_htlc(slow_htlc_id, secret).await {
                Ok(tx_hash) => {
                    info!(
                        "Secret relayed to slow chain — tx: 0x{}",
                        hex::encode(&tx_hash)
                    );
                    return Ok(tx_hash);
                }
                Err(e) => {
                    retries += 1;
                    if retries >= self.config.max_retries {
                        error!("Failed to relay secret after {} retries: {}", retries, e);
                        return Err(e);
                    }
                    warn!("Relay attempt {} failed: {} — retrying", retries, e);
                    tokio::time::sleep(tokio::time::Duration::from_millis(
                        self.config.poll_interval_ms,
                    ))
                    .await;
                }
            }
        }
    }

    /// Relay a secret from the slow chain to claim on the fast chain.
    pub async fn relay_secret_slow_to_fast(
        &self,
        fast_htlc_id: &HtlcId,
        secret: &HtlcSecret,
    ) -> Result<Vec<u8>, CoordinatorError> {
        info!(
            fast_htlc = %fast_htlc_id.to_hex(),
            fast_vm = %self.fast_adapter.vm_target(),
            "Relaying secret from slow chain to fast chain"
        );

        self.fast_adapter.claim_htlc(fast_htlc_id, secret).await
    }

    /// Check if either HTLC's timelock is about to expire and trigger refunds.
    pub async fn check_timelocks(
        &self,
        fast_htlc_id: &HtlcId,
        slow_htlc_id: &HtlcId,
        fast_timelock: u64,
        slow_timelock: u64,
        safety_margin_secs: u64,
    ) -> Result<Option<String>, CoordinatorError> {
        let now = self.fast_adapter.current_time().await?;

        if now + safety_margin_secs >= fast_timelock {
            warn!("Fast chain timelock approaching — triggering refund");
            let _ = self.fast_adapter.refund_htlc(fast_htlc_id).await?;
            return Ok(Some("fast_timelock_expired".to_string()));
        }

        if now + safety_margin_secs >= slow_timelock {
            warn!("Slow chain timelock approaching — triggering refund");
            let _ = self.slow_adapter.refund_htlc(slow_htlc_id).await?;
            return Ok(Some("slow_timelock_expired".to_string()));
        }

        Ok(None)
    }

    /// Monitor HTLC confirmations on both chains.
    pub async fn poll_confirmations(
        &self,
        fast_htlc_id: &HtlcId,
        slow_htlc_id: &HtlcId,
    ) -> Result<(u32, u32), CoordinatorError> {
        let (_, fast_confs) = self.fast_adapter.query_htlc(fast_htlc_id).await?;
        let (_, slow_confs) = self.slow_adapter.query_htlc(slow_htlc_id).await?;
        Ok((fast_confs, slow_confs))
    }
}
