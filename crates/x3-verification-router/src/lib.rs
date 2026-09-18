//! X3 Verification Router — Production verification routing with strategy dispatch.
//!
//! Routes proofs to the appropriate verifier based on `VerificationStrategy`.
//! Supports EVM receipt proofs (light client or validator quorum), Solana finalized
//! commitment proofs, Bitcoin SPV proofs, and X3 internal finalized proofs.
//!
//! # Production rules
//! - TestOnly verifier is feature-gated behind `test-verifier` and MUST NOT compile
//!   in production builds.
//! - Unsupported verifier MUST fail closed.
//! - All verification results are indexed and replay-checked.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use core::marker::{Send, Sync};
use core::option::Option;
use core::option::Option::{None, Some};
use core::result::Result;
use core::result::Result::{Err, Ok};

pub mod evm_receipt;
pub mod gateway_types;

pub use evm_receipt::{
    deposit_locked_selector, withdrawal_released_selector, ProductionEvmReceiptVerifier,
    DEPOSIT_LOCKED_SELECTOR, WITHDRAWAL_RELEASED_SELECTOR,
};
pub use gateway_types::{
    ExternalAssetRef, ExternalChainId, VerificationRequest, VerificationResult,
};

use alloc::collections::btree_map::BTreeMap;
use alloc::string::ToString;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::fmt::{Display, Formatter};
use sha2::{Digest, Sha256};

// ── Compile-time guard: TestOnly verifier cannot be used in production ──────
#[cfg(all(feature = "production", feature = "test-verifier"))]
compile_error!(
    "MAINNET VIOLATION: `test-verifier` must not be enabled in production builds. \
     Use a real verifier (EVM light client, validator quorum, etc.)."
);

// ── Types ───────────────────────────────────────────────────────────────────

/// Proof source chain types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, scale_info::TypeInfo)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub enum ChainKind {
    Evm { chain_id: u64 },
    Solana,
    Bitcoin,
    X3,
}

/// Verification strategy for a given proof
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, scale_info::TypeInfo)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub enum VerificationStrategy {
    /// Only available in test builds — compile error in production
    #[cfg(feature = "test-verifier")]
    TestOnly,
    /// Validator quorum attestation (N of M validators sign)
    ValidatorQuorum { threshold: u32, total: u32 },
    /// EVM light client proof (block header chain + receipt merkle proof)
    EvmReceiptProof,
    /// Solana finalized commitment proof
    SolanaFinalizedProof,
    /// Bitcoin SPV proof (header chain + tx merkle proof)
    BitcoinSpvProof,
    /// X3 internal finalized proof (the kernel itself)
    X3Internal,
    /// Fallback — always fails closed
    Unsupported,
}

/// A proof envelope ready for verification
#[derive(Debug, Clone, PartialEq, Eq, scale_info::TypeInfo)]
pub struct ProofEnvelope {
    pub proof_id: [u8; 32],
    pub strategy: VerificationStrategy,
    pub source_chain: ChainKind,
    pub destination_chain: ChainKind,
    pub payload: alloc::vec::Vec<u8>,
    pub expected_asset_id: [u8; 32],
    pub expected_amount: u128,
    pub expected_sender: alloc::vec::Vec<u8>,
    pub expected_recipient: alloc::vec::Vec<u8>,
}

/// Outcome of a verification attempt
#[derive(Debug, Clone, PartialEq, Eq, scale_info::TypeInfo)]
pub struct VerificationOutcome {
    pub accepted: bool,
    pub reason: &'static str,
    pub verified_at_height: Option<u64>,
}

/// Verification error types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerificationError {
    MissingVerifier,
    MalformedProof,
    InvalidStrategy,
    UnsupportedChain,
    ReplayDetected,
    /// The strategy reached a verifier that has no real verification logic.
    /// Security code fails closed rather than accepting an unchecked proof.
    NotImplemented,
    /// The payload's format version is not the one this verifier implements.
    UnsupportedProofFormat,
    /// The verifier has no authorized validator set, so nothing can be verified.
    NoAuthorizedValidators,
    /// Fewer distinct authorized validators signed than the threshold requires.
    InsufficientValidSignatures,
}

impl Display for VerificationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self {
            VerificationError::MissingVerifier => {
                write!(f, "no verifier registered for proof kind")
            }
            VerificationError::MalformedProof => write!(f, "malformed proof payload"),
            VerificationError::InvalidStrategy => write!(f, "invalid verification strategy"),
            VerificationError::UnsupportedChain => write!(f, "unsupported source chain"),
            VerificationError::ReplayDetected => write!(f, "replay detected: proof already used"),
            VerificationError::NotImplemented => {
                write!(
                    f,
                    "no verifier implemented for this strategy: failing closed"
                )
            }
            VerificationError::UnsupportedProofFormat => {
                write!(f, "unsupported proof payload format")
            }
            VerificationError::NoAuthorizedValidators => {
                write!(f, "no authorized validators configured: failing closed")
            }
            VerificationError::InsufficientValidSignatures => {
                write!(f, "insufficient valid validator attestations")
            }
        }
    }
}

// ── Verifier trait ──────────────────────────────────────────────────────────

pub trait Verifier: Send + Sync {
    fn verify(&self, proof: &ProofEnvelope) -> Result<VerificationOutcome, VerificationError>;
    fn strategy(&self) -> VerificationStrategy;
}

// ── Verification Router ─────────────────────────────────────────────────────

