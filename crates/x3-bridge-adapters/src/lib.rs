//! X3 Bridge Adapters
//!
//! This crate provides implementations of bridge adapters for external chains
//! (Ethereum, Solana, Bitcoin) that integrate with the X3 cross-chain gateway.

pub mod bitcoin;
pub mod ethereum;
pub mod solana;

pub use bitcoin::BitcoinBridgeAdapter;
pub use ethereum::EthereumBridgeAdapter;
pub use solana::SolanaBridgeAdapter;

/// Bridge adapter trait for external chain integration
pub trait BridgeAdapter {
    /// Get the chain name
    fn chain_name(&self) -> &str;

    /// Get the chain ID
    fn chain_id(&self) -> u64;

    /// Validate a block header
    fn validate_header(&self, header: &[u8]) -> Result<(), BridgeError>;

    /// Generate a proof for a block
    fn generate_proof(&self, block_number: u64) -> Result<Vec<u8>, BridgeError>;

    /// Get the latest block number
    fn get_latest_block_number(&self) -> Result<u64, BridgeError>;
}

/// Bridge adapter error
#[derive(Debug, thiserror::Error)]
pub enum BridgeError {
    #[error("Network error: {0}")]
    Network(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Timeout error")]
    Timeout,

    #[error("Invalid header: {0}")]
    InvalidHeader(String),

    #[error("RPC error: {0}")]
    RpcError(String),

    #[error("Empty response")]
    EmptyResponse,

    #[error("BTC adapter disabled")]
    BtcAdapterDisabled,
}

/// Make a JSON-RPC call to a chain node using the reqwest HTTP client.
///
/// Returns the `result` field of the JSON-RPC response, or a `BridgeError`.
pub fn make_json_rpc_call(
    rpc_url: &str,
    method: &str,
    params: serde_json::Value,
) -> Result<serde_json::Value, BridgeError> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| BridgeError::Network(format!("Failed to create HTTP client: {e}")))?;

    let request_body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": method,
        "params": params,
        "id": 1
    });

    let response = client
        .post(rpc_url)
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .map_err(|e| BridgeError::RpcError(format!("RPC request failed: {e}")))?;

    let rpc_response: serde_json::Value = response
        .json()
        .map_err(|e| BridgeError::Serialization(format!("Invalid JSON response: {e}")))?;

    if let Some(error) = rpc_response.get("error") {
        return Err(BridgeError::RpcError(format!(
            "RPC error: {}",
            error
                .get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("Unknown error")
        )));
    }

    rpc_response
        .get("result")
        .cloned()
        .ok_or(BridgeError::EmptyResponse)
}

// ── Substrate-backed adapters (SubstrateClientBalanceAdapter, PalletEscrowAdapter,
//    OffchainEscrowPersistence, RuntimeCrossVmDispatcher) ──────────────────────
// These are the production wiring types used by node/src/service.rs.

use codec::{Decode, Encode};
use pallet_x3_kernel::AtlasKernelRuntimeApi;
use sha2::{Digest, Sha256};
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_core::{crypto::AccountId32, offchain::OffchainStorage, H256};
use sp_runtime::traits::Block as BlockT;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::{Arc, Mutex, RwLock};
use x3_cross_vm_bridge::{CrossVmDispatcher, CrossVmResult, VmId};
use x3_vm::bridge::{BalanceProvider, BridgeConfig, CrossVmEscrow, X3VMBridge};

pub use pallet_x3_kernel::StateChange;
pub use x3_cross_vm_bridge::CrossVmDispatcher as CrossVmDispatcherTrait;

/// Build an `X3VMBridge` with canonical balance and escrow providers attached.
///
/// This is the node/runtime adapter boundary: callers supply providers backed by
/// runtime storage or pallet calls, and the returned bridge registers hostcalls
/// that route through `X3VMBridge::with_balances()` and `with_escrow()`.
pub fn build_x3vm_bridge(
    balances: Arc<dyn BalanceProvider>,
    escrow: Arc<dyn CrossVmEscrow>,
    config: BridgeConfig,
) -> X3VMBridge {
    X3VMBridge::with_config(config)
        .with_balances(balances)
        .with_escrow(escrow)
}

fn default_runtime_bridge_config() -> BridgeConfig {
    BridgeConfig {
        enable_svm: true,
        enable_evm: true,
        enable_gpu: false,
        gas_limit: 10_000_000,
        max_cpi_depth: 4,
    }
}

/// Node-side bundle that keeps the runtime-backed providers alive with the
/// `X3VMBridge` that captures them in its hostcall closures.
pub struct SubstrateX3VmBridge<C, Block, P = ()>
where
    P: EscrowPersistence,
{
    pub bridge: Arc<X3VMBridge>,
    pub balances: Arc<SubstrateClientBalanceAdapter<C, Block>>,
    pub escrow: Arc<PalletEscrowAdapter<C, Block, P>>,
}

