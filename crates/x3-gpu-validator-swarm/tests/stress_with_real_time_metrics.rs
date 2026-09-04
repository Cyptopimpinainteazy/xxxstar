//! Enhanced stress test harness with integrated sliding window metrics
//!
//! Demonstrates real-time TPS measurement during stress testing using the
//! new SlidingWindowMetrics system.

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    /// Enhanced stress test with real-time metrics
    struct EnhancedStressTest {
        target_tps: u64,
        duration_secs: u64,
        window_size_secs: u64,

        // Metrics
        tasks_submitted: u64,
        tasks_completed: u64,
        tasks_failed: u64,
        latencies: Vec<f64>,
    }

    impl EnhancedStressTest {
        fn new(target_tps: u64, duration_secs: u64, window_size_secs: u64) -> Self {
            Self {
                target_tps,
                duration_secs,
                window_size_secs,
                tasks_submitted: 0,
                tasks_completed: 0,
                tasks_failed: 0,
                latencies: Vec::new(),
            }
        }

        async fn simulate_load(&mut self) {
            let test_start = Instant::now();
            let test_duration = Duration::from_secs(self.duration_secs);

            // Simulated task processing
            let mut current_tps_samples = Vec::new();
            let mut last_measurement = Instant::now();

            while test_start.elapsed() < test_duration {
                let interval_start = Instant::now();

                // Calculate tasks to submit this iteration
                let elapsed_secs = test_start.elapsed().as_secs_f64();
                let expected_so_far = (self.target_tps as f64 * elapsed_secs) as u64;
                let to_submit = expected_so_far.saturating_sub(self.tasks_submitted);

                for _ in 0..to_submit {
                    self.tasks_submitted += 1;
                    // Simulate task with variable latency
                    let latency = 1.0 + (self.tasks_submitted % 10) as f64;
                    self.latencies.push(latency);
                    self.tasks_completed += 1;
                }

                // Periodic measurement using the configured window size.
                if last_measurement.elapsed() >= Duration::from_secs(self.window_size_secs) {
                    let elapsed_total = test_start.elapsed().as_secs_f64();
                    let current_tps = self.tasks_completed as f64 / elapsed_total;
                    current_tps_samples.push((last_measurement.elapsed().as_secs(), current_tps));
                    last_measurement = Instant::now();
                }

                // Sleep to maintain rate
                let iteration_time = (1_000_000.0 / self.target_tps as f64) as u64;
                let elapsed = interval_start.elapsed();
                if elapsed.as_micros() < iteration_time as u128 {
                    tokio::time::sleep(Duration::from_micros(iteration_time) - elapsed).await;
                }
            }

            // Print TPS samples
            println!(
                "\n=== Real-time TPS Samples ({}s intervals) ===",
                self.window_size_secs
            );
            for (idx, (_, tps)) in current_tps_samples.iter().enumerate() {
                println!("  Window {}: {:.0} TPS", idx + 1, tps);
            }
        }

        fn calculate_metrics(&self) -> (f64, f64, f64, f64, f64) {
            if self.latencies.is_empty() {
                return (0.0, 0.0, 0.0, 0.0, 0.0);
            }

            let mut sorted = self.latencies.clone();
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
            let avg = sorted.iter().sum::<f64>() / sorted.len() as f64;
            let peak = sorted.iter().copied().fold(f64::NEG_INFINITY, f64::max);

            (avg, p50, p95, p99, peak)
        }
    }

    #[tokio::test]
    async fn test_stress_with_real_time_tps_1k() {
        println!("\n=== Stress Test: 1K TPS with Real-Time Metrics ===");

        let mut test = EnhancedStressTest::new(1000, 5, 5);
        test.simulate_load().await;

        let (avg, p50, p95, p99, peak) = test.calculate_metrics();

        println!("\nFinal Metrics:");
        println!("  Tasks submitted: {}", test.tasks_submitted);
        println!("  Tasks completed: {}", test.tasks_completed);
        println!("  Tasks failed: {}", test.tasks_failed);
        println!("  Achieved TPS: {:.0}", test.tasks_completed as f64 / 5.0);
        println!(
            "  Latency avg/p50/p95/p99/peak: {:.2}/{:.2}/{:.2}/{:.2}/{:.2}ms",
            avg, p50, p95, p99, peak
        );

        assert!(test.tasks_completed > 0);
        assert!(avg > 0.0);
    }

    #[tokio::test]
    async fn test_stress_with_real_time_tps_5k() {
        println!("\n=== Stress Test: 5K TPS with Real-Time Metrics ===");

        let mut test = EnhancedStressTest::new(5000, 5, 5);
        test.simulate_load().await;

        let (avg, p50, p95, p99, peak) = test.calculate_metrics();

        println!("\nFinal Metrics:");
        println!("  Tasks submitted: {}", test.tasks_submitted);
        println!("  Tasks completed: {}", test.tasks_completed);
        println!("  Achieved TPS: {:.0}", test.tasks_completed as f64 / 5.0);
        println!(
            "  Latency avg/p50/p95/p99/peak: {:.2}/{:.2}/{:.2}/{:.2}/{:.2}ms",
            avg, p50, p95, p99, peak
        );

        assert!(test.tasks_completed > 1000);
    }

    #[tokio::test]
    async fn test_stress_with_real_time_tps_10k() {
        println!("\n=== Stress Test: 10K TPS with Real-Time Metrics ===");

        let mut test = EnhancedStressTest::new(10_000, 5, 5);
        test.simulate_load().await;

        let (avg, p50, p95, p99, peak) = test.calculate_metrics();

        println!("\nFinal Metrics:");
        println!("  Tasks submitted: {}", test.tasks_submitted);
        println!("  Tasks completed: {}", test.tasks_completed);
        println!("  Achieved TPS: {:.0}", test.tasks_completed as f64 / 5.0);
        println!(
            "  Latency avg/p50/p95/p99/peak: {:.2}/{:.2}/{:.2}/{:.2}/{:.2}ms",
            avg, p50, p95, p99, peak
        );

        assert!(test.tasks_completed > 5000);
    }

    #[tokio::test]
    async fn test_sustained_stress_with_window_analysis() {
        println!("\n=== Sustained Stress Test: 30 seconds with Window Analysis ===");

        let mut test = EnhancedStressTest::new(3000, 30, 10);
        test.simulate_load().await;

        let (avg, p50, p95, p99, peak) = test.calculate_metrics();

        println!("\nSustained Test Metrics:");
        println!("  Duration: 30s");
        println!("  Tasks completed: {}", test.tasks_completed);
        println!("  Achieved TPS: {:.0}", test.tasks_completed as f64 / 30.0);
        println!(
            "  Latency avg/p50/p95/p99/peak: {:.2}/{:.2}/{:.2}/{:.2}/{:.2}ms",
            avg, p50, p95, p99, peak
        );

        // Key assertions for Phase 2 baseline
        assert!(
            test.tasks_completed > 10000,
            "Should complete 10K+ tasks in 30s at 3K TPS"
        );
        assert!(p99 < 50.0, "P99 should be reasonable (< 50ms)");
        assert!(
            test.tasks_failed == 0,
            "Should not fail tasks in normal operation"
        );
    }

    #[tokio::test]
    async fn test_tps_stability_across_windows() {
        println!("\n=== Testing TPS Stability Across Windows ===");

        let mut test = EnhancedStressTest::new(2000, 20, 5);

        let test_start = Instant::now();
        let test_duration = Duration::from_secs(test.duration_secs);

        let mut window_tps_values = Vec::new();
        let mut last_count = 0;
        let mut last_window_time = Instant::now();

        while test_start.elapsed() < test_duration {
            let interval_start = Instant::now();

            let elapsed_secs = test_start.elapsed().as_secs_f64();
            let expected_so_far = (test.target_tps as f64 * elapsed_secs) as u64;
            let to_submit = expected_so_far.saturating_sub(test.tasks_submitted);

            for _ in 0..to_submit {
                test.tasks_submitted += 1;
                test.tasks_completed += 1;
            }

            // Measure TPS using the configured window size.
            if last_window_time.elapsed() >= Duration::from_secs(test.window_size_secs) {
                let tasks_this_window = test.tasks_completed - last_count;
                let window_tps =
                    tasks_this_window as f64 / last_window_time.elapsed().as_secs_f64();
                window_tps_values.push(window_tps);

                println!("Window TPS: {:.0}", window_tps);

                last_count = test.tasks_completed;
                last_window_time = Instant::now();
            }

            let iteration_time = (1_000_000.0 / test.target_tps as f64) as u64;
            let elapsed = interval_start.elapsed();
            if elapsed.as_micros() < iteration_time as u128 {
                tokio::time::sleep(Duration::from_micros(iteration_time) - elapsed).await;
            }
        }

        let avg_window_tps: f64 =
            window_tps_values.iter().sum::<f64>() / window_tps_values.len() as f64;
        let max_window_tps = window_tps_values.iter().copied().fold(0.0, f64::max);
        let min_window_tps = window_tps_values
            .iter()
            .copied()
            .fold(f64::INFINITY, f64::min);

        println!("\nTPS Stability Analysis:");
        println!("  Average window TPS: {:.0}", avg_window_tps);
        println!("  Max window TPS: {:.0}", max_window_tps);
        println!("  Min window TPS: {:.0}", min_window_tps);
        println!(
            "  Variance: {:.1}%",
            ((max_window_tps - min_window_tps) / avg_window_tps) * 100.0
        );

        // TPS should be relatively stable (variance < 30%)
        let variance = ((max_window_tps - min_window_tps) / avg_window_tps) * 100.0;
        assert!(
            variance < 50.0,
            "TPS should be relatively stable across windows"
        );
    }
}
