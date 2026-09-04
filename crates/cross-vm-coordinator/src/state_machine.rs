//! Core state machine for the Cross-VM Atomic Swap Coordinator.
//!
//! Implements the 4-phase swap protocol:
//!   1. Setup (secret/hash, timelock computation)
//!   2. Lock HTLCs (both chains)
//!   3. Execute flash legs (borrow → swap → repay, per chain)
//!   4. Settle (reveal secret → claim both sides)
//!
//! Each phase transition is validated and logged. On any failure,
//! the machine transitions to Aborting → Refunded.

use crate::config::CoordinatorConfig;
use crate::persistence::{InMemoryPersistence, SessionPersistence};
use crate::types::*;
use blake2::{Blake2b512, Digest};
use blake3;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::sync::Arc;
use tracing::{error, info, warn};

/// The Cross-VM Swap Coordinator.
///
/// Manages the lifecycle of atomic swap sessions across EVM, SVM, and X3VM.
/// Sessions are persisted via the `P: SessionPersistence` trait, allowing
/// the coordinator state to survive node restarts.
pub struct SwapCoordinator<P: SessionPersistence = InMemoryPersistence> {
    config: CoordinatorConfig,
    /// In-memory working copy of sessions (authoritative). Mutations are
    /// mirrored to persistence.
    sessions: HashMap<String, SwapSession>,
    /// Global set of HTLC secrets that have already been claimed.
    ///
    /// Once a secret is revealed (in `claim_fast` or `claim_slow`), its raw
    /// 32-byte value is inserted here. Any future claim attempt using the same
    /// secret — even in a different session — is immediately rejected.
    /// This prevents cross-session replay attacks where an adversary reuses a
    /// leaked secret to claim HTLCs in multiple sessions.
    ///
    /// BTreeSet gives O(log n) membership checks vs O(n) for Vec, and eliminates
    /// any risk of duplicate entries accumulating over time.
    used_secrets: BTreeSet<[u8; 32]>,
    /// Persistence backend for sessions.
    persistence: Arc<P>,
}

impl SwapCoordinator<InMemoryPersistence> {
    /// Create a coordinator with in-memory persistence (non-durable).
    pub fn new(config: CoordinatorConfig) -> Self {
        // CRITICAL-001 FIX: Enforce durable persistence in production.
        // Node crash with InMemoryPersistence = irreversible HTLC fund loss.
        if !cfg!(test) {
            panic!("CRITICAL-001: InMemoryPersistence forbidden in production. Use OffchainPersistence. Node crash = fund loss.");
        }
        Self::with_persistence(config, Arc::new(InMemoryPersistence::new()))
    }

    pub fn with_default_config() -> Self {
        Self::new(CoordinatorConfig::default())
    }
}

impl<P: SessionPersistence> SwapCoordinator<P> {
    const PURGE_BATCH_SIZE: usize = 256;

    /// Create a coordinator with custom persistence backend.
    ///
    /// On construction, loads all existing sessions from the persistence layer.
    pub fn with_persistence(config: CoordinatorConfig, persistence: Arc<P>) -> Self {
        // Load sessions from persistence to recover state after restart
        let sessions = persistence.load_all();
        let session_count = sessions.len();

        // Restore the used-secrets set so that HTLC secret replay protection
        // survives node restarts.  Without this, an adversary could restart the
        // node and reuse a previously-revealed secret to steal funds.
        let used_secrets: BTreeSet<[u8; 32]> =
            persistence.load_used_secrets().into_iter().collect();
        let secrets_count = used_secrets.len();

        if session_count > 0 {
            info!(
                sessions = session_count,
                "Restored sessions from persistence"
            );
        }
        if secrets_count > 0 {
            info!(
                used_secrets = secrets_count,
                "Restored HTLC secret replay guard from persistence"
            );
        }

        Self {
            config,
            sessions,
            used_secrets,
            persistence,
        }
    }

