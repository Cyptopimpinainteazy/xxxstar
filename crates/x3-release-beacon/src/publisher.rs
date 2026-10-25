//! Publishes validator-signed X3 release beacons from the finalized chain head.

use crate::{
    exchange::{ReleaseBeaconExchangeClient, ReleaseBeaconSubmission},
    sign_attestation, ReleaseBeaconError,
};
use ed25519_dalek::SigningKey;
use serde::Deserialize;
use std::time::Duration;
use zeroize::Zeroize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FinalizedBlock {
    pub number: u64,
    pub hash: [u8; 32],
}

pub struct ReleaseBeaconSigner {
    attestor_id: String,
    signing_key: SigningKey,
}

impl ReleaseBeaconSigner {
    pub fn from_hex(
        attestor_id: impl Into<String>,
        seed_hex: &str,
    ) -> Result<Self, ReleaseBeaconError> {
        let attestor_id = attestor_id.into();
        if attestor_id.trim().is_empty() {
            return Err(ReleaseBeaconError::EmptyAttestorId);
        }
        let encoded = seed_hex
            .trim()
            .strip_prefix("0x")
            .unwrap_or(seed_hex.trim());
        let mut bytes = hex::decode(encoded).map_err(|error| {
            ReleaseBeaconError::Exchange(format!("invalid signer seed: {error}"))
        })?;
        if bytes.len() != 32 {
            return Err(ReleaseBeaconError::Exchange(format!(
                "release beacon signer seed is {} bytes, expected 32",
                bytes.len()
            )));
        }
        let mut seed = [0u8; 32];
        seed.copy_from_slice(&bytes);
        bytes.zeroize();
        let signing_key = SigningKey::from_bytes(&seed);
        let all_zero = seed.iter().all(|byte| *byte == 0);
        seed.zeroize();
        if all_zero {
            return Err(ReleaseBeaconError::Exchange(
                "release beacon signer seed must not be zero".into(),
            ));
        }
        Ok(Self {
            attestor_id: attestor_id.trim().to_string(),
            signing_key,
        })
    }

    pub fn attestor_id(&self) -> &str {
        &self.attestor_id
    }

    pub fn public_key(&self) -> [u8; 32] {
        self.signing_key.verifying_key().to_bytes()
    }

    pub fn sign(
        &self,
        block: FinalizedBlock,
    ) -> Result<ReleaseBeaconSubmission, ReleaseBeaconError> {
        if block.hash == [0u8; 32] {
            return Err(ReleaseBeaconError::Exchange(
                "refusing to sign a zero finalized block hash".into(),
            ));
        }
        Ok(ReleaseBeaconSubmission {
            block_number: block.number,
            block_hash: block.hash,
            attestation: sign_attestation(
                &self.attestor_id,
                &self.signing_key,
                block.number,
                block.hash,
            )?,
        })
    }
}

#[derive(Clone)]
pub struct X3FinalizedHeadClient {
    rpc_url: String,
    client: reqwest::Client,
}

impl X3FinalizedHeadClient {
    pub fn new(rpc_url: impl Into<String>) -> Result<Self, ReleaseBeaconError> {
        Ok(Self {
            rpc_url: rpc_url.into(),
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(15))
                .build()
                .map_err(|error| ReleaseBeaconError::Exchange(error.to_string()))?,
        })
    }

    pub async fn finalized_block(&self) -> Result<FinalizedBlock, ReleaseBeaconError> {
        let hash_value = self
            .call("chain_getFinalizedHead", serde_json::json!([]))
            .await?;
        let formatted = finalized_hash_string(&hash_value)?;
        let hash = decode_hash(&formatted)?;
        let header = self
            .call("chain_getHeader", serde_json::json!([formatted]))
            .await?;
        let number = finalized_header_number(&header)?;
        if hash == [0u8; 32] {
            return Err(ReleaseBeaconError::Exchange(
                "X3 RPC returned a zero finalized block hash".into(),
            ));
        }
        Ok(FinalizedBlock { number, hash })
    }

    async fn call(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, ReleaseBeaconError> {
        let response = self
            .client
            .post(&self.rpc_url)
            .json(&serde_json::json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": method,
                "params": params,
            }))
            .send()
            .await
            .map_err(|error| ReleaseBeaconError::Exchange(error.to_string()))?;
        if !response.status().is_success() {
            return Err(ReleaseBeaconError::Exchange(format!(
                "X3 RPC {method} returned HTTP {}",
                response.status()
            )));
        }
        let body: JsonRpcResponse = response
            .json()
            .await
            .map_err(|error| ReleaseBeaconError::Exchange(error.to_string()))?;
        if let Some(error) = body.error {
            return Err(ReleaseBeaconError::Exchange(format!(
                "X3 RPC {method} failed: {}",
                error.message
            )));
        }
        body.result.ok_or_else(|| {
            ReleaseBeaconError::Exchange(format!("X3 RPC {method} returned no result"))
        })
    }
}

