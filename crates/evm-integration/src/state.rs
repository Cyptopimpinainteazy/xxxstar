/// EVM State Integration for X3 Chain
///
/// Manages Ethereum Virtual Machine state, account storage, and gas metering.
use sp_runtime::traits::Zero;
use sp_std::{collections::btree_map::BTreeMap, vec::Vec};

/// EVM account state
#[derive(Clone, Debug, Default)]
pub struct EvmAccount {
    /// Account nonce
    pub nonce: u64,
    /// Account balance
    pub balance: u128,
    /// Account code hash
    pub code_hash: [u8; 32],
    /// Storage root
    pub storage_root: [u8; 32],
}

impl EvmAccount {
    /// Create new EVM account
    pub fn new() -> Self {
        Self::default()
    }

    /// Set account balance
    pub fn set_balance(&mut self, balance: u128) {
        self.balance = balance;
    }

    /// Increment nonce
    pub fn increment_nonce(&mut self) {
        self.nonce = self.nonce.saturating_add(1);
    }

    /// Check if account is empty
    pub fn is_empty(&self) -> bool {
        self.balance.is_zero() && self.nonce == 0 && self.code_hash == [0u8; 32]
    }
}

/// EVM contract code
#[derive(Clone, Debug)]
pub struct EvmCode {
    /// Contract bytecode
    pub bytecode: Vec<u8>,
    /// Code hash
    pub code_hash: [u8; 32],
}

impl EvmCode {
    /// Create new EVM code
    pub fn new(bytecode: Vec<u8>) -> Self {
        use sp_io::hashing::keccak_256;
        let code_hash = keccak_256(&bytecode);
        Self {
            bytecode,
            code_hash,
        }
    }

    /// Get code size
    pub fn len(&self) -> usize {
        self.bytecode.len()
    }

    /// Check if code is empty
    pub fn is_empty(&self) -> bool {
        self.bytecode.is_empty()
    }
}

/// EVM storage entry
pub type StorageValue = [u8; 32];

/// EVM state database
#[derive(Default)]
pub struct EvmStateDb {
    accounts: BTreeMap<[u8; 20], EvmAccount>,
    code: BTreeMap<[u8; 32], EvmCode>,
    storage: BTreeMap<([u8; 20], [u8; 32]), StorageValue>,
}

impl EvmStateDb {
    /// Create new EVM state database
    pub fn new() -> Self {
        Self::default()
    }

    /// Get account by address
    pub fn account(&self, address: &[u8; 20]) -> Option<&EvmAccount> {
        self.accounts.get(address)
    }

    /// Get mutable account reference
    pub fn account_mut(&mut self, address: &[u8; 20]) -> &mut EvmAccount {
        self.accounts.entry(*address).or_default()
    }

    /// Get account nonce
    pub fn nonce(&self, address: &[u8; 20]) -> u64 {
        self.account(address).map(|a| a.nonce).unwrap_or(0)
    }

    /// Get account balance
    pub fn balance(&self, address: &[u8; 20]) -> u128 {
        self.account(address).map(|a| a.balance).unwrap_or(0)
    }

    /// Set account balance
    pub fn set_balance(&mut self, address: &[u8; 20], balance: u128) {
        self.account_mut(address).set_balance(balance);
    }

    /// Transfer between accounts with overflow/underflow protection
    pub fn transfer(
        &mut self,
        from: &[u8; 20],
        to: &[u8; 20],
        value: u128,
    ) -> Result<(), &'static str> {
        let from_balance = self.balance(from);
        from_balance
            .checked_sub(value)
            .ok_or("Insufficient balance")?;

        self.account_mut(from).balance = from_balance - value;
        let to_balance = self.balance(to);
        self.account_mut(to).balance = to_balance.checked_add(value).ok_or("Balance overflow")?;

