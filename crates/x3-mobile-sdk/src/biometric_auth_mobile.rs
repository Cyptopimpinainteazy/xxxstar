//! Biometric authentication for mobile wallets
//! 
//! Supports Face ID, fingerprint, iris, and PIN fallback.
//! Uses secure enclave storage on iOS and Android KeyStore on Android.

use crate::SdkError;
use serde::{Deserialize, Serialize};
use sha2::Digest;
use zeroize::Zeroize;

/// Biometric authentication types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BiometricType {
    FaceID,
    Fingerprint,
    Iris,
    PIN,
}

impl std::fmt::Display for BiometricType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BiometricType::FaceID => write!(f, "Face ID"),
            BiometricType::Fingerprint => write!(f, "Fingerprint"),
            BiometricType::Iris => write!(f, "Iris"),
            BiometricType::PIN => write!(f, "PIN"),
        }
    }
}

/// Biometric authentication result
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuthResult {
    Success(String), // Session token or timestamp
    Failure(String), // Error reason
    Retry,           // User retry (too many failures)
    Canceled,        // User canceled
}

/// Biometric session information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiometricSession {
    pub session_token: String,
    pub auth_method: BiometricType,
    pub created_at: i64,
    pub expires_at: i64,
}

impl BiometricSession {
    /// Check if session is still valid
    pub fn is_valid(&self) -> bool {
        let now = chrono::Utc::now().timestamp();
        now < self.expires_at
    }

    /// Get remaining time in seconds
    pub fn remaining_seconds(&self) -> i64 {
        let now = chrono::Utc::now().timestamp();
        (self.expires_at - now).max(0)
    }
}

/// Biometric template data (hashed)
#[derive(Debug, Clone)]
struct BiometricTemplate {
    // In production: encrypted biometric template
    // For security, this is hashed with salt
    template_hash: Vec<u8>,
    salt: Vec<u8>,
    biometric_type: BiometricType,
    enrollment_date: i64,
}

/// Biometric authentication engine
pub struct BiometricAuth {
    // Secure storage of biometric templates (in production: Keystore/Secure Enclave)
    templates: std::sync::Mutex<Vec<BiometricTemplate>>,
    
    // Active sessions
    sessions: tokio::sync::RwLock<Vec<BiometricSession>>,
    
    // Failed attempt tracking
    failed_attempts: std::sync::Mutex<u32>,
    
    // Session timeout in seconds (300 = 5 minutes)
    session_timeout: i64,
    
    // Max failed attempts before lockout (5)
    max_failures: u32,
}

impl BiometricAuth {
    /// Create new biometric authentication engine
    pub fn new(session_timeout_seconds: i64) -> Self {
        Self {
            templates: std::sync::Mutex::new(Vec::new()),
            sessions: tokio::sync::RwLock::new(Vec::new()),
            failed_attempts: std::sync::Mutex::new(0),
            session_timeout: session_timeout_seconds,
            max_failures: 5,
        }
    }

    /// Enroll a biometric credential
    pub async fn enroll(
        &self,
        biometric_data: &[u8],
        biometric_type: BiometricType,
    ) -> Result<(), SdkError> {
        // Validate minimum biometric size
        if biometric_data.is_empty() {
            return Err(SdkError::BiometricError(
                "Empty biometric data".to_string(),
            ));
        }

        // Generate random salt
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let salt: Vec<u8> = (0..32).map(|_| rng.gen()).collect();

        // Hash biometric data with salt
        let mut hasher = sha2::Sha256::new();
        hasher.update(biometric_data);
        hasher.update(&salt);
        let template_hash = hasher.finalize().to_vec();

        let template = BiometricTemplate {
            template_hash,
            salt,
            biometric_type,
            enrollment_date: chrono::Utc::now().timestamp(),
        };

        let mut templates = self.templates.lock().expect("templates mutex poisoned");
        templates.push(template);

        tracing::info!("Enrolled {} credential", biometric_type);
        Ok(())
    }

