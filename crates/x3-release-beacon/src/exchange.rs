//! Authenticated, durable exchange for independently signed X3 beacons.
//!
//! The exchange is evidence transport, not the threshold authority. It validates
//! that every accepted attestation is signed by a configured key, refuses
//! equivocation, persists only verified evidence, and returns the complete set
//! for the coordinator's `ReleaseBeaconPolicy` to evaluate.

use crate::{
    verify_attestation, ReleaseBeaconAttestation, ReleaseBeaconError, ReleaseBeaconEvidence,
    RELEASE_BEACON_PUBLIC_KEY_LEN,
};
use axum::{
    extract::{DefaultBodyLimit, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::OpenOptions,
    io::{ErrorKind, Write},
    net::SocketAddr,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};
use tokio::{net::TcpListener, sync::RwLock};

const MAX_BEACON_BODY_BYTES: usize = 64 * 1024;
const STORE_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReleaseBeaconSubmission {
    pub block_number: u64,
    pub block_hash: [u8; 32],
    pub attestation: ReleaseBeaconAttestation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReleaseBeaconQuery {
    pub block_number: u64,
    pub block_hash: [u8; 32],
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReleaseBeaconPublishResult {
    pub accepted: bool,
    pub attestations: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReleaseBeaconExchangeError {
    pub error: String,
}

#[derive(Debug)]
pub struct ReleaseBeaconExchange {
    path: PathBuf,
    authorized: BTreeSet<[u8; RELEASE_BEACON_PUBLIC_KEY_LEN]>,
    records: RwLock<Vec<PersistedBeacon>>,
}

impl ReleaseBeaconExchange {
    /// Open or create an exchange store and refuse any persisted record that no
    /// longer verifies against the configured authority set.
    pub fn open(
        path: impl AsRef<Path>,
        authorized: impl IntoIterator<Item = [u8; RELEASE_BEACON_PUBLIC_KEY_LEN]>,
    ) -> Result<Self, ReleaseBeaconError> {
        let path = path.as_ref().to_path_buf();
        let authorized: BTreeSet<_> = authorized.into_iter().collect();
        if authorized.is_empty()
            || authorized
                .iter()
                .any(|key| key.iter().all(|byte| *byte == 0))
        {
            return Err(ReleaseBeaconError::InvalidPolicy);
        }
        let records = load_store(&path, &authorized)?;
        Ok(Self {
            path,
            authorized,
            records: RwLock::new(records),
        })
    }

    pub fn authorized_attestors(&self) -> &BTreeSet<[u8; RELEASE_BEACON_PUBLIC_KEY_LEN]> {
        &self.authorized
    }

    /// Validate and durably record one attestation.
    pub async fn publish(
        &self,
        submission: ReleaseBeaconSubmission,
    ) -> Result<ReleaseBeaconPublishResult, ReleaseBeaconError> {
        validate_submission(&submission, &self.authorized)?;

        let mut records = self.records.write().await;
        let mut next_records = records.clone();
        let key = submission.attestation.public_key;
        let mut equivocated = false;
        for record in next_records.iter_mut().filter(|record| {
            record.block_number == submission.block_number
                && record.block_hash != submission.block_hash
        }) {
            let before = record.attestations.len();
            record
                .attestations
                .retain(|attestation| attestation.public_key != key);
            equivocated |= record.attestations.len() != before;
        }
        next_records.retain(|record| !record.attestations.is_empty());
        if equivocated {
            persist_store(&self.path, &next_records)?;
            *records = next_records;
            return Err(ReleaseBeaconError::Exchange(format!(
                "attestor {} equivocated at X3 height {}; its prior attestation was removed",
                submission.attestation.attestor_id, submission.block_number
            )));
        }

        let position = next_records.iter().position(|record| {
            record.block_number == submission.block_number
                && record.block_hash == submission.block_hash
        });
        let index = match position {
            Some(index) => index,
            None => {
                next_records.push(PersistedBeacon {
                    block_number: submission.block_number,
                    block_hash: submission.block_hash,
                    attestations: Vec::new(),
                });
                next_records.len() - 1
            }
        };
        let attestation_count = {
            let record = &mut next_records[index];
            if let Some(existing) = record
                .attestations
                .iter()
                .find(|attestation| attestation.public_key == key)
            {
                if existing == &submission.attestation {
                    return Ok(ReleaseBeaconPublishResult {
                        accepted: true,
                        attestations: record.attestations.len(),
                    });
                }
                return Err(ReleaseBeaconError::Exchange(
                    "one attestor key submitted conflicting evidence for the same beacon".into(),
                ));
            }
            record.attestations.push(submission.attestation);
            record
                .attestations
                .sort_by(|left, right| left.public_key.cmp(&right.public_key));
            record.attestations.len()
        };
        persist_store(&self.path, &next_records)?;
        *records = next_records;
        Ok(ReleaseBeaconPublishResult {
            accepted: true,
            attestations: attestation_count,
        })
    }

    /// Return every verified attestation for exactly this height and hash.
    pub async fn evidence(&self, query: ReleaseBeaconQuery) -> ReleaseBeaconEvidence {
        let records = self.records.read().await;
        let attestations = records
            .iter()
            .find(|record| {
                record.block_number == query.block_number && record.block_hash == query.block_hash
            })
            .map(|record| record.attestations.clone())
            .unwrap_or_default();
        ReleaseBeaconEvidence {
            block_number: query.block_number,
            block_hash: query.block_hash,
            attestations,
        }
    }
}

fn validate_submission(
    submission: &ReleaseBeaconSubmission,
    authorized: &BTreeSet<[u8; RELEASE_BEACON_PUBLIC_KEY_LEN]>,
) -> Result<(), ReleaseBeaconError> {
    if !authorized.contains(&submission.attestation.public_key) {
        return Err(ReleaseBeaconError::UnauthorizedAttestor);
    }
    if submission.block_hash == [0u8; 32] {
        return Err(ReleaseBeaconError::Exchange(
            "release beacon block hash must not be zero".into(),
        ));
    }
    verify_attestation(
        &submission.attestation.attestor_id,
        submission.block_number,
        submission.block_hash,
        &submission.attestation.public_key,
        &submission.attestation.signature,
    )
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct PersistedBeacon {
    block_number: u64,
    block_hash: [u8; 32],
    attestations: Vec<ReleaseBeaconAttestation>,
}

#[derive(Debug, Serialize, Deserialize)]
struct PersistedStore {
    version: u32,
    beacons: Vec<PersistedBeacon>,
}

fn load_store(
    path: &Path,
    authorized: &BTreeSet<[u8; RELEASE_BEACON_PUBLIC_KEY_LEN]>,
) -> Result<Vec<PersistedBeacon>, ReleaseBeaconError> {
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => {
            return Err(ReleaseBeaconError::Exchange(format!(
                "could not read {}: {error}",
                path.display()
            )))
        }
    };
    let mut store: PersistedStore = serde_json::from_slice(&bytes)
        .map_err(|error| ReleaseBeaconError::Exchange(error.to_string()))?;
    if store.version != STORE_VERSION {
        return Err(ReleaseBeaconError::Exchange(format!(
            "unsupported release beacon store version {}",
            store.version
        )));
    }

    let mut keys_at_height: BTreeMap<u64, BTreeMap<[u8; 32], BTreeSet<[u8; 32]>>> = BTreeMap::new();
    for record in &mut store.beacons {
        if record.block_hash == [0u8; 32] {
            return Err(ReleaseBeaconError::Exchange(
                "persisted release beacon has a zero block hash".into(),
            ));
        }
        let mut seen = BTreeSet::new();
        for attestation in &record.attestations {
            if !authorized.contains(&attestation.public_key) {
                return Err(ReleaseBeaconError::UnauthorizedAttestor);
            }
            if !seen.insert(attestation.public_key)
                || verify_attestation(
                    &attestation.attestor_id,
                    record.block_number,
                    record.block_hash,
                    &attestation.public_key,
                    &attestation.signature,
                )
                .is_err()
            {
                return Err(ReleaseBeaconError::Exchange(format!(
                    "persisted release beacon at {}:{} contains an invalid or duplicate attestation",
                    record.block_number,
                    hex::encode(record.block_hash)
                )));
            }
            let keys = keys_at_height
                .entry(record.block_number)
                .or_default()
                .entry(record.block_hash)
                .or_default();
            if !keys.insert(attestation.public_key) {
                return Err(ReleaseBeaconError::Exchange(format!(
                    "persisted release beacon at {}:{} repeats an attestor key",
                    record.block_number,
                    hex::encode(record.block_hash)
                )));
            }
        }
        record
            .attestations
            .sort_by(|left, right| left.public_key.cmp(&right.public_key));
    }

    for (height, hashes) in &keys_at_height {
        let mut by_key: BTreeMap<[u8; 32], usize> = BTreeMap::new();
        for keys in hashes.values() {
            for key in keys {
                *by_key.entry(*key).or_default() += 1;
            }
        }
        if let Some((key, _)) = by_key.into_iter().find(|(_, count)| *count > 1) {
            return Err(ReleaseBeaconError::Exchange(format!(
                "persisted store contains equivocation by key {} at height {height}",
                hex::encode(key)
            )));
        }
    }
    store
        .beacons
        .retain(|record| !record.attestations.is_empty());
    Ok(store.beacons)
}

fn persist_store(path: &Path, records: &[PersistedBeacon]) -> Result<(), ReleaseBeaconError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| {
            ReleaseBeaconError::Exchange(format!("could not create {}: {error}", parent.display()))
        })?;
    }
    static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);
    let temp = path.with_extension(format!(
        "tmp-{}-{}",
        std::process::id(),
        TEMP_COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    let store = PersistedStore {
        version: STORE_VERSION,
        beacons: records.to_vec(),
    };
    let bytes = serde_json::to_vec_pretty(&store)
        .map_err(|error| ReleaseBeaconError::Exchange(error.to_string()))?;
    let write_result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)?;
        file.write_all(&bytes)?;
        file.sync_all()
    })();
    if let Err(error) = write_result {
        let _ = std::fs::remove_file(&temp);
        return Err(ReleaseBeaconError::Exchange(format!(
            "could not durably write {}: {error}",
            temp.display()
        )));
    }
    std::fs::rename(&temp, path).map_err(|error| {
        let _ = std::fs::remove_file(&temp);
        ReleaseBeaconError::Exchange(format!("could not replace {}: {error}", path.display()))
    })?;
    if let Some(parent) = path.parent() {
        std::fs::File::open(parent)
            .and_then(|directory| directory.sync_all())
            .map_err(|error| {
                ReleaseBeaconError::Exchange(format!(
                    "could not sync directory {}: {error}",
                    parent.display()
                ))
            })?;
    }
    Ok(())
}