#[derive(Deserialize)]
struct JsonRpcResponse {
    result: Option<serde_json::Value>,
    error: Option<JsonRpcError>,
}

#[derive(Deserialize)]
struct JsonRpcError {
    message: String,
}

fn finalized_hash_string(value: &serde_json::Value) -> Result<String, ReleaseBeaconError> {
    value
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| ReleaseBeaconError::Exchange("finalized head is not a hash string".into()))
}

fn finalized_header_number(value: &serde_json::Value) -> Result<u64, ReleaseBeaconError> {
    let number = value
        .get("number")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| ReleaseBeaconError::Exchange("finalized header has no number".into()))?;
    u64::from_str_radix(number.trim_start_matches("0x"), 16).map_err(|error| {
        ReleaseBeaconError::Exchange(format!("invalid finalized header number: {error}"))
    })
}

fn decode_hash(value: &str) -> Result<[u8; 32], ReleaseBeaconError> {
    let trimmed = value.trim().strip_prefix("0x").unwrap_or(value.trim());
    let bytes = hex::decode(trimmed)
        .map_err(|error| ReleaseBeaconError::Exchange(format!("invalid block hash: {error}")))?;
    bytes.try_into().map_err(|bytes: Vec<u8>| {
        ReleaseBeaconError::Exchange(format!(
            "finalized block hash is {} bytes, expected 32",
            bytes.len()
        ))
    })
}

pub async fn run_release_beacon_publisher(
    signer: ReleaseBeaconSigner,
    rpc_url: String,
    exchange_url: String,
    poll_interval: Duration,
) {
    let rpc = match X3FinalizedHeadClient::new(rpc_url) {
        Ok(rpc) => rpc,
        Err(error) => {
            eprintln!("could not construct X3 RPC client: {error}");
            return;
        }
    };
    let exchange = match ReleaseBeaconExchangeClient::new(exchange_url) {
        Ok(exchange) => exchange,
        Err(error) => {
            eprintln!("could not construct release beacon exchange client: {error}");
            return;
        }
    };
    let mut last_published = None;
    loop {
        match rpc.finalized_block().await {
            Ok(block) if last_published != Some((block.number, block.hash)) => {
                match signer.sign(block) {
                    Ok(submission) => match exchange.publish(&submission).await {
                        Ok(result) => {
                            println!(
                                "published release beacon {}:{} as {} ({} attestations)",
                                block.number,
                                hex::encode(block.hash),
                                signer.attestor_id(),
                                result.attestations
                            );
                            last_published = Some((block.number, block.hash));
                        }
                        Err(error) => {
                            eprintln!("release beacon exchange rejected publication: {error}")
                        }
                    },
                    Err(error) => eprintln!("release beacon signing failed: {error}"),
                }
            }
            Ok(_) => {}
            Err(error) => eprintln!("could not read X3 finalized head: {error}"),
        }
        tokio::time::sleep(poll_interval).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signer_binds_finalized_height_and_hash() {
        let signer = ReleaseBeaconSigner::from_hex("validator-1", &hex::encode([1u8; 32])).unwrap();
        let submission = signer
            .sign(FinalizedBlock {
                number: 77,
                hash: [0x91; 32],
            })
            .unwrap();
        crate::verify_attestation(
            "validator-1",
            77,
            [0x91; 32],
            &submission.attestation.public_key,
            &submission.attestation.signature,
        )
        .unwrap();
        assert!(signer
            .sign(FinalizedBlock {
                number: 77,
                hash: [0u8; 32],
            })
            .is_err());
    }

    #[test]
    fn finalized_header_number_is_parsed_as_chain_hex() {
        assert_eq!(
            finalized_header_number(&serde_json::json!({"number": "0x4d"})).unwrap(),
            77
        );
        assert!(finalized_header_number(&serde_json::json!({})).is_err());
    }
}