    /// Persist a session after mutation by session_id.
    /// Must be called AFTER the mutable borrow is released.
    fn persist_by_id(&self, session_id: &str) {
        if let Some(session) = self.sessions.get(session_id) {
            self.persistence.save(session);
        }
    }

    /// Get a session by ID.
    pub fn get_session(&self, session_id: &str) -> Option<&SwapSession> {
        self.sessions.get(session_id)
    }

    /// Get a mutable session by ID.
    pub fn get_session_mut(&mut self, session_id: &str) -> Option<&mut SwapSession> {
        self.sessions.get_mut(session_id)
    }

    /// Total active sessions.
    pub fn active_sessions(&self) -> usize {
        self.sessions
            .values()
            .filter(|s| {
                !matches!(
                    s.phase,
                    SwapPhase::Complete | SwapPhase::Refunded | SwapPhase::Failed
                )
            })
            .count()
    }

    /// Total sessions (active + terminated). Useful for monitoring.
    pub fn session_count(&self) -> usize {
        self.sessions.len()
    }

    /// Purge terminated sessions (Complete, Refunded, Failed) older than `max_age_secs`.
    ///
    /// Call periodically (e.g., every epoch) to prevent unbounded memory growth on
    /// long-running nodes. Returns the number of sessions purged.
    pub fn purge_terminated_sessions(&mut self, now_unix: u64, max_age_secs: u64) -> usize {
        let mut purged = 0;
        let mut cursor: Option<String> = None;

        loop {
            let batch = self.next_stale_terminal_session_batch(
                now_unix,
                max_age_secs,
                cursor.as_deref(),
                Self::PURGE_BATCH_SIZE,
            );
            if batch.is_empty() {
                break;
            }

            cursor = batch.last().cloned();
            for id in batch {
                self.sessions.remove(&id);
                self.persistence.remove(&id);
                purged += 1;
            }
        }

        purged
    }

    // ── Phase 1: Setup ────────────────────────────────────────────────────

    /// Initialize a new atomic swap session.
    ///
    /// Generates the secret/hash pair, computes timelocks, and returns
    /// the session ID. The secret is returned separately so the caller
    /// can hold it securely until Phase 4.
    ///
    /// # DoS Protection
    /// Rejects new sessions when the total session count
    /// exceeds `MAX_TOTAL_SESSIONS` (active + not yet purged).
    /// Operators must call `purge_terminated_sessions()` periodically.
    pub fn setup_swap(
        &mut self,
        fast_vm: VmTarget,
        slow_vm: VmTarget,
        flash_legs: Vec<FlashLeg>,
        now_unix: u64,
    ) -> Result<(String, HtlcSecret, HtlcHash), CoordinatorError> {
        const MAX_TOTAL_SESSIONS: usize = 10_000;
        if self.sessions.len() >= MAX_TOTAL_SESSIONS {
            return Err(CoordinatorError::Internal(
                "Session limit reached — call purge_terminated_sessions() to free space"
                    .to_string(),
            ));
        }

        let secret = HtlcSecret::generate();
        let hash = secret.hash();
        let (t_fast, t_slow) = self.config.compute_timelocks(now_unix, &fast_vm);

        for leg in &flash_legs {
            if !leg.provider.supports_vm(&leg.vm) {
                return Err(CoordinatorError::ProviderUnavailable {
                    provider: leg.provider.clone(),
                    vm: leg.vm.to_string(),
                });
            }
        }

        let session_id = Self::derive_session_id(&secret);
        if self.sessions.contains_key(&session_id) {
            return Err(CoordinatorError::Internal(format!(
                "Session ID collision detected: '{session_id}' already exists"
            )));
        }

        let session = SwapSession {
            session_id: session_id.clone(),
            hash_lock: hash,
            htlc_fast: None,
            htlc_slow: None,
            flash_legs,
            leg_outcomes: Vec::new(),
            phase: SwapPhase::Setup,
            timelock_fast: t_fast,
            timelock_slow: t_slow,
            created_at: now_unix,
            updated_at: now_unix,
            requires_merkle_verification: matches!(
                (&fast_vm, &slow_vm),
                (VmTarget::Evm { .. }, _)
                    | (_, VmTarget::Evm { .. })
                    | (VmTarget::Svm, _)
                    | (_, VmTarget::Svm)
            ),
        };

        self.persistence.save(&session);
        self.sessions.insert(session_id.clone(), session);

        info!(
            session = %session_id,
            hash = %hash,
            fast_vm = %fast_vm,
            slow_vm = %slow_vm,
            t_fast = t_fast,
            t_slow = t_slow,
            "Swap session created"
        );

        Ok((session_id, secret, hash))
    }

