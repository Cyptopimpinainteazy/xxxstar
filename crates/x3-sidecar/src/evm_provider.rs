use anyhow::{anyhow, Context};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct EvmProviderPool {
    client: Client,
    providers: Arc<RwLock<Vec<ProviderState>>>,
}

#[derive(Clone, Debug)]
struct ProviderState {
    url: String,
    successes: u64,
    failures: u64,
    last_latency_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvmTransaction {
    pub hash: String,
    pub from: String,
    pub to: Option<String>,
    pub input: String,
    pub gas: u64,
    pub value: u128,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvmBlock {
    pub number: u64,
    pub timestamp: u64,
    pub transactions: Vec<EvmTransaction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvmReceipt {
    pub transaction_hash: String,
    pub status: bool,
    pub gas_used: u64,
    pub logs_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvmLog {
    pub transaction_hash: Option<String>,
    pub address: String,
    pub topic0: Option<String>,
    pub block_number: u64,
}

#[derive(Debug, Clone)]
pub struct EvmIngestionWindow {
    pub blocks: Vec<EvmBlock>,
    pub receipts: Vec<EvmReceipt>,
    pub logs: Vec<EvmLog>,
}

#[derive(Debug, Deserialize)]
struct JsonRpcResponse {
    id: u64,
    result: Option<Value>,
    error: Option<JsonRpcError>,
}

#[derive(Debug, Deserialize)]
struct JsonRpcError {
    code: i64,
    message: String,
}

impl EvmProviderPool {
    pub fn new(endpoints: Vec<String>) -> anyhow::Result<Self> {
        if endpoints.is_empty() {
            anyhow::bail!("at least one EVM endpoint is required");
        }

        Ok(Self {
            client: Client::new(),
            providers: Arc::new(RwLock::new(
                endpoints
                    .into_iter()
                    .map(|url| ProviderState {
                        url,
                        successes: 0,
                        failures: 0,
                        last_latency_ms: 0,
                    })
                    .collect(),
            )),
        })
    }

    pub async fn ingest_recent_window(
        &self,
        sample_blocks: u64,
    ) -> anyhow::Result<EvmIngestionWindow> {
        let latest_block = self.latest_block_number().await?;
        let sample_blocks = sample_blocks.max(2).min(16);
        let start = latest_block.saturating_sub(sample_blocks.saturating_sub(1));

        let block_requests: Vec<Value> = (start..=latest_block)
            .enumerate()
            .map(|(idx, block_number)| {
                json!({
                    "jsonrpc": "2.0",
                    "id": idx as u64 + 1,
                    "method": "eth_getBlockByNumber",
                    "params": [format!("0x{:x}", block_number), true]
                })
            })
            .collect();

        let block_responses = self.batch_request(block_requests).await?;
        let mut blocks = Vec::new();
        for response in block_responses {
            if let Some(error) = response.error {
                return Err(anyhow!(
                    "provider returned block error {}: {}",
                    error.code,
                    error.message
                ));
            }
            let block = response
                .result
                .as_ref()
                .context("missing block result")
                .and_then(parse_block)?;
            blocks.push(block);
        }

        let tx_hashes: Vec<String> = blocks
            .iter()
            .flat_map(|block| block.transactions.iter().map(|tx| tx.hash.clone()))
            .collect();

        let receipt_requests: Vec<Value> = tx_hashes
            .iter()
            .enumerate()
            .map(|(idx, tx_hash)| {
                json!({
                    "jsonrpc": "2.0",
                    "id": idx as u64 + 10_000,
                    "method": "eth_getTransactionReceipt",
                    "params": [tx_hash]
                })
            })
            .collect();

        let receipt_responses = self.batch_request(receipt_requests).await?;
        let mut receipts = Vec::new();
        for response in receipt_responses {
            if let Some(error) = response.error {
                return Err(anyhow!(
                    "provider returned receipt error {}: {}",
                    error.code,
                    error.message
                ));
            }
            let receipt = response
                .result
                .as_ref()
                .context("missing receipt result")
                .and_then(parse_receipt)?;
            receipts.push(receipt);
        }

        let log_response = self
            .single_request(json!({
                "jsonrpc": "2.0",
                "id": 20_000u64,
                "method": "eth_getLogs",
                "params": [{
                    "fromBlock": format!("0x{:x}", start),
                    "toBlock": format!("0x{:x}", latest_block)
                }]
            }))
            .await?;

        if let Some(error) = log_response.error {
            return Err(anyhow!(
                "provider returned logs error {}: {}",
                error.code,
                error.message
            ));
        }

        let logs = log_response
            .result
            .as_ref()
            .context("missing logs result")
            .and_then(parse_logs)?;

        Ok(EvmIngestionWindow {
            blocks,
            receipts,
            logs,
        })
    }

    async fn latest_block_number(&self) -> anyhow::Result<u64> {
        let response = self
            .single_request(json!({
                "jsonrpc": "2.0",
                "id": 1u64,
                "method": "eth_blockNumber",
                "params": []
            }))
            .await?;

        if let Some(error) = response.error {
            return Err(anyhow!(
                "provider returned latest block error {}: {}",
                error.code,
                error.message
            ));
        }

        parse_hex_u64(
            response
                .result
                .as_ref()
                .context("missing latest block result")?,
        )
    }

    async fn single_request(&self, request: Value) -> anyhow::Result<JsonRpcResponse> {
        let index = self.best_provider_index().await?;
        let url = self.provider_url(index).await?;
        let start = Instant::now();
        let result = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .with_context(|| format!("failed request to {url}"))?
            .json::<JsonRpcResponse>()
            .await;
        self.record_result(index, start.elapsed().as_millis() as u64, result.is_ok())
            .await;
        Ok(result?)
    }

    async fn batch_request(&self, requests: Vec<Value>) -> anyhow::Result<Vec<JsonRpcResponse>> {
        let index = self.best_provider_index().await?;
        let url = self.provider_url(index).await?;
        let start = Instant::now();
        let result = self
            .client
            .post(&url)
            .json(&requests)
            .send()
            .await
            .with_context(|| format!("failed batch request to {url}"))?
            .json::<Vec<JsonRpcResponse>>()
            .await;
        self.record_result(index, start.elapsed().as_millis() as u64, result.is_ok())
            .await;
        let mut responses = result?;
        responses.sort_by_key(|response| response.id);
        Ok(responses)
    }

    async fn best_provider_index(&self) -> anyhow::Result<usize> {
        let providers = self.providers.read().await;
        providers
            .iter()
            .enumerate()
            .max_by_key(|(_, provider)| {
                let success_bias = provider.successes.saturating_mul(1000);
                let failure_penalty = provider.failures.saturating_mul(100);
                let latency_penalty = provider.last_latency_ms;
                success_bias
                    .saturating_sub(failure_penalty)
                    .saturating_sub(latency_penalty)
            })
            .map(|(idx, _)| idx)
            .ok_or_else(|| anyhow!("no providers configured"))
    }

    async fn provider_url(&self, index: usize) -> anyhow::Result<String> {
        self.providers
            .read()
            .await
            .get(index)
            .map(|provider| provider.url.clone())
            .ok_or_else(|| anyhow!("provider index {index} out of bounds"))
    }

    async fn record_result(&self, index: usize, latency_ms: u64, success: bool) {
        if let Some(provider) = self.providers.write().await.get_mut(index) {
            provider.last_latency_ms = latency_ms;
            if success {
                provider.successes = provider.successes.saturating_add(1);
            } else {
                provider.failures = provider.failures.saturating_add(1);
            }
        }
    }
}

fn parse_block(value: &Value) -> anyhow::Result<EvmBlock> {
    let number = parse_hex_u64(value.get("number").context("missing block number")?)?;
    let timestamp = parse_hex_u64(value.get("timestamp").context("missing block timestamp")?)?;
    let transactions = value
        .get("transactions")
        .and_then(Value::as_array)
        .context("missing block transactions")?
        .iter()
        .map(parse_transaction)
        .collect::<anyhow::Result<Vec<_>>>()?;
    Ok(EvmBlock {
        number,
        timestamp,
        transactions,
    })
}

fn parse_transaction(value: &Value) -> anyhow::Result<EvmTransaction> {
    Ok(EvmTransaction {
        hash: value
            .get("hash")
            .and_then(Value::as_str)
            .context("missing tx hash")?
            .to_string(),
        from: value
            .get("from")
            .and_then(Value::as_str)
            .context("missing tx from")?
            .to_string(),
        to: value
            .get("to")
            .and_then(Value::as_str)
            .map(ToString::to_string),
        input: value
            .get("input")
            .and_then(Value::as_str)
            .unwrap_or("0x")
            .to_string(),
        gas: parse_hex_u64(value.get("gas").context("missing tx gas")?)?,
        value: parse_hex_u128(value.get("value").context("missing tx value")?)?,
    })
}

fn parse_receipt(value: &Value) -> anyhow::Result<EvmReceipt> {
    let logs_count = value
        .get("logs")
        .and_then(Value::as_array)
        .map(|logs| logs.len())
        .unwrap_or(0);
    let status = parse_hex_u64(value.get("status").context("missing receipt status")?)? == 1;
    Ok(EvmReceipt {
        transaction_hash: value
            .get("transactionHash")
            .and_then(Value::as_str)
            .context("missing receipt transactionHash")?
            .to_string(),
        status,
        gas_used: parse_hex_u64(value.get("gasUsed").context("missing receipt gasUsed")?)?,
        logs_count,
    })
}

fn parse_logs(value: &Value) -> anyhow::Result<Vec<EvmLog>> {
    value
        .as_array()
        .context("logs result must be an array")?
        .iter()
        .map(parse_log)
        .collect()
}

fn parse_log(value: &Value) -> anyhow::Result<EvmLog> {
    let topic0 = value
        .get("topics")
        .and_then(Value::as_array)
        .and_then(|topics| topics.first())
        .and_then(Value::as_str)
        .map(ToString::to_string);

    Ok(EvmLog {
        transaction_hash: value
            .get("transactionHash")
            .and_then(Value::as_str)
            .map(ToString::to_string),
        address: value
            .get("address")
            .and_then(Value::as_str)
            .context("missing log address")?
            .to_string(),
        topic0,
        block_number: parse_hex_u64(
            value
                .get("blockNumber")
                .context("missing log blockNumber")?,
        )?,
    })
}

fn parse_hex_u64(value: &Value) -> anyhow::Result<u64> {
    let raw = value.as_str().context("hex value must be string")?;
    u64::from_str_radix(raw.trim_start_matches("0x"), 16)
        .with_context(|| format!("invalid u64 hex value {raw}"))
}

fn parse_hex_u128(value: &Value) -> anyhow::Result<u128> {
    let raw = value.as_str().context("hex value must be string")?;
    u128::from_str_radix(raw.trim_start_matches("0x"), 16)
        .with_context(|| format!("invalid u128 hex value {raw}"))
}

pub fn lane_key(tx: &EvmTransaction) -> String {
    if let Some(to) = &tx.to {
        if tx.input.len() >= 10 {
            return format!("{}:{}", to.to_lowercase(), &tx.input[..10]);
        }
        return to.to_lowercase();
    }
    tx.from.to_lowercase()
}

#[cfg(test)]
pub(crate) async fn start_mock_evm_server() -> httpmock::MockServer {
    use httpmock::Method::POST;

    let server = httpmock::MockServer::start_async().await;

    server
        .mock_async(|when, then| {
            when.method(POST)
                .header("content-type", "application/json")
                .body_contains("\"method\":\"eth_blockNumber\"");
            then.status(200)
                .header("content-type", "application/json")
                .body(
                    serde_json::to_string(&serde_json::json!({
                        "id": 1,
                        "result": "0xa",
                        "error": null
                    }))
                    .expect("serialize"),
                );
        })
        .await;

    server
        .mock_async(|when, then| {
            when.method(POST)
                .header("content-type", "application/json")
                .body_contains("eth_getBlockByNumber");
            then.status(200)
                .header("content-type", "application/json")
                .body(
                    serde_json::to_string(&vec![
                        serde_json::json!({
                            "id": 1,
                            "result": {
                                "number": "0x9",
                                "timestamp": "0x64",
                                "transactions": [{
                                    "hash": "0xaaa",
                                    "from": "0x111",
                                    "to": "0x222",
                                    "input": "0xabcdef12",
                                    "gas": "0x5208",
                                    "value": "0x1"
                                }]},
                            "error": null
                        }),
                        serde_json::json!({
                            "id": 2,
                            "result": {
                                "number": "0xa",
                                "timestamp": "0x66",
                                "transactions": [{
                                    "hash": "0xbbb",
                                    "from": "0x333",
                                    "to": "0x444",
                                    "input": "0xdeadbeef",
                                    "gas": "0x5208",
                                    "value": "0x2"
                                }]},
                            "error": null
                        }),
                    ])
                    .expect("serialize"),
                );
        })
        .await;

    server
        .mock_async(|when, then| {
            when.method(POST)
                .header("content-type", "application/json")
                .body_contains("eth_getTransactionReceipt");
            then.status(200)
                .header("content-type", "application/json")
                .body(
                    serde_json::to_string(&vec![
                        serde_json::json!({
                            "id": 10000,
                            "result": {
                                "transactionHash": "0xaaa",
                                "status": "0x1",
                                "gasUsed": "0x5208",
                                "logs": []
                            },
                            "error": null
                        }),
                        serde_json::json!({
                            "id": 10001,
                            "result": {
                                "transactionHash": "0xbbb",
                                "status": "0x1",
                                "gasUsed": "0x5208",
                                "logs": []
                            },
                            "error": null
                        }),
                    ])
                    .expect("serialize"),
                );
        })
        .await;

    server
        .mock_async(|when, then| {
            when.method(POST)
                .header("content-type", "application/json")
                .body_contains("\"method\":\"eth_getLogs\"");
            then.status(200)
                .header("content-type", "application/json")
                .body(
                    serde_json::to_string(&serde_json::json!({
                        "id": 20000,
                        "result": [
                            {
                                "transactionHash": "0xaaa",
                                "address": "0x222",
                                "topics": ["0xddf252ad"],
                                "blockNumber": "0x9"
                            },
                            {
                                "transactionHash": "0xbbb",
                                "address": "0x444",
                                "topics": ["0x8c5be1e5"],
                                "blockNumber": "0xa"
                            },
                            {
                                "transactionHash": "0xbbb",
                                "address": "0x444",
                                "topics": ["0xddf252ad"],
                                "blockNumber": "0xa"
                            }
                        ],
                        "error": null
                    }))
                    .expect("serialize"),
                );
        })
        .await;

    server
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn provider_pool_ingests_recent_window() {
        let server = start_mock_evm_server().await;

        let pool = EvmProviderPool::new(vec![server.url("/")]).expect("pool");
        let window = pool.ingest_recent_window(2).await.expect("window");
        assert_eq!(window.blocks.len(), 2);
        assert_eq!(window.receipts.len(), 2);
        assert_eq!(window.logs.len(), 3);
    }
}