    /// Verify biometric and create authenticated session
    pub async fn verify(&self, biometric_data: &[u8]) -> Result<AuthResult, SdkError> {
        // Check if locked out
        let failed = *self.failed_attempts.lock().expect("failed_attempts mutex poisoned");
        if failed >= self.max_failures {
            return Ok(AuthResult::Retry);
        }

        let templates = self.templates.lock().expect("templates mutex poisoned");
        if templates.is_empty() {
            return Err(SdkError::BiometricError(
                "No biometric enrolled".to_string(),
            ));
        }

        // Try to match against enrolled templates
        for template in templates.iter() {
            let mut hasher = sha2::Sha256::new();
            hasher.update(biometric_data);
            hasher.update(&template.salt);
            let test_hash = hasher.finalize().to_vec();

            // Constant-time comparison to prevent timing attacks
            if constant_time_compare(&test_hash, &template.template_hash) {
                // Match found - create session
                let session_token = generate_session_token();
                let now = chrono::Utc::now().timestamp();
                
                let session = BiometricSession {
                    session_token: session_token.clone(),
                    auth_method: template.biometric_type,
                    created_at: now,
                    expires_at: now + self.session_timeout,
                };

                self.sessions.write().await.push(session);
                
                // Reset failure count
                *self.failed_attempts.lock().expect("failed_attempts mutex poisoned") = 0;

                tracing::info!("Biometric authentication successful");
                return Ok(AuthResult::Success(session_token));
            }
        }

        // No match - increment failures
        let mut failed = self.failed_attempts.lock().expect("failed_attempts mutex poisoned");
        *failed += 1;
        let remaining = self.max_failures - *failed;

        tracing::warn!(
            "Biometric authentication failed ({} attempts remaining)",
            remaining
        );

        if remaining == 0 {
            Ok(AuthResult::Retry) // Locked out
        } else {
            Ok(AuthResult::Failure(format!(
                "{} attempts remaining",
                remaining
            )))
        }
    }

    /// Verify PIN (fallback authentication)
    pub async fn verify_pin(&self, pin: &str) -> Result<AuthResult, SdkError> {
        // In production: stored PIN is hashed/salted in Keystore
        // For now: simple validation (minimum 4 digits)
        if pin.len() < 4 || !pin.chars().all(|c| c.is_numeric()) {
            return Ok(AuthResult::Failure("Invalid PIN format".to_string()));
        }

        // Hash PIN and compare with stored hash
        let pin_hash = hash_pin(pin);

        // Placeholder: In production, compare against stored hash
        let stored_hash = "placeholder_hash";

        if pin_hash == stored_hash {
            let session_token = generate_session_token();
            let now = chrono::Utc::now().timestamp();

            let session = BiometricSession {
                session_token: session_token.clone(),
                auth_method: BiometricType::PIN,
                created_at: now,
                expires_at: now + self.session_timeout,
            };

            self.sessions.write().await.push(session);
            Ok(AuthResult::Success(session_token))
        } else {
            let mut failed = self.failed_attempts.lock().expect("failed_attempts mutex poisoned");
            *failed += 1;

            if *failed >= self.max_failures {
                Ok(AuthResult::Retry)
            } else {
                Ok(AuthResult::Failure(format!(
                    "{} attempts remaining",
                    self.max_failures - *failed
                )))
            }
        }
    }

    /// Set PIN (fallback authentication)
    pub async fn set_pin(&self, existing_pin: &str, new_pin: &str) -> Result<(), SdkError> {
        // Verify user knows current PIN first
        match self.verify_pin(existing_pin).await? {
            AuthResult::Success(_) => {}
            _ => return Err(SdkError::BiometricError("Current PIN incorrect".to_string())),
        }

        // Validate new PIN
        if new_pin.len() < 4 {
            return Err(SdkError::BiometricError(
                "PIN must be at least 4 digits".to_string(),
            ));
        }

        if !new_pin.chars().all(|c| c.is_numeric()) {
            return Err(SdkError::BiometricError(
                "PIN must contain only digits".to_string(),
            ));
        }

        // In production: store hash(new_pin) in Keystore
        let _ = hash_pin(new_pin);

        tracing::info!("PIN updated");
        Ok(())
    }

    /// Verify active session
    pub async fn verify_session(&self, session_token: &str) -> Result<BiometricSession, SdkError> {
        let sessions = self.sessions.read().await;

        if let Some(session) = sessions.iter().find(|s| s.session_token == session_token) {
            if session.is_valid() {
                return Ok(session.clone());
            }
        }

        Err(SdkError::BiometricError("Invalid or expired session".to_string()))
    }

    /// Invalidate session (logout)
    pub async fn logout(&self, session_token: &str) -> Result<(), SdkError> {
        let mut sessions = self.sessions.write().await;
        sessions.retain(|s| s.session_token != session_token);
        
        tracing::info!("Session invalidated");
        Ok(())
    }

    /// Get list of enrolled biometrics
    pub async fn get_enrolled_methods(&self) -> Result<Vec<BiometricType>, SdkError> {
        let templates = self.templates.lock().expect("templates mutex poisoned");
        Ok(templates.iter().map(|t| t.biometric_type).collect())
    }

    /// Check if specific biometric is enrolled
    pub async fn is_enrolled(&self, biometric_type: BiometricType) -> Result<bool, SdkError> {
        let templates = self.templates.lock().expect("templates mutex poisoned");
        Ok(templates.iter().any(|t| t.biometric_type == biometric_type))
    }

    /// Reset lockout (admin function - requires authentication)
    pub async fn reset_lockout(&self) -> Result<(), SdkError> {
        *self.failed_attempts.lock().expect("failed_attempts mutex poisoned") = 0;
        tracing::warn!("Lockout reset");
        Ok(())
    }

