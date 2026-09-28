//! # FinalityOracle — Chain-specific finality verification for atomic swaps.
//!
//! Provides a trait-based finality oracle that maps chains to their required
//! finality parameters and verifies whether on-chain data meets those thresholds.
//!
//! ## Mapping
//!
//! | Spec Chain | ChainKind variant |
//! |------------|-------------------|
//! | EVM        | `ChainKind::Ethereum` |
//! | Solana     | `ChainKind::Solana` |
//! | Bitcoin    | `ChainKind::Bitcoin` |
//! | Substrate  | `ChainKind::X3` (X3 runtime is Substrate-based) |
//! | Cosmos     | `ChainKind::Cosmos` |
//!
//! ## Why a certificate, and not a number
//!
//! This module used to decide finality from two caller-supplied integers —
//! `verify_finality(chain, current_confirms, commitment)` — and `FinalityCheckData::block_height`
//! was never read at all. A caller could type `12` and the oracle answered `Ok(true)`; nothing
//! bound the count to a block, so a swap's safety rested on a value the counterparty chose.
//!
//! [`FinalityCertificate`] replaces that. It names the chain, the block the transaction is anchored
//! in (`block_height` and `block_hash`), the transaction (`tx_id`), the chain tip the observation
//! was made at (`observed_at`), and the resulting `confirmations` — which is *derived*, never
//! supplied: `confirmations == observed_at - block_height + 1`. A certificate whose count disagrees
//! with its own anchor cannot be built (`SwapError::CertificateConfirmationsDisagree`).
//!
//! The certificate does not, by itself, prove that `block_hash` is the block at `block_height` on
//! `chain`; binding the hash to the chain is the reader's job (an RPC quorum, a light client, a
//! receipt proof). What the type removes is a caller *inventing* confirmations for a transaction it
//! never anchored, and — through the oracle's remembered tips — accepting a rewound or stale anchor
//! as though it were fresh.

use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};

use crate::error::SwapError;
use crate::intent::ChainKind;
use serde::{Deserialize, Serialize};

/// Default number of observed-chain blocks after which an observation is treated as stale.
///
/// A certificate is stale when the tallest tip the oracle has witnessed sits more than this many
/// blocks above the certificate's own `observed_at`: it describes a chain state far enough behind
/// one the oracle has already seen that it may no longer be canonical. The default is deliberately
/// generous — it discards hours of EVM history, not seconds — and operators set it per chain with
/// [`InMemoryFinalityOracle::set_staleness_window`].
pub const DEFAULT_CERTIFICATE_STALENESS_BLOCKS: u64 = 512;

/// Whether a chain decides finality from a block depth (`confirmations`) rather than from a
/// commitment level (Solana), a GRANDPA round count (`X3`), or a Tendermint block count (Cosmos).
pub fn is_confirmation_based(chain: ChainKind) -> bool {
    matches!(
        chain,
        ChainKind::Ethereum
            | ChainKind::Base
            | ChainKind::Arbitrum
            | ChainKind::Optimism
            | ChainKind::Bsc
            | ChainKind::Polygon
            | ChainKind::Avalanche
            | ChainKind::Bitcoin
    )
}

/// Chain-specific finality configuration.
#[derive(Debug, Clone)]
pub struct FinalityConfig {
    /// The chain this config applies to.
    pub chain: ChainKind,
    /// Number of block confirmations required (EVM, Bitcoin, and general PoW chains).
    pub confirmations: u32,
    /// Solana commitment level (e.g. "finalized", "confirmed"). Ignored for other chains.
    pub commitment_level: String,
    /// Number of GRANDPA rounds to wait (Substrate placeholder).
    pub grandpa_rounds: u32,
    /// Number of Tendermint blocks to wait (Cosmos placeholder).
    pub tendermint_blocks: u32,
    /// How far behind a witnessed tip a certificate may be before it is refused as stale.
    pub certificate_staleness_blocks: u64,
}

impl FinalityConfig {
    /// Create an EVM (Ethereum) finality config with the default 12 confirmations.
    pub fn evm() -> Self {
        Self {
            chain: ChainKind::Ethereum,
            confirmations: 12,
            commitment_level: String::new(),
            grandpa_rounds: 0,
            tendermint_blocks: 0,
            certificate_staleness_blocks: DEFAULT_CERTIFICATE_STALENESS_BLOCKS,
        }
    }

    /// Create a Solana finality config with the given commitment level.
    pub fn solana(commitment_level: &str) -> Self {
        Self {
            chain: ChainKind::Solana,
            confirmations: 0,
            commitment_level: commitment_level.to_string(),
            grandpa_rounds: 0,
            tendermint_blocks: 0,
            certificate_staleness_blocks: DEFAULT_CERTIFICATE_STALENESS_BLOCKS,
        }
    }

