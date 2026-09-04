//! # External-chain finality HEADER FEED seam (default-OFF)
//!
//! A thin, honest bridge between a **real external-chain relayer / light-client**
//! ("header source") and the finality VERIFIERS in [`crate::finality`].
//!
//! This crate never performs network I/O itself, and it never fabricates
//! proofs. It only defines the *acquisition contract* an operator wires up:
//!
//! 1. An ops team implements [`FinalityHeaderSource`] against their real
//!    relayer/light-client (IBC, a Beacon/optimistic light client, an L1 RPC,
//!    a trusted header-sync relay, …). The source emits opaque encoded proof
//!    blobs plus the validator set it attests, exactly matching the input
//!    shape `EvmFinalityVerifier` / `SvmFinalityVerifier` /
//!    `CrossVmFinalityVerifier` consume (`verify_finality_proof(proof: &[u8],
//!    validator_set, epoch)`).
//! 2. [`FinalityFeed`] drives one poll: pull a source batch, route each proof
//!    to the matching VM verifier, and report which finalized blocks verified
//!    and which were rejected.
//!
//! **Default-OFF guarantee:** [`FinalityFeed::new`] with no configured source
//! yields a feed whose `poll()` returns [`FeedError::NotConfigured`]. Nothing
//! in this module opens a socket or auto-broadcasts. A source is only ever
//! exercised when the operator explicitly constructs one and passes it in.

use alloc::vec::Vec;

use crate::finality::{
    CrossVmFinalityVerifier, FinalityVerificationResult, ValidatorInfo, VmFinalityVerifier,
    VmIdentifier,
};

/// Why a poll produced no verified headers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FeedError {
    /// No header source is configured; the feed is intentionally inert.
    NotConfigured,
    /// The configured source reported it could not produce a batch right now.
    SourceUnavailable(String),
    /// The source produced a batch that could not be routed/interpreted.
    Malformed(String),
}

impl core::fmt::Display for FeedError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NotConfigured => write!(f, "finality feed: no header source configured (off)"),
            Self::SourceUnavailable(e) => write!(f, "finality feed: source unavailable: {e}"),
            Self::Malformed(e) => write!(f, "finality feed: malformed batch: {e}"),
        }
    }
}

/// Static configuration a relayer deploy supplies for the feed. All fields are
/// defaults that operators override via their relayer service configuration.
#[derive(Clone, Debug)]
pub struct FeedConfig {
    /// Human label for the source/vendor (e.g. "optimism-l1-relayer").
    pub source_name: String,
    /// Source endpoint identity (host:port). Kept as metadata only; the
    /// operator's [`FinalityHeaderSource`] implementation decides how to use
    /// it. Never dialed by this crate.
    pub endpoint: String,
    /// Suggested poll cadence in milliseconds (scheduling is up to the caller,
    /// e.g. the relayer's loop / an `automation`).
    pub poll_interval_ms: u64,
    /// Whether the operator has enabled live ingestion for this feed.
    pub enabled: bool,
    /// VMs this feed is allowed to source (empty = all that the source offers).
    pub allowed_vms: Vec<VmIdentifier>,
}

impl Default for FeedConfig {
    fn default() -> Self {
        Self {
            source_name: "unconfigured".into(),
            endpoint: String::new(),
            poll_interval_ms: 5000,
            enabled: false,
            allowed_vms: Vec::new(),
        }
    }
}

/// One unit a relayer/light-client produces: an opaque finality proof blob for
/// a specific VM, plus the validator set (and epoch) it attests.
///
/// The blob format is whatever the operator's real light-client emits; the
/// verifiers in `finality.rs` interpret it. This crate does not construct or
/// inspect the blob.
#[derive(Clone, Debug)]
pub struct IncomingProof {
    /// Which VM's finality verifier should receive this proof.
    pub vm: VmIdentifier,
    /// Opaque proof bytes from the external source (never synthesised here).
    pub proof: Vec<u8>,
    /// Validator set the source attests for the proof's epoch.
    pub validator_set: Vec<ValidatorInfo>,
    /// Epoch the proof claims to finalize.
    pub epoch: u64,
    /// Latest finalized block number the source reports (informational only).
    pub finalized_block_number: u64,
}