/// Axum router for the release beacon exchange.
pub fn release_beacon_router(exchange: Arc<ReleaseBeaconExchange>) -> Router {
    Router::new()
        .route("/v1/x3-release-beacons", post(publish_beacon))
        .route("/v1/x3-release-beacons/query", post(query_beacons))
        .layer(DefaultBodyLimit::max(MAX_BEACON_BODY_BYTES))
        .with_state(exchange)
}

pub async fn serve_release_beacon_exchange(
    listener: TcpListener,
    exchange: ReleaseBeaconExchange,
) -> std::io::Result<()> {
    axum::serve(listener, release_beacon_router(Arc::new(exchange))).await
}

async fn publish_beacon(
    State(exchange): State<Arc<ReleaseBeaconExchange>>,
    Json(submission): Json<ReleaseBeaconSubmission>,
) -> Response {
    match exchange.publish(submission).await {
        Ok(result) => Json(result).into_response(),
        Err(error @ ReleaseBeaconError::UnauthorizedAttestor)
        | Err(error @ ReleaseBeaconError::InvalidSignature)
        | Err(error @ ReleaseBeaconError::InvalidPublicKey)
        | Err(error @ ReleaseBeaconError::InvalidSignatureLength { .. })
        | Err(error @ ReleaseBeaconError::EmptyAttestorId) => (
            StatusCode::UNAUTHORIZED,
            Json(ReleaseBeaconExchangeError {
                error: error.to_string(),
            }),
        )
            .into_response(),
        Err(error @ ReleaseBeaconError::Exchange(_)) => (
            StatusCode::CONFLICT,
            Json(ReleaseBeaconExchangeError {
                error: error.to_string(),
            }),
        )
            .into_response(),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(ReleaseBeaconExchangeError {
                error: error.to_string(),
            }),
        )
            .into_response(),
    }
}

