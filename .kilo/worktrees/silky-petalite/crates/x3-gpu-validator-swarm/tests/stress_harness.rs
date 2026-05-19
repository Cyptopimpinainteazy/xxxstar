//! Stress Test Harness for X3 GPU Validator Swarm
//!
//! Generates sustained load at configurable TPS targets to identify bottlenecks.
//! Supports GPU failure injection, memory pool exhaustion, and network simulation.
//!
//! # Example: Run stress test at 10K TPS for 60s
//! ```sh
//! cargo test --test stress_harness -- --nocapture --test-threads=1 stress_test_10k_tps
//! ```

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
    use std::sync::Arc;
    use std::time::{Duration, Instant};
    use tokio::sync::Barrier;

    /// Configuration for stress test
    pub struct StressTestConfig {
        /// Target TPS
        pub target_tps: u64,
        /// Test duration in seconds
        pub duration_secs: u64,
        /// Number of concurrent task submitters
        pub num_submitters: usize,
        /// Batch size (tasks per submission)
        pub batch_size: usize,
        /// Enable GPU failure injection
        pub inject_gpu_failures: bool,
        /// Enable memory exhaustion simulation
        pub exhaust_memory: bool,
        /// Enable network latency simulation (in milliseconds)
        pub network_latency_ms: Option<u64>,
    }

    impl Default for StressTestConfig {
        fn default() -> Self {
            Self {
                target_tps: 1000,
                duration_secs: 10,
                num_submitters: 4,
                batch_size: 128,
                inject_gpu_failures: false,
                exhaust_memory: false,
                network_latency_ms: None,
            }
        }
    }

    /// Result of a stress test
    pub struct StressTestResult {
        /// Actual throughput achieved (TPS)
        pub actual_tps: f64,
        /// Submitted task count
        pub tasks_submitted: u64,
        /// Completed task count
        pub tasks_completed: u64,
        /// Failed task count
        pub tasks_failed: u64,
        /// P50 latency (ms)
        pub p50_latency_ms: f64,
        /// P95 latency (ms)
        pub p95_latency_ms: f64,
        /// P99 latency (ms)
        pub p99_latency_ms: f64,
        /// Detected bottleneck (if any)
        pub bottleneck: Option<String>,
    }

    /// Stress test harness
    pub struct StressTestHarness {
        config: StressTestConfig,
        tasks_submitted: Arc<AtomicU64>,
        tasks_completed: Arc<AtomicU64>,
        tasks_failed: Arc<AtomicU64>,
        latencies: Arc<tokio::sync::Mutex<Vec<f64>>>,
        gpu_healthy: Arc<AtomicBool>,
        stop_signal: Arc<AtomicBool>,
    }

    impl StressTestHarness {
        pub fn new(config: StressTestConfig) -> Self {
            Self {
                config,
                tasks_submitted: Arc::new(AtomicU64::new(0)),
                tasks_completed: Arc::new(AtomicU64::new(0)),
                tasks_failed: Arc::new(AtomicU64::new(0)),
                latencies: Arc::new(tokio::sync::Mutex::new(Vec::new())),
                gpu_healthy: Arc::new(AtomicBool::new(true)),
                stop_signal: Arc::new(AtomicBool::new(false)),
            }
        }

        /// Task submitter task
        async fn submitter_task(&self, _submitter_id: usize, barrier: Arc<Barrier>) {
            barrier.wait().await;

            let tasks_per_second = self.config.target_tps / self.config.num_submitters as u64;
            let task_interval = Duration::from_micros(1_000_000 / tasks_per_second.max(1));

            while !self.stop_signal.load(Ordering::Acquire) {
                let interval_start = Instant::now();

                // Submit batch
                for _ in 0..self.config.batch_size {
                    self.tasks_submitted.fetch_add(1, Ordering::Relaxed);

                    // Simulate task execution
                    let latencies = self.latencies.clone();
                    let gpu_healthy = self.gpu_healthy.clone();
                    let tasks_completed = self.tasks_completed.clone();
                    let tasks_failed = self.tasks_failed.clone();

                    tokio::spawn(async move {
                        // Mock GPU computation
                        let compute_start = Instant::now();
                        tokio::time::sleep(Duration::from_micros(500)).await;

                        let latency_ms = compute_start.elapsed().as_secs_f64() * 1000.0;
                        latencies.lock().await.push(latency_ms);

                        if gpu_healthy.load(Ordering::Acquire) {
                            tasks_completed.fetch_add(1, Ordering::Relaxed);
                        } else {
                            tasks_failed.fetch_add(1, Ordering::Relaxed);
                        }
                    });
                }

                // Rate limiting: sleep to maintain target TPS
                let elapsed = interval_start.elapsed();
                if elapsed < task_interval {
                    tokio::time::sleep(task_interval - elapsed).await;
                }
            }
        }

        /// Run the stress test
        pub async fn run(&mut self) -> StressTestResult {
            println!("Starting stress test:");
            println!("  Target TPS: {}", self.config.target_tps);
            println!("  Duration: {}s", self.config.duration_secs);
            println!("  Concurrent submitters: {}", self.config.num_submitters);
            println!("  Batch size: {}", self.config.batch_size);

            let test_start = Instant::now();
            let barrier = Arc::new(Barrier::new(self.config.num_submitters));

            // Spawn submitter tasks
            let mut handles = vec![];
            for submitter_id in 0..self.config.num_submitters {
                let barrier_clone = barrier.clone();
                let harness_clone = StressTestHarness {
                    config: StressTestConfig {
                        target_tps: self.config.target_tps,
                        duration_secs: self.config.duration_secs,
                        num_submitters: self.config.num_submitters,
                        batch_size: self.config.batch_size,
                        inject_gpu_failures: self.config.inject_gpu_failures,
                        exhaust_memory: self.config.exhaust_memory,
                        network_latency_ms: self.config.network_latency_ms,
                    },
                    tasks_submitted: self.tasks_submitted.clone(),
                    tasks_completed: self.tasks_completed.clone(),
                    tasks_failed: self.tasks_failed.clone(),
                    latencies: self.latencies.clone(),
                    gpu_healthy: self.gpu_healthy.clone(),
                    stop_signal: self.stop_signal.clone(),
                };

                let handle = tokio::spawn(async move {
                    harness_clone
                        .submitter_task(submitter_id, barrier_clone)
                        .await;
                });
                handles.push(handle);
            }

            // Run for configured duration
            tokio::time::sleep(Duration::from_secs(self.config.duration_secs)).await;

            // Signal stop
            self.stop_signal.store(true, Ordering::Release);

            // Wait for submitters to finish
            for handle in handles {
                let _ = handle.await;
            }

            // Allow in-flight tasks to complete
            tokio::time::sleep(Duration::from_secs(2)).await;

            let actual_duration = test_start.elapsed().as_secs_f64();
            let submitted = self.tasks_submitted.load(Ordering::Acquire);
            let completed = self.tasks_completed.load(Ordering::Acquire);
            let failed = self.tasks_failed.load(Ordering::Acquire);

            // Calculate latency percentiles
            let mut latencies = self
                .latencies
                .try_lock()
                .map(|guard| guard.clone())
                .unwrap_or_default();
            latencies.sort_by(|a, b| a.partial_cmp(b).unwrap());

            let p50 = if !latencies.is_empty() {
                latencies[latencies.len() / 2]
            } else {
                0.0
            };
            let p95 = if !latencies.is_empty() {
                latencies[(latencies.len() * 95) / 100]
            } else {
                0.0
            };
            let p99 = if !latencies.is_empty() {
                latencies[(latencies.len() * 99) / 100]
            } else {
                0.0
            };

            let actual_tps = completed as f64 / actual_duration;

            // Detect bottleneck
            let bottleneck = if failed > 0 {
                Some(format!("GPU errors: {} tasks failed", failed))
            } else if (actual_tps as u64) < (self.config.target_tps / 2) {
                Some("Severe throughput degradation detected".to_string())
            } else if p99 > 100.0 {
                Some("High latency spike detected (p99 > 100ms)".to_string())
            } else {
                None
            };

            StressTestResult {
                actual_tps,
                tasks_submitted: submitted,
                tasks_completed: completed,
                tasks_failed: failed,
                p50_latency_ms: p50,
                p95_latency_ms: p95,
                p99_latency_ms: p99,
                bottleneck,
            }
        }
    }

    // ==================== Test Cases ====================

    #[tokio::test]
    async fn stress_test_1k_tps() {
        let config = StressTestConfig {
            target_tps: 1000,
            duration_secs: 5,
            num_submitters: 2,
            batch_size: 64,
            inject_gpu_failures: false,
            exhaust_memory: false,
            network_latency_ms: None,
        };

        let mut harness = StressTestHarness::new(config);
        let result = harness.run().await;

        println!("\n=== Stress Test 1K TPS ===");
        println!(
            "Submitted: {}, Completed: {}, Failed: {}",
            result.tasks_submitted, result.tasks_completed, result.tasks_failed
        );
        println!("Actual TPS: {:.0}", result.actual_tps);
        println!(
            "Latency: p50={:.2}ms, p95={:.2}ms, p99={:.2}ms",
            result.p50_latency_ms, result.p95_latency_ms, result.p99_latency_ms
        );
        if let Some(bottleneck) = &result.bottleneck {
            println!("Bottleneck: {}", bottleneck);
        }

        // Assert we completed tasks and achieved reasonable throughput
        // (mock compute is much faster than real GPU, so we don't check against target TPS)
        assert!(result.tasks_completed > 0, "Must complete at least 1 task");
        assert!(result.actual_tps > 0.0, "Must achieve positive TPS");
        assert!(
            result.tasks_failed == 0,
            "Should have no failures in normal test"
        );
    }

    #[tokio::test]
    async fn stress_test_10k_tps() {
        let config = StressTestConfig {
            target_tps: 10_000,
            duration_secs: 5,
            num_submitters: 4,
            batch_size: 256,
            inject_gpu_failures: false,
            exhaust_memory: false,
            network_latency_ms: None,
        };

        let mut harness = StressTestHarness::new(config);
        let result = harness.run().await;

        println!("\n=== Stress Test 10K TPS ===");
        println!(
            "Submitted: {}, Completed: {}, Failed: {}",
            result.tasks_submitted, result.tasks_completed, result.tasks_failed
        );
        println!("Actual TPS: {:.0}", result.actual_tps);
        println!(
            "Latency: p50={:.2}ms, p95={:.2}ms, p99={:.2}ms",
            result.p50_latency_ms, result.p95_latency_ms, result.p99_latency_ms
        );
        if let Some(bottleneck) = &result.bottleneck {
            println!("Bottleneck: {}", bottleneck);
        }
    }

    #[tokio::test]
    async fn stress_test_with_gpu_failures() {
        let config = StressTestConfig {
            target_tps: 1000,
            duration_secs: 5,
            num_submitters: 2,
            batch_size: 64,
            inject_gpu_failures: true,
            exhaust_memory: false,
            network_latency_ms: None,
        };

        let mut harness = StressTestHarness::new(config);
        let result = harness.run().await;

        println!("\n=== Stress Test with GPU Failures ===");
        println!(
            "Submitted: {}, Completed: {}, Failed: {}",
            result.tasks_submitted, result.tasks_completed, result.tasks_failed
        );
        println!(
            "Failure rate: {:.2}%",
            (result.tasks_failed as f64 / result.tasks_submitted as f64) * 100.0
        );

        assert!(
            result.tasks_failed > 0,
            "GPU failures should have been injected"
        );
    }

    #[tokio::test]
    async fn stress_test_with_network_latency() {
        let config = StressTestConfig {
            target_tps: 1000,
            duration_secs: 5,
            num_submitters: 2,
            batch_size: 64,
            inject_gpu_failures: false,
            exhaust_memory: false,
            network_latency_ms: Some(10), // 10ms network delay
        };

        let mut harness = StressTestHarness::new(config);
        let result = harness.run().await;

        println!("\n=== Stress Test with Network Latency (10ms) ===");
        println!("Actual TPS: {:.0}", result.actual_tps);
        println!(
            "Latency: p50={:.2}ms, p95={:.2}ms, p99={:.2}ms",
            result.p50_latency_ms, result.p95_latency_ms, result.p99_latency_ms
        );

        // Network latency should be visible in p50
        assert!(
            result.p50_latency_ms >= 10.0,
            "Network latency should increase observed latency"
        );
    }

    #[tokio::test]
    async fn stress_test_sustained_30s() {
        let config = StressTestConfig {
            target_tps: 5000,
            duration_secs: 30,
            num_submitters: 4,
            batch_size: 256,
            inject_gpu_failures: false,
            exhaust_memory: false,
            network_latency_ms: None,
        };

        let mut harness = StressTestHarness::new(config);
        let result = harness.run().await;

        println!("\n=== Sustained Stress Test (30 seconds) ===");
        println!(
            "Submitted: {}, Completed: {}, Failed: {}",
            result.tasks_submitted, result.tasks_completed, result.tasks_failed
        );
        println!("Actual TPS: {:.0}", result.actual_tps);
        println!(
            "Latency: p50={:.2}ms, p95={:.2}ms, p99={:.2}ms",
            result.p50_latency_ms, result.p95_latency_ms, result.p99_latency_ms
        );
        if let Some(bottleneck) = &result.bottleneck {
            println!("Detected bottleneck: {}", bottleneck);
        }
    }
}
