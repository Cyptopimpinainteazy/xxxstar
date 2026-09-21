//! Polygon Chain Adapter
//!
//! Adapter for Polygon PoS (formerly Matic)
//! Chain ID: 137

use crate::adapter::*;
use crate::error::ExternalChainError;
use crate::ChainType;
use sp_core::{H160, H256, U256};
use sp_std::vec::Vec;

/// Polygon chain adapter
pub struct PolygonAdapter {
    config: ChainConfig,
    #[allow(dead_code)]
    nonce: u64,
}

impl PolygonAdapter {
    /// Create new Polygon adapter
    pub fn new(config: ChainConfig) -> Self {
        Self { config, nonce: 0 }
    }

    /// Polygon PoS Bridge (Root Chain Manager)
    pub const ROOT_CHAIN_MANAGER: H160 = H160(hex_literal::hex!(
        "A0c68C638235ee32657e8f720a23ceC1bFc77C77"
    ));

    /// Polygon PoS Bridge (Child Chain Manager Proxy)
    pub const CHILD_CHAIN_MANAGER: H160 = H160(hex_literal::hex!(
        "A6FA4fB5f76172d178d61B04b0ecd319C5d1C0aa"
    ));

    /// MATIC token on Ethereum (for reference)
    pub const MATIC_ON_ETH: H160 = H160(hex_literal::hex!(
        "7D1AfA7B718fb893dB30A3aBc0Cfc608AaCfeBB0"
    ));

    /// POL token (native token on Polygon)
    pub const POL_TOKEN: H160 = H160(hex_literal::hex!(
        "0000000000000000000000000000000000001010"
    ));

    /// Encode deposit for POL (native transfer via PoS bridge)
    pub fn encode_deposit_for(user: H160, token: H160, deposit_data: Vec<u8>) -> Vec<u8> {
        // depositFor(address user, address rootToken, bytes depositData)
        let mut calldata = Vec::with_capacity(4 + 32 * 3 + deposit_data.len());

        // Function selector: depositFor(address,address,bytes)
        calldata.extend_from_slice(&[0xe8, 0x27, 0x42, 0x12]);

        // User address
        calldata.extend_from_slice(&[0u8; 12]);
        calldata.extend_from_slice(user.as_bytes());

        // Token address
        calldata.extend_from_slice(&[0u8; 12]);
        calldata.extend_from_slice(token.as_bytes());

        // Deposit data offset
        calldata.extend_from_slice(&[0u8; 31]);
        calldata.push(0x60);

        // Deposit data length
        let data_len = deposit_data.len() as u32;
        calldata.extend_from_slice(&[0u8; 28]);
        calldata.extend_from_slice(&data_len.to_be_bytes());

        // Deposit data
        calldata.extend_from_slice(&deposit_data);
        let padding = (32 - deposit_data.len() % 32) % 32;
        calldata.extend_from_slice(&vec![0u8; padding]);

        calldata
    }

    /// Encode withdraw (burn on Polygon, exit on Ethereum)
    pub fn encode_withdraw(amount: U256) -> Vec<u8> {
        // withdraw(uint256 amount) for native POL
        let mut calldata = Vec::with_capacity(4 + 32);

        // Function selector: withdraw(uint256)
        calldata.extend_from_slice(&[0x2e, 0x1a, 0x7d, 0x4d]);

        // Amount
        let amount_bytes = amount.to_big_endian();
        calldata.extend_from_slice(&amount_bytes);

        calldata
    }

    /// Get checkpoint data for exit proofs
    #[allow(dead_code)]
    async fn get_checkpoint(&self, _block_number: u64) -> AdapterResult<H256> {
        // Refused: this returned the zero hash for every block, which reads as
        // "this block has a checkpoint and it is zero". Polygon checkpoints are
        // submitted to Ethereum, and this adapter cannot read Ethereum state.
        Err(ExternalChainError::adapter_unimplemented(
            "polygon: checkpoints live on Ethereum's RootChain contract; this adapter cannot \
             read them and refuses rather than returning the zero hash",
        ))
    }
}

#[async_trait::async_trait]
impl ChainAdapter for PolygonAdapter {
    fn chain_type(&self) -> ChainType {
        ChainType::Polygon
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
        // that was never broadcast through StateSender.
        Err(ExternalChainError::adapter_unimplemented(
            "polygon: send_message needs a signed StateSender transaction and this adapter has \
             no signer",
        ))
    }

    async fn receive_messages(&self) -> AdapterResult<Vec<ChainMessage>> {
        // Refused, and the reason is not "not implemented yet": `StateSync` is
        // emitted by the StateSender contract on **Ethereum**, not on Polygon.
        // This adapter talks to a Polygon endpoint, where the event does not
        // exist, so a query here can only ever answer an empty queue — the lie
        // this method used to tell. Observing a state sync needs an Ethereum-side
        // watcher whose messages *target* Polygon (`source_chain` 1,
        // `dest_chain` 137); that is a different adapter from this one.
        Err(ExternalChainError::adapter_unimplemented(
            "polygon: StateSync is emitted on Ethereum by the StateSender, not on Polygon — \
             this adapter reads Polygon, where the event does not exist. A Polygon-bound message \
             needs an Ethereum-side watcher, not a decoder here",
        ))
    }

    async fn initiate_transfer(&self, _transfer: CrossChainTransfer) -> AdapterResult<H256> {
        // Refused: this returned `transfer.id` for a deposit/burn never sent.
        Err(ExternalChainError::adapter_unimplemented(
            "polygon: initiate_transfer needs a signed RootChainManager/ChildToken transaction \
             and this adapter has no signer",
        ))
    }

    async fn check_transfer_status(&self, _transfer_id: H256) -> AdapterResult<TransferStatus> {
        // Refused: this answered `Completed` for every transfer id.
        Err(ExternalChainError::adapter_unimplemented(
            "polygon: check_transfer_status needs checkpoint inclusion, which this adapter cannot \
             read; refusing rather than reporting every transfer as complete",
        ))
    }

    async fn verify_message_proof(
        &self,
        _message: &ChainMessage,
        _proof: &[u8],
    ) -> AdapterResult<bool> {
        // Refused, not shape-checked: a Polygon exit proof is a Merkle proof
        // against a checkpointed root, and `!proof.is_empty()` is not that.
        Err(ExternalChainError::VerificationUnavailable)
    }

    async fn finalize_transfer(&self, _transfer_id: H256, _proof: Vec<u8>) -> AdapterResult<H256> {
        // Refused: this returned the transfer id as though `exit()` had run.
        Err(ExternalChainError::adapter_unimplemented(
            "polygon: finalize_transfer needs a signed RootChainManager exit and a proof this \
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
    fn test_polygon_adapter() {
        let adapter = PolygonAdapter::new(ChainConfig::for_chain(ChainType::Polygon));
        assert_eq!(adapter.chain_type(), ChainType::Polygon);
        assert_eq!(adapter.config().chain_type, 137);
    }

    #[test]
    fn test_encode_withdraw() {
        let calldata = PolygonAdapter::encode_withdraw(U256::from(1_000_000_000_000_000_000u64));
        assert_eq!(&calldata[0..4], &[0x2e, 0x1a, 0x7d, 0x4d]);
        assert_eq!(calldata.len(), 36);
    }
}
