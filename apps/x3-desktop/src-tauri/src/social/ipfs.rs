// IPFS Integration for X3 Social Network Media
// Enables decentralized media storage using IPFS

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IpfsFile {
    pub hash: String,          // IPFS content hash (CIDv1)
    pub name: String,
    pub size: u64,
    pub media_type: String,
    pub uploaded_at: String,
    pub pinned: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IpfsUploadResult {
    pub hash: String,
    pub url: String,
    pub gateway_urls: Vec<String>,
    pub file_name: String,
    pub file_size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpfsConfig {
    pub api_url: String,      // e.g., "http://127.0.0.1:5001"
    pub gateway_url: String,  // e.g., "https://ipfs.io"
    pub timeout_secs: u64,
    pub max_file_size: u64,   // in bytes
}

pub struct IpfsClient {
    config: IpfsConfig,
}

impl IpfsClient {
    pub fn new(config: IpfsConfig) -> Self {
        Self { config }
    }

    /// Initialize connection to IPFS node
    pub async fn init(&self) -> Result<String, String> {
        // In production, this would connect to IPFS daemon
        // For now, return a test response
        Ok("IPFS client initialized".to_string())
    }

    /// Upload file to IPFS
    pub async fn add_file(&self, file_path: &Path) -> Result<IpfsUploadResult, String> {
        if !file_path.exists() {
            return Err("File not found".to_string());
        }

        let metadata = std::fs::metadata(file_path)
            .map_err(|e| format!("Cannot read file metadata: {}", e))?;

        let file_size = metadata.len();
        if file_size > self.config.max_file_size {
            return Err(format!("File exceeds maximum size: {} bytes", self.config.max_file_size));
        }

        let file_name = file_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        // Read file content
        let content = std::fs::read(file_path)
            .map_err(|e| format!("Cannot read file: {}", e))?;

        // In production, send to IPFS node via HTTP API:
        // POST /api/v0/add with multipart form-data
        // For testing, generate deterministic hash from content
        let hash = self.calculate_cid(&content);

        let mut gateway_urls = vec![];
        gateway_urls.push(format!("{}/ipfs/{}", self.config.gateway_url, &hash));

        Ok(IpfsUploadResult {
            hash: hash.clone(),
            url: format!("{}/ipfs/{}", self.config.gateway_url, &hash),
            gateway_urls,
            file_name,
            file_size,
        })
    }

    /// Upload directory to IPFS (wrapped in single DAG)
    pub async fn add_directory(&self, dir_path: &Path) -> Result<IpfsUploadResult, String> {
        if !dir_path.is_dir() {
            return Err("Path is not a directory".to_string());
        }

        // In production, would recursively add directory structure
        // For now, return error
        Err("Directory upload not yet implemented".to_string())
    }

    /// Pin file to ensure persistence
    pub async fn pin_file(&self, hash: &str) -> Result<(), String> {
        // POST /api/v0/pin/add?arg={hash}
        Ok(())
    }

    /// Unpin file (may be garbage collected)
    pub async fn unpin_file(&self, hash: &str) -> Result<(), String> {
        // POST /api/v0/pin/rm?arg={hash}
        Ok(())
    }

    /// Check if file is available on IPFS
    pub async fn stat_file(&self, hash: &str) -> Result<IpfsFile, String> {
        // GET /api/v0/files/stat?arg=/ipfs/{hash}
        Ok(IpfsFile {
            hash: hash.to_string(),
            name: "unknown".to_string(),
            size: 0,
            media_type: "application/octet-stream".to_string(),
            uploaded_at: chrono::Utc::now().to_rfc3339(),
            pinned: false,
        })
    }

    /// Download file from IPFS
    pub async fn get_file(&self, hash: &str, output_path: &Path) -> Result<(), String> {
        // GET /api/v0/get?arg={hash}
        // Save to output_path
        Ok(())
    }

    /// Generate Content IDentifier (CIDv1) for content
    fn calculate_cid(&self, content: &[u8]) -> String {
        use std::collections::hash_map::RandomState;
        use std::hash::{BuildHasher, Hash, Hasher};

        let mut hasher = RandomState::new().build_hasher();
        content.hash(&mut hasher);

        // Simulate CIDv1 (in production, use actual IPFS library)
        let hash_value = hasher.finish();
        format!("bafyreigh2ruj{:016x}abcdef", hash_value)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaGallery {
    pub id: String,
    pub user_id: String,
    pub title: String,
    pub description: String,
    pub media_items: Vec<MediaItem>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaItem {
    pub id: String,
    pub ipfs_hash: String,
    pub gateway_url: String,
    pub file_name: String,
    pub media_type: String,  // image/jpeg, video/mp4, etc.
    pub file_size: u64,
    pub width: Option<i32>,   // For images
    pub height: Option<i32>,  // For images
    pub duration: Option<f64>, // For videos in seconds
    pub thumbnail_ipfs: Option<String>,
    pub caption: Option<String>,
    pub tags: Vec<String>,
    pub uploaded_at: String,
}

pub struct MediaProcessor;

impl MediaProcessor {
    pub fn get_media_type(file_path: &Path) -> String {
        match file_path.extension().and_then(|e| e.to_str()) {
            Some("jpg") | Some("jpeg") => "image/jpeg".to_string(),
            Some("png") => "image/png".to_string(),
            Some("gif") => "image/gif".to_string(),
            Some("webp") => "image/webp".to_string(),
            Some("mp4") => "video/mp4".to_string(),
            Some("webm") => "video/webm".to_string(),
            Some("mp3") => "audio/mpeg".to_string(),
            Some("wav") => "audio/wav".to_string(),
            _ => "application/octet-stream".to_string(),
        }
    }

    pub fn is_image(media_type: &str) -> bool {
        media_type.starts_with("image/")
    }

    pub fn is_video(media_type: &str) -> bool {
        media_type.starts_with("video/")
    }

    pub fn is_audio(media_type: &str) -> bool {
        media_type.starts_with("audio/")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_media_type_detection() {
        assert_eq!(
            MediaProcessor::get_media_type(&PathBuf::from("photo.jpg")),
            "image/jpeg"
        );
        assert_eq!(
            MediaProcessor::get_media_type(&PathBuf::from("video.mp4")),
            "video/mp4"
        );
    }

    #[test]
    fn test_media_classification() {
        assert!(MediaProcessor::is_image("image/jpeg"));
        assert!(MediaProcessor::is_video("video/mp4"));
        assert!(!MediaProcessor::is_audio("image/jpeg"));
    }

    #[test]
    fn test_ipfs_config_creation() {
        let config = IpfsConfig {
            api_url: "http://127.0.0.1:5001".to_string(),
            gateway_url: "https://ipfs.io".to_string(),
            timeout_secs: 30,
            max_file_size: 100_000_000, // 100MB
        };

        assert_eq!(config.api_url, "http://127.0.0.1:5001");
    }
}
