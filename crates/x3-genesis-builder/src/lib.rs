use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenesisManifest {
    pub chain_id: String,
    pub runtime_wasm_path: String,
    pub bootnodes: Vec<String>,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum GenesisBuilderError {
    #[error("chain_id cannot be empty")]
    EmptyChainId,
    #[error("runtime_wasm_path cannot be empty")]
    EmptyRuntimePath,
}

impl GenesisManifest {
    pub fn validate(&self) -> Result<(), GenesisBuilderError> {
        if self.chain_id.trim().is_empty() {
            return Err(GenesisBuilderError::EmptyChainId);
        }
        if self.runtime_wasm_path.trim().is_empty() {
            return Err(GenesisBuilderError::EmptyRuntimePath);
        }
        Ok(())
    }

    pub fn canonical_json(&self) -> Result<String, GenesisBuilderError> {
        self.validate()?;
        let mut bootnodes = self.bootnodes.clone();
        bootnodes.sort();
        let canonical = GenesisManifest {
            chain_id: self.chain_id.clone(),
            runtime_wasm_path: self.runtime_wasm_path.clone(),
            bootnodes,
        };
        Ok(serde_json::to_string(&canonical).expect("serializing manifest should not fail"))
    }

    pub fn digest_hex(&self) -> Result<String, GenesisBuilderError> {
        let canonical = self.canonical_json()?;
        let hash = Sha256::digest(canonical.as_bytes());
        Ok(format!("{:x}", hash))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_hash_is_stable_for_same_inputs() {
        let manifest = GenesisManifest {
            chain_id: "x3-mainnet".to_string(),
            runtime_wasm_path: "runtime/mainnet.wasm".to_string(),
            bootnodes: vec!["node-b".to_string(), "node-a".to_string()],
        };

        let first = manifest.digest_hex().expect("digest should succeed");
        let second = manifest.digest_hex().expect("digest should succeed");

        assert_eq!(first, second);
    }

    #[test]
    fn missing_chain_id_is_rejected() {
        let manifest = GenesisManifest {
            chain_id: String::new(),
            runtime_wasm_path: "runtime/mainnet.wasm".to_string(),
            bootnodes: vec![],
        };

        assert_eq!(manifest.validate(), Err(GenesisBuilderError::EmptyChainId));
    }
}
