//! Lazy chain-backed SputnikVM state. Every key lives in Substrate's transactional trie.
use alloc::vec::Vec;
use evm::backend::{Apply, Backend, Basic, MemoryBackend};
use primitive_types::{H160, H256, U256};

const PREFIX: &[u8] = b"x3:evm:v1:";
fn key(address: H160, kind: u8) -> Vec<u8> {
    let mut key = PREFIX.to_vec();
    key.extend_from_slice(address.as_bytes());
    key.push(kind);
    key
}
fn slot_key(address: H160, slot: H256) -> Vec<u8> {
    let mut result = key(address, b's');
    // Changing the generation resets storage in constant time. Old generations
    // remain unreachable; pruning can reclaim them without changing execution.
    let generation = sp_io::storage::get(&key(address, b'g')).unwrap_or_default();
    result.extend_from_slice(&generation);
    result.extend_from_slice(slot.as_bytes());
    result
}
fn reset(address: H160) {
    let generation_key = key(address, b'g');
    let mut previous = sp_io::storage::get(&generation_key)
        .unwrap_or_default()
        .to_vec();
    previous.extend_from_slice(address.as_bytes());
    sp_io::storage::set(&generation_key, &sp_io::hashing::blake2_256(&previous));
}

pub(crate) struct ChainBackend<'a> {
    memory: MemoryBackend<'a>,
    persistent: bool,
}
impl<'a> ChainBackend<'a> {
    pub(crate) fn new(memory: MemoryBackend<'a>, persistent: bool) -> Self {
        Self { memory, persistent }
    }
    pub(crate) fn apply_changes<A, I>(&mut self, changes: A)
    where
        A: IntoIterator<Item = Apply<I>>,
        I: IntoIterator<Item = (H256, H256)>,
    {
        if !self.persistent {
            return;
        }
        for change in changes {
            match change {
                Apply::Modify {
                    address,
                    basic,
                    code,
                    storage,
                    reset_storage,
                } => {
                    let code = code.unwrap_or_else(|| self.code(address));
                    // EIP-161: touching an empty account must not make it exist.
                    if basic.nonce.is_zero() && basic.balance.is_zero() && code.is_empty() {
                        if sp_io::storage::exists(&key(address, b'b')) {
                            reset(address);
                            sp_io::storage::clear(&key(address, b'b'));
                            sp_io::storage::clear(&key(address, b'c'));
                        }
                        continue;
                    }
                    if reset_storage {
                        reset(address);
                    }
                    let mut encoded = basic.nonce.to_big_endian().to_vec();
                    encoded.extend_from_slice(&basic.balance.to_big_endian());
                    sp_io::storage::set(&key(address, b'b'), &encoded);
                    // Seeded payload code must survive after the first call even
                    // when Sputnik does not include unchanged code in its delta.
                    sp_io::storage::set(&key(address, b'c'), &code);
                    for (slot, value) in storage {
                        let key = slot_key(address, slot);
                        if value == H256::zero() {
                            sp_io::storage::clear(&key);
                        } else {
                            sp_io::storage::set(&key, value.as_bytes());
                        }
                    }
                }
                Apply::Delete { address } => {
                    reset(address);
                    sp_io::storage::clear(&key(address, b'b'));
                    sp_io::storage::clear(&key(address, b'c'));
                }
            }
        }
    }
}
macro_rules! context {
    ($($name:ident -> $ty:ty),* $(,)?) => {$(fn $name(&self) -> $ty { self.memory.$name() })*};
}
impl Backend for ChainBackend<'_> {
    context! {
        gas_price -> U256, origin -> H160, block_number -> U256,
        block_coinbase -> H160, block_timestamp -> U256, block_difficulty -> U256,
        block_randomness -> Option<H256>, block_gas_limit -> U256,
        block_base_fee_per_gas -> U256, chain_id -> U256,
    }
    fn block_hash(&self, number: U256) -> H256 {
        self.memory.block_hash(number)
    }
    fn exists(&self, address: H160) -> bool {
        (self.persistent && sp_io::storage::exists(&key(address, b'b')))
            || self.memory.exists(address)
    }
    fn basic(&self, address: H160) -> Basic {
        if self.persistent {
            if let Some(bytes) = sp_io::storage::get(&key(address, b'b')) {
                // Corrupt state must never be interpreted as a fresh account.
                assert_eq!(bytes.len(), 64, "Invalid persisted EVM account");
                return Basic {
                    nonce: U256::from_big_endian(&bytes[..32]),
                    balance: U256::from_big_endian(&bytes[32..]),
                };
            }
        }
        self.memory.basic(address)
    }
    fn code(&self, address: H160) -> Vec<u8> {
        if self.persistent {
            if let Some(code) = sp_io::storage::get(&key(address, b'c')) {
                return code.to_vec();
            }
        }
        self.memory.code(address)
    }
    fn storage(&self, address: H160, index: H256) -> H256 {
        if self.persistent {
            return sp_io::storage::get(&slot_key(address, index))
                .map(|bytes| {
                    assert_eq!(bytes.len(), 32, "Invalid persisted EVM slot");
                    H256::from_slice(&bytes)
                })
                .unwrap_or_default();
        }
        self.memory.storage(address, index)
    }
    fn transient_storage(&self, _address: H160, _index: H256) -> H256 {
        H256::zero()
    }
    fn original_storage(&self, address: H160, index: H256) -> Option<H256> {
        Some(self.storage(address, index))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{mini_evm::execute_evm_persistent, EvmConfig};
    use sp_core::{H160 as SpH160, U256 as SpU256};

    // Increment storage[0] and return the new value.
    const INCREMENT: &[u8] = &[
        0x60, 0, 0x54, 0x60, 1, 0x01, 0x80, 0x60, 0, 0x55, 0x60, 0, 0x52, 0x60, 32, 0x60, 0, 0xf3,
    ];
    fn execute(code: &[u8]) -> crate::EvmResult<crate::EvmExecutionResult> {
        execute_evm_persistent(code, SpH160::zero(), SpU256::zero(), &EvmConfig::default())
    }

    #[test]
    fn state_survives_externalities_restart_and_metering_uses_original_storage() {
        let mut ext = sp_io::TestExternalities::default();
        let first = ext.execute_with(|| execute(INCREMENT).unwrap());
        let snapshot = ext.execute_with(|| sp_io::storage::root(sp_runtime::StateVersion::V1));
        // Commit the host overlay, as block import does, before taking the
        // persisted trie snapshot. Raw snapshots intentionally exclude overlays.
        ext.commit_all().unwrap();
        let (raw, root) = ext.into_raw_snapshot();
        let mut ext =
            sp_io::TestExternalities::from_raw_snapshot(raw, root, sp_runtime::StateVersion::V1);
        let second = ext.execute_with(|| execute(INCREMENT).unwrap());
        assert_eq!(first.output[31], 1);
        assert_eq!(second.output[31], 2);
        assert!(second.gas_used < first.gas_used);
        assert_ne!(
            snapshot,
            ext.execute_with(|| sp_io::storage::root(sp_runtime::StateVersion::V1))
        );
        assert_ne!(first.state_root, second.state_root);
    }

    #[test]
    fn enclosing_cross_vm_transaction_rolls_back_successful_evm_writes() {
        sp_io::TestExternalities::default().execute_with(|| {
            let before = sp_io::storage::root(sp_runtime::StateVersion::V1);
            sp_io::storage::start_transaction();
            assert_eq!(execute(INCREMENT).unwrap().output[31], 1);
            sp_io::storage::rollback_transaction();
            assert_eq!(before, sp_io::storage::root(sp_runtime::StateVersion::V1));
            assert_eq!(execute(INCREMENT).unwrap().output[31], 1);
        });
    }

    #[test]
    fn revert_and_out_of_gas_publish_no_state() {
        sp_io::TestExternalities::default().execute_with(|| {
            let before = sp_io::storage::root(sp_runtime::StateVersion::V1);
            let revert = [0x60, 42, 0x60, 0, 0x55, 0x60, 0, 0x60, 0, 0xfd];
            assert!(execute(&revert).is_err());
            assert_eq!(before, sp_io::storage::root(sp_runtime::StateVersion::V1));
            let config = EvmConfig {
                gas_limit: 21_001,
                ..EvmConfig::default()
            };
            assert!(
                execute_evm_persistent(INCREMENT, SpH160::zero(), SpU256::zero(), &config).is_err()
            );
            assert_eq!(before, sp_io::storage::root(sp_runtime::StateVersion::V1));
        });
    }

    #[test]
    fn zero_balance_caller_cannot_mint_value() {
        sp_io::TestExternalities::default().execute_with(|| {
            let before = sp_io::storage::root(sp_runtime::StateVersion::V1);
            assert!(execute_evm_persistent(
                INCREMENT,
                SpH160::zero(),
                SpU256::one(),
                &EvmConfig::default()
            )
            .is_err());
            assert_eq!(before, sp_io::storage::root(sp_runtime::StateVersion::V1));
        });
    }

    #[test]
    fn independent_hosts_agree_on_state_root_and_account_nonce() {
        let run = || {
            sp_io::TestExternalities::default().execute_with(|| {
                let receipt = execute(INCREMENT).unwrap();
                let caller = sp_io::storage::get(&key(H160::zero(), b'b')).unwrap();
                assert_eq!(U256::from_big_endian(&caller[..32]), U256::one());
                assert_eq!(U256::from_big_endian(&caller[32..]), U256::zero());
                (receipt, sp_io::storage::root(sp_runtime::StateVersion::V1))
            })
        };
        let first = run();
        let second = run();
        assert_eq!(first.0, second.0);
        assert_eq!(first.1, second.1);
    }

    #[test]
    fn distinct_content_addresses_keep_independent_storage() {
        sp_io::TestExternalities::default().execute_with(|| {
            let mut other = INCREMENT.to_vec();
            other.push(0); // Unreachable STOP changes the content address.
            assert_eq!(execute(INCREMENT).unwrap().output[31], 1);
            assert_eq!(execute(&other).unwrap().output[31], 1);
            assert_eq!(execute(INCREMENT).unwrap().output[31], 2);
            assert_eq!(execute(&other).unwrap().output[31], 2);
        });
    }

    #[test]
    fn create_persists_child_code_and_uses_parent_nonce_across_transactions() {
        sp_io::TestExternalities::default().execute_with(|| {
            // Copy init code to memory, CREATE, then return its address. Init
            // code deploys one STOP byte; actual CREATE runs inside Sputnik.
            let code = [
                0x60, 10, 0x60, 22, 0x60, 0, 0x39, 0x60, 10, 0x60, 0, 0x60, 0, 0xf0, 0x60, 0, 0x52,
                0x60, 32, 0x60, 0, 0xf3, 0x60, 0, 0x60, 0, 0x53, 0x60, 1, 0x60, 0, 0xf3,
            ];
            let first = execute(&code).unwrap();
            let child = H160::from_slice(&first.output[12..]);
            assert_ne!(child, H160::zero());
            assert_eq!(
                sp_io::storage::get(&key(child, b'c')).unwrap().as_ref(),
                &[0]
            );
            let second = execute(&code).unwrap();
            assert_ne!(
                first.output, second.output,
                "parent CREATE nonce must persist"
            );
            assert_eq!(
                sp_io::storage::get(&key(child, b'c')).unwrap().as_ref(),
                &[0]
            );
        });
    }

    #[test]
    fn real_interpreter_logs_are_retained_in_receipt() {
        sp_io::TestExternalities::default().execute_with(|| {
            let code = [0x60, 42, 0x60, 0, 0x52, 0x60, 32, 0x60, 0, 0xa0, 0x00];
            let receipt = execute(&code).unwrap();
            assert_eq!(receipt.logs.len(), 1);
            assert_eq!(receipt.logs[0].data.len(), 32);
            assert_eq!(receipt.logs[0].data[31], 42);
            assert!(receipt.logs[0].topics.is_empty());
        });
    }

    #[test]
    fn touching_empty_account_does_not_create_it() {
        sp_io::TestExternalities::default().execute_with(|| {
            // CALL an absent account with zero value; EIP-161 leaves it absent.
            let code = [
                0x60, 0, 0x60, 0, 0x60, 0, 0x60, 0, 0x60, 0, 0x60, 0x44, 0x61, 0xff, 0xff, 0xf1,
                0x00,
            ];
            execute(&code).unwrap();
            assert!(!sp_io::storage::exists(&key(
                H160::from_low_u64_be(0x44),
                b'b'
            )));
        });
    }

    #[test]
    fn generations_reset_slots_without_resurrecting_deleted_values() {
        sp_io::TestExternalities::default().execute_with(|| {
            let address = H160::repeat_byte(42);
            let slot = H256::zero();
            sp_io::storage::set(&slot_key(address, slot), H256::repeat_byte(1).as_bytes());
            reset(address);
            assert!(sp_io::storage::get(&slot_key(address, slot)).is_none());
            sp_io::storage::set(&slot_key(address, slot), H256::repeat_byte(2).as_bytes());
            reset(address);
            assert!(sp_io::storage::get(&slot_key(address, slot)).is_none());
        });
    }
}
