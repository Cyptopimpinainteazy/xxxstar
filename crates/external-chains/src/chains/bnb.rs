//! BNB Smart Chain Adapter
//!
//! Adapter for BNB Smart Chain (formerly Binance Smart Chain)
//! Chain ID: 56

use crate::adapter::*;
use crate::error::ExternalChainError;
use crate::ChainType;
use sp_core::{H160, H256, U256};
use sp_std::vec::Vec;

/// BNB Smart Chain adapter
pub struct BnbAdapter {
    config: ChainConfig,
    #[allow(dead_code)]
    nonce: u64,
}

impl BnbAdapter {
    /// Create new BNB adapter
    pub fn new(config: ChainConfig) -> Self {
        Self { config, nonce: 0 }
    }

    /// BNB Token Hub (cross-chain bridge to BNB Beacon Chain)
    pub const TOKEN_HUB: H160 = H160(hex_literal::hex!(
        "0000000000000000000000000000000000001004"
    ));

    /// Cross-chain contract for Beacon Chain communication
    pub const CROSS_CHAIN: H160 = H160(hex_literal::hex!(
        "0000000000000000000000000000000000002000"
    ));

    /// WBNB token
    pub const WBNB: H160 = H160(hex_literal::hex!(
        "bb4CdB9CBd36B01bD1cBaEBF2De08d9173bc095c"
    ));

    /// PancakeSwap Router (for reference - common DEX)
    pub const PANCAKE_ROUTER: H160 = H160(hex_literal::hex!(
        "10ED43C718714eb63d5aA57B78B54704E256024E"
    ));

    /// Encode cross-chain transfer to Beacon Chain
    pub fn encode_transfer_out(
        contract_addr: H160,
        recipient: Vec<u8>, // BNB Beacon Chain address (bech32)
        amount: U256,
        expire_time: u64,
    ) -> Vec<u8> {
        // transferOut(address,bytes,uint256,uint64)
        let mut calldata = Vec::with_capacity(4 + 32 * 4 + recipient.len());

        // Function selector
        calldata.extend_from_slice(&[0xaa, 0x7a, 0x56, 0x21]);

        // Contract address (token)
        calldata.extend_from_slice(&[0u8; 12]);
        calldata.extend_from_slice(contract_addr.as_bytes());

        // Recipient offset
        calldata.extend_from_slice(&[0u8; 31]);
        calldata.push(0x80);

        // Amount
        let amount_bytes = amount.to_big_endian();
        calldata.extend_from_slice(&amount_bytes);

        // Expire time
        calldata.extend_from_slice(&[0u8; 24]);
        calldata.extend_from_slice(&expire_time.to_be_bytes());

        // Recipient length
        let recipient_len = recipient.len() as u32;
        calldata.extend_from_slice(&[0u8; 28]);
        calldata.extend_from_slice(&recipient_len.to_be_bytes());

        // Recipient data
        calldata.extend_from_slice(&recipient);
        let padding = (32 - recipient.len() % 32) % 32;
        calldata.extend_from_slice(&vec![0u8; padding]);

        calldata
    }

    /// Encode BEP20 approve
    pub fn encode_approve(spender: H160, amount: U256) -> Vec<u8> {
        let mut calldata = Vec::with_capacity(4 + 64);

        // approve(address,uint256)
        calldata.extend_from_slice(&[0x09, 0x5e, 0xa7, 0xb3]);

        // Spender
        calldata.extend_from_slice(&[0u8; 12]);
        calldata.extend_from_slice(spender.as_bytes());

        // Amount
        let amount_bytes = amount.to_big_endian();
        calldata.extend_from_slice(&amount_bytes);

        calldata
    }

    /// Get validator set from system contract
    #[allow(dead_code)]
    async fn get_validators(&self) -> AdapterResult<Vec<H160>> {
        // Refused: this returned an empty validator set, which reads as "BSC
        // has no validators". The real set is stored in the system contract at
        // 0x...1000 and needs that contract's storage layout.
        Err(ExternalChainError::adapter_unimplemented(
            "bnb: the PoSA validator set lives in the system contract; this adapter cannot read \
             it and refuses rather than reporting an empty set",
        ))
    }
}

#[async_trait::async_trait]
impl ChainAdapter for BnbAdapter {
    fn chain_type(&self) -> ChainType {
        ChainType::Bnb
    }

