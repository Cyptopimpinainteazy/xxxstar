//! QR code scanning and generation for mobile wallets
//! 
//! Handles: address QR scans, payment request parsing, URI decoding

use crate::SdkError;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// QR code data types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum QRDataType {
    Address,
    PaymentRequest,
    SigningRequest,
    DeeplinkURI,
    RawData,
}

/// Parsed QR code data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QRData {
    pub data_type: QRDataType,
    pub raw_data: String,
    pub parsed_address: Option<String>,
    pub parsed_amount: Option<u128>,
    pub parsed_memo: Option<String>,
    pub scanned_at: i64,
}

impl QRData {
    /// Create from raw QR string
    pub fn from_raw(raw_data: String) -> Result<Self, SdkError> {
        let parsed = Self::parse_qr_string(&raw_data)?;
        
        Ok(Self {
            data_type: parsed.0,
            raw_data,
            parsed_address: parsed.1,
            parsed_amount: parsed.2,
            parsed_memo: parsed.3,
            scanned_at: chrono::Utc::now().timestamp(),
        })
    }

    /// Parse X3 URI: x3:address?amount=1000&memo=hello
    fn parse_qr_string(raw: &str) -> Result<(QRDataType, Option<String>, Option<u128>, Option<String>), SdkError> {
        // X3 payment URI
        if raw.starts_with("x3:") {
            let uri = &raw[3..];
            
            // Split address and params
            let parts: Vec<&str> = uri.split('?').collect();
            let address = parts[0].to_string();

            // Validate address format
            if !address.starts_with("x3:") && address.len() < 50 {
                return Err(SdkError::InvalidAddress);
            }

            let mut amount = None;
            let mut memo = None;

            // Parse query params
            if parts.len() > 1 {
                let params = parts[1];
                for param in params.split('&') {
                    if let Some((key, val)) = param.split_once('=') {
                        match key {
                            "amount" => {
                                amount = val.parse::<u128>().ok();
                            }
                            "memo" => {
                                memo = Some(
                                    urlencoding::decode(val)
                                        .unwrap_or_default()
                                        .into_owned(),
                                );
                            }
                            _ => {}
                        }
                    }
                }
            }

            return Ok((QRDataType::PaymentRequest, Some(address), amount, memo));
        }

        // Plain address
        if raw.len() >= 50 && raw.contains(':') {
            return Ok((QRDataType::Address, Some(raw.to_string()), None, None));
        }

        // Deeplink
        if raw.contains("://") {
            return Ok((QRDataType::DeeplinkURI, Some(raw.to_string()), None, None));
        }

        // Raw data
        Ok((QRDataType::RawData, None, None, None))
    }

    /// Generate QR code string for address
    pub fn generate_receive_qr(address: &str) -> String {
        format!("x3:{}", address)
    }

    /// Generate payment request QR
    pub fn generate_payment_qr(address: &str, amount: Option<u128>, memo: Option<&str>) -> String {
        let mut uri = format!("x3:{}", address);

        if amount.is_some() || memo.is_some() {
            uri.push('?');
            
            if let Some(amt) = amount {
                uri.push_str(&format!("amount={}", amt));
            }

            if let Some(m) = memo {
                if amount.is_some() {
                    uri.push('&');
                }
                uri.push_str(&format!("memo={}", urlencoding::encode(m)));
            }
        }

        uri
    }
}

/// QR scanner engine
pub struct QRScanner {
    // Scan history
    history: tokio::sync::RwLock<Vec<QRData>>,
    
    // Trusted QR addresses (whitelist)
    trusted_addresses: tokio::sync::RwLock<Vec<String>>,
}

impl QRScanner {
    /// Create new QR scanner
    pub fn new() -> Self {
        Self {
            history: tokio::sync::RwLock::new(Vec::new()),
            trusted_addresses: tokio::sync::RwLock::new(Vec::new()),
        }
    }

    /// Scan and parse QR code
    pub async fn scan(&self, qr_raw: String) -> Result<QRData, SdkError> {
        let qr_data = QRData::from_raw(qr_raw)?;

        // Add to history
        self.history.write().await.push(qr_data.clone());

        tracing::info!("Scanned QR: {:?}", qr_data.data_type);
        Ok(qr_data)
    }

    /// Get scan history (last N scans)
    pub async fn get_history(&self, limit: usize) -> Result<Vec<QRData>, SdkError> {
        let history = self.history.read().await;
        Ok(history.iter().rev().take(limit).cloned().collect())
    }

    /// Clear scan history
    pub async fn clear_history(&self) -> Result<(), SdkError> {
        self.history.write().await.clear();
        tracing::info!("QR scan history cleared");
        Ok(())
    }

    /// Trust an address (whitelist)
    pub async fn trust_address(&self, address: &str) -> Result<(), SdkError> {
        if address.len() < 50 {
            return Err(SdkError::InvalidAddress);
        }

        let mut trusted = self.trusted_addresses.write().await;
        if !trusted.contains(&address.to_string()) {
            trusted.push(address.to_string());
            tracing::info!("Trusted address: {}", address);
        }

        Ok(())
    }

    /// Forget a trusted address
    pub async fn untrust_address(&self, address: &str) -> Result<(), SdkError> {
        let mut trusted = self.trusted_addresses.write().await;
        trusted.retain(|a| a != address);
        
        tracing::info!("Untrusted address: {}", address);
        Ok(())
    }

