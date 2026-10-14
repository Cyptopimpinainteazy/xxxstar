/// Atomic swap error types.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SwapError {
    MissingField {
        field: &'static str,
    },

    InvalidTimeoutOrdering {
        destination_timeout: u64,
        source_timeout: u64,
    },

    AlreadyTerminal {
        status: crate::intent::AtomicSwapStatus,
    },

    InvalidTransition {
        from: crate::intent::AtomicSwapStatus,
        to: crate::intent::AtomicSwapStatus,
    },

    HashlockMismatch,

    InsufficientFinality {
        chain: crate::intent::ChainKind,
        required: crate::intent::FinalityLevel,
        actual: crate::intent::FinalityLevel,
    },

    InsufficientRelayerQuorum {
        required: u32,
        actual: u32,
    },

    MissingProof {
        proof_name: &'static str,
    },

    ProofVerificationFailed {
        proof_name: &'static str,
        reason: alloc::string::String,
    },

    TimeoutNotElapsed {
        current: u64,
        timeout: u64,
    },

    AlreadyLocked {
        chain: crate::intent::ChainKind,
    },

    NotLockable {
        status: crate::intent::AtomicSwapStatus,
    },

    InvalidRefundPath {
        reason: alloc::string::String,
    },

    TxNotFound {
        tx_hash: alloc::string::String,
    },

    InvalidPreimageLength {
        expected: usize,
        actual: usize,
    },

    LockFailed {
        reason: alloc::string::String,
    },

    SourceLockFailed {
        reason: alloc::string::String,
    },

    FinalityNotMet {
        chain: alloc::string::String,
        required: u32,
        current: u32,
    },

    ProofNotFound {
        proof_id: alloc::string::String,
        intent_id: u64,
    },

    MissingTxHash {
        step: alloc::string::String,
        chain: alloc::string::String,
    },

    ClaimFailed {
        chain: alloc::string::String,
        reason: alloc::string::String,
    },

    RefundFailed {
        chain: alloc::string::String,
        reason: alloc::string::String,
    },

    Internal(alloc::string::String),

    /// Slashing: case not found by slash_id.
    SlashNotFound {
        slash_id: u64,
    },

    /// Slashing: evidence size below minimum.
    InsufficientEvidence {
        minimum: usize,
        actual: usize,
    },

    /// Slashing: actor's stake is less than the slash amount.
    InsufficientStake {
        actor: alloc::string::String,
        available: u128,
        required: u128,
    },

    /// Slashing: case cannot be transitioned in its current status.
    InvalidSlashStatus {
        slash_id: u64,
        reason: alloc::string::String,
    },

    /// RPC call failed
    RpcError(alloc::string::String),

    /// Dispute: case not found by dispute_id.
    DisputeNotFound {
        dispute_id: u64,
    },

    /// Dispute: cannot transition in its current status.
    InvalidDisputeStatus {
        dispute_id: u64,
        reason: alloc::string::String,
    },

    /// Intent validation: amount_in < min_amount_out — economically impossible.
    PartialFillNotAllowed {
        amount_in: u128,
        min_amount_out: u128,
    },

    /// Finality certificate: the certificate names a different chain than the one asked about.
    CertificateChainMismatch {
        expected: crate::intent::ChainKind,
        found: crate::intent::ChainKind,
    },

    /// Finality certificate: the anchor is above the tip it claims to have been observed at.
    CertificateBlockAfterObservation {
        block_height: u64,
        observed_at: u64,
    },

    /// Finality certificate: the derived confirmation depth does not fit a `u32`.
    CertificateConfirmationsOverflow {
        block_height: u64,
        observed_at: u64,
    },

    /// Finality certificate: the reported confirmation count is not the one its own anchor implies.
    ///
    /// The count is a *derived* value (`observed_at - block_height + 1`). A caller that supplies a
    /// count its anchor does not have is trying to claim depth the anchored block does not carry.
    CertificateConfirmationsDisagree {
        chain: crate::intent::ChainKind,
        block_height: u64,
        observed_at: u64,
        reported: u32,
        expected: u32,
    },

    /// Finality certificate: the tip is below the tallest tip this oracle already accepted.
    ///
    /// A chain cannot go backwards. A certificate observed at a lower tip than one already accepted
    /// is a rollback presented as a fresh fact, and is refused rather than re-read.
    CertificateRewindsAcceptedAnchor {
        chain: crate::intent::ChainKind,
        accepted_tip: u64,
        certificate_tip: u64,
    },

    /// Finality certificate: the tip is further behind a witnessed tip than the configured window.
    CertificateStale {
        chain: crate::intent::ChainKind,
        seen_tip: u64,
        certificate_tip: u64,
        window: u64,
    },

    /// Finality producer: the node the certificate is being built from is a different chain.
    ///
    /// A receipt is only evidence about the chain that produced it. A reader pointed at the wrong
    /// network — or at a node answering for a fork with its own chain id — must refuse *for the
    /// chain*, before any depth is considered, so a foreign chain's receipt cannot be graded as a
    /// shallow-but-valid one.
    FinalityChainIdMismatch {
        expected: u64,
        found: u64,
    },

    /// Finality producer: the receipt's block hash is not the hash of the block at its own height.
    ///
    /// The receipt names both the height and the hash of the block a transaction is anchored in.
    /// If the chain's block at that height carries a different hash, the receipt does not describe
    /// this chain's history at that height — it is refused, never repaired to the chain's hash.
    FinalityBlockHashMismatch {
        block_height: u64,
        receipt_hash: alloc::string::String,
        block_hash: alloc::string::String,
    },

    /// Finality producer: the block returned for a height reports a different height.
    ///
    /// A node that answers `eth_getBlockByNumber(n)` with a block numbered `m != n` is not a
    /// trustworthy reader for this purpose; the mismatch is refused rather than trusted.
    FinalityBlockNumberMismatch {
        receipt_number: u64,
        block_number: u64,
    },

    /// Finality store: the persisted tip ledger could not be read or written.
    FinalityTipStore(alloc::string::String),
}

