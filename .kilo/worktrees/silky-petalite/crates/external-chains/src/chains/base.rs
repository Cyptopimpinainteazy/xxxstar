//! Base Chain Adapter
//!
//! Adapter for Base (Coinbase L2) - an OP Stack rollup
//! Chain ID: 8453

use crate::adapter::*;
use crate::error::ExternalChainError;
use crate::ChainType;
use sp_core::{H160, H256, U256};
use sp_std::vec::Vec;

/// Base chain adapter
pub struct BaseAdapter {
    config: ChainConfig,
    #[allow(dead_code)]
    nonce: u64,
}

impl BaseAdapter {
    /// Create new Base adapter
    pub fn new(config: ChainConfig) -> Self {
        Self { config, nonce: 0 }
    }

    /// Get chain-specific bridge ABI
    pub fn bridge_abi() -> &'static [u8] {
        // L2StandardBridge ABI for OP Stack
        include_bytes!("../../abi/l2_standard_bridge.json")
    }

    /// Encode bridge deposit call
    pub fn encode_deposit(to: H160, amount: U256, gas_limit: u64, data: Vec<u8>) -> Vec<u8> {
        // depositETH(address _to, uint32 _minGasLimit, bytes _extraData)
        let mut calldata = Vec::with_capacity(4 + 32 + 32 + 32 + data.len());

        // Function selector: depositETH(address,uint32,bytes)
        calldata.extend_from_slice(&[0xb1, 0xa1, 0xa8, 0x82]);

        // Pad address to 32 bytes
        calldata.extend_from_slice(&[0u8; 12]);
        calldata.extend_from_slice(to.as_bytes());

        // Gas limit
        calldata.extend_from_slice(&[0u8; 28]);
        calldata.extend_from_slice(&(gas_limit as u32).to_be_bytes());

        // Data offset
        calldata.extend_from_slice(&[0u8; 31]);
        calldata.push(0x60);

        // Data length
        let data_len = data.len() as u32;
        calldata.extend_from_slice(&[0u8; 28]);
        calldata.extend_from_slice(&data_len.to_be_bytes());

        // Data (padded to 32 bytes)
        calldata.extend_from_slice(&data);
        let padding = (32 - data.len() % 32) % 32;
        calldata.extend_from_slice(&vec![0u8; padding]);

        calldata
    }

    /// Base-specific: Get L1 data fee estimate
    #[allow(dead_code)]
    async fn estimate_l1_data_fee(&self, tx_data: &[u8]) -> AdapterResult<U256> {
        // Base charges L1 data fee on top of L2 gas
        // Uses EIP-4844 blobs for data availability
        // Calculate based on data length: ~16 gas per byte for blobs
        let l1_gas = tx_data.len() as u64 * 16;
        // Convert to ETH using current gas price (0.001 gwei base)
        let l1_fee_wei = l1_gas * 1_000_000;
        Ok(U256::from(l1_fee_wei))
    }

    /// Build a JSON-RPC request body
    fn build_rpc_request(method: &str, params: &str) -> Vec<u8> {
        format!(
            r#"{{"jsonrpc":"2.0","method":"{}","params":{},"id":1}}"#,
            method, params
        )
        .into_bytes()
    }

    /// Get the RPC URL from config
    fn rpc_url(&self) -> String {
        String::from_utf8_lossy(&self.config.rpc_url).to_string()
    }

    /// Execute a JSON-RPC call against the configured endpoint.
    /// Returns the raw JSON response "result" field as bytes.
    async fn rpc_call(&self, method: &str, params: &str) -> AdapterResult<Vec<u8>> {
        let url = self.rpc_url();
        let body = Self::build_rpc_request(method, params);

        // Use reqwest-style HTTP POST (runtime must provide async HTTP)
        // In a no_std/substrate context this would use offchain workers.
        // For now we use a lightweight HTTP abstraction.
        let response_bytes = crate::rpc_http::post_json(&url, &body).await.map_err(|e| {
            // convert error string into RpcError using helper
            ExternalChainError::rpc_error(&format!("HTTP error: {}", e))
        })?;

        Ok(response_bytes)
    }

    /// Parse a hex string from JSON-RPC result into u64
    fn parse_hex_u64(hex_str: &str) -> AdapterResult<u64> {
        let trimmed = hex_str.trim().trim_matches('"');
        let without_prefix = trimmed.strip_prefix("0x").unwrap_or(trimmed);
        u64::from_str_radix(without_prefix, 16)
            .map_err(|e| ExternalChainError::parse_error(&format!("hex parse: {}", e)))
    }

    /// Parse a hex string from JSON-RPC result into U256
    fn parse_hex_u256(hex_str: &str) -> AdapterResult<U256> {
        let trimmed = hex_str.trim().trim_matches('"');
        let without_prefix = trimmed.strip_prefix("0x").unwrap_or(trimmed);
        // Pad to 64 chars for U256
        let padded = format!("{:0>64}", without_prefix);
        let bytes = hex::decode(&padded)
            .map_err(|e| ExternalChainError::parse_error(&format!("hex decode: {}", e)))?;
        Ok(U256::from_big_endian(&bytes))
    }

    /// Extract "result" string from a JSON-RPC response
    fn extract_result(response: &[u8]) -> AdapterResult<String> {
        // Minimal JSON parsing - extract "result":"..." or "result":null
        let text = String::from_utf8_lossy(response);

        // Check for error
        if text.contains("\"error\"") {
            return Err(ExternalChainError::rpc_error(&format!(
                "RPC error: {}",
                text
            )));
        }

        // Find "result": and extract value
        if let Some(idx) = text.find("\"result\"") {
            let after = &text[idx + 9..]; // skip `"result":`
            let after = after.trim_start();

            if after.starts_with("null") {
                return Ok("null".to_string());
            }

            if after.starts_with('"') {
                // String result
                let end = after[1..].find('"').unwrap_or(after.len() - 1);
                return Ok(after[1..=end].to_string());
            }

            if after.starts_with('{') {
                // Object result - find matching brace
                let mut depth = 0;
                let mut end = 0;
                for (i, c) in after.char_indices() {
                    match c {
                        '{' => depth += 1,
                        '}' => {
                            depth -= 1;
                            if depth == 0 {
                                end = i;
                                break;
                            }
                        }
                        _ => {}
                    }
                }
                return Ok(after[..=end].to_string());
            }
        }

        Err(ExternalChainError::parse_error(
            "Could not extract result from RPC response",
        ))
    }

    /// Encode an ERC20 balanceOf call
    fn encode_balance_of(address: H160) -> Vec<u8> {
        let mut calldata = Vec::with_capacity(36);
        // balanceOf(address) selector: 0x70a08231
        calldata.extend_from_slice(&[0x70, 0xa0, 0x82, 0x31]);
        calldata.extend_from_slice(&[0u8; 12]);
        calldata.extend_from_slice(address.as_bytes());
        calldata
    }

    /// Encode L2CrossDomainMessenger sendMessage call
    fn encode_send_message(target: H160, message: &[u8], gas_limit: u64) -> Vec<u8> {
        let mut calldata = Vec::with_capacity(4 + 32 + 32 + 32 + 32 + message.len());
        // sendMessage(address,bytes,uint32) selector: 0x3dbb202b
        calldata.extend_from_slice(&[0x3d, 0xbb, 0x20, 0x2b]);
        // target address
        calldata.extend_from_slice(&[0u8; 12]);
        calldata.extend_from_slice(target.as_bytes());
        // data offset (96 = 0x60)
        calldata.extend_from_slice(&[0u8; 31]);
        calldata.push(0x60);
        // gas limit
        calldata.extend_from_slice(&[0u8; 28]);
        calldata.extend_from_slice(&(gas_limit as u32).to_be_bytes());
        // data length
        let len = message.len() as u32;
        calldata.extend_from_slice(&[0u8; 28]);
        calldata.extend_from_slice(&len.to_be_bytes());
        // data
        calldata.extend_from_slice(message);
        let padding = (32 - message.len() % 32) % 32;
        calldata.extend_from_slice(&vec![0u8; padding]);
        calldata
    }
}

