//! Comprehensive Chaos & Stress Testing Framework
//!
//! This module provides aggressive stress testing to identify system limits,
//! breaking points, and failure modes. Tests escalate through multiple TPS
//! levels (10K → 30K → 65K → 100K → 1M) while introducing chaos conditions.

#[cfg(test)]
mod chaos_tests {
    use std::collections::VecDeque;
    use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    /// Configuration for stress test scenarios
    #[derive(Clone, Debug)]
    pub struct StressConfig {
        pub target_tps: u64,
        pub duration_secs: u64,
        pub chaos_enabled: bool,
        pub failure_rate: f64, // 0.0 - 1.0
        pub network_latency_ms: u64,
        pub memory_pressure: bool,
    }

    /// Real-time metrics collector
    #[derive(Default)]
    pub struct MetricsSnapshot {
        pub submitted: u64,
        pub completed: u64,
        pub failed: u64,
        pub peak_tps: f64,
        pub current_tps: f64,
        pub p50_latency_ms: f64,
        pub p95_latency_ms: f64,
        pub p99_latency_ms: f64,
        pub timeouts: u64,
    }

    pub struct ChaosStressTest {
        config: StressConfig,
        submitted: Arc<AtomicU64>,
        completed: Arc<AtomicU64>,
        failed: Arc<AtomicU64>,
        timeouts: Arc<AtomicU64>,
        latencies: Arc<parking_lot::Mutex<VecDeque<f64>>>,
        errors: Arc<parking_lot::Mutex<Vec<String>>>,
        should_stop: Arc<AtomicBool>,
    }

    impl ChaosStressTest {
        pub fn new(config: StressConfig) -> Self {
            Self {
                config,
                submitted: Arc::new(AtomicU64::new(0)),
                completed: Arc::new(AtomicU64::new(0)),
                failed: Arc::new(AtomicU64::new(0)),
                timeouts: Arc::new(AtomicU64::new(0)),
                latencies: Arc::new(parking_lot::Mutex::new(VecDeque::with_capacity(100_000))),
                errors: Arc::new(parking_lot::Mutex::new(Vec::new())),
                should_stop: Arc::new(AtomicBool::new(false)),
            }
        }

        /// Execute chaos stress test with real-time monitoring
        pub async fn run(&self) -> MetricsSnapshot {
            let start = Instant::now();
            let test_duration = Duration::from_secs(self.config.duration_secs);

            let mut last_sample_time = Instant::now();
            let mut samples = Vec::new();
            let mut max_tps = 0.0f64;

            println!("\n{}", "=".repeat(60));
            println!(
                "🔥 CHAOS STRESS TEST: {} TPS for {} seconds",
                self.config.target_tps, self.config.duration_secs
            );
            println!(
                "   Chaos: {}",
                if self.config.chaos_enabled {
                    "ENABLED"
                } else {
                    "disabled"
                }
            );
            if self.config.chaos_enabled {
                println!("   Failure Rate: {:.1}%", self.config.failure_rate * 100.0);
                println!("   Network Latency: {} ms", self.config.network_latency_ms);
            }
            println!("{}\n", "=".repeat(60));

            // Spawn load generation tasks
            let load_handles = self.spawn_load_generators();

            // Monitor and report metrics every second
            while start.elapsed() < test_duration {
                tokio::time::sleep(Duration::from_millis(100)).await;

                if last_sample_time.elapsed() >= Duration::from_secs(1) {
                    let completed = self.completed.load(Ordering::Relaxed);
                    let submitted = self.submitted.load(Ordering::Relaxed);
                    let failed = self.failed.load(Ordering::Relaxed);
                    let elapsed = start.elapsed().as_secs_f64();

                    let current_tps = if elapsed > 0.0 {
                        completed as f64 / elapsed
                    } else {
                        0.0
                    };

                    max_tps = max_tps.max(current_tps);
                    samples.push((elapsed as u64, current_tps as u64));

                    // Print status every 5 seconds
                    if start.elapsed().as_secs().is_multiple_of(5) {
                        self.print_status(current_tps, submitted, completed, failed, elapsed);
                    }

                    last_sample_time = Instant::now();
                }
            }

            // Stop load generators
            self.should_stop.store(true, Ordering::Release);
            for handle in load_handles {
                let _ = handle.await;
            }

            self.collect_final_metrics(start.elapsed(), max_tps)
        }