struct OverlayEntry {
    current: u128,
    chain_snapshot: u128,
}

pub struct SubstrateClientBalanceAdapter<C, Block> {
    client: Arc<C>,
    overlay: Arc<RwLock<HashMap<Vec<u8>, OverlayEntry>>>,
    _phantom: PhantomData<Block>,
}

impl<C, Block> SubstrateClientBalanceAdapter<C, Block>
where
    Block: BlockT,
    C: ProvideRuntimeApi<Block> + HeaderBackend<Block> + Send + Sync + 'static,
    C::Api: AtlasKernelRuntimeApi<Block, AccountId32, u128, u32>,
{
    pub fn new(client: Arc<C>) -> Self {
        Self {
            client,
            overlay: Arc::new(RwLock::new(HashMap::new())),
            _phantom: PhantomData,
        }
    }

    fn best_hash(&self) -> Block::Hash {
        self.client.info().best_hash
    }

    fn fetch_from_chain(&self, address: &[u8]) -> u128 {
        let at = self.best_hash();
        let api = self.client.runtime_api();
        match address.len() {
            20 => api
                .get_evm_balance(at, address.to_vec(), 0u32)
                .unwrap_or(None)
                .unwrap_or(0),
            32 => api.get_svm_balance(at, address.to_vec()).unwrap_or(0) as u128,
            _ => 0,
        }
    }

    fn ensure_loaded(&self, address: &[u8]) -> u128 {
        {
            let guard = self.overlay.read().expect("overlay read");
            if let Some(entry) = guard.get(address) {
                return entry.current;
            }
        }
        let chain_bal = self.fetch_from_chain(address);
        let mut guard = self.overlay.write().expect("overlay write");
        guard.entry(address.to_vec()).or_insert(OverlayEntry {
            current: chain_bal,
            chain_snapshot: chain_bal,
        });
        chain_bal
    }

    pub(crate) fn credit(&self, address: &[u8], amount: u128) {
        let current = self.ensure_loaded(address);
        let mut guard = self.overlay.write().expect("overlay write");
        guard
            .get_mut(address)
            .expect("credit: address must be loaded")
            .current = current.saturating_add(amount);
    }

    pub(crate) fn debit(&self, address: &[u8], amount: u128) -> Result<(), &'static str> {
        let current = self.ensure_loaded(address);
        if current < amount {
            return Err("insufficient balance");
        }
        let mut guard = self.overlay.write().expect("overlay write");
        guard
            .get_mut(address)
            .expect("debit: address must be loaded")
            .current = current - amount;
        Ok(())
    }

    pub fn take_state_changes(&self) -> Vec<StateChange> {
        let guard = self.overlay.read().expect("overlay read");
        guard
            .iter()
            .filter(|(_, entry)| entry.current != entry.chain_snapshot)
            .map(|(addr, entry)| {
                let mut value_bytes = [0u8; 32];
                value_bytes[..16].copy_from_slice(&entry.current.to_le_bytes());
                StateChange {
                    address: addr.clone(),
                    key: H256::zero(),
                    value: H256::from(value_bytes),
                }
            })
            .collect()
    }
}

impl<C, Block> BalanceProvider for SubstrateClientBalanceAdapter<C, Block>
where
    Block: BlockT,
    C: ProvideRuntimeApi<Block> + HeaderBackend<Block> + Send + Sync + 'static,
    C::Api: AtlasKernelRuntimeApi<Block, AccountId32, u128, u32>,
{
    fn get_balance(&self, address: &[u8]) -> u128 {
        self.ensure_loaded(address)
    }

    fn transfer(&self, from: &[u8], to: &[u8], amount: u128) -> Result<(), &'static str> {
        let from_bal = self.ensure_loaded(from);
        let to_bal = self.ensure_loaded(to);
        if from_bal < amount {
            return Err("insufficient balance");
        }
        let mut guard = self.overlay.write().expect("overlay write");
        guard.get_mut(from).expect("from must be loaded").current -= amount;
        guard.get_mut(to).expect("to must be loaded").current = to_bal.saturating_add(amount);
        Ok(())
    }
}

#[derive(Clone, Encode, Decode, Debug)]
pub struct EscrowPersistedEntry {
    pub from: Vec<u8>,
    pub amount: u128,
    pub spent: bool,
}

pub trait EscrowPersistence: Send + Sync {
    fn save_ticket(&self, ticket: &[u8; 32], entry: &EscrowPersistedEntry);
    fn load_ticket(&self, ticket: &[u8; 32]) -> Option<EscrowPersistedEntry>;
}

impl EscrowPersistence for () {
    fn save_ticket(&self, _ticket: &[u8; 32], _entry: &EscrowPersistedEntry) {}
    fn load_ticket(&self, _ticket: &[u8; 32]) -> Option<EscrowPersistedEntry> {
        None
    }
}

pub struct OffchainEscrowPersistence<O> {
    storage: Mutex<O>,
}

