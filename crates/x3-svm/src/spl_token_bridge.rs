use parity_scale_codec::{Decode, DecodeWithMemTracking, Encode};
use sp_std::collections::btree_map::BTreeMap;
use sp_std::vec::Vec;

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct SPLTokenMint {
    pub solana_mint: [u8; 32],
    pub x3_wrapped_token_id: u128,
    pub decimals: u8,
    pub is_bridged: bool,
    pub total_supply: u64,
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct BridgeVault {
    pub vault_owner: [u8; 32],
    pub token_mint: [u8; 32],
    pub vault_balance: u64,
    pub is_locked: bool,
    pub chain_id: u32,
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct TokenBridgeRequest {
    pub id: [u8; 32],
    pub source_chain: u32,
    pub destination_chain: u32,
    pub token_mint: [u8; 32],
    pub amount: u64,
    pub recipient: [u8; 32],
    pub status: u8, // 0=pending, 1=locked, 2=minted, 3=failed
    pub nonce: u64,
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct WrappedToken {
    pub original_mint: [u8; 32],
    pub original_chain: u32,
    pub x3_token_id: u128,
    pub supply_on_x3: u64,
    pub supply_on_solana: u64,
    pub is_canonical: bool,
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct BridgedBalance {
    pub user: [u8; 32],
    pub chain_id: u32,
    pub token_id: u128,
    pub balance: u64,
    pub locked_amount: u64,
}

#[derive(Debug, PartialEq, Eq)]
pub enum BridgeError {
    NotFound(&'static str),
    InsufficientBalance,
    InvalidAmount,
    InvalidDecimals,
    VaultLocked,
    SupplyInconsistency,
    AlreadyExists(&'static str),
    DecodeError,
}

/// Lightweight in-memory store for cross-chain bridge state.
/// Uses BTreeMap for no_std compatibility. Keys are byte-prefixed tuples;
/// values are SCALE-encoded structs.
#[derive(Default)]
pub struct BridgeStore {
    data: BTreeMap<Vec<u8>, Vec<u8>>,
}

impl BridgeStore {
    pub fn new() -> Self {
        Self {
            data: BTreeMap::new(),
        }
    }

    pub fn put(&mut self, key: &[u8], value: &[u8]) {
        self.data.insert(key.to_vec(), value.to_vec());
    }

    pub fn get(&self, key: &[u8]) -> Option<Vec<u8>> {
        self.data.get(key).cloned()
    }

    pub fn exists(&self, key: &[u8]) -> bool {
        self.data.contains_key(key)
    }

    pub fn remove(&mut self, key: &[u8]) -> bool {
        self.data.remove(key).is_some()
    }

    pub fn put_encoded<T: Encode>(&mut self, key: &[u8], value: &T) {
        self.put(key, &value.encode());
    }

    pub fn get_decoded<T: Decode>(&self, key: &[u8]) -> Option<Result<T, BridgeError>> {
        self.get(key)
            .map(|bytes| T::decode(&mut &bytes[..]).map_err(|_| BridgeError::DecodeError))
    }

    /// Insert and return the previous value if any.
    pub fn replace(&mut self, key: &[u8], value: &[u8]) -> Option<Vec<u8>> {
        self.data.insert(key.to_vec(), value.to_vec())
    }
}

// ---------------------------------------------------------------------------
// Key-helpers (fixed-size segments, no delimiter needed)
// ---------------------------------------------------------------------------

fn key_mint(mint: &[u8; 32]) -> Vec<u8> {
    let mut k = Vec::with_capacity(33);
    k.push(0x01); // prefix: mint
    k.extend_from_slice(mint);
    k
}

fn key_request(id: &[u8; 32]) -> Vec<u8> {
    let mut k = Vec::with_capacity(33);
    k.push(0x02); // prefix: request
    k.extend_from_slice(id);
    k
}

fn key_balance(chain_id: u32, user: &[u8; 32], token_id: u128) -> Vec<u8> {
    let mut k = Vec::with_capacity(1 + 4 + 32 + 16);
    k.push(0x03); // prefix: balance
    k.extend_from_slice(&chain_id.to_le_bytes());
    k.extend_from_slice(user);
    k.extend_from_slice(&token_id.to_le_bytes());
    k
}

fn key_paused(mint: &[u8; 32]) -> Vec<u8> {
    let mut k = Vec::with_capacity(33);
    k.push(0x04); // prefix: paused
    k.extend_from_slice(mint);
    k
}

#[derive(Default)]
pub struct SPLTokenBridge {
    store: BridgeStore,
}

impl SPLTokenBridge {
    const SOLANA_CHAIN_ID: u32 = 101;
    const X3_CHAIN_ID: u32 = 1;

    pub fn new() -> Self {
        Self {
            store: BridgeStore::new(),
        }
    }

    /// Expose store for external inspection or recovery.
    pub fn store(&self) -> &BridgeStore {
        &self.store
    }

    pub fn store_mut(&mut self) -> &mut BridgeStore {
        &mut self.store
    }

    // ------------------------------------------------------------------
    // Stateful bridge operations
    // ------------------------------------------------------------------

    /// Register a Solana SPL mint for bridging
    pub fn register_spl_mint(
        &mut self,
        solana_mint: [u8; 32],
        decimals: u8,
    ) -> Result<u128, BridgeError> {
        if decimals > 18 {
            return Err(BridgeError::InvalidDecimals);
        }

        let key = key_mint(&solana_mint);
        if self.store.exists(&key) {
            return Err(BridgeError::AlreadyExists("mint already registered"));
        }

        let x3_token_id = Self::derive_wrapped_token_id(&solana_mint);

        let mint = SPLTokenMint {
            solana_mint,
            x3_wrapped_token_id: x3_token_id,
            decimals,
            is_bridged: true,
            total_supply: 0,
        };

        self.store.put_encoded(&key, &mint);
        Ok(x3_token_id)
    }

    /// Initiate lock-and-mint from Solana to X3
    pub fn lock_on_solana(
        &mut self,
        token_mint: [u8; 32],
        amount: u64,
        recipient: [u8; 32],
        nonce: u64,
    ) -> Result<[u8; 32], BridgeError> {
        if amount == 0 {
            return Err(BridgeError::InvalidAmount);
        }

        let request_id = Self::derive_request_id(&token_mint, amount, &recipient, nonce);

        let key = key_request(&request_id);
        if self.store.exists(&key) {
            return Err(BridgeError::AlreadyExists("request already exists"));
        }

        let request = TokenBridgeRequest {
            id: request_id,
            source_chain: Self::SOLANA_CHAIN_ID,
            destination_chain: Self::X3_CHAIN_ID,
            token_mint,
            amount,
            recipient,
            status: 1, // locked
            nonce,
        };

        self.store.put_encoded(&key, &request);
        Ok(request_id)
    }

    /// Mint wrapped tokens on X3 (called by relayer after Solana lock finalized)
    pub fn mint_wrapped_on_x3(
        &mut self,
        request_id: [u8; 32],
        token_mint: [u8; 32],
        amount: u64,
        recipient: [u8; 32],
    ) -> Result<BridgedBalance, BridgeError> {
        if amount == 0 {
            return Err(BridgeError::InvalidAmount);
        }

        // Verify lock request exists
        let req_key = key_request(&request_id);
        let request: TokenBridgeRequest = self
            .store
            .get_decoded(&req_key)
            .ok_or(BridgeError::NotFound("lock request not found"))?
            .map_err(|_| BridgeError::DecodeError)?;

        if request.status != 1 {
            return Err(BridgeError::NotFound("lock request not in locked status"));
        }

        let x3_token_id = Self::derive_wrapped_token_id(&token_mint);
        let bal_key = key_balance(Self::X3_CHAIN_ID, &recipient, x3_token_id);

        let mut balance = self
            .store
            .get_decoded::<BridgedBalance>(&bal_key)
            .and_then(|r| r.ok())
            .unwrap_or(BridgedBalance {
                user: recipient,
                chain_id: Self::X3_CHAIN_ID,
                token_id: x3_token_id,
                balance: 0,
                locked_amount: 0,
            });

        balance.balance = balance.balance.saturating_add(amount);
        self.store.put_encoded(&bal_key, &balance);

        // Mark request as minted
        let mut updated = request;
        updated.status = 2;
        self.store.put_encoded(&req_key, &updated);

        Ok(balance)
    }

    /// Initiate burn-and-unlock from X3 to Solana
    pub fn burn_on_x3(
        &mut self,
        x3_token_id: u128,
        amount: u64,
        solana_recipient: [u8; 32],
        nonce: u64,
    ) -> Result<[u8; 32], BridgeError> {
        if amount == 0 {
            return Err(BridgeError::InvalidAmount);
        }

        // Verify user has a BridgedBalance with sufficient amount.
        // We don't know who the caller is here, so for now we check
        // whether any balance exists that can cover `amount` for the
        // recipient — the recipient burns on their own behalf.
        let caller_key = key_balance(Self::X3_CHAIN_ID, &solana_recipient, x3_token_id);
        let balance: BridgedBalance = self
            .store
            .get_decoded(&caller_key)
            .ok_or(BridgeError::NotFound("balance not found"))?
            .map_err(|_| BridgeError::DecodeError)?;

        if balance.balance < amount {
            return Err(BridgeError::InsufficientBalance);
        }

        let request_id =
            Self::derive_wrapped_request_id(x3_token_id, amount, &solana_recipient, nonce);
        let req_key = key_request(&request_id);
        if self.store.exists(&req_key) {
            return Err(BridgeError::AlreadyExists("burn request already exists"));
        }

        let request = TokenBridgeRequest {
            id: request_id,
            source_chain: Self::X3_CHAIN_ID,
            destination_chain: Self::SOLANA_CHAIN_ID,
            token_mint: [0; 32],
            amount,
            recipient: solana_recipient,
            status: 1, // locked (burn initiated)
            nonce,
        };
        self.store.put_encoded(&req_key, &request);

        // Deduct balance
        let mut updated_balance = balance;
        updated_balance.balance -= amount;
        updated_balance.locked_amount = updated_balance.locked_amount.saturating_add(amount);
        self.store.put_encoded(&caller_key, &updated_balance);

        Ok(request_id)
    }

    /// Unlock tokens on Solana (called by relayer after X3 burn finalized)
    pub fn unlock_on_solana(
        &mut self,
        request_id: [u8; 32],
        token_mint: [u8; 32],
        amount: u64,
        recipient: [u8; 32],
    ) -> Result<bool, BridgeError> {
        if amount == 0 {
            return Err(BridgeError::InvalidAmount);
        }

        // Verify burn request exists and is still pending
        let req_key = key_request(&request_id);
        let request: TokenBridgeRequest = self
            .store
            .get_decoded(&req_key)
            .ok_or(BridgeError::NotFound("burn request not found"))?
            .map_err(|_| BridgeError::DecodeError)?;

        if request.status != 1 {
            return Err(BridgeError::NotFound("burn request not in locked status"));
        }

        // Mark request as completed
        let mut updated = request;
        updated.status = 2;
        self.store.put_encoded(&req_key, &updated);

        // Record unlock balance on Solana side
        let x3_token_id = Self::derive_wrapped_token_id(&token_mint);
        let bal_key = key_balance(Self::SOLANA_CHAIN_ID, &recipient, x3_token_id);
        let mut unlock_balance = self
            .store
            .get_decoded::<BridgedBalance>(&bal_key)
            .and_then(|r| r.ok())
            .unwrap_or(BridgedBalance {
                user: recipient,
                chain_id: Self::SOLANA_CHAIN_ID,
                token_id: x3_token_id,
                balance: 0,
                locked_amount: 0,
            });

        unlock_balance.balance = unlock_balance.balance.saturating_add(amount);
        self.store.put_encoded(&bal_key, &unlock_balance);

        Ok(true)
    }

    // ------------------------------------------------------------------
    // Vault operations (work on caller-supplied BridgeVault, no store)
    // ------------------------------------------------------------------

    pub fn create_bridge_vault(
        vault_owner: [u8; 32],
        token_mint: [u8; 32],
    ) -> Result<BridgeVault, BridgeError> {
        Ok(BridgeVault {
            vault_owner,
            token_mint,
            vault_balance: 0,
            is_locked: false,
            chain_id: Self::SOLANA_CHAIN_ID,
        })
    }

    pub fn deposit_to_vault(vault: &mut BridgeVault, amount: u64) -> Result<u64, BridgeError> {
        if vault.is_locked {
            return Err(BridgeError::VaultLocked);
        }
        vault.vault_balance = vault.vault_balance.saturating_add(amount);
        Ok(vault.vault_balance)
    }

    pub fn withdraw_from_vault(vault: &mut BridgeVault, amount: u64) -> Result<u64, BridgeError> {
        if vault.is_locked {
            return Err(BridgeError::VaultLocked);
        }
        if vault.vault_balance < amount {
            return Err(BridgeError::InsufficientBalance);
        }
        vault.vault_balance -= amount;
        Ok(vault.vault_balance)
    }

    // ------------------------------------------------------------------
    // Pure / stateless helpers
    // ------------------------------------------------------------------

    pub fn calculate_bridge_fee(amount: u64) -> u64 {
        amount / 1000
    }

    pub fn is_token_canonical(_token_mint: [u8; 32]) -> bool {
        true
    }

    pub fn get_total_locked_supply(token_mint: [u8; 32], vaults: &[BridgeVault]) -> u64 {
        let mut total: u64 = 0;
        for vault in vaults {
            if vault.token_mint == token_mint && !vault.is_locked {
                total = total.saturating_add(vault.vault_balance);
            }
        }
        total
    }

    pub fn validate_supply_consistency(
        wrapped: &WrappedToken,
        total_x3_supply: u64,
        total_solana_supply: u64,
    ) -> Result<bool, BridgeError> {
        let total = total_x3_supply.saturating_add(total_solana_supply);
        if total
            != wrapped
                .supply_on_x3
                .saturating_add(wrapped.supply_on_solana)
        {
            return Err(BridgeError::SupplyInconsistency);
        }
        Ok(true)
    }

    /// Emergency pause/unpause bridge for a token
    pub fn set_bridge_paused(
        &mut self,
        token_mint: [u8; 32],
        is_paused: bool,
    ) -> Result<bool, BridgeError> {
        let pkey = key_paused(&token_mint);
        self.store.put_encoded(&pkey, &is_paused);

        // Also update the mint record if it exists
        let mkey = key_mint(&token_mint);
        if let Some(Ok(mut mint)) = self.store.get_decoded::<SPLTokenMint>(&mkey) {
            mint.is_bridged = !is_paused;
            self.store.put_encoded(&mkey, &mint);
        }

        Ok(!is_paused)
    }

    /// Query bridge paused state
    pub fn is_bridge_paused(&self, token_mint: &[u8; 32]) -> bool {
        self.store
            .get_decoded::<bool>(&key_paused(token_mint))
            .and_then(|r| r.ok())
            .unwrap_or(false)
    }

    // ------------------------------------------------------------------
    // Deterministic ID derivation
    // ------------------------------------------------------------------

    fn derive_wrapped_token_id(solana_mint: &[u8; 32]) -> u128 {
        let mut id: u128 = 0;
        for (i, byte) in solana_mint.iter().enumerate().take(16) {
            id = id.saturating_add((*byte as u128) << (i * 8));
        }
        id
    }

    fn derive_request_id(
        token_mint: &[u8; 32],
        amount: u64,
        recipient: &[u8; 32],
        nonce: u64,
    ) -> [u8; 32] {
        let mut id = [0u8; 32];
        for (i, byte) in token_mint.iter().enumerate() {
            id[i] = *byte;
        }
        for (i, byte) in recipient.iter().take(16).enumerate() {
            id[i + 16] = *byte;
        }
        let ab = amount.to_le_bytes();
        let nb = nonce.to_le_bytes();
        id[12] = ab[0];
        id[13] = ab[1];
        id[14] = nb[0];
        id[15] = nb[1];
        id
    }

    fn derive_wrapped_request_id(
        token_id: u128,
        amount: u64,
        recipient: &[u8; 32],
        nonce: u64,
    ) -> [u8; 32] {
        let mut id = [0u8; 32];
        for (i, byte) in recipient.iter().enumerate() {
            id[i] = *byte;
        }
        let token_bytes = token_id.to_le_bytes();
        for (i, byte) in token_bytes.iter().enumerate() {
            id[i + 8] = *byte;
        }
        let ab = amount.to_le_bytes();
        let nb = nonce.to_le_bytes();
        id[24] = ab[0];
        id[25] = ab[1];
        id[26] = nb[0];
        id[27] = nb[1];
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_spl_mint() {
        let mut bridge = SPLTokenBridge::new();
        let mint = [1; 32];
        let token_id = bridge.register_spl_mint(mint, 6).unwrap();
        assert!(token_id > 0);

        // Verify it's stored
        let stored = bridge.store.get_decoded::<SPLTokenMint>(&key_mint(&mint));
        assert!(stored.is_some());
        let mint_record = stored.unwrap().unwrap();
        assert_eq!(mint_record.x3_wrapped_token_id, token_id);
        assert!(mint_record.is_bridged);
    }

    #[test]
    fn test_register_invalid_decimals() {
        let mut bridge = SPLTokenBridge::new();
        let result = bridge.register_spl_mint([1; 32], 19);
        assert_eq!(result, Err(BridgeError::InvalidDecimals));
    }

    #[test]
    fn test_register_duplicate_mint() {
        let mut bridge = SPLTokenBridge::new();
        bridge.register_spl_mint([1; 32], 6).unwrap();
        let result = bridge.register_spl_mint([1; 32], 6);
        assert_eq!(
            result,
            Err(BridgeError::AlreadyExists("mint already registered"))
        );
    }

    #[test]
    fn test_lock_on_solana() {
        let mut bridge = SPLTokenBridge::new();
        let mint = [1; 32];
        let recipient = [2; 32];
        let request_id = bridge
            .lock_on_solana(mint, 1_000_000, recipient, 0)
            .unwrap();
        assert_ne!(request_id, [0; 32]);

        // Verify request is stored
        let stored = bridge
            .store
            .get_decoded::<TokenBridgeRequest>(&key_request(&request_id));
        assert!(stored.is_some());
        let req = stored.unwrap().unwrap();
        assert_eq!(req.source_chain, SPLTokenBridge::SOLANA_CHAIN_ID);
        assert_eq!(req.destination_chain, SPLTokenBridge::X3_CHAIN_ID);
        assert_eq!(req.amount, 1_000_000);
        assert_eq!(req.status, 1);
    }

    #[test]
    fn test_lock_zero_amount() {
        let mut bridge = SPLTokenBridge::new();
        let result = bridge.lock_on_solana([1; 32], 0, [2; 32], 0);
        assert_eq!(result, Err(BridgeError::InvalidAmount));
    }

    #[test]
    fn test_lock_duplicate_request() {
        let mut bridge = SPLTokenBridge::new();
        bridge
            .lock_on_solana([1; 32], 1_000_000, [2; 32], 0)
            .unwrap();
        let result = bridge.lock_on_solana([1; 32], 1_000_000, [2; 32], 0);
        assert_eq!(
            result,
            Err(BridgeError::AlreadyExists("request already exists"))
        );
    }

    #[test]
    fn test_mint_wrapped_on_x3() {
        let mut bridge = SPLTokenBridge::new();
        let mint = [1; 32];
        let recipient = [3; 32];

        // Lock first
        let request_id = bridge
            .lock_on_solana(mint, 1_000_000, recipient, 0)
            .unwrap();

        // Mint should succeed
        let balance = bridge
            .mint_wrapped_on_x3(request_id, mint, 1_000_000, recipient)
            .unwrap();
        assert_eq!(balance.balance, 1_000_000);
        assert_eq!(
            balance.token_id,
            SPLTokenBridge::derive_wrapped_token_id(&mint)
        );

        // Verify balance persisted
        let x3_token_id = SPLTokenBridge::derive_wrapped_token_id(&mint);
        let bal_key = key_balance(SPLTokenBridge::X3_CHAIN_ID, &recipient, x3_token_id);
        let stored = bridge.store.get_decoded::<BridgedBalance>(&bal_key);
        assert!(stored.is_some());
        assert_eq!(stored.unwrap().unwrap().balance, 1_000_000);

        // Verify request marked as minted
        let req = bridge
            .store
            .get_decoded::<TokenBridgeRequest>(&key_request(&request_id))
            .unwrap()
            .unwrap();
        assert_eq!(req.status, 2);
    }

    #[test]
    fn test_mint_wrapped_missing_request() {
        let mut bridge = SPLTokenBridge::new();
        let result = bridge.mint_wrapped_on_x3([9; 32], [1; 32], 1_000_000, [3; 32]);
        assert_eq!(result, Err(BridgeError::NotFound("lock request not found")));
    }

    #[test]
    fn test_mint_cumulative() {
        let mut bridge = SPLTokenBridge::new();
        let mint = [1; 32];
        let recipient = [3; 32];

        let rid1 = bridge.lock_on_solana(mint, 500_000, recipient, 0).unwrap();
        bridge
            .mint_wrapped_on_x3(rid1, mint, 500_000, recipient)
            .unwrap();

        let rid2 = bridge.lock_on_solana(mint, 300_000, recipient, 1).unwrap();
        let bal = bridge
            .mint_wrapped_on_x3(rid2, mint, 300_000, recipient)
            .unwrap();
        assert_eq!(bal.balance, 800_000);
    }

    #[test]
    fn test_burn_on_x3() {
        let mut bridge = SPLTokenBridge::new();
        let mint = [1; 32];
        let recipient = [4; 32];
        let token_id = SPLTokenBridge::derive_wrapped_token_id(&mint);

        // Lock + mint to create balance
        let req_id = bridge.lock_on_solana(mint, 500_000, recipient, 0).unwrap();
        bridge
            .mint_wrapped_on_x3(req_id, mint, 500_000, recipient)
            .unwrap();

        // Burn
        let burn_id = bridge.burn_on_x3(token_id, 200_000, recipient, 0).unwrap();
        assert_ne!(burn_id, [0; 32]);

        // Verify burn request stored
        let stored = bridge
            .store
            .get_decoded::<TokenBridgeRequest>(&key_request(&burn_id));
        assert!(stored.is_some());
        let burn_req = stored.unwrap().unwrap();
        assert_eq!(burn_req.amount, 200_000);
        assert_eq!(burn_req.source_chain, SPLTokenBridge::X3_CHAIN_ID);
        assert_eq!(burn_req.destination_chain, SPLTokenBridge::SOLANA_CHAIN_ID);
        assert_eq!(burn_req.status, 1);

        // Verify balance deducted
        let bal_key = key_balance(SPLTokenBridge::X3_CHAIN_ID, &recipient, token_id);
        let remaining = bridge
            .store
            .get_decoded::<BridgedBalance>(&bal_key)
            .unwrap()
            .unwrap();
        assert_eq!(remaining.balance, 300_000);
        assert_eq!(remaining.locked_amount, 200_000);
    }

    #[test]
    fn test_burn_insufficient_balance() {
        let mut bridge = SPLTokenBridge::new();
        let mint = [1; 32];
        let recipient = [4; 32];
        let token_id = SPLTokenBridge::derive_wrapped_token_id(&mint);

        let req_id = bridge.lock_on_solana(mint, 100, recipient, 0).unwrap();
        bridge
            .mint_wrapped_on_x3(req_id, mint, 100, recipient)
            .unwrap();

        let result = bridge.burn_on_x3(token_id, 200, recipient, 0);
        assert_eq!(result, Err(BridgeError::InsufficientBalance));
    }

    #[test]
    fn test_burn_no_balance() {
        let mut bridge = SPLTokenBridge::new();
        let result = bridge.burn_on_x3(42, 100, [4; 32], 0);
        assert_eq!(result, Err(BridgeError::NotFound("balance not found")));
    }

    #[test]
    fn test_unlock_on_solana() {
        let mut bridge = SPLTokenBridge::new();
        let mint = [1; 32];
        let recipient = [5; 32];
        let token_id = SPLTokenBridge::derive_wrapped_token_id(&mint);

        // Lock + mint on X3
        let req_id = bridge.lock_on_solana(mint, 500_000, recipient, 0).unwrap();
        bridge
            .mint_wrapped_on_x3(req_id, mint, 500_000, recipient)
            .unwrap();

        // Burn on X3
        let burn_id = bridge.burn_on_x3(token_id, 200_000, recipient, 1).unwrap();

        // Unlock on Solana
        let result = bridge
            .unlock_on_solana(burn_id, mint, 200_000, recipient)
            .unwrap();
        assert!(result);

        // Verify burn request marked completed
        let stored = bridge
            .store
            .get_decoded::<TokenBridgeRequest>(&key_request(&burn_id));
        assert!(stored.is_some());
        assert_eq!(stored.unwrap().unwrap().status, 2);

        // Verify Solana-side balance recorded
        let sol_key = key_balance(SPLTokenBridge::SOLANA_CHAIN_ID, &recipient, token_id);
        let sol_bal = bridge
            .store
            .get_decoded::<BridgedBalance>(&sol_key)
            .unwrap()
            .unwrap();
        assert_eq!(sol_bal.balance, 200_000);
    }

    #[test]
    fn test_unlock_missing_request() {
        let mut bridge = SPLTokenBridge::new();
        let result = bridge.unlock_on_solana([9; 32], [1; 32], 100, [5; 32]);
        assert_eq!(result, Err(BridgeError::NotFound("burn request not found")));
    }

    #[test]
    fn test_create_bridge_vault() {
        let vault_owner = [1; 32];
        let mint = [2; 32];
        let vault = SPLTokenBridge::create_bridge_vault(vault_owner, mint).unwrap();
        assert_eq!(vault.vault_balance, 0);
        assert!(!vault.is_locked);
    }

    #[test]
    fn test_deposit_to_vault() {
        let mut vault = BridgeVault {
            vault_owner: [1; 32],
            token_mint: [2; 32],
            vault_balance: 100,
            is_locked: false,
            chain_id: 101,
        };

        let balance = SPLTokenBridge::deposit_to_vault(&mut vault, 50).unwrap();
        assert_eq!(balance, 150);
    }

    #[test]
    fn test_deposit_locked_vault() {
        let mut vault = BridgeVault {
            vault_owner: [1; 32],
            token_mint: [2; 32],
            vault_balance: 100,
            is_locked: true,
            chain_id: 101,
        };

        let result = SPLTokenBridge::deposit_to_vault(&mut vault, 50);
        assert_eq!(result, Err(BridgeError::VaultLocked));
    }

    #[test]
    fn test_withdraw_from_vault() {
        let mut vault = BridgeVault {
            vault_owner: [1; 32],
            token_mint: [2; 32],
            vault_balance: 200,
            is_locked: false,
            chain_id: 101,
        };

        let balance = SPLTokenBridge::withdraw_from_vault(&mut vault, 75).unwrap();
        assert_eq!(balance, 125);
    }

    #[test]
    fn test_withdraw_insufficient_balance() {
        let mut vault = BridgeVault {
            vault_owner: [1; 32],
            token_mint: [2; 32],
            vault_balance: 50,
            is_locked: false,
            chain_id: 101,
        };

        let result = SPLTokenBridge::withdraw_from_vault(&mut vault, 100);
        assert_eq!(result, Err(BridgeError::InsufficientBalance));
    }

    #[test]
    fn test_calculate_bridge_fee() {
        let fee = SPLTokenBridge::calculate_bridge_fee(1_000_000);
        assert_eq!(fee, 1_000);
    }

    #[test]
    fn test_get_total_locked_supply() {
        let vaults = vec![
            BridgeVault {
                vault_owner: [1; 32],
                token_mint: [2; 32],
                vault_balance: 100,
                is_locked: false,
                chain_id: 101,
            },
            BridgeVault {
                vault_owner: [1; 32],
                token_mint: [2; 32],
                vault_balance: 200,
                is_locked: false,
                chain_id: 101,
            },
        ];

        let total = SPLTokenBridge::get_total_locked_supply([2; 32], &vaults);
        assert_eq!(total, 300);
    }

    #[test]
    fn test_set_bridge_paused() {
        let mut bridge = SPLTokenBridge::new();
        let mint = [1; 32];

        bridge.register_spl_mint(mint, 6).unwrap();

        let state = bridge.set_bridge_paused(mint, true).unwrap();
        assert!(!state);

        // Verify paused state stored
        assert!(bridge.is_bridge_paused(&mint));

        // Verify mint record updated
        let stored = bridge
            .store
            .get_decoded::<SPLTokenMint>(&key_mint(&mint))
            .unwrap()
            .unwrap();
        assert!(!stored.is_bridged);
    }

    #[test]
    fn test_pause_toggle() {
        let mut bridge = SPLTokenBridge::new();
        let mint = [1; 32];

        bridge.set_bridge_paused(mint, true).unwrap();
        assert!(bridge.is_bridge_paused(&mint));

        bridge.set_bridge_paused(mint, false).unwrap();
        assert!(!bridge.is_bridge_paused(&mint));
    }

    #[test]
    fn test_full_bridge_roundtrip() {
        let mut bridge = SPLTokenBridge::new();

        let sol_mint = [42; 32];
        let user = [99; 32];
        let sol_recipient = [88; 32];

        // 1. Register
        let token_id = bridge.register_spl_mint(sol_mint, 9).unwrap();
        assert!(token_id > 0);

        // 2. Lock on Solana
        let lock_id = bridge.lock_on_solana(sol_mint, 1_000_000, user, 1).unwrap();

        // 3. Mint on X3
        let bal = bridge
            .mint_wrapped_on_x3(lock_id, sol_mint, 1_000_000, user)
            .unwrap();
        assert_eq!(bal.balance, 1_000_000);

        // 4. Burn on X3
        let burn_id = bridge.burn_on_x3(token_id, 400_000, user, 2).unwrap();

        // 5. Unlock on Solana
        let unlocked = bridge
            .unlock_on_solana(burn_id, sol_mint, 400_000, sol_recipient)
            .unwrap();
        assert!(unlocked);

        // Verify final X3 balance
        let x3_key = key_balance(SPLTokenBridge::X3_CHAIN_ID, &user, token_id);
        let x3_bal = bridge
            .store
            .get_decoded::<BridgedBalance>(&x3_key)
            .unwrap()
            .unwrap();
        assert_eq!(x3_bal.balance, 600_000);

        // Verify Solana unlock balance
        let sol_key = key_balance(SPLTokenBridge::SOLANA_CHAIN_ID, &sol_recipient, token_id);
        let sol_bal = bridge
            .store
            .get_decoded::<BridgedBalance>(&sol_key)
            .unwrap()
            .unwrap();
        assert_eq!(sol_bal.balance, 400_000);
    }

    #[test]
    fn test_bridge_error_debug() {
        let err = BridgeError::NotFound("test");
        assert!(format!("{:?}", err).contains("test"));
    }
}