    /// Create a Bitcoin finality config with the default 6 confirmations.
    pub fn bitcoin() -> Self {
        Self {
            chain: ChainKind::Bitcoin,
            confirmations: 6,
            commitment_level: String::new(),
            grandpa_rounds: 0,
            tendermint_blocks: 0,
            certificate_staleness_blocks: DEFAULT_CERTIFICATE_STALENESS_BLOCKS,
        }
    }

    /// Create a Substrate (X3) finality config (placeholder).
    pub fn substrate() -> Self {
        Self {
            chain: ChainKind::X3,
            confirmations: 0,
            commitment_level: String::new(),
            grandpa_rounds: 1,
            tendermint_blocks: 0,
            certificate_staleness_blocks: DEFAULT_CERTIFICATE_STALENESS_BLOCKS,
        }
    }

    /// Create a Cosmos (Tendermint) finality config (placeholder).
    pub fn cosmos() -> Self {
        Self {
            chain: ChainKind::Cosmos,
            confirmations: 0,
            commitment_level: String::new(),
            grandpa_rounds: 0,
            tendermint_blocks: 1,
            certificate_staleness_blocks: DEFAULT_CERTIFICATE_STALENESS_BLOCKS,
        }
    }

    /// Override the staleness window (builder style).
    pub fn with_staleness_window(mut self, blocks: u64) -> Self {
        self.certificate_staleness_blocks = blocks;
        self
    }
}

/// An observation that a transaction is anchored in a block, at a depth its own anchor implies.
///
/// The invariant is enforced at construction and re-checked on every verification:
/// `confirmations == observed_at - block_height + 1`. The fields are private so the only ways to
/// build one are [`FinalityCertificate::observe`] and
/// [`FinalityCertificate::with_reported_confirmations`], both of which refuse an anchor above the
/// tip it claims to have been observed at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FinalityCertificate {
    chain: ChainKind,
    block_height: u64,
    block_hash: [u8; 32],
    tx_id: [u8; 32],
    confirmations: u32,
    observed_at: u64,
    commitment_level: String,
}

impl FinalityCertificate {
    /// The number of confirmations a block at `block_height` has when the chain tip is `observed_at`.
    ///
    /// The anchored block itself counts as the first confirmation, so a block at the tip has one.
    pub fn confirmations_at(block_height: u64, observed_at: u64) -> Result<u32, SwapError> {
        let behind = observed_at.checked_sub(block_height).ok_or(
            SwapError::CertificateBlockAfterObservation {
                block_height,
                observed_at,
            },
        )?;
        let depth = behind
            .checked_add(1)
            .ok_or(SwapError::CertificateConfirmationsOverflow {
                block_height,
                observed_at,
            })?;
        u32::try_from(depth).map_err(|_| SwapError::CertificateConfirmationsOverflow {
            block_height,
            observed_at,
        })
    }

    /// Build a certificate observed at `observed_at`, deriving the depth from the anchor.
    pub fn observe(
        chain: ChainKind,
        block_height: u64,
        block_hash: [u8; 32],
        tx_id: [u8; 32],
        observed_at: u64,
    ) -> Result<Self, SwapError> {
        let confirmations = Self::confirmations_at(block_height, observed_at)?;
        Ok(Self {
            chain,
            block_height,
            block_hash,
            tx_id,
            confirmations,
            observed_at,
            commitment_level: String::new(),
        })
    }

    /// Build a certificate from a count the reader reports, refusing one its anchor does not imply.
    ///
    /// This is the entry point for a reader that already has a number in hand (an RPC that reports
    /// `12` confirmations, say). It is not a way to *choose* the number: a count that differs from
    /// `confirmations_at(block_height, observed_at)` is refused with
    /// [`SwapError::CertificateConfirmationsDisagree`] rather than stored.
    pub fn with_reported_confirmations(
        chain: ChainKind,
        block_height: u64,
        block_hash: [u8; 32],
        tx_id: [u8; 32],
        observed_at: u64,
        reported: u32,
    ) -> Result<Self, SwapError> {
        let expected = Self::confirmations_at(block_height, observed_at)?;
        if reported != expected {
            return Err(SwapError::CertificateConfirmationsDisagree {
                chain,
                block_height,
                observed_at,
                reported,
                expected,
            });
        }
        Ok(Self {
            chain,
            block_height,
            block_hash,
            tx_id,
            confirmations: reported,
            observed_at,
            commitment_level: String::new(),
        })
    }

    /// Attach the commitment level a commitment-based chain reported (Solana).
    pub fn with_commitment_level(mut self, level: impl Into<String>) -> Self {
        self.commitment_level = level.into();
        self
    }

