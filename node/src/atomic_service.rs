//! Node-side atomic gateway service.
//!
//! Holds the configured X3-lang gateway key, tracks the gateway's extrinsic
//! nonce, and submits atomic-kernel extrinsics through the node's own
//! transaction pool.

use crate::atomic_gateway::AtomicGatewayKey;
use crate::service::FullClient;
use atomic_swap_orchestrator::{
    kernel_compatible_receipt_root, AtomicExecutionRequest, AtomicLegExecution,
    KernelBundleLeg, KernelReceiptRootData, KernelVmType,
};
use codec::Encode;
use pallet_x3_atomic_kernel::vm_revert::StateDiff;
use pallet_x3_atomic_kernel::vm_revert::OverlayDomain;
use pallet_x3_atomic_kernel::X3AtomicKernelApi;
use sc_client_api::{BlockBackend, HeaderBackend};
use sp_api::ProvideRuntimeApi;
use sc_transaction_pool_api::{TransactionPool, TransactionSource};
use sp_core::H256;
use sp_runtime::traits::SaturatedConversion;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;
use x3_chain_runtime::opaque::Block;
use pallet_x3_atomic_kernel::BundleStatus;
use x3_bridge_adapters::{
    overlay_state_diff_for_domain, RuntimeCrossVmDispatcher, SubstrateClientBalanceAdapter,
    SubstrateX3VmBridge,
};
use x3_cross_vm_bridge::{CrossVmCall, CrossVmDispatcher, CrossVmStatus, VmId};
use x3_chain_runtime::{Runtime, RuntimeCall, UncheckedExtrinsic};

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
    balances: Arc<SubstrateClientBalanceAdapter<FullClient, Block>>,
    dispatcher: RuntimeCrossVmDispatcher<FullClient, Block>,
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
        let runtime_bridge =
            Arc::new(SubstrateX3VmBridge::<FullClient, Block>::new(client.clone()));
        let balances = runtime_bridge.balances.clone();
        let dispatcher = RuntimeCrossVmDispatcher::<FullClient, Block>::new(client.clone())
            .with_x3vm_bridge(runtime_bridge.bridge.clone());
        Ok(Self {
            key,
            client,
            pool,
            genesis_hash,
            next_tx_nonce: Arc::new(AtomicU64::new(0)),
            balances,
            dispatcher,
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
        let (extrinsic, legs_hash, executions, request_clone) = match command {
            AtomicGatewayCommand::SubmitBundle(request) => {
                let legs_hash = request.legs_hash();
                let request_clone = request.clone();
                let executions = request.executions.clone();
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
                (
                    extrinsic,
                    Some(legs_hash),
                    Some(executions),
                    Some(request_clone),
                )
            }
            AtomicGatewayCommand::AssignExecutor { bundle_id } => (
                self.key
                    .assign_bundle_executor(bundle_id, self.genesis_hash, tx_nonce)?,
                None,
                None,
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
            let bundle_id = self.wait_for_submission_and_assign(legs_hash).await?;
            if let Some(executions) = executions {
                self.execute_legs(&executions, bundle_id).await?;
                if let Some(request) = request_clone {
                    self.finalize_bundle(&request, bundle_id).await?;
                }
            }
        }
        Ok(())
    }

    async fn wait_for_submission_and_assign(
        &self,
        legs_hash: H256,
    ) -> Result<H256, String> {
        for _ in 0..50 {
            let at = self.client.info().best_hash;
            let submitter = self.key.account();
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
                for _ in 0..50 {
                    let status = self
                        .client
                        .runtime_api()
                        .get_bundle_status(self.client.info().best_hash, bundle_id)
                        .map_err(|e| format!("bundle status runtime call failed: {e}"))?;
                    if matches!(status, Some(BundleStatus::Executing)) {
                        return Ok(bundle_id);
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
                }
                return Err("bundle did not enter Executing state".to_string());
            }
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        }
        Err("bundle submission was not found on-chain within timeout".to_string())
    }

    async fn execute_legs(
        &self,
        executions: &[AtomicLegExecution],
        bundle_id: H256,
    ) -> Result<(), String> {
        for (index, execution) in executions.iter().enumerate() {
            let state_diff = self.execute_x3_leg(execution, index as u64).await?;
            self.record_leg_receipt(bundle_id, index as u32, state_diff)
                .await?;
        }
        Ok(())
    }

    async fn execute_x3_leg(
        &self,
        execution: &AtomicLegExecution,
        index: u64,
    ) -> Result<StateDiff, String> {
        let AtomicLegExecution::X3 {
            caller,
            selector,
            payload,
        } = execution
        else {
            return Err(
                "Path B executes atomic legs through the X3-native overlay bridge; \
                 only X3 executions are supported"
                    .to_string(),
            );
        };
        let call = CrossVmCall::new(
            VmId::X3Vm,
            VmId::X3Vm,
            *selector,
            payload.clone(),
            10_000_000,
            index,
            10_000,
        )
        .map_err(|e| format!("failed to build CrossVmCall: {e:?}"))?;

        let receipt = self
            .dispatcher
            .execute_x3vm_tx(caller, &call)
            .map_err(|e| format!("X3 leg execution failed: {e:?}"))?;
        if receipt.status != CrossVmStatus::Success {
            return Err(format!(
                "X3 leg {} reverted: {:?}",
                index, receipt.status
            ));
        }

        let transitions = self.balances.take_overlay_transitions();
        Ok(overlay_state_diff_for_domain(&transitions, OverlayDomain::X3))
    }

    async fn record_leg_receipt(
        &self,
        bundle_id: H256,
        leg_index: u32,
        state_diff: StateDiff,
    ) -> Result<(), String> {
        let call = RuntimeCall::X3AtomicKernel(
            pallet_x3_atomic_kernel::Call::<Runtime>::record_leg_execution_receipt {
                bundle_id,
                leg_index,
                state_diff,
            },
        );
        let extrinsic: UncheckedExtrinsic = UncheckedExtrinsic::new_bare(call);
        self.pool
            .submit_one(
                self.client.info().best_hash,
                TransactionSource::External,
                extrinsic.into(),
            )
            .await
            .map(|_| ())
            .map_err(|e| format!("record_leg_execution_receipt rejected: {e}"))
    }

    async fn finalize_bundle(
        &self,
        request: &AtomicExecutionRequest,
        bundle_id: H256,
    ) -> Result<(), String> {
        for _ in 0..100 {
            let info = self.client.info();
            let block_num: u64 = info.finalized_number.saturated_into();
            if block_num == 0 {
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                continue;
            }
            let finalized_hash = info.best_hash;
            let finality_cert = match self
                .client
                .runtime_api()
                .get_finality_cert_anchor(finalized_hash, block_num)
            {
                Ok(Some(cert)) => cert,
                Ok(None) => {
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                    continue;
                }
                Err(e) => {
                    return Err(format!(
                        "finality cert anchor runtime call failed: {e}"
                    ));
                }
            };

            let submitter = self.key.account();
            let executor_hash = H256(sp_core::hashing::blake2_256(&submitter.encode()));
            let receipt_root = kernel_compatible_receipt_root(&KernelReceiptRootData {
                bundle_id,
                legs_hash: request.legs_hash(),
                leg_count: request.legs.len() as u32,
                executor_hash,
                finalized_block: block_num,
                finality_cert,
            });
            let finalize_nonce =
                self.next_tx_nonce.fetch_add(1, Ordering::Relaxed) as u32;
            let extrinsic = self.key.finalize_atomic_bundle(
                bundle_id,
                receipt_root,
                finality_cert,
                block_num as u32,
                self.genesis_hash,
                finalize_nonce,
            )?;
            return self
                .pool
                .submit_one(
                    finalized_hash,
                    TransactionSource::External,
                    extrinsic.into(),
                )
                .await
                .map(|_| ())
                .map_err(|e| format!("submit_finalization_result rejected: {e}"));
        }
        Err("no anchored finality certificate within timeout".to_string())
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
