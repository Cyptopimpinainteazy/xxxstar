//! Runtime API for X3 Kernel pallet

use parity_scale_codec::Codec;
use sp_core::H256;
use sp_std::vec::Vec;

sp_api::decl_runtime_apis! {
    pub trait AtlasKernelApi<AccountId, Balance, AssetId>
    where
        AccountId: Codec,
        Balance: Codec,
        AssetId: Codec,
    {
        /// Query canonical ledger balance for an EVM address
        fn get_evm_balance(
            account: Vec<u8>,
            asset_id: AssetId,
        ) -> Option<Balance>;

        /// Query contract bytecode
        fn get_evm_code(address: Vec<u8>) -> Vec<u8>;

        /// Query contract storage
        fn get_evm_storage(
            address: Vec<u8>,
            storage_key: H256,
        ) -> Option<H256>;

        /// Get asset metadata (symbol and decimals)
        fn get_asset_metadata(asset_id: AssetId) -> Option<(Vec<u8>, u8)>;

        /// Check if account is authorized for Comits
        fn is_authorized(account: Vec<u8>) -> bool;

        /// Get all authorized accounts
        fn get_authorized_accounts() -> Vec<Vec<u8>>;

        /// Get current authority set
        fn get_authorities() -> Vec<Vec<u8>>;

        /// Deploy a new EVM contract dynamically
        /// Returns the deployed contract address on success
        fn deploy_evm_contract(
            bytecode: Vec<u8>,
            salt: Option<Vec<u8>>,
            init_code_hash: Option<Vec<u8>>,
        ) -> Result<Vec<u8>, sp_runtime::DispatchError>;
    }
}