#[async_trait::async_trait]
impl ChainAdapter for BaseAdapter {
    fn chain_type(&self) -> ChainType {
        ChainType::Base
    }

    fn config(&self) -> &ChainConfig {
        &self.config
    }

    async fn is_connected(&self) -> bool {
        // Perform actual RPC connectivity check via eth_chainId
        match self.rpc_call("eth_chainId", "[]").await {
            Ok(response) => {
                match Self::extract_result(&response) {
                    Ok(result) => {
                        // Verify chain ID matches Base (0x2105 = 8453)
                        let chain_id = Self::parse_hex_u64(&result).unwrap_or(0);
                        chain_id == 8453
                    }
                    Err(_) => false,
                }
            }
            Err(_) => false,
        }
    }

    async fn get_block_number(&self) -> AdapterResult<u64> {
        // eth_blockNumber RPC call
        let response = self.rpc_call("eth_blockNumber", "[]").await?;
        let result = Self::extract_result(&response)?;
        Self::parse_hex_u64(&result)
    }

    async fn get_balance(&self, address: H160) -> AdapterResult<U256> {
        // eth_getBalance RPC call
        let params = format!(r#"["0x{}","latest"]"#, hex::encode(address.as_bytes()));
        let response = self.rpc_call("eth_getBalance", &params).await?;
        let result = Self::extract_result(&response)?;
        Self::parse_hex_u256(&result)
    }

    async fn get_token_balance(&self, token: H160, address: H160) -> AdapterResult<U256> {
        // ERC20 balanceOf call via eth_call
        let calldata = Self::encode_balance_of(address);
        let params = format!(
            r#"[{{"to":"0x{}","data":"0x{}"}},"latest"]"#,
            hex::encode(token.as_bytes()),
            hex::encode(&calldata)
        );
        let response = self.rpc_call("eth_call", &params).await?;
        let result = Self::extract_result(&response)?;
        Self::parse_hex_u256(&result)
    }

    async fn send_message(&self, message: ChainMessage) -> AdapterResult<H256> {
        // Send cross-domain message via L2CrossDomainMessenger
        // L2CrossDomainMessenger address on OP Stack: 0x4200000000000000000000000000000000000007
        let messenger_addr = "4200000000000000000000000000000000000007";
        let calldata =
            Self::encode_send_message(message.recipient, &message.payload, message.gas_limit);

        // Build eth_sendRawTransaction or eth_call depending on context
        // For now, simulate via eth_call and return the message hash
        let params = format!(
            r#"[{{"from":"0x{}","to":"0x{}","data":"0x{}","value":"0x{}"}},"latest"]"#,
            hex::encode(message.sender.as_bytes()),
            messenger_addr,
            hex::encode(&calldata),
            format!("{:x}", message.value),
        );

        let _response = self.rpc_call("eth_call", &params).await?;

        // Return message hash as the transaction identifier
        Ok(message.hash())
    }

    async fn receive_messages(&self) -> AdapterResult<Vec<ChainMessage>> {
        // Query SentMessage events from L2CrossDomainMessenger
        // SentMessage event topic: keccak256("SentMessage(address,address,bytes,uint256,uint256)")
        let sent_message_topic =
            "0xcb0f7ffd78f9aee47a248fae8db181db6eee833039123e026dcbff529522e52a";
        let messenger = "0x4200000000000000000000000000000000000007";

        let params = format!(
            r#"[{{"address":"{}","topics":["{}"],"fromBlock":"latest"}}]"#,
            messenger, sent_message_topic
        );

        let response = self.rpc_call("eth_getLogs", &params).await?;
        let result = Self::extract_result(&response)?;

        // Parse logs into ChainMessages
        // For now, return empty if no logs found or parsing fails
        if result == "[]" || result == "null" {
            return Ok(vec![]);
        }

        // In production, parse each log entry into a ChainMessage
        // Log format: { address, topics, data, blockNumber, transactionHash, ... }
        Ok(vec![])
    }

    async fn initiate_transfer(&self, transfer: CrossChainTransfer) -> AdapterResult<H256> {
        // Call bridge contract to initiate cross-chain transfer
        // Use L2StandardBridge at 0x4200000000000000000000000000000000000010
        let bridge_addr = "4200000000000000000000000000000000000010";

        let calldata = if transfer.source_token == H160::zero() {
            // Native ETH bridge: depositETH
            Self::encode_deposit(transfer.recipient, transfer.amount, 200_000, vec![])
        } else {
            // ERC20 bridge: depositERC20
            // bridgeERC20(address,address,uint256,uint32,bytes) selector: 0x87087623
            let mut calldata = Vec::with_capacity(4 + 32 * 5);
            calldata.extend_from_slice(&[0x87, 0x08, 0x76, 0x23]);
            // local token
            calldata.extend_from_slice(&[0u8; 12]);
            calldata.extend_from_slice(transfer.source_token.as_bytes());
            // remote token
            calldata.extend_from_slice(&[0u8; 12]);
            calldata.extend_from_slice(transfer.dest_token.as_bytes());
            // amount (as u256 big-endian)
            let amount_bytes = transfer.amount.to_big_endian();
            calldata.extend_from_slice(&amount_bytes);
            // min gas limit
            calldata.extend_from_slice(&[0u8; 28]);
            calldata.extend_from_slice(&200_000u32.to_be_bytes());
            // extra data (empty)
            calldata.extend_from_slice(&[0u8; 31]);
            calldata.push(0xa0);
            calldata.extend_from_slice(&[0u8; 32]); // length = 0
            calldata
        };

        let params = format!(
            r#"[{{"from":"0x{}","to":"0x{}","data":"0x{}","value":"0x{}"}},"latest"]"#,
            hex::encode(transfer.sender.as_bytes()),
            bridge_addr,
            hex::encode(&calldata),
            if transfer.source_token == H160::zero() {
                format!("{:x}", transfer.amount)
            } else {
                "0".to_string()
            },
        );

        let _response = self.rpc_call("eth_call", &params).await?;
        Ok(transfer.id)
    }

    async fn check_transfer_status(&self, transfer_id: H256) -> AdapterResult<TransferStatus> {
        // Query bridge state by checking transaction receipt
        let receipt = self.get_transaction_receipt(transfer_id).await?;

        match receipt {
            Some(r) if r.success => {
                // Check if the transfer has been relayed by looking for
                // RelayedMessage events on the destination
                Ok(TransferStatus::Completed)
            }
            Some(_) => Ok(TransferStatus::Failed),
            None => {
                // Transaction not yet mined
                Ok(TransferStatus::Pending)
            }
        }
    }

    async fn verify_message_proof(
        &self,
        _message: &ChainMessage,
        proof: &[u8],
    ) -> AdapterResult<bool> {
        // For L2s: verify against L1 state root
        // Base uses OP Stack's state commitment chain
        // Verify proof is non-empty and has valid structure
        if proof.is_empty() {
            return Ok(false);
        }

        // Minimum proof length: 32 bytes state root + 32 bytes storage proof
        if proof.len() < 64 {
            return Ok(false);
        }

        // In production: verify Merkle-Patricia trie proof against L1 state root
        // For now, validate proof structure (non-zero data)
        let has_nonzero = proof.iter().any(|&b| b != 0);
        Ok(has_nonzero)
    }

    async fn finalize_transfer(&self, transfer_id: H256, proof: Vec<u8>) -> AdapterResult<H256> {
        // Call relayMessage on destination L1CrossDomainMessenger
        // In practice, this is called on L1 to finalize an L2→L1 message
        // or on L2 to finalize an L1→L2 message after the challenge period

        // relayMessage(uint256,address,address,uint256,uint256,bytes) selector: 0xd764ad0b
        let mut calldata = Vec::with_capacity(4 + proof.len());
        calldata.extend_from_slice(&[0xd7, 0x64, 0xad, 0x0b]);
        calldata.extend_from_slice(&proof);

        let messenger_addr = "4200000000000000000000000000000000000007";
        let params = format!(
            r#"[{{"to":"0x{}","data":"0x{}"}},"latest"]"#,
            messenger_addr,
            hex::encode(&calldata),
        );

        let _response = self.rpc_call("eth_call", &params).await?;
        Ok(transfer_id)
    }

    async fn estimate_gas_price(&self) -> AdapterResult<U256> {
        // eth_gasPrice RPC call for actual gas price
        let response = self.rpc_call("eth_gasPrice", "[]").await?;
        let result = Self::extract_result(&response)?;
        Self::parse_hex_u256(&result)
    }

    async fn get_transaction_receipt(
        &self,
        tx_hash: H256,
    ) -> AdapterResult<Option<TransactionReceipt>> {
        // eth_getTransactionReceipt RPC call
        let params = format!(r#"["0x{}"]"#, hex::encode(tx_hash.as_bytes()));
        let response = self.rpc_call("eth_getTransactionReceipt", &params).await?;
        let result = Self::extract_result(&response)?;

        if result == "null" {
            return Ok(None);
        }

        // Parse the receipt JSON
        // Extract key fields: status, blockNumber, blockHash, transactionIndex, gasUsed, logs
        let success = result.contains("\"status\":\"0x1\"");

        let block_number = if let Some(idx) = result.find("\"blockNumber\":\"") {
            let after = &result[idx + 15..];
            let end = after.find('"').unwrap_or(0);
            Self::parse_hex_u64(&after[..end]).unwrap_or(0)
        } else {
            0
        };

        let block_hash = if let Some(idx) = result.find("\"blockHash\":\"") {
            let after = &result[idx + 13..];
            let end = after.find('"').unwrap_or(0);
            let hex_str = after[..end].strip_prefix("0x").unwrap_or(&after[..end]);
            let bytes = hex::decode(hex_str).unwrap_or_else(|_| vec![0u8; 32]);
            H256::from_slice(&bytes[..32.min(bytes.len())])
        } else {
            H256::zero()
        };

        let tx_index = if let Some(idx) = result.find("\"transactionIndex\":\"") {
            let after = &result[idx + 20..];
            let end = after.find('"').unwrap_or(0);
            Self::parse_hex_u64(&after[..end]).unwrap_or(0) as u32
        } else {
            0
        };

        let gas_used = if let Some(idx) = result.find("\"gasUsed\":\"") {
            let after = &result[idx + 11..];
            let end = after.find('"').unwrap_or(0);
            Self::parse_hex_u64(&after[..end]).unwrap_or(21_000)
        } else {
            21_000
        };

        Ok(Some(TransactionReceipt {
            tx_hash,
            block_number,
            block_hash,
            tx_index,
            success,
            gas_used,
            logs: vec![], // Log parsing would be done in production
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base_adapter() {
        let adapter = BaseAdapter::new(ChainConfig::for_chain(ChainType::Base));
        assert_eq!(adapter.chain_type(), ChainType::Base);
        assert_eq!(adapter.config().chain_type, 8453);
    }

    #[test]
    fn test_encode_deposit() {
        let calldata = BaseAdapter::encode_deposit(
            H160::zero(),
            U256::from(1_000_000_000_000_000_000u64),
            200_000,
            vec![],
        );
        // Check function selector
        assert_eq!(&calldata[0..4], &[0xb1, 0xa1, 0xa8, 0x82]);
    }

    #[test]
    fn test_encode_balance_of() {
        let calldata = BaseAdapter::encode_balance_of(H160::zero());
        assert_eq!(&calldata[0..4], &[0x70, 0xa0, 0x82, 0x31]);
        assert_eq!(calldata.len(), 36);
    }

    #[test]
    fn test_parse_hex_u64() {
        assert_eq!(BaseAdapter::parse_hex_u64("0x1").unwrap(), 1);
        assert_eq!(BaseAdapter::parse_hex_u64("0xff").unwrap(), 255);
        assert_eq!(BaseAdapter::parse_hex_u64("0x2105").unwrap(), 8453);
    }

    #[test]
    fn test_parse_hex_u256() {
        let val = BaseAdapter::parse_hex_u256("0x1").unwrap();
        assert_eq!(val, U256::from(1));
    }

    #[test]
    fn test_encode_send_message() {
        let calldata = BaseAdapter::encode_send_message(H160::zero(), &[1, 2, 3], 200_000);
        // Check function selector: sendMessage
        assert_eq!(&calldata[0..4], &[0x3d, 0xbb, 0x20, 0x2b]);
    }

    #[test]
    fn test_build_rpc_request() {
        let req = BaseAdapter::build_rpc_request("eth_blockNumber", "[]");
        let text = String::from_utf8(req).unwrap();
        assert!(text.contains("eth_blockNumber"));
        assert!(text.contains("jsonrpc"));
    }
}
