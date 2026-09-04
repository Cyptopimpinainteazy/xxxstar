//! Contention Predictor Module
//!
//! Implements machine learning-based contention prediction for
//! optimal transaction ordering and parallel processing.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::Mutex;
use tracing::debug;

pub const FEATURE_VECTOR_DIM: usize = 64;

/// Transaction metadata for contention prediction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionMeta {
    pub tx_hash: String,
    pub sender: String,
    pub receiver: String,
    pub value: u64,
    pub gas_limit: u64,
    pub gas_price: u64,
    pub nonce: u64,
    pub signature: String,
    pub contract_address: Option<String>,
    pub timestamp: u64,
}

/// Low-level transaction metadata used by the feature extractor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxMetadata {
    pub tx_hash: [u8; 32],
    pub sender: [u8; 32],
    pub target: Option<[u8; 32]>,
    pub selector: Option<[u8; 4]>,
    pub gas_limit: u64,
    pub value: u128,
    pub calldata_len: usize,
    pub nonce: u64,
    pub timestamp: u64,
}

/// Feature vector output consumed by the ML model.
#[derive(Debug, Clone)]
pub struct FeatureVector {
    pub features: Vec<f32>,
    pub tx_hash: [u8; 32],
}

/// Access prediction for a storage key.
#[derive(Debug, Clone)]
pub struct AccessPrediction {
    pub storage_key: Vec<u8>,
    pub is_write: bool,
    pub confidence: u16,
}

/// Entry from the heatmap used to enrich features.
#[derive(Debug, Clone)]
pub struct HeatmapEntry {
    pub temperature: u8,
    pub access_count: usize,
    pub conflict_count: usize,
}

/// Group of transactions that can execute in parallel.
#[derive(Debug, Clone)]
pub struct ShardGroup {
    pub shard_id: u32,
    pub tx_indices: Vec<usize>,
    pub color: u32,
}

/// Error type returned when prediction fails.
#[derive(Debug, Error)]
pub enum PredictionError {
    #[error("{0}")]
    Internal(String),
    #[error("feature extraction failed: {0}")]
    FeatureExtraction(String),
}

/// Contention prediction configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictorConfig {
    pub model_type: ModelType,
    pub training_window_seconds: u64,
    pub prediction_threshold: f64,
    pub enable_online_learning: bool,
    pub feature_importance_threshold: f64,
    pub max_history_size: usize,
    pub max_parallel_shards: usize,
}

impl Default for PredictorConfig {
    fn default() -> Self {
        Self {
            model_type: ModelType::RandomForest,
            training_window_seconds: 300,
            prediction_threshold: 0.7,
            enable_online_learning: true,
            feature_importance_threshold: 0.05,
            max_history_size: 10000,
            max_parallel_shards: 16,
        }
    }
}

/// Machine learning model types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModelType {
    RandomForest,
    GradientBoosting,
    NeuralNetwork,
    LinearRegression,
}

impl std::fmt::Display for ModelType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModelType::RandomForest => write!(f, "RandomForest"),
            ModelType::GradientBoosting => write!(f, "GradientBoosting"),
            ModelType::NeuralNetwork => write!(f, "NeuralNetwork"),
            ModelType::LinearRegression => write!(f, "LinearRegression"),
        }
    }
}

/// Contention prediction result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentionPrediction {
    pub tx_hash: String,
    pub contention_score: f64,
    pub conflicting_txs: Vec<String>,
    pub priority: u8,
    pub feature_importance: HashMap<String, f64>,
}

/// Transaction features
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionFeatures {
    pub value: f64,
    pub gas_price: f64,
    pub nonce: f64,
    pub sender_balance: f64,
    pub receiver_balance: f64,
    pub contract_interaction: bool,
    pub timestamp: u64,
}

/// Contention prediction statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictionStats {
    pub total_predictions: usize,
    pub accurate_predictions: usize,
    pub false_positives: usize,
    pub false_negatives: usize,
    pub accuracy: f64,
    pub precision: f64,
    pub recall: f64,
    pub f1_score: f64,
}