    /// The chain the transaction was anchored on.
    pub fn chain(&self) -> ChainKind {
        self.chain
    }

    /// The height of the block the transaction is anchored in.
    pub fn block_height(&self) -> u64 {
        self.block_height
    }

    /// The hash of the block the transaction is anchored in.
    pub fn block_hash(&self) -> &[u8; 32] {
        &self.block_hash
    }

    /// The transaction this observation is about.
    pub fn tx_id(&self) -> &[u8; 32] {
        &self.tx_id
    }

    /// The depth of the anchor at the observed tip. Always equals
    /// `observed_at - block_height + 1`.
    pub fn confirmations(&self) -> u32 {
        self.confirmations
    }

    /// The chain tip the observation was made at.
    pub fn observed_at(&self) -> u64 {
        self.observed_at
    }

    /// The commitment level reported by a commitment-based chain, if any.
    pub fn commitment_level(&self) -> &str {
        self.commitment_level.as_str()
    }
}

/// Trait for verifying chain-specific finality of cross-chain swap transactions.
///
/// The methods take `&mut self` because an oracle remembers the tips it has seen per chain, and
/// that memory is what makes a rewind refusable rather than indistinguishable from a fresh fact.
pub trait FinalityOracle {
    /// Return the required finality configuration for a given chain.
    fn required_finality(&self, chain: ChainKind) -> FinalityConfig;

    /// Verify that a certificate meets the required finality for `chain`.
    ///
    /// Returns `Ok(true)` when finality is met. A certificate that is for another chain, that
    /// reports a depth its anchor does not imply, whose tip is below one already accepted, whose
    /// tip is stale for the configured window, or whose depth is short of the requirement, is
    /// refused with a typed error — never answered `Ok(true)`.
    fn verify_finality(
        &mut self,
        chain: ChainKind,
        certificate: &FinalityCertificate,
    ) -> Result<bool, SwapError>;

    /// Ask the same question as [`FinalityOracle::verify_finality`].
    fn is_finalized(
        &mut self,
        chain: ChainKind,
        certificate: &FinalityCertificate,
    ) -> Result<bool, SwapError> {
        self.verify_finality(chain, certificate)
    }
}

/// A simple in-memory finality oracle with default chain configurations.
///
/// Uses `ChainKind::default_safe_confirmations()` from intent.rs for EVM-like chains,
/// "finalized" for Solana, and placeholder stubs for Substrate/Cosmos.
#[derive(Debug, Clone, Default)]
pub struct InMemoryFinalityOracle {
    /// Tallest tip this oracle has accepted a *final* certificate at, per chain.
    accepted_tip: BTreeMap<ChainKind, u64>,
    /// Tallest tip this oracle has witnessed at all, per chain, whatever the verdict was.
    ///
    /// Every certificate presented is a claim about its chain's tip, so witnessing is separate
    /// from accepting: a certificate that is refused for being too shallow still tells the oracle
    /// how far the chain has come, which is what lets a *later*, older certificate be refused as
    /// stale instead of being read as the current state.
    seen_tip: BTreeMap<ChainKind, u64>,
    /// Per-chain overrides for the staleness window.
    staleness_window: BTreeMap<ChainKind, u64>,
}

impl InMemoryFinalityOracle {
    /// Create a new oracle with all-default finality configs.
    pub fn new() -> Self {
        Self::default()
    }

    /// The tallest tip this oracle has accepted a final certificate at, for `chain`.
    pub fn accepted_tip(&self, chain: ChainKind) -> Option<u64> {
        self.accepted_tip.get(&chain).copied()
    }

    /// The tallest tip this oracle has witnessed for `chain`, whatever the verdict on it was.
    pub fn seen_tip(&self, chain: ChainKind) -> Option<u64> {
        self.seen_tip.get(&chain).copied()
    }

    /// Override the staleness window for one chain on this oracle.
    pub fn set_staleness_window(&mut self, chain: ChainKind, blocks: u64) -> &mut Self {
        self.staleness_window.insert(chain, blocks);
        self
    }

    fn witness(&mut self, chain: ChainKind, tip: u64) {
        let entry = self.seen_tip.entry(chain).or_insert(tip);
        if tip > *entry {
            *entry = tip;
        }
    }

    fn accept(&mut self, chain: ChainKind, tip: u64) {
        let entry = self.accepted_tip.entry(chain).or_insert(tip);
        if tip > *entry {
            *entry = tip;
        }
    }

