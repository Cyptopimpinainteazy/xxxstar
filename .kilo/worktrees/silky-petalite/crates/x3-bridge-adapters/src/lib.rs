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
use x3_vm::bridge::{BalanceProvider, CrossVmEscrow};
use x3_cross_vm_bridge::{CrossVmDispatcher, CrossVmResult};

pub use pallet_x3_kernel::StateChange;
pub use x3_cross_vm_bridge::{CrossVmDispatcher as CrossVmDispatcherTrait};

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
            20 => api.get_evm_balance(at, address.to_vec(), 0u32).unwrap_or(None).unwrap_or(0),
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
        guard.get_mut(address).expect("credit: address must be loaded").current =
            current.saturating_add(amount);
    }

    pub(crate) fn debit(&self, address: &[u8], amount: u128) -> Result<(), &'static str> {
        let current = self.ensure_loaded(address);
        if current < amount {
            return Err("insufficient balance");
        }
        let mut guard = self.overlay.write().expect("overlay write");
        guard.get_mut(address).expect("debit: address must be loaded").current = current - amount;
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
    fn load_ticket(&self, _ticket: &[u8; 32]) -> Option<EscrowPersistedEntry> { None }
}

pub struct OffchainEscrowPersistence<O> {
    storage: Mutex<O>,
}

impl<O> OffchainEscrowPersistence<O> {
    pub fn new(storage: O) -> Self {
        Self { storage: Mutex::new(storage) }
    }
}

impl<O: OffchainStorage + Send + 'static> EscrowPersistence for OffchainEscrowPersistence<O> {
    fn save_ticket(&self, ticket: &[u8; 32], entry: &EscrowPersistedEntry) {
        let mut key = [0u8; 38];
        key[..6].copy_from_slice(b"x3esc:");
        key[6..].copy_from_slice(ticket);
        let value = entry.encode();
        self.storage.lock().expect("offchain storage lock").set(
            sp_core::offchain::STORAGE_PREFIX, &key, &value,
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
        Self { balances, tickets: RwLock::new(HashMap::new()), persistence }
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
        self.persistence.load_ticket(ticket).map(|e| (e.amount, e.spent, e.from))
    }

    fn lock_internal(&self, from: &[u8], amount: u128) -> Result<[u8; 32], &'static str> {
        self.balances.debit(from, amount)?;
        let ticket = Self::make_ticket(from, amount);
        self.persistence.save_ticket(&ticket, &EscrowPersistedEntry {
            from: from.to_vec(), amount, spent: false,
        });
        self.tickets.write().expect("ticket write").insert(
            ticket,
            InMemoryEscrowEntry { from: from.to_vec(), amount, spent: false },
        );
        Ok(ticket)
    }

    fn release_internal(&self, ticket: &[u8; 32], to: &[u8], amount: u128) -> Result<(), &'static str> {
        let (locked_amount, spent, from) = self.find_ticket(ticket).ok_or("unknown escrow ticket")?;
        if spent { return Err("escrow ticket already spent"); }
        if locked_amount < amount { return Err("escrow release amount exceeds locked amount"); }
        {
            let mut guard = self.tickets.write().expect("ticket write");
            if let Some(e) = guard.get_mut(ticket) {
                e.spent = true;
            } else {
                guard.insert(*ticket, InMemoryEscrowEntry { from: from.clone(), amount: locked_amount, spent: true });
            }
        }
        self.persistence.save_ticket(ticket, &EscrowPersistedEntry { from, amount: locked_amount, spent: true });
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
    fn release_evm(&self, to: &[u8; 20], ticket: &[u8; 32], amount: u128) -> Result<(), &'static str> {
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

pub struct RuntimeCrossVmDispatcher<C, Block> {
    client: Arc<C>,
    _phantom: PhantomData<Block>,
}

impl<C, Block> RuntimeCrossVmDispatcher<C, Block>
where
    Block: BlockT,
    C: ProvideRuntimeApi<Block> + HeaderBackend<Block> + Send + Sync + 'static,
    C::Api: AtlasKernelRuntimeApi<Block, AccountId32, u128, u32>,
{
    pub fn new(client: Arc<C>) -> Self {
        Self { client, _phantom: PhantomData }
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
        Ok(CrossVmReceipt {
            call_hash: call.call_hash(&zero),
            source_state_root: zero,
            target_state_root: zero,
            status: CrossVmStatus::InternalError,
            gas_used: 0,
            logs: Vec::new(),
        })
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
        [0u8; 20]
    }

    fn get_svm_bridge_escrow(&self) -> [u8; 32] {
        [0u8; 32]
    }
}