impl<O> OffchainEscrowPersistence<O> {
    pub fn new(storage: O) -> Self {
        Self {
            storage: Mutex::new(storage),
        }
    }
}

impl<O: OffchainStorage + Send + 'static> EscrowPersistence for OffchainEscrowPersistence<O> {
    fn save_ticket(&self, ticket: &[u8; 32], entry: &EscrowPersistedEntry) {
        let mut key = [0u8; 38];
        key[..6].copy_from_slice(b"x3esc:");
        key[6..].copy_from_slice(ticket);
        let value = entry.encode();
        self.storage.lock().expect("offchain storage lock").set(
            sp_core::offchain::STORAGE_PREFIX,
            &key,
            &value,
        );
    }

    fn load_ticket(&self, ticket: &[u8; 32]) -> Option<EscrowPersistedEntry> {
        let mut key = [0u8; 38];
        key[..6].copy_from_slice(b"x3esc:");
        key[6..].copy_from_slice(ticket);
        let guard = self.storage.lock().expect("offchain storage lock");
        let bytes = guard.get(sp_core::offchain::STORAGE_PREFIX, &key)?;
        EscrowPersistedEntry::decode(&mut &bytes[..]).ok()
    }
}

struct InMemoryEscrowEntry {
    from: Vec<u8>,
    amount: u128,
    spent: bool,
}

pub struct PalletEscrowAdapter<C, Block, P = ()>
where
    P: EscrowPersistence,
{
    balances: Arc<SubstrateClientBalanceAdapter<C, Block>>,
    tickets: RwLock<HashMap<[u8; 32], InMemoryEscrowEntry>>,
    persistence: P,
}

impl<C, Block, P: EscrowPersistence> PalletEscrowAdapter<C, Block, P>
where
    Block: BlockT,
    C: ProvideRuntimeApi<Block> + HeaderBackend<Block> + Send + Sync + 'static,
    C::Api: AtlasKernelRuntimeApi<Block, AccountId32, u128, u32>,
{
    pub fn with_persistence(
        balances: Arc<SubstrateClientBalanceAdapter<C, Block>>,
        persistence: P,
    ) -> Self {
        Self {
            balances,
            tickets: RwLock::new(HashMap::new()),
            persistence,
        }
    }

    fn make_ticket(from: &[u8], amount: u128) -> [u8; 32] {
        static SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let seq = SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let mut h = Sha256::new();
        h.update(b"x3esc_lock");
        h.update(from);
        h.update(amount.to_le_bytes());
        h.update(seq.to_le_bytes());
        h.finalize().into()
    }

    fn find_ticket(&self, ticket: &[u8; 32]) -> Option<(u128, bool, Vec<u8>)> {
        {
            let guard = self.tickets.read().expect("ticket read");
            if let Some(e) = guard.get(ticket) {
                return Some((e.amount, e.spent, e.from.clone()));
            }
        }
        self.persistence
            .load_ticket(ticket)
            .map(|e| (e.amount, e.spent, e.from))
    }

    fn lock_internal(&self, from: &[u8], amount: u128) -> Result<[u8; 32], &'static str> {
        self.balances.debit(from, amount)?;
        let ticket = Self::make_ticket(from, amount);
        self.persistence.save_ticket(
            &ticket,
            &EscrowPersistedEntry {
                from: from.to_vec(),
                amount,
                spent: false,
            },
        );
        self.tickets.write().expect("ticket write").insert(
            ticket,
            InMemoryEscrowEntry {
                from: from.to_vec(),
                amount,
                spent: false,
            },
        );
        Ok(ticket)
    }

    fn release_internal(
        &self,
        ticket: &[u8; 32],
        to: &[u8],
        amount: u128,
    ) -> Result<(), &'static str> {
        let (locked_amount, spent, from) =
            self.find_ticket(ticket).ok_or("unknown escrow ticket")?;
        if spent {
            return Err("escrow ticket already spent");
        }
        if locked_amount < amount {
            return Err("escrow release amount exceeds locked amount");
        }
        {
            let mut guard = self.tickets.write().expect("ticket write");
            if let Some(e) = guard.get_mut(ticket) {
                e.spent = true;
            } else {
                guard.insert(
                    *ticket,
                    InMemoryEscrowEntry {
                        from: from.clone(),
                        amount: locked_amount,
                        spent: true,
                    },
                );
            }
        }
        self.persistence.save_ticket(
            ticket,
            &EscrowPersistedEntry {
                from,
                amount: locked_amount,
                spent: true,
            },
        );
        self.balances.credit(to, amount);
        Ok(())
    }
}