        Ok(())
    }

    /// Get code by hash
    pub fn code(&self, code_hash: &[u8; 32]) -> Option<&EvmCode> {
        self.code.get(code_hash)
    }

    /// Set code at address
    pub fn set_code(&mut self, address: &[u8; 20], code: EvmCode) {
        let code_hash = code.code_hash;
        self.code.insert(code_hash, code);
        self.account_mut(address).code_hash = code_hash;
    }

    /// Get storage value
    pub fn storage(&self, address: &[u8; 20], key: &[u8; 32]) -> StorageValue {
        self.storage
            .get(&(*address, *key))
            .copied()
            .unwrap_or([0u8; 32])
    }

    /// Set storage value
    pub fn set_storage(&mut self, address: &[u8; 20], key: [u8; 32], value: StorageValue) {
        self.storage.insert((*address, key), value);
    }

    /// Get account count
    pub fn account_count(&self) -> usize {
        self.accounts.len()
    }

    /// Get all accounts
    pub fn accounts(&self) -> impl Iterator<Item = (&[u8; 20], &EvmAccount)> {
        self.accounts.iter()
    }
}

/// EVM execution context
#[derive(Clone, Debug)]
pub struct EvmContext {
    /// Current block number
    pub block_number: u32,
    /// Current block timestamp
    pub block_timestamp: u64,
    /// Gas price
    pub gas_price: u128,
    /// Call origin
    pub origin: [u8; 20],
    /// Caller address
    pub caller: [u8; 20],
    /// Call value
    pub call_value: u128,
    /// Gas limit
    pub gas_limit: u64,
}

impl EvmContext {
    /// Create new EVM context
    pub fn new(origin: [u8; 20]) -> Self {
        Self {
            block_number: 0,
            block_timestamp: 0,
            gas_price: 1,
            origin,
            caller: origin,
            call_value: 0,
            gas_limit: 1_000_000,
        }
    }

    /// Set block context
    pub fn with_block(mut self, number: u32, timestamp: u64) -> Self {
        self.block_number = number;
        self.block_timestamp = timestamp;
        self
    }

    /// Set gas parameters
    pub fn with_gas(mut self, limit: u64, price: u128) -> Self {
        self.gas_limit = limit;
        self.gas_price = price;
        self
    }
}

/// Gas metering tracker for EVM execution
pub struct GasMeter {
    /// Gas limit for the execution
    limit: u64,
    /// Gas consumed so far
    consumed: u64,
    /// Gas refunded (SSTORE clears etc.)
    refunded: u64,
}

impl GasMeter {
    /// Create a new gas meter with the given limit
    pub fn new(limit: u64) -> Self {
        Self {
            limit,
            consumed: 0,
            refunded: 0,
        }
    }

    /// Consume gas; returns Err if out of gas
    pub fn consume(&mut self, amount: u64) -> Result<(), &'static str> {
        let new_consumed = self.consumed.checked_add(amount).ok_or("gas overflow")?;
        if new_consumed > self.limit {
            return Err("out of gas");
        }
        self.consumed = new_consumed;
        Ok(())
    }

    /// Record a gas refund
    pub fn refund(&mut self, amount: u64) {
        self.refunded = self.refunded.saturating_add(amount);
    }

    /// Get remaining gas
    pub fn remaining(&self) -> u64 {
        self.limit.saturating_sub(self.consumed)
    }

    /// Get effective gas used (consumed - min(refunded, consumed/5) per EIP-3529)
    pub fn effective_gas_used(&self) -> u64 {
        let max_refund = self.consumed / 5;
        let actual_refund = core::cmp::min(self.refunded, max_refund);
        self.consumed.saturating_sub(actual_refund)
    }

    /// Get total consumed gas
    pub fn consumed(&self) -> u64 {
        self.consumed
    }

    /// Get total refunded gas
    pub fn refunded_gas(&self) -> u64 {
        self.refunded
    }
}