    fn config(&self) -> &ChainConfig {
        &self.config
    }

    async fn is_connected(&self) -> bool {
        // Real `eth_chainId` probe: the endpoint must answer *as this chain*.
        matches!(
            crate::evm_rpc::chain_id(&crate::evm_rpc::url(&self.config)).await,
            Ok(id) if id == self.config.chain_type
        )
    }

    async fn get_block_number(&self) -> AdapterResult<u64> {
        crate::evm_rpc::block_number(&crate::evm_rpc::url(&self.config)).await
    }

    async fn get_balance(&self, address: H160) -> AdapterResult<U256> {
        crate::evm_rpc::balance(&crate::evm_rpc::url(&self.config), address).await
    }

    async fn get_token_balance(&self, token: H160, address: H160) -> AdapterResult<U256> {
        crate::evm_rpc::token_balance(&crate::evm_rpc::url(&self.config), token, address).await
    }

    async fn send_message(&self, _message: ChainMessage) -> AdapterResult<H256> {
        // Refused, not invented: this returned `message.hash()` for a message
        // that was never broadcast through the CrossChain contract.
        Err(ExternalChainError::adapter_unimplemented(
            "bnb: send_message needs a signed CrossChain contract transaction and this adapter \
             has no signer",
        ))
    }

    async fn receive_messages(&self) -> AdapterResult<Vec<ChainMessage>> {
        // Refused: an empty list reads as "no CrossChainPackage events".
        Err(ExternalChainError::adapter_unimplemented(
            "bnb: receive_messages cannot decode CrossChainPackage events yet",
        ))
    }

    async fn initiate_transfer(&self, _transfer: CrossChainTransfer) -> AdapterResult<H256> {
        // Refused: this returned `transfer.id` for a transfer never sent.
        Err(ExternalChainError::adapter_unimplemented(
            "bnb: initiate_transfer needs a signed TokenHub transaction and this adapter has no \
             signer",
        ))
    }

    async fn check_transfer_status(&self, _transfer_id: H256) -> AdapterResult<TransferStatus> {
        // Refused: this answered `Completed` for every transfer id.
        Err(ExternalChainError::adapter_unimplemented(
            "bnb: check_transfer_status needs the destination CrossChain state; nothing here can \
             tell a relayed message from an unrelayed one",
        ))
    }

    async fn verify_message_proof(
        &self,
        _message: &ChainMessage,
        _proof: &[u8],
    ) -> AdapterResult<bool> {
        // Refused, not shape-checked: a BSC cross-chain proof is a validator
        // signature set, and `!proof.is_empty()` accepts any byte string.
        Err(ExternalChainError::VerificationUnavailable)
    }

    async fn finalize_transfer(&self, _transfer_id: H256, _proof: Vec<u8>) -> AdapterResult<H256> {
        // Refused: "handled by relayer infrastructure" was not this adapter
        // doing anything, yet it returned the id as though it had.
        Err(ExternalChainError::adapter_unimplemented(
            "bnb: finalize_transfer needs a signed destination transaction and a proof this \
             adapter cannot verify",
        ))
    }

    async fn estimate_gas_price(&self) -> AdapterResult<U256> {
        crate::evm_rpc::gas_price(&crate::evm_rpc::url(&self.config)).await
    }

    async fn get_transaction_receipt(
        &self,
        tx_hash: H256,
    ) -> AdapterResult<Option<TransactionReceipt>> {
        // Refused: this reported `success: true` for every transaction hash.
        crate::evm_rpc::receipt(&crate::evm_rpc::url(&self.config), tx_hash).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bnb_adapter() {
        let adapter = BnbAdapter::new(ChainConfig::for_chain(ChainType::Bnb));
        assert_eq!(adapter.chain_type(), ChainType::Bnb);
        assert_eq!(adapter.config().chain_type, 56);
    }

    #[test]
    fn test_encode_approve() {
        let calldata = BnbAdapter::encode_approve(H160::zero(), U256::MAX);
        assert_eq!(&calldata[0..4], &[0x09, 0x5e, 0xa7, 0xb3]);
        assert_eq!(calldata.len(), 68);
    }

    #[test]
    fn test_well_known_addresses() {
        assert_ne!(BnbAdapter::WBNB, H160::zero());
        assert_ne!(BnbAdapter::PANCAKE_ROUTER, H160::zero());
    }
}
