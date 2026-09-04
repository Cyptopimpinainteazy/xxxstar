//! Deep link handling for wallet integration
//! 
//! Handles: x3:// URL scheme, app-to-app communication, universal links

use crate::SdkError;
use serde::{Deserialize, Serialize};
use url::Url;

/// Deep link request types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeeplinkRequestType {
    SendTransaction,
    ViewAddress,
    ImportWallet,
    SignMessage,
    ConnectDApp,
    OpenDApp,
}

/// Parsed deep link request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeeplinkRequest {
    pub request_type: DeeplinkRequestType,
    pub callback_url: Option<String>,
    pub params: std::collections::HashMap<String, String>,
    pub timestamp: i64,
}

impl DeeplinkRequest {
    /// Parse x3:// URL scheme
    pub fn from_url(url_str: &str) -> Result<Self, SdkError> {
        let url = Url::parse(url_str).map_err(|_| SdkError::DeeplinkError("Invalid URL".to_string()))?;

        let request_type = match url.host_str().unwrap_or("send") {
            "send" | "tx" => DeeplinkRequestType::SendTransaction,
            "address" | "view" => DeeplinkRequestType::ViewAddress,
            "import" => DeeplinkRequestType::ImportWallet,
            "sign" => DeeplinkRequestType::SignMessage,
            "connect" => DeeplinkRequestType::ConnectDApp,
            "open" => DeeplinkRequestType::OpenDApp,
            _ => return Err(SdkError::DeeplinkError("Unknown deeplink type".to_string())),
        };

        let mut params = std::collections::HashMap::new();
        let mut callback_url = None;

        for (key, val) in url.query_pairs() {
            if key == "callback" {
                callback_url = Some(val.into_owned());
            } else {
                params.insert(key.into_owned(), val.into_owned());
            }
        }

        Ok(Self {
            request_type,
            callback_url,
            params,
            timestamp: chrono::Utc::now().timestamp(),
        })
    }

    /// Get parameter
    pub fn get_param(&self, key: &str) -> Option<&str> {
        self.params.get(key).map(|s| s.as_str())
    }

    /// Get required parameter
    pub fn require_param(&self, key: &str) -> Result<String, SdkError> {
        self.get_param(key)
            .map(|s| s.to_string())
            .ok_or_else(|| SdkError::DeeplinkError(format!("Missing parameter: {}", key)))
    }
}

/// Deep link handler
pub struct DeeplinkHandler {
    // Registered app schemes
    allowed_schemes: std::sync::Mutex<Vec<String>>,
    
    // Deep link history
    history: tokio::sync::RwLock<Vec<DeeplinkRequest>>,
}

impl DeeplinkHandler {
    /// Create new handler
    pub fn new() -> Self {
        Self {
            allowed_schemes: std::sync::Mutex::new(vec![
                "x3://".to_string(),
                "ethereum://".to_string(),
            ]),
            history: tokio::sync::RwLock::new(Vec::new()),
        }
    }

    /// Handle incoming deep link
    pub async fn handle(&self, url: &str) -> Result<DeeplinkRequest, SdkError> {
        let request = DeeplinkRequest::from_url(url)?;

        // Validate scheme
        let allowed = self.allowed_schemes.lock().expect("allowed_schemes mutex poisoned");
        let is_allowed = allowed.iter().any(|scheme| url.starts_with(scheme));

        if !is_allowed {
            return Err(SdkError::DeeplinkError("Scheme not allowed".to_string()));
        }

        // Add to history
        self.history.write().await.push(request.clone());

        tracing::info!("Handled deeplink: {:?}", request.request_type);
        Ok(request)
    }

    /// Register app scheme
    pub async fn register_scheme(&self, scheme: String) -> Result<(), SdkError> {
        if !scheme.ends_with("://") {
            return Err(SdkError::DeeplinkError(
                "Scheme must end with ://".to_string(),
            ));
        }

        let mut allowed = self.allowed_schemes.lock().expect("allowed_schemes mutex poisoned");
        if !allowed.contains(&scheme) {
            allowed.push(scheme);
        }

        Ok(())
    }

    /// Revoke app scheme
    pub async fn revoke_scheme(&self, scheme: &str) -> Result<(), SdkError> {
        let mut allowed = self.allowed_schemes.lock().expect("allowed_schemes mutex poisoned");
        allowed.retain(|s| s != scheme);
        
        tracing::info!("Revoked scheme: {}", scheme);
        Ok(())
    }

    /// Get handling history
    pub async fn get_history(&self, limit: usize) -> Result<Vec<DeeplinkRequest>, SdkError> {
        let history = self.history.read().await;
        Ok(history.iter().rev().take(limit).cloned().collect())
    }

    /// Clear history
    pub async fn clear_history(&self) -> Result<(), SdkError> {
        self.history.write().await.clear();
        tracing::info!("Deeplink history cleared");
        Ok(())
    }