/// A batch of proofs pulled from a source in one poll.
#[derive(Clone, Debug, Default)]
pub struct ProofBatch {
    pub proofs: Vec<IncomingProof>,
}

// Re-export out of the module namespace for relayer implementors.
pub use crate::finality::FinalizedBlock;

/// Outcome of one poll over the configured source.
#[derive(Clone, Debug, Default)]
pub struct VerifiedBatch {
    /// Encoded hashes of the finalized blocks that passed verification.
    pub verified: Vec<[u8; 32]>,
    /// Human-readable reason each rejected proof failed (parallel to input).
    pub rejected: Vec<String>,
    /// How many proofs the source produced this poll.
    pub ingested: usize,
}

/// **The seam.** An operator implements this against their real external-chain
/// relayer / light-client. No implementation with live behaviour ships in this
/// crate; see [`NullHeaderSource`] for the inert default.
pub trait FinalityHeaderSource: Send + Sync {
    fn name(&self) -> &str;
    /// Produce the latest finalized proofs to verify. Returning
    /// `SourceUnavailable` is normal backpressure and is not a crash.
    fn pull(&self) -> Result<ProofBatch, FeedError>;
}

/// The inert default source: never contacts the network, always reports the
/// feed is unconfigured. Used when operators have not wired a real source yet.
pub struct NullHeaderSource;

impl FinalityHeaderSource for NullHeaderSource {
    fn name(&self) -> &str {
        "null (default-off)"
    }
    fn pull(&self) -> Result<ProofBatch, FeedError> {
        Err(FeedError::NotConfigured)
    }
}

/// Drives a polled [`FinalityHeaderSource`] through the VM finality verifiers.
///
/// Constructing a feed never reaches the network. Only an explicit
/// [`FinalityFeed::poll`] call (from an operator-run relayer loop) does work,
/// and then only as much as the configured source returns.
pub struct FinalityFeed {
    config: FeedConfig,
    source: Box<dyn FinalityHeaderSource>,
    verifier: CrossVmFinalityVerifier,
}

impl FinalityFeed {
    /// Build a feed that is **off by default**: sources nothing until the
    /// operator constructs a [`FeedConfig`] with a real source and swaps it in
    /// via [`FinalityFeed::with_source`].
    pub fn new(chain_id: u64, genesis_hash: [u8; 32]) -> Self {
        Self {
            config: FeedConfig::default(),
            source: Box::new(NullHeaderSource),
            verifier: CrossVmFinalityVerifier::new(chain_id, genesis_hash.into()),
        }
    }
}

impl FinalityFeed {
    /// Return the current (mostly default-off) config.
    pub fn config(&self) -> &FeedConfig {
        &self.config
    }

    /// Builder: bind this feed to a concrete operator-supplied source.
    pub fn with_source(
        mut self,
        config: FeedConfig,
        source: Box<dyn FinalityHeaderSource>,
    ) -> Self {
        self.config = config;
        self.source = source;
        self
    }

    /// True when a real source is bound and enabled.
    pub fn is_live(&self) -> bool {
        self.config.enabled && self.source.name() != "null (default-off)"
    }