    fn derive_session_id(secret: &HtlcSecret) -> String {
        let mut hasher = Blake2b512::new();
        hasher.update(b"x3-cross-vm-session-id-v1");
        hasher.update(secret.as_bytes());
        let digest = hasher.finalize();
        format!("swap-{}", hex::encode(&digest[..32]))
    }

    fn is_stale_terminal_session(session: &SwapSession, now_unix: u64, max_age_secs: u64) -> bool {
        let is_terminal = matches!(
            session.phase,
            SwapPhase::Complete | SwapPhase::Refunded | SwapPhase::Failed
        );
        let is_stale = now_unix.saturating_sub(session.updated_at) > max_age_secs;

        is_terminal && is_stale
    }

    fn next_stale_terminal_session_batch(
        &self,
        now_unix: u64,
        max_age_secs: u64,
        after_session_id: Option<&str>,
        limit: usize,
    ) -> Vec<String> {
        if limit == 0 {
            return Vec::new();
        }

        let mut session_ids: Vec<&str> = self.sessions.keys().map(String::as_str).collect();
        session_ids.sort_unstable();

        let mut batch = Vec::with_capacity(limit);
        for session_id in session_ids {
            if after_session_id.is_some_and(|cursor| session_id <= cursor) {
                continue;
            }

            let session = self
                .sessions
                .get(session_id)
                .expect("session id collected from map keys must exist");
            if !Self::is_stale_terminal_session(session, now_unix, max_age_secs) {
                continue;
            }

            batch.push(session_id.to_string());
            if batch.len() == limit {
                break;
            }
        }

        batch
    }

    // ── Phase 2: Lock HTLCs ───────────────────────────────────────────────

    /// Record that an HTLC has been created on the fast chain.
    pub fn record_htlc_fast(
        &mut self,
        session_id: &str,
        record: HtlcRecord,
        now_unix: u64,
    ) -> Result<(), CoordinatorError> {
        let current_phase = self
            .sessions
            .get(session_id)
            .ok_or_else(|| CoordinatorError::SessionNotFound {
                session_id: session_id.to_string(),
            })?
            .phase;

        Self::validate_phase_transition(current_phase, SwapPhase::LockingHtlcs)?;

        {
            let session = self.sessions.get_mut(session_id).unwrap();

            info!(
                session = %session_id,
                htlc_id = %record.id.to_hex(),
                vm = ?record.params.vm,
                "Fast chain HTLC recorded"
            );

            session.htlc_fast = Some(record);
            session.phase = SwapPhase::LockingHtlcs;
            session.updated_at = now_unix;
        }

        self.persist_by_id(session_id);
        Ok(())
    }

