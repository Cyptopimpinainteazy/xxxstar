//! X3 parity core: the single Rust source of truth for the flashloan
//! repay-or-revert math.
//!
//! This crate is intentionally tiny and dependency-light (only `serde`) so
//! that:
//!
//! * The SVM Anchor program (`X3-contracts/svm/programs/x3_core`) can mirror
//!   these helpers without dragging the Solana toolchain into the parity
//!   harness.
//! * The EVM Foundry tests (`X3-contracts/evm/test/parity/*.t.sol`) load the
//!   same JSON vectors via `vm.readFile` so divergence is caught as a math
//!   bug rather than as runtime drift.
//! * The proof-forge parity runner can drive the harness as a plain
//!   `cargo test` invocation against any host.
//!
//! Invariants (mirror `X3Flashloan.sol` and `x3_core::lib.rs`):
//!
//! * I1 atomicity     : terminal pool balance must be `>= pre + fee`, else fail.
//! * I2 no reentrancy : a flashloan call cannot recursively borrow the same asset.
//! * I3 fee monotonic : `fee` is purely additive; protocol never owes borrower.
//! * I4 round-up      : fee rounds up so 1-wei loops cannot drain the pool.
//!
//! Drift between this crate and the SVM/EVM implementations is enforced by:
//!
//! * `tests/parity_vectors.rs` — runs every JSON vector against
//!   [`simulate_flashloan`].
//! * `X3Flashloan.t.sol::testFuzz_FeeIsAlwaysAdditive` and the parity Foundry
//!   test — same vectors, same expectations on the EVM side.
//! * `x3_core` unit tests — same vectors, same expectations on the SVM side.

use serde::{Deserialize, Serialize};

/// Hard cap on protocol fee, in basis points. Mirrors the constructor check
/// in `X3Flashloan.sol` (1000 bps = 10%).
pub const MAX_FEE_BPS: u16 = 1000;

/// Compute flashloan fee, rounding up.
///
/// `fee = ceil(amount * fee_bps / 10_000)`. Saturating arithmetic guarantees
/// no Rust-side overflow on adversarial inputs; the EVM side relies on
/// `unchecked`-free Solidity 0.8 math, which reverts on overflow rather than
/// wrapping. The harness only asserts equality on inputs the EVM side
/// accepts, so saturation here is a defense-in-depth choice.
pub fn quote_fee(amount: u128, fee_bps: u16) -> u128 {
    let num = amount.saturating_mul(fee_bps as u128);
    num.saturating_add(9_999) / 10_000
}

/// Borrower behavior matching both the EVM `MockBorrower` family and the SVM
/// `BorrowerKind` enum.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BorrowerKind {
    Honest,
    Deadbeat,
    Underpay,
}

/// Outcome of running a single vector through the simulator.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FlashloanOutcome {
    /// `true` iff the protocol committed.
    pub ok: bool,
    /// Stable reason tag matching the EVM error name on revert; empty on ok.
    pub revert_reason: String,
    /// Signed change in pool balance. Always 0 on revert (atomicity, I1).
    pub pool_delta: i128,
}

/// Pure simulator. No I/O, no allocation beyond `revert_reason`.
pub fn simulate_flashloan(
    amount: u128,
    fee_bps: u16,
    borrower: BorrowerKind,
) -> FlashloanOutcome {
    let fee = quote_fee(amount, fee_bps);
    match borrower {
        BorrowerKind::Honest => FlashloanOutcome {
            ok: true,
            revert_reason: String::new(),
            pool_delta: fee as i128,
        },
        BorrowerKind::Deadbeat => FlashloanOutcome {
            ok: false,
            revert_reason: "CallbackFailed".to_string(),
            pool_delta: 0,
        },
        BorrowerKind::Underpay => FlashloanOutcome {
            ok: false,
            revert_reason: "NotRepaid".to_string(),
            pool_delta: 0,
        },
    }
}

// ---------------------------------------------------------------------------
// Vector schema
// ---------------------------------------------------------------------------

/// Top-level JSON document at `shared/test-vectors/flashloan_repay_or_revert.json`.
#[derive(Deserialize, Debug)]
pub struct VectorDoc {
    pub spec_version: u32,
    pub spec: String,
    #[serde(default)]
    pub spec_doc: Option<String>,
    pub fee_bps: u16,
    pub vectors: Vec<Vector>,
}

#[derive(Deserialize, Debug)]
pub struct Vector {
    pub id: String,
    pub asset: String,
    /// Decimal string to keep `u128` precision intact across JSON.
    pub amount: String,
    pub borrower_kind: BorrowerKind,
    pub expected: ExpectedOutcome,
}

#[derive(Deserialize, Debug)]
pub struct ExpectedOutcome {
    /// `"ok"` or `"revert"`.
    pub result: String,
    pub revert_reason: Option<String>,
    /// Decimal string `i128` (signed for symmetry with `pool_delta`).
    pub pool_delta: String,
}

impl Vector {
    pub fn amount_u128(&self) -> u128 {
        self.amount
            .parse::<u128>()
            .unwrap_or_else(|_| panic!("vector {} has non-u128 amount", self.id))
    }

    pub fn expected_pool_delta_i128(&self) -> i128 {
        self.expected
            .pool_delta
            .parse::<i128>()
            .unwrap_or_else(|_| panic!("vector {} has non-i128 pool_delta", self.id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quote_fee_rounds_up() {
        // 1 * 9 / 10_000 = 0.0009 -> rounds up to 1.
        assert_eq!(quote_fee(1, 9), 1);
        // 100e18 * 9 / 10_000 = 9e16 exactly.
        assert_eq!(quote_fee(100_000_000_000_000_000_000u128, 9), 90_000_000_000_000_000u128);
    }

    #[test]
    fn honest_pool_delta_is_fee() {
        let out = simulate_flashloan(1_000, 9, BorrowerKind::Honest);
        assert!(out.ok);
        assert_eq!(out.pool_delta as u128, quote_fee(1_000, 9));
        assert!(out.revert_reason.is_empty());
    }

    #[test]
    fn deadbeat_reverts_with_stable_tag() {
        let out = simulate_flashloan(1_000, 9, BorrowerKind::Deadbeat);
        assert!(!out.ok);
        assert_eq!(out.revert_reason, "CallbackFailed");
        assert_eq!(out.pool_delta, 0);
    }

    #[test]
    fn underpay_reverts_with_stable_tag() {
        let out = simulate_flashloan(1_000, 9, BorrowerKind::Underpay);
        assert!(!out.ok);
        assert_eq!(out.revert_reason, "NotRepaid");
        assert_eq!(out.pool_delta, 0);
    }

    #[test]
    fn fee_cap_constant_matches_evm_constructor() {
        // X3Flashloan.sol enforces fee_bps <= 1000 in its constructor.
        // Keep this constant in sync with that contract; the parity harness
        // uses it as a precondition when running adversarial vectors.
        assert_eq!(MAX_FEE_BPS, 1000);
    }
}
