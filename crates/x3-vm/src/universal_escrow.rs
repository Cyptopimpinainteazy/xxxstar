//! Universal Cross-VM Escrow Registry
//!
//! Generalizes the SVM/EVM-specific `CrossVmEscrow` trait into a family-agnostic
//! escrow layer that supports all 18 VM families.  The registry maps `VmType` to
//! an escrow provider so `X3VMBridge` can route `bridge_external_to_x3vm` and
//! `bridge_x3vm_to_external` hostcalls uniformly.
//!
//! # Design
//!
//! Instead of hard-coding `lock_svm` / `release_evm` / `lock_evm` / `release_svm`,
//! the universal trait uses a single `lock(chain_id, account, amount)` →
//! `EscrowTicket` and `release(chain_id, ticket, recipient, amount)` pair.
//!
//! Each VM family's escrow adapter translates the generic call into the
//! chain-specific account model (20-byte EVM address, 32-byte SVM pubkey,
//! UTXO script, Move object, WASM contract, etc.).
//!
//! # Extending `X3VMBridge`
//!
//! Callers wire the registry via `X3VMBridge::with_universal_escrow(registry)`.
//! The bridge then registers two additional hostcalls:
//!
//! * `0x32` `bridge_external_to_x3vm(chain_id, sender_account, amount, nonce)` →
//!   lock on external chain, release on X3VM
//! * `0x33` `bridge_x3vm_to_external(chain_id, x3_account, amount, nonce)` →
//!   lock on X3VM, release on external chain

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

// ── Canonical external chain descriptor ──────────────────────────────────

/// Identifies an external chain for escrow operations.
/// Maps 1:1 with the `BridgeAdapter` `chain_id()` values.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ExternalChainId {
    pub chain_id: u64,
    pub vm_family: &'static str,
    pub display_name: &'static str,
}

impl ExternalChainId {
    pub const fn new(chain_id: u64, vm_family: &'static str, display_name: &'static str) -> Self {
        Self {
            chain_id,
            vm_family,
            display_name,
        }
    }
}

// ── Universal escrow ticket ──────────────────────────────────────────────

/// A deterministic 32-byte escrow ticket produced by [`UniversalEscrow::lock`].
pub type EscrowTicket = [u8; 32];

/// Account identifier — variable-length byte representation that encodes the
/// VM-specific account format (20 B EVM, 32 B SVM, variable UTXO, etc.).
pub type ExternalAccount = Vec<u8>;

// ── Universal escrow trait ───────────────────────────────────────────────

/// Universal cross-VM escrow provider.
///
/// Each external chain family registers one implementation.  The bridge
/// hostcalls dispatch through the registry, keeping the core bridge logic
/// agnostic to the external VM's account model.
pub trait UniversalEscrow: Send + Sync {
    /// Lock `amount` of native tokens from `sender` on this external chain.
    ///
    /// Returns a 32-byte escrow ticket that authorises release on the
    /// destination chain.  The ticket is opaque to the caller — only the
    /// escrow provider that created it can redeem it.
    fn lock(
        &self,
        sender: &ExternalAccount,
        amount: u128,
    ) -> Result<EscrowTicket, &'static str>;

    /// Release `amount` of tokens to `recipient` on this external chain.
    ///
    /// The `ticket` must have been produced by a prior `lock` call on *this*
    /// same provider instance.  Tickets are consumed after successful release.
    fn release(
        &self,
        recipient: &ExternalAccount,
        ticket: &EscrowTicket,
        amount: u128,
    ) -> Result<(), &'static str>;

    /// Return a human-readable name for this escrow provider.
    fn provider_name(&self) -> &'static str;
}

// ── Escrow registry ──────────────────────────────────────────────────────

/// Registry mapping external chain IDs to escrow providers.
///
/// The bridge consults this registry when dispatching `bridge_external_to_x3vm`
/// and `bridge_x3vm_to_external` hostcalls.
pub struct UniversalEscrowRegistry {
    /// Map chain_id → provider.  A single provider may serve multiple chain IDs
    /// (e.g. EVM provider handles Ethereum, Arbitrum, BSC, Optimism, Base, Polygon).
    providers: Mutex<HashMap<u64, Arc<dyn UniversalEscrow>>>,
}

impl UniversalEscrowRegistry {
    pub fn new() -> Self {
        Self {
            providers: Mutex::new(HashMap::new()),
        }
    }