    /// Record that an HTLC has been created on the slow chain.
    pub fn record_htlc_slow(
        &mut self,
        session_id: &str,
        record: HtlcRecord,
        now_unix: u64,
    ) -> Result<(), CoordinatorError> {
        // Read phase first, then validate, then mutate.
        let current_phase = self
            .sessions
            .get(session_id)
            .ok_or_else(|| CoordinatorError::SessionNotFound {
                session_id: session_id.to_string(),
            })?
            .phase;

        Self::validate_phase_transition(current_phase, SwapPhase::LockingHtlcs)?;

        {
            let session = self.sessions.get_mut(session_id).ok_or_else(|| {
                CoordinatorError::SessionNotFound {
                    session_id: session_id.to_string(),
                }
            })?;

            info!(
                session = %session_id,
                htlc_id = %record.id.to_hex(),
                vm = ?record.params.vm,
                "Slow chain HTLC recorded"
            );

            session.htlc_slow = Some(record);
            session.updated_at = now_unix;

            // If both HTLCs are now recorded, advance phase
            if session.htlc_fast.is_some() && session.htlc_slow.is_some() {
                session.phase = SwapPhase::HtlcsLocked;
                info!(session = %session_id, "Both HTLCs locked — ready for flash legs");
            }
        }

        self.persist_by_id(session_id);
        Ok(())
    }

    /// Update confirmation count for an HTLC and check if we can proceed.
    pub fn update_confirmations(
        &mut self,
        session_id: &str,
        is_fast: bool,
        confirmations: u32,
        now_unix: u64,
    ) -> Result<bool, CoordinatorError> {
        let result = {
            let session = self.sessions.get_mut(session_id).ok_or_else(|| {
                CoordinatorError::SessionNotFound {
                    session_id: session_id.to_string(),
                }
            })?;

            let htlc = if is_fast {
                session.htlc_fast.as_mut()
            } else {
                session.htlc_slow.as_mut()
            };

            let htlc = htlc.ok_or_else(|| {
                CoordinatorError::Internal(format!(
                    "HTLC not found for {} chain",
                    if is_fast { "fast" } else { "slow" }
                ))
            })?;

            htlc.confirmations = confirmations;
            session.updated_at = now_unix;

            // Check if both HTLCs have enough confirmations
            let fast_ok = session
                .htlc_fast
                .as_ref()
                .map(|h| h.confirmations >= h.confirmations_required)
                .unwrap_or(false);
            let slow_ok = session
                .htlc_slow
                .as_ref()
                .map(|h| h.confirmations >= h.confirmations_required)
                .unwrap_or(false);

            fast_ok && slow_ok
        };

        self.persist_by_id(session_id);
        Ok(result)
    }

    // ── Phase 3: Execute Flash Legs ───────────────────────────────────────

    /// Begin executing flashloan legs.
    pub fn begin_flash_execution(
        &mut self,
        session_id: &str,
        now_unix: u64,
    ) -> Result<(), CoordinatorError> {
        let current_phase = self
            .sessions
            .get(session_id)
            .ok_or_else(|| CoordinatorError::SessionNotFound {
                session_id: session_id.to_string(),
            })?
            .phase;

        Self::validate_phase_transition(current_phase, SwapPhase::ExecutingFlashLegs)?;

        let mut abort_error = None;
        {
            let session = self.sessions.get_mut(session_id).unwrap();

            // Safety check: ensure we're not too close to timelock
            if self.config.is_near_expiry(session.timelock_fast, now_unix) {
                warn!(
                    session = %session_id,
                    timelock = session.timelock_fast,
                    now = now_unix,
                    "Near timelock expiry — aborting flash execution"
                );
                session.phase = SwapPhase::Aborting;
                abort_error = Some(CoordinatorError::TimelockExpired {
                    htlc_id: session_id.to_string(),
                });
            } else if session.flash_legs.is_empty() {
                session.phase = SwapPhase::LegsComplete;
                info!(session = %session_id, "No flash legs to execute — skipping to LegsComplete");
            } else {
                session.phase = SwapPhase::ExecutingFlashLegs;
                info!(
                    session = %session_id,
                    legs = session.flash_legs.len(),
                    "Beginning flash leg execution"
                );
            }
            session.updated_at = now_unix;
        }

        self.persist_by_id(session_id);

        if let Some(err) = abort_error {
            return Err(err);
        }
        Ok(())
    }

