//! X3 Mobile Wallet SDK
//!
//! Provides iOS/Android wallet functionality via React Native bridge.
//! Handles: biometric auth, transaction signing, QR codes, deep linking.

pub mod biometric_auth_mobile;
pub mod deeplink_handler;
pub mod mobile_wallet_core;
pub mod qr_scanner;
pub mod transaction_signer_mobile;

// Re-exports for easy access
pub use biometric_auth_mobile::{AuthResult, BiometricAuth, BiometricType};
pub use deeplink_handler::{DeeplinkHandler, DeeplinkRequest};
pub use mobile_wallet_core::{MobileWallet, MobileWalletConfig, WalletBalance};
pub use qr_scanner::{QRData, QRDataType, QRScanner};
pub use transaction_signer_mobile::{MobileTransactionSigner, SigningRequest};

/// SDK version
pub const SDK_VERSION: &str = "1.0.0";

/// The account-id part of an X3 address: 40 hex characters (a 20-byte id).
///
/// This is the shape `MobileWallet::import_from_seed` produces
/// (`x3:` + `hex(public_key[0..20])`). It used to live only inside the QR
/// scanner, as a `len >= 50` check — which rejected every address this SDK
/// generates (43 characters) and accepted arbitrary longer strings that merely
/// looked like one.
pub fn is_x3_account_id(body: &str) -> bool {
    body.len() == 40 && body.chars().all(|c| c.is_ascii_hexdigit())
}

/// An X3 address: `x3:` followed by a 40-hex account id.
pub fn is_x3_address(address: &str) -> bool {
    address.strip_prefix("x3:").is_some_and(is_x3_account_id)
}

/// Supported platforms
#[allow(non_camel_case_types)] // `Platform::iOS` is the platform's own spelling
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    iOS,
    Android,
}

/// SDK initialization result
pub type SdkResult<T> = Result<T, SdkError>;

/// SDK error types
#[derive(Debug, thiserror::Error)]
pub enum SdkError {
    #[error("Biometric authentication failed: {0}")]
    BiometricError(String),

    #[error("Transaction signing failed: {0}")]
    SigningError(String),

    #[error("QR code scan failed: {0}")]
    QRScanError(String),

    #[error("Deeplink handler error: {0}")]
    DeeplinkError(String),

    #[error("Wallet not initialized")]
    WalletNotInitialized,

    #[error("Invalid address format")]
    InvalidAddress,

    #[error("Insufficient balance")]
    InsufficientBalance,

    #[error("Network error: {0}")]
    NetworkError(String),

    /// A call that needs the chain client was made, and this SDK has none.
    ///
    /// Distinct from `NetworkError`, which means a request was attempted and
    /// failed: this means nothing was attempted. It exists because
    /// `fetch_balance` used to answer with a balance of exactly 10 X3 and
    /// `get_network_status` with `is_connected: true` at block 1000, without
    /// talking to anything.
    #[error("chain RPC is not implemented in this SDK: {0}")]
    RpcNotImplemented(String),

    #[error("Cryptographic error: {0}")]
    Crypto(String),

    #[error("Storage error: {0}")]
    StorageError(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),
}

/// Initialize SDK with platform and configuration
pub async fn init_sdk(platform: Platform, config: MobileWalletConfig) -> SdkResult<MobileWallet> {
    tracing::info!(
        "Initializing X3 Mobile SDK v{} for {:?}",
        SDK_VERSION,
        platform
    );
    MobileWallet::new(config).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sdk_version() {
        assert_eq!(SDK_VERSION, "1.0.0");
    }

    #[test]
    fn test_platform_enum() {
        assert_eq!(Platform::iOS, Platform::iOS);
        assert_ne!(Platform::iOS, Platform::Android);
    }

    #[tokio::test]
    async fn test_sdk_initialization() {
        let config = MobileWalletConfig::default();
        let wallet = MobileWallet::new(config).await.unwrap();

        // A fresh wallet has no addresses, no cached balances and no endpoint.
        // This test used to be `assert!(true)`.
        assert!(wallet.rpc_endpoint().await.is_none());
        assert!(wallet.get_balance("x3:nobody").await.unwrap().is_none());

        // And it will not invent either: with no chain client, a balance
        // request is refused rather than answered with a default.
        assert!(matches!(
            wallet.fetch_balance("x3:nobody").await,
            Err(SdkError::RpcNotImplemented(_))
        ));
        assert!(matches!(
            wallet.get_network_status().await,
            Err(SdkError::RpcNotImplemented(_))
        ));

        wallet.set_rpc_endpoint("https://rpc.example").await;
        assert_eq!(
            wallet.rpc_endpoint().await.as_deref(),
            Some("https://rpc.example")
        );
    }
}