impl SwapError {
    /// Construct a generic internal error from a string.
    pub fn generic(msg: impl Into<alloc::string::String>) -> Self {
        SwapError::Internal(msg.into())
    }
}

impl core::fmt::Display for SwapError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            SwapError::MissingField { field } => write!(f, "Missing required field: {}", field),
            SwapError::InvalidTimeoutOrdering {
                destination_timeout,
                source_timeout,
            } => {
                write!(
                    f,
                    "Invalid timeout ordering: destination_timeout({}) >= source_timeout({})",
                    destination_timeout, source_timeout
                )
            }
            SwapError::AlreadyTerminal { status } => {
                write!(f, "Intent already in terminal status: {:?}", status)
            }
            SwapError::InvalidTransition { from, to } => {
                write!(f, "Invalid status transition: {:?} -> {:?}", from, to)
            }
            SwapError::HashlockMismatch => {
                write!(f, "Hashlock mismatch: preimage does not match hash")
            }
            SwapError::InsufficientFinality {
                chain,
                required,
                actual,
            } => {
                write!(
                    f,
                    "Insufficient finality: {:?} requires {:?}, got {:?}",
                    chain, required, actual
                )
            }
            SwapError::InsufficientRelayerQuorum { required, actual } => {
                write!(
                    f,
                    "Insufficient relayer quorum: required {}, got {}",
                    required, actual
                )
            }
            SwapError::MissingProof { proof_name } => write!(f, "Proof missing: {}", proof_name),
            SwapError::ProofVerificationFailed { proof_name, reason } => {
                write!(f, "Proof verification failed: {} — {}", proof_name, reason)
            }
            SwapError::TimeoutNotElapsed { current, timeout } => {
                write!(
                    f,
                    "Timeout not yet elapsed: current {} < timeout {}",
                    current, timeout
                )
            }
            SwapError::AlreadyLocked { chain } => {
                write!(f, "Swap already has a lock tx on {:?}", chain)
            }
            SwapError::NotLockable { status } => {
                write!(f, "Swap is not in a lockable state: {:?}", status)
            }
            SwapError::InvalidRefundPath { reason } => write!(f, "Refund path invalid: {}", reason),
            SwapError::TxNotFound { tx_hash } => write!(f, "Transaction not found: {}", tx_hash),
            SwapError::InvalidPreimageLength { expected, actual } => {
                write!(
                    f,
                    "Invalid preimage length: expected {}, got {}",
                    expected, actual
                )
            }
            SwapError::LockFailed { reason } => write!(f, "Lock failed: {}", reason),
            SwapError::SourceLockFailed { reason } => write!(f, "Source lock failed: {}", reason),
            SwapError::FinalityNotMet {
                chain,
                required,
                current,
            } => {
                write!(
                    f,
                    "Finality not met on {}: required {} confirmations, got {}",
                    chain, required, current
                )
            }
            SwapError::ProofNotFound {
                proof_id,
                intent_id,
            } => {
                write!(
                    f,
                    "Proof not found: id={} for intent={}",
                    proof_id, intent_id
                )
            }
            SwapError::MissingTxHash { step, chain } => {
                write!(
                    f,
                    "Missing transaction hash for step '{}' on chain '{}'",
                    step, chain
                )
            }
            SwapError::ClaimFailed { chain, reason } => {
                write!(f, "Claim failed on {}: {}", chain, reason)
            }
            SwapError::RefundFailed { chain, reason } => {
                write!(f, "Refund failed on {}: {}", chain, reason)
            }
            SwapError::Internal(msg) => write!(f, "Internal error: {}", msg),
            SwapError::SlashNotFound { slash_id } => {
                write!(f, "Slash case not found: slash_id={}", slash_id)
            }
            SwapError::InsufficientEvidence { minimum, actual } => {
                write!(
                    f,
                    "Insufficient evidence: minimum {} bytes, got {} bytes",
                    minimum, actual
                )
            }
            SwapError::InsufficientStake {
                actor,
                available,
                required,
            } => {
                write!(
                    f,
                    "Insufficient stake for {}: available {}, required {}",
                    actor, available, required
                )
            }
            SwapError::InvalidSlashStatus { slash_id, reason } => {
                write!(f, "Invalid slash status for case {}: {}", slash_id, reason)
            }
            SwapError::RpcError(msg) => write!(f, "RPC call failed: {}", msg),
            SwapError::DisputeNotFound { dispute_id } => {
                write!(f, "Dispute not found: dispute_id={}", dispute_id)
            }
            SwapError::InvalidDisputeStatus { dispute_id, reason } => {
                write!(
                    f,
                    "Invalid dispute status for case {}: {}",
                    dispute_id, reason
                )
            }
            SwapError::PartialFillNotAllowed {
                amount_in,
                min_amount_out,
            } => {
                write!(
                    f,
                    "Partial fill not allowed: amount_in({}) < min_amount_out({})",
                    amount_in, min_amount_out
                )
            }
            SwapError::CertificateChainMismatch { expected, found } => {
                write!(
                    f,
                    "Finality certificate is for {:?}, not {:?}",
                    found, expected
                )
            }
            SwapError::CertificateBlockAfterObservation {
                block_height,
                observed_at,
            } => {
                write!(
                    f,
                    "Finality certificate anchors block {} but was observed at tip {}",
                    block_height, observed_at
                )
            }
            SwapError::CertificateConfirmationsOverflow {
                block_height,
                observed_at,
            } => {
                write!(
                    f,
                    "Finality certificate depth from block {} to tip {} does not fit a u32",
                    block_height, observed_at
                )
            }
            SwapError::CertificateConfirmationsDisagree {
                chain,
                block_height,
                observed_at,
                reported,
                expected,
            } => {
                write!(
                    f,
                    "Finality certificate on {:?} reports {} confirmations for block {} at tip {} — the anchor implies {}",
                    chain, reported, block_height, observed_at, expected
                )
            }
            SwapError::CertificateRewindsAcceptedAnchor {
                chain,
                accepted_tip,
                certificate_tip,
            } => {
                write!(
                    f,
                    "Finality certificate on {:?} rewinds the chain: tip {} is below the accepted tip {}",
                    chain, certificate_tip, accepted_tip
                )
            }
            SwapError::CertificateStale {
                chain,
                seen_tip,
                certificate_tip,
                window,
            } => {
                write!(
                    f,
                    "Finality certificate on {:?} is stale: tip {} is {} blocks behind the witnessed tip {} (window {})",
                    chain,
                    certificate_tip,
                    seen_tip.saturating_sub(*certificate_tip),
                    seen_tip,
                    window
                )
            }
            SwapError::FinalityChainIdMismatch { expected, found } => {
                write!(
                    f,
                    "Finality producer: node reports chain id {} but chain id {} was required",
                    found, expected
                )
            }
            SwapError::FinalityBlockHashMismatch {
                block_height,
                receipt_hash,
                block_hash,
            } => {
                write!(
                    f,
                    "Finality producer: receipt anchors block {} at {} but the chain's block at that height is {}",
                    block_height, receipt_hash, block_hash
                )
            }
            SwapError::FinalityBlockNumberMismatch {
                receipt_number,
                block_number,
            } => {
                write!(
                    f,
                    "Finality producer: receipt anchors block {} but the node returned block {} for that height",
                    receipt_number, block_number
                )
            }
            SwapError::FinalityTipStore(msg) => {
                write!(f, "Finality tip store error: {}", msg)
            }
        }
    }
}
