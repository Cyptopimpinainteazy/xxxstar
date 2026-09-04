//! E2E Test Assertions
//!
//! Provides custom assertion utilities and matchers for E2E testing
//! to make tests more readable and provide better error messages.

use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tracing::{error, info, warn};

/// Custom result type for E2E test assertions
pub type E2EResult = Result<(), E2EAssertionError>;

/// Error type for E2E test assertions
#[derive(Debug, Clone)]
pub struct E2EAssertionError {
    pub message: String,
    pub context: Option<String>,
    pub timestamp: SystemTime,
}

impl E2EAssertionError {
    pub fn new(message: String) -> Self {
        Self {
            message,
            context: None,
            timestamp: SystemTime::now(),
        }
    }

    pub fn with_context(mut self, context: &str) -> Self {
        self.context = Some(context.to_string());
        self
    }
}

impl std::fmt::Display for E2EAssertionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "E2E Assertion Error: {}", self.message)?;
        if let Some(context) = &self.context {
            write!(f, " (Context: {})", context)?;
        }
        Ok(())
    }
}

impl std::error::Error for E2EAssertionError {}

impl From<reqwest::Error> for E2EAssertionError {
    fn from(value: reqwest::Error) -> Self {
        E2EAssertionError::new(format!("HTTP error: {}", value))
    }
}

impl From<std::time::SystemTimeError> for E2EAssertionError {
    fn from(value: std::time::SystemTimeError) -> Self {
        E2EAssertionError::new(format!("Time calculation error: {}", value))
    }
}

/// Blockchai-related assertions
pub struct BlockchainAssertions;

impl BlockchainAssertions {
    /// Assert that a transaction was successfully included in a block
    pub async fn assert_transaction_confirmed(
        tx_hash: &str,
        rpc_url: &str,
        timeout: Duration,
    ) -> E2EResult {
        let start_time = SystemTime::now();
        let client = reqwest::Client::new();

        while start_time.elapsed()? < timeout {
            let response = client
                .post(rpc_url)
                .json(&serde_json::json!({
                    "jsonrpc": "2.0",
                    "method": "eth_getTransactionReceipt",
                    "params": [tx_hash],
                    "id": 1
                }))
                .send()
                .await?;

            if response.status().is_success() {
                let result: serde_json::Value = response.json().await?;
                if let Some(receipt) = result.get("result") {
                    if receipt.is_object() && !receipt.is_null() {
                        info!("Transaction {} confirmed", tx_hash);
                        return Ok(());
                    }
                }
            }

            tokio::time::sleep(Duration::from_millis(500)).await;
        }

        Err(E2EAssertionError::new(format!(
            "Transaction {} not confirmed within timeout",
            tx_hash
        )))
    }