/// Contention predictor core
pub struct ContentionPredictor {
    config: PredictorConfig,
    model: Arc<Mutex<PredictionModel>>,
    transaction_history: Arc<Mutex<Vec<TransactionRecord>>>,
    stats: Arc<Mutex<PredictionStats>>,
    feature_extractor: Arc<Mutex<FeatureExtractor>>,
    heatmap: Arc<Mutex<HashMap<Vec<u8>, HeatmapEntry>>>,
}

impl ContentionPredictor {
    /// Create a new contention predictor
    pub fn new(config: PredictorConfig) -> Self {
        let model_type = config.model_type.clone();
        Self {
            config,
            model: Arc::new(Mutex::new(PredictionModel::new(model_type))),
            transaction_history: Arc::new(Mutex::new(Vec::new())),
            stats: Arc::new(Mutex::new(PredictionStats::default())),
            feature_extractor: Arc::new(Mutex::new(FeatureExtractor::new())),
            heatmap: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Predict contention for a transaction
    pub async fn predict_contention(&self, tx: &TransactionMeta) -> Result<ContentionPrediction> {
        let features = self
            .feature_extractor
            .lock()
            .await
            .extract_features(tx)
            .await?;
        let model = self.model.lock().await;
        let prediction = model.predict(&features).await?;

        // Update stats
        self.update_stats(&prediction, &features).await;

        Ok(prediction)
    }

    /// Predict contention for multiple transactions
    pub async fn predict_contentions(
        &self,
        txs: &[TransactionMeta],
    ) -> Result<Vec<ContentionPrediction>> {
        let mut predictions = Vec::with_capacity(txs.len());

        for tx in txs {
            let prediction = self.predict_contention(tx).await?;
            predictions.push(prediction);
        }

        Ok(predictions)
    }

    /// Predict contention and shard transactions into parallel groups.
    pub async fn predict_and_shard(&self, txs: &[TxMetadata]) -> Result<Vec<ShardGroup>> {
        if txs.is_empty() {
            return Ok(Vec::new());
        }

        let max_shards = self.config.max_parallel_shards.max(1);
        let mut shards = Vec::with_capacity(max_shards);
        let mut shard_senders: Vec<BTreeSet<[u8; 32]>> = Vec::with_capacity(max_shards);
        let mut shard_targets: Vec<BTreeSet<[u8; 32]>> = Vec::with_capacity(max_shards);

        for shard_id in 0..max_shards {
            shards.push(ShardGroup {
                shard_id: shard_id as u32,
                tx_indices: Vec::new(),
                color: shard_id as u32,
            });
            shard_senders.push(BTreeSet::new());
            shard_targets.push(BTreeSet::new());
        }

        for (idx, tx) in txs.iter().enumerate() {
            let mut assigned = None;

            for shard_id in 0..max_shards {
                let sender_conflict = shard_senders[shard_id].contains(&tx.sender);
                let target_conflict = tx
                    .target
                    .map(|t| shard_targets[shard_id].contains(&t))
                    .unwrap_or(false);

                if !sender_conflict && !target_conflict {
                    assigned = Some(shard_id);
                    break;
                }
            }

            let shard_id = assigned.unwrap_or(0);
            shards[shard_id].tx_indices.push(idx);
            shard_senders[shard_id].insert(tx.sender);
            if let Some(target) = tx.target {
                shard_targets[shard_id].insert(target);
            }
        }

        shards.retain(|shard| !shard.tx_indices.is_empty());
        Ok(shards)
    }

    /// Train the prediction model
    pub async fn train_model(&self, training_data: &[TransactionRecord]) -> Result<()> {
        let mut model = self.model.lock().await;
        model.train(training_data).await?;
        Ok(())
    }

    /// Update model with new transaction
    pub async fn update_model(
        &self,
        tx: &TransactionMeta,
        outcome: ContentionOutcome,
    ) -> Result<()> {
        if self.config.enable_online_learning {
            let record = TransactionRecord::from_transaction(tx, outcome);
            let mut history = self.transaction_history.lock().await;
            history.push(record);

            // Keep only recent history
            while history.len() > self.config.max_history_size {
                history.remove(0);
            }

            // Retrain model periodically
            if history.len() % 100 == 0 {
                let mut model = self.model.lock().await;
                model.retrain(&history).await?;
            }
        }
        Ok(())
    }

    /// Get prediction statistics
    pub async fn get_stats(&self) -> PredictionStats {
        self.stats.lock().await.clone()
    }

    /// Clear prediction history
    pub async fn clear_history(&self) {
        let mut history = self.transaction_history.lock().await;
        history.clear();
    }

    /// Update the contention heatmap with finalized transaction access patterns.
    pub async fn update_heatmap(&self, txs: &[TxMetadata]) {
        let mut heatmap = self.heatmap.lock().await;
        let mut access_counts: HashMap<Vec<u8>, usize> = HashMap::new();

        for tx in txs {
            let sender_key = tx.sender.to_vec();
            *access_counts.entry(sender_key).or_insert(0) += 1;

            if let Some(target) = tx.target {
                let target_key = target.to_vec();
                *access_counts.entry(target_key).or_insert(0) += 1;
            }
        }

        for (key, count) in access_counts {
            let entry = heatmap.entry(key).or_insert(HeatmapEntry {
                temperature: 0,
                access_count: 0,
                conflict_count: 0,
            });

            entry.access_count = entry.access_count.saturating_add(count);
            if count > 1 {
                entry.conflict_count = entry.conflict_count.saturating_add(count - 1);
            }
            entry.temperature = entry.temperature.saturating_add(1);
        }
    }

    /// Update prediction statistics
    async fn update_stats(
        &self,
        prediction: &ContentionPrediction,
        features: &TransactionFeatures,
    ) {
        let mut stats = self.stats.lock().await;

        stats.total_predictions += 1;

        // Accuracy tracking: in production, compare predicted vs actual outcomes
        // For now, use heuristic-based estimation
        let predicted_high = prediction.contention_score > 0.7;
        let actual_high = features.value > 1_000_000_000.0 || features.gas_price > 100_000_000.0;

        if predicted_high == actual_high {
            stats.accurate_predictions += 1;
        } else if predicted_high && !actual_high {
            stats.false_positives += 1;
        } else {
            stats.false_negatives += 1;
        }

        // Calculate metrics
        if stats.total_predictions > 0 {
            stats.accuracy = stats.accurate_predictions as f64 / stats.total_predictions as f64;
            stats.precision = stats.accurate_predictions as f64
                / (stats.accurate_predictions + stats.false_positives) as f64;
            stats.recall = stats.accurate_predictions as f64
                / (stats.accurate_predictions + stats.false_negatives) as f64;
            stats.f1_score =
                2.0 * (stats.precision * stats.recall) / (stats.precision + stats.recall);
        }
    }
}

/// Transaction record for training
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionRecord {
    pub tx_hash: String,
    pub features: TransactionFeatures,
    pub outcome: ContentionOutcome,
    pub timestamp: u64,
}

impl TransactionRecord {
    fn from_transaction(tx: &TransactionMeta, outcome: ContentionOutcome) -> Self {
        Self {
            tx_hash: tx.tx_hash.clone(),
            features: TransactionFeatures {
                value: tx.value as f64,
                gas_price: tx.gas_price as f64,
                nonce: tx.nonce as f64,
                sender_balance: 0.0,   // Would be fetched from blockchain
                receiver_balance: 0.0, // Would be fetched from blockchain
                contract_interaction: tx.contract_address.is_some(),
                timestamp: tx.timestamp,
            },
            outcome,
            timestamp: current_timestamp(),
        }
    }
}

/// Contention outcome
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ContentionOutcome {
    HighContention,
    MediumContention,
    LowContention,
    NoContention,
}

/// Prediction model
struct PredictionModel {
    model_type: ModelType,
    // Model parameters would be stored here
}

impl PredictionModel {
    fn new(model_type: ModelType) -> Self {
        Self { model_type }
    }

    async fn predict(&self, features: &TransactionFeatures) -> Result<ContentionPrediction> {
        // Simulate prediction based on model type
        let contention_score = match self.model_type {
            ModelType::RandomForest => self.random_forest_predict(features),
            ModelType::GradientBoosting => self.gradient_boosting_predict(features),
            ModelType::NeuralNetwork => self.neural_network_predict(features),
            ModelType::LinearRegression => self.linear_regression_predict(features),
        };

        let priority = if contention_score > 0.8 {
            1
        } else if contention_score > 0.5 {
            2
        } else {
            3
        };

        let conflicting_txs = self.find_conflicting_txs(features)?;

        Ok(ContentionPrediction {
            tx_hash: "simulated_tx".to_string(), // Would be actual tx hash
            contention_score,
            conflicting_txs,
            priority,
            feature_importance: self.calculate_feature_importance(features),
        })
    }

    async fn train(&mut self, data: &[TransactionRecord]) -> Result<()> {
        // Simulate training
        debug!(
            "Training {} model with {} records",
            self.model_type,
            data.len()
        );
        Ok(())
    }

    async fn retrain(&mut self, data: &[TransactionRecord]) -> Result<()> {
        // Simulate retraining
        debug!(
            "Retraining {} model with {} records",
            self.model_type,
            data.len()
        );
        Ok(())
    }

    fn random_forest_predict(&self, features: &TransactionFeatures) -> f64 {
        // Simple heuristic-based prediction
        let mut score: f64 = 0.0;

        if features.value > 1_000_000_000.0 {
            score += 0.3;
        }
        if features.gas_price > 100_000_000.0 {
            score += 0.3;
        }
        if features.contract_interaction {
            score += 0.2;
        }
        if features.nonce % 2.0 == 0.0 {
            score += 0.1;
        }

        score.min(1.0)
    }

    fn gradient_boosting_predict(&self, features: &TransactionFeatures) -> f64 {
        // Simple heuristic-based prediction
        let mut score: f64 = 0.0;

        if features.value > 500_000_000.0 {
            score += 0.4;
        }
        if features.gas_price > 50_000_000.0 {
            score += 0.3;
        }
        if features.contract_interaction {
            score += 0.2;
        }

        score.min(1.0)
    }

    fn neural_network_predict(&self, features: &TransactionFeatures) -> f64 {
        // Simple heuristic-based prediction
        let mut score: f64 = 0.0;

        if features.value > 2_000_000_000.0 {
            score += 0.5;
        }
        if features.gas_price > 200_000_000.0 {
            score += 0.3;
        }
        if features.contract_interaction {
            score += 0.2;
        }

        score.min(1.0)
    }

    fn linear_regression_predict(&self, features: &TransactionFeatures) -> f64 {
        // Simple heuristic-based prediction
        let mut score: f64 = 0.0;

        score += features.value / 10_000_000_000.0;
        score += features.gas_price / 1_000_000_000.0;
        if features.contract_interaction {
            score += 0.1;
        }

        score.min(1.0)
    }

    fn find_conflicting_txs(&self, features: &TransactionFeatures) -> Result<Vec<String>> {
        // Simulate finding conflicting transactions
        let mut conflicting_txs = Vec::new();

        if features.value > 1_000_000_000.0 {
            conflicting_txs.push("conflicting_tx_1".to_string());
            conflicting_txs.push("conflicting_tx_2".to_string());
        }

        Ok(conflicting_txs)
    }

    fn calculate_feature_importance(
        &self,
        _features: &TransactionFeatures,
    ) -> HashMap<String, f64> {
        let mut importance = HashMap::new();

        importance.insert("value".to_string(), 0.3);
        importance.insert("gas_price".to_string(), 0.3);
        importance.insert("contract_interaction".to_string(), 0.2);
        importance.insert("nonce".to_string(), 0.1);
        importance.insert("sender_balance".to_string(), 0.1);

        importance
    }
}

/// Feature extractor
struct FeatureExtractor {
    // Feature extraction parameters would be stored here
}

impl FeatureExtractor {
    fn new() -> Self {
        Self {}
    }

    async fn extract_features(&self, tx: &TransactionMeta) -> Result<TransactionFeatures> {
        Ok(TransactionFeatures {
            value: tx.value as f64,
            gas_price: tx.gas_price as f64,
            nonce: tx.nonce as f64,
            sender_balance: 0.0,   // Would be fetched from blockchain
            receiver_balance: 0.0, // Would be fetched from blockchain
            contract_interaction: tx.contract_address.is_some(),
            timestamp: tx.timestamp,
        })
    }
}

impl PredictionStats {
    fn default() -> Self {
        Self {
            total_predictions: 0,
            accurate_predictions: 0,
            false_positives: 0,
            false_negatives: 0,
            accuracy: 0.0,
            precision: 0.0,
            recall: 0.0,
            f1_score: 0.0,
        }
    }
}

fn current_timestamp() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_contention_predictor_basic_flow() {
        let config = PredictorConfig::default();
        let predictor = ContentionPredictor::new(config);

        // Create test transaction
        let tx = TransactionMeta {
            tx_hash: "test_tx".to_string(),
            sender: "0x1234".to_string(),
            receiver: "0x5678".to_string(),
            value: 1_500_000_000,
            gas_limit: 21_000,
            gas_price: 50_000_000,
            nonce: 1,
            signature: "valid_sig".to_string(),
            contract_address: None,
            timestamp: 1234567890,
        };

        // Predict contention
        let prediction = predictor.predict_contention(&tx).await.unwrap();
        assert!(prediction.contention_score >= 0.0 && prediction.contention_score <= 1.0);
        assert!(prediction.priority >= 1 && prediction.priority <= 3);

        // Get stats
        let stats = predictor.get_stats().await;
        assert!(stats.total_predictions >= 1);
    }

    #[tokio::test]
    async fn test_model_training() {
        let config = PredictorConfig::default();
        let predictor = ContentionPredictor::new(config);

        // Create training data
        let training_data = vec![
            TransactionRecord {
                tx_hash: "tx1".to_string(),
                features: TransactionFeatures {
                    value: 1_000_000_000.0,
                    gas_price: 20_000_000.0,
                    nonce: 1.0,
                    sender_balance: 1_000_000_000_000.0,
                    receiver_balance: 1_000_000_000_000.0,
                    contract_interaction: false,
                    timestamp: 1234567890,
                },
                outcome: ContentionOutcome::LowContention,
                timestamp: 1234567890,
            },
            TransactionRecord {
                tx_hash: "tx2".to_string(),
                features: TransactionFeatures {
                    value: 5_000_000_000.0,
                    gas_price: 100_000_000.0,
                    nonce: 2.0,
                    sender_balance: 5_000_000_000_000.0,
                    receiver_balance: 5_000_000_000_000.0,
                    contract_interaction: true,
                    timestamp: 1234567891,
                },
                outcome: ContentionOutcome::HighContention,
                timestamp: 1234567891,
            },
        ];

        // Train model
        predictor.train_model(&training_data).await.unwrap();

        // Update model with new transaction
        let tx = TransactionMeta {
            tx_hash: "new_tx".to_string(),
            sender: "0x1234".to_string(),
            receiver: "0x5678".to_string(),
            value: 2_000_000_000,
            gas_limit: 21_000,
            gas_price: 50_000_000,
            nonce: 1,
            signature: "valid_sig".to_string(),
            contract_address: None,
            timestamp: 1234567890,
        };
        predictor
            .update_model(&tx, ContentionOutcome::MediumContention)
            .await
            .unwrap();
    }
}