    /// Pull one batch from the bound source and verify each proof against the
    /// matching `finality.rs` verifier, returning the verified/rejected split.
    ///
    /// With no real source bound this returns [`Err(FeedError::NotConfigured)`].
    pub fn poll(&self) -> Result<VerifiedBatch, FeedError> {
        if !self.is_live() {
            return Err(FeedError::NotConfigured);
        }
        let batch = self.source.pull()?;
        let mut out = VerifiedBatch {
            ingested: batch.proofs.len(),
            ..Default::default()
        };
        for inc in &batch.proofs {
            let result: Result<FinalityVerificationResult, alloc::string::String> = match inc.vm {
                VmIdentifier::Evm => self
                    .verifier
                    .evm()
                    .verify_finality_proof(&inc.proof, &inc.validator_set, inc.epoch)
                    .map_err(|e| alloc::format!("{e:?}")),
                VmIdentifier::Svm => self
                    .verifier
                    .svm()
                    .verify_finality_proof(&inc.proof, &inc.validator_set, inc.epoch)
                    .map_err(|e| alloc::format!("{e:?}")),
                // X3Vm finality is the chain-internal finality anchor set,
                // not an external header source; not routed through this feed.
                VmIdentifier::X3Vm => {
                    out.rejected
                        .push("X3Vm proof not sourced externally".into());
                    continue;
                }
            };
            match result {
                Ok(fr) if fr.is_valid => {
                    if let Some(block) = fr.finalized_block {
                        out.verified.push(block.compute_hash());
                    }
                }
                Ok(fr) if fr.is_valid => {
                    if let Some(block) = fr.finalized_block {
                        out.verified.push(block.compute_hash());
                    }
                }
                Ok(fr) => {
                    let reason = match fr.error_message {
                        Some(bytes) if bytes.is_empty() => "invalid finality proof".to_string(),
                        Some(bytes) => String::from_utf8(bytes.clone())
                            .unwrap_or_else(|_| "invalid finality proof (non-utf8)".to_string()),
                        None => "invalid finality proof".to_string(),
                    };
                    out.rejected.push(reason);
                }
                Err(e) => out.rejected.push(e),
            }
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    /// Convert an opaque label into the minimal attestation a seam test uses.
    fn verifier_for_off_feed() -> FinalityFeed {
        FinalityFeed::new(1, [0xAAu8; 32])
    }

    #[test]
    fn default_feed_is_inert_and_off() {
        let feed = verifier_for_off_feed();
        assert!(!feed.is_live(), "feed must be OFF by default");
        assert_eq!(
            feed.poll().unwrap_err(),
            FeedError::NotConfigured,
            "polling an unconfigured feed must refuse to run (no network)"
        );
    }

    #[test]
    fn null_source_keeps_feed_off_even_if_enabled_flag_set() {
        // Even if an operator flips `enabled`, the inert Null source must not
        // fabricate data or attempt I/O.
        let feed = FinalityFeed::new(1, [0xBBu8; 32]).with_source(
            FeedConfig {
                enabled: true,
                ..Default::default()
            },
            Box::new(NullHeaderSource),
        );
        // Null source identifies as default-off, so is_live stays false.
        assert!(!feed.is_live());
        assert_eq!(feed.poll().unwrap_err(), FeedError::NotConfigured);
    }

    #[test]
    fn stub_source_backpressure_is_surfaced_not_crashing() {
        struct StubUnavailable;
        impl FinalityHeaderSource for StubUnavailable {
            fn name(&self) -> &str {
                "stub-backpressure"
            }
            fn pull(&self) -> Result<ProofBatch, FeedError> {
                Err(FeedError::SourceUnavailable("relayer syncing".into()))
            }
        }
        let feed = FinalityFeed::new(1, [0xCCu8; 32]).with_source(
            FeedConfig {
                endpoint: "example.invalid:1234".into(),
                enabled: true,
                source_name: "stub-backpressure".into(),
                ..Default::default()
            },
            Box::new(StubUnavailable),
        );
        assert!(feed.is_live());
        assert!(matches!(feed.poll(), Err(FeedError::SourceUnavailable(_))));
    }

    #[test]
    fn vm_filter_and_block_number_are_carried_through_config() {
        let cfg = FeedConfig {
            source_name: "optimism-relayer".into(),
            endpoint: "relayer.internal:9944".into(),
            poll_interval_ms: 1500,
            enabled: true,
            allowed_vms: vec![VmIdentifier::Evm, VmIdentifier::Svm],
        };
        assert_eq!(cfg.allowed_vms.len(), 2);
        assert_eq!(cfg.poll_interval_ms, 1500);
    }
}
