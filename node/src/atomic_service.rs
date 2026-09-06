//! Node-side atomic gateway service.
//!
//! Holds the configured X3-lang gateway key, tracks the gateway's extrinsic
//! nonce, and submits atomic-kernel extrinsics through the node's own
//! transaction pool.

use crate::atomic_gateway::AtomicGatewayKey;
use crate::service::FullClient;
use sc_client_api::{BlockBackend, HeaderBackend};
use sc_transaction_pool_api::{TransactionPool, TransactionSource};
use sp_core::H256;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;
use x3_chain_runtime::opaque::Block;

/// Transaction pool type used by the node atomic gateway service.
pub type AtomicPool = sc_transaction_pool::TransactionPoolHandle<Block, FullClient>;

/// A request accepted by the atomic gateway service.
#[derive(Debug, Clone)]
pub enum AtomicGatewayCommand {
    /// Submit a new bundle on-chain.
    SubmitBundle {
        /// Kernel accounting legs to record on-chain.
        legs: Vec<pallet_x3_atomic_kernel::proof::BundleLeg>,
        /// Submission deadline in X3 blocks.
        deadline_blocks: u32,
        /// Chain id for the pallet nonce registry.
        chain_id: u32,
        /// Strictly-increasing per chain/account nonce.
        nonce: u64,
    },
    /// Assign the gateway account as executor of a pending bundle.
    AssignExecutor {
        /// On-chain bundle identifier.
        bundle_id: H256,
    },
}

/// Service that signs and submits atomic-kernel calls.
pub struct AtomicGatewayService {
    key: AtomicGatewayKey,
    client: Arc<FullClient>,
    pool: Arc<AtomicPool>,
    genesis_hash: H256,
    next_tx_nonce: Arc<AtomicU64>,
}

impl AtomicGatewayService {
    /// Create the service. `uri` is the sr25519 secret URI for the runtime's
    /// configured `X3LangGatewayAccount`.
    pub fn new(
        uri: &str,
        client: Arc<FullClient>,
        pool: Arc<AtomicPool>,
    ) -> Result<Self, String> {
        let key = AtomicGatewayKey::from_uri(uri)?;
        let genesis_hash = client
            .block_hash(0)
            .map_err(|e| format!("failed to read genesis hash: {e}"))?
            .ok_or_else(|| "genesis block not found".to_string())?;
        Ok(Self {
            key,
            client,
            pool,
            genesis_hash,
            next_tx_nonce: Arc::new(AtomicU64::new(0)),
        })
    }

    /// Run the service until the command channel closes.
    pub async fn run(
        self,
        mut commands: mpsc::Receiver<AtomicGatewayCommand>,
    ) {
        while let Some(cmd) = commands.recv().await {
            if let Err(e) = self.handle(cmd).await {
                log::error!(target: "x3-atomic-gateway", "atomic gateway command failed: {e}");
            }
        }
    }

    async fn handle(&self, command: AtomicGatewayCommand) -> Result<(), String> {
        let tx_nonce = self.next_tx_nonce.fetch_add(1, Ordering::Relaxed) as u32;
        let extrinsic = match command {
            AtomicGatewayCommand::SubmitBundle {
                legs,
                deadline_blocks,
                chain_id,
                nonce,
            } => self.key.submit_atomic_bundle(
                legs,
                deadline_blocks,
                chain_id,
                nonce,
                self.genesis_hash,
                tx_nonce,
            )?,
            AtomicGatewayCommand::AssignExecutor { bundle_id } => self
                .key
                .assign_bundle_executor(bundle_id, self.genesis_hash, tx_nonce)?,
        };
        let best_hash = self.client.info().best_hash;
        let at = best_hash;
        self.pool
            .submit_one(at, TransactionSource::External, extrinsic.into())
            .await
            .map(|_| ())
            .map_err(|e| format!("transaction pool rejected atomic extrinsic: {e}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use codec::Encode;

    #[test]
    fn gateway_key_loads_well_known_dev_uri() {
        let key = AtomicGatewayKey::from_uri("//x3-atomic-gateway").expect("key loads");
        assert!(!key.account().encode().is_empty());
    }

    #[test]
    fn gateway_command_encode_is_stable_shape() {
        let cmd = AtomicGatewayCommand::AssignExecutor {
            bundle_id: H256([0xAB; 32]),
        };
        let encoded = match cmd {
            AtomicGatewayCommand::AssignExecutor { bundle_id } => bundle_id.encode(),
            _ => unreachable!(),
        };
        assert_eq!(encoded.len(), 32);
    }
}
