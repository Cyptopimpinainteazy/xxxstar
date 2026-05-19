// Claim parsing and registry management

use crate::proof::Claim;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaimFile {
    pub version: String,
    pub claims: Vec<Claim>,
}

pub fn parse_claims(content: &str) -> anyhow::Result<ClaimFile> {
    serde_json::from_str(content).map_err(|e| anyhow::anyhow!("Failed to parse claims: {}", e))
}

pub fn serialize_claims(claims: &[Claim]) -> anyhow::Result<String> {
    let file = ClaimFile {
        version: "1.0".to_string(),
        claims: claims.to_vec(),
    };
    serde_json::to_string_pretty(&file)
        .map_err(|e| anyhow::anyhow!("Failed to serialize claims: {}", e))
}