impl<C, Block, P: EscrowPersistence> CrossVmEscrow for PalletEscrowAdapter<C, Block, P>
where
    Block: BlockT,
    C: ProvideRuntimeApi<Block> + HeaderBackend<Block> + Send + Sync + 'static,
    C::Api: AtlasKernelRuntimeApi<Block, AccountId32, u128, u32>,
{
    fn lock_svm(&self, from: &[u8], amount: u128) -> Result<[u8; 32], &'static str> {
        self.lock_internal(from, amount)
    }
    fn release_evm(
        &self,
        to: &[u8; 20],
        ticket: &[u8; 32],
        amount: u128,
    ) -> Result<(), &'static str> {
        self.release_internal(ticket, to.as_slice(), amount)
    }
    fn lock_evm(&self, from: &[u8; 20], amount: u128) -> Result<[u8; 32], &'static str> {
        self.lock_internal(from.as_slice(), amount)
    }
    fn release_svm(&self, to: &[u8], ticket: &[u8; 32], amount: u128) -> Result<(), &'static str> {
        self.release_internal(ticket, to, amount)
    }
}

impl<C, Block> PalletEscrowAdapter<C, Block, ()>
where
    Block: BlockT,
    C: ProvideRuntimeApi<Block> + HeaderBackend<Block> + Send + Sync + 'static,
    C::Api: AtlasKernelRuntimeApi<Block, AccountId32, u128, u32>,
{
    pub fn new(balances: Arc<SubstrateClientBalanceAdapter<C, Block>>) -> Self {
        Self::with_persistence(balances, ())
    }
}

impl<C, Block> SubstrateX3VmBridge<C, Block, ()>
where
    Block: BlockT,
    C: ProvideRuntimeApi<Block> + HeaderBackend<Block> + Send + Sync + 'static,
    C::Api: AtlasKernelRuntimeApi<Block, AccountId32, u128, u32>,
{
    pub fn new(client: Arc<C>) -> Self {
        Self::with_persistence_and_config(client, (), default_runtime_bridge_config())
    }
}

impl<C, Block, P> SubstrateX3VmBridge<C, Block, P>
where
    Block: BlockT,
    C: ProvideRuntimeApi<Block> + HeaderBackend<Block> + Send + Sync + 'static,
    C::Api: AtlasKernelRuntimeApi<Block, AccountId32, u128, u32>,
    P: EscrowPersistence + 'static,
{
    pub fn with_persistence(client: Arc<C>, persistence: P) -> Self {
        Self::with_persistence_and_config(client, persistence, default_runtime_bridge_config())
    }

    pub fn with_persistence_and_config(
        client: Arc<C>,
        persistence: P,
        config: BridgeConfig,
    ) -> Self {
        let balances = Arc::new(SubstrateClientBalanceAdapter::new(client));
        let escrow = Arc::new(PalletEscrowAdapter::with_persistence(
            balances.clone(),
            persistence,
        ));
        let balance_provider: Arc<dyn BalanceProvider> = balances.clone();
        let escrow_provider: Arc<dyn CrossVmEscrow> = escrow.clone();
        let bridge = Arc::new(build_x3vm_bridge(balance_provider, escrow_provider, config));

        Self {
            bridge,
            balances,
            escrow,
        }
    }
}

pub struct RuntimeCrossVmDispatcher<C, Block> {
    client: Arc<C>,
    x3vm_bridge: Option<Arc<X3VMBridge>>,
    _phantom: PhantomData<Block>,
}

impl<C, Block> RuntimeCrossVmDispatcher<C, Block>
where
    Block: BlockT,
    C: ProvideRuntimeApi<Block> + HeaderBackend<Block> + Send + Sync + 'static,
    C::Api: AtlasKernelRuntimeApi<Block, AccountId32, u128, u32>,
{
    pub fn new(client: Arc<C>) -> Self {
        Self {
            client,
            x3vm_bridge: None,
            _phantom: PhantomData,
        }
    }

    pub fn with_x3vm_bridge(mut self, bridge: Arc<X3VMBridge>) -> Self {
        self.x3vm_bridge = Some(bridge);
        self
    }

    fn best_hash(&self) -> Block::Hash {
        self.client.info().best_hash
    }
}

