//! Feature extraction from transaction metadata.
//!
//! Converts raw TX metadata + heatmap context into fixed-size feature vectors
//! suitable for ML model inference.

use crate::{FeatureVector, HeatmapEntry, TxMetadata};
use std::collections::HashMap;

/// Feature indices (into the 64-dim feature vector).
const FEAT_GAS_LIMIT: usize = 0;
const FEAT_VALUE_LOG: usize = 1;
const FEAT_CALLDATA_LEN: usize = 2;
const FEAT_NONCE: usize = 3;
const FEAT_HAS_TARGET: usize = 4;
const FEAT_SELECTOR_0: usize = 5;
const FEAT_SELECTOR_1: usize = 6;
const FEAT_SELECTOR_2: usize = 7;
const FEAT_SELECTOR_3: usize = 8;
const FEAT_SENDER_ENTROPY: usize = 9;
const FEAT_TARGET_ENTROPY: usize = 10;
const FEAT_SENDER_HEATMAP_TEMP: usize = 11;
const FEAT_TARGET_HEATMAP_TEMP: usize = 12;
const FEAT_SENDER_CONFLICT_RATE: usize = 13;
const FEAT_TARGET_CONFLICT_RATE: usize = 14;
// Features 15-63 are reserved for extended context

/// Extract a feature vector from a single transaction.
pub fn extract_features(
    tx: &TxMetadata,
    heatmap: &HashMap<Vec<u8>, HeatmapEntry>,
) -> FeatureVector {
    let mut features = vec![0.0f32; 64];

    // Basic transaction features
    features[FEAT_GAS_LIMIT] = normalize_gas(tx.gas_limit);
    features[FEAT_VALUE_LOG] = log_normalize(tx.value);
    features[FEAT_CALLDATA_LEN] = (tx.calldata_len as f32) / 65536.0;
    features[FEAT_NONCE] = (tx.nonce as f32).min(10000.0) / 10000.0;
    features[FEAT_HAS_TARGET] = if tx.target.is_some() { 1.0 } else { 0.0 };

    // Function selector bytes (normalized 0-1)
    if let Some(sel) = &tx.selector {
        features[FEAT_SELECTOR_0] = sel[0] as f32 / 255.0;
        features[FEAT_SELECTOR_1] = sel[1] as f32 / 255.0;
        features[FEAT_SELECTOR_2] = sel[2] as f32 / 255.0;
        features[FEAT_SELECTOR_3] = sel[3] as f32 / 255.0;
    }

    // Entropy of sender/target addresses (proxy for address diversity)
    features[FEAT_SENDER_ENTROPY] = byte_entropy(&tx.sender);
    features[FEAT_TARGET_ENTROPY] = tx.target.as_ref().map(|t| byte_entropy(t)).unwrap_or(0.0);

    // Heatmap features
    let sender_key = tx.sender.to_vec();
    if let Some(entry) = heatmap.get(&sender_key) {
        features[FEAT_SENDER_HEATMAP_TEMP] = entry.temperature as f32;
        features[FEAT_SENDER_CONFLICT_RATE] = if entry.access_count > 0 {
            entry.conflict_count as f32 / entry.access_count as f32
        } else {
            0.0
        };
    }

    if let Some(target) = &tx.target {
        let target_key = target.to_vec();
        if let Some(entry) = heatmap.get(&target_key) {
            features[FEAT_TARGET_HEATMAP_TEMP] = entry.temperature as f32;
            features[FEAT_TARGET_CONFLICT_RATE] = if entry.access_count > 0 {
                entry.conflict_count as f32 / entry.access_count as f32
            } else {
                0.0
            };
        }
    }

    FeatureVector {
        features,
        tx_hash: tx.tx_hash,
    }
}

/// Normalize gas limit to [0, 1] range.
fn normalize_gas(gas: u64) -> f32 {
    // Block gas limit ~30M, normalize to that
    (gas as f32) / 30_000_000.0
}

/// Log-normalize a u128 value.
fn log_normalize(value: u128) -> f32 {
    if value == 0 {
        return 0.0;
    }
    // log2(value) / 128 to normalize
    let bits = 128 - value.leading_zeros();
    (bits as f32) / 128.0
}

/// Compute byte-level Shannon entropy of an address (normalized 0-1).
fn byte_entropy(addr: &[u8; 32]) -> f32 {
    let mut counts = [0u32; 256];
    for &b in addr.iter() {
        counts[b as usize] += 1;
    }
    let len = addr.len() as f64;
    let entropy: f64 = counts
        .iter()
        .filter(|&&c| c > 0)
        .map(|&c| {
            let p = c as f64 / len;
            -p * p.log2()
        })
        .sum();
    // Max entropy for 32 bytes of uniform random = log2(256) = 8 bits
    (entropy / 8.0) as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn feature_vector_has_correct_dimension() {
        let tx = TxMetadata {
            tx_hash: [0x01; 32],
            sender: [0xAA; 32],
            target: Some([0xBB; 32]),
            selector: Some([0xA9, 0x05, 0x9C, 0xBB]),
            gas_limit: 100_000,
            value: 1_000_000_000_000_000_000, // 1 ETH
            calldata_len: 68,
            nonce: 42,
        };

        let fv = extract_features(&tx, &HashMap::new());
        assert_eq!(fv.features.len(), 64);
        assert!(fv.features[FEAT_GAS_LIMIT] > 0.0);
        assert!(fv.features[FEAT_HAS_TARGET] == 1.0);
    }

    #[test]
    fn zero_value_tx_features() {
        let tx = TxMetadata {
            tx_hash: [0x00; 32],
            sender: [0x00; 32],
            target: None,
            selector: None,
            gas_limit: 0,
            value: 0,
            calldata_len: 0,
            nonce: 0,
        };

        let fv = extract_features(&tx, &HashMap::new());
        assert_eq!(fv.features[FEAT_VALUE_LOG], 0.0);
        assert_eq!(fv.features[FEAT_HAS_TARGET], 0.0);
    }
}
