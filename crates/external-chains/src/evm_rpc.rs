//! The Ethereum JSON-RPC calls every EVM adapter makes, implemented once.
//!
//! Every EVM chain in this crate speaks the same wire protocol for the
//! read-only calls: `eth_chainId`, `eth_blockNumber`, `eth_getBalance`,
//! `eth_call`, `eth_gasPrice`, `eth_getTransactionReceipt`. Before this module
//! existed each adapter carried a private copy of the request encoder and hex
//! parsers, and only `BaseAdapter` ever called them — the other adapters
//! answered `get_balance` with a constant 1 ETH and `get_block_number` with a
//! constant block number instead.
//!
//! Everything here either returns what the node said or fails. Nothing in this
//! module invents a value.

use crate::adapter::{AdapterResult, ChainConfig, TransactionReceipt};
use crate::error::ExternalChainError;
use alloc::{format, string::String, vec, vec::Vec};
use sp_core::{H160, H256, U256};

/// Build a JSON-RPC 2.0 request body.
pub(crate) fn request(method: &str, params: &str) -> Vec<u8> {
    format!(
        r#"{{"jsonrpc":"2.0","method":"{}","params":{},"id":1}}"#,
        method, params
    )
    .into_bytes()
}

/// The configured RPC endpoint as a string.
pub(crate) fn url(config: &ChainConfig) -> String {
    String::from_utf8_lossy(&config.rpc_url).to_string()
}

/// POST a JSON-RPC call to `url` and return the raw response body.
pub(crate) async fn call(url: &str, method: &str, params: &str) -> AdapterResult<Vec<u8>> {
    let body = request(method, params);
    crate::rpc_http::post_json(url, &body)
        .await
        .map_err(|e| ExternalChainError::rpc_error(&format!("HTTP error: {}", e)))
}

/// Extract the `result` field of a JSON-RPC response.
///
/// A response with `"error":null` is a success — JSON-RPC allows the member to
/// be present and null, and treating its mere presence as a failure rejected
/// every response from nodes that include it.
pub(crate) fn extract_result(response: &[u8]) -> AdapterResult<String> {
    let text = String::from_utf8_lossy(response);

    if let Some(idx) = text.find("\"error\"") {
        let after = text[idx + 7..].trim_start();
        let after = after.strip_prefix(':').unwrap_or(after).trim_start();
        if !after.starts_with("null") {
            return Err(ExternalChainError::rpc_error(&format!(
                "RPC error: {}",
                text
            )));
        }
    }

    let idx = text
        .find("\"result\"")
        .ok_or_else(|| ExternalChainError::parse_error("RPC response has no result field"))?;
    let after = text[idx + 8..].trim_start();
    let after = after.strip_prefix(':').unwrap_or(after).trim_start();

    if after.starts_with("null") {
        return Ok("null".to_string());
    }

    if after.starts_with('"') {
        let end = after[1..]
            .find('"')
            .ok_or_else(|| ExternalChainError::parse_error("unterminated string result"))?;
        return Ok(after[1..=end].to_string());
    }

    for (open, close) in [('{', '}'), ('[', ']')] {
        if after.starts_with(open) {
            let mut depth = 0i32;
            let mut in_string = false;
            let mut escaped = false;
            for (i, c) in after.char_indices() {
                if in_string {
                    if escaped {
                        escaped = false;
                    } else if c == '\\' {
                        escaped = true;
                    } else if c == '"' {
                        in_string = false;
                    }
                    continue;
                }
                match c {
                    '"' => in_string = true,
                    _ if c == open => depth += 1,
                    _ if c == close => {
                        depth -= 1;
                        if depth == 0 {
                            return Ok(after[..=i].to_string());
                        }
                    }
                    _ => {}
                }
            }
            return Err(ExternalChainError::parse_error(
                "unterminated JSON value in RPC result",
            ));
        }
    }

    // A bare number (JSON-RPC allows `"result": 12`) is returned as-is.
    let end = after
        .find(|c: char| !c.is_ascii_alphanumeric() && c != '.' && c != '-' && c != '+')
        .unwrap_or(after.len());
    if end == 0 {
        return Err(ExternalChainError::parse_error(
            "RPC result is not a JSON value",
        ));
    }
    Ok(after[..end].to_string())
}

