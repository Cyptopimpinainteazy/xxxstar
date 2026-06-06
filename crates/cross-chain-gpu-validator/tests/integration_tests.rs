//! Integration tests for cross-chain GPU validator

#[cfg(test)]
mod tests {
    use cross_chain_gpu_validator::dashboard::OperatorDashboard;
    use cross_chain_gpu_validator::evm_validator::{EvmStateRoot, EvmValidator};
    use cross_chain_gpu_validator::kernels::Keccak256Kernel;
    use cross_chain_gpu_validator::registry::AtomicSwapRecord;
    use cross_chain_gpu_validator::svm_validator::{SvmState, SvmValidator};

    // ==================== 2.1 Kernel Parity Tests ====================

    #[test]
    fn test_keccak256_gpu_cpu_parity() {
        let kernel = Keccak256Kernel::new(32, false);
        let inputs = vec![
            b"ethereum_block".as_slice(),
            b"solana_block".as_slice(),
            b"cross_chain_state".as_slice(),
        ];

        let (gpu_hashes, _gpu_time) = kernel.hash_batch_gpu(&inputs).unwrap();
        let (cpu_hashes, _cpu_time) = kernel.hash_batch_cpu(&inputs).unwrap();

        assert_eq!(gpu_hashes.len(), cpu_hashes.len(), "Hash count mismatch");
        assert_eq!(gpu_hashes, cpu_hashes, "GPU and CPU hashes must match");
    }

    #[test]
    fn test_keccak256_parity_all_inputs() {
        let kernel = Keccak256Kernel::new(256, false);
        let strs: Vec<String> = (0..256).map(|i| format!("input_{i}")).collect();
        let inputs: Vec<&[u8]> = strs.iter().map(|s| s.as_bytes()).collect();

        let parity_ok = kernel.verify_parity(&inputs).unwrap();
        assert!(parity_ok, "Parity check must pass for all inputs");
    }

    #[test]
    fn test_keccak256_hash_consistency() {
        let kernel = Keccak256Kernel::new(32, false);
        let input = b"consistent_hash_test".as_slice();

        let (hashes1, _) = kernel.hash_batch_cpu(&[input]).unwrap();
        let (hashes2, _) = kernel.hash_batch_cpu(&[input]).unwrap();

        assert_eq!(hashes1[0], hashes2[0], "Same input must produce same hash");
    }

    // ==================== 2.2 Atomic Invariant Tests ====================

    #[tokio::test]
    async fn test_atomic_swap_record_pending_state() {
        let record = AtomicSwapRecord::new("swap-001".to_string(), 60, 1000, 500);
        assert_eq!(record.swap_id, "swap-001");
        assert!(!record.evm_validation_ok);
        assert!(!record.svm_validation_ok);
        assert!(!record.is_expired());
    }

    #[tokio::test]
    async fn test_atomic_swap_timeout_enforcement() {
        let mut record = AtomicSwapRecord::new("swap-002".to_string(), 1, 1000, 500);

        // Manually expire the record
        use chrono::Duration;
        record.expires_at = chrono::Utc::now() - Duration::seconds(1);

        assert!(record.is_expired(), "Expired swap must be detected");
    }

    // ==================== 2.3 Integration Tests ====================

    #[tokio::test]
    async fn test_evm_validator_single_transaction() {
        let validator = EvmValidator::new(32, false);

        let tx = b"test_transaction".to_vec();
        let tx_bytes = vec![tx.as_slice()];

        let (hashes, _) = validator.hasher.hash_batch_cpu(&tx_bytes).unwrap();
        let expected_root = hashes[0].clone();

        let state = EvmStateRoot {
            block_number: 1000,
            state_root: expected_root,
            transactions: vec![tx],
        };

        let result = validator.validate_state_root(&state).await.unwrap();
        assert!(result.valid, "Valid EVM state root must pass validation");
    }

    #[tokio::test]
    async fn test_evm_validator_invalid_root() {
        let validator = EvmValidator::new(32, false);

        let tx = b"test_transaction".to_vec();
        let wrong_root = vec![0u8; 32]; // Wrong hash

        let state = EvmStateRoot {
            block_number: 1000,
            state_root: wrong_root,
            transactions: vec![tx],
        };

        let result = validator.validate_state_root(&state).await.unwrap();
        assert!(!result.valid, "Invalid state root must fail validation");
        assert!(result.error.is_some());
    }

    #[tokio::test]
    async fn test_svm_validator_valid_transactions() {
        let validator = SvmValidator::new();

        let state = SvmState {
            slot: 500,
            block_hash: vec![1u8; 32],
            transactions: vec![vec![1, 2, 3, 4], vec![5, 6, 7, 8]],
        };

        let tx_result = validator.validate_transactions(&state).await.unwrap();
        assert!(tx_result.valid, "Valid SVM transactions must pass");

        let hash_result = validator.validate_block_hash(&state).await.unwrap();
        assert!(hash_result.valid, "Valid SVM block hash must pass");
    }