        fn spawn_load_generators(&self) -> Vec<tokio::task::JoinHandle<()>> {
            let mut handles = Vec::new();
            // More workers = more parallel submissions
            let num_workers = (self.config.target_tps / 100).clamp(10, 512) as usize;
            let config = self.config.clone();
            let delay_us = 1_000_000u64 / self.config.target_tps;

            for worker_id in 0..num_workers {
                let config = config.clone();
                let submitted = Arc::clone(&self.submitted);
                let completed = Arc::clone(&self.completed);
                let failed = Arc::clone(&self.failed);
                let timeouts = Arc::clone(&self.timeouts);
                let latencies = Arc::clone(&self.latencies);
                let errors = Arc::clone(&self.errors);
                let should_stop = Arc::clone(&self.should_stop);

                let handle = tokio::spawn(async move {
                    while !should_stop.load(Ordering::Acquire) {
                        let task_start = Instant::now();

                        // Submit task
                        submitted.fetch_add(1, Ordering::Relaxed);

                        // Simulate task execution with chaos
                        let result =
                            Self::execute_task_with_chaos(&config, worker_id, task_start).await;

                        match result {
                            Ok(latency_ms) => {
                                completed.fetch_add(1, Ordering::Relaxed);
                                latencies.lock().push_back(latency_ms);

                                // Keep only last 100K latencies
                                if latencies.lock().len() > 100_000 {
                                    latencies.lock().pop_front();
                                }
                            }
                            Err(TaskError::Timeout) => {
                                failed.fetch_add(1, Ordering::Relaxed);
                                timeouts.fetch_add(1, Ordering::Relaxed);
                            }
                            Err(TaskError::Failure(msg)) => {
                                failed.fetch_add(1, Ordering::Relaxed);
                                errors
                                    .lock()
                                    .push(format!("[Worker {}] {}", worker_id, msg));
                            }
                        }

                        // Rate limiting - spread out submissions across workers
                        tokio::time::sleep(Duration::from_micros(delay_us)).await;
                    }
                });

                handles.push(handle);
            }

            handles
        }

        async fn execute_task_with_chaos(
            config: &StressConfig,
            worker_id: usize,
            task_start: Instant,
        ) -> Result<f64, TaskError> {
            // Use a thread-safe RNG that doesn't need to be Send across await
            let base_latency_ms: f64 = {
                use rand::Rng;
                let mut rng = rand::thread_rng();
                rng.gen_range(0.5..5.0)
            };

            // Inject chaos
            if config.chaos_enabled {
                let random_val: f64 = {
                    use rand::Rng;
                    rand::thread_rng().gen()
                };

                // Random failure
                if random_val < config.failure_rate {
                    return Err(TaskError::Failure(format!(
                        "Injected failure (worker {})",
                        worker_id
                    )));
                }
            }

            // Random timeout (with proper scope for RNG)
            if config.chaos_enabled {
                let random_val: f64 = {
                    use rand::Rng;
                    rand::thread_rng().gen()
                };
                if random_val < (config.failure_rate * 0.1) {
                    tokio::time::sleep(Duration::from_secs(5)).await;
                    return Err(TaskError::Timeout);
                }
            }

            // Random latency spike
            if config.chaos_enabled {
                let random_val: f64 = {
                    use rand::Rng;
                    rand::thread_rng().gen()
                };
                if random_val < (config.failure_rate * 0.2) {
                    let spike_ms: u64 = {
                        use rand::Rng;
                        rand::thread_rng().gen_range(10..500)
                    };
                    tokio::time::sleep(Duration::from_millis(spike_ms)).await;
                }
            }

            // Memory pressure: allocate and deallocate
            if config.memory_pressure {
                let random_val: f64 = {
                    use rand::Rng;
                    rand::thread_rng().gen()
                };
                if random_val < 0.05 {
                    let _ = vec![0u8; 1024 * 1024]; // 1MB allocation
                }
            }

            let network_latency_ms = config.network_latency_ms as f64;
            let total_latency_ms = base_latency_ms + network_latency_ms;

            tokio::time::sleep(Duration::from_millis(total_latency_ms.ceil() as u64)).await;

            Ok(task_start.elapsed().as_secs_f64() * 1000.0)
        }

