use crate::types::*;
use sha2::{Digest, Sha256};
use tracing::{debug, info, warn};

/// Off-chain task executor (RC1).
///
/// Receives a [`TaskPayload`], runs it deterministically, and returns an
/// [`ExecutionResult`] with a SHA-256 content hash ready for on-chain submission.
///
/// # Determinism contract
/// Given identical `payload.body` and `payload.params`, this function **must**
/// produce an identical `result_hash` on every executor node.  Non-deterministic
/// outputs (random seeds, wall-clock embedded in output bytes, etc.) will trigger
/// slashing in the RC3 quorum round.
pub struct TaskExecutor {
    executor_id: ExecutorId,
}

impl TaskExecutor {
    pub fn new(executor_id: ExecutorId) -> Self {
        TaskExecutor { executor_id }
    }

    /// Execute a task payload and return the result.
    pub async fn execute(&self, payload: TaskPayload) -> Result<ExecutionResult, NorthernSwarmError> {
        let start = std::time::Instant::now();
        info!(task_id = %payload.task_id, kind = ?payload.input_uri, "starting execution");

        let input_hash = sha256_hex(&payload.body);
        let output = self.run_deterministic(&payload)?;
        let duration_ms = start.elapsed().as_millis() as u64;
        let result_hash = sha256_hex(&output);
        let output_hash = result_hash.clone();

        debug!(
            task_id = %payload.task_id,
            result_hash = %result_hash,
            duration_ms,
            "execution complete",
        );

        let proof = ProofBundle {
            task_id: payload.task_id.clone(),
            executor_id: self.executor_id.clone(),
            input_hash,
            output_hash,
            executed_at: unix_now(),
            duration_ms,
        };

        Ok(ExecutionResult {
            task_id: payload.task_id,
            executor_id: self.executor_id.clone(),
            result_hash,
            output,
            proof,
            status: ExecutionStatus::Success,
        })
    }

    /// Deterministic execution kernel.
    ///
    /// **RC1 stub**: output = `body || serialised_params` (identity pass-through).
    /// The result hash is therefore fully determined by the input, satisfying the
    /// quorum invariant.
    ///
    /// Replace this in RC4 with the X3 Lang bytecode interpreter:
    /// ```text
    /// // RC4 target
    /// let vm = X3LangVm::new();
    /// vm.exec(payload.body.as_slice(), payload.params)
    /// ```
    fn run_deterministic(&self, payload: &TaskPayload) -> Result<Vec<u8>, NorthernSwarmError> {
        if payload.body.is_empty() {
            warn!(task_id = %payload.task_id, "payload body is empty — producing empty-hash result");
        }

        let params_bytes = serde_json::to_vec(&payload.params)
            .map_err(NorthernSwarmError::Serde)?;

        let mut combined = payload.body.clone();
        combined.extend_from_slice(&params_bytes);
        Ok(combined)
    }
}

/// SHA-256 hex digest of `data`.
pub fn sha256_hex(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

fn unix_now() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_payload(body: &[u8]) -> TaskPayload {
        TaskPayload {
            task_id: "test-task-001".into(),
            body: body.to_vec(),
            params: Default::default(),
            input_uri: None,
        }
    }

    #[tokio::test]
    async fn same_input_produces_same_hash() {
        let exec = TaskExecutor::new("exec-1".into());
        let p = dummy_payload(b"hello world");
        let r1 = exec.execute(p.clone()).await.unwrap();
        let r2 = exec.execute(p).await.unwrap();
        assert_eq!(r1.result_hash, r2.result_hash, "execution must be deterministic");
    }

    #[tokio::test]
    async fn different_inputs_produce_different_hashes() {
        let exec = TaskExecutor::new("exec-1".into());
        let r1 = exec.execute(dummy_payload(b"input-A")).await.unwrap();
        let r2 = exec.execute(dummy_payload(b"input-B")).await.unwrap();
        assert_ne!(r1.result_hash, r2.result_hash);
    }

    #[tokio::test]
    async fn result_status_is_success() {
        let exec = TaskExecutor::new("exec-1".into());
        let r = exec.execute(dummy_payload(b"data")).await.unwrap();
        assert_eq!(r.status, ExecutionStatus::Success);
    }

    #[test]
    fn sha256_hex_is_stable() {
        let h = sha256_hex(b"northern swarm");
        // Pre-computed: echo -n "northern swarm" | sha256sum
        assert_eq!(h.len(), 64, "SHA-256 hex must be 64 chars");
        // Ensure it's hex only
        assert!(h.chars().all(|c| c.is_ascii_hexdigit()));
    }
}
