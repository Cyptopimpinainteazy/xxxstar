//! Drift detection between on-chain canonical truth and a surface-observed snapshot.
//!
//! This module is only compiled when the `std` feature is enabled (the crate default).
//! It provides the types and logic needed for service-side surfaces — the Tauri desktop
//! app, the browser extension, and the web portal — to detect when their locally-cached
//! state has diverged from the authoritative on-chain roots.
//!
//! # Mechanism
//!
//! Each surface periodically fetches a [`CanonicalSnapshot`] from the X3 node RPC.
//! The snapshot contains three 32-byte Merkle roots:
//!
//! - `identity_root` — root of all identity records
//! - `asset_root`    — root of all asset supply entries
//! - `treasury_root` — root of all treasury vault states
//!
//! [`detect_drift`] compares an on-chain reference snapshot against a surface-observed
//! one.  Any root mismatch becomes a [`DriftKind`] entry in the returned [`DriftReport`].
//!
//! # Example
//!
//! ```rust
//! use x3_canonical_truth::sync::{CanonicalSnapshot, SurfaceKind, detect_drift};
//!
//! let on_chain = CanonicalSnapshot {
//!     block_number: 1_000,
//!     identity_root: [1u8; 32],
//!     asset_root: [2u8; 32],
//!     treasury_root: [3u8; 32],
//!     captured_at_ms: 1_700_000_000_000,
//! };
//!
//! // A surface whose identity root has drifted from on-chain truth.
//! let mut observed = on_chain.clone();
//! observed.identity_root = [0u8; 32];
//!
//! let report = detect_drift(&on_chain, &observed, SurfaceKind::Extension);
//! assert_eq!(report.drifts.len(), 1);
//! assert_eq!(report.snapshot_block, 1_000);
//! ```

use std::time::{SystemTime, UNIX_EPOCH};

// ─── Canonical snapshot ────────────────────────────────────────────────────────

/// Point-in-time snapshot of the three canonical truth Merkle roots at one block.
///
/// Roots are blake2-256 Merkle roots of, respectively:
/// - all identity records known at `block_number`,
/// - all asset supply entries at `block_number`,
/// - all treasury vault states at `block_number`.
///
/// Two snapshots captured at the same `block_number` on any surface MUST have
/// identical roots.  Any discrepancy is a canonical truth drift and must be
/// surfaced via [`detect_drift`].
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct CanonicalSnapshot {
    /// Block number at which the roots were captured.
    pub block_number: u64,
    /// Merkle root of all identity records at this block.
    pub identity_root: [u8; 32],
    /// Merkle root of all asset supply entries at this block.
    pub asset_root: [u8; 32],
    /// Merkle root of all treasury vault states at this block.
    pub treasury_root: [u8; 32],
    /// Unix timestamp in milliseconds at which this snapshot was captured.
    pub captured_at_ms: u64,
}

// ─── Surface kind ──────────────────────────────────────────────────────────────

/// Identifies the client surface from which an observed snapshot originates.
///
/// Used in [`DriftReport`] so monitoring infrastructure can route alerts to the
/// correct surface owner without parsing free-form strings.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum SurfaceKind {
    /// Tauri desktop application (`apps/x3-desktop/`).
    Desktop,
    /// Manifest V3 browser extension (`apps/x3-extension/`).
    Extension,
    /// Web portal frontend (`x3fronend/`).
    Web,
}

// ─── Drift kind ────────────────────────────────────────────────────────────────

/// Describes a single canonical truth discrepancy found for one surface.
///
/// Root-level comparisons cannot pinpoint the specific diverging record;
/// they only establish that divergence exists.  The fields carry the
/// **on-chain** root bytes as identifiers so downstream reconciliation jobs
/// know which reference root was expected.
///
/// When a mismatch is only at the root level (not resolved to individual
/// records), the `expected` / `observed` `u128` fields are set to `0`.
/// Per-record supply values are filled in only after a full re-fetch and
/// reconciliation pass.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum DriftKind {
    /// The identity Merkle root observed by the surface does not match the
    /// on-chain reference.
    ///
    /// `account_id` carries the on-chain `identity_root` as a 32-byte
    /// identifier so the report is self-documenting.
    IdentityMismatch {
        /// On-chain identity root (the expected value).
        account_id: [u8; 32],
    },

    /// The asset supply Merkle root observed by the surface does not match the
    /// on-chain reference.
    ///
    /// `asset_id` carries the on-chain `asset_root`.  `expected` and `observed`
    /// are `0` for root-level mismatches; they are populated with atomic supply
    /// amounts only during per-asset reconciliation.
    AssetSupplyMismatch {
        /// On-chain asset root (the expected value).
        asset_id: [u8; 32],
        /// Expected supply in atomic units (0 at root granularity).
        expected: u128,
        /// Observed supply in atomic units (0 at root granularity).
        observed: u128,
    },

    /// The treasury vault state Merkle root observed by the surface does not
    /// match the on-chain reference.
    ///
    /// `vault_id` carries the on-chain `treasury_root`.  Balance fields are
    /// `0` at root granularity.
    TreasuryStateMismatch {
        /// On-chain treasury root (the expected value).
        vault_id: [u8; 32],
        /// Expected vault balance in atomic units (0 at root granularity).
        expected: u128,
        /// Observed vault balance in atomic units (0 at root granularity).
        observed: u128,
    },
}

