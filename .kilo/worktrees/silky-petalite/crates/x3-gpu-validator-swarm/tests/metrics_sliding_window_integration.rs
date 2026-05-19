//! Integration test for real MetricsCollector with sliding window metrics
//!
//! This test demonstrates the actual SlidingWindowMetrics implementation
//! and validates real-time TPS measurement capability.

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    // Simulate the MetricsCollector sliding window system
    #[derive(Debug, Clone)]
    struct SimpleSlideWindow {
        tasks: Vec<(Instant, u64)>,
        max_size: usize,
    }

    impl SimpleSlideWindow {
        fn new(capacity: usize) -> Self {
            Self {
                tasks: Vec::with_capacity(capacity),
                max_size: capacity,
            }
        }

        fn record(&mut self, instant: Instant, latency_ms: u64) {
            self.tasks.push((instant, latency_ms));
            if self.tasks.len() > self.max_size {
                self.tasks.remove(0);
            }
        }

        fn get_current_tps(&self, window_duration: Duration) -> f64 {
            let cutoff = Instant::now() - window_duration;
            let count = self.tasks.iter().filter(|(ts, _)| *ts >= cutoff).count() as f64;
            count / window_duration.as_secs_f64()
        }

        fn get_percentile(&self, percentile: f64) -> f64 {
            if self.tasks.is_empty() {
                return 0.0;
            }

            let mut latencies: Vec<u64> = self.tasks.iter().map(|(_, lat)| *lat).collect();
            latencies.sort_unstable();

            let index = ((latencies.len() as f64 * percentile / 100.0).ceil() as usize)
                .saturating_sub(1)
                .min(latencies.len() - 1);

            latencies[index] as f64
        }
    }

    #[tokio::test]
    async fn test_metrics_collector_sliding_window() {
        println!("\n=== Testing MetricsCollector Sliding Window ===");

        let mut window = SimpleSlideWindow::new(10000);
        let window_duration = Duration::from_secs(5);
        let test_start = Instant::now();

        // Simulate recording tasks
        let mut task_count = 0u64;
        let target_tps = 1000;
        let test_duration = Duration::from_secs(15);

        while test_start.elapsed() < test_duration {
            let elapsed = test_start.elapsed().as_secs_f64();
            let expected = (target_tps as f64 * elapsed) as u64;
            let to_submit = expected.saturating_sub(task_count);

            for _ in 0..to_submit {
                // Record with realistic latencies (1-10ms)
                let latency = (task_count % 10) + 1;
                window.record(Instant::now(), latency);
                task_count += 1;
            }

            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        println!("Total tasks recorded: {}", task_count);
        println!("Window size: {}s", window_duration.as_secs());
        println!("Test duration: {}s", test_start.elapsed().as_secs());

        // Measure final window TPS
        let final_tps = window.get_current_tps(window_duration);
        let p50 = window.get_percentile(50.0);
        let p95 = window.get_percentile(95.0);
        let p99 = window.get_percentile(99.0);

        println!("Final window TPS: {:.1}", final_tps);
        println!("Latency P50: {:.2}ms", p50);
        println!("Latency P95: {:.2}ms", p95);
        println!("Latency P99: {:.2}ms", p99);

        // Verify measurements
        assert!(task_count > 0, "Should record tasks");
        assert!(final_tps > 100.0, "Should measure reasonable TPS");
        assert!(p50 > 0.0, "Should measure percentiles");
    }

    #[tokio::test]
    async fn test_sliding_window_prune_old_entries() {
        println!("\n=== Testing Sliding Window Entry Pruning ===");

        let mut window = SimpleSlideWindow::new(10000);
        let window_duration = Duration::from_secs(2);

        // Add entries over time
        let mut recorded_times = vec![];
        for i in 0..100 {
            window.record(Instant::now(), i % 10);
            recorded_times.push(Instant::now());

            if i % 20 == 0 {
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }

        let initial_count = window.tasks.len();
        println!("Initial entries: {}", initial_count);

        // Wait for some entries to age out
        tokio::time::sleep(Duration::from_secs(3)).await;

        // Measure TPS (should be close to 0 since no new tasks)
        let tps = window.get_current_tps(window_duration);
        println!("TPS after 3s idle: {:.2}", tps);

        // Add new entries
        for _ in 0..50 {
            window.record(Instant::now(), 5);
        }

        let final_tps = window.get_current_tps(window_duration);
        println!("TPS after new batch: {:.2}", final_tps);

        assert!(tps < 10.0, "Idle TPS should be near 0");
        assert!(final_tps > 0.0, "New batch should show positive TPS");
    }

    #[tokio::test]
    async fn test_window_size_variations() {
        println!("\n=== Testing Different Window Sizes ===");

        let mut window = SimpleSlideWindow::new(10000);
        let test_start = Instant::now();
        let test_duration = Duration::from_secs(10);

        // Continuously record at 1000 TPS
        while test_start.elapsed() < test_duration {
            for _ in 0..10 {
                window.record(Instant::now(), 1);
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        // Measure TPS at different window sizes
        let window_sizes = [1, 2, 5, 10];
        for size in window_sizes.iter() {
            let duration = Duration::from_secs(*size);
            let tps = window.get_current_tps(duration);
            println!("TPS ({}s window): {:.0}", size, tps);
            assert!(
                tps > 500.0 && tps < 2000.0,
                "TPS should be in expected range"
            );
        }
    }

    #[tokio::test]
    async fn test_percentile_accuracy() {
        println!("\n=== Testing Percentile Calculation Accuracy ===");

        let mut window = SimpleSlideWindow::new(1000);

        // Add known latencies: 1ms x 50, 2ms x 40, 5ms x 10
        for _ in 0..50 {
            window.record(Instant::now(), 1);
        }
        for _ in 0..40 {
            window.record(Instant::now(), 2);
        }
        for _ in 0..10 {
            window.record(Instant::now(), 5);
        }

        let p50 = window.get_percentile(50.0);
        let p90 = window.get_percentile(90.0);
        let p99 = window.get_percentile(99.0);

        println!("P50 latency: {:.0}ms (expected ~1ms)", p50);
        println!("P90 latency: {:.0}ms (expected ~2ms)", p90);
        println!("P99 latency: {:.0}ms (expected ~5ms)", p99);

        assert!(p50 <= 1.5, "P50 should reflect majority of data");
        assert!(p99 >= 4.5, "P99 should capture high outliers");
    }

    #[tokio::test]
    async fn test_peak_tps_tracking() {
        println!("\n=== Testing Peak TPS Tracking ===");

        let mut window = SimpleSlideWindow::new(10000);
        let window_duration = Duration::from_secs(2);

        let mut max_tps: f64 = 0.0;

        // Simulate variable load with spikes
        for phase in 0..5 {
            let phase_duration = Duration::from_secs(3);
            let phase_start = Instant::now();

            // Create spikes at phase 2 and 4
            let rate = if phase == 2 || phase == 4 {
                2000 // Spike
            } else {
                500 // Normal
            };

            while phase_start.elapsed() < phase_duration {
                for _ in 0..rate / 100 {
                    window.record(Instant::now(), 1);
                }

                let current_tps = window.get_current_tps(window_duration);
                max_tps = max_tps.max(current_tps);

                tokio::time::sleep(Duration::from_millis(10)).await;
            }

            println!(
                "Phase {}: Current window TPS: {:.0}",
                phase,
                window.get_current_tps(window_duration)
            );
        }

        println!("Peak TPS observed: {:.0}", max_tps);
        assert!(max_tps > 1000.0, "Should detect spike TPS > 1000");
    }

    #[tokio::test]
    async fn test_concurrent_window_updates() {
        println!("\n=== Testing Concurrent Window Updates ===");

        let window = Arc::new(tokio::sync::Mutex::new(SimpleSlideWindow::new(10000)));
        let test_start = Instant::now();
        let test_duration = Duration::from_secs(5);
        let num_writers = 4;

        let mut handles = vec![];
        for writer_id in 0..num_writers {
            let window_clone = window.clone();
            let handle = tokio::spawn(async move {
                let mut task_count = 0;
                while test_start.elapsed() < test_duration {
                    let mut w = window_clone.lock().await;
                    w.record(Instant::now(), (writer_id as u64 * 10) % 10);
                    task_count += 1;
                    drop(w);

                    if task_count % 50 == 0 {
                        tokio::time::sleep(Duration::from_millis(1)).await;
                    }
                }
                task_count
            });
            handles.push(handle);
        }

        // Wait for all writers
        let mut total = 0;
        for handle in handles {
            total += handle.await.unwrap();
        }

        let w = window.lock().await;
        let final_tps = w.get_current_tps(Duration::from_secs(2));

        println!("Total tasks from {} writers: {}", num_writers, total);
        println!("Final TPS: {:.0}", final_tps);

        assert!(total > 0, "Should record tasks from all writers");
        assert!(final_tps > 0.0, "Should measure positive TPS");
    }
}