    /// The tips this oracle currently remembers, as a storable snapshot.
    ///
    /// This is what [`FinalityTipStore::store_tips`] persists and what
    /// [`InMemoryFinalityOracle::restore_tips`] reloads; the two are inverses.
    pub fn snapshot_tips(&self) -> BTreeMap<ChainKind, FinalityTipRecord> {
        let mut out: BTreeMap<ChainKind, FinalityTipRecord> = BTreeMap::new();
        for (chain, accepted) in &self.accepted_tip {
            out.entry(*chain).or_default().accepted_tip = Some(*accepted);
        }
        for (chain, seen) in &self.seen_tip {
            out.entry(*chain).or_default().seen_tip = Some(*seen);
        }
        out
    }

    /// Replace the remembered tips with a previously snapshotted set.
    ///
    /// Used on reload: an oracle that just started has no memory of the tips its predecessor
    /// accepted, so without this a rewind that spans a process restart would look like a fresh,
    /// lower chain rather than the rollback it is.
    pub fn restore_tips(&mut self, tips: BTreeMap<ChainKind, FinalityTipRecord>) {
        self.accepted_tip.clear();
        self.seen_tip.clear();
        for (chain, record) in tips {
            if let Some(tip) = record.accepted_tip {
                self.accepted_tip.insert(chain, tip);
            }
            if let Some(tip) = record.seen_tip {
                self.seen_tip.insert(chain, tip);
            }
        }
    }

    /// Whether a certificate meets its chain's depth rule.
    fn depth_met(
        chain: ChainKind,
        config: &FinalityConfig,
        certificate: &FinalityCertificate,
    ) -> Result<bool, SwapError> {
        match chain {
            ChainKind::Ethereum
            | ChainKind::Base
            | ChainKind::Arbitrum
            | ChainKind::Optimism
            | ChainKind::Bsc
            | ChainKind::Polygon
            | ChainKind::Avalanche
            | ChainKind::Bitcoin => {
                if certificate.confirmations() >= config.confirmations {
                    Ok(true)
                } else {
                    Err(SwapError::FinalityNotMet {
                        chain: chain.as_str().to_string(),
                        required: config.confirmations,
                        current: certificate.confirmations(),
                    })
                }
            }
            ChainKind::Solana => {
                let commitment = certificate.commitment_level();
                if commitment == "finalized"
                    || (commitment == "confirmed" && config.commitment_level == "confirmed")
                {
                    Ok(true)
                } else {
                    Err(SwapError::FinalityNotMet {
                        chain: chain.as_str().to_string(),
                        required: 0,
                        current: certificate.confirmations(),
                    })
                }
            }
            ChainKind::X3 => {
                if certificate.confirmations() >= config.grandpa_rounds {
                    Ok(true)
                } else {
                    Err(SwapError::FinalityNotMet {
                        chain: chain.as_str().to_string(),
                        required: config.grandpa_rounds,
                        current: certificate.confirmations(),
                    })
                }
            }
            ChainKind::Cosmos => {
                if certificate.confirmations() >= config.tendermint_blocks {
                    Ok(true)
                } else {
                    Err(SwapError::FinalityNotMet {
                        chain: chain.as_str().to_string(),
                        required: config.tendermint_blocks,
                        current: certificate.confirmations(),
                    })
                }
            }
        }
    }
}

impl FinalityOracle for InMemoryFinalityOracle {
    fn required_finality(&self, chain: ChainKind) -> FinalityConfig {
        let window = self
            .staleness_window
            .get(&chain)
            .copied()
            .unwrap_or(DEFAULT_CERTIFICATE_STALENESS_BLOCKS);
        let mut config = match chain {
            ChainKind::Ethereum
            | ChainKind::Base
            | ChainKind::Arbitrum
            | ChainKind::Optimism
            | ChainKind::Bsc
            | ChainKind::Polygon
            | ChainKind::Avalanche => FinalityConfig {
                chain,
                confirmations: chain.default_safe_confirmations(),
                commitment_level: String::new(),
                grandpa_rounds: 0,
                tendermint_blocks: 0,
                certificate_staleness_blocks: DEFAULT_CERTIFICATE_STALENESS_BLOCKS,
            },
            ChainKind::Bitcoin => FinalityConfig::bitcoin(),
            ChainKind::Solana => FinalityConfig::solana("finalized"),
            ChainKind::X3 => FinalityConfig::substrate(),
            ChainKind::Cosmos => FinalityConfig::cosmos(),
        };
        config.certificate_staleness_blocks = window;
        config
    }

