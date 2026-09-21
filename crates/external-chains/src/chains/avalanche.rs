//! Avalanche C-Chain Adapter
//!
//! Adapter for Avalanche C-Chain (EVM compatible)
//! Chain ID: 43114

use crate::adapter::*;
use crate::error::ExternalChainError;
use crate::ChainType;
use sp_core::{H160, H256, U256};
use sp_std::vec::Vec;

/// Avalanche C-Chain adapter
pub struct AvalancheAdapter {
    config: ChainConfig,
    #[allow(dead_code)]
    nonce: u64,
}

impl AvalancheAdapter {
    /// Create new Avalanche adapter
    pub fn new(config: ChainConfig) -> Self {
        Self { config, nonce: 0 }
    }

    /// Avalanche Bridge contract
    pub const BRIDGE_CONTRACT: H160 = H160(hex_literal::hex!(
        "8EB8a3b98659Cce290402893d0123abb75E3ab28"
    ));

    /// WAVAX token (wrapped AVAX)
    pub const WAVAX: H160 = H160(hex_literal::hex!(
        "B31f66AA3C1e785363F0875A1B74E27b85FD66c7"
    ));

    /// Teleporter Messenger (Avalanche's native cross-chain)
    pub const TELEPORTER_MESSENGER: H160 = H160(hex_literal::hex!(
        "253b2784c75e510dD0fF1da844684a1aC0aa5fcf"
    ));

    /// Encode Teleporter message send
    pub fn encode_send_cross_chain_message(
        destination_chain_id: H256, // Avalanche uses 32-byte chain IDs
        destination_address: H160,
        message: Vec<u8>,
        required_gas_limit: U256,
    ) -> Vec<u8> {
        // sendCrossChainMessage(TeleporterMessageInput)
        let mut calldata = Vec::with_capacity(4 + 32 * 6 + message.len());

        // Function selector (simplified)
        calldata.extend_from_slice(&[0x62, 0xe0, 0xa3, 0xf1]);

        // Destination chain ID
        calldata.extend_from_slice(destination_chain_id.as_bytes());

        // Destination address
        calldata.extend_from_slice(&[0u8; 12]);
        calldata.extend_from_slice(destination_address.as_bytes());

        // Fee info (simplified - using native AVAX)
        calldata.extend_from_slice(&[0u8; 32]); // feeTokenAddress = 0
        calldata.extend_from_slice(&[0u8; 32]); // feeAmount = 0

        // Required gas limit
        let gas_bytes = required_gas_limit.to_big_endian();
        calldata.extend_from_slice(&gas_bytes);

        // Message offset
        calldata.extend_from_slice(&[0u8; 31]);
        calldata.push(0xc0);

        // Message length
        let msg_len = message.len() as u32;
        calldata.extend_from_slice(&[0u8; 28]);
        calldata.extend_from_slice(&msg_len.to_be_bytes());

        // Message data
        calldata.extend_from_slice(&message);
        let padding = (32 - message.len() % 32) % 32;
        calldata.extend_from_slice(&vec![0u8; padding]);

        calldata
    }

    /// Avalanche-specific: Get P-Chain block height
    #[allow(dead_code)]
    async fn get_p_chain_height(&self) -> AdapterResult<u64> {
        // Refused: this returned a constant 50_000_000. The P-Chain speaks a
        // different RPC (`platform.getHeight`) from the C-Chain's JSON-RPC, and
        // this adapter only knows the latter.
        Err(ExternalChainError::adapter_unimplemented(
            "avalanche: the P-Chain uses platform.* RPC, not Ethereum JSON-RPC; refusing rather \
             than returning a constant height",
        ))
    }

    /// Check if subnet is validated
    #[allow(dead_code)]
    async fn is_subnet_validated(&self, _subnet_id: H256) -> AdapterResult<bool> {
        // Refused: this answered `true` for every subnet id, including ones
        // that do not exist. Subnet validation is P-Chain state.
        Err(ExternalChainError::adapter_unimplemented(
            "avalanche: subnet validation is P-Chain state this adapter cannot query",
        ))
    }
}

