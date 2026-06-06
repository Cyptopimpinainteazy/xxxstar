//! Bitcoin / UTXO bridge adapter (target F in the production contract).
//!
//! The contract says: "BTC/UTXO path works if existing code supports it;
//! otherwise feature-gated with explicit safe failure." The repository has
//! a skeleton `BitcoinBridgeAdapter` in `crates/x3-bridge-adapters` with
//! many `TODO: Implement ...` markers and no real production behavior. We
//! therefore ship a feature-gated adapter that:
//!
//! - exposes the production `BridgeAdapter` shape so it can be wired into
//!   `VM::with_bridge` like any other adapter,
//! - **fails closed** on every cross-VM call (returns a
//!   `X3_BTC_ADAPTER_DISABLED` error) when the `bitcoin-adapter` feature is
//!   not enabled, so a misconfigured production environment cannot
//!   silently route Bitcoin/UTXO calls through a stub,
//! - records the call as a `dry-run-btc-bridge` receipt in dry-run mode
//!   (matching the existing dry-run pattern for EVM/SVM) so the VM
//!   test-suite can still drive this code path under the default
//!   features.
//!
//! The real production wiring is left to the consuming runtime; this
//! module is the **contract surface** that future production code can
//! fill in without changing the public API.

use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt;

#[cfg(feature = "bitcoin-adapter")]
mod real {
    use super::*;
    /// Real Bitcoin light client (skeleton — full PoW header validation
    /// is out of scope for this iteration; the surface exists so the
    /// consuming crate can plug in a header-chain verifier).
    pub struct BitcoinLightClient;

    impl BitcoinLightClient {
        pub fn new() -> Self {
            Self
        }
    }
}

/// Stable error code returned when a Bitcoin/UTXO call is rejected.
pub const BTC_DISABLED_CODE: &str = "X3_BTC_ADAPTER_DISABLED";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BtcBridgeError {
    pub code: &'static str,
    pub message: String,
}

impl fmt::Display for BtcBridgeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl Error for BtcBridgeError {}

impl BtcBridgeError {
    pub fn disabled() -> Self {
        Self {
            code: BTC_DISABLED_CODE,
            message: "BTC/UTXO adapter is feature-gated and not enabled in this build; \
                     enable the `bitcoin-adapter` feature to route Bitcoin/UTXO calls. \
                     Refusing to silently fall through to a stub."
                .to_string(),
        }
    }
}

/// Bitcoin/UTXO bridge adapter.
///
/// In the default build this adapter is **always disabled**: every call
/// returns `BtcBridgeError::disabled()`. With the `bitcoin-adapter`
/// feature enabled, the real implementation is selected at compile
/// time. The shape of the public API does not change between the two
/// builds so consumers do not have to fork their wiring.
pub struct BtcBridgeAdapter {
    chain_id: u64,
    /// When true (dry-run mode), the adapter records a stable
    /// `dry-run-btc-bridge` receipt instead of failing. This is the
    /// same convention used by the EVM/SVM dry-run bridges and is
    /// what makes the BTC path exercisable from tests.
    dry_run: bool,
    receipts: Vec<String>,
}

impl BtcBridgeAdapter {
    /// Build a production-style adapter (fails closed on every call).
    pub fn production(chain_id: u64) -> Self {
        Self {
            chain_id,
            dry_run: false,
            receipts: Vec::new(),
        }
    }

    /// Build a dry-run adapter. Every call succeeds with a synthetic
    /// `dry-run-btc-bridge` receipt — this is the only mode that
    /// supports the BTC/UTXO test fixtures and CLI examples.
    pub fn dry_run(chain_id: u64) -> Self {
        Self {
            chain_id,
            dry_run: true,
            receipts: Vec::new(),
        }
    }

    pub fn chain_id(&self) -> u64 {
        self.chain_id
    }

    pub fn receipts(&self) -> &[String] {
        &self.receipts
    }

    fn gate<T>(&mut self, success: T) -> Result<T, BtcBridgeError> {
        if self.dry_run {
            Ok(success)
        } else {
            Err(BtcBridgeError::disabled())
        }
    }
}