async fn query_beacons(
    State(exchange): State<Arc<ReleaseBeaconExchange>>,
    Json(query): Json<ReleaseBeaconQuery>,
) -> Json<ReleaseBeaconEvidence> {
    Json(exchange.evidence(query).await)
}

#[derive(Clone)]
pub struct ReleaseBeaconExchangeClient {
    base_url: String,
    client: reqwest::Client,
}

impl ReleaseBeaconExchangeClient {
    pub fn new(base_url: impl Into<String>) -> Result<Self, ReleaseBeaconError> {
        Ok(Self {
            base_url: base_url.into().trim_end_matches('/').to_string(),
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(15))
                .build()
                .map_err(|error| ReleaseBeaconError::Exchange(error.to_string()))?,
        })
    }

    pub async fn publish(
        &self,
        submission: &ReleaseBeaconSubmission,
    ) -> Result<ReleaseBeaconPublishResult, ReleaseBeaconError> {
        let response = self
            .client
            .post(format!("{}/v1/x3-release-beacons", self.base_url))
            .json(submission)
            .send()
            .await
            .map_err(|error| ReleaseBeaconError::Exchange(error.to_string()))?;
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(ReleaseBeaconError::Exchange(format!(
                "exchange rejected submission with {status}: {body}"
            )));
        }
        response
            .json()
            .await
            .map_err(|error| ReleaseBeaconError::Exchange(error.to_string()))
    }

    pub async fn evidence(
        &self,
        block_number: u64,
        block_hash: [u8; 32],
    ) -> Result<ReleaseBeaconEvidence, ReleaseBeaconError> {
        let response = self
            .client
            .post(format!("{}/v1/x3-release-beacons/query", self.base_url))
            .json(&ReleaseBeaconQuery {
                block_number,
                block_hash,
            })
            .send()
            .await
            .map_err(|error| ReleaseBeaconError::Exchange(error.to_string()))?;
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(ReleaseBeaconError::Exchange(format!(
                "exchange query failed with {status}: {body}"
            )));
        }
        response
            .json()
            .await
            .map_err(|error| ReleaseBeaconError::Exchange(error.to_string()))
    }
}