    /// Check if address is trusted
    pub async fn is_trusted(&self, address: &str) -> Result<bool, SdkError> {
        Ok(self
            .trusted_addresses
            .read()
            .await
            .contains(&address.to_string()))
    }

    /// Get trusted addresses
    pub async fn get_trusted_addresses(&self) -> Result<Vec<String>, SdkError> {
        Ok(self.trusted_addresses.read().await.clone())
    }

    /// Validate scanned address
    pub fn validate_address(address: &str) -> Result<bool, SdkError> {
        // X3 addresses
        if address.starts_with("x3:") && address.len() >= 50 {
            return Ok(true);
        }

        // Ethereum addresses (0x...)
        if address.starts_with("0x") && address.len() == 42 {
            return Ok(true);
        }

        // Solana addresses
        if address.len() == 44 && address.chars().all(|c| c.is_alphanumeric()) {
            return Ok(true);
        }

        Ok(false)
    }

    /// Detect phishing attempts (suspicious patterns)
    pub fn detect_phishing(address: &str) -> Result<bool, SdkError> {
        // Check for homograph attacks
        let suspicious_patterns = vec![
            "()[]{}",
            "O0", // Letter O vs Zero
            "l1", // Letter l vs One
            "Il", // I vs lowercase l
        ];

        for pattern in suspicious_patterns {
            if address.contains(pattern) {
                tracing::warn!("Suspicious pattern detected in address");
                return Ok(true);
            }
        }

        Ok(false)
    }
}

impl Default for QRScanner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_address_qr() {
        let raw = "x3:1234567890abc1234567890abc1234567890abc1234567890abc";
        let qr = QRData::from_raw(raw.to_string()).unwrap();
        assert_eq!(qr.data_type, QRDataType::PaymentRequest);
    }

    #[test]
    fn test_parse_payment_qr() {
        let raw = "x3:1234567890abc1234567890abc1234567890abc1234567890abc?amount=1000&memo=test";
        let qr = QRData::from_raw(raw.to_string()).unwrap();
        assert_eq!(qr.parsed_amount, Some(1000));
        assert_eq!(qr.parsed_memo, Some("test".to_string()));
    }

    #[test]
    fn test_generate_receive_qr() {
        let address = "1234567890abc1234567890abc1234567890abc1234567890abc";
        let qr = QRData::generate_receive_qr(address);
        assert_eq!(qr, format!("x3:{}", address));
    }

    #[test]
    fn test_generate_payment_qr() {
        let address = "1234567890abc1234567890abc1234567890abc1234567890abc";
        let qr = QRData::generate_payment_qr(address, Some(5000), Some("lunch"));
        assert!(qr.contains("amount=5000"));
        assert!(qr.contains("memo=lunch"));
    }

    #[tokio::test]
    async fn test_qr_scanner_creation() {
        let scanner = QRScanner::new();
        let history = scanner.get_history(10).await.unwrap();
        assert!(history.is_empty());
    }

    #[tokio::test]
    async fn test_scan_and_history() {
        let scanner = QRScanner::new();
        
        let raw = "x3:1234567890abc1234567890abc1234567890abc1234567890abc";
        scanner.scan(raw.to_string()).await.unwrap();

        let history = scanner.get_history(10).await.unwrap();
        assert_eq!(history.len(), 1);
    }

    #[tokio::test]
    async fn test_trust_address() {
        let scanner = QRScanner::new();
        let address = "x3:1234567890abc1234567890abc1234567890abc1234567890abc";

        scanner.trust_address(address).await.unwrap();
        
        let is_trusted = scanner.is_trusted(address).await.unwrap();
        assert!(is_trusted);
    }

    #[tokio::test]
    async fn test_untrust_address() {
        let scanner = QRScanner::new();
        let address = "x3:1234567890abc1234567890abc1234567890abc1234567890abc";

        scanner.trust_address(address).await.unwrap();
        scanner.untrust_address(address).await.unwrap();
        
        let is_trusted = scanner.is_trusted(address).await.unwrap();
        assert!(!is_trusted);
    }

    #[test]
    fn test_validate_x3_address() {
        let valid = "x3:1234567890abc1234567890abc1234567890abc1234567890abc";
        assert!(QRScanner::validate_address(valid).unwrap());

        let invalid = "x3:short";
        assert!(!QRScanner::validate_address(invalid).unwrap());
    }

    #[test]
    fn test_detect_phishing() {
        let suspicious = "0x123456[789]abc"; // Contains suspicious pattern
        assert!(QRScanner::detect_phishing(suspicious).unwrap());
    }

    #[tokio::test]
    async fn test_clear_history() {
        let scanner = QRScanner::new();
        
        let raw = "x3:1234567890abc1234567890abc1234567890abc1234567890abc";
        scanner.scan(raw.to_string()).await.unwrap();

        scanner.clear_history().await.unwrap();

        let history = scanner.get_history(10).await.unwrap();
        assert!(history.is_empty());
    }

    #[test]
    fn test_qr_data_type_enum() {
        assert_eq!(QRDataType::Address, QRDataType::Address);
        assert_ne!(QRDataType::Address, QRDataType::PaymentRequest);
    }
}