// ─── Drift report ──────────────────────────────────────────────────────────────

/// Complete drift report for one surface at one block.
///
/// An empty `drifts` list means the surface is fully in sync with on-chain
/// canonical truth.  A non-empty list must trigger an alert and initiate a
/// full reconciliation pass.
#[derive(Clone, Debug)]
pub struct DriftReport {
    /// Surface that produced the observed snapshot being compared.
    pub surface: SurfaceKind,
    /// All detected drift items.  Empty when no divergence was found.
    pub drifts: Vec<DriftKind>,
    /// Block number from the on-chain snapshot used as the reference.
    pub snapshot_block: u64,
    /// Unix timestamp in milliseconds at which drift detection ran.
    pub detected_at_ms: u64,
}

// ─── Drift detection ───────────────────────────────────────────────────────────

/// Compare an on-chain snapshot against a surface-observed snapshot and return
/// a [`DriftReport`] listing every root-level mismatch.
///
/// The comparison is a simple byte-equality check on each Merkle root.
/// No cryptographic verification is performed here — the caller is responsible
/// for ensuring `on_chain` was obtained from a trusted node endpoint.
///
/// # Arguments
///
/// * `on_chain` — authoritative snapshot sourced directly from the X3 node RPC.
/// * `observed` — snapshot as cached or reconstructed by the client surface.
/// * `surface`  — which surface produced `observed`.
///
/// # Returns
///
/// A [`DriftReport`] whose `drifts` field is empty when the surface is in sync,
/// or contains one [`DriftKind`] entry per mismatched root (maximum three).
///
/// # Example
///
/// ```rust
/// use x3_canonical_truth::sync::{CanonicalSnapshot, DriftKind, SurfaceKind, detect_drift};
///
/// let reference = CanonicalSnapshot {
///     block_number: 42,
///     identity_root: [0xAAu8; 32],
///     asset_root:    [0xBBu8; 32],
///     treasury_root: [0xCCu8; 32],
///     captured_at_ms: 1_700_000_000_000,
/// };
///
/// // Surface whose treasury root has drifted.
/// let mut stale = reference.clone();
/// stale.treasury_root = [0u8; 32];
///
/// let report = detect_drift(&reference, &stale, SurfaceKind::Desktop);
/// assert_eq!(report.drifts.len(), 1);
/// assert!(matches!(report.drifts[0], DriftKind::TreasuryStateMismatch { .. }));
/// assert_eq!(report.snapshot_block, 42);
/// ```
#[must_use]
pub fn detect_drift(
    on_chain: &CanonicalSnapshot,
    observed: &CanonicalSnapshot,
    surface: SurfaceKind,
) -> DriftReport {
    let mut drifts = Vec::new();

    if on_chain.identity_root != observed.identity_root {
        drifts.push(DriftKind::IdentityMismatch {
            account_id: on_chain.identity_root,
        });
    }

    if on_chain.asset_root != observed.asset_root {
        drifts.push(DriftKind::AssetSupplyMismatch {
            asset_id: on_chain.asset_root,
            expected: 0,
            observed: 0,
        });
    }

    if on_chain.treasury_root != observed.treasury_root {
        drifts.push(DriftKind::TreasuryStateMismatch {
            vault_id: on_chain.treasury_root,
            expected: 0,
            observed: 0,
        });
    }

    let detected_at_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
        .unwrap_or(0);

    DriftReport {
        surface,
        drifts,
        snapshot_block: on_chain.block_number,
        detected_at_ms,
    }
}

