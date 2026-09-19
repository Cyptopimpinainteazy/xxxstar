//! Arbitrum Chain Adapter
//!
//! Adapter for Arbitrum One - Optimistic rollup with Nitro stack
//! Chain ID: 42161

use crate::adapter::*;
use crate::ChainType;
use crate::ExternalChainError;
use alloc::{
    format,
    string::{String, ToString},
};
use sp_core::{H160, H256, U256};
use sp_std::vec::Vec;

/// Arbitrum chain adapter
pub struct ArbitrumAdapter {
    config: ChainConfig,
    #[allow(dead_code)]
    nonce: u64,
}

impl ArbitrumAdapter {
    /// Create new Arbitrum adapter
    pub fn new(config: ChainConfig) -> Self {
        Self { config, nonce: 0 }
    }

    /// Arbitrum Inbox contract for L1->L2 messages
    pub const INBOX_ADDRESS: H160 = H160(hex_literal::hex!(
        "4Dbd4fc535Ac27206064B68FfCf827b0A60BAB3f"
    ));

    /// Arbitrum Gateway Router for token bridging
    pub const GATEWAY_ROUTER: H160 = H160(hex_literal::hex!(
        "72Ce9c846789fdB6fC1f34aC4AD25Dd9ef7031ef"
    ));

    /// ArbSys precompile address
    pub const ARBSYS_ADDRESS: H160 = H160(hex_literal::hex!(
        "0000000000000000000000000000000000000064"
    ));

    /// Encode outboundTransfer call for token bridging
    pub fn encode_outbound_transfer(token: H160, to: H160, amount: U256, data: Vec<u8>) -> Vec<u8> {
        // outboundTransfer(address _token, address _to, uint256 _amount, bytes _data)
        let mut calldata = Vec::with_capacity(4 + 32 * 4 + data.len());

        // Function selector: outboundTransfer(address,address,uint256,bytes)
        calldata.extend_from_slice(&[0xd2, 0xce, 0x7d, 0x65]);

        // Token address (padded to 32 bytes)
        calldata.extend_from_slice(&[0u8; 12]);
        calldata.extend_from_slice(token.as_bytes());

        // To address (padded to 32 bytes)
        calldata.extend_from_slice(&[0u8; 12]);
        calldata.extend_from_slice(to.as_bytes());

        // Amount (32 bytes)
        let amount_bytes = amount.to_big_endian();
        calldata.extend_from_slice(&amount_bytes);

        // Data offset
        calldata.extend_from_slice(&[0u8; 31]);
        calldata.push(0x80);

        // Data length
        let data_len = data.len() as u32;
        calldata.extend_from_slice(&[0u8; 28]);
        calldata.extend_from_slice(&data_len.to_be_bytes());

        // Data (padded)
        calldata.extend_from_slice(&data);
        let padding = (32 - data.len() % 32) % 32;
        calldata.extend_from_slice(&vec![0u8; padding]);

        calldata
    }

    /// Encode sendL2Message for cross-chain messaging
    pub fn encode_send_l2_message(target: H160, calldata: Vec<u8>) -> Vec<u8> {
        // sendL2Message(address _target, bytes _data)
        let mut encoded = Vec::with_capacity(4 + 32 + 32 + 32 + calldata.len() + 31);

        // Function selector
        encoded.extend_from_slice(&[0x67, 0x9a, 0xef, 0xce]);

        // Target address
        encoded.extend_from_slice(&[0u8; 12]);
        encoded.extend_from_slice(target.as_bytes());

        // Dynamic bytes offset = 0x40 (after two static args)
        encoded.extend_from_slice(&[0u8; 31]);
        encoded.push(0x40);

        // Dynamic bytes section: length + data + padding
        let mut len_word = [0u8; 32];
        let len = calldata.len() as u32;
        len_word[28..32].copy_from_slice(&len.to_be_bytes());
        encoded.extend_from_slice(&len_word);
        encoded.extend_from_slice(&calldata);
        let padding = (32 - calldata.len() % 32) % 32;
        encoded.extend_from_slice(&vec![0u8; padding]);

        encoded
    }

    /// Arbitrum-specific: Get L1 block info from ArbSys
    #[allow(dead_code)]
    async fn get_l1_block_number(&self) -> AdapterResult<u64> {
        // ArbSys.arbBlockNumber() selector = first 4 bytes of keccak256("arbBlockNumber()")
        let selector = sp_io::hashing::keccak_256(b"arbBlockNumber()");
        let call_data = &selector[0..4];

        let params = format!(
            r#"[{{"to":"0x{}","data":"0x{}"}},"latest"]"#,
            hex::encode(Self::ARBSYS_ADDRESS.as_bytes()),
            hex::encode(call_data)
        );

        let response =
            crate::evm_rpc::call(&crate::evm_rpc::url(&self.config), "eth_call", &params).await?;
        let result = crate::evm_rpc::extract_result(&response)?;
        crate::evm_rpc::parse_hex_u64(&result)
    }
}

