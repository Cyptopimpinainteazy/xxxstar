//! X3 Liquidity Core — v0.4 spot-AMM wrapper.
//!
//! This crate is a **thin façade** over [`x3_dex`] that exposes only the
//! modules required for the v0.4 minimal internal mainnet:
//!
//! * [`launchpad`] — pool creation and initial liquidity bootstrap
//! * [`settlement`] — swap execution and LP withdrawal for cross-VM settlement
//! * [`anti_rug`] — basic LP lock and rug-pull mitigation checks
//!
//! Advanced DEX functionality (perpetuals, options, flash loans, liquidity
//! mining, concentrated liquidity) is intentionally excluded here and can be
//! reached directly through `x3-dex` when the `advanced` feature flag is
//! activated.
//!
//! # Scope rationale
//!
//! `x3-dex` carries `#![allow(dead_code, unused_*)]` broadly because many of
//! its modules are under active development.  This wrapper does NOT copy that
//! lint suppression; every symbol exposed here must be genuinely used.
//!
//! # v0.4 status
//!
//! * Spot AMM (constant-product): **wired, tested**
//! * Settlement path: **wired, tested**
//! * Anti-rug LP lock: **wired, tested**
//! * Advanced features: **gated behind `advanced` feature, not active in v0.4**

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub mod anti_rug;
pub mod launchpad;
pub mod settlement;

pub use anti_rug::{AntiRugError, LpLock, LpLockRegistry};
pub use launchpad::{LaunchError, LaunchRequest, Launchpad};
pub use settlement::{SettleError, SettleRequest, Settlement};

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Launchpad

    #[test]
    fn launch_request_validates_fee() {
        assert!(Launchpad::build(1, 2, 10_000, 0).is_err()); // zero initial_a
        assert!(Launchpad::build(1, 2, 0, 10_000).is_err()); // zero initial_b
        assert!(Launchpad::build(1, 2, 1_000, 1_000).is_ok());
    }

    #[test]
    fn launch_request_rejects_same_token() {
        assert!(matches!(
            Launchpad::build(5, 5, 1_000, 1_000),
            Err(LaunchError::SameToken)
        ));
    }

    #[test]
    fn launch_request_rejects_excessive_fee() {
        // fee > 1000 bps (10 %) is considered unreasonable.
        assert!(matches!(
            Launchpad::build_with_fee(1, 2, 1_000, 1_000, 1_001),
            Err(LaunchError::FeeTooHigh)
        ));
    }

    // ── Settlement

    #[test]
    fn settle_request_valid() {
        let req = Settlement::build(42, 100, 95).unwrap();
        assert_eq!(req.pool_id, 42);
        assert_eq!(req.amount_in, 100);
        assert_eq!(req.min_out, 95);
    }

    #[test]
    fn settle_request_rejects_zero_amount() {
        assert!(matches!(
            Settlement::build(42, 0, 0),
            Err(SettleError::ZeroAmount)
        ));
    }

    #[test]
    fn settle_request_rejects_inverted_bounds() {
        // min_out > amount_in is impossible by construction.
        assert!(matches!(
            Settlement::build(42, 50, 100),
            Err(SettleError::InvertedBounds)
        ));
    }

    // ── Anti-rug

    #[test]
    fn lp_lock_stores_and_retrieves() {
        let mut reg = LpLockRegistry::new();
        reg.lock([0x01; 32], 42, 1000, 100).unwrap();
        let lock = reg.get(&[0x01; 32], 42).unwrap();
        assert_eq!(lock.lp_amount, 1000);
        assert_eq!(lock.unlock_at_block, 100);
    }

    #[test]
    fn lp_lock_rejects_zero_amount() {
        let mut reg = LpLockRegistry::new();
        assert!(matches!(
            reg.lock([0x01; 32], 42, 0, 100),
            Err(AntiRugError::ZeroAmount)
        ));
    }

    #[test]
    fn lp_lock_enforces_unlock_block() {
        let mut reg = LpLockRegistry::new();
        reg.lock([0x02; 32], 1, 500, 200).unwrap();
        // Cannot withdraw before unlock block.
        assert!(matches!(
            reg.withdraw(&[0x02; 32], 1, 199),
            Err(AntiRugError::LockNotExpired)
        ));
        // Can withdraw at or after unlock block.
        assert!(reg.withdraw(&[0x02; 32], 1, 200).is_ok());
    }

    #[test]
    fn lp_lock_not_found_error() {
        let mut reg = LpLockRegistry::new();
        assert!(matches!(
            reg.withdraw(&[0x99; 32], 1, 999),
            Err(AntiRugError::NotFound)
        ));
    }
}
