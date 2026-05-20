//! ML model wrapper for contention prediction.
//!
//! In production this wraps a TensorRT-optimized transformer model (~12MB FP16).
//! For now it uses a simple heuristic-based model that can be replaced with
//! a real ML backend.

use crate::{AccessPrediction, FeatureVector, PredictionError};

/// The prediction model.
pub struct PredictionModel {
    feature_dim: usize,
    use_gpu: bool,
    // In production: TensorRT engine handle, ONNX session, etc.
}

impl PredictionModel {
    /// Create a new prediction model.
    pub fn new(feature_dim: usize, use_gpu: bool) -> Self {
        Self {
            feature_dim,
            use_gpu,
        }
    }

    /// Run batch inference on feature vectors.
    ///
    /// Returns predicted access patterns for each transaction.
    pub fn predict_batch(
        &self,
        features: &[FeatureVector],
    ) -> Result<Vec<Vec<AccessPrediction>>, PredictionError> {
        // Heuristic model: predict access patterns based on feature analysis.
        // In production, this would call TensorRT or ONNX Runtime.

        let mut predictions = Vec::with_capacity(features.len());

        for fv in features {
            let access_preds = self.predict_single(fv)?;
            predictions.push(access_preds);
        }

        Ok(predictions)
    }

    /// Predict access patterns for a single transaction.
    fn predict_single(&self, fv: &FeatureVector) -> Result<Vec<AccessPrediction>, PredictionError> {
        let mut preds = Vec::new();

        // Heuristic: Use sender and target bytes as predicted storage keys.
        // The selector hints at whether it's a read or write.

        // Feature indices (from feature_extractor.rs)
        let has_target = fv.features[4]; // FEAT_HAS_TARGET
        let selector_0 = fv.features[5];
        let gas_limit = fv.features[0];

        // Predict sender state access (always accessed)
        preds.push(AccessPrediction {
            storage_key: fv.tx_hash[..8].to_vec(), // sender nonce key
            is_write: true,
            confidence: 9500, // 95% — nonce always written
        });

        if has_target > 0.5 {
            // Target contract gets accessed
            let is_likely_write = selector_0 > 0.5 || gas_limit > 0.003; // > 100k gas

            preds.push(AccessPrediction {
                storage_key: fv.tx_hash[8..16].to_vec(),
                is_write: is_likely_write,
                confidence: if is_likely_write { 7500 } else { 8500 },
            });

            // If high gas, predict additional storage slot access
            if gas_limit > 0.01 {
                preds.push(AccessPrediction {
                    storage_key: fv.tx_hash[16..24].to_vec(),
                    is_write: is_likely_write,
                    confidence: 6000,
                });
            }
        }

        // Heatmap temperature features influence confidence
        let sender_temp = fv.features[11]; // FEAT_SENDER_HEATMAP_TEMP
        let target_temp = fv.features[12]; // FEAT_TARGET_HEATMAP_TEMP

        // Hot addresses get higher confidence predictions
        if sender_temp > 0.5 || target_temp > 0.5 {
            for pred in &mut preds {
                pred.confidence = (pred.confidence as f32 * 1.05).min(10000.0) as u16;
            }
        }

        Ok(preds)
    }

    /// Load a model from a file path.
    ///
    /// In production: loads ONNX/TensorRT model.
    pub fn load_from_path(_path: &str) -> Result<Self, PredictionError> {
        Ok(Self {
            feature_dim: 64,
            use_gpu: false,
        })
    }

    /// Report model info.
    pub fn info(&self) -> ModelInfo {
        ModelInfo {
            feature_dim: self.feature_dim,
            use_gpu: self.use_gpu,
            model_type: "heuristic-v1".to_string(),
            size_bytes: 0,
        }
    }
}

/// Model metadata.
#[derive(Debug, Clone)]
pub struct ModelInfo {
    pub feature_dim: usize,
    pub use_gpu: bool,
    pub model_type: String,
    pub size_bytes: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heuristic_model_produces_predictions() {
        let model = PredictionModel::new(64, false);

        let fv = FeatureVector {
            features: {
                let mut f = vec![0.0; 64];
                f[0] = 0.003; // gas
                f[4] = 1.0; // has_target
                f[5] = 0.7; // selector_0
                f
            },
            tx_hash: [0xAB; 32],
        };

        let preds = model.predict_single(&fv).unwrap();
        assert!(!preds.is_empty());
        assert!(preds[0].confidence > 0);
    }

    #[test]
    fn batch_prediction_count_matches_input() {
        let model = PredictionModel::new(64, false);

        let features: Vec<FeatureVector> = (0..5)
            .map(|i| FeatureVector {
                features: vec![0.0; 64],
                tx_hash: [i as u8; 32],
            })
            .collect();

        let preds = model.predict_batch(&features).unwrap();
        assert_eq!(preds.len(), 5);
    }
}
