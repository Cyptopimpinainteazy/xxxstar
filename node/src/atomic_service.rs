//! Node-side atomic gateway service.
//!
//! Holds the configured X3-lang gateway key, tracks the gateway's extrinsic
//! nonce, and submits atomic-kernel extrinsics through the node's own
//! transaction pool.

use crate::atomic_gateway::AtomicGatewayKey;
use crate::service::FullClient;
use atomic_swap_orchestrator::{
    AtomicExecutionRequest, KernelBundleLeg, KernelVmType,
};
use pallet_x3_atomic_kernel::X3AtomicKernelApi;
use sc_client_api::{BlockBackend, HeaderBackend};
use sp_api::ProvideRuntimeApi;
use sc_transaction_pool_api::{TransactionPool, TransactionSource};
use sp_core::H256;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;
use x3_chain_runtime::opaque::Block;
use pallet_x3_atomic_kernel::BundleStatus;

/// Transaction pool type used by the node atomic gateway service.
pub type AtomicPool = sc_transaction_pool::TransactionPoolHandle<Block, FullClient>;

/// A request accepted by the atomic gateway service.
#[derive(Debug, Clone)]
pub enum AtomicGatewayCommand {
    /// Submit a canonical execution request on-chain.
    SubmitBundle(AtomicExecutionRequest),
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
        let (extrinsic, legs_hash) = match command {
            AtomicGatewayCommand::SubmitBundle(request) => {
                let legs_hash = request.legs_hash();
                let legs = request
                    .legs
                    .iter()
                    .map(to_pallet_leg)
                    .collect::<Result<Vec<_>, _>>()?;
                let extrinsic = self.key.submit_atomic_bundle(
                    legs,
                    request.deadline_blocks,
                    request.chain_id,
                    request.nonce,
                    self.genesis_hash,
                    tx_nonce,
                )?;
                (extrinsic, Some(legs_hash))
            }
            AtomicGatewayCommand::AssignExecutor { bundle_id } => (
                self.key
                    .assign_bundle_executor(bundle_id, self.genesis_hash, tx_nonce)?,
                None,
            ),
        };
        let best_hash = self.client.info().best_hash;
        let at = best_hash;
        self.pool
            .submit_one(at, TransactionSource::External, extrinsic.into())
            .await
            .map_err(|e| format!("transaction pool rejected atomic extrinsic: {e}"))
            .map(|_| ())?;

        if let Some(legs_hash) = legs_hash {
            self.wait_for_submission_and_assign(legs_hash).await?;
        }
        Ok(())
    }

    async fn wait_for_submission_and_assign(
        &self,
        legs_hash: H256,
    ) -> Result<(), String> {
        for _ in 0..50 {
            let at = self.client.info().best_hash;
            let submitter: sp_core::crypto::AccountId32 = self.key.account().into();
            let found = self
                .client
                .runtime_api()
                .find_bundle(at, submitter, legs_hash)
                .map_err(|e| format!("find_bundle runtime call failed: {e}"))?;

            if let Some((bundle_id, BundleStatus::Pending)) = found {
                let assign_nonce =
                    self.next_tx_nonce.fetch_add(1, Ordering::Relaxed) as u32;
                let extrinsic = self.key.assign_bundle_executor(
                    bundle_id,
                    self.genesis_hash,
                    assign_nonce,
                )?;
                self.pool
                    .submit_one(
                        self.client.info().best_hash,
                        TransactionSource::External,
                        extrinsic.into(),
                    )
                    .await
                    .map_err(|e| format!("assign_bundle_executor rejected: {e}"))?;
                return Ok(());
            }
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        }
        Err("bundle submission was not found on-chain within timeout".to_string())
    }
}

fn to_pallet_leg(
    leg: &KernelBundleLeg,
) -> Result<pallet_x3_atomic_kernel::proof::BundleLeg, String> {
    use pallet_x3_atomic_kernel::proof::{BundleLeg, DeclaredAccess, VmType};
    let vm_type = match leg.vm_type {
        KernelVmType::Evm => VmType::Evm,
        KernelVmType::Svm => VmType::Svm,
        KernelVmType::X3 => VmType::X3,
        KernelVmType::Cross => VmType::Cross,
    };
    let access = DeclaredAccess {
        reads: leg
            .access
            .reads
            .clone()
            .try_into()
            .map_err(|_| "access reads exceed 64 entries".to_string())?,
        writes: leg
            .access
            .writes
            .clone()
            .try_into()
            .map_err(|_| "access writes exceed 64 entries".to_string())?,
    };
    Ok(BundleLeg {
        vm_type,
        token_in: leg.token_in,
        token_out: leg.token_out,
        amount_in: leg.amount_in,
        min_amount_out: leg.min_amount_out,
        deadline: leg.deadline,
        access,
    })
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
