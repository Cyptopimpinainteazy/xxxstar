// lightweight HTTP helper for external-chains
// only available when `std` feature is enabled

#[cfg(feature = "std")]
use reqwest;

/// Post JSON bytes to an HTTP endpoint and return response bytes.
///
/// This abstraction exists so that when building with `no_std` we can stub
/// out HTTP logic; in that case the function returns an error.
pub async fn post_json(url: &str, body: &[u8]) -> Result<Vec<u8>, String> {
    #[cfg(feature = "std")]
    {
        let client = reqwest::Client::new();
        let resp = client
            .post(url)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(body.to_vec())
            .send()
            .await
            .map_err(|e| e.to_string())?;
        let bytes = resp.bytes().await.map_err(|e| e.to_string())?;
        Ok(bytes.to_vec())
    }

    #[cfg(not(feature = "std"))]
    {
        Err("HTTP unavailable in no_std build".to_string())
    }
}