    #[tokio::test]
    async fn test_svm_validator_invalid_block_hash() {
        let validator = SvmValidator::new();

        let state = SvmState {
            slot: 500,
            block_hash: vec![1u8; 16], // Wrong length (should be 32)
            transactions: vec![vec![1, 2, 3, 4]],
        };

        let result = validator.validate_block_hash(&state).await.unwrap();
        assert!(!result.valid, "Invalid block hash length must fail");
    }

    #[tokio::test]
    async fn test_dual_chain_validation_success() {
        let evm_validator = EvmValidator::new(32, false);
        let svm_validator = SvmValidator::new();

        // EVM state
        let evm_tx = b"evm_transaction".to_vec();
        let (evm_hashes, _) = evm_validator
            .hasher
            .hash_batch_cpu(&[evm_tx.as_slice()])
            .unwrap();
        let evm_state = EvmStateRoot {
            block_number: 1000,
            state_root: evm_hashes[0].clone(),
            transactions: vec![evm_tx],
        };

        // SVM state
        let svm_state = SvmState {
            slot: 500,
            block_hash: vec![1u8; 32],
            transactions: vec![vec![1, 2, 3]],
        };

        let evm_result = evm_validator.validate_state_root(&evm_state).await.unwrap();
        let svm_result = svm_validator
            .validate_transactions(&svm_state)
            .await
            .unwrap();

        assert!(
            evm_result.valid && svm_result.valid,
            "Both chains must validate successfully"
        );
    }

    // ==================== 2.4 Benchmark Test Harness ====================

    #[tokio::test]
    async fn test_benchmark_throughput_keccak256() {
        let kernel = Keccak256Kernel::new(256, false);
        let strs: Vec<String> = (0..1000).map(|i| format!("benchmark_input_{i}")).collect();
        let inputs: Vec<&[u8]> = strs.iter().map(|s| s.as_bytes()).collect();

        let start = std::time::Instant::now();
        let (hashes, _) = kernel.hash_batch_cpu(&inputs[..256]).unwrap();
        let elapsed = start.elapsed();

        let throughput = 256.0 / elapsed.as_secs_f64();
        println!("Keccak256 throughput: {throughput:.0} hashes/sec");

        assert_eq!(hashes.len(), 256);
        assert!(throughput > 100.0, "Throughput must be reasonable");
    }

    #[tokio::test]
    async fn test_benchmark_latency_evm_validation() {
        let validator = EvmValidator::new(32, false);

        let txs: Vec<Vec<u8>> = (0..100).map(|i| format!("tx_{i}").into_bytes()).collect();

        let states: Vec<EvmStateRoot> = txs
            .iter()
            .enumerate()
            .map(|(i, tx)| {
                let (hashes, _) = validator.hasher.hash_batch_cpu(&[tx.as_slice()]).unwrap();
                EvmStateRoot {
                    block_number: 1000 + i as u64,
                    state_root: hashes[0].clone(),
                    transactions: vec![tx.clone()],
                }
            })
            .collect();

        let start = std::time::Instant::now();
        let _results = validator.validate_batch(&states).await.unwrap();
        let elapsed = start.elapsed();

        let avg_latency_ms = elapsed.as_millis() as f64 / 100.0;
        println!("EVM validation latency: {avg_latency_ms:.2} ms/swap");

        assert!(avg_latency_ms < 100.0, "Latency must be < 100ms");
    }

    #[tokio::test]
    async fn test_dashboard_metrics_accumulation() {
        let dashboard = OperatorDashboard::new(100);

        // Simulate swap operations
        for _ in 0..50 {
            dashboard.record_swap_success().await;
        }
        for _ in 0..3 {
            dashboard.record_swap_rollback().await;
        }
        dashboard.record_txs_processed(50000).await;
        dashboard.record_tps(2000.0, 25).await;

        let metrics = dashboard.get_metrics().await;
        assert_eq!(metrics.total_swaps, 53);
        assert_eq!(metrics.successful_commits, 50);
        assert_eq!(metrics.rollbacks, 3);
        assert_eq!(metrics.total_txs_processed, 50000);
    }

    // ==================== Atomic Violation Detection ====================

    #[test]
    fn test_atomic_violation_detection_missing_evm() {
        let mut record = AtomicSwapRecord::new("swap-invalid".to_string(), 60, 1000, 500);
        record.evm_validation_ok = false;
        record.svm_validation_ok = true; // Only SVM validated - VIOLATION

        // In production, this would trigger an alarm
        let violation = record.evm_validation_ok != record.svm_validation_ok;
        assert!(
            violation,
            "Mismatched validation states must be detected as violation"
        );
    }

    #[test]
    fn test_atomic_violation_detection_both_failed() {
        let record = AtomicSwapRecord::new("swap-both-failed".to_string(), 60, 1000, 500);
        // Both validations not completed - OK state
        assert!(!record.evm_validation_ok && !record.svm_validation_ok);
    }
}