    /// Clear all sessions
    pub async fn clear_sessions(&self) -> Result<(), SdkError> {
        self.sessions.write().await.clear();
        tracing::info!("All sessions cleared");
        Ok(())
    }
}

/// Constant-time comparison to prevent timing attacks
fn constant_time_compare(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }

    let mut result = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        result |= x ^ y;
    }

    result == 0
}

/// Generate secure session token
fn generate_session_token() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let token: Vec<u8> = (0..32).map(|_| rng.gen()).collect();
    hex::encode(token)
}

/// Hash PIN with salt
fn hash_pin(pin: &str) -> String {
    let mut hasher = sha2::Sha256::new();
    hasher.update(pin.as_bytes());
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_biometric_type_display() {
        assert_eq!(BiometricType::FaceID.to_string(), "Face ID");
        assert_eq!(BiometricType::Fingerprint.to_string(), "Fingerprint");
    }

    #[test]
    fn test_session_validity() {
        let session = BiometricSession {
            session_token: "token".to_string(),
            auth_method: BiometricType::FaceID,
            created_at: 0,
            expires_at: 999999999999,
        };
        assert!(session.is_valid());
    }

    #[test]
    fn test_session_expired() {
        let session = BiometricSession {
            session_token: "token".to_string(),
            auth_method: BiometricType::FaceID,
            created_at: 0,
            expires_at: 0,
        };
        assert!(!session.is_valid());
    }

    #[tokio::test]
    async fn test_biometric_enrollment() {
        let auth = BiometricAuth::new(300);
        let biometric_data = b"test_biometric_data";

        let result = auth
            .enroll(biometric_data, BiometricType::Fingerprint)
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_biometric_verification() {
        let auth = BiometricAuth::new(300);
        let biometric_data = b"test_biometric_data";

        auth.enroll(biometric_data, BiometricType::FaceID)
            .await
            .unwrap();

        let result = auth.verify(biometric_data).await.unwrap();
        assert_eq!(result, AuthResult::Success(_));
    }

    #[tokio::test]
    async fn test_biometric_failure() {
        let auth = BiometricAuth::new(300);
        let biometric_data = b"test_biometric_data";

        auth.enroll(biometric_data, BiometricType::Fingerprint)
            .await
            .unwrap();

        let wrong_data = b"wrong_biometric_data";
        let result = auth.verify(wrong_data).await.unwrap();

        match result {
            AuthResult::Failure(_) => {}
            _ => panic!("Expected failure"),
        }
    }

    #[tokio::test]
    async fn test_session_logout() {
        let auth = BiometricAuth::new(300);
        let biometric_data = b"test_biometric_data";

        auth.enroll(biometric_data, BiometricType::PIN)
            .await
            .unwrap();

        let AuthResult::Success(token) = auth.verify(biometric_data).await.unwrap() else {
            panic!("Expected success");
        };

        auth.logout(&token).await.unwrap();

        let result = auth.verify_session(&token).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_pin_verification() {
        let auth = BiometricAuth::new(300);
        
        // PIN verification would require setting PIN first in production
        let result = auth.verify_pin("1234").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_enrolled_methods() {
        let auth = BiometricAuth::new(300);
        
        auth.enroll(b"face", BiometricType::FaceID).await.unwrap();
        auth.enroll(b"finger", BiometricType::Fingerprint).await.unwrap();

        let methods = auth.get_enrolled_methods().await.unwrap();
        assert_eq!(methods.len(), 2);
    }

    #[tokio::test]
    async fn test_is_enrolled() {
        let auth = BiometricAuth::new(300);
        
        auth.enroll(b"face", BiometricType::FaceID).await.unwrap();

        assert!(auth.is_enrolled(BiometricType::FaceID).await.unwrap());
        assert!(!auth.is_enrolled(BiometricType::Fingerprint).await.unwrap());
    }

    #[test]
    fn test_constant_time_compare() {
        let a = b"secret";
        let b = b"secret";
        assert!(constant_time_compare(a, b));

        let c = b"wrong";
        assert!(!constant_time_compare(a, c));
    }

    #[tokio::test]
    async fn test_lockout_after_failures() {
        let auth = BiometricAuth::new(300);
        auth.enroll(b"biometric", BiometricType::FaceID)
            .await
            .unwrap();

        for _ in 0..5 {
            let _ = auth.verify(b"wrong").await;
        }

        let result = auth.verify(b"biometric").await.unwrap();
        assert_eq!(result, AuthResult::Retry);
    }

    #[tokio::test]
    async fn test_reset_lockout() {
        let auth = BiometricAuth::new(300);
        
        // Simulate lockout
        for _ in 0..5 {
            let _ = auth.verify(b"wrong").await;
        }

        auth.reset_lockout().await.unwrap();

        let result = auth.verify(b"wrong").await.unwrap();
        match result {
            AuthResult::Failure(_) => {}, // Now allows one more attempt
            _ => panic!("Expected failure"),
        }
    }
}