    /// Generate send transaction deeplink
    pub fn generate_send_deeplink(
        recipient: &str,
        amount: Option<u128>,
        callback: Option<&str>,
    ) -> String {
        let mut url = format!("x3://send?to={}", urlencoding::encode(recipient));

        if let Some(amt) = amount {
            url.push_str(&format!("&amount={}", amt));
        }

        if let Some(cb) = callback {
            url.push_str(&format!("&callback={}", urlencoding::encode(cb)));
        }

        url
    }

    /// Generate sign message deeplink
    pub fn generate_sign_deeplink(message: &str, callback: Option<&str>) -> String {
        let mut url = format!("x3://sign?message={}", urlencoding::encode(message));

        if let Some(cb) = callback {
            url.push_str(&format!("&callback={}", urlencoding::encode(cb)));
        }

        url
    }

    /// Generate import wallet deeplink
    pub fn generate_import_deeplink(seed_phrase: &str) -> String {
        format!("x3://import?seed={}", urlencoding::encode(seed_phrase))
    }
}

impl Default for DeeplinkHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_send_deeplink() {
        let url = "x3://send?to=x3:recipient&amount=1000";
        let request = DeeplinkRequest::from_url(url).unwrap();
        
        assert_eq!(request.request_type, DeeplinkRequestType::SendTransaction);
        assert_eq!(request.get_param("to"), Some("x3:recipient"));
        assert_eq!(request.get_param("amount"), Some("1000"));
    }

    #[test]
    fn test_sign_deeplink() {
        let url = "x3://sign?message=hello&callback=https://example.com";
        let request = DeeplinkRequest::from_url(url).unwrap();
        
        assert_eq!(request.request_type, DeeplinkRequestType::SignMessage);
        assert_eq!(request.get_param("message"), Some("hello"));
        assert_eq!(request.callback_url, Some("https://example.com".to_string()));
    }

    #[test]
    fn test_connect_deeplink() {
        let url = "x3://connect?app=MyDApp&callback=https://mydapp.com";
        let request = DeeplinkRequest::from_url(url).unwrap();
        
        assert_eq!(request.request_type, DeeplinkRequestType::ConnectDApp);
    }

    #[tokio::test]
    async fn test_handler_creation() {
        let handler = DeeplinkHandler::new();
        let history = handler.get_history(10).await.unwrap();
        assert!(history.is_empty());
    }

    #[tokio::test]
    async fn test_handle_deeplink() {
        let handler = DeeplinkHandler::new();
        
        let url = "x3://send?to=x3:recipient&amount=500";
        let request = handler.handle(url).await.unwrap();
        
        assert_eq!(request.request_type, DeeplinkRequestType::SendTransaction);
    }

    #[tokio::test]
    async fn test_register_scheme() {
        let handler = DeeplinkHandler::new();
        
        handler.register_scheme("myapp://".to_string()).await.unwrap();
        
        // Try to handle with new scheme
        let url = "myapp://send?to=x3:recipient";
        let result = handler.handle(url).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_revoke_scheme() {
        let handler = DeeplinkHandler::new();
        
        handler.revoke_scheme("ethereum://").await.unwrap();
        
        // Try to handle with revoked scheme
        let url = "ethereum://send?to=x3:recipient";
        let result = handler.handle(url).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_deeplink_history() {
        let handler = DeeplinkHandler::new();
        
        let url1 = "x3://send?to=x3:recipient1";
        let url2 = "x3://send?to=x3:recipient2";
        
        handler.handle(url1).await.unwrap();
        handler.handle(url2).await.unwrap();

        let history = handler.get_history(10).await.unwrap();
        assert_eq!(history.len(), 2);
    }

    #[tokio::test]
    async fn test_clear_history() {
        let handler = DeeplinkHandler::new();
        
        let url = "x3://send?to=x3:recipient";
        handler.handle(url).await.unwrap();
        
        handler.clear_history().await.unwrap();
        
        let history = handler.get_history(10).await.unwrap();
        assert!(history.is_empty());
    }

    #[test]
    fn test_generate_send_deeplink() {
        let deeplink = DeeplinkHandler::generate_send_deeplink(
            "x3:recipient",
            Some(1000),
            Some("https://callback.com"),
        );
        
        assert!(deeplink.contains("x3://send"));
        assert!(deeplink.contains("to=x3:recipient"));
        assert!(deeplink.contains("amount=1000"));
    }

    #[test]
    fn test_generate_sign_deeplink() {
        let deeplink = DeeplinkHandler::generate_sign_deeplink(
            "hello world",
            Some("https://callback.com"),
        );
        
        assert!(deeplink.contains("x3://sign"));
        assert!(deeplink.contains("message=hello"));
    }

    #[test]
    fn test_invalid_url() {
        let result = DeeplinkRequest::from_url("not a valid url");
        assert!(result.is_err());
    }

    #[test]
    fn test_request_type_enum() {
        assert_eq!(DeeplinkRequestType::SendTransaction, DeeplinkRequestType::SendTransaction);
        assert_ne!(DeeplinkRequestType::SendTransaction, DeeplinkRequestType::SignMessage);
    }
}
