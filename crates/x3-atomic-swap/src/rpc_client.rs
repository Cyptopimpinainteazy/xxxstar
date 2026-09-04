//! # Generic JSON-RPC Client
//!
//! A minimal JSON-RPC client with real HTTP transport via `ureq` when the `std`
//! feature is enabled. In `no_std` mode, all RPC methods return
//! [`SwapError::RpcError`] indicating that the `std` feature is required.

use crate::error::SwapError;
use alloc::string::String;
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// A JSON-RPC request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: u64,
    pub method: String,
    #[serde(default)]
    pub params: Vec<Value>,
}

/// A JSON-RPC response
#[derive(Debug, Clone, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: u64,
    pub result: Option<Value>,
    pub error: Option<JsonRpcError>,
}

/// JSON-RPC error detail
#[derive(Debug, Clone, Deserialize)]
pub struct JsonRpcError {
    pub code: i64,
    pub message: String,
}

/// RPC client configuration
#[derive(Debug, Clone)]
pub struct RpcClientConfig {
    pub rpc_url: String,
    pub timeout_secs: u64,
    pub max_retries: u32,
    pub retry_delay_ms: u64,
    pub chain_id: u64,
}

impl Default for RpcClientConfig {
    fn default() -> Self {
        Self {
            rpc_url: String::new(),
            timeout_secs: 30,
            max_retries: 3,
            retry_delay_ms: 500,
            chain_id: 1,
        }
    }
}

/// An HTTP response (minimal)
#[derive(Debug)]
pub struct HttpResponse {
    pub status: u16,
    pub body: String,
}

/// Generic JSON-RPC client for making real RPC calls to chain nodes.
///
/// # Feature flags
///
/// - **`std`** (default off): enables real HTTP transport via `ureq`.
/// - **no_std**: all methods return [`SwapError::RpcError`] — HTTP transport
///   unavailable.
#[derive(Debug, Clone)]
pub struct RpcClient {
    pub config: RpcClientConfig,
    pub request_id: u64,
}

impl RpcClient {
    /// Create a new RPC client with the given URL and chain ID.
    pub fn new(rpc_url: String, chain_id: u64) -> Self {
        Self {
            config: RpcClientConfig {
                rpc_url,
                chain_id,
                ..RpcClientConfig::default()
            },
            request_id: 1,
        }
    }

    /// Create a new RPC client from a full configuration.
    pub fn new_with_config(config: RpcClientConfig) -> Self {
        Self {
            request_id: 1,
            config,
        }
    }

    /// Send an HTTP POST request with a JSON body.
    ///
    /// In `no_std` mode this always returns an error instructing the caller to
    /// enable the `std` feature. When `std` is enabled it uses `ureq`.
    fn http_post(&self, body: &str) -> Result<HttpResponse, SwapError> {
        #[cfg(feature = "std")]
        {
            let response = ureq::post(&self.config.rpc_url)
                .set("Content-Type", "application/json")
                .timeout(std::time::Duration::from_secs(self.config.timeout_secs))
                .send_string(body)
                .map_err(|e| SwapError::RpcError(format!("HTTP request failed: {}", e)))?;

            let status = response.status();
            let body_str = response
                .into_string()
                .map_err(|e| SwapError::RpcError(format!("Failed to read response body: {}", e)))?;

            Ok(HttpResponse {
                status,
                body: body_str,
            })
        }

        #[cfg(not(feature = "std"))]
        {
            let _ = body;
            Err(SwapError::RpcError(
                "HTTP transport requires the 'std' feature. Enable std feature for real RPC calls."
                    .into(),
            ))
        }
    }

    /// Make a raw JSON-RPC call.
    pub fn call(&mut self, method: &str, params: Vec<Value>) -> Result<JsonRpcResponse, SwapError> {
        let request = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: self.request_id,
            method: method.into(),
            params,
        };
        self.request_id += 1;

        let body = serde_json::to_string(&request)
            .map_err(|e| SwapError::RpcError(format!("Failed to serialize request: {}", e)))?;

        let response = self.http_post(&body)?;