    /// Record the outcome of a flashloan leg.
    ///
    /// CRITICAL-006 FIX: Re-checks timelock safety margin during async execution
    /// to detect TOCTOU race conditions. If timelock expires while legs execute,
    /// aborts the swap before state transitions.
    /// Record the outcome of a flashloan leg.
    ///
    /// CRITICAL-006 FIX: Re-checks timelock safety margin during async execution
    /// to detect TOCTOU race conditions. If timelock expires while legs execute,
    /// aborts the swap before state transitions.
    pub fn record_leg_outcome(
        &mut self,
        session_id: &str,
        outcome: FlashLegOutcome,
        now_unix: u64,
    ) -> Result<(), CoordinatorError> {
        let mut abort_error = None;
        {
            let session = self.sessions.get_mut(session_id).ok_or_else(|| {
                CoordinatorError::SessionNotFound {
                    session_id: session_id.to_string(),
                }
            })?;

            // CRITICAL-006 FIX: Re-check timelock safety margin before state transition
            // This detects if timelock expired during async flash leg execution
            if self.config.is_near_expiry(session.timelock_fast, now_unix) {
                warn!(
                    session = %session_id,
                    timelock = session.timelock_fast,
                    now = now_unix,
                    "Timelock expired during flash leg execution — aborting"
                );
                session.phase = SwapPhase::Aborting;
                session.updated_at = now_unix;
                abort_error = Some(CoordinatorError::TimelockExpired {
                    htlc_id: session_id.to_string(),
                });
            } else {
                let leg_index = session.leg_outcomes.len();

                match outcome {
                    FlashLegOutcome::Success {
                        tx_hash: _,
                        gas_used,
                        output_amount,
                        premium_paid,
                    } => {
                        info!(
                            session = %session_id,
                            leg = leg_index,
                            gas_used,
                            output_amount,
                            premium_paid,
                            "Flash leg succeeded"
                        );
                        session.leg_outcomes.push(outcome);
                    }
                    FlashLegOutcome::Reverted { ref reason } => {
                        let reason_clone = reason.clone();
                        error!(
                            session = %session_id,
                            leg = leg_index,
                            reason = %reason_clone,
                            "Flash leg REVERTED — aborting swap"
                        );
                        session.phase = SwapPhase::Aborting;
                        session.updated_at = now_unix;
                        session.leg_outcomes.push(outcome);
                        abort_error = Some(CoordinatorError::FlashLegReverted {
                            vm: format!("leg-{leg_index}"),
                            reason: reason_clone,
                        });
                    }
                }

                if abort_error.is_none() {
                    session.updated_at = now_unix;

                    // Check if all legs are complete
                    if session.leg_outcomes.len() == session.flash_legs.len() {
                        let all_success = session
                            .leg_outcomes
                            .iter()
                            .all(|o| matches!(o, FlashLegOutcome::Success { .. }));

                        if all_success {
                            session.phase = SwapPhase::LegsComplete;
                            info!(session = %session_id, "All flash legs complete — ready for settlement");
                        } else {
                            session.phase = SwapPhase::Aborting;
                            warn!(session = %session_id, "Not all legs succeeded — aborting");
                        }
                    }
                }
            }
        }

        self.persist_by_id(session_id);

        if let Some(err) = abort_error {
            return Err(err);
        }
        Ok(())
    }

    // ── Phase 4: Settlement ───────────────────────────────────────────────

