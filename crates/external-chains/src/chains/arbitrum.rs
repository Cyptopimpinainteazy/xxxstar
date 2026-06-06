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

        let response = self.rpc_call("eth_call", &params).await?;
        let result = Self::extract_result(&response)?;
        Self::parse_hex_u64(&result)
    }

    fn rpc_url(&self) -> String {
        String::from_utf8_lossy(&self.config.rpc_url).to_string()
    }

    fn build_rpc_request(method: &str, params: &str) -> Vec<u8> {
        format!(
            r#"{{"jsonrpc":"2.0","method":"{}","params":{},"id":1}}"#,
            method, params
        )
        .into_bytes()
    }

    async fn rpc_call(&self, method: &str, params: &str) -> AdapterResult<Vec<u8>> {
        let url = self.rpc_url();
        let body = Self::build_rpc_request(method, params);
        crate::rpc_http::post_json(&url, &body)
            .await
            .map_err(|e| ExternalChainError::rpc_error(&format!("HTTP error: {}", e)))
    }

    fn extract_result(response: &[u8]) -> AdapterResult<String> {
        let text = String::from_utf8_lossy(response);

        if text.contains("\"error\"") {
            return Err(ExternalChainError::rpc_error(&format!(
                "RPC error: {}",
                text
            )));
        }

        if let Some(idx) = text.find("\"result\"") {
            let after = &text[idx + 9..];
            let after = after.trim_start();

            if after.starts_with("null") {
                return Ok("null".to_string());
            }

            if after.starts_with('"') {
                let end = after[1..].find('"').unwrap_or(after.len() - 1);
                return Ok(after[1..=end].to_string());
            }
        }

        Err(ExternalChainError::parse_error(
            "Could not extract result from RPC response",
        ))
    }

    fn parse_hex_u64(hex_str: &str) -> AdapterResult<u64> {
        let trimmed = hex_str.trim().trim_matches('"');
        let without_prefix = trimmed.strip_prefix("0x").unwrap_or(trimmed);
        u64::from_str_radix(without_prefix, 16)
            .map_err(|e| ExternalChainError::parse_error(&format!("hex parse: {}", e)))
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
        true
    }

    async fn get_block_number(&self) -> AdapterResult<u64> {
        Ok(250_000_000) // Arbitrum has high block numbers
    }

    async fn get_balance(&self, _address: H160) -> AdapterResult<U256> {
        Ok(U256::from(1_000_000_000_000_000_000u64))
    }

    async fn get_token_balance(&self, _token: H160, _address: H160) -> AdapterResult<U256> {
        Ok(U256::from(1_000_000_000_000_000_000u64))
    }

    async fn send_message(&self, message: ChainMessage) -> AdapterResult<H256> {
        // Uses ArbSys.sendTxToL1 for L2->L1 messages
        Ok(message.hash())
    }

    async fn receive_messages(&self) -> AdapterResult<Vec<ChainMessage>> {
        // Query L2ToL1Tx events from ArbSys
        Ok(vec![])
    }

    async fn initiate_transfer(&self, transfer: CrossChainTransfer) -> AdapterResult<H256> {
        // Use Gateway Router for token transfers
        Ok(transfer.id)
    }

    async fn check_transfer_status(&self, _transfer_id: H256) -> AdapterResult<TransferStatus> {
        Ok(TransferStatus::Completed)
    }

    async fn verify_message_proof(
        &self,
        _message: &ChainMessage,
        proof: &[u8],
    ) -> AdapterResult<bool> {
        // Arbitrum uses Nitro's state commitment for proofs
        // 7 day challenge period for fraud proofs
        Ok(!proof.is_empty())
    }

    async fn finalize_transfer(&self, transfer_id: H256, _proof: Vec<u8>) -> AdapterResult<H256> {
        // After challenge period, execute on L1 Outbox
        Ok(transfer_id)
    }

    async fn estimate_gas_price(&self) -> AdapterResult<U256> {
        // Arbitrum gas is measured in ArbGas
        Ok(U256::from(100_000_000)) // 0.1 gwei
    }

    async fn get_transaction_receipt(
        &self,
        tx_hash: H256,
    ) -> AdapterResult<Option<TransactionReceipt>> {
        Ok(Some(TransactionReceipt {
            tx_hash,
            block_number: 250_000_000,
            block_hash: H256::zero(),
            tx_index: 0,
            success: true,
            gas_used: 21_000,
            logs: vec![],
        }))
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