/// Parse a `0x`-prefixed hex quantity into a `u64`.
pub(crate) fn parse_hex_u64(hex_str: &str) -> AdapterResult<u64> {
    let trimmed = hex_str.trim().trim_matches('"');
    let without_prefix = trimmed.strip_prefix("0x").unwrap_or(trimmed);
    u64::from_str_radix(without_prefix, 16)
        .map_err(|e| ExternalChainError::parse_error(&format!("hex parse: {}", e)))
}

/// Parse a `0x`-prefixed hex quantity into a `U256`.
///
/// A word longer than 32 bytes is refused rather than truncated: silently
/// keeping the low 32 bytes of an oversized quantity would turn a malformed
/// response into a plausible-looking balance.
pub(crate) fn parse_hex_u256(hex_str: &str) -> AdapterResult<U256> {
    let trimmed = hex_str.trim().trim_matches('"');
    let without_prefix = trimmed.strip_prefix("0x").unwrap_or(trimmed);
    if without_prefix.is_empty() || without_prefix.len() > 64 {
        return Err(ExternalChainError::parse_error(
            "hex quantity is not a 32-byte word",
        ));
    }
    let padded = format!("{:0>64}", without_prefix);
    let bytes = hex::decode(&padded)
        .map_err(|e| ExternalChainError::parse_error(&format!("hex decode: {}", e)))?;
    let mut word = [0u8; 32];
    word.copy_from_slice(&bytes);
    Ok(U256::from_big_endian(&word))
}

/// Parse a 32-byte hex hash.
pub(crate) fn parse_hex_h256(hex_str: &str) -> AdapterResult<H256> {
    let trimmed = hex_str.trim().trim_matches('"');
    let without_prefix = trimmed.strip_prefix("0x").unwrap_or(trimmed);
    let bytes = hex::decode(without_prefix)
        .map_err(|e| ExternalChainError::parse_error(&format!("hash decode: {}", e)))?;
    if bytes.len() != 32 {
        return Err(ExternalChainError::parse_error("hash is not 32 bytes long"));
    }
    Ok(H256::from_slice(&bytes))
}

/// `eth_chainId`.
pub(crate) async fn chain_id(url: &str) -> AdapterResult<u64> {
    let response = call(url, "eth_chainId", "[]").await?;
    parse_hex_u64(&extract_result(&response)?)
}

/// `eth_blockNumber`.
pub(crate) async fn block_number(url: &str) -> AdapterResult<u64> {
    let response = call(url, "eth_blockNumber", "[]").await?;
    parse_hex_u64(&extract_result(&response)?)
}