#[derive(Default)]
pub struct VerificationRouter {
    verifiers: BTreeMap<u8, Arc<dyn Verifier>>, // keyed by strategy discriminant
    used_proofs: BTreeMap<[u8; 32], bool>,
}

impl VerificationRouter {
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a router with a given block height (used by gateway for initialization).
    pub fn at_block(_block: u64) -> Self {
        Self::default()
    }

    pub fn register_verifier(&mut self, verifier: Arc<dyn Verifier>) {
        let key = match verifier.strategy() {
            VerificationStrategy::ValidatorQuorum { .. } => 1,
            VerificationStrategy::EvmReceiptProof => 2,
            VerificationStrategy::SolanaFinalizedProof => 3,
            VerificationStrategy::BitcoinSpvProof => 4,
            VerificationStrategy::X3Internal => 5,
            #[cfg(feature = "test-verifier")]
            VerificationStrategy::TestOnly => 99,
            VerificationStrategy::Unsupported => 0,
        };
        self.verifiers.insert(key, verifier);
    }

    /// Route a proof to the correct verifier and check replay.
    pub fn route(&self, proof: &ProofEnvelope) -> Result<VerificationOutcome, VerificationError> {
        // Check replay protection
        if self
            .used_proofs
            .get(&proof.proof_id)
            .copied()
            .unwrap_or(false)
        {
            return Err(VerificationError::ReplayDetected);
        }

        // Unsupported always fails closed
        if matches!(proof.strategy, VerificationStrategy::Unsupported) {
            return Err(VerificationError::InvalidStrategy);
        }

        let key = match proof.strategy {
            VerificationStrategy::ValidatorQuorum { .. } => 1,
            VerificationStrategy::EvmReceiptProof => 2,
            VerificationStrategy::SolanaFinalizedProof => 3,
            VerificationStrategy::BitcoinSpvProof => 4,
            VerificationStrategy::X3Internal => 5,
            #[cfg(feature = "test-verifier")]
            VerificationStrategy::TestOnly => 99,
            VerificationStrategy::Unsupported => return Err(VerificationError::InvalidStrategy),
        };

        let verifier = self
            .verifiers
            .get(&key)
            .ok_or(VerificationError::MissingVerifier)?;

        verifier.verify(proof)
    }

    /// Mark a proof as used (for replay protection after successful verification)
    pub fn mark_used(&mut self, proof_id: [u8; 32]) {
        self.used_proofs.insert(proof_id, true);
    }

    /// Check if a proof has been used
    pub fn is_used(&self, proof_id: &[u8; 32]) -> bool {
        self.used_proofs.get(proof_id).copied().unwrap_or(false)
    }

    /// Gateway-facing route method — translates `VerificationRequest` into
    /// an internal `ProofEnvelope` and routes to the correct verifier.
    pub fn route_verification_request(
        &mut self,
        request: VerificationRequest,
    ) -> VerificationResult {
        let proof = ProofEnvelope {
            proof_id: request.proof_id,
            strategy: request.strategy,
            source_chain: match request.source_chain {
                gateway_types::ExternalChainId::EthereumSepolia
                | gateway_types::ExternalChainId::BaseSepolia
                | gateway_types::ExternalChainId::EthereumMainnet
                | gateway_types::ExternalChainId::BaseMainnet
                | gateway_types::ExternalChainId::Other(_) => ChainKind::Evm { chain_id: 1 },
                gateway_types::ExternalChainId::SolanaDevnet
                | gateway_types::ExternalChainId::SolanaMainnet => ChainKind::Solana,
            },
            destination_chain: ChainKind::X3,
            payload: request.proof_payload,
            expected_asset_id: [0u8; 32],
            expected_amount: request.amount,
            expected_sender: request.sender.as_bytes().to_vec(),
            expected_recipient: request.recipient.as_bytes().to_vec(),
        };

        match self.route(&proof) {
            Ok(outcome) => VerificationResult {
                proof_id: request.proof_id,
                verified: outcome.accepted,
                failure_reason: if outcome.accepted {
                    None
                } else {
                    Some(outcome.reason.to_string())
                },
                chain: request.source_chain,
                verified_at_block: outcome.verified_at_height,
                strategy: request.strategy,
                confidence_bps: if outcome.accepted { 10_000 } else { 0 },
            },
            Err(e) => VerificationResult {
                proof_id: request.proof_id,
                verified: false,
                failure_reason: Some(e.to_string()),
                chain: request.source_chain,
                verified_at_block: None,
                strategy: request.strategy,
                confidence_bps: 0,
            },
        }
    }
}

// ── EVM Receipt Proof Verifier ──────────────────────────────────────────────

pub struct EvmReceiptVerifier {
    pub min_confirmations: u64,
}

impl EvmReceiptVerifier {
    pub fn new(min_confirmations: u64) -> Self {
        Self { min_confirmations }
    }
}

impl Verifier for EvmReceiptVerifier {
    fn strategy(&self) -> VerificationStrategy {
        VerificationStrategy::EvmReceiptProof
    }