/// Compute state root hash from the state database
pub fn compute_state_root(db: &EvmStateDb) -> [u8; 32] {
    use sp_io::hashing::blake2_256;

    let mut data = Vec::new();
    for (addr, account) in db.accounts() {
        data.extend_from_slice(addr);
        data.extend_from_slice(&account.nonce.to_le_bytes());
        data.extend_from_slice(&account.balance.to_le_bytes());
        data.extend_from_slice(&account.code_hash);
        data.extend_from_slice(&account.storage_root);
    }

    if data.is_empty() {
        [0u8; 32]
    } else {
        blake2_256(&data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_evm_account() {
        let mut account = EvmAccount::new();
        assert!(account.is_empty());

        account.set_balance(1000);
        assert!(!account.is_empty());
        assert_eq!(account.balance, 1000);

        account.increment_nonce();
        assert_eq!(account.nonce, 1);
    }

    #[test]
    fn test_evm_state_db_transfer() {
        let mut db = EvmStateDb::new();
        let addr1 = [1u8; 20];
        let addr2 = [2u8; 20];

        db.set_balance(&addr1, 1000);
        assert!(db.transfer(&addr1, &addr2, 500).is_ok());
        assert_eq!(db.balance(&addr1), 500);
        assert_eq!(db.balance(&addr2), 500);
    }

    #[test]
    fn test_evm_state_db_insufficient_balance() {
        let mut db = EvmStateDb::new();
        let addr1 = [1u8; 20];
        let addr2 = [2u8; 20];

        db.set_balance(&addr1, 100);
        assert!(db.transfer(&addr1, &addr2, 200).is_err());
    }

    #[test]
    fn test_evm_code() {
        let bytecode = vec![0x60, 0x01, 0x61]; // PUSH1 01 PUSH2
        let code = EvmCode::new(bytecode.clone());
        assert_eq!(code.len(), 3);
        assert!(!code.is_empty());
    }

    #[test]
    fn test_gas_meter_basic() {
        let mut meter = GasMeter::new(1_000_000);
        assert_eq!(meter.remaining(), 1_000_000);

        assert!(meter.consume(21_000).is_ok());
        assert_eq!(meter.remaining(), 979_000);
        assert_eq!(meter.consumed(), 21_000);
    }

    #[test]
    fn test_gas_meter_out_of_gas() {
        let mut meter = GasMeter::new(100);
        assert!(meter.consume(50).is_ok());
        assert!(meter.consume(51).is_err());
    }

    #[test]
    fn test_gas_meter_refund() {
        let mut meter = GasMeter::new(100_000);
        meter.consume(50_000).unwrap();
        meter.refund(15_000);
        // EIP-3529: max refund = consumed/5 = 10,000
        assert_eq!(meter.effective_gas_used(), 40_000); // 50000 - min(15000, 10000)
    }

    #[test]
    fn test_evm_context_builder() {
        let ctx = EvmContext::new([0xAA; 20])
            .with_block(100, 1_700_000_000)
            .with_gas(5_000_000, 20_000_000_000);
        assert_eq!(ctx.block_number, 100);
        assert_eq!(ctx.gas_limit, 5_000_000);
        assert_eq!(ctx.gas_price, 20_000_000_000);
    }

    #[test]
    fn test_state_db_storage() {
        let mut db = EvmStateDb::new();
        let addr = [0x01; 20];
        let key = [0x02; 32];
        let val = [0x03; 32];

        db.set_storage(&addr, key, val);
        assert_eq!(db.storage(&addr, &key), val);
    }

    #[test]
    fn test_state_db_code() {
        let mut db = EvmStateDb::new();
        let addr = [0x01; 20];
        let code = EvmCode::new(vec![0x60, 0x01]);

        let code_hash = code.code_hash;
        db.set_code(&addr, code);

        let stored = db.code(&code_hash).unwrap();
        assert_eq!(stored.len(), 2);
        assert_eq!(db.account(&addr).unwrap().code_hash, code_hash);
    }

    #[test]
    fn test_compute_state_root_empty() {
        let db = EvmStateDb::new();
        assert_eq!(compute_state_root(&db), [0u8; 32]);
    }

    #[test]
    fn test_compute_state_root_deterministic() {
        let mut db = EvmStateDb::new();
        db.set_balance(&[1u8; 20], 1000);
        db.set_balance(&[2u8; 20], 2000);

        let root1 = compute_state_root(&db);
        let root2 = compute_state_root(&db);
        assert_eq!(root1, root2);
        assert_ne!(root1, [0u8; 32]);
    }
}