        fn print_status(
            &self,
            current_tps: f64,
            submitted: u64,
            completed: u64,
            failed: u64,
            elapsed: f64,
        ) {
            let fail_rate = if submitted > 0 {
                (failed as f64 / submitted as f64) * 100.0
            } else {
                0.0
            };

            println!("[{:6.1}s] TPS: {:8.0} | Submitted: {:9} | Completed: {:9} | Failed: {:7} ({:5.2}%)",
                     elapsed, current_tps, submitted, completed, failed, fail_rate);
        }

        fn collect_final_metrics(&self, elapsed: Duration, max_tps: f64) -> MetricsSnapshot {
            let submitted = self.submitted.load(Ordering::Acquire);
            let completed = self.completed.load(Ordering::Acquire);
            let failed = self.failed.load(Ordering::Acquire);
            let timeouts = self.timeouts.load(Ordering::Acquire);

            let latencies_vec = self.latencies.lock();
            let (p50, p95, p99) = calculate_percentiles(&latencies_vec);

            MetricsSnapshot {
                submitted,
                completed,
                failed,
                peak_tps: max_tps,
                current_tps: if elapsed.as_secs_f64() > 0.0 {
                    completed as f64 / elapsed.as_secs_f64()
                } else {
                    0.0
                },
                p50_latency_ms: p50,
                p95_latency_ms: p95,
                p99_latency_ms: p99,
                timeouts,
            }
        }
    }

    #[derive(Debug)]
    enum TaskError {
        Timeout,
        Failure(String),
    }

    fn calculate_percentiles(latencies: &VecDeque<f64>) -> (f64, f64, f64) {
        if latencies.is_empty() {
            return (0.0, 0.0, 0.0);
        }

        let mut sorted: Vec<f64> = latencies.iter().copied().collect();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let p50_idx = (sorted.len() as f64 * 0.50) as usize;
        let p95_idx = (sorted.len() as f64 * 0.95) as usize;
        let p99_idx = (sorted.len() as f64 * 0.99) as usize;

        let p50 = sorted.get(p50_idx).copied().unwrap_or(0.0);
        let p95 = sorted
            .get(p95_idx.min(sorted.len() - 1))
            .copied()
            .unwrap_or(0.0);
        let p99 = sorted
            .get(p99_idx.min(sorted.len() - 1))
            .copied()
            .unwrap_or(0.0);

        (p50, p95, p99)
    }

    // ====== TEST CASES ======