impl<C, Block> CrossVmDispatcher for RuntimeCrossVmDispatcher<C, Block>
where
    Block: BlockT,
    C: ProvideRuntimeApi<Block> + HeaderBackend<Block> + Send + Sync + 'static,
    C::Api: AtlasKernelRuntimeApi<Block, AccountId32, u128, u32>,
{
    fn execute_evm_tx(
        &self,
        caller: &[u8; 20],
        target: &[u8; 20],
        input: &[u8],
        value: u128,
    ) -> Result<CrossVmResult, sp_runtime::DispatchError> {
        let at = self.best_hash();
        let api = self.client.runtime_api();
        let mut payload = Vec::with_capacity(20 + 20 + 16 + 4 + input.len());
        payload.extend_from_slice(caller);
        payload.extend_from_slice(target);
        payload.extend_from_slice(&value.to_le_bytes());
        payload.extend_from_slice(&(input.len() as u32).to_le_bytes());
        payload.extend_from_slice(input);
        match api.submit_evm_transaction(at, payload) {
            Ok(Ok(tx_hash)) => Ok(CrossVmResult::success(tx_hash, 21_000)),
            Ok(Err(err)) => Ok(CrossVmResult::failed(err, 21_000)),
            Err(_) => Err(sp_runtime::DispatchError::Other("EVM runtime API error")),
        }
    }

    fn execute_svm_tx(
        &self,
        _caller: &[u8; 32],
        program_id: &[u8; 32],
        input: &[u8],
    ) -> Result<CrossVmResult, sp_runtime::DispatchError> {
        let at = self.best_hash();
        let api = self.client.runtime_api();
        if !api.is_svm_program(at, program_id.to_vec()).unwrap_or(false) {
            return Ok(CrossVmResult::failed(b"program not found".to_vec(), 1_000));
        }
        match api.submit_svm_instruction(at, *program_id, input.to_vec()) {
            Ok(Ok(output)) => Ok(CrossVmResult::success(output, 5_000)),
            Ok(Err(err)) => Ok(CrossVmResult::failed(err, 5_000)),
            Err(_) => Err(sp_runtime::DispatchError::Other("SVM runtime API error")),
        }
    }

    fn execute_x3vm_tx(
        &self,
        _caller: &[u8; 32],
        call: &x3_cross_vm_bridge::CrossVmCall,
    ) -> Result<x3_cross_vm_bridge::CrossVmReceipt, sp_runtime::DispatchError> {
        use x3_cross_vm_bridge::{CrossVmReceipt, CrossVmStatus};
        let zero = sp_core::H256::zero();
        call.ensure_current_version()?;

        if call.target != VmId::X3Vm {
            return Ok(CrossVmReceipt {
                call_hash: call.call_hash(&zero),
                source_state_root: zero,
                target_state_root: zero,
                status: CrossVmStatus::InternalError,
                gas_used: 0,
                logs: Vec::new(),
            });
        }

        let Some(bridge) = &self.x3vm_bridge else {
            return Ok(CrossVmReceipt {
                call_hash: call.call_hash(&zero),
                source_state_root: zero,
                target_state_root: zero,
                status: CrossVmStatus::InternalError,
                gas_used: 0,
                logs: vec![b"x3vm bridge not configured".to_vec()],
            });
        };

        let function_index = u32::from_le_bytes(call.selector) as usize;
        match bridge.execute(call.payload.as_ref(), function_index, &[]) {
            Ok(result) => Ok(CrossVmReceipt {
                call_hash: call.call_hash(&zero),
                source_state_root: zero,
                target_state_root: zero,
                status: CrossVmStatus::Success,
                gas_used: result.gas_used,
                logs: result.value.map(value_to_log).into_iter().collect(),
            }),
            Err(err) => Ok(CrossVmReceipt {
                call_hash: call.call_hash(&zero),
                source_state_root: zero,
                target_state_root: zero,
                status: CrossVmStatus::Reverted,
                gas_used: call.gas_budget,
                logs: vec![format!("{err:?}").into_bytes()],
            }),
        }
    }

    fn get_evm_balance(&self, address: &[u8; 20]) -> u128 {
        let at = self.best_hash();
        let api = self.client.runtime_api();
        api.get_evm_balance(at, address.to_vec(), 0u32)
            .unwrap_or(None)
            .unwrap_or(0)
    }

    fn get_svm_balance(&self, pubkey: &[u8; 32]) -> u64 {
        let at = self.best_hash();
        let api = self.client.runtime_api();
        api.get_svm_balance(at, pubkey.to_vec()).unwrap_or(0)
    }

    fn get_evm_bridge_escrow(&self) -> [u8; 20] {
        let at = self.best_hash();
        let api = self.client.runtime_api();
        let bytes = api.get_evm_bridge_escrow(at).unwrap_or_default();
        let mut escrow = [0u8; 20];
        if bytes.len() == 20 {
            escrow.copy_from_slice(&bytes);
        }
        escrow
    }

    fn get_svm_bridge_escrow(&self) -> [u8; 32] {
        let at = self.best_hash();
        let api = self.client.runtime_api();
        api.get_svm_bridge_escrow(at).unwrap_or([0u8; 32])
    }
}