    /// **Fails closed.** This is the legacy structural check: it only looked at
    /// payload length and chain kind, so 64 arbitrary bytes were "verified". The
    /// real EVM verification lives in `evm_receipt::ProductionEvmReceiptVerifier`
    /// (RLP decode + merkle-patricia proof + confirmations) and is what
    /// production code must register.
    ///
    /// `test-verifier` (without `production`) opts back into the old permissive
    /// behaviour for plumbing tests; `production` always wins.
    fn verify(&self, proof: &ProofEnvelope) -> Result<VerificationOutcome, VerificationError> {
        if proof.payload.is_empty() || proof.payload.len() < 64 {
            return Err(VerificationError::MalformedProof);
        }

        #[cfg(all(feature = "test-verifier", not(feature = "production")))]
        {
            match proof.source_chain {
                ChainKind::Evm { chain_id: _ } => {}
                _ => return Err(VerificationError::UnsupportedChain),
            }
            let _ = self.min_confirmations;
            Ok(VerificationOutcome {
                accepted: true,
                reason: "evm_receipt_proof_structural_only_test_verifier",
                verified_at_height: None,
            })
        }

        #[cfg(not(all(feature = "test-verifier", not(feature = "production"))))]
        {
            let _ = (self.min_confirmations, proof);
            Err(VerificationError::NotImplemented)
        }
    }
}

// ── Validator Quorum Verifier ───────────────────────────────────────────────

pub struct ValidatorQuorumVerifier {
    pub threshold: u32,
    pub total_validators: u32,
}

impl ValidatorQuorumVerifier {
    pub fn new(threshold: u32, total: u32) -> Self {
        Self {
            threshold,
            total_validators: total,
        }
    }
}

impl Verifier for ValidatorQuorumVerifier {
    fn strategy(&self) -> VerificationStrategy {
        VerificationStrategy::ValidatorQuorum {
            threshold: self.threshold,
            total: self.total_validators,
        }
    }

    /// **Fails closed.** Counting attestations without verifying their signatures
    /// accepts any non-empty payload, which is not a quorum check at all. See
    /// `docs/reports/SECURITY_BLOCKERS.md` (stub verifiers, CRITICAL).
    fn verify(&self, proof: &ProofEnvelope) -> Result<VerificationOutcome, VerificationError> {
        if proof.payload.is_empty() {
            return Err(VerificationError::MalformedProof);
        }

        #[cfg(all(feature = "test-verifier", not(feature = "production")))]
        {
            let _ = (self.threshold, self.total_validators, proof);
            Ok(VerificationOutcome {
                accepted: true,
                reason: "validator_quorum_structural_only_test_verifier",
                verified_at_height: None,
            })
        }

        #[cfg(not(all(feature = "test-verifier", not(feature = "production"))))]
        {
            let _ = (self.threshold, self.total_validators, proof);
            Err(VerificationError::NotImplemented)
        }
    }
}

// ── Solana Finalized Proof Verifier ─────────────────────────────────────────

/// Payload format version for `VerificationStrategy::SolanaFinalizedProof`.
///
/// ```text
/// offset 0  : u8    version (= SOLANA_FINALIZED_FORMAT_V1)
/// offset 1  : u64   slot                 (little endian)
/// offset 9  : [32]  blockhash
/// offset 41 : u32   attestation count    (little endian)
/// offset 45 : count * ( [32] ed25519 pubkey || [64] ed25519 signature )
/// ```
///
/// Each attestation signs `BLAKE2b-256(slot_le || blockhash)` — the message the
/// relayer produces in `Submitter::sign_proof_payload`. Signers are matched
/// against the verifier's authorized set, counted once each regardless of how
/// often they appear, and the count must reach the threshold.
pub const SOLANA_FINALIZED_FORMAT_V1: u8 = 1;

pub const SOLANA_ATTESTATION_LEN: usize = 96;
pub const SOLANA_PAYLOAD_HEADER_LEN: usize = 45;

/// Verification of Solana finalized-commitment attestations.
///
/// Construct with the authorized validator set and the required threshold. There
/// is deliberately no default constructor that trusts the payload's own key
/// list: an empty set verifies nothing, so a caller that has not been wired to a
/// governance-controlled set fails closed.
pub struct SolanaFinalizedVerifier {
    validators: Vec<[u8; 32]>,
    threshold: u32,
}

impl SolanaFinalizedVerifier {
    pub fn new(validators: Vec<[u8; 32]>, threshold: u32) -> Self {
        Self {
            validators,
            threshold,
        }
    }

    /// No authorized validators. Every proof fails closed; use this until a
    /// governance-controlled validator set is wired in.
    pub fn empty() -> Self {
        Self {
            validators: Vec::new(),
            threshold: 1,
        }
    }

    pub fn threshold(&self) -> u32 {
        self.threshold
    }

    pub fn is_authorized(&self, pubkey: &[u8; 32]) -> bool {
        self.validators.iter().any(|known| known == pubkey)
    }
}

/// `BLAKE2b-256(slot_le || blockhash)` — the message SVM validators attest to.
pub fn solana_attestation_message(slot: u64, blockhash: &[u8; 32]) -> [u8; 32] {
    use blake2::digest::consts::U32;
    use blake2::{Blake2b, Digest};

    let mut hasher = Blake2b::<U32>::new();
    hasher.update(slot.to_le_bytes());
    hasher.update(blockhash);
    let mut out = [0u8; 32];
    out.copy_from_slice(&hasher.finalize());
    out
}

impl Verifier for SolanaFinalizedVerifier {
    fn strategy(&self) -> VerificationStrategy {
        VerificationStrategy::SolanaFinalizedProof
    }