    fn verify_finality(
        &mut self,
        chain: ChainKind,
        certificate: &FinalityCertificate,
    ) -> Result<bool, SwapError> {
        if certificate.chain() != chain {
            return Err(SwapError::CertificateChainMismatch {
                expected: chain,
                found: certificate.chain(),
            });
        }

        // Re-derive the depth from the certificate's own anchor. The constructors enforce this, but
        // a certificate decoded from bytes has not been through one, so the check is repeated here.
        let implied = FinalityCertificate::confirmations_at(
            certificate.block_height(),
            certificate.observed_at(),
        )?;
        if implied != certificate.confirmations() {
            return Err(SwapError::CertificateConfirmationsDisagree {
                chain,
                block_height: certificate.block_height(),
                observed_at: certificate.observed_at(),
                reported: certificate.confirmations(),
                expected: implied,
            });
        }

        // Witness the tip the certificate claims, whether or not its depth passes.
        self.witness(chain, certificate.observed_at());

        let config = self.required_finality(chain);

        if let Some(accepted) = self.accepted_tip(chain) {
            if certificate.observed_at() < accepted {
                return Err(SwapError::CertificateRewindsAcceptedAnchor {
                    chain,
                    accepted_tip: accepted,
                    certificate_tip: certificate.observed_at(),
                });
            }
        }

        if let Some(seen) = self.seen_tip(chain) {
            let behind = seen.saturating_sub(certificate.observed_at());
            if behind > config.certificate_staleness_blocks {
                return Err(SwapError::CertificateStale {
                    chain,
                    seen_tip: seen,
                    certificate_tip: certificate.observed_at(),
                    window: config.certificate_staleness_blocks,
                });
            }
        }

        let met = Self::depth_met(chain, &config, certificate)?;
        if met {
            self.accept(chain, certificate.observed_at());
        }
        Ok(met)
    }
}

/// The tips an oracle remembers for one chain, in a storable shape.
///
/// `accepted_tip` is the tallest tip a certificate was accepted at; `seen_tip` is the tallest tip
/// witnessed at all, whatever the verdict. Both are needed: `seen_tip` is what makes a later, older
/// certificate refusable as stale, and `accepted_tip` is what makes a lower tip refusable as a
/// rewind. Losing either across a restart re-opens the gap this type exists to close.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FinalityTipRecord {
    /// Tallest tip a final certificate was accepted at, if any.
    pub accepted_tip: Option<u64>,
    /// Tallest tip witnessed at all, if any.
    pub seen_tip: Option<u64>,
}

/// Durable storage for the tips that make a rewind refusable across a process restart.
///
/// The oracle's memory is what refuses a rewound anchor, and that memory used to live only in the
/// process that built it. This trait separates *what* must be remembered from *where*; the crate
/// that owns a process supplies the implementation (a file, a database, the proof ledger). Both
/// methods take `&self` so a store can be shared or internally mutable.
pub trait FinalityTipStore {
    /// Load every persisted tip. An empty map means nothing has been remembered yet.
    fn load_tips(&self) -> Result<BTreeMap<ChainKind, FinalityTipRecord>, SwapError>;

    /// Persist the full set of tips, replacing whatever was stored.
    fn store_tips(&self, tips: &BTreeMap<ChainKind, FinalityTipRecord>) -> Result<(), SwapError>;
}

/// A finality oracle whose remembered tips survive a process restart.
///
/// It wraps [`InMemoryFinalityOracle`] with a [`FinalityTipStore`]: the tips are loaded once when
/// the oracle is built, and written back whenever the oracle learns something. An *acceptance* is
/// only reported once it is durable — a restart must not be able to forget the tip a certificate
/// was accepted at. A *refusal* still records the witnessed tip when the store accepts the write,
/// but a store failure on that path does not turn the refusal into a success.
#[derive(Debug)]
pub struct PersistentFinalityOracle<S: FinalityTipStore> {
    inner: InMemoryFinalityOracle,
    store: S,
}

impl<S: FinalityTipStore> PersistentFinalityOracle<S> {
    /// Build an oracle from the tips the store already holds.
    pub fn load(store: S) -> Result<Self, SwapError> {
        let tips = store.load_tips()?;
        let mut inner = InMemoryFinalityOracle::new();
        inner.restore_tips(tips);
        Ok(Self { inner, store })
    }

    /// The store this oracle persists through.
    pub fn store(&self) -> &S {
        &self.store
    }

    /// The wrapped in-memory oracle, for reads that do not need persistence.
    pub fn inner(&self) -> &InMemoryFinalityOracle {
        &self.inner
    }

    /// The tallest tip this oracle has accepted a final certificate at, for `chain`.
    pub fn accepted_tip(&self, chain: ChainKind) -> Option<u64> {
        self.inner.accepted_tip(chain)
    }

    /// The tallest tip this oracle has witnessed for `chain`, whatever the verdict on it was.
    pub fn seen_tip(&self, chain: ChainKind) -> Option<u64> {
        self.inner.seen_tip(chain)
    }