fn value_to_log(value: x3_vm::Value) -> Vec<u8> {
    match value {
        x3_vm::Value::I64(v) => v.to_le_bytes().to_vec(),
        x3_vm::Value::F64(v) => v.to_bits().to_le_bytes().to_vec(),
        x3_vm::Value::Bool(v) => vec![v as u8],
        x3_vm::Value::String(v) => v.into_bytes(),
        x3_vm::Value::Bytes(v) => v,
        x3_vm::Value::Addr(v) => v.to_le_bytes().to_vec(),
        x3_vm::Value::Unit => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sp_blockchain::{BlockStatus, Info};
    use sp_runtime::traits::{Block as BlockT, Header as HeaderT};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use x3_vm::{Value, VM};

    type Block = sp_runtime::testing::Block<sp_runtime::OpaqueExtrinsic>;

    #[derive(Clone)]
    struct RuntimeApiHarness {
        evm_escrow: [u8; 20],
        svm_escrow: [u8; 32],
    }

    sp_api::mock_impl_runtime_apis! {
        impl AtlasKernelRuntimeApi<Block, AccountId32, u128, u32> for RuntimeApiHarness {
            fn get_canonical_balance(&self, _account: AccountId32, _asset_id: u32) -> u128 {
                0
            }

            fn get_asset_metadata(&self, _asset_id: u32) -> Option<(Vec<u8>, u8)> {
                None
            }

            fn is_authorized(&self, _account: AccountId32) -> bool {
                false
            }

            fn get_authorized_accounts(&self) -> Vec<AccountId32> {
                Vec::new()
            }

            fn get_authorities(&self) -> Vec<AccountId32> {
                Vec::new()
            }

            fn map_evm_address(&self, _address: Vec<u8>) -> Option<AccountId32> {
                None
            }

            fn get_evm_balance(&self, _evm_address: Vec<u8>, _asset_id: u32) -> Option<u128> {
                Some(0)
            }

            fn get_evm_code(&self, _evm_address: Vec<u8>) -> Vec<u8> {
                Vec::new()
            }

            fn get_evm_storage(&self, _evm_address: Vec<u8>, _storage_key: H256) -> Option<H256> {
                None
            }

            fn get_evm_nonce(&self, _evm_address: Vec<u8>) -> u64 {
                0
            }

            fn get_svm_balance(&self, _svm_pubkey: Vec<u8>) -> u64 {
                0
            }

            fn get_evm_bridge_escrow(&self) -> Vec<u8> {
                self.evm_escrow.to_vec()
            }

            fn get_svm_bridge_escrow(&self) -> [u8; 32] {
                self.svm_escrow
            }

            fn is_svm_program(&self, _svm_pubkey: Vec<u8>) -> bool {
                false
            }

            fn submit_evm_transaction(&self, _raw_tx: Vec<u8>) -> Result<Vec<u8>, Vec<u8>> {
                Err(b"not wired in harness".to_vec())
            }

            fn validate_evm_transaction(&self, _raw_tx: Vec<u8>) -> Result<Vec<u8>, Vec<u8>> {
                Err(b"not wired in harness".to_vec())
            }

            fn submit_svm_instruction(
                &self,
                _program_id: [u8; 32],
                _instruction_data: Vec<u8>,
            ) -> Result<Vec<u8>, Vec<u8>> {
                Err(b"not wired in harness".to_vec())
            }

            fn call_evm(
                &self,
                _caller: Option<Vec<u8>>,
                _evm_address: Vec<u8>,
                _input: Vec<u8>,
                _gas_limit: u64,
            ) -> Result<Vec<u8>, Vec<u8>> {
                Err(b"not wired in harness".to_vec())
            }

            fn estimate_evm_gas(
                &self,
                _caller: Option<Vec<u8>>,
                _evm_address: Vec<u8>,
                _input: Vec<u8>,
                _gas_limit: u64,
            ) -> Result<u64, Vec<u8>> {
                Err(b"not wired in harness".to_vec())
            }

            fn get_evm_transaction(&self, _tx_hash: Vec<u8>) -> Option<Vec<u8>> {
                None
            }

            fn get_evm_transaction_by_hash(&self, _tx_hash: Vec<u8>) -> Option<Vec<u8>> {
                None
            }

            fn get_evm_receipt(&self, _tx_hash: Vec<u8>) -> Option<Vec<u8>> {
                None
            }

            fn get_evm_logs(&self, _filter: Vec<u8>) -> Vec<Vec<u8>> {
                Vec::new()
            }

            fn get_evm_transaction_logs(&self, _tx_hash: Vec<u8>) -> Vec<Vec<u8>> {
                Vec::new()
            }

            fn chain_id(&self) -> u64 {
                0
            }

            fn get_svm_slot(&self) -> u64 {
                0
            }

            fn get_svm_blockhash(&self, _slot: u64) -> Option<H256> {
                None
            }

            fn get_svm_transaction_count(&self, _svm_pubkey: Vec<u8>) -> u64 {
                0
            }

            fn get_svm_slot_by_blockhash(&self, _blockhash: H256) -> Option<u64> {
                None
            }

            fn deploy_evm_contract(
                &self,
                _caller: Option<Vec<u8>>,
                _bytecode: Vec<u8>,
                _gas_limit: u64,
            ) -> Result<Vec<u8>, Vec<u8>> {
                Err(b"not wired in harness".to_vec())
            }

            fn get_evm_contract_receipt(&self, _contract_address: Vec<u8>) -> Option<Vec<u8>> {
                None
            }

            fn get_svm_program_data(&self, _svm_pubkey: Vec<u8>) -> Option<Vec<u8>> {
                None
            }

            fn get_svm_account_data(&self, _svm_pubkey: Vec<u8>) -> Option<Vec<u8>> {
                None
            }

            fn get_svm_slot_history(&self, _limit: u32) -> Vec<u64> {
                Vec::new()
            }

            fn get_svm_recent_blockhashes(&self, _limit: u32) -> Vec<H256> {
                Vec::new()
            }
        }
    }

    struct RuntimeClientHarness {
        api: RuntimeApiHarness,
        best_hash: <Block as BlockT>::Hash,
    }

    impl RuntimeClientHarness {
        fn new(evm_escrow: [u8; 20], svm_escrow: [u8; 32]) -> Self {
            Self {
                api: RuntimeApiHarness {
                    evm_escrow,
                    svm_escrow,
                },
                best_hash: H256::repeat_byte(0x42),
            }
        }
    }

    impl ProvideRuntimeApi<Block> for RuntimeClientHarness {
        type Api = RuntimeApiHarness;

        fn runtime_api(&self) -> sp_api::ApiRef<'_, Self::Api> {
            self.api.clone().into()
        }
    }

    impl HeaderBackend<Block> for RuntimeClientHarness {
        fn header(
            &self,
            _hash: <Block as BlockT>::Hash,
        ) -> sp_blockchain::Result<Option<<Block as BlockT>::Header>> {
            Ok(None)
        }

        fn info(&self) -> Info<Block> {
            Info {
                best_hash: self.best_hash,
                best_number: 1,
                genesis_hash: H256::zero(),
                finalized_hash: self.best_hash,
                finalized_number: 1,
                finalized_state: Some((self.best_hash, 1)),
                number_leaves: 1,
                block_gap: None,
            }
        }

        fn status(&self, _hash: <Block as BlockT>::Hash) -> sp_blockchain::Result<BlockStatus> {
            Ok(BlockStatus::InChain)
        }

        fn number(
            &self,
            _hash: <Block as BlockT>::Hash,
        ) -> sp_blockchain::Result<Option<<<Block as BlockT>::Header as HeaderT>::Number>> {
            Ok(Some(1))
        }

        fn hash(
            &self,
            _number: <<Block as BlockT>::Header as HeaderT>::Number,
        ) -> sp_blockchain::Result<Option<<Block as BlockT>::Hash>> {
            Ok(Some(self.best_hash))
        }
    }

    struct RecordingBalanceProvider {
        ledger: Mutex<HashMap<Vec<u8>, u128>>,
        reads: AtomicUsize,
        transfers: AtomicUsize,
    }

    impl RecordingBalanceProvider {
        fn new(entries: impl IntoIterator<Item = (Vec<u8>, u128)>) -> Self {
            Self {
                ledger: Mutex::new(entries.into_iter().collect()),
                reads: AtomicUsize::new(0),
                transfers: AtomicUsize::new(0),
            }
        }
    }

    impl BalanceProvider for RecordingBalanceProvider {
        fn get_balance(&self, address: &[u8]) -> u128 {
            self.reads.fetch_add(1, Ordering::SeqCst);
            *self
                .ledger
                .lock()
                .expect("ledger lock")
                .get(address)
                .unwrap_or(&0)
        }

        fn transfer(&self, from: &[u8], to: &[u8], amount: u128) -> Result<(), &'static str> {
            self.transfers.fetch_add(1, Ordering::SeqCst);
            let mut ledger = self.ledger.lock().expect("ledger lock");
            let from_balance = *ledger.get(from).unwrap_or(&0);
            if from_balance < amount {
                return Err("insufficient balance");
            }
            ledger.insert(from.to_vec(), from_balance - amount);
            let to_balance = *ledger.get(to).unwrap_or(&0);
            ledger.insert(to.to_vec(), to_balance.saturating_add(amount));
            Ok(())
        }
    }

    struct RecordingEscrowProvider {
        balances: Arc<RecordingBalanceProvider>,
        tickets: Mutex<HashMap<[u8; 32], (Vec<u8>, u128, bool)>>,
        locks: AtomicUsize,
        releases: AtomicUsize,
    }

    impl RecordingEscrowProvider {
        fn new(balances: Arc<RecordingBalanceProvider>) -> Self {
            Self {
                balances,
                tickets: Mutex::new(HashMap::new()),
                locks: AtomicUsize::new(0),
                releases: AtomicUsize::new(0),
            }
        }

        fn lock_from(&self, from: &[u8], amount: u128) -> Result<[u8; 32], &'static str> {
            self.locks.fetch_add(1, Ordering::SeqCst);
            self.balances.transfer(from, b"x3-escrow-vault", amount)?;
            let mut ticket = [0u8; 32];
            let seq = self.locks.load(Ordering::SeqCst) as u64;
            ticket[..8].copy_from_slice(&seq.to_le_bytes());
            ticket[8..24].copy_from_slice(&amount.to_le_bytes());
            self.tickets
                .lock()
                .expect("ticket lock")
                .insert(ticket, (from.to_vec(), amount, false));
            Ok(ticket)
        }

        fn release_to(
            &self,
            to: &[u8],
            ticket: &[u8; 32],
            amount: u128,
        ) -> Result<(), &'static str> {
            self.releases.fetch_add(1, Ordering::SeqCst);
            let mut tickets = self.tickets.lock().expect("ticket lock");
            let entry = tickets.get_mut(ticket).ok_or("unknown ticket")?;
            if entry.2 {
                return Err("spent ticket");
            }
            if entry.1 < amount {
                return Err("release exceeds locked amount");
            }
            entry.2 = true;
            drop(tickets);
            self.balances.transfer(b"x3-escrow-vault", to, amount)
        }
    }

    impl CrossVmEscrow for RecordingEscrowProvider {
        fn lock_svm(&self, from: &[u8], amount: u128) -> Result<[u8; 32], &'static str> {
            self.lock_from(from, amount)
        }

        fn release_evm(
            &self,
            to: &[u8; 20],
            ticket: &[u8; 32],
            amount: u128,
        ) -> Result<(), &'static str> {
            self.release_to(to, ticket, amount)
        }

        fn lock_evm(&self, from: &[u8; 20], amount: u128) -> Result<[u8; 32], &'static str> {
            self.lock_from(from, amount)
        }

        fn release_svm(
            &self,
            to: &[u8],
            ticket: &[u8; 32],
            amount: u128,
        ) -> Result<(), &'static str> {
            self.release_to(to, ticket, amount)
        }
    }

    #[test]
    fn x3vm_bridge_routes_balance_escrow_and_nonce_through_runtime_boundary() {
        let svm_addr = vec![0x11; 32];
        let evm_addr = vec![0x22; 20];
        let balances = Arc::new(RecordingBalanceProvider::new([
            (svm_addr.clone(), 1_000),
            (b"x3-escrow-vault".to_vec(), 0),
            (evm_addr.clone(), 0),
        ]));
        let escrow = Arc::new(RecordingEscrowProvider::new(balances.clone()));

        let bridge = build_x3vm_bridge(
            balances.clone(),
            escrow.clone(),
            default_runtime_bridge_config(),
        );
        let mut vm =
            VM::from_bytes(&x3_vm::bridge::bc_format_helpers::assemble_simple_module()).unwrap();
        bridge.register_bridge_hostcalls(&mut vm);

        let before = vm
            .invoke_hostcall(0x12, &[Value::Bytes(svm_addr.clone())])
            .expect("balance hostcall should route")
            .expect("balance value");
        assert_eq!(before, Value::I64(1_000));

        let args = [
            Value::Bytes(svm_addr.clone()),
            Value::Bytes(evm_addr.clone()),
            Value::I64(100),
            Value::Bytes(vec![0xAA; 32]),
        ];
        assert!(vm.invoke_hostcall(0x30, &args).is_ok());
        let replay = vm.invoke_hostcall(0x30, &args);
        assert!(format!("{:?}", replay.unwrap_err()).contains("nonce replay"));

        assert_eq!(balances.get_balance(&svm_addr), 900);
        assert_eq!(balances.get_balance(&evm_addr), 100);
        assert!(balances.reads.load(Ordering::SeqCst) >= 3);
        assert_eq!(escrow.locks.load(Ordering::SeqCst), 1);
        assert_eq!(escrow.releases.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn runtime_dispatcher_executes_x3vm_with_client_backed_escrow_config() {
        let evm_escrow = [0xE1; 20];
        let svm_escrow = [0x51; 32];
        let client = Arc::new(RuntimeClientHarness::new(evm_escrow, svm_escrow));
        let bridge = Arc::new(build_x3vm_bridge(
            Arc::new(RecordingBalanceProvider::new([])),
            Arc::new(RecordingEscrowProvider::new(Arc::new(
                RecordingBalanceProvider::new([]),
            ))),
            default_runtime_bridge_config(),
        ));
        let dispatcher = RuntimeCrossVmDispatcher::<RuntimeClientHarness, Block>::new(client)
            .with_x3vm_bridge(bridge);
        let bytecode = x3_vm::bridge::bc_format_helpers::assemble_simple_module();
        let call = x3_cross_vm_bridge::CrossVmCall::new(
            VmId::X3Vm,
            VmId::X3Vm,
            0u32.to_le_bytes(),
            bytecode,
            1_000_000,
            1,
            100,
        )
        .expect("test bytecode fits cross-vm payload");

        let receipt = dispatcher
            .execute_x3vm_tx(&[0u8; 32], &call)
            .expect("dispatcher should execute x3vm call");

        assert_eq!(receipt.status, x3_cross_vm_bridge::CrossVmStatus::Success);
        assert_eq!(receipt.call_hash, call.call_hash(&H256::zero()));
        assert_eq!(dispatcher.get_evm_bridge_escrow(), evm_escrow);
        assert_eq!(dispatcher.get_svm_bridge_escrow(), svm_escrow);
    }
}