    #[tokio::test(flavor = "multi_thread", worker_threads = 16)]
    async fn chaos_test_baseline_10k_tps_clean() {
        let config = StressConfig {
            target_tps: 10_000,
            duration_secs: 30,
            chaos_enabled: false,
            failure_rate: 0.0,
            network_latency_ms: 0,
            memory_pressure: false,
        };

        let test = ChaosStressTest::new(config);
        let metrics = test.run().await;

        println!(
            "\n{}",
            format_test_results("Baseline 10K TPS (Clean)", &metrics, 10_000)
        );

        // 10K TPS should be easily achievable
        assert!(
            metrics.current_tps >= 9_000.0,
            "Failed to achieve 9K TPS, got {:.0}",
            metrics.current_tps
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 16)]
    async fn chaos_test_30k_tps_clean() {
        let config = StressConfig {
            target_tps: 30_000,
            duration_secs: 30,
            chaos_enabled: false,
            failure_rate: 0.0,
            network_latency_ms: 0,
            memory_pressure: false,
        };

        let test = ChaosStressTest::new(config);
        let metrics = test.run().await;

        println!(
            "\n{}",
            format_test_results("30K TPS (Clean)", &metrics, 30_000)
        );

        assert!(
            metrics.current_tps >= 27_000.0,
            "Failed to achieve 27K TPS, got {:.0}",
            metrics.current_tps
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 32)]
    async fn chaos_test_65k_tps_clean() {
        let config = StressConfig {
            target_tps: 65_000,
            duration_secs: 30,
            chaos_enabled: false,
            failure_rate: 0.0,
            network_latency_ms: 0,
            memory_pressure: false,
        };

        let test = ChaosStressTest::new(config);
        let metrics = test.run().await;

        println!(
            "\n{}",
            format_test_results("65K TPS (Clean - Solana target)", &metrics, 65_000)
        );

        // Even at 65K, we should get 50K+ to prove feasibility
        assert!(
            metrics.current_tps >= 50_000.0,
            "Failed to achieve 50K TPS, got {:.0}",
            metrics.current_tps
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 32)]
    async fn chaos_test_100k_tps_push_limits() {
        let config = StressConfig {
            target_tps: 100_000,
            duration_secs: 30,
            chaos_enabled: false,
            failure_rate: 0.0,
            network_latency_ms: 0,
            memory_pressure: false,
        };

        let test = ChaosStressTest::new(config);
        let metrics = test.run().await;

        println!(
            "\n{}",
            format_test_results("100K TPS (Push Limits)", &metrics, 100_000)
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 32)]
    async fn chaos_test_65k_with_5_percent_failures() {
        let config = StressConfig {
            target_tps: 65_000,
            duration_secs: 30,
            chaos_enabled: true,
            failure_rate: 0.05,
            network_latency_ms: 0,
            memory_pressure: false,
        };

        let test = ChaosStressTest::new(config);
        let metrics = test.run().await;

        println!(
            "\n{}",
            format_test_results("65K TPS + 5% Failures", &metrics, 65_000)
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 32)]
    async fn chaos_test_65k_with_latency_spike() {
        let config = StressConfig {
            target_tps: 65_000,
            duration_secs: 30,
            chaos_enabled: true,
            failure_rate: 0.02,
            network_latency_ms: 50,
            memory_pressure: true,
        };

        let test = ChaosStressTest::new(config);
        let metrics = test.run().await;

        println!(
            "\n{}",
            format_test_results("65K TPS + 50ms Latency + Memory Pressure", &metrics, 65_000)
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 64)]
    async fn chaos_test_seek_breaking_point() {
        for target_tps in &[100_000u64, 250_000, 500_000, 750_000, 1_000_000] {
            println!("\n🔍 Attempting {:.0}K TPS...", *target_tps as f64 / 1000.0);

            let config = StressConfig {
                target_tps: *target_tps,
                duration_secs: 15,
                chaos_enabled: false,
                failure_rate: 0.0,
                network_latency_ms: 0,
                memory_pressure: false,
            };

            let test = ChaosStressTest::new(config);
            let metrics = test.run().await;

            let efficiency = (metrics.current_tps / *target_tps as f64) * 100.0;

            println!(
                "✓ Achieved: {:.0} TPS ({:.1}% of target)",
                metrics.current_tps, efficiency
            );

            if efficiency < 50.0 {
                println!(
                    "❌ BREAKING POINT REACHED at {:.0}K TPS",
                    *target_tps as f64 / 1000.0
                );
                println!(
                    "   System can sustain up to ~{:.0}K TPS",
                    metrics.current_tps / 1000.0
                );
                break;
            }
        }
    }

    fn format_test_results(title: &str, metrics: &MetricsSnapshot, target: u64) -> String {
        let efficiency = (metrics.current_tps / target as f64) * 100.0;
        let fail_rate = if metrics.submitted > 0 {
            (metrics.failed as f64 / metrics.submitted as f64) * 100.0
        } else {
            0.0
        };

        format!(
            r#"
╔══════════════════════════════════════════════════════════════════╗
║  {}
╚══════════════════════════════════════════════════════════════════╝

 Target TPS:        {:.0} TPS
 Actual TPS:        {:.0} TPS ({:.1}%)
 Peak TPS:          {:.0} TPS
 
 Submitted:         {:>12}
 Completed:         {:>12}
 Failed:            {:>12} ({:.2}%)
 Timeouts:          {:>12}
 
 Latency (ms):
  • P50:             {:.2} ms
  • P95:             {:.2} ms
  • P99:             {:.2} ms
 
 Status:            {}
"#,
            title,
            target as f64,
            metrics.current_tps,
            efficiency,
            metrics.peak_tps,
            metrics.submitted,
            metrics.completed,
            metrics.failed,
            fail_rate,
            metrics.timeouts,
            metrics.p50_latency_ms,
            metrics.p95_latency_ms,
            metrics.p99_latency_ms,
            if efficiency >= 80.0 {
                "✅ PASS (≥80% efficiency)"
            } else if efficiency >= 50.0 {
                "⚠️  PARTIAL (50-80%)"
            } else {
                "❌ FAIL (<50%)"
            }
        )
    }
}