    /// Verifies the attestations in `SOLANA_FINALIZED_FORMAT_V1`.
    ///
    /// Fails closed: unknown signers, repeated signers (counted once), malformed
    /// payloads and a threshold that cannot be met all produce an error rather
    /// than an accepted outcome.
    ///
    /// `test-verifier` only relaxes the *unconfigured* case (no validator set
    /// wired yet): there it accepts a structural proof so plumbing tests can run.
    /// A configured verifier always performs the real signature check, in every
    /// feature configuration — otherwise the test hook could hide a broken
    /// verification path.
    fn verify(&self, proof: &ProofEnvelope) -> Result<VerificationOutcome, VerificationError> {
        match proof.source_chain {
            ChainKind::Solana => {}
            _ => return Err(VerificationError::UnsupportedChain),
        }

        if proof.payload.is_empty() {
            return Err(VerificationError::MalformedProof);
        }

        // Unconditional: with no authorized set there is nothing to verify, so
        // the proof is refused in every feature configuration. `test-verifier`
        // deliberately does not relax this — a test that wants a Solana proof to
        // pass must supply real attestations from an authorized key.
        if self.validators.is_empty() || self.threshold == 0 {
            return Err(VerificationError::NoAuthorizedValidators);
        }

        let payload = &proof.payload;
        if payload.len() < SOLANA_PAYLOAD_HEADER_LEN {
            return Err(VerificationError::MalformedProof);
        }
        if payload[0] != SOLANA_FINALIZED_FORMAT_V1 {
            return Err(VerificationError::UnsupportedProofFormat);
        }

        let mut slot_bytes = [0u8; 8];
        slot_bytes.copy_from_slice(&payload[1..9]);
        let slot = u64::from_le_bytes(slot_bytes);

        let mut blockhash = [0u8; 32];
        blockhash.copy_from_slice(&payload[9..41]);

        let mut count_bytes = [0u8; 4];
        count_bytes.copy_from_slice(&payload[41..45]);
        let count = u32::from_le_bytes(count_bytes) as usize;

        let expected_len = SOLANA_PAYLOAD_HEADER_LEN + count * SOLANA_ATTESTATION_LEN;
        if payload.len() != expected_len {
            return Err(VerificationError::MalformedProof);
        }

        let message = solana_attestation_message(slot, &blockhash);
        let mut counted: alloc::collections::btree_set::BTreeSet<[u8; 32]> =
            alloc::collections::btree_set::BTreeSet::new();

        for i in 0..count {
            let start = SOLANA_PAYLOAD_HEADER_LEN + i * SOLANA_ATTESTATION_LEN;
            let mut pubkey = [0u8; 32];
            pubkey.copy_from_slice(&payload[start..start + 32]);

            if !self.is_authorized(&pubkey) || counted.contains(&pubkey) {
                continue;
            }

            let mut signature = [0u8; 64];
            signature.copy_from_slice(&payload[start + 32..start + SOLANA_ATTESTATION_LEN]);

            if ed25519_dalek::VerifyingKey::from_bytes(&pubkey)
                .ok()
                .and_then(|key| {
                    ed25519_dalek::Signature::from_slice(&signature)
                        .ok()
                        .map(|sig| key.verify_strict(&message, &sig).is_ok())
                })
                .unwrap_or(false)
            {
                counted.insert(pubkey);
            }
        }

        if (counted.len() as u32) < self.threshold {
            return Err(VerificationError::InsufficientValidSignatures);
        }

        Ok(VerificationOutcome {
            accepted: true,
            reason: "solana_finalized_attestations_verified",
            verified_at_height: Some(slot),
        })
    }
}

// ── X3 Internal Verifier ────────────────────────────────────────────────────

pub struct X3InternalVerifier;

impl Verifier for X3InternalVerifier {
    fn strategy(&self) -> VerificationStrategy {
        VerificationStrategy::X3Internal
    }

    /// Internal-only strategy: an X3-internal transfer is proven by the kernel
    /// itself, so there is nothing external to verify. The router only routes
    /// `VerificationStrategy::X3Internal` proofs here, and this verifier must
    /// never be registered under an external-chain strategy.
    fn verify(&self, _proof: &ProofEnvelope) -> Result<VerificationOutcome, VerificationError> {
        Ok(VerificationOutcome {
            accepted: true,
            reason: "x3_internal_kernel_is_the_proof",
            verified_at_height: None,
        })
    }
}

// ── Bitcoin SPV Proof Wire Format ───────────────────────────────────────────
//
// The proof payload is structured as:
//   [8 bytes: current_chain_tip_height (little-endian)]
//   [4 bytes: num_headers (little-endian)]
//   [num_headers × 80 bytes: block header chain (newest-first)]
//   [4 bytes: tx_index (little-endian)]
//   [4 bytes: num_merkle_proof_nodes (little-endian)]
//   [num_merkle_proof_nodes × 32 bytes: merkle proof hashes]
//   [32 bytes: txid being proven]

fn decode_u64_le(buf: &[u8]) -> u64 {
    let mut out = [0u8; 8];
    let n = buf.len().min(8);
    out[..n].copy_from_slice(&buf[..n]);
    u64::from_le_bytes(out)
}

fn decode_u32_le(buf: &[u8]) -> u32 {
    let mut out = [0u8; 4];
    out.copy_from_slice(&buf[..4]);
    u32::from_le_bytes(out)
}

fn sha256d(data: &[u8]) -> [u8; 32] {
    let h1 = Sha256::digest(data);
    let h2 = Sha256::digest(h1);
    let mut out = [0u8; 32];
    out.copy_from_slice(&h2);
    out
}