    /// Override the staleness window for one chain on this oracle.
    pub fn set_staleness_window(&mut self, chain: ChainKind, blocks: u64) -> &mut Self {
        self.inner.set_staleness_window(chain, blocks);
        self
    }

    /// Flush the currently remembered tips to the store.
    pub fn persist(&self) -> Result<(), SwapError> {
        self.store.store_tips(&self.inner.snapshot_tips())
    }
}

impl<S: FinalityTipStore> FinalityOracle for PersistentFinalityOracle<S> {
    fn required_finality(&self, chain: ChainKind) -> FinalityConfig {
        self.inner.required_finality(chain)
    }

    fn verify_finality(
        &mut self,
        chain: ChainKind,
        certificate: &FinalityCertificate,
    ) -> Result<bool, SwapError> {
        let verdict = self.inner.verify_finality(chain, certificate);

        if verdict == Ok(true) {
            // A depth that passed is a fact a restart must not forget: report success only after
            // the accepted tip is durable. A store failure here fails closed.
            self.store.store_tips(&self.inner.snapshot_tips())?;
        } else {
            // The certificate was still a claim about its chain's tip, so the witness is worth
            // keeping — but a store failure on a refusal path must not mask the refusal.
            let _ = self.store.store_tips(&self.inner.snapshot_tips());
        }

        verdict
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hash(byte: u8) -> [u8; 32] {
        [byte; 32]
    }

    /// An Ethereum certificate anchored at `block_height` and observed at `observed_at`.
    fn eth(block_height: u64, observed_at: u64) -> FinalityCertificate {
        FinalityCertificate::observe(
            ChainKind::Ethereum,
            block_height,
            hash(0xAB),
            hash(0xCD),
            observed_at,
        )
        .expect("test anchors are at or below their tip")
    }

    /// Golden path: an anchor at exactly the required depth passes, and the oracle remembers it.
    #[test]
    fn test_a_certificate_at_exactly_the_required_depth_passes() {
        let mut oracle = InMemoryFinalityOracle::new();

        // Ethereum requires 12 confirmations. 1000..=1011 is exactly twelve blocks.
        let cert = eth(1000, 1011);
        assert_eq!(cert.confirmations(), 12);
        assert_eq!(
            oracle.verify_finality(ChainKind::Ethereum, &cert),
            Ok(true),
            "a certificate at exactly the required depth must pass"
        );
        assert_eq!(oracle.accepted_tip(ChainKind::Ethereum), Some(1011));

        // One block short is refused by depth, and a refusal does not advance the accepted tip.
        let mut oracle = InMemoryFinalityOracle::new();
        let shallow = eth(1000, 1010);
        assert_eq!(shallow.confirmations(), 11);
        assert_eq!(
            oracle.verify_finality(ChainKind::Ethereum, &shallow),
            Err(SwapError::FinalityNotMet {
                chain: "eth".to_string(),
                required: 12,
                current: 11,
            })
        );
        assert_eq!(oracle.accepted_tip(ChainKind::Ethereum), None);

        // `is_finalized` asks the same question.
        let mut oracle = InMemoryFinalityOracle::new();
        assert_eq!(
            oracle.is_finalized(ChainKind::Ethereum, &eth(1000, 1011)),
            Ok(true)
        );
    }

    /// Ugly path: a count its own anchor does not imply cannot be built at all.
    #[test]
    fn test_a_reported_count_the_anchor_does_not_imply_is_refused() {
        // 1005..=1011 is seven confirmations, not the twelve the caller reports.
        let refused = FinalityCertificate::with_reported_confirmations(
            ChainKind::Ethereum,
            1005,
            hash(0xAB),
            hash(0xCD),
            1011,
            12,
        )
        .expect_err("a count the anchor does not imply must be refused");
        assert_eq!(
            refused,
            SwapError::CertificateConfirmationsDisagree {
                chain: ChainKind::Ethereum,
                block_height: 1005,
                observed_at: 1011,
                reported: 12,
                expected: 7,
            }
        );

        // The same anchor with the count it does imply builds, so the refusal is the disagreement
        // and not the anchor.
        let built = FinalityCertificate::with_reported_confirmations(
            ChainKind::Ethereum,
            1005,
            hash(0xAB),
            hash(0xCD),
            1011,
            7,
        )
        .expect("seven is what 1005..=1011 implies");
        assert_eq!(built.confirmations(), 7);

        // An anchor above the tip it claims to have been observed at is refused too.
        assert_eq!(
            FinalityCertificate::observe(ChainKind::Ethereum, 1012, hash(0xAB), hash(0xCD), 1011),
            Err(SwapError::CertificateBlockAfterObservation {
                block_height: 1012,
                observed_at: 1011,
            })
        );
    }

    /// Ugly path: a certificate for another chain is not accepted for the one asked about.
    #[test]
    fn test_a_certificate_for_the_wrong_chain_is_refused() {
        let mut oracle = InMemoryFinalityOracle::new();
        let bitcoin =
            FinalityCertificate::observe(ChainKind::Bitcoin, 800_000, hash(1), hash(2), 800_010)
                .expect("valid anchor");
        assert_eq!(
            oracle.verify_finality(ChainKind::Ethereum, &bitcoin),
            Err(SwapError::CertificateChainMismatch {
                expected: ChainKind::Ethereum,
                found: ChainKind::Bitcoin,
            })
        );
        assert_eq!(oracle.accepted_tip(ChainKind::Bitcoin), None);
        assert_eq!(oracle.accepted_tip(ChainKind::Ethereum), None);
    }

    /// Ugly path: an anchor below one already accepted is a rewind, not a fresh fact.
    #[test]
    fn test_a_rewound_anchor_is_refused() {
        let mut oracle = InMemoryFinalityOracle::new();
        assert_eq!(
            oracle.verify_finality(ChainKind::Ethereum, &eth(1000, 1012)),
            Ok(true)
        );

        // 900..=1011 is 112 confirmations — deeper than required — but the chain cannot have gone
        // back from tip 1012 to tip 1011.
        let rewound = eth(900, 1011);
        assert_eq!(rewound.confirmations(), 112);
        assert_eq!(
            oracle.verify_finality(ChainKind::Ethereum, &rewound),
            Err(SwapError::CertificateRewindsAcceptedAnchor {
                chain: ChainKind::Ethereum,
                accepted_tip: 1012,
                certificate_tip: 1011,
            })
        );
        assert_eq!(oracle.accepted_tip(ChainKind::Ethereum), Some(1012));
    }

    /// Ugly path: an observation far behind a witnessed tip is refused as stale.
    #[test]
    fn test_a_stale_certificate_is_refused() {
        let mut oracle = InMemoryFinalityOracle::new();

        // Witness tip 2200 with a certificate whose depth does not pass: the tip is a fact about
        // the chain even though the transaction in it is not yet final.
        let shallow = eth(2195, 2200);
        assert!(oracle
            .verify_finality(ChainKind::Ethereum, &shallow)
            .is_err());
        assert_eq!(oracle.seen_tip(ChainKind::Ethereum), Some(2200));
        assert_eq!(oracle.accepted_tip(ChainKind::Ethereum), None);

        // A 12-deep certificate at tip 1011 is 1189 blocks behind the witnessed tip.
        let stale = eth(1000, 1011);
        assert_eq!(
            oracle.verify_finality(ChainKind::Ethereum, &stale),
            Err(SwapError::CertificateStale {
                chain: ChainKind::Ethereum,
                seen_tip: 2200,
                certificate_tip: 1011,
                window: DEFAULT_CERTIFICATE_STALENESS_BLOCKS,
            })
        );
        assert_eq!(oracle.accepted_tip(ChainKind::Ethereum), None);

        // The window is load-bearing: with it widened past the gap, the same certificate is read
        // rather than refused.
        let mut oracle = InMemoryFinalityOracle::new();
        assert!(oracle
            .verify_finality(ChainKind::Ethereum, &shallow)
            .is_err());
        oracle.set_staleness_window(ChainKind::Ethereum, 4096);
        assert_eq!(
            oracle.verify_finality(ChainKind::Ethereum, &stale),
            Ok(true)
        );
        assert_eq!(oracle.accepted_tip(ChainKind::Ethereum), Some(1011));
    }

    /// Solana is decided by the commitment level on the certificate, not by a caller's string.
    #[test]
    fn test_solana_finality_uses_the_certificate_commitment() {
        let mut oracle = InMemoryFinalityOracle::new();

        let finalized = FinalityCertificate::observe(ChainKind::Solana, 500, hash(3), hash(4), 500)
            .expect("valid anchor")
            .with_commitment_level("finalized");
        assert_eq!(
            oracle.verify_finality(ChainKind::Solana, &finalized),
            Ok(true)
        );

        let mut oracle = InMemoryFinalityOracle::new();
        let confirmed = FinalityCertificate::observe(ChainKind::Solana, 500, hash(3), hash(4), 500)
            .expect("valid anchor")
            .with_commitment_level("confirmed");
        assert_eq!(
            oracle.verify_finality(ChainKind::Solana, &confirmed),
            Err(SwapError::FinalityNotMet {
                chain: "sol".to_string(),
                required: 0,
                current: 1,
            }),
            "the oracle requires finalized, so confirmed is refused"
        );
    }

    /// The confirmation helper is the single definition of depth.
    #[test]
    fn test_confirmations_at_counts_the_anchored_block() {
        assert_eq!(FinalityCertificate::confirmations_at(100, 100), Ok(1));
        assert_eq!(FinalityCertificate::confirmations_at(100, 111), Ok(12));
        assert!(FinalityCertificate::confirmations_at(100, 99).is_err());
        assert!(FinalityCertificate::confirmations_at(0, u64::MAX).is_err());
    }

    /// A store whose tips live in a shared cell, so an oracle can be dropped ("the process exits")
    /// and a new one reloaded from the same bytes.
    #[derive(Clone, Default)]
    struct SharedTipStore(
        alloc::rc::Rc<core::cell::RefCell<BTreeMap<ChainKind, FinalityTipRecord>>>,
    );

    impl FinalityTipStore for SharedTipStore {
        fn load_tips(&self) -> Result<BTreeMap<ChainKind, FinalityTipRecord>, SwapError> {
            Ok(self.0.borrow().clone())
        }

        fn store_tips(
            &self,
            tips: &BTreeMap<ChainKind, FinalityTipRecord>,
        ) -> Result<(), SwapError> {
            *self.0.borrow_mut() = tips.clone();
            Ok(())
        }
    }

    /// A store that always fails, to prove an unpersisted acceptance is not reported as success.
    struct FailingTipStore;

    impl FinalityTipStore for FailingTipStore {
        fn load_tips(&self) -> Result<BTreeMap<ChainKind, FinalityTipRecord>, SwapError> {
            Ok(BTreeMap::new())
        }

        fn store_tips(
            &self,
            _tips: &BTreeMap<ChainKind, FinalityTipRecord>,
        ) -> Result<(), SwapError> {
            Err(SwapError::FinalityTipStore(
                "store is unavailable".to_string(),
            ))
        }
    }

    /// The restart gap this closes: an accepted tip must survive dropping the oracle, and the
    /// reloaded oracle must refuse an older certificate as a rewind rather than read it as current.
    #[test]
    fn test_accepted_tip_survives_a_reload_and_refuses_a_rewind() {
        let store = SharedTipStore::default();
        {
            let mut oracle =
                PersistentFinalityOracle::load(store.clone()).expect("an empty store loads");
            assert_eq!(oracle.accepted_tip(ChainKind::Ethereum), None);
            assert_eq!(
                oracle.verify_finality(ChainKind::Ethereum, &eth(1000, 1011)),
                Ok(true)
            );
            assert_eq!(oracle.accepted_tip(ChainKind::Ethereum), Some(1011));
        }

        // "Process restart": the oracle is dropped; only the store remains.
        let mut reloaded = PersistentFinalityOracle::load(store).expect("store reloads");
        assert_eq!(
            reloaded.accepted_tip(ChainKind::Ethereum),
            Some(1011),
            "the accepted tip must reload from the store"
        );
        assert_eq!(
            reloaded.verify_finality(ChainKind::Ethereum, &eth(1000, 1005)),
            Err(SwapError::CertificateRewindsAcceptedAnchor {
                chain: ChainKind::Ethereum,
                accepted_tip: 1011,
                certificate_tip: 1005,
            }),
            "a certificate below the reloaded accepted tip is a rewind, not a fresh fact"
        );
    }

    /// A witnessed-but-not-accepted tip is persisted too, so staleness survives a restart.
    #[test]
    fn test_witnessed_tip_survives_a_reload() {
        let store = SharedTipStore::default();
        {
            let mut oracle = PersistentFinalityOracle::load(store.clone()).unwrap();
            // One short of the requirement: refused, but the tip is still witnessed.
            assert!(oracle
                .verify_finality(ChainKind::Ethereum, &eth(1000, 1010))
                .is_err());
            assert_eq!(oracle.seen_tip(ChainKind::Ethereum), Some(1010));
        }
        let reloaded = PersistentFinalityOracle::load(store).unwrap();
        assert_eq!(reloaded.seen_tip(ChainKind::Ethereum), Some(1010));
        assert_eq!(reloaded.accepted_tip(ChainKind::Ethereum), None);
    }

    /// An acceptance that cannot be made durable fails closed instead of being reported.
    #[test]
    fn test_an_unpersisted_acceptance_is_not_reported_as_success() {
        let mut oracle = PersistentFinalityOracle::load(FailingTipStore).unwrap();
        assert_eq!(
            oracle.verify_finality(ChainKind::Ethereum, &eth(1000, 1011)),
            Err(SwapError::FinalityTipStore(
                "store is unavailable".to_string()
            ))
        );
    }
}