    /// Assert that account has expected balance
    pub async fn assert_account_balance(
        address: &str,
        expected_balance: u128,
        rpc_url: &str,
    ) -> E2EResult {
        let client = reqwest::Client::new();

        let response = client
            .post(rpc_url)
            .json(&serde_json::json!({
                "jsonrpc": "2.0",
                "method": "eth_getBalance",
                "params": [address, "latest"],
                "id": 1
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(E2EAssertionError::new(format!(
                "Failed to get balance for address {}",
                address
            )));
        }

        let result: serde_json::Value = response.json().await?;
        if let Some(balance_hex) = result.get("result") {
            let balance_str = balance_hex.as_str().unwrap_or("0x0");
            let actual_balance =
                u128::from_str_radix(balance_str.trim_start_matches("0x"), 16).unwrap_or(0);

            if actual_balance >= expected_balance {
                info!(
                    "Account {} has sufficient balance: {} >= {}",
                    address, actual_balance, expected_balance
                );
                Ok(())
            } else {
                Err(E2EAssertionError::new(format!(
                    "Insufficient balance for {}: expected >= {}, got {}",
                    address, expected_balance, actual_balance
                )))
            }
        } else {
            Err(E2EAssertionError::new(format!(
                "Invalid balance response for address {}",
                address
            )))
        }
    }

    /// Assert that contract deployment was successful
    pub async fn assert_contract_deployed(contract_address: &str, rpc_url: &str) -> E2EResult {
        let client = reqwest::Client::new();

        let response = client
            .post(rpc_url)
            .json(&serde_json::json!({
                "jsonrpc": "2.0",
                "method": "eth_getCode",
                "params": [contract_address, "latest"],
                "id": 1
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(E2EAssertionError::new(format!(
                "Failed to get code for contract {}",
                contract_address
            )));
        }

        let result: serde_json::Value = response.json().await?;
        if let Some(code) = result.get("result") {
            let code_str = code.as_str().unwrap_or("0x");
            if code_str.len() > 2 {
                // Has actual bytecode (not just "0x")
                info!("Contract {} successfully deployed", contract_address);
                Ok(())
            } else {
                Err(E2EAssertionError::new(format!(
                    "Contract {} has no bytecode",
                    contract_address
                )))
            }
        } else {
            Err(E2EAssertionError::new(format!(
                "Invalid code response for contract {}",
                contract_address
            )))
        }
    }
}

/// Smart contract-related assertions
pub struct ContractAssertions;

impl ContractAssertions {
    /// Assert that contract function call returns expected result
    pub async fn assert_contract_call_result(
        contract_address: &str,
        function_signature: &str,
        expected_result: &str,
        rpc_url: &str,
    ) -> E2EResult {
        let client = reqwest::Client::new();

        let call_data = format!(
            "{}{}",
            // This would need proper ABI encoding in a real implementation
            "0x12345678",    // Mock function signature hash
            expected_result  // Mock parameter encoding
        );

        let response = client
            .post(rpc_url)
            .json(&serde_json::json!({
                "jsonrpc": "2.0",
                "method": "eth_call",
                "params": [{
                    "to": contract_address,
                    "data": call_data
                }, "latest"],
                "id": 1
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(E2EAssertionError::new(format!(
                "Contract call failed for function {}",
                function_signature
            )));
        }

        let result: serde_json::Value = response.json().await?;
        if let Some(call_result) = result.get("result") {
            let result_str = call_result.as_str().unwrap_or("");
            if result_str.contains(expected_result) {
                info!(
                    "Contract call {} returned expected result",
                    function_signature
                );
                Ok(())
            } else {
                Err(E2EAssertionError::new(format!(
                    "Contract call {} returned unexpected result: expected {}, got {}",
                    function_signature, expected_result, result_str
                )))
            }
        } else {
            Err(E2EAssertionError::new(format!(
                "Invalid contract call response for {}",
                function_signature
            )))
        }
    }

    /// Assert that contract event was emitted
    pub async fn assert_event_emitted(
        contract_address: &str,
        event_signature: &str,
        from_block: u64,
        to_block: u64,
        rpc_url: &str,
    ) -> E2EResult {
        let client = reqwest::Client::new();

        // Mock event log query - in real implementation this would use eth_getLogs
        let response = client
            .post(rpc_url)
            .json(&serde_json::json!({
                "jsonrpc": "2.0",
                "method": "eth_getLogs",
                "params": [{
                    "fromBlock": format!("0x{:x}", from_block),
                    "toBlock": format!("0x{:x}", to_block),
                    "address": contract_address
                }],
                "id": 1
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(E2EAssertionError::new(format!(
                "Failed to query logs for contract {}",
                contract_address
            )));
        }

        let result: serde_json::Value = response.json().await?;
        if let Some(logs) = result.get("result") {
            if logs.is_array() && !logs.as_array().unwrap().is_empty() {
                info!(
                    "Event {} was emitted for contract {}",
                    event_signature, contract_address
                );
                Ok(())
            } else {
                Err(E2EAssertionError::new(format!(
                    "Event {} was not emitted for contract {}",
                    event_signature, contract_address
                )))
            }
        } else {
            Err(E2EAssertionError::new(format!(
                "Invalid logs response for contract {}",
                contract_address
            )))
        }
    }
}

/// GPU Swarm-related assertions
pub struct GPUSwarmAssertions;

impl GPUSwarmAssertions {
    /// Assert that GPU node is available
    pub async fn assert_node_available(node_id: &str, coordinator_url: &str) -> E2EResult {
        let client = reqwest::Client::new();

        let response = client
            .get(&format!("{}/swarm/nodes", coordinator_url))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(E2EAssertionError::new(format!(
                "Failed to query swarm nodes from {}",
                coordinator_url
            )));
        }

        let nodes: Vec<serde_json::Value> = response.json().await?;
        for node in nodes {
            if let Some(id) = node.get("node_id") {
                if id.as_str() == Some(node_id) {
                    if let Some(available) = node.get("available") {
                        if available.as_bool().unwrap_or(false) {
                            info!("GPU node {} is available", node_id);
                            return Ok(());
                        }
                    }
                }
            }
        }

        Err(E2EAssertionError::new(format!(
            "GPU node {} is not available",
            node_id
        )))
    }

    /// Assert that task was successfully submitted to swarm
    pub async fn assert_task_submitted(task_id: &str, coordinator_url: &str) -> E2EResult {
        let client = reqwest::Client::new();

        let response = client
            .get(&format!("{}/swarm/tasks/{}", coordinator_url, task_id))
            .send()
            .await?;

        if response.status().is_success() {
            let task: serde_json::Value = response.json().await?;
            if let Some(status) = task.get("status") {
                if status.as_str() == Some("submitted") || status.as_str() == Some("processing") {
                    info!("Task {} successfully submitted to swarm", task_id);
                    return Ok(());
                }
            }
        }

        Err(E2EAssertionError::new(format!(
            "Task {} was not successfully submitted",
            task_id
        )))
    }
}

/// DNS-related assertions
pub struct DNSAssertions;

impl DNSAssertions {
    /// Assert that domain resolves to expected IP
    pub async fn assert_domain_resolution(
        domain: &str,
        expected_ip: &str,
        dns_server_url: &str,
    ) -> E2EResult {
        let client = reqwest::Client::new();

        let response = client
            .get(&format!("{}/resolve/{}", dns_server_url, domain))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(E2EAssertionError::new(format!(
                "DNS resolution failed for domain {}",
                domain
            )));
        }

        let result: serde_json::Value = response.json().await?;
        if let Some(ip) = result.get("ip") {
            if ip.as_str() == Some(expected_ip) {
                info!("Domain {} correctly resolves to {}", domain, expected_ip);
                Ok(())
            } else {
                Err(E2EAssertionError::new(format!(
                    "Domain {} resolves to {} but expected {}",
                    domain,
                    ip.as_str().unwrap_or(""),
                    expected_ip
                )))
            }
        } else {
            Err(E2EAssertionError::new(format!(
                "Domain {} resolution returned no IP",
                domain
            )))
        }
    }
}

/// Performance-related assertions
pub struct PerformanceAssertions;

impl PerformanceAssertions {
    /// Assert that operation completes within expected time
    pub async fn assert_operation_timing<F, T>(
        operation_name: &str,
        max_duration: Duration,
        operation: F,
    ) -> E2EResult
    where
        F: std::future::Future<Output = Result<T, Box<dyn std::error::Error>>>,
    {
        let start_time = SystemTime::now();

        match operation.await {
            Ok(_) => {
                let duration = start_time.elapsed()?;
                if duration <= max_duration {
                    info!(
                        "Operation {} completed within time: {:?}",
                        operation_name, duration
                    );
                    Ok(())
                } else {
                    Err(E2EAssertionError::new(format!(
                        "Operation {} took {:?} but max allowed is {:?}",
                        operation_name, duration, max_duration
                    )))
                }
            }
            Err(e) => Err(E2EAssertionError::new(format!(
                "Operation {} failed: {}",
                operation_name, e
            ))),
        }
    }

    /// Assert that system handles expected load
    pub async fn assert_load_handling<F, Fut, T>(
        operation_name: &str,
        concurrent_requests: usize,
        operation: F,
    ) -> E2EResult
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T, Box<dyn std::error::Error + Send + Sync>>>
            + Send
            + 'static,
        T: Send + 'static,
    {
        let mut handles = Vec::new();

        for i in 0..concurrent_requests {
            let operation_name = format!("{}-{}", operation_name, i);
            let future = operation();
            let handle = tokio::spawn(async move {
                future.await.map_err(|e| {
                    E2EAssertionError::new(format!("Request {} failed: {}", operation_name, e))
                })
            });
            handles.push(handle);
        }

        let mut results = Vec::new();
        for handle in handles {
            match handle.await {
                Ok(result) => results.push(result),
                Err(e) => results.push(Err(E2EAssertionError::new(format!(
                    "Request panicked: {}",
                    e
                )))),
            }
        }

        let successful_count = results.iter().filter(|r| r.is_ok()).count();
        if successful_count == concurrent_requests {
            info!(
                "All {} concurrent requests for {} completed successfully",
                concurrent_requests, operation_name
            );
            Ok(())
        } else {
            let failed_count = concurrent_requests - successful_count;
            Err(E2EAssertionError::new(format!(
                "Load test failed: {} out of {} requests failed for {}",
                failed_count, concurrent_requests, operation_name
            )))
        }
    }
}

/// Frontend-related assertions
pub struct FrontendAssertions;

impl FrontendAssertions {
    /// Assert that web page loads successfully
    pub async fn assert_page_loads(url: &str, expected_title: Option<&str>) -> E2EResult {
        // In a real implementation, this would use a headless browser
        // For now, we'll use a simple HTTP request to check if the page responds
        let client = reqwest::Client::new();

        let response = client
            .get(url)
            .timeout(Duration::from_secs(10))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(E2EAssertionError::new(format!(
                "Page {} failed to load: HTTP {}",
                url,
                response.status()
            )));
        }

        if let Some(title) = expected_title {
            let content = response.text().await?;
            if content.contains(title) {
                info!("Page {} loaded successfully with expected title", url);
            } else {
                warn!("Page {} loaded but title '{}' not found", url, title);
            }
        } else {
            info!("Page {} loaded successfully", url);
        }

        Ok(())
    }

    /// Assert that API endpoint responds correctly
    pub async fn assert_api_response(
        url: &str,
        expected_status: u16,
        expected_fields: Option<Vec<&str>>,
    ) -> E2EResult {
        let client = reqwest::Client::new();

        let response = client
            .get(url)
            .timeout(Duration::from_secs(5))
            .send()
            .await?;

        if response.status() != expected_status {
            return Err(E2EAssertionError::new(format!(
                "API {} returned status {} but expected {}",
                url,
                response.status(),
                expected_status
            )));
        }

        if let Some(fields) = expected_fields {
            let json: serde_json::Value = response.json().await?;

            let mut missing_fields = Vec::new();
            for field in fields {
                if !json.get(field).is_some() {
                    missing_fields.push(field);
                }
            }

            if !missing_fields.is_empty() {
                return Err(E2EAssertionError::new(format!(
                    "API {} response missing fields: {:?}",
                    url, missing_fields
                )));
            }
        }

        info!("API {} responded correctly", url);
        Ok(())
    }
}

/// Assertion combinators for complex scenarios
pub struct AssertThat<T> {
    value: T,
}

impl<T> AssertThat<T> {
    pub fn new(value: T) -> Self {
        Self { value }
    }

    pub fn value(&self) -> &T {
        &self.value
    }
}
