//! Parallel Proposer Integration Test
//!
//! Tests the integration of parallel-proposer with GPU signature verification
//! and contention prediction in a simulated blockchain environment.

use parallel_proposer::{ParallelProposer, ProposalConfig, TransactionMeta};
use gpu_sig_verifier::{GPUSignatureVerifier, VerifierConfig};
use contention_predictor::{ContentionPredictor, PredictorConfig};
use import_queue_wrapper::{ImportQueueWrapper, QueueConfig};
use tracing::{info, debug, warn};
use tokio::time::Duration;
use anyhow::{Result, anyhow};
use std::sync::Arc;
use std::time::Instant;

#[tokio::test]
async fn test_parallel_proposer_integration() -> Result<()> {
    info!("Starting parallel-proposer integration test");

    // Create configuration
    let proposer_config = ProposalConfig {
        max_parallelism: 8,
        contention_threshold: 0.7,
        gpu_batch_size: 64,
        timeout_seconds: 10,
        signature_batch_size: 32,
    };

    let verifier_config = VerifierConfig {
        batch_size: 64,
        timeout_seconds: 10,
        max_retries: 2,
        gpu_device_id: 0,
        enable_profiling: false,
    };

    let predictor_config = PredictorConfig {
        model_type: contention_predictor::ModelType::RandomForest,
        training_window_seconds: 300,
        prediction_threshold: 0.7,
        enable_online_learning: true,
        feature_importance_threshold: 0.05,
        max_history_size: 1000,
        max_parallel_shards: 16,
    };

    let queue_config = QueueConfig {
        max_queue_size: 1000,
        parallel_workers: 4,
        batch_size: 64,
        verification_timeout: 10,
        cleanup_interval_seconds: 60,
        enable_priority: true,
    };

    // Create components
    let verifier = GPUSignatureVerifier::new(verifier_config);
    let predictor = ContentionPredictor::new(predictor_config);
    let proposer = ParallelProposer::new(proposer_config);
    let mut queue = ImportQueueWrapper::new(queue_config, verifier);

    // Start queue
    queue.start().await?;

    // Create test transactions
    let mut transactions = Vec::new();
    for i in 0..100 {
        let tx = TransactionMeta {
            tx_hash: format!("tx_{}", i),
            sender: format!("0x{:04x}", i),
            receiver: format!("0x{:04x}", i + 1),
            value: (i as u128) * 1_000_000_000,
            gas_limit: 21_000,
            gas_price: 20_000_000 + (i as u128) * 1_000_000,
            nonce: i as u64,
            signature: format!("sig_{}", i),
            contract_address: if i % 10 == 0 { Some(format!("0x{:04x}", i)) } else { None },
            timestamp: 1234567890 + i as u64,
        };
        transactions.push(tx);
    }

    // Submit transactions to queue
    info!("Submitting {} transactions to queue", transactions.len());
    let start_time = Instant::now();

    for (i, tx) in transactions.iter().enumerate() {
        let priority = if i % 2 == 0 { 1 } else { 5 };
        queue.submit_transaction(tx.clone(), priority).await?;
    }

    // Wait for processing
    tokio::time::sleep(Duration::from_secs(5)).await;

    // Get queue statistics
    let queue_stats = queue.get_stats().await;
    info!("Queue statistics: {:?}", queue_stats);

    // Create proposal
    info!("Creating parallel proposal");
    let proposal = proposer.create_proposal().await?;

    // Verify proposal
    assert!(!proposal.block_hash.is_empty());
    assert!(proposal.transactions.len() > 0);
    assert!(proposal.execution_order.len() > 0);
    assert!(proposal.contention_predictions.len() > 0);
    assert!(proposal.verification_stats.total_verified > 0);

    // Get predictor statistics
    let predictor_stats = predictor.get_stats().await;
    info!("Predictor statistics: {:?}", predictor_stats);

    // Get verifier statistics
    let verifier_stats = verifier.get_stats().await;
    info!("Verifier statistics: {:?}", verifier_stats);

    // Calculate performance metrics
    let processing_time = start_time.elapsed();
    info!("Processed {} transactions in {:?}", transactions.len(), processing_time);

    // Test contention prediction
    info!("Testing contention prediction");
    let predictions = predictor.predict_contentions(&transactions).await?;
    assert_eq!(predictions.len(), transactions.len());

    for prediction in predictions {
        assert!(prediction.contention_score >= 0.0 && prediction.contention_score <= 1.0);
        assert!(prediction.priority >= 1 && prediction.priority <= 3);
    }

    // Test signature verification
    info!("Testing signature verification");
    let signatures = transactions.iter().map(|tx| (&tx.signature, tx.tx_hash.as_bytes())).collect::<Vec<_>>();
    let verification_results = verifier.verify_signatures(signatures).await?;
    assert_eq!(verification_results.len(), transactions.len());

    for result in verification_results {
        assert!(result.verified || result.error_message.is_some());
    }

    // Stop queue
    queue.stop().await?;

    info!("Parallel-proposer integration test completed successfully");

    Ok(())
}

