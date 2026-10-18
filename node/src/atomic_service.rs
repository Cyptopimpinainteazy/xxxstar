//! Node-side atomic gateway service.
//!
//! Holds the configured X3-lang gateway key, tracks the gateway's extrinsic
//! nonce, and submits atomic-kernel extrinsics through the node's own
//! transaction pool.

use crate::atomic_gateway::AtomicGatewayKey;
use crate::finality_certs::{
    decide_finalization_cert, FinalizationCertificate, ObservedFinalityCerts,
};
use crate::service::FullClient;
use atomic_swap_orchestrator::{
    kernel_compatible_receipt_root, AtomicExecutionRequest, AtomicLegExecution, KernelBundleLeg,
    KernelReceiptRootData, KernelVmType,
};
use codec::Encode;
use pallet_x3_atomic_kernel::chain_legs::{LegExecution, MAX_LEG_PROGRAM_BYTES};
use pallet_x3_atomic_kernel::BundleStatus;
use pallet_x3_atomic_kernel::X3AtomicKernelApi;
use sc_client_api::{BlockBackend, HeaderBackend};
use sc_transaction_pool_api::{TransactionPool, TransactionSource};
use sp_api::ProvideRuntimeApi;
use sp_core::H256;
use sp_runtime::traits::SaturatedConversion;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;
use x3_chain_runtime::opaque::Block;
use x3_chain_runtime::AccountId;

/// Transaction pool type used by the node atomic gateway service.
pub type AtomicPool = sc_transaction_pool::TransactionPoolHandle<Block, FullClient>;

/// A request accepted by the atomic gateway service.
#[derive(Debug, Clone)]
pub enum AtomicGatewayCommand {
    /// Submit a canonical execution request on-chain.
    SubmitBundle(AtomicExecutionRequest),
    /// Submit, assign, and execute, but wait for an explicit rollback/finalize.
    SubmitBundleHoldForRollback(AtomicExecutionRequest),
    /// Assign the gateway account as executor of a pending bundle.
    AssignExecutor {
        /// On-chain bundle identifier.
        bundle_id: H256,
    },
    /// Roll back a submitted bundle.
    Rollback {
        /// On-chain bundle identifier.
        bundle_id: H256,
        /// Reason recorded on-chain.
        reason: pallet_x3_atomic_kernel::BundleRollbackReason,
    },
}

/// Service that signs and submits atomic-kernel calls.
pub struct AtomicGatewayService {
    key: AtomicGatewayKey,
    client: Arc<FullClient>,
    pool: Arc<AtomicPool>,
    genesis_hash: H256,
    next_tx_nonce: Arc<AtomicU64>,
    /// The certificates this node's finality tasks observed, per block (TICKET-107).
    observed_certs: ObservedFinalityCerts,
}