/// `eth_getBalance`.
pub(crate) async fn balance(url: &str, address: H160) -> AdapterResult<U256> {
    let params = format!(r#"["0x{}","latest"]"#, hex::encode(address.as_bytes()));
    let response = call(url, "eth_getBalance", &params).await?;
    parse_hex_u256(&extract_result(&response)?)
}

/// ERC20 `balanceOf(address)` through `eth_call`.
pub(crate) async fn token_balance(url: &str, token: H160, address: H160) -> AdapterResult<U256> {
    let calldata = encode_balance_of(address);
    let params = format!(
        r#"[{{"to":"0x{}","data":"0x{}"}},"latest"]"#,
        hex::encode(token.as_bytes()),
        hex::encode(&calldata)
    );
    let response = call(url, "eth_call", &params).await?;
    parse_hex_u256(&extract_result(&response)?)
}

/// `eth_gasPrice`.
pub(crate) async fn gas_price(url: &str) -> AdapterResult<U256> {
    let response = call(url, "eth_gasPrice", "[]").await?;
    parse_hex_u256(&extract_result(&response)?)
}

/// `eth_getTransactionReceipt`.
pub(crate) async fn receipt(url: &str, tx_hash: H256) -> AdapterResult<Option<TransactionReceipt>> {
    let params = format!(r#"["0x{}"]"#, hex::encode(tx_hash.as_bytes()));
    let response = call(url, "eth_getTransactionReceipt", &params).await?;
    let result = extract_result(&response)?;
    parse_receipt(tx_hash, &result)
}

/// One `eth_getLogs` entry, as the node stated it.
///
/// Every field is required: a log missing one of them is refused by `logs()`
/// rather than defaulted, because a defaulted block number or topic would be a
/// message the relayer acts on but the chain never emitted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogEntry {
    pub address: H160,
    pub topics: Vec<H256>,
    pub data: Vec<u8>,
    pub block_number: u64,
    pub transaction_hash: H256,
    pub log_index: u64,
}

/// Hex-decode a `0x`-prefixed string, refusing an odd length or a bad digit.
fn decode_hex(value: &str, what: &str) -> AdapterResult<Vec<u8>> {
    let digits = value.strip_prefix("0x").unwrap_or(value);
    if digits.len() % 2 != 0 {
        return Err(ExternalChainError::rpc_error(&format!(
            "{what} has an odd number of hex digits: {value}"
        )));
    }
    hex::decode(digits)
        .map_err(|e| ExternalChainError::rpc_error(&format!("{what} is not hex ({value}): {e}")))
}

fn field_u64(value: &serde_json::Value, what: &str) -> AdapterResult<u64> {
    let text = value
        .as_str()
        .ok_or_else(|| ExternalChainError::rpc_error(&format!("{what} is not a string")))?;
    parse_hex_u64(text)
        .map_err(|e| ExternalChainError::rpc_error(&format!("{what} is not a quantity: {e}")))
}

/// Parse one `eth_getLogs` entry.
pub(crate) fn parse_log_entry(entry: &serde_json::Value) -> AdapterResult<LogEntry> {
    let object = entry
        .as_object()
        .ok_or_else(|| ExternalChainError::rpc_error("a log entry is not an object"))?;
    let get = |name: &str| -> AdapterResult<&serde_json::Value> {
        object
            .get(name)
            .ok_or_else(|| ExternalChainError::rpc_error(&format!("a log entry has no {name}")))
    };

    let address_bytes = decode_hex(
        get("address")?
            .as_str()
            .ok_or_else(|| ExternalChainError::rpc_error("log address is not a string"))?,
        "log address",
    )?;
    if address_bytes.len() != 20 {
        return Err(ExternalChainError::rpc_error(&format!(
            "log address is {} bytes, not 20",
            address_bytes.len()
        )));
    }
    let mut address = [0u8; 20];
    address.copy_from_slice(&address_bytes);

    let mut topics = Vec::new();
    for topic in get("topics")?
        .as_array()
        .ok_or_else(|| ExternalChainError::rpc_error("log topics are not an array"))?
    {
        let bytes = decode_hex(
            topic
                .as_str()
                .ok_or_else(|| ExternalChainError::rpc_error("a topic is not a string"))?,
            "log topic",
        )?;
        if bytes.len() != 32 {
            return Err(ExternalChainError::rpc_error(&format!(
                "a log topic is {} bytes, not 32",
                bytes.len()
            )));
        }
        topics.push(H256::from_slice(&bytes));
    }

    let data = decode_hex(
        get("data")?
            .as_str()
            .ok_or_else(|| ExternalChainError::rpc_error("log data is not a string"))?,
        "log data",
    )?;

    let transaction_hash_bytes = decode_hex(
        get("transactionHash")?
            .as_str()
            .ok_or_else(|| ExternalChainError::rpc_error("log transactionHash is not a string"))?,
        "log transactionHash",
    )?;
    if transaction_hash_bytes.len() != 32 {
        return Err(ExternalChainError::rpc_error(&format!(
            "log transactionHash is {} bytes, not 32",
            transaction_hash_bytes.len()
        )));
    }

    Ok(LogEntry {
        address: H160(address),
        topics,
        data,
        block_number: field_u64(get("blockNumber")?, "log blockNumber")?,
        transaction_hash: H256::from_slice(&transaction_hash_bytes),
        log_index: field_u64(get("logIndex")?, "log logIndex")?,
    })
}

/// Fetch logs for a bounded block range, filtered by address and first topic.
///
/// The range is inclusive and bounded by the caller on purpose: `eth_getLogs`
/// over an open range is a request every public node refuses, and a silent
/// fallback to "no range" would be a query that never returns logs.
pub(crate) async fn logs(
    url: &str,
    from_block: u64,
    to_block: u64,
    address: H160,
    topic0: H256,
) -> AdapterResult<Vec<LogEntry>> {
    if from_block > to_block {
        return Err(ExternalChainError::rpc_error(&format!(
            "log range is inverted: {from_block}..{to_block}"
        )));
    }
    let params = format!(
        r#"[{{"fromBlock":"0x{:x}","toBlock":"0x{:x}","address":"0x{}","topics":["0x{}"]}}]"#,
        from_block,
        to_block,
        hex::encode(address.as_bytes()),
        hex::encode(topic0.as_bytes())
    );
    let response = call(url, "eth_getLogs", &params).await?;
    let text = String::from_utf8_lossy(&response);
    let parsed: serde_json::Value = serde_json::from_str(&text).map_err(|e| {
        ExternalChainError::rpc_error(&format!("eth_getLogs response is not JSON: {e}"))
    })?;
    if let Some(error) = parsed.get("error").filter(|e| !e.is_null()) {
        return Err(ExternalChainError::rpc_error(&format!(
            "eth_getLogs error: {error}"
        )));
    }
    let result = parsed
        .get("result")
        .and_then(|r| r.as_array())
        .ok_or_else(|| ExternalChainError::rpc_error("eth_getLogs returned no result array"))?;

    let mut entries = Vec::with_capacity(result.len());
    for entry in result {
        entries.push(parse_log_entry(entry)?);
    }
    Ok(entries)
}

/// The Unix timestamp of a block, as the chain states it.
pub(crate) async fn block_timestamp(url: &str, block_number: u64) -> AdapterResult<u64> {
    let params = format!(r#"["0x{block_number:x}",false]"#);
    let response = call(url, "eth_getBlockByNumber", &params).await?;
    let text = String::from_utf8_lossy(&response);
    let parsed: serde_json::Value = serde_json::from_str(&text).map_err(|e| {
        ExternalChainError::rpc_error(&format!("eth_getBlockByNumber response is not JSON: {e}"))
    })?;
    let result = parsed
        .get("result")
        .filter(|r| !r.is_null())
        .ok_or_else(|| {
            ExternalChainError::rpc_error(&format!("no block at height {block_number}"))
        })?;
    field_u64(
        result
            .get("timestamp")
            .ok_or_else(|| ExternalChainError::rpc_error("the block has no timestamp"))?,
        "block timestamp",
    )
}

/// Encode the ERC20 `balanceOf(address)` call.
pub(crate) fn encode_balance_of(address: H160) -> Vec<u8> {
    let mut calldata = Vec::with_capacity(36);
    // balanceOf(address) selector
    calldata.extend_from_slice(&[0x70, 0xa0, 0x82, 0x31]);
    calldata.extend_from_slice(&[0u8; 12]);
    calldata.extend_from_slice(address.as_bytes());
    calldata
}

/// Parse an `eth_getTransactionReceipt` result.
///
/// The receipt is only reported when every field it claims could be read from
/// the response. A receipt that carried logs is *refused*: `TransactionReceipt`
/// has a `logs` field, and returning it empty when the chain returned logs
/// would tell a relayer that a bridge transaction emitted nothing.
pub(crate) fn parse_receipt(
    tx_hash: H256,
    result: &str,
) -> AdapterResult<Option<TransactionReceipt>> {
    if result == "null" {
        return Ok(None);
    }

    if !result.starts_with('{') {
        return Err(ExternalChainError::parse_error(
            "transaction receipt is not a JSON object",
        ));
    }

    if let Some(logs) = json_array_field(result, "logs") {
        if logs.trim() != "[]" {
            return Err(ExternalChainError::adapter_unimplemented(
                "transaction receipt carries logs, which this adapter cannot decode; \
                 refusing rather than reporting an empty log list",
            ));
        }
    }

    let status = json_string_field(result, "status")
        .ok_or_else(|| ExternalChainError::parse_error("receipt has no status field"))?;
    let success = status == "0x1";
    if !success && status != "0x0" {
        return Err(ExternalChainError::parse_error(
            "receipt status is neither 0x0 nor 0x1",
        ));
    }

    let block_number = parse_hex_u64(
        &json_string_field(result, "blockNumber")
            .ok_or_else(|| ExternalChainError::parse_error("receipt has no blockNumber field"))?,
    )?;
    let block_hash = parse_hex_h256(
        &json_string_field(result, "blockHash")
            .ok_or_else(|| ExternalChainError::parse_error("receipt has no blockHash field"))?,
    )?;
    let tx_index = parse_hex_u64(
        &json_string_field(result, "transactionIndex")
            .ok_or_else(|| ExternalChainError::parse_error("receipt has no transactionIndex"))?,
    )?;
    if tx_index > u32::MAX as u64 {
        return Err(ExternalChainError::parse_error(
            "receipt transactionIndex does not fit in u32",
        ));
    }
    let gas_used = parse_hex_u64(
        &json_string_field(result, "gasUsed")
            .ok_or_else(|| ExternalChainError::parse_error("receipt has no gasUsed"))?,
    )?;

    Ok(Some(TransactionReceipt {
        tx_hash,
        block_number,
        block_hash,
        tx_index: tx_index as u32,
        success,
        gas_used,
        logs: vec![],
    }))
}

/// Read `"<field>":"<value>"` out of a flat JSON object body.
fn json_string_field(body: &str, field: &str) -> Option<String> {
    let needle = format!("\"{}\":\"", field);
    let idx = body.find(&needle)?;
    let after = &body[idx + needle.len()..];
    let end = after.find('"')?;
    Some(after[..end].to_string())
}

/// Read `"<field>":[...]` out of a flat JSON object body.
fn json_array_field(body: &str, field: &str) -> Option<String> {
    let needle = format!("\"{}\":", field);
    let idx = body.find(&needle)?;
    let after = body[idx + needle.len()..].trim_start();
    if !after.starts_with('[') {
        return None;
    }
    let mut depth = 0i32;
    for (i, c) in after.char_indices() {
        match c {
            '[' => depth += 1,
            ']' => {
                depth -= 1;
                if depth == 0 {
                    return Some(after[..=i].to_string());
                }
            }
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    const SHORT_RECEIPT: &str = r#"{"jsonrpc":"2.0","result":{"transactionHash":"0x1111111111111111111111111111111111111111111111111111111111111111","status":"0x1","blockNumber":"0x10","blockHash":"0x2222222222222222222222222222222222222222222222222222222222222222","transactionIndex":"0x2","gasUsed":"0x5208","logs":[]},"id":1}"#;

    #[test]
    fn a_successful_response_with_a_null_error_is_not_an_error() {
        let body = br#"{"jsonrpc":"2.0","result":"0x2105","error":null,"id":1}"#;
        assert_eq!(extract_result(body).unwrap(), "0x2105");
    }

    #[test]
    fn a_real_error_member_is_still_an_error() {
        let body =
            br#"{"jsonrpc":"2.0","error":{"code":-32601,"message":"method not found"},"id":1}"#;
        assert!(matches!(
            extract_result(body),
            Err(ExternalChainError::RpcError(_))
        ));
    }

    #[test]
    fn object_and_array_results_are_extracted_whole() {
        assert_eq!(
            extract_result(br#"{"result":{"a":"}"},"id":1}"#).unwrap(),
            r#"{"a":"}"}"#
        );
        assert_eq!(
            extract_result(br#"{"result":[{"a":1},{"b":2}],"id":1}"#).unwrap(),
            r#"[{"a":1},{"b":2}]"#
        );
    }

    #[test]
    fn an_oversized_hex_quantity_is_refused_not_truncated() {
        let too_long = ["0x", &"f".repeat(66)].concat();
        assert!(matches!(
            parse_hex_u256(&too_long),
            Err(ExternalChainError::ParseError(_))
        ));
        assert_eq!(parse_hex_u256("0x0").unwrap(), U256::zero());
        assert_eq!(
            parse_hex_u256("0xde0b6b3a7640000").unwrap(),
            U256::from(1_000_000_000_000_000_000u64)
        );
    }

    #[test]
    fn a_hash_that_is_not_32_bytes_is_refused_not_padded() {
        assert!(parse_hex_h256("0xabcd").is_err());
        assert_eq!(
            parse_hex_h256("0x2222222222222222222222222222222222222222222222222222222222222222")
                .unwrap(),
            H256::from_slice(&[0x22u8; 32])
        );
    }

    #[test]
    fn a_receipt_without_logs_parses() {
        let parsed = parse_receipt(H256::from_slice(&[0x11u8; 32]), "{\"status\":\"0x1\",\"blockNumber\":\"0x10\",\"blockHash\":\"0x2222222222222222222222222222222222222222222222222222222222222222\",\"transactionIndex\":\"0x2\",\"gasUsed\":\"0x5208\",\"logs\":[]}")
            .expect("receipt parses");
        let receipt = parsed.expect("receipt is present");
        assert!(receipt.success);
        assert_eq!(receipt.block_number, 16);
        assert_eq!(receipt.tx_index, 2);
        assert_eq!(receipt.gas_used, 21_000);
        assert!(receipt.logs.is_empty());
    }

    #[test]
    fn a_null_receipt_is_pending_not_an_error() {
        assert!(parse_receipt(H256::zero(), "null").unwrap().is_none());
    }

    #[test]
    fn a_receipt_that_carries_logs_is_refused_rather_than_stripped() {
        let with_logs = r#"{"status":"0x1","blockNumber":"0x10","blockHash":"0x2222222222222222222222222222222222222222222222222222222222222222","transactionIndex":"0x2","gasUsed":"0x5208","logs":[{"address":"0x4200000000000000000000000000000000000007"}]}"#;
        assert!(matches!(
            parse_receipt(H256::zero(), with_logs),
            Err(ExternalChainError::AdapterUnimplemented(_))
        ));
    }

    #[test]
    fn a_failed_receipt_reports_failure() {
        let reverted = r#"{"status":"0x0","blockNumber":"0x10","blockHash":"0x2222222222222222222222222222222222222222222222222222222222222222","transactionIndex":"0x0","gasUsed":"0x5208","logs":[]}"#;
        let receipt = parse_receipt(H256::zero(), reverted).unwrap().unwrap();
        assert!(!receipt.success);
    }

    #[test]
    fn a_receipt_missing_a_field_is_refused_not_defaulted() {
        let no_block_hash = r#"{"status":"0x1","blockNumber":"0x10","transactionIndex":"0x0","gasUsed":"0x5208","logs":[]}"#;
        assert!(parse_receipt(H256::zero(), no_block_hash).is_err());
    }

    #[test]
    fn the_rpc_response_used_in_the_receipt_test_is_a_json_rpc_body() {
        // Guards the fixture above against drifting out of the shape `receipt()`
        // feeds `parse_receipt` (it passes the extracted result, not the body).
        let result = extract_result(SHORT_RECEIPT.as_bytes()).unwrap();
        assert!(parse_receipt(H256::zero(), &result).unwrap().is_some());
    }
}