// ─── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_snapshot(block: u64, id: u8, asset: u8, treasury: u8) -> CanonicalSnapshot {
        CanonicalSnapshot {
            block_number: block,
            identity_root: [id; 32],
            asset_root: [asset; 32],
            treasury_root: [treasury; 32],
            captured_at_ms: 1_700_000_000_000,
        }
    }

    #[test]
    fn no_drift_when_roots_match() {
        let snap = make_snapshot(100, 0xAA, 0xBB, 0xCC);
        let report = detect_drift(&snap, &snap.clone(), SurfaceKind::Web);
        assert!(
            report.drifts.is_empty(),
            "identical roots must yield no drift"
        );
        assert_eq!(report.snapshot_block, 100);
    }

    #[test]
    fn identity_root_mismatch_produces_one_drift() {
        let on_chain = make_snapshot(200, 0xAA, 0xBB, 0xCC);
        let mut observed = on_chain.clone();
        observed.identity_root = [0u8; 32];

        let report = detect_drift(&on_chain, &observed, SurfaceKind::Desktop);
        assert_eq!(report.drifts.len(), 1);
        assert!(
            matches!(&report.drifts[0], DriftKind::IdentityMismatch { account_id }
                if *account_id == [0xAA; 32]),
            "account_id must carry the on-chain identity root"
        );
    }

    #[test]
    fn asset_root_mismatch_produces_one_drift() {
        let on_chain = make_snapshot(300, 0x11, 0x22, 0x33);
        let mut observed = on_chain.clone();
        observed.asset_root = [0xFFu8; 32];

        let report = detect_drift(&on_chain, &observed, SurfaceKind::Extension);
        assert_eq!(report.drifts.len(), 1);
        assert!(
            matches!(&report.drifts[0], DriftKind::AssetSupplyMismatch { asset_id, expected, observed }
                if *asset_id == [0x22; 32] && *expected == 0 && *observed == 0),
            "asset_id must carry the on-chain asset root; supply values are zero at root granularity"
        );
    }

    #[test]
    fn treasury_root_mismatch_produces_one_drift() {
        let on_chain = make_snapshot(400, 0x11, 0x22, 0x44);
        let mut observed = on_chain.clone();
        observed.treasury_root = [0u8; 32];

        let report = detect_drift(&on_chain, &observed, SurfaceKind::Web);
        assert_eq!(report.drifts.len(), 1);
        assert!(
            matches!(&report.drifts[0], DriftKind::TreasuryStateMismatch { vault_id, expected, observed }
                if *vault_id == [0x44; 32] && *expected == 0 && *observed == 0),
            "vault_id must carry the on-chain treasury root"
        );
    }

    #[test]
    fn all_three_roots_mismatched_produces_three_drifts() {
        let on_chain = make_snapshot(500, 0xAA, 0xBB, 0xCC);
        let observed = make_snapshot(500, 0x11, 0x22, 0x33);

        let report = detect_drift(&on_chain, &observed, SurfaceKind::Desktop);
        assert_eq!(
            report.drifts.len(),
            3,
            "all three roots mismatched — expect three drift entries"
        );
        assert!(matches!(
            report.drifts[0],
            DriftKind::IdentityMismatch { .. }
        ));
        assert!(matches!(
            report.drifts[1],
            DriftKind::AssetSupplyMismatch { .. }
        ));
        assert!(matches!(
            report.drifts[2],
            DriftKind::TreasuryStateMismatch { .. }
        ));
    }

    #[test]
    fn snapshot_block_is_taken_from_on_chain_reference() {
        let on_chain = make_snapshot(9_999, 0x01, 0x02, 0x03);
        let observed = make_snapshot(8_000, 0x01, 0x02, 0x03); // same roots, different block
        let report = detect_drift(&on_chain, &observed, SurfaceKind::Web);
        assert_eq!(report.snapshot_block, 9_999);
        assert!(
            report.drifts.is_empty(),
            "root values match even though block numbers differ"
        );
    }

    #[test]
    fn detected_at_ms_is_nonzero() {
        let snap = make_snapshot(1, 0x00, 0x00, 0x00);
        let report = detect_drift(&snap, &snap.clone(), SurfaceKind::Extension);
        // Should be a real Unix timestamp — well past the year 2000 (946_684_800_000 ms).
        assert!(
            report.detected_at_ms > 946_684_800_000,
            "detected_at_ms must be a plausible Unix timestamp in milliseconds"
        );
    }
}