#[async_trait::async_trait]
impl ChainAdapter for AvalancheAdapter {
    fn chain_type(&self) -> ChainType {
        ChainType::Avalanche
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
        // that was never broadcast through Teleporter.
        Err(ExternalChainError::adapter_unimplemented(
            "avalanche: send_message needs a signed Teleporter transaction and this adapter has \
             no signer",
        ))
    }

    async fn receive_messages(&self) -> AdapterResult<Vec<ChainMessage>> {
        // Refused, and not because the event is missing: Avalanche's Teleporter
        // does emit `TeleporterMessageReceived` here. It does not fit
        // `ChainMessage`, and each mismatch would have to be resolved by
        // inventing data:
        //
        //   TeleporterMessageReceived(bytes32 indexed teleporterMessageID,
        //       address sender, address[] destinationAddresses,
        //       address[] feeTokenAddresses, uint256[] feeAmounts, bytes message)
        //
        // * `teleporterMessageID` is 32 bytes; `ChainMessage::nonce` is a u64, and
        //   truncating an identifier is the kind of quiet loss this crate refuses
        // * `destinationAddresses` is a list — a message with three recipients
        //   cannot be represented by one `recipient` field
        // * fees are per-token arrays, so `value` has no single meaning
        // * the event states no gas limit and no timestamp
        //
        // What this needs is a message type that carries a 32-byte id, a recipient
        // list and per-token fees; until something consumes such messages (there is
        // no reader of `ChainMessage` outside the adapters), adding those fields
        // would be designing blind.
        Err(ExternalChainError::adapter_unimplemented(
            "avalanche: TeleporterMessageReceived carries a 32-byte message id, a recipient list \
             and per-token fees — none of which ChainMessage can hold without losing data \
             (nonce is u64, recipient is one address, value is one amount)",
        ))
    }

    async fn initiate_transfer(&self, _transfer: CrossChainTransfer) -> AdapterResult<H256> {
        // Refused: this returned `transfer.id` for a transfer never sent.
        Err(ExternalChainError::adapter_unimplemented(
            "avalanche: initiate_transfer needs a signed Teleporter/Bridge transaction and this \
             adapter has no signer",
        ))
    }

    async fn check_transfer_status(&self, _transfer_id: H256) -> AdapterResult<TransferStatus> {
        // Refused: this answered `Completed` for every transfer id.
        Err(ExternalChainError::adapter_unimplemented(
            "avalanche: check_transfer_status needs the destination Teleporter state; nothing \
             here can tell a relayed message from an unrelayed one",
        ))
    }

    async fn verify_message_proof(
        &self,
        _message: &ChainMessage,
        _proof: &[u8],
    ) -> AdapterResult<bool> {
        // Refused, not shape-checked: Teleporter proofs are validator
        // signature aggregations, and `!proof.is_empty()` is not that.
        Err(ExternalChainError::VerificationUnavailable)
    }

    async fn finalize_transfer(&self, _transfer_id: H256, _proof: Vec<u8>) -> AdapterResult<H256> {
        // Refused: this returned the transfer id as though finalization had
        // happened.
        Err(ExternalChainError::adapter_unimplemented(
            "avalanche: finalize_transfer needs a signed delivery transaction and a proof this \
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
    fn test_avalanche_adapter() {
        let adapter = AvalancheAdapter::new(ChainConfig::for_chain(ChainType::Avalanche));
        assert_eq!(adapter.chain_type(), ChainType::Avalanche);
        assert_eq!(adapter.config().chain_type, 43114);
    }

    #[test]
    fn test_well_known_addresses() {
        assert_ne!(AvalancheAdapter::WAVAX, H160::zero());
        assert_ne!(AvalancheAdapter::TELEPORTER_MESSENGER, H160::zero());
    }
}