#[tokio::test]
async fn test_contention_prediction_accuracy() -> Result<()> {
    info!("Testing contention prediction accuracy");

    let predictor_config = PredictorConfig::default();
    let predictor = ContentionPredictor::new(predictor_config);

    // Create test transactions with known contention patterns
    let high_contention_tx = TransactionMeta {
        tx_hash: "high_contention".to_string(),
        sender: "0x1111".to_string(),
        receiver: "0x2222".to_string(),
        value: 5_000_000_000,
        gas_limit: 21_000,
        gas_price: 100_000_000,
        nonce: 1,
        signature: "valid_sig".to_string(),
        contract_address: Some("0x3333".to_string()),
        timestamp: 1234567890,
    };

    let low_contention_tx = TransactionMeta {
        tx_hash: "low_contention".to_string(),
        sender: "0x4444".to_string(),
        receiver: "0x5555".to_string(),
        value: 100_000_000,
        gas_limit: 21_000,
        gas_price: 10_000_000,
        nonce: 2,
        signature: "valid_sig".to_string(),
        contract_address: None,
        timestamp: 1234567891,
    };

    // Predict contention
    let predictions = predictor.predict_contentions(&[high_contention_tx.clone(), low_contention_tx.clone()]).await?;

    // Verify high contention transaction
    let high_prediction = predictions.iter().find(|p| p.tx_hash == "high_contention").unwrap();
    assert!(high_prediction.contention_score > 0.5);
    assert_eq!(high_prediction.priority, 1);

    // Verify low contention transaction
    let low_prediction = predictions.iter().find(|p| p.tx_hash == "low_contention").unwrap();
    assert!(low_prediction.contention_score < 0.5);
    assert_eq!(low_prediction.priority, 3);

    info!("Contention prediction accuracy test completed successfully");

    Ok(())
}

#[tokio::test]
async fn test_gpu_verification_performance() -> Result<()> {
    info!("Testing GPU verification performance");

    let verifier_config = VerifierConfig {
        batch_size: 128,
        timeout_seconds: 30,
        max_retries: 3,
        gpu_device_id: 0,
        enable_profiling: true,
    };

    let verifier = GPUSignatureVerifier::new(verifier_config);

    // Create test signatures
    let mut signatures = Vec::new();
    for i in 0..1000 {
        signatures.push((format!("sig_{}", i), format!("data_{}", i).as_bytes()));
    }

    // Measure verification time
    let start_time = Instant::now();
    let results = verifier.verify_signatures(signatures).await?;
    let elapsed_time = start_time.elapsed();

    info!("Verified 1000 signatures in {:?}", elapsed_time);
    assert!(results.len() == 1000);

    // Check statistics
    let stats = verifier.get_stats().await;
    assert!(stats.total_verifications == 1000);
    assert!(stats.successful_verifications > 0);
    assert!(stats.failed_verifications >= 0);
    assert!(stats.average_time_ms > 0.0);

    info!("GPU verification performance test completed successfully");

    Ok(())
}
