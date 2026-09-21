//! Real EVM contract deployment: compiles a contract with the real Solidity
//! compiler, then signs and submits a genuine contract-creation transaction
//! to a JSON-RPC node, waiting for it to be mined.
//!
//! Every value this module returns (address, tx hash, block number, gas
//! used) comes from the node's own transaction receipt -- never computed or
//! guessed locally -- because only the node knows what actually landed on
//! chain.
//!
//! This crate has no binary target and nothing in the repo currently calls
//! its (synchronous) public API from within a running Tokio runtime -- the
//! one real UI entry point, apps/x3-desktop's `foundry_deploy` Tauri
//! command, does not yet call into this crate at all (see #110 item 3). So
//! [`Deployer::deploy_contracts`](crate::deployer::Deployer::deploy_contracts)
//! bridges to this module's async functions with a short-lived
//! current-thread runtime. If a caller ever needs to invoke deployment from
//! inside an existing async context, that bridge needs to change to an
//! injected runtime handle instead (starting a runtime inside a runtime
//! panics).

use crate::error::FoundryError;
use ethers::prelude::*;
use ethers::types::transaction::eip2718::TypedTransaction;
use std::str::FromStr;

/// Resolves a `chain` name to a JSON-RPC URL.
///
/// A literal `http://`/`https://` value is used as-is (this is how tests
/// point deployment at a local `anvil` instance). Named chains resolve via
/// environment variables, mirroring apps/x3-desktop's `chain_rpc` module so
/// the same override conventions work across both.
pub fn resolve_rpc_url(chain: &str) -> String {
    if chain.starts_with("http://") || chain.starts_with("https://") {
        return chain.to_string();
    }
    match chain {
        "ethereum" | "ethereum-mainnet" => std::env::var("ETH_RPC_URL")
            .unwrap_or_else(|_| "https://eth-mainnet.g.alchemy.com/v2/demo".to_string()),
        "x3-testnet" | "x3-mainnet" | "x3" | "x3-chain" => std::env::var("X3_NODE_RPC")
            .unwrap_or_else(|_| "http://rpc.testnet.x3-chain.io:9944".to_string()),
        _ => std::env::var("LOCAL_RPC_URL").unwrap_or_else(|_| "http://127.0.0.1:8545".to_string()),
    }
}

/// The real, node-confirmed outcome of a contract-creation transaction.
pub struct DeployedTx {
    pub address: String,
    pub tx_hash: String,
    pub block_number: u64,
    pub gas_used: u64,
}

/// Signs and submits a contract-creation transaction carrying `bytecode`,
/// waits for it to be mined, and returns the receipt's own address, hash,
/// block number, and gas used.
///
/// `private_key` must be a real hex-encoded secp256k1 key (with or without
/// a `0x` prefix); this is a hard requirement now that deployment is real,
/// not a placeholder string.
pub async fn deploy_bytecode(
    rpc_url: &str,
    private_key: &str,
    bytecode: Vec<u8>,
) -> Result<DeployedTx, FoundryError> {
    let key_hex = private_key
        .trim_start_matches("0x")
        .trim_start_matches("0X");
    let wallet = LocalWallet::from_str(key_hex).map_err(|e| {
        FoundryError::DeploymentFailed(format!("invalid deployer private key: {e}"))
    })?;

    let provider = Provider::<Http>::try_from(rpc_url)
        .map_err(|e| FoundryError::DeploymentFailed(format!("bad RPC URL `{rpc_url}`: {e}")))?;

    let chain_id = provider
        .get_chainid()
        .await
        .map_err(|e| FoundryError::DeploymentFailed(format!("could not reach {rpc_url}: {e}")))?
        .as_u64();
    let wallet = wallet.with_chain_id(chain_id);
    let from = wallet.address();

    let nonce = provider
        .get_transaction_count(from, None)
        .await
        .map_err(|e| {
            FoundryError::DeploymentFailed(format!("could not fetch nonce for {from:?}: {e}"))
        })?;
    let gas_price = provider
        .get_gas_price()
        .await
        .map_err(|e| FoundryError::DeploymentFailed(format!("could not fetch gas price: {e}")))?;

    let mut typed_tx: TypedTransaction = TransactionRequest::new()
        .from(from)
        .data(bytecode)
        .nonce(nonce)
        .gas_price(gas_price)
        .chain_id(chain_id)
        .into();

    let gas_estimate = provider.estimate_gas(&typed_tx, None).await.map_err(|e| {
        FoundryError::DeploymentFailed(format!(
            "gas estimation failed (the deployment would likely revert): {e}"
        ))
    })?;
    typed_tx.set_gas(gas_estimate);

    let signature = wallet.sign_transaction(&typed_tx).await.map_err(|e| {
        FoundryError::DeploymentFailed(format!("failed to sign deployment transaction: {e}"))
    })?;
    let signed_tx = typed_tx.rlp_signed(&signature);

    let pending_tx = provider
        .send_raw_transaction(signed_tx)
        .await
        .map_err(|e| {
            FoundryError::DeploymentFailed(format!("failed to submit deployment transaction: {e}"))
        })?;
    let tx_hash = pending_tx.tx_hash();

    let receipt = pending_tx
        .await
        .map_err(|e| {
            FoundryError::DeploymentFailed(format!(
                "deployment transaction {tx_hash:?} errored while waiting to be mined: {e}"
            ))
        })?
        .ok_or_else(|| {
            FoundryError::DeploymentFailed(format!(
                "deployment transaction {tx_hash:?} was dropped or never mined"
            ))
        })?;

    if receipt.status != Some(1.into()) {
        return Err(FoundryError::DeploymentFailed(format!(
            "deployment transaction {tx_hash:?} reverted"
        )));
    }

    let address = receipt.contract_address.ok_or_else(|| {
        FoundryError::DeploymentFailed(format!(
            "transaction {tx_hash:?} succeeded but the node reported no contract address"
        ))
    })?;

    Ok(DeployedTx {
        address: format!("{address:?}"),
        tx_hash: format!("{:?}", receipt.transaction_hash),
        block_number: receipt.block_number.map(|b| b.as_u64()).unwrap_or(0),
        gas_used: receipt.gas_used.map(|g| g.as_u64()).unwrap_or(0),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_rpc_url_passes_through_literal_urls() {
        assert_eq!(
            resolve_rpc_url("http://127.0.0.1:9999"),
            "http://127.0.0.1:9999"
        );
        assert_eq!(
            resolve_rpc_url("https://example.com/rpc"),
            "https://example.com/rpc"
        );
    }

    #[test]
    fn test_resolve_rpc_url_unknown_chain_falls_back_to_local() {
        // Avoid depending on process-wide env state from other tests running
        // in parallel; only assert the shape when no override is set.
        if std::env::var("LOCAL_RPC_URL").is_err() {
            assert_eq!(
                resolve_rpc_url("some-unknown-chain"),
                "http://127.0.0.1:8545"
            );
        }
    }
}