/// Public verification of Bitcoin-style finality.
///
/// Returns `Ok(())` only when:
/// - the `bitcoin-adapter` feature is enabled AND
/// - the proof buffer is a non-empty UTF-8 string starting with
///   `btc-header-v1` (placeholder for the real PoW header check that
///   the production runtime must implement).
///
/// In the default (no-feature) build this returns the `disabled()`
/// error so production callers cannot accidentally route a
/// Bitcoin/UTXO call through an unverified stub.
pub fn verify_btc_finality(proof: &[u8]) -> Result<(), BtcBridgeError> {
    if proof.is_empty() {
        return Err(BtcBridgeError {
            code: "X3_BTC_PROOF_EMPTY",
            message: "BTC finality proof is empty".to_string(),
        });
    }
    #[cfg(feature = "bitcoin-adapter")]
    {
        let _ = real::BitcoinLightClient::new();
        // Real header-chain validation belongs in the consuming crate;
        // we only assert the proof prefix here.
        if proof.starts_with(b"btc-header-v1") {
            Ok(())
        } else {
            Err(BtcBridgeError {
                code: "X3_BTC_PROOF_PREFIX",
                message: "BTC finality proof missing btc-header-v1 prefix".to_string(),
            })
        }
    }
    #[cfg(not(feature = "bitcoin-adapter"))]
    {
        let _ = proof;
        Err(BtcBridgeError::disabled())
    }
}

impl BtcBridgeAdapter {
    /// Build a synthetic dry-run receipt. Mirrors the format used by
    /// the EVM and SVM dry-run bridges.
    pub fn synthetic_dry_run_receipt(
        &mut self,
        from_chain: &str,
        from_asset: &str,
        to_chain: &str,
        to_asset: &str,
        amount: u128,
    ) -> Vec<u8> {
        let payload = format!(
            "dry-run-btc-bridge:{}->{}:{}:{}-{}:{}",
            from_chain, to_chain, from_asset, to_asset, amount, self.chain_id
        );
        let receipt = payload;
        self.receipts.push(receipt.clone());
        receipt.into_bytes()
    }

    /// Construct a BridgeTransferRequest-shaped result for the dry-run
    /// path; production paths must construct one in their real
    /// implementation.
    pub fn transfer(
        &mut self,
        from_chain: &str,
        from_asset: &str,
        to_chain: &str,
        to_asset: &str,
        amount: u128,
        receiver: &[u8],
        source_finality_proof: &[u8],
        transfer_proof: &[u8],
    ) -> Result<Vec<u8>, BtcBridgeError> {
        // The dry-run path accepts any inputs (matching the EVM/SVM
        // behavior) so existing test fixtures can drive this code.
        if self.dry_run {
            let _ = (source_finality_proof, transfer_proof, receiver);
            let receipt =
                self.synthetic_dry_run_receipt(from_chain, from_asset, to_chain, to_asset, amount);
            return self.gate(receipt);
        }
        // Production path: fail closed unless feature enabled.
        #[cfg(feature = "bitcoin-adapter")]
        {
            // Real implementation goes here; we at least verify the
            // proofs before returning a failure so a typo can't
            // silently let an unverified call through.
            if let Err(e) = verify_btc_finality(source_finality_proof) {
                return Err(e);
            }
            if transfer_proof.is_empty() {
                return Err(BtcBridgeError {
                    code: "X3_BTC_TRANSFER_PROOF_EMPTY",
                    message: "BTC transfer proof is empty".to_string(),
                });
            }
            Err(BtcBridgeError::disabled())
        }
        #[cfg(not(feature = "bitcoin-adapter"))]
        {
            Err(BtcBridgeError::disabled())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn production_adapter_fails_closed() {
        let mut adapter = BtcBridgeAdapter::production(0);
        let result = adapter.transfer("btc", "BTC", "ethereum", "WBTC", 100, b"0x", b"", b"");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.code, BTC_DISABLED_CODE);
    }

    #[test]
    fn dry_run_records_receipt() {
        let mut adapter = BtcBridgeAdapter::dry_run(0);
        let result = adapter.transfer("btc", "BTC", "ethereum", "WBTC", 100, b"0x", b"", b"");
        assert!(result.is_ok());
        let bytes = result.unwrap();
        let receipt = std::str::from_utf8(&bytes).unwrap();
        assert!(receipt.starts_with("dry-run-btc-bridge:"));
        assert_eq!(adapter.receipts().len(), 1);
    }

    #[test]
    fn empty_proof_rejected() {
        let result = verify_btc_finality(b"");
        assert!(result.is_err());
    }

    #[cfg(feature = "bitcoin-adapter")]
    #[test]
    fn feature_enabled_verifies_prefix() {
        assert!(verify_btc_finality(b"btc-header-v1:abc").is_ok());
        assert!(verify_btc_finality(b"something-else").is_err());
    }

    #[cfg(not(feature = "bitcoin-adapter"))]
    #[test]
    fn feature_disabled_returns_disabled_error() {
        let err = verify_btc_finality(b"btc-header-v1:abc").unwrap_err();
        assert_eq!(err.code, BTC_DISABLED_CODE);
    }
}