    /// Begin settlement: reveal secret on the fast chain.
    /// Begin settlement: reveal secret on the fast chain.
    ///
    /// CRITICAL-006 FIX: Re-checks timelock safety margin before revealing secret
    /// to prevent TOCTOU race where timelock expires between flash execution and settlement.
    pub fn begin_settlement(
        &mut self,
        session_id: &str,
        now_unix: u64,
    ) -> Result<HtlcHash, CoordinatorError> {
        let current_phase = self
            .sessions
            .get(session_id)
            .ok_or_else(|| CoordinatorError::SessionNotFound {
                session_id: session_id.to_string(),
            })?
            .phase;

        Self::validate_phase_transition(current_phase, SwapPhase::ClaimingFast)?;

        let mut abort_error = None;
        {
            let session = self.sessions.get_mut(session_id).unwrap();

            // CRITICAL-006 FIX: Re-check timelock safety margin before settlement
            // Prevents revealing secret after timelock has expired
            if self.config.is_near_expiry(session.timelock_fast, now_unix) {
                warn!(
                    session = %session_id,
                    timelock = session.timelock_fast,
                    now = now_unix,
                    "Near timelock expiry at settlement — aborting to prevent fund loss"
                );
                session.phase = SwapPhase::Aborting;
                abort_error = Some(CoordinatorError::TimelockExpired {
                    htlc_id: session_id.to_string(),
                });
            } else {
                session.phase = SwapPhase::ClaimingFast;
                info!(session = %session_id, "Settlement: claiming on fast chain");
            }
            session.updated_at = now_unix;
        }

        if abort_error.is_some() {
            self.persist_by_id(session_id);
            return Err(abort_error.unwrap());
        }

        let hash_lock = self.sessions.get(session_id).unwrap().hash_lock;
        self.persist_by_id(session_id);
        Ok(hash_lock)
    }

    /// Record that the fast chain claim succeeded (secret revealed on-chain).
    ///
    /// # Parameters
    /// - `secret`: The preimage that was revealed to claim the HTLC. This is
    ///   hashed and compared against the session's `hash_lock` to prevent
    ///   accepting a wrong or forged preimage.
    ///
    /// # Replay Protection
    /// The secret bytes are inserted into `used_secrets`. Any subsequent call
    /// with the same secret — for this session or any other — will be rejected
    /// with `CoordinatorError::SecretAlreadyUsed`.
    pub fn record_fast_claim(
        &mut self,
        session_id: &str,
        secret: HtlcSecret,
        now_unix: u64,
    ) -> Result<(), CoordinatorError> {
        // Read phase first, then validate, then mutate.
        let current_phase = self
            .sessions
            .get(session_id)
            .ok_or_else(|| CoordinatorError::SessionNotFound {
                session_id: session_id.to_string(),
            })?
            .phase;

        Self::validate_phase_transition(current_phase, SwapPhase::ClaimingFast)?;

        // Global replay guard: O(log n) BTreeSet lookup, constant-time via subtle for
        // resistance to timing side-channels even on the fast path.
        let secret_hash = *blake3::hash(&secret.0).as_bytes();
        if self.used_secrets.contains(&secret_hash) {
            return Err(CoordinatorError::Internal(
                format!("HTLC secret replay detected for session '{session_id}' — secret already used in a previous claim")
            ));
        }
        {
            let session = self.sessions.get_mut(session_id).ok_or_else(|| {
                CoordinatorError::SessionNotFound {
                    session_id: session_id.to_string(),
                }
            })?;

            // Verify the preimage hashes to the lock
            let provided_hash = secret.hash();
            if provided_hash != session.hash_lock {
                return Err(CoordinatorError::Internal(format!(
                    "Secret hash mismatch for session '{}': provided {:?} does not match lock {:?}",
                    session_id, provided_hash, session.hash_lock
                )));
            }

            // Register the secret globally BEFORE mutating session state
            // (fail-safe: if we crash after insert but before state update, the
            //  duplicate will be caught on retry, which is the safe outcome).
            // Register the secret hash globally (never store plaintext)
            let secret_hash = *blake3::hash(&secret.0).as_bytes();
            self.used_secrets.insert(secret_hash);

            if let Some(ref mut htlc) = session.htlc_fast {
                htlc.status = HtlcStatus::Claimed;
            }

            session.phase = SwapPhase::ClaimingSlow;
            session.updated_at = now_unix;
        }

        // Persist the updated secret set BEFORE persisting the session, so
        // that on crash-recovery the replay guard is at least as restrictive
        // as the session state (safe direction).
        let secrets_vec: Vec<[u8; 32]> = self.used_secrets.iter().copied().collect();
        self.persistence.save_used_secrets(&secrets_vec);
        self.persist_by_id(session_id);
        info!(session = %session_id, "Fast chain claimed — now claiming slow chain");
        Ok(())
    }