impl AtomicGatewayService {
    /// Create the service. `uri` is the sr25519 secret URI for the runtime's
    /// configured `X3LangGatewayAccount`.
    pub fn new(
        uri: &str,
        client: Arc<FullClient>,
        pool: Arc<AtomicPool>,
        observed_certs: ObservedFinalityCerts,
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
            observed_certs,
        })
    }

    /// Run the service until the command channel closes.
    pub async fn run(self, mut commands: mpsc::Receiver<AtomicGatewayCommand>) {
        while let Some(cmd) = commands.recv().await {
            if let Err(e) = self.handle(cmd).await {
                log::error!(target: "x3-atomic-gateway", "atomic gateway command failed: {e}");
            }
        }
    }

    async fn handle(&self, command: AtomicGatewayCommand) -> Result<(), String> {
        let tx_nonce = self.next_tx_nonce.fetch_add(1, Ordering::Relaxed) as u32;
        let (extrinsic, legs_hash, executions, request_clone, should_finalize) = match command {
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
                    true,
                )
            }
            AtomicGatewayCommand::SubmitBundleHoldForRollback(request) => {
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
                    false,
                )
            }
            AtomicGatewayCommand::AssignExecutor { bundle_id } => (
                self.key
                    .assign_bundle_executor(bundle_id, self.genesis_hash, tx_nonce)?,
                None,
                None,
                None,
                false,
            ),
            AtomicGatewayCommand::Rollback { bundle_id, reason } => (
                self.key
                    .rollback_atomic_bundle(bundle_id, reason, self.genesis_hash, tx_nonce)?,
                None,
                None,
                None,
                false,
            ),
        };
        let best_hash = self.client.info().best_hash;
        let at = best_hash;
        self.pool
            .submit_one(at, TransactionSource::External, extrinsic.into())
            .await
            .map_err(|e| format!("transaction pool rejected atomic extrinsic: {e}"))
            .map(|_| ())?;
        log::info!(target: "x3-atomic-gateway", "atomic extrinsic submitted");

        if let Some(legs_hash) = legs_hash {
            let bundle_id = self.wait_for_submission_and_assign(legs_hash).await?;
            if let Some(executions) = executions {
                log::info!(target: "x3-atomic-gateway", "bundle {bundle_id:?} executing {}/{} legs", executions.len(), executions.len());
                self.execute_legs(&executions, bundle_id).await?;
                if should_finalize {
                    if let Some(request) = request_clone {
                        self.finalize_bundle(&request, bundle_id).await?;
                    }
                }
            }
        }
        Ok(())
    }

    async fn wait_for_submission_and_assign(&self, legs_hash: H256) -> Result<H256, String> {
        for _ in 0..50 {
            let at = self.client.info().best_hash;
            let submitter = self.key.account();
            let found = self
                .client
                .runtime_api()
                .find_bundle(at, submitter, legs_hash)
                .map_err(|e| format!("find_bundle runtime call failed: {e}"))?;

            if let Some((bundle_id, BundleStatus::Pending)) = found {
                let assign_nonce = self.next_tx_nonce.fetch_add(1, Ordering::Relaxed) as u32;
                let extrinsic =
                    self.key
                        .assign_bundle_executor(bundle_id, self.genesis_hash, assign_nonce)?;
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

    /// Have the chain execute every leg. Each leg is a signed `execute_bundle_leg` from the
    /// bundle's executor: the runtime runs it against chain state and records the journal a
    /// rollback undoes, so nothing this node computes decides what a leg wrote.
    async fn execute_legs(
        &self,
        executions: &[AtomicLegExecution],
        bundle_id: H256,
    ) -> Result<(), String> {
        let gateway = self.key.account();
        let chain_legs = executions
            .iter()
            .enumerate()
            .map(|(index, execution)| chain_leg(execution, &gateway, index))
            .collect::<Result<Vec<_>, _>>()?;
        for (index, execution) in chain_legs.into_iter().enumerate() {
            let tx_nonce = self.next_tx_nonce.fetch_add(1, Ordering::Relaxed) as u32;
            let extrinsic = self.key.execute_bundle_leg(
                bundle_id,
                index as u32,
                execution,
                self.genesis_hash,
                tx_nonce,
            )?;
            self.pool
                .submit_one(
                    self.client.info().best_hash,
                    TransactionSource::External,
                    extrinsic.into(),
                )
                .await
                .map_err(|e| format!("execute_bundle_leg {index} rejected: {e}"))?;
            log::info!(target: "x3-atomic-gateway", "bundle {bundle_id:?} leg {index} submitted for on-chain execution");
        }
        Ok(())
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
            let finalized_hash = info.finalized_hash;
            let anchored = match self
                .client
                .runtime_api()
                .get_finality_cert_anchor(finalized_hash, block_num)
            {
                Ok(anchor) => anchor,
                Err(e) => {
                    return Err(format!("finality cert anchor runtime call failed: {e}"));
                }
            };

            // TICKET-107. The anchor is written by an unsigned call, first-write-wins, so a peer can
            // plant a certificate for this height before any honest writer gets there. This node
            // finalizes only with a certificate *it* observed, and only once the chain agrees:
            // signing whatever the chain happened to hold would make this node attest a value no
            // voter produced. A disagreement is refused loudly instead (forgery becomes a bounded
            // liveness failure that names the block).
            let finality_cert =
                match decide_finalization_cert(self.observed_certs.get(block_num), anchored) {
                    FinalizationCertificate::Finalize(cert) => cert,
                    FinalizationCertificate::Wait => {
                        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                        continue;
                    }
                    FinalizationCertificate::Poisoned { observed, anchored } => {
                        return Err(format!(
                            "refusing to finalize bundle {bundle_id:?} at block {block_num}: the \
                             chain's anchor is 0x{} but this node observed 0x{} — a planted anchor \
                             is not something this node will sign",
                            hex::encode(anchored.as_bytes()),
                            hex::encode(observed.as_bytes())
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
            let finalize_nonce = self.next_tx_nonce.fetch_add(1, Ordering::Relaxed) as u32;
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
                .map(|_| log::info!(target: "x3-atomic-gateway", "bundle {bundle_id:?} finalized"))
                .map_err(|e| format!("finalize_atomic_bundle rejected: {e}"));
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

/// The chain-executed form of a requested leg, or why the chain cannot run it.
///
/// An X3 leg's payload is its X3BC program. A transfer moves the gateway's own funds of the X3
/// native ledger asset (0): the chain moves only the bundle executor's (the gateway's) funds, so a transfer from
/// any other account, or on any other VM's balances, is refused here instead of being simulated.
pub fn chain_leg(
    execution: &AtomicLegExecution,
    gateway: &AccountId,
    index: usize,
) -> Result<LegExecution<AccountId>, String> {
    match execution {
        AtomicLegExecution::X3 { payload, .. } => {
            let program = payload.clone().try_into().map_err(|_| {
                format!(
                    "leg {index}: X3 program of {} bytes exceeds the chain's {MAX_LEG_PROGRAM_BYTES}",
                    payload.len()
                )
            })?;
            Ok(LegExecution::X3 { program })
        }
        AtomicLegExecution::Transfer {
            vm: KernelVmType::X3,
            from,
            to,
            amount,
        } => {
            if from.as_slice() != AsRef::<[u8]>::as_ref(gateway) {
                return Err(format!(
                    "leg {index}: the chain moves only the gateway's own funds; a transfer from \
                     another account cannot execute"
                ));
            }
            let to: [u8; 32] = to.as_slice().try_into().map_err(|_| {
                format!("leg {index}: transfer recipient must be a 32-byte account")
            })?;
            Ok(LegExecution::Transfer {
                asset: X3_NATIVE_LEDGER_ASSET,
                to: AccountId::new(to),
                amount: *amount,
            })
        }
        AtomicLegExecution::Transfer { vm, .. } => Err(format!(
            "leg {index}: {vm:?} balances are not on the canonical ledger the chain executes \
             transfers against"
        )),
        AtomicLegExecution::Evm { .. } | AtomicLegExecution::Svm { .. } => Err(format!(
            "leg {index}: the chain executes X3 programs and ledger transfers; EVM/SVM legs are \
             not executable on chain"
        )),
    }
}

/// The canonical-ledger asset of the X3 domain's native balance.
pub const X3_NATIVE_LEDGER_ASSET: u32 = 0;

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

    fn gateway() -> AccountId {
        AtomicGatewayKey::from_uri("//x3-atomic-gateway")
            .expect("key loads")
            .account()
    }

    #[test]
    fn an_x3_leg_runs_its_program_on_chain() {
        let leg = AtomicLegExecution::X3 {
            caller: [7; 32],
            selector: [1, 2, 3, 4],
            payload: vec![9, 8, 7],
        };
        assert_eq!(
            chain_leg(&leg, &gateway(), 0),
            Ok(LegExecution::X3 {
                program: vec![9, 8, 7].try_into().expect("small"),
            })
        );
        let huge = AtomicLegExecution::X3 {
            caller: [7; 32],
            selector: [0; 4],
            payload: vec![0; MAX_LEG_PROGRAM_BYTES as usize + 1],
        };
        assert!(chain_leg(&huge, &gateway(), 1).is_err());
    }

    #[test]
    fn a_transfer_leg_moves_only_the_gateways_own_x3_funds() {
        let gateway = gateway();
        let own = AtomicLegExecution::Transfer {
            vm: KernelVmType::X3,
            from: AsRef::<[u8]>::as_ref(&gateway).to_vec(),
            to: vec![5; 32],
            amount: 42,
        };
        assert_eq!(
            chain_leg(&own, &gateway, 0),
            Ok(LegExecution::Transfer {
                asset: X3_NATIVE_LEDGER_ASSET,
                to: AccountId::new([5; 32]),
                amount: 42,
            })
        );
        let someone_elses = AtomicLegExecution::Transfer {
            vm: KernelVmType::X3,
            from: vec![6; 32],
            to: vec![5; 32],
            amount: 42,
        };
        assert!(chain_leg(&someone_elses, &gateway, 0).is_err());
        let evm = AtomicLegExecution::Transfer {
            vm: KernelVmType::Evm,
            from: vec![1; 20],
            to: vec![2; 20],
            amount: 1,
        };
        assert!(chain_leg(&evm, &gateway, 0).is_err());
        let short_recipient = AtomicLegExecution::Transfer {
            vm: KernelVmType::X3,
            from: AsRef::<[u8]>::as_ref(&gateway).to_vec(),
            to: vec![5; 20],
            amount: 1,
        };
        assert!(chain_leg(&short_recipient, &gateway, 0).is_err());
    }

    #[test]
    fn evm_and_svm_legs_are_refused_rather_than_simulated() {
        let evm = AtomicLegExecution::Evm {
            caller: [1; 20],
            target: [2; 20],
            value: 0,
            input: vec![],
        };
        assert!(chain_leg(&evm, &gateway(), 0).is_err());
    }
}
