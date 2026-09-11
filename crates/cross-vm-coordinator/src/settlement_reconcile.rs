//! Deterministic chain reconciliation for settlement outbox records.
//!
//! The observer is deliberately abstract: RPC/subxt/node wiring can implement
//! it later. The decision table itself is pure and fail-closed.

use crate::{
    CoordinatorError, SettlementOutboxRecord, SettlementOutboxStatus,
};
use async_trait::async_trait;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettlementTxObservation {
    /// Authoritative query did not find the tx. For a Signed record this means
    /// it is safe to rebroadcast the exact same persisted signed bytes.
    Unknown,
    Pending,
    Included { block_number: u64 },
    Rejected { reason: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeSettlementObservation {
    Unknown,
    NonTerminal,
    Finalized,
    Refunded,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettlementReconcileDecision {
    SignPrepared,
    /// Never create a new transaction. Re-send only record.signed_extrinsic.
    RebroadcastExactSignedExtrinsic,
    WaitForInclusion,
    MarkIncluded { block_number: u64 },
    MarkFailed { reason: String },
    ObserveSettlementState,
    MarkTerminalObserved,
    Done,
    ManualHalt { reason: String },
}

#[async_trait]
pub trait SettlementChainObserver: Send + Sync {
    async fn transaction_status(
        &self,
        tx_id: &str,
    ) -> Result<SettlementTxObservation, CoordinatorError>;

    async fn settlement_state(
        &self,
        runtime_intent_id: [u8; 32],
    ) -> Result<RuntimeSettlementObservation, CoordinatorError>;
}

/// Pure reconciliation decision. No network calls and no state writes.
///
/// Purpose is encoded in the outbox as:
/// - 0 = Claim
/// - 1 = Refund
pub fn decide_settlement_reconciliation(
    record: &SettlementOutboxRecord,
    tx: Option<&SettlementTxObservation>,
    runtime: Option<RuntimeSettlementObservation>,
) -> SettlementReconcileDecision {
    match record.status {
        SettlementOutboxStatus::Prepared => SettlementReconcileDecision::SignPrepared,

        SettlementOutboxStatus::Signed | SettlementOutboxStatus::Broadcast => {
            let Some(tx) = tx else {
                return SettlementReconcileDecision::WaitForInclusion;
            };
            match tx {
                SettlementTxObservation::Unknown => {
                    if record.status == SettlementOutboxStatus::Signed
                        && record
                            .signed_extrinsic
                            .as_ref()
                            .is_some_and(|bytes| !bytes.is_empty())
                        && record.tx_id.is_some()
                    {
                        SettlementReconcileDecision::RebroadcastExactSignedExtrinsic
                    } else {
                        // A Broadcast tx disappearing is ambiguous: do not
                        // create or sign a replacement transaction.
                        SettlementReconcileDecision::WaitForInclusion
                    }
                }
                SettlementTxObservation::Pending => SettlementReconcileDecision::WaitForInclusion,
                SettlementTxObservation::Included { block_number } => {
                    SettlementReconcileDecision::MarkIncluded {
                        block_number: *block_number,
                    }
                }
                SettlementTxObservation::Rejected { reason } => {
                    SettlementReconcileDecision::MarkFailed {
                        reason: reason.clone(),
                    }
                }
            }
        }

        SettlementOutboxStatus::Included => {
            let Some(runtime) = runtime else {
                return SettlementReconcileDecision::ObserveSettlementState;
            };
            match (record.purpose, runtime) {
                (_, RuntimeSettlementObservation::Unknown)
                | (_, RuntimeSettlementObservation::NonTerminal) => {
                    SettlementReconcileDecision::ObserveSettlementState
                }
                (0, RuntimeSettlementObservation::Finalized)
                | (1, RuntimeSettlementObservation::Refunded) => {
                    SettlementReconcileDecision::MarkTerminalObserved
                }
                (0, RuntimeSettlementObservation::Refunded) => {
                    SettlementReconcileDecision::ManualHalt {
                        reason: "claim submission observed Refunded terminal state".into(),
                    }
                }
                (1, RuntimeSettlementObservation::Finalized) => {
                    SettlementReconcileDecision::ManualHalt {
                        reason: "refund submission observed Finalized terminal state".into(),
                    }
                }
                (_, RuntimeSettlementObservation::Finalized)
                | (_, RuntimeSettlementObservation::Refunded) => {
                    SettlementReconcileDecision::ManualHalt {
                        reason: "unknown settlement submission purpose at terminal state".into(),
                    }
                }
            }
        }

        SettlementOutboxStatus::TerminalObserved => SettlementReconcileDecision::Done,

        SettlementOutboxStatus::Failed => SettlementReconcileDecision::ManualHalt {
            reason: record
                .error
                .clone()
                .unwrap_or_else(|| "settlement submission is in Failed state".into()),
        },
    }
}

/// Query only the evidence required by the current durable outbox state and
/// return a pure reconciliation decision.
pub async fn observe_and_decide<O: SettlementChainObserver>(
    observer: &O,
    record: &SettlementOutboxRecord,
) -> Result<SettlementReconcileDecision, CoordinatorError> {
    match record.status {
        SettlementOutboxStatus::Signed | SettlementOutboxStatus::Broadcast => {
            let tx_id = record.tx_id.as_deref().ok_or_else(|| {
                CoordinatorError::Internal(
                    "signed/broadcast settlement record has no tx id".into(),
                )
            })?;
            let tx = observer.transaction_status(tx_id).await?;
            Ok(decide_settlement_reconciliation(record, Some(&tx), None))
        }
        SettlementOutboxStatus::Included => {
            let runtime = observer
                .settlement_state(record.runtime_intent_id)
                .await?;
            Ok(decide_settlement_reconciliation(
                record,
                None,
                Some(runtime),
            ))
        }
        _ => Ok(decide_settlement_reconciliation(record, None, None)),
    }
}



/// HTTP JSON-RPC observer backed by the canonical X3 node RPC.
///
/// Transaction lookup scans a bounded window of finalized blocks and compares
/// the exact Substrate extrinsic hash. A missing hash is deliberately reported
/// as Unknown rather than Rejected: after restart, absence from the bounded
/// finalized window is not enough evidence to manufacture a replacement tx.
#[derive(Clone)]
pub struct X3JsonRpcSettlementObserver {
    rpc_url: String,
    client: reqwest::Client,
    finalized_scan_depth: u64,
}

impl X3JsonRpcSettlementObserver {
    pub fn new(rpc_url: impl Into<String>) -> Self {
        Self {
            rpc_url: rpc_url.into(),
            client: reqwest::Client::new(),
            finalized_scan_depth: 256,
        }
    }

    pub fn with_finalized_scan_depth(
        rpc_url: impl Into<String>,
        finalized_scan_depth: u64,
    ) -> Self {
        Self {
            rpc_url: rpc_url.into(),
            client: reqwest::Client::new(),
            finalized_scan_depth: finalized_scan_depth.max(1),
        }
    }

    async fn rpc(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, CoordinatorError> {
        let response = self
            .client
            .post(&self.rpc_url)
            .json(&serde_json::json!({
                "jsonrpc": "2.0",
                "id": 1u64,
                "method": method,
                "params": params,
            }))
            .send()
            .await
            .map_err(|e| {
                CoordinatorError::Internal(format!(
                    "X3 reconciliation RPC {method} request failed: {e}"
                ))
            })?;

        if !response.status().is_success() {
            return Err(CoordinatorError::Internal(format!(
                "X3 reconciliation RPC {method} HTTP status {}",
                response.status()
            )));
        }

        let body: serde_json::Value = response.json().await.map_err(|e| {
            CoordinatorError::Internal(format!(
                "X3 reconciliation RPC {method} JSON decode failed: {e}"
            ))
        })?;
        if let Some(error) = body.get("error") {
            return Err(CoordinatorError::Internal(format!(
                "X3 reconciliation RPC {method} returned error: {error}"
            )));
        }
        Ok(body.get("result").cloned().unwrap_or(serde_json::Value::Null))
    }

    fn parse_hex_u64(value: &serde_json::Value, label: &str) -> Result<u64, CoordinatorError> {
        let raw = value.as_str().ok_or_else(|| {
            CoordinatorError::Internal(format!("{label} must be a hex string"))
        })?;
        u64::from_str_radix(raw.trim_start_matches("0x"), 16).map_err(|e| {
            CoordinatorError::Internal(format!("invalid {label} '{raw}': {e}"))
        })
    }

    fn normalize_hash(value: &str) -> Result<[u8; 32], CoordinatorError> {
        let raw = value.strip_prefix("0x").unwrap_or(value);
        let bytes = hex::decode(raw).map_err(|e| {
            CoordinatorError::Internal(format!("invalid transaction hash '{value}': {e}"))
        })?;
        if bytes.len() != 32 {
            return Err(CoordinatorError::Internal(format!(
                "transaction hash must be 32 bytes, got {}",
                bytes.len()
            )));
        }
        let mut out = [0u8; 32];
        out.copy_from_slice(&bytes);
        Ok(out)
    }

    fn substrate_extrinsic_hash(encoded_extrinsic: &[u8]) -> Result<[u8; 32], CoordinatorError> {
        use blake2::digest::{Update, VariableOutput};
        use blake2::Blake2bVar;

        let mut hasher = Blake2bVar::new(32).map_err(|e| {
            CoordinatorError::Internal(format!("create blake2b-256 hasher failed: {e}"))
        })?;
        hasher.update(encoded_extrinsic);
        let mut out = [0u8; 32];
        hasher.finalize_variable(&mut out).map_err(|e| {
            CoordinatorError::Internal(format!("finalize blake2b-256 failed: {e}"))
        })?;
        Ok(out)
    }

    fn classify_runtime_state(state: &str) -> RuntimeSettlementObservation {
        match state {
            "Finalized" => RuntimeSettlementObservation::Finalized,
            "Refunded" => RuntimeSettlementObservation::Refunded,
            "Unknown" => RuntimeSettlementObservation::Unknown,
            _ => RuntimeSettlementObservation::NonTerminal,
        }
    }
}

#[async_trait]
impl SettlementChainObserver for X3JsonRpcSettlementObserver {
    async fn transaction_status(
        &self,
        tx_id: &str,
    ) -> Result<SettlementTxObservation, CoordinatorError> {
        let expected = Self::normalize_hash(tx_id)?;
        let finalized_head = self
            .rpc("chain_getFinalizedHead", serde_json::json!([]))
            .await?;
        let finalized_hash = finalized_head.as_str().ok_or_else(|| {
            CoordinatorError::Internal(
                "chain_getFinalizedHead did not return a block hash".into(),
            )
        })?;

        let header = self
            .rpc("chain_getHeader", serde_json::json!([finalized_hash]))
            .await?;
        let head_number = Self::parse_hex_u64(
            header.get("number").unwrap_or(&serde_json::Value::Null),
            "finalized block number",
        )?;
        let lower = head_number.saturating_sub(self.finalized_scan_depth.saturating_sub(1));

        for number in (lower..=head_number).rev() {
            let block_hash = self
                .rpc("chain_getBlockHash", serde_json::json!([number]))
                .await?;
            let Some(block_hash) = block_hash.as_str() else {
                continue;
            };
            let block = self
                .rpc("chain_getBlock", serde_json::json!([block_hash]))
                .await?;
            let Some(extrinsics) = block
                .pointer("/block/extrinsics")
                .and_then(serde_json::Value::as_array)
            else {
                continue;
            };

            for encoded in extrinsics.iter().filter_map(serde_json::Value::as_str) {
                let bytes = hex::decode(encoded.trim_start_matches("0x")).map_err(|e| {
                    CoordinatorError::Internal(format!(
                        "decode finalized X3 extrinsic failed: {e}"
                    ))
                })?;
                if Self::substrate_extrinsic_hash(&bytes)? == expected {
                    return Ok(SettlementTxObservation::Included {
                        block_number: number,
                    });
                }
            }
        }

        Ok(SettlementTxObservation::Unknown)
    }

    async fn settlement_state(
        &self,
        runtime_intent_id: [u8; 32],
    ) -> Result<RuntimeSettlementObservation, CoordinatorError> {
        let intent = format!("0x{}", hex::encode(runtime_intent_id));
        let value = self
            .rpc("x3_settlementState", serde_json::json!([intent]))
            .await?;
        let state = value
            .get("state")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| {
                CoordinatorError::Internal(
                    "x3_settlementState response missing state".into(),
                )
            })?;
        Ok(Self::classify_runtime_state(state))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(status: SettlementOutboxStatus, purpose: u8) -> SettlementOutboxRecord {
        SettlementOutboxRecord {
            submission_id: [1u8; 32],
            session_id: "swap-a".into(),
            runtime_intent_id: [2u8; 32],
            purpose,
            call_index: 33,
            call_args_hash: [3u8; 32],
            proof_hashes: vec![[4u8; 32]],
            status,
            tx_id: Some("0xabc".into()),
            signed_extrinsic: Some(vec![1, 2, 3]),
            block_number: None,
            owner_id: Some("worker-a".into()),
            fence: Some(7),
            prepared_at: 100,
            updated_at: 101,
            error: None,
        }
    }

    #[test]
    fn signed_unknown_rebroadcasts_only_exact_bytes() {
        let r = record(SettlementOutboxStatus::Signed, 0);
        assert_eq!(
            decide_settlement_reconciliation(
                &r,
                Some(&SettlementTxObservation::Unknown),
                None,
            ),
            SettlementReconcileDecision::RebroadcastExactSignedExtrinsic
        );
    }

    #[test]
    fn broadcast_unknown_never_recommends_new_transaction() {
        let r = record(SettlementOutboxStatus::Broadcast, 0);
        assert_eq!(
            decide_settlement_reconciliation(
                &r,
                Some(&SettlementTxObservation::Unknown),
                None,
            ),
            SettlementReconcileDecision::WaitForInclusion
        );
    }

    #[test]
    fn included_claim_accepts_only_finalized_terminal() {
        let r = record(SettlementOutboxStatus::Included, 0);
        assert_eq!(
            decide_settlement_reconciliation(
                &r,
                None,
                Some(RuntimeSettlementObservation::Finalized),
            ),
            SettlementReconcileDecision::MarkTerminalObserved
        );
        assert!(matches!(
            decide_settlement_reconciliation(
                &r,
                None,
                Some(RuntimeSettlementObservation::Refunded),
            ),
            SettlementReconcileDecision::ManualHalt { .. }
        ));
    }

    #[test]
    fn included_refund_accepts_only_refunded_terminal() {
        let r = record(SettlementOutboxStatus::Included, 1);
        assert_eq!(
            decide_settlement_reconciliation(
                &r,
                None,
                Some(RuntimeSettlementObservation::Refunded),
            ),
            SettlementReconcileDecision::MarkTerminalObserved
        );
        assert!(matches!(
            decide_settlement_reconciliation(
                &r,
                None,
                Some(RuntimeSettlementObservation::Finalized),
            ),
            SettlementReconcileDecision::ManualHalt { .. }
        ));
    }

    #[test]
    fn included_tx_moves_to_included_block() {
        let r = record(SettlementOutboxStatus::Broadcast, 0);
        assert_eq!(
            decide_settlement_reconciliation(
                &r,
                Some(&SettlementTxObservation::Included { block_number: 77 }),
                None,
            ),
            SettlementReconcileDecision::MarkIncluded { block_number: 77 }
        );
    }
}