#[async_trait::async_trait]
impl ChainAdapter for ArbitrumAdapter {
    fn chain_type(&self) -> ChainType {
        ChainType::Arbitrum
    }

    fn config(&self) -> &ChainConfig {
        &self.config
    }

    async fn is_connected(&self) -> bool {
        // Real `eth_chainId` probe: the endpoint must answer *as Arbitrum*.
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
        // Refused, not invented. This returned `message.hash()` for a message
        // that was never broadcast over ArbSys.sendTxToL1.
        Err(ExternalChainError::adapter_unimplemented(
            "arbitrum: send_message needs a signed ArbSys.sendTxToL1 transaction and this \
             adapter has no signer",
        ))
    }

    async fn receive_messages(&self) -> AdapterResult<Vec<ChainMessage>> {
        // Refused, not answered with an empty list: an empty list reads as
        // "no pending L2ToL1Tx events" no matter what the chain said.
        Err(ExternalChainError::adapter_unimplemented(
            "arbitrum: receive_messages cannot decode L2ToL1Tx events yet; refusing rather than \
             reporting an empty message queue",
        ))
    }

    async fn initiate_transfer(&self, _transfer: CrossChainTransfer) -> AdapterResult<H256> {
        // Refused: this returned `transfer.id` for a Gateway Router transfer
        // that was never sent.
        Err(ExternalChainError::adapter_unimplemented(
            "arbitrum: initiate_transfer needs a signed Gateway Router transaction and this \
             adapter has no signer",
        ))
    }

    async fn check_transfer_status(&self, _transfer_id: H256) -> AdapterResult<TransferStatus> {
        // Refused: this answered `Completed` for every transfer id, including
        // ids that do not exist. A relayer acting on that releases funds for a
        // message that never arrived.
        Err(ExternalChainError::adapter_unimplemented(
            "arbitrum: check_transfer_status needs the destination Outbox state; nothing here \
             can tell a relayed message from an unrelayed one",
        ))
    }

    async fn verify_message_proof(
        &self,
        _message: &ChainMessage,
        _proof: &[u8],
    ) -> AdapterResult<bool> {
        // Refused, not shape-checked. Arbitrum proofs walk to Nitro's state
        // commitment through a 7-day challenge period; `!proof.is_empty()` is
        // not that, and returning `true` for it made any byte string a proof.
        Err(ExternalChainError::VerificationUnavailable)
    }

    async fn finalize_transfer(&self, _transfer_id: H256, _proof: Vec<u8>) -> AdapterResult<H256> {
        // Refused: this returned the transfer id as though the message had been
        // executed on the L1 Outbox.
        Err(ExternalChainError::adapter_unimplemented(
            "arbitrum: finalize_transfer needs a signed L1 Outbox execution and a proof this \
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
        // Refused: this reported `success: true` for *every* transaction hash,
        // including hashes that were never mined.
        crate::evm_rpc::receipt(&crate::evm_rpc::url(&self.config), tx_hash).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arbitrum_adapter() {
        let adapter = ArbitrumAdapter::new(ChainConfig::for_chain(ChainType::Arbitrum));
        assert_eq!(adapter.chain_type(), ChainType::Arbitrum);
        assert_eq!(adapter.config().chain_type, 42161);
    }

    #[test]
    fn test_constants() {
        // Verify well-known addresses
        assert_ne!(ArbitrumAdapter::INBOX_ADDRESS, H160::zero());
        assert_ne!(ArbitrumAdapter::GATEWAY_ROUTER, H160::zero());
    }

    #[test]
    fn test_encode_send_l2_message_dynamic_bytes_layout() {
        let target = H160::from_low_u64_be(0xBEEF);
        let payload = vec![1u8, 2, 3, 4, 5];
        let encoded = ArbitrumAdapter::encode_send_l2_message(target, payload.clone());

        // selector + address + offset + len + payload...
        assert!(encoded.len() >= 4 + 32 + 32 + 32 + payload.len());
        // offset word should end in 0x40
        assert_eq!(encoded[4 + 32 + 31], 0x40);
        // dynamic length word should match payload length
        assert_eq!(encoded[4 + 64 + 31], payload.len() as u8);
    }
}