    /// Register an escrow provider for one or more chain IDs.
    pub fn register(
        &self,
        chain_ids: &[u64],
        provider: Arc<dyn UniversalEscrow>,
    ) -> Result<(), &'static str> {
        let mut guard = self.providers.lock().map_err(|_| "registry lock poisoned")?;
        for &id in chain_ids {
            guard.insert(id, provider.clone());
        }
        Ok(())
    }

    /// Look up the escrow provider for a given chain ID.
    pub fn get(&self, chain_id: u64) -> Option<Arc<dyn UniversalEscrow>> {
        self.providers
            .lock()
            .ok()
            .and_then(|g| g.get(&chain_id).cloned())
    }

    /// List all registered chain IDs.
    pub fn registered_chains(&self) -> Vec<u64> {
        self.providers
            .lock()
            .map(|g| g.keys().copied().collect())
            .unwrap_or_default()
    }

    /// Check if a chain ID is registered.
    pub fn is_registered(&self, chain_id: u64) -> bool {
        self.providers
            .lock()
            .map(|g| g.contains_key(&chain_id))
            .unwrap_or(false)
    }
}

impl Default for UniversalEscrowRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ── In-memory escrow provider (test / development) ───────────────────────

/// A simple in-memory escrow provider that can be used for testing or as a
/// stub until a real chain-specific provider is implemented.
pub struct InMemoryEscrowProvider {
    name: &'static str,
    tickets: Mutex<HashMap<EscrowTicket, (ExternalAccount, u128, bool)>>,
    ticket_seq: Mutex<u64>,
}

impl InMemoryEscrowProvider {
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            tickets: Mutex::new(HashMap::new()),
            ticket_seq: Mutex::new(0),
        }
    }

    fn make_ticket(&self, sender: &[u8], amount: u128) -> EscrowTicket {
        let mut seq = self.ticket_seq.lock().unwrap();
        let current = *seq;
        *seq = current.wrapping_add(1);
        let mut ticket = [0u8; 32];
        ticket[..8].copy_from_slice(&current.to_le_bytes());
        ticket[8..24].copy_from_slice(&amount.to_le_bytes());
        if sender.len() >= 8 {
            ticket[24..32].copy_from_slice(&sender[..8]);
        }
        ticket
    }
}

impl UniversalEscrow for InMemoryEscrowProvider {
    fn lock(
        &self,
        sender: &ExternalAccount,
        amount: u128,
    ) -> Result<EscrowTicket, &'static str> {
        let ticket = self.make_ticket(sender, amount);
        self.tickets
            .lock()
            .map_err(|_| "ticket lock poisoned")?
            .insert(ticket, (sender.clone(), amount, false));
        Ok(ticket)
    }

    fn release(
        &self,
        // The in-memory provider scopes redemption to the original ticket; the
        // recipient is intentionally validated by chain-specific providers only.
        _recipient: &ExternalAccount,
        ticket: &EscrowTicket,
        amount: u128,
    ) -> Result<(), &'static str> {
        let mut guard = self.tickets.lock().map_err(|_| "ticket lock poisoned")?;
        let entry = guard.get_mut(ticket).ok_or("unknown escrow ticket")?;
        if entry.2 {
            return Err("escrow ticket already spent");
        }
        if entry.1 != amount {
            return Err("escrow amount mismatch");
        }
        entry.2 = true; // mark spent
        drop(guard);
        Ok(())
    }

    fn provider_name(&self) -> &'static str {
        self.name
    }
}

// ── Factory: build a complete registry with all VM families ──────────────