fn parse_btc_header(raw: &[u8]) -> Result<([u8; 32], [u8; 32]), &'static str> {
    if raw.len() != 80 {
        return Err("bitcoin header must be 80 bytes");
    }
    let mut prev_block = [0u8; 32];
    let mut merkle_root = [0u8; 32];
    prev_block.copy_from_slice(&raw[4..36]);
    merkle_root.copy_from_slice(&raw[36..68]);
    let bits = u32::from_le_bytes([raw[72], raw[73], raw[74], raw[75]]);
    let hash = sha256d(raw);
    let target = compact_to_target(bits);
    for i in (0..32).rev() {
        if hash[i] < target[i] {
            return Ok((prev_block, merkle_root));
        }
        if hash[i] > target[i] {
            return Err("proof-of-work not satisfied");
        }
    }
    Ok((prev_block, merkle_root))
}

fn compact_to_target(bits: u32) -> [u8; 32] {
    let exponent = (bits >> 24) as usize;
    let mantissa = bits & 0x00FF_FFFF;
    let mut target = [0u8; 32];
    if exponent == 0 || exponent > 34 {
        return target;
    }
    let shift = exponent.saturating_sub(3);
    if shift < 29 {
        target[shift] = ((mantissa >> 16) & 0xFF) as u8;
        target[shift + 1] = ((mantissa >> 8) & 0xFF) as u8;
        target[shift + 2] = (mantissa & 0xFF) as u8;
    }
    target
}

fn verify_btc_merkle_proof(txid: &[u8; 32], merkle_root: &[u8; 32], proof: &[u8]) -> bool {
    if proof.is_empty() {
        return txid == merkle_root;
    }
    if !proof.len().is_multiple_of(32) {
        return false;
    }
    let mut hash = *txid;
    for chunk in proof.chunks(32) {
        let mut sibling = [0u8; 32];
        sibling.copy_from_slice(chunk);
        let combined = if hash <= sibling {
            [hash.as_slice(), sibling.as_slice()].concat()
        } else {
            [sibling.as_slice(), hash.as_slice()].concat()
        };
        hash = sha256d(&combined);
    }
    hash == *merkle_root
}

// ── Bitcoin SPV Verifier ────────────────────────────────────────────────────
//
// Integrates with `x3-bitcoin-vault` so confirmation thresholds and the
// signer-threshold configuration come from a single canonical source rather
// than being hard-coded in the verifier. This promotes the previously-orphan
// `x3-bitcoin-vault` crate to a real compiled consumer of the production
// Bitcoin SPV path used by `pallet_x3_crosschain_gateway`.

/// Verifies Bitcoin SPV proofs: header-chain linkage (`sha256d`), the merkle
/// proof against the last header's merkle root, and the confirmation count.
///
/// **Vault signer approvals are not verified.** An earlier version carried
/// `vault_threshold` / `vault_total_signers` and documented that SPV-verified
/// deposits must be backed by that many vault signers — but `verify` never read
/// those fields, so the policy only looked enforced. They were **removed**
/// rather than "enforced" by counting approvals, because
/// `BtcVault::add_signer_approval` stores signer signature bytes *without
/// verifying them* (issue #272), so counting them would prove nothing.
///
/// Deposits are therefore credited on the SPV proof plus confirmations. If
/// vault-signer authorization is wanted, it needs a signed message format and a
/// payload extension — exactly what the Solana path got in
/// `SOLANA_FINALIZED_FORMAT_V1`.
pub struct BitcoinSpvVerifier {
    pub min_confirmations: u64,
}

impl BitcoinSpvVerifier {
    /// Default constructor: takes the confirmation policy from
    /// `x3-bitcoin-vault`. Prefer this over `BitcoinSpvVerifier::new(6)` so the
    /// confirmation policy stays centralized.
    pub fn from_vault_defaults() -> Self {
        Self {
            min_confirmations: x3_bitcoin_vault::MIN_BITCOIN_CONFIRMATIONS,
        }
    }

    pub fn new(min_confirmations: u64) -> Self {
        Self { min_confirmations }
    }

    /// Adopt the vault's confirmation policy. The vault's signer set is *not*
    /// adopted, because this verifier cannot check signer approvals (see the
    /// type docs and issue #272).
    pub fn with_vault_config(mut self, config: &x3_bitcoin_vault::BtcVaultConfig) -> Self {
        self.min_confirmations = config.min_confirmations;
        self
    }
}

impl Verifier for BitcoinSpvVerifier {
    fn strategy(&self) -> VerificationStrategy {
        VerificationStrategy::BitcoinSpvProof
    }