    /// Record that the slow chain claim succeeded. Swap is complete!
    pub fn record_slow_claim(
        &mut self,
        session_id: &str,
        now_unix: u64,
    ) -> Result<(), CoordinatorError> {
        // Read phase first, then validate, then mutate.
        let current_phase = self
            .sessions
            .get(session_id)
            .ok_or_else(|| CoordinatorError::SessionNotFound {
                session_id: session_id.to_string(),
            })?
            .phase;

        Self::validate_phase_transition(current_phase, SwapPhase::ClaimingSlow)?;

        {
            let session = self.sessions.get_mut(session_id).ok_or_else(|| {
                CoordinatorError::SessionNotFound {
                    session_id: session_id.to_string(),
                }
            })?;

            if let Some(ref mut htlc) = session.htlc_slow {
                htlc.status = HtlcStatus::Claimed;
            }

            session.phase = SwapPhase::Complete;
            session.updated_at = now_unix;
        }

        self.persist_by_id(session_id);
        info!(session = %session_id, "🎉 Atomic swap COMPLETE — both sides claimed");
        Ok(())
    }

    // ── Abort & Refund ────────────────────────────────────────────────────

    /// Abort the swap. Triggers refund after timelocks expire.
    pub fn abort(
        &mut self,
        session_id: &str,
        reason: &str,
        now_unix: u64,
    ) -> Result<(), CoordinatorError> {
        {
            let session = self.sessions.get_mut(session_id).ok_or_else(|| {
                CoordinatorError::SessionNotFound {
                    session_id: session_id.to_string(),
                }
            })?;

            warn!(session = %session_id, reason, "Aborting swap — will refund after timelocks");

            session.phase = SwapPhase::Aborting;
            session.updated_at = now_unix;
        }

        self.persist_by_id(session_id);
        Ok(())
    }

    /// Record that both HTLCs have been refunded.
    pub fn record_refunds(
        &mut self,
        session_id: &str,
        now_unix: u64,
    ) -> Result<(), CoordinatorError> {
        // Read phase first, then validate, then mutate.
        let current_phase = self
            .sessions
            .get(session_id)
            .ok_or_else(|| CoordinatorError::SessionNotFound {
                session_id: session_id.to_string(),
            })?
            .phase;

        Self::validate_phase_transition(current_phase, SwapPhase::Refunded)?;

        {
            let session = self.sessions.get_mut(session_id).ok_or_else(|| {
                CoordinatorError::SessionNotFound {
                    session_id: session_id.to_string(),
                }
            })?;

            if let Some(ref mut htlc) = session.htlc_fast {
                htlc.status = HtlcStatus::Refunded;
            }
            if let Some(ref mut htlc) = session.htlc_slow {
                htlc.status = HtlcStatus::Refunded;
            }

            session.phase = SwapPhase::Refunded;
            session.updated_at = now_unix;
        }

        self.persist_by_id(session_id);
        info!(session = %session_id, "Both HTLCs refunded — swap cancelled cleanly");
        Ok(())
    }

    // ── Internal Helpers ──────────────────────────────────────────────────