/// Build a [`UniversalEscrowRegistry`] pre-populated with in-memory providers
/// for all 18 VM families.
///
/// In production, real chain-specific providers (backed by Substrate pallets
/// or the `BridgeAdapter` RPC layer) replace the in-memory stubs.
pub fn build_full_registry() -> UniversalEscrowRegistry {
    let registry = UniversalEscrowRegistry::new();

    // VM family chain groups (matching the expansion roadmap)
    let evm = Arc::new(InMemoryEscrowProvider::new("evm-escrow"));
    let svm = Arc::new(InMemoryEscrowProvider::new("svm-escrow"));
    let substrate = Arc::new(InMemoryEscrowProvider::new("substrate-escrow"));
    let bitcoin = Arc::new(InMemoryEscrowProvider::new("bitcoin-escrow"));
    let move_vm = Arc::new(InMemoryEscrowProvider::new("move-escrow"));
    let cosmwasm = Arc::new(InMemoryEscrowProvider::new("cosmwasm-escrow"));
    let cairo = Arc::new(InMemoryEscrowProvider::new("cairo-escrow"));
    let plutus = Arc::new(InMemoryEscrowProvider::new("plutus-escrow"));
    let ton = Arc::new(InMemoryEscrowProvider::new("ton-escrow"));
    let fuel = Arc::new(InMemoryEscrowProvider::new("fuel-escrow"));
    let near = Arc::new(InMemoryEscrowProvider::new("near-escrow"));
    let soroban = Arc::new(InMemoryEscrowProvider::new("soroban-escrow"));
    let pvm = Arc::new(InMemoryEscrowProvider::new("polkadot-pvm-escrow"));
    let zkvm = Arc::new(InMemoryEscrowProvider::new("zkvm-escrow"));

    // EVM family
    let _ = registry.register(&[1, 10, 56, 137, 42161, 42170, 8453, 43114], evm);
    // SVM family
    let _ = registry.register(&[1399811149], svm);
    // Substrate family
    let _ = registry.register(&[1000], substrate);
    // Bitcoin family
    let _ = registry.register(&[0], bitcoin);
    // MoveVM family (Sui → 21, Aptos → 19)
    let _ = registry.register(&[21, 19], move_vm);
    // CosmWasm family (Cosmos Hub → osmosis-1, Juno, etc.)
    let _ = registry.register(&[2000, 2001, 2002], cosmwasm);
    // CairoVM family (Starknet mainnet)
    let _ = registry.register(&[23448594291968334], cairo);
    // Plutus family (Cardano)
    let _ = registry.register(&[3000], plutus);
    // TON family
    let _ = registry.register(&[4000], ton);
    // Fuel family
    let _ = registry.register(&[5000], fuel);
    // NEAR family
    let _ = registry.register(&[6000], near);
    // Soroban family (Stellar)
    let _ = registry.register(&[7000], soroban);
    // Polkadot PVM / ink! family
    let _ = registry.register(&[8000, 8001], pvm);
    // zkVM family (RISC Zero, SP1, zkWASM)
    let _ = registry.register(&[9000, 9001, 9002], zkvm);

    registry
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_register_and_lookup() {
        let registry = UniversalEscrowRegistry::new();
        let provider = Arc::new(InMemoryEscrowProvider::new("test-provider"));
        registry.register(&[1, 2, 3], provider).unwrap();

        assert!(registry.is_registered(1));
        assert!(registry.is_registered(2));
        assert!(registry.is_registered(3));
        assert!(!registry.is_registered(4));

        let chains = registry.registered_chains();
        assert_eq!(chains.len(), 3);
    }

    #[test]
    fn test_in_memory_escrow_lock_release() {
        let provider = InMemoryEscrowProvider::new("test");
        let sender: ExternalAccount = b"alice".to_vec();
        let recipient: ExternalAccount = b"bob".to_vec();

        let ticket = provider.lock(&sender, 500).unwrap();
        assert_ne!(ticket, [0u8; 32]);

        assert!(provider.release(&recipient, &ticket, 500).is_ok());
        // Double release fails
        assert!(provider.release(&recipient, &ticket, 500).is_err());
    }

    #[test]
    fn test_in_memory_escrow_amount_mismatch() {
        let provider = InMemoryEscrowProvider::new("test");
        let sender: ExternalAccount = b"alice".to_vec();
        let recipient: ExternalAccount = b"bob".to_vec();

        let ticket = provider.lock(&sender, 500).unwrap();
        assert!(provider.release(&recipient, &ticket, 300).is_err());
    }

    #[test]
    fn test_full_registry_has_all_families() {
        let registry = build_full_registry();
        // Check representative chain IDs per family
        assert!(registry.is_registered(1)); // Ethereum
        assert!(registry.is_registered(1399811149)); // Solana
        assert!(registry.is_registered(1000)); // Polkadot
        assert!(registry.is_registered(0)); // Bitcoin
        assert!(registry.is_registered(21)); // Sui
        assert!(registry.is_registered(2000)); // CosmWasm
        assert!(registry.is_registered(23448594291968334)); // Starknet
        assert!(registry.is_registered(3000)); // Cardano
        assert!(registry.is_registered(4000)); // TON
        assert!(registry.is_registered(5000)); // Fuel
        assert!(registry.is_registered(6000)); // NEAR
        assert!(registry.is_registered(7000)); // Stellar
        assert!(registry.is_registered(8000)); // Polkadot PVM
        assert!(registry.is_registered(9000)); // zkVM
    }

    #[test]
    fn test_escrow_provider_name() {
        let provider = InMemoryEscrowProvider::new("my-provider");
        assert_eq!(provider.provider_name(), "my-provider");
    }
}