pub async fn bind_exchange(address: SocketAddr) -> std::io::Result<TcpListener> {
    TcpListener::bind(address).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sign_attestation;
    use ed25519_dalek::SigningKey;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn key(seed: u8) -> SigningKey {
        SigningKey::from_bytes(&[seed; 32])
    }

    fn submission(seed: u8, block_hash: [u8; 32]) -> ReleaseBeaconSubmission {
        let key = key(seed);
        ReleaseBeaconSubmission {
            block_number: 77,
            block_hash,
            attestation: sign_attestation(&format!("validator-{seed}"), &key, 77, block_hash)
                .unwrap(),
        }
    }

    fn test_path(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "x3-release-beacon-{name}-{}-{nanos}.json",
            std::process::id()
        ))
    }

    #[tokio::test]
    async fn evidence_survives_restart_and_preserves_independent_attestations() {
        let path = test_path("restart");
        let authorized = vec![
            key(1).verifying_key().to_bytes(),
            key(2).verifying_key().to_bytes(),
        ];
        {
            let exchange = ReleaseBeaconExchange::open(&path, authorized.clone()).unwrap();
            exchange.publish(submission(1, [0x91; 32])).await.unwrap();
            exchange.publish(submission(2, [0x91; 32])).await.unwrap();
        }
        let exchange = ReleaseBeaconExchange::open(&path, authorized).unwrap();
        let evidence = exchange
            .evidence(ReleaseBeaconQuery {
                block_number: 77,
                block_hash: [0x91; 32],
            })
            .await;
        assert_eq!(evidence.attestations.len(), 2);
        assert_ne!(
            evidence.attestations[0].public_key,
            evidence.attestations[1].public_key
        );
        let _ = std::fs::remove_file(path);
    }

    #[tokio::test]
    async fn equivocation_is_refused_and_the_prior_vote_is_removed() {
        let path = test_path("equivocation");
        let exchange =
            ReleaseBeaconExchange::open(&path, [key(1).verifying_key().to_bytes()]).unwrap();
        exchange.publish(submission(1, [0x91; 32])).await.unwrap();
        assert!(exchange.publish(submission(1, [0x92; 32])).await.is_err());
        assert!(exchange
            .evidence(ReleaseBeaconQuery {
                block_number: 77,
                block_hash: [0x91; 32],
            })
            .await
            .attestations
            .is_empty());
        let _ = std::fs::remove_file(path);
    }

    #[tokio::test]
    async fn forged_or_unauthorized_attestations_are_not_persisted() {
        let path = test_path("unauthorized");
        let exchange =
            ReleaseBeaconExchange::open(&path, [key(1).verifying_key().to_bytes()]).unwrap();
        let mut forged = submission(1, [0x91; 32]);
        forged.attestation.signature[0] ^= 0xff;
        assert!(exchange.publish(forged).await.is_err());
        assert!(exchange.publish(submission(2, [0x91; 32])).await.is_err());
        assert!(exchange
            .evidence(ReleaseBeaconQuery {
                block_number: 77,
                block_hash: [0x91; 32],
            })
            .await
            .attestations
            .is_empty());
        let _ = std::fs::remove_file(path);
    }

    #[tokio::test]
    async fn real_http_client_round_trips_verified_evidence() {
        let path = test_path("http");
        let exchange =
            ReleaseBeaconExchange::open(&path, [key(1).verifying_key().to_bytes()]).unwrap();
        let listener = bind_exchange("127.0.0.1:0".parse().unwrap()).await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(serve_release_beacon_exchange(listener, exchange));
        let client = ReleaseBeaconExchangeClient::new(format!("http://{address}")).unwrap();
        let submission = submission(1, [0x91; 32]);
        client.publish(&submission).await.unwrap();
        let evidence = client.evidence(77, [0x91; 32]).await.unwrap();
        assert_eq!(evidence.attestations, vec![submission.attestation]);
        crate::ReleaseBeaconPolicy::new(1, vec![key(1).verifying_key().to_bytes()])
            .unwrap()
            .verify(&evidence)
            .unwrap();
        server.abort();
        let _ = std::fs::remove_file(path);
    }
}
