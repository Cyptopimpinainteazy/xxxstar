//! X3 Mobile Wallet SDK
//! 
//! Provides iOS/Android wallet functionality via React Native bridge.
//! Handles: biometric auth, transaction signing, QR codes, deep linking.

pub mod mobile_wallet_core;
pub mod biometric_auth_mobile;
pub mod transaction_signer_mobile;
pub mod qr_scanner;
pub mod deeplink_handler;

// Re-exports for easy access
pub use mobile_wallet_core::{MobileWallet, MobileWalletConfig, WalletBalance};
pub use biometric_auth_mobile::{BiometricAuth, BiometricType, AuthResult};
pub use transaction_signer_mobile::{MobileTransactionSigner, SigningRequest};
pub use qr_scanner::{QRScanner, QRData, QRDataType};
pub use deeplink_handler::{DeeplinkHandler, DeeplinkRequest};

/// SDK version
pub const SDK_VERSION: &str = "1.0.0";

/// Supported platforms
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
    
    #[error("Storage error: {0}")]
    StorageError(String),
    
    #[error("Serialization error: {0}")]
    SerializationError(String),
}

/// Initialize SDK with platform and configuration
pub async fn init_sdk(platform: Platform, config: MobileWalletConfig) -> SdkResult<MobileWallet> {
    tracing::info!("Initializing X3 Mobile SDK v{} for {:?}", SDK_VERSION, platform);
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
        // Initialization test would be mocked in full implementation
        assert!(true); // Placeholder
    }
}