        if response.status != 200 {
            return Err(SwapError::RpcError(format!(
                "HTTP {}: {}",
                response.status, response.body
            )));
        }

        let rpc_resp: JsonRpcResponse = serde_json::from_str(&response.body)
            .map_err(|e| SwapError::RpcError(format!("Failed to parse response: {}", e)))?;

        if let Some(err) = &rpc_resp.error {
            return Err(SwapError::RpcError(format!(
                "JSON-RPC error {}: {}",
                err.code, err.message
            )));
        }

        Ok(rpc_resp)
    }

    /// Get the latest block number from the chain.
    pub fn get_block_number(&mut self) -> Result<u64, SwapError> {
        let resp = self.call("eth_blockNumber", Vec::new())?;
        if let Some(result) = resp.result {
            if let Some(hex_str) = result.as_str() {
                let stripped = hex_str.strip_prefix("0x").unwrap_or(hex_str);
                u64::from_str_radix(stripped, 16).map_err(|e| {
                    SwapError::RpcError(format!(
                        "failed to parse block number '{}': {}",
                        hex_str, e
                    ))
                })
            } else {
                Err(SwapError::RpcError(
                    "block number result is not a string".into(),
                ))
            }
        } else {
            Err(SwapError::RpcError(
                "no result in block number response".into(),
            ))
        }
    }

    /// Get the balance of an address (in wei for EVM chains).
    pub fn get_balance(&mut self, address: &str) -> Result<u128, SwapError> {
        let params = vec![
            Value::String(address.into()),
            Value::String("latest".into()),
        ];
        let resp = self.call("eth_getBalance", params)?;
        if let Some(result) = resp.result {
            if let Some(hex_str) = result.as_str() {
                let stripped = hex_str.strip_prefix("0x").unwrap_or(hex_str);
                u128::from_str_radix(stripped, 16).map_err(|e| {
                    SwapError::RpcError(format!("failed to parse balance '{}': {}", hex_str, e))
                })
            } else {
                Err(SwapError::RpcError("balance result is not a string".into()))
            }
        } else {
            Err(SwapError::RpcError("no result in balance response".into()))
        }
    }

    /// Get a transaction receipt.
    pub fn get_transaction_receipt(&mut self, tx_hash: &str) -> Result<Option<Value>, SwapError> {
        let params = vec![Value::String(tx_hash.into())];
        let resp = self.call("eth_getTransactionReceipt", params)?;
        if let Some(result) = resp.result {
            if result.is_null() {
                Ok(None)
            } else {
                Ok(Some(result))
            }
        } else {
            Ok(None)
        }
    }

    /// Get a block by number.
    pub fn get_block_by_number(
        &mut self,
        block_number: u64,
        full_tx: bool,
    ) -> Result<Value, SwapError> {
        let params = vec![
            Value::String(format!("0x{:x}", block_number)),
            Value::Bool(full_tx),
        ];
        let resp = self.call("eth_getBlockByNumber", params)?;
        if let Some(result) = resp.result {
            Ok(result)
        } else {
            Err(SwapError::RpcError("no result in block response".into()))
        }
    }

    /// Estimate gas for a transaction.
    pub fn estimate_gas(&mut self, from: &str, to: &str, data: &str) -> Result<u64, SwapError> {
        let tx_obj = json!({
            "from": from,
            "to": to,
            "data": data,
        });
        let params = vec![tx_obj];
        let resp = self.call("eth_estimateGas", params)?;
        if let Some(result) = resp.result {
            if let Some(hex_str) = result.as_str() {
                let stripped = hex_str.strip_prefix("0x").unwrap_or(hex_str);
                u64::from_str_radix(stripped, 16).map_err(|e| {
                    SwapError::RpcError(format!(
                        "failed to parse gas estimate '{}': {}",
                        hex_str, e
                    ))
                })
            } else {
                Err(SwapError::RpcError(
                    "gas estimate result is not a string".into(),
                ))
            }
        } else {
            Err(SwapError::RpcError(
                "no result in gas estimate response".into(),
            ))
        }
    }

    /// Get the current gas price (eth_gasPrice).
    pub fn gas_price(&mut self) -> Result<u128, SwapError> {
        let resp = self.call("eth_gasPrice", vec![])?;
        let result = resp
            .result
            .ok_or_else(|| SwapError::RpcError("No result".into()))?;
        let hex_str = result
            .as_str()
            .ok_or_else(|| SwapError::RpcError("gasPrice not a string".into()))?;
        u128::from_str_radix(hex_str.trim_start_matches("0x"), 16)
            .map_err(|e| SwapError::RpcError(format!("Invalid gas price hex: {}", e)))
    }

    /// Get transaction count (nonce) for an address (eth_getTransactionCount).
    pub fn get_transaction_count(&mut self, address: &str, block: &str) -> Result<u64, SwapError> {
        let resp = self.call(
            "eth_getTransactionCount",
            vec![serde_json::json!(address), serde_json::json!(block)],
        )?;
        let result = resp
            .result
            .ok_or_else(|| SwapError::RpcError("No result".into()))?;
        let hex_str = result
            .as_str()
            .ok_or_else(|| SwapError::RpcError("txCount not a string".into()))?;
        u64::from_str_radix(hex_str.trim_start_matches("0x"), 16)
            .map_err(|e| SwapError::RpcError(format!("Invalid nonce hex: {}", e)))
    }

    /// Make an eth_call (execute a call without sending a transaction).
    pub fn eth_call(
        &mut self,
        from: &str,
        to: &str,
        data: &str,
        block: &str,
    ) -> Result<String, SwapError> {
        let tx_obj = serde_json::json!({
            "from": from,
            "to": to,
            "data": data,
        });
        let resp = self.call("eth_call", vec![tx_obj, serde_json::json!(block)])?;
        let result = resp
            .result
            .ok_or_else(|| SwapError::RpcError("No result".into()))?;
        result
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| SwapError::RpcError("eth_call result not a string".into()))
    }

    /// Get chain ID (eth_chainId).
    pub fn chain_id(&mut self) -> Result<u64, SwapError> {
        let resp = self.call("eth_chainId", vec![])?;
        let result = resp
            .result
            .ok_or_else(|| SwapError::RpcError("No result".into()))?;
        let hex_str = result
            .as_str()
            .ok_or_else(|| SwapError::RpcError("chainId not a string".into()))?;
        u64::from_str_radix(hex_str.trim_start_matches("0x"), 16)
            .map_err(|e| SwapError::RpcError(format!("Invalid chainId hex: {}", e)))
    }

    /// Send a signed raw transaction (eth_sendRawTransaction).
    /// The `signed_tx_hex` must be the fully RLP-encoded, EIP-155 signed transaction as a hex string with 0x prefix.
    pub fn send_raw_transaction(&mut self, signed_tx_hex: &str) -> Result<String, SwapError> {
        let resp = self.call(
            "eth_sendRawTransaction",
            vec![serde_json::json!(signed_tx_hex)],
        )?;
        let result = resp
            .result
            .ok_or_else(|| SwapError::RpcError("No result".into()))?;
        result
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| SwapError::RpcError("sendRawTransaction result not a string".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rpc_client_create() {
        let client = RpcClient::new("https://sepolia.infura.io/v3/test".into(), 11155111);
        assert_eq!(client.config.rpc_url, "https://sepolia.infura.io/v3/test");
        assert_eq!(client.config.chain_id, 11155111);
        assert_eq!(client.request_id, 1);
        assert_eq!(client.config.timeout_secs, 30);
    }

    #[test]
    fn test_rpc_client_request_id_increments() {
        let mut client = RpcClient::new("https://test.com/rpc".into(), 1);
        let _ = client.call("eth_blockNumber", vec![]);
        // In no_std the call fails, but request_id should still advance
        // because it's incremented before the HTTP call.
        assert_eq!(client.request_id, 2);
        let _ = client.call("eth_chainId", vec![]);
        assert_eq!(client.request_id, 3);
    }

    #[test]
    fn test_rpc_client_get_block_number_no_std() {
        let mut client = RpcClient::new("https://test.com/rpc".into(), 1);
        let result = client.get_block_number();
        // Without std feature, must return RpcError with descriptive message
        assert!(matches!(&result, Err(SwapError::RpcError(_))));
    }

    #[test]
    fn test_rpc_client_estimate_gas_no_std() {
        let mut client = RpcClient::new("https://test.com/rpc".into(), 1);
        let result = client.estimate_gas("0xabc", "0xdef", "0x1234");
        assert!(matches!(&result, Err(SwapError::RpcError(_))));
    }

    #[test]
    fn test_rpc_client_get_balance_no_std() {
        let mut client = RpcClient::new("https://test.com/rpc".into(), 1);
        let result = client.get_balance("0xabc");
        assert!(matches!(&result, Err(SwapError::RpcError(_))));
    }

    #[test]
    fn test_rpc_client_serialize_request() {
        let request = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: 42,
            method: "eth_blockNumber".into(),
            params: vec![],
        };
        let json = serde_json::to_string(&request).unwrap();
        assert_eq!(
            json,
            r#"{"jsonrpc":"2.0","id":42,"method":"eth_blockNumber","params":[]}"#
        );
    }

    #[test]
    fn test_rpc_client_deserialize_response() {
        let json = r#"{"jsonrpc":"2.0","id":1,"result":"0x10","error":null}"#;
        let resp: JsonRpcResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.jsonrpc, "2.0");
        assert_eq!(resp.id, 1);
        assert!(resp.error.is_none());
        assert_eq!(resp.result.unwrap().as_str().unwrap(), "0x10");
    }

    #[test]
    fn test_rpc_client_deserialize_error() {
        let json = r#"{"jsonrpc":"2.0","id":1,"result":null,"error":{"code":-32601,"message":"Method not found"}}"#;
        let resp: JsonRpcResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.jsonrpc, "2.0");
        let err = resp.error.unwrap();
        assert_eq!(err.code, -32601);
        assert_eq!(err.message, "Method not found");
        assert!(resp.result.is_none());
    }

    #[test]
    fn test_rpc_client_config_defaults() {
        let config = RpcClientConfig::default();
        assert_eq!(config.rpc_url, "");
        assert_eq!(config.timeout_secs, 30);
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.retry_delay_ms, 500);
        assert_eq!(config.chain_id, 1);
    }

    #[test]
    fn test_rpc_client_json_rpc_fields() {
        let req = serde_json::from_str::<JsonRpcRequest>(
            r#"{"jsonrpc":"2.0","id":1,"method":"test","params":["a"]}"#,
        )
        .unwrap();
        assert_eq!(req.jsonrpc, "2.0");
        assert_eq!(req.id, 1);
        assert_eq!(req.method, "test");
        assert_eq!(req.params.len(), 1);
    }

    #[test]
    fn test_gas_price_no_std() {
        let mut client = RpcClient::new("https://test.com/rpc".into(), 1);
        let result = client.gas_price();
        assert!(matches!(&result, Err(SwapError::RpcError(_))));
    }

    #[test]
    fn test_get_transaction_count_no_std() {
        let mut client = RpcClient::new("https://test.com/rpc".into(), 1);
        let result = client.get_transaction_count("0xabc", "latest");
        assert!(matches!(&result, Err(SwapError::RpcError(_))));
    }

    #[test]
    fn test_eth_call_no_std() {
        let mut client = RpcClient::new("https://test.com/rpc".into(), 1);
        let result = client.eth_call("0xabc", "0xdef", "0x1234", "latest");
        assert!(matches!(&result, Err(SwapError::RpcError(_))));
    }

    #[test]
    fn test_send_raw_transaction_no_std() {
        let mut client = RpcClient::new("https://test.com/rpc".into(), 1);
        let result = client.send_raw_transaction("0xdeadbeef");
        assert!(matches!(&result, Err(SwapError::RpcError(_))));
    }

    #[test]
    fn test_chain_id_no_std() {
        let mut client = RpcClient::new("https://test.com/rpc".into(), 1);
        let result = client.chain_id();
        assert!(matches!(&result, Err(SwapError::RpcError(_))));
    }
}