    fn verify(&self, proof: &ProofEnvelope) -> Result<VerificationOutcome, VerificationError> {
        match proof.source_chain {
            ChainKind::Bitcoin => {}
            _ => return Err(VerificationError::UnsupportedChain),
        }

        let payload = &proof.payload;
        if payload.len() < 16 {
            return Err(VerificationError::MalformedProof);
        }

        // Parse current chain tip (block number that the verifier knows about)
        let chain_tip = decode_u64_le(&payload[0..8]);
        let num_headers = decode_u32_le(&payload[8..12]) as usize;

        let mut offset = 12usize;
        if payload.len() < offset + num_headers * 80 + 4 + 4 {
            return Err(VerificationError::MalformedProof);
        }

        // Verify header chain (newest first order)
        let mut prev_hash: Option<[u8; 32]> = None;
        let mut last_merkle_root = [0u8; 32];
        for i in 0..num_headers {
            let h_start = offset + i * 80;
            if h_start + 80 > payload.len() {
                return Err(VerificationError::MalformedProof);
            }
            let raw = &payload[h_start..h_start + 80];
            let (prev_block, merkle_root) =
                parse_btc_header(raw).map_err(|_| VerificationError::MalformedProof)?;
            if let Some(p) = prev_hash {
                if prev_block != p {
                    return Err(VerificationError::MalformedProof);
                }
            }
            prev_hash = Some(sha256d(raw));
            if i == num_headers - 1 {
                last_merkle_root = merkle_root;
            }
        }

        offset += num_headers * 80;

        // Parse tx_index and merkle proof
        if payload.len() < offset + 8 {
            return Err(VerificationError::MalformedProof);
        }
        let tx_index = decode_u32_le(&payload[offset..offset + 4]);
        let num_proof_nodes = decode_u32_le(&payload[offset + 4..offset + 8]) as usize;
        offset += 8;

        if payload.len() < offset + num_proof_nodes * 32 + 32 {
            return Err(VerificationError::MalformedProof);
        }

        let proof_nodes = &payload[offset..offset + num_proof_nodes * 32];
        offset += num_proof_nodes * 32;

        let mut txid = [0u8; 32];
        txid.copy_from_slice(&payload[offset..offset + 32]);

        // Verify merkle proof
        if !verify_btc_merkle_proof(&txid, &last_merkle_root, proof_nodes) {
            return Err(VerificationError::MalformedProof);
        }

        // Check confirmations
        if chain_tip < tx_index as u64 {
            return Err(VerificationError::MalformedProof);
        }
        let confirmations = chain_tip - tx_index as u64 + 1;
        if confirmations < self.min_confirmations {
            return Err(VerificationError::MalformedProof);
        }

        Ok(VerificationOutcome {
            accepted: true,
            reason: "bitcoin_spv_proof_verified",
            verified_at_height: Some(tx_index as u64),
        })
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    fn dummy_proof(strategy: VerificationStrategy) -> ProofEnvelope {
        ProofEnvelope {
            proof_id: [1u8; 32],
            strategy,
            source_chain: ChainKind::Evm { chain_id: 1 },
            destination_chain: ChainKind::X3,
            payload: vec![1u8; 96],
            expected_asset_id: [0u8; 32],
            expected_amount: 1000,
            expected_sender: vec![0x01],
            expected_recipient: vec![0x02; 20],
        }
    }

    #[test]
    fn unsupported_strategy_fails() {
        let router = VerificationRouter::new();
        let proof = dummy_proof(VerificationStrategy::Unsupported);
        let result = router.route(&proof);
        assert!(matches!(result, Err(VerificationError::InvalidStrategy)));
    }

    #[test]
    fn missing_verifier_fails() {
        let router = VerificationRouter::new();
        let proof = dummy_proof(VerificationStrategy::EvmReceiptProof);
        let result = router.route(&proof);
        assert!(matches!(result, Err(VerificationError::MissingVerifier)));
    }

    /// The permissive path is opt-in (`test-verifier`, and never together with
    /// `production`). These three tests document that the opt-in exists; the
    /// fail-closed tests below cover what production builds do.
    #[cfg(all(feature = "test-verifier", not(feature = "production")))]
    #[test]
    fn evm_receipt_verifier_works_under_test_verifier() {
        let mut router = VerificationRouter::new();
        router.register_verifier(Arc::new(EvmReceiptVerifier::new(12)));

        let proof = dummy_proof(VerificationStrategy::EvmReceiptProof);
        let outcome = router.route(&proof).expect("should verify");
        assert!(outcome.accepted);
    }

    #[cfg(all(feature = "test-verifier", not(feature = "production")))]
    #[test]
    fn validator_quorum_works_under_test_verifier() {
        let mut router = VerificationRouter::new();
        router.register_verifier(Arc::new(ValidatorQuorumVerifier::new(3, 5)));

        let proof = dummy_proof(VerificationStrategy::ValidatorQuorum {
            threshold: 3,
            total: 5,
        });
        let outcome = router.route(&proof).expect("should verify");
        assert!(outcome.accepted);
    }

    /// What a production build does: every strategy without a real verifier
    /// refuses the proof. This is the posture the audit requires
    /// (`docs/reports/SECURITY_BLOCKERS.md`); the acceptance test that enforces
    /// it from outside the crate is
    /// `audit-artifacts/mainnet-readiness/2026-09-05-6a24d8cf-audit/audit-harness/proof`.
    #[cfg(not(all(feature = "test-verifier", not(feature = "production"))))]
    #[test]
    fn unimplemented_strategies_fail_closed() {
        let well_formed = vec![1u8; 96];

        let mut evm_router = VerificationRouter::new();
        evm_router.register_verifier(Arc::new(EvmReceiptVerifier::new(12)));
        let mut evm_proof = dummy_proof(VerificationStrategy::EvmReceiptProof);
        evm_proof.payload = vec![1u8; 64];
        assert!(
            matches!(
                evm_router.route(&evm_proof),
                Err(VerificationError::NotImplemented)
            ),
            "the legacy structural EVM verifier must not accept unverified bytes"
        );

        let mut quorum_router = VerificationRouter::new();
        quorum_router.register_verifier(Arc::new(ValidatorQuorumVerifier::new(3, 5)));
        let mut quorum_proof = dummy_proof(VerificationStrategy::ValidatorQuorum {
            threshold: 3,
            total: 5,
        });
        quorum_proof.payload = well_formed.clone();
        assert!(
            matches!(
                quorum_router.route(&quorum_proof),
                Err(VerificationError::NotImplemented)
            ),
            "an unsigned one-byte quorum proof must not pass"
        );

        let mut solana_router = VerificationRouter::new();
        // No governance-controlled validator set is wired to this verifier yet,
        // so it must refuse every proof rather than accept it unchecked.
        solana_router.register_verifier(Arc::new(SolanaFinalizedVerifier::empty()));
        let mut solana_proof = dummy_proof(VerificationStrategy::SolanaFinalizedProof);
        solana_proof.source_chain = ChainKind::Solana;
        let mut well_formed = vec![SOLANA_FINALIZED_FORMAT_V1];
        well_formed.extend_from_slice(&1u64.to_le_bytes());
        well_formed.extend_from_slice(&[0u8; 32]);
        well_formed.extend_from_slice(&0u32.to_le_bytes());
        solana_proof.payload = well_formed;
        assert!(
            matches!(
                solana_router.route(&solana_proof),
                Err(VerificationError::NoAuthorizedValidators)
            ),
            "an unconfigured Solana verifier must fail closed"
        );
    }

    #[test]
    fn bitcoin_spv_works() {
        let mut router = VerificationRouter::new();
        router.register_verifier(Arc::new(BitcoinSpvVerifier::new(6)));

        let mut proof = dummy_proof(VerificationStrategy::BitcoinSpvProof);
        proof.source_chain = ChainKind::Bitcoin;

        // Build a valid SPV proof payload:
        // [8 bytes: chain_tip = 200]
        // [4 bytes: num_headers = 1]
        // [80 bytes: header with easy target (nBits = 0x1D00FFFF)]
        // [4 bytes: tx_index = 100]
        // [4 bytes: num_proof_nodes = 0]
        // [32 bytes: txid = all zeros]
        let mut payload = Vec::new();
        payload.extend_from_slice(&200u64.to_le_bytes());
        payload.extend_from_slice(&1u32.to_le_bytes());

        let mut header = [0u8; 80];
        // Pre-computed valid header: bits=0x1EFFFFFF, nonce=2561 → hash meets target
        header[72..76].copy_from_slice(&[0xFF, 0xFF, 0xFF, 0x1E]);
        header[76..80].copy_from_slice(&2561u32.to_le_bytes());
        payload.extend_from_slice(&header);

        payload.extend_from_slice(&100u32.to_le_bytes());
        payload.extend_from_slice(&0u32.to_le_bytes());
        payload.extend_from_slice(&[0u8; 32]);

        proof.payload = payload;
        let outcome = router.route(&proof).expect("should verify");
        assert!(outcome.accepted);
    }

    #[test]
    fn bitcoin_spv_below_confirmation_threshold_rejected() {
        // Same proof as `bitcoin_spv_works` (chain tip 200, tx at index 100 →
        // 101 confirmations) but a verifier that demands more than the chain has.
        let mut router = VerificationRouter::new();
        router.register_verifier(Arc::new(BitcoinSpvVerifier::new(102)));

        let mut proof = dummy_proof(VerificationStrategy::BitcoinSpvProof);
        proof.source_chain = ChainKind::Bitcoin;

        let mut payload = Vec::new();
        payload.extend_from_slice(&200u64.to_le_bytes());
        payload.extend_from_slice(&1u32.to_le_bytes());
        let mut header = [0u8; 80];
        header[72..76].copy_from_slice(&[0xFF, 0xFF, 0xFF, 0x1E]);
        header[76..80].copy_from_slice(&2561u32.to_le_bytes());
        payload.extend_from_slice(&header);
        payload.extend_from_slice(&100u32.to_le_bytes());
        payload.extend_from_slice(&0u32.to_le_bytes());
        payload.extend_from_slice(&[0u8; 32]);
        proof.payload = payload;

        assert!(matches!(
            router.route(&proof),
            Err(VerificationError::MalformedProof)
        ));
    }

    #[test]
    fn x3_internal_works() {
        let mut router = VerificationRouter::new();
        router.register_verifier(Arc::new(X3InternalVerifier));

        let proof = dummy_proof(VerificationStrategy::X3Internal);
        let outcome = router.route(&proof).expect("should verify");
        assert!(outcome.accepted);
    }

    #[test]
    fn malformed_proof_fails() {
        let router = VerificationRouter::new();

        // Empty payload is checked in EvmReceiptVerifier
        let mut proof = dummy_proof(VerificationStrategy::EvmReceiptProof);
        proof.payload = vec![];

        // Since no verifier is registered, it should return MissingVerifier
        let result = router.route(&proof);
        assert!(matches!(result, Err(VerificationError::MissingVerifier)));
    }

    // ── Solana finalized-proof verifier (real Ed25519 attestations) ─────────

    /// Builds a `SOLANA_FINALIZED_FORMAT_V1` payload signed by the given keys.
    /// Mirrors what the relayer's `sign_proof_payload` produces: each signature
    /// is over `BLAKE2b-256(slot_le || blockhash)`.
    fn solana_payload(
        slot: u64,
        blockhash: [u8; 32],
        signers: &[&ed25519_dalek::SigningKey],
        signed_blockhash: Option<[u8; 32]>,
        format_version: u8,
    ) -> Vec<u8> {
        use ed25519_dalek::Signer;

        let message = solana_attestation_message(slot, &signed_blockhash.unwrap_or(blockhash));
        let mut payload = Vec::new();
        payload.push(format_version);
        payload.extend_from_slice(&slot.to_le_bytes());
        payload.extend_from_slice(&blockhash);
        payload.extend_from_slice(&(signers.len() as u32).to_le_bytes());
        for key in signers {
            payload.extend_from_slice(&key.verifying_key().to_bytes());
            payload.extend_from_slice(&key.sign(&message).to_bytes());
        }
        payload
    }

    fn solana_proof(payload: Vec<u8>) -> ProofEnvelope {
        let mut proof = dummy_proof(VerificationStrategy::SolanaFinalizedProof);
        proof.source_chain = ChainKind::Solana;
        proof.payload = payload;
        proof
    }

    fn key(seed: u8) -> ed25519_dalek::SigningKey {
        ed25519_dalek::SigningKey::from_bytes(&[seed; 32])
    }

    #[test]
    fn solana_attestations_at_threshold_are_accepted() {
        let (a, b) = (key(1), key(2));
        let authorized = vec![a.verifying_key().to_bytes(), b.verifying_key().to_bytes()];
        let verifier = SolanaFinalizedVerifier::new(authorized, 2);

        let payload = solana_payload(42, [9u8; 32], &[&a, &b], None, SOLANA_FINALIZED_FORMAT_V1);
        let outcome = verifier
            .verify(&solana_proof(payload))
            .expect("two authorized attestations must verify");

        assert!(outcome.accepted);
        assert_eq!(outcome.verified_at_height, Some(42));
    }

    #[test]
    fn solana_insufficient_signatures_rejected() {
        let (a, b) = (key(1), key(2));
        let verifier = SolanaFinalizedVerifier::new(
            vec![a.verifying_key().to_bytes(), b.verifying_key().to_bytes()],
            2,
        );

        let payload = solana_payload(42, [9u8; 32], &[&a], None, SOLANA_FINALIZED_FORMAT_V1);
        assert!(matches!(
            verifier.verify(&solana_proof(payload)),
            Err(VerificationError::InsufficientValidSignatures)
        ));
    }

    #[test]
    fn solana_duplicate_signer_counts_once() {
        let (a, b) = (key(1), key(2));
        let verifier = SolanaFinalizedVerifier::new(
            vec![a.verifying_key().to_bytes(), b.verifying_key().to_bytes()],
            2,
        );

        // The same authorized validator twice is still one validator.
        let payload = solana_payload(42, [9u8; 32], &[&a, &a], None, SOLANA_FINALIZED_FORMAT_V1);
        assert!(matches!(
            verifier.verify(&solana_proof(payload)),
            Err(VerificationError::InsufficientValidSignatures)
        ));
    }

    #[test]
    fn solana_unknown_signer_is_not_counted() {
        let (known, rogue) = (key(1), key(7));
        let verifier = SolanaFinalizedVerifier::new(vec![known.verifying_key().to_bytes()], 1);

        let payload = solana_payload(42, [9u8; 32], &[&rogue], None, SOLANA_FINALIZED_FORMAT_V1);
        assert!(
            matches!(
                verifier.verify(&solana_proof(payload)),
                Err(VerificationError::InsufficientValidSignatures)
            ),
            "an unauthorized key must never satisfy the threshold"
        );
    }

    #[test]
    fn solana_signature_over_other_blockhash_rejected() {
        let a = key(1);
        let verifier = SolanaFinalizedVerifier::new(vec![a.verifying_key().to_bytes()], 1);

        // Signatures are over a different blockhash than the payload claims.
        let payload = solana_payload(
            42,
            [9u8; 32],
            &[&a],
            Some([8u8; 32]),
            SOLANA_FINALIZED_FORMAT_V1,
        );
        assert!(matches!(
            verifier.verify(&solana_proof(payload)),
            Err(VerificationError::InsufficientValidSignatures)
        ));
    }

    #[test]
    fn solana_tampered_signature_rejected() {
        let a = key(1);
        let verifier = SolanaFinalizedVerifier::new(vec![a.verifying_key().to_bytes()], 1);

        let mut payload = solana_payload(42, [9u8; 32], &[&a], None, SOLANA_FINALIZED_FORMAT_V1);
        // Flip a bit in the last byte of the 64-byte signature.
        let last = payload.len() - 1;
        payload[last] ^= 0x01;

        assert!(matches!(
            verifier.verify(&solana_proof(payload)),
            Err(VerificationError::InsufficientValidSignatures)
        ));
    }

    #[test]
    fn solana_format_version_and_validator_set_are_enforced() {
        let a = key(1);
        let verifier = SolanaFinalizedVerifier::new(vec![a.verifying_key().to_bytes()], 1);

        let wrong_version = solana_payload(42, [9u8; 32], &[&a], None, 0x02);
        assert!(matches!(
            verifier.verify(&solana_proof(wrong_version)),
            Err(VerificationError::UnsupportedProofFormat)
        ));

        let payload = solana_payload(42, [9u8; 32], &[&a], None, SOLANA_FINALIZED_FORMAT_V1);
        // Outside `test-verifier` an unconfigured verifier must refuse: there is
        // no validator set to check the attestations against.
        assert!(matches!(
            SolanaFinalizedVerifier::empty().verify(&solana_proof(payload.clone())),
            Err(VerificationError::NoAuthorizedValidators)
        ));

        let mut wrong_chain = solana_proof(payload);
        wrong_chain.source_chain = ChainKind::Bitcoin;
        assert!(matches!(
            verifier.verify(&wrong_chain),
            Err(VerificationError::UnsupportedChain)
        ));
    }

    #[test]
    fn solana_truncated_payload_rejected() {
        let a = key(1);
        let verifier = SolanaFinalizedVerifier::new(vec![a.verifying_key().to_bytes()], 1);
        let mut payload = solana_payload(42, [9u8; 32], &[&a], None, SOLANA_FINALIZED_FORMAT_V1);
        payload.truncate(payload.len() - 1);

        assert!(matches!(
            verifier.verify(&solana_proof(payload)),
            Err(VerificationError::MalformedProof)
        ));
    }
}