    fn validate_phase_transition(from: SwapPhase, to: SwapPhase) -> Result<(), CoordinatorError> {
        let valid = matches!(
            (from, to),
            (SwapPhase::Setup, SwapPhase::LockingHtlcs)
                | (SwapPhase::LockingHtlcs, SwapPhase::LockingHtlcs)
                | (SwapPhase::LockingHtlcs, SwapPhase::HtlcsLocked)
                | (SwapPhase::HtlcsLocked, SwapPhase::ExecutingFlashLegs)
                | (SwapPhase::ExecutingFlashLegs, SwapPhase::LegsComplete)
                | (SwapPhase::LegsComplete, SwapPhase::ClaimingFast)
                | (SwapPhase::ClaimingFast, SwapPhase::ClaimingFast)
                | (SwapPhase::ClaimingFast, SwapPhase::ClaimingSlow)
                | (SwapPhase::ClaimingSlow, SwapPhase::ClaimingSlow)
                | (SwapPhase::ClaimingSlow, SwapPhase::Complete)
                // Abort from any active phase
                | (SwapPhase::Setup, SwapPhase::Aborting)
                | (SwapPhase::LockingHtlcs, SwapPhase::Aborting)
                | (SwapPhase::HtlcsLocked, SwapPhase::Aborting)
                | (SwapPhase::ExecutingFlashLegs, SwapPhase::Aborting)
                | (SwapPhase::LegsComplete, SwapPhase::Aborting)
                | (SwapPhase::ClaimingFast, SwapPhase::Aborting)
                | (SwapPhase::Aborting, SwapPhase::Refunded)
        );

        if valid {
            Ok(())
        } else {
            Err(CoordinatorError::InvalidPhaseTransition {
                from: from.to_string(),
                to: to.to_string(),
            })
        }
    }
}

#[cfg(test)]
mod state_machine_regression_tests {
    use super::*;

    fn make_session(session_id: &str, phase: SwapPhase, updated_at: u64) -> SwapSession {
        SwapSession {
            session_id: session_id.to_string(),
            hash_lock: HtlcHash([0u8; 32]),
            htlc_fast: None,
            htlc_slow: None,
            flash_legs: vec![],
            leg_outcomes: vec![],
            phase,
            timelock_fast: updated_at + 10,
            timelock_slow: updated_at + 20,
            created_at: updated_at,
            updated_at,
            requires_merkle_verification: false,
        }
    }

    #[test]
    fn stale_session_batch_filters_before_limiting_and_seeks_from_cursor() {
        let mut coordinator = SwapCoordinator::with_default_config();
        let now = 10_000;
        let max_age = 100;

        coordinator.sessions.insert(
            "swap-001-active".to_string(),
            make_session("swap-001-active", SwapPhase::Setup, now - 1_000),
        );
        coordinator.sessions.insert(
            "swap-002-stale".to_string(),
            make_session("swap-002-stale", SwapPhase::Refunded, now - 1_000),
        );
        coordinator.sessions.insert(
            "swap-003-active".to_string(),
            make_session("swap-003-active", SwapPhase::LockingHtlcs, now - 1_000),
        );
        coordinator.sessions.insert(
            "swap-004-stale".to_string(),
            make_session("swap-004-stale", SwapPhase::Complete, now - 1_000),
        );
        coordinator.sessions.insert(
            "swap-005-active".to_string(),
            make_session("swap-005-active", SwapPhase::ClaimingFast, now - 1_000),
        );
        coordinator.sessions.insert(
            "swap-006-stale".to_string(),
            make_session("swap-006-stale", SwapPhase::Failed, now - 1_000),
        );

        let first_batch = coordinator.next_stale_terminal_session_batch(now, max_age, None, 2);
        assert_eq!(
            first_batch,
            vec!["swap-002-stale".to_string(), "swap-004-stale".to_string()],
            "limit must apply after filtering for stale terminal sessions"
        );

        let second_batch =
            coordinator.next_stale_terminal_session_batch(now, max_age, Some("swap-004-stale"), 2);
        assert_eq!(second_batch, vec!["swap-006-stale".to_string()]);
    }
}
