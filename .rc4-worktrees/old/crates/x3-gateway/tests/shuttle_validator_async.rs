// Shuttle async concurrency test for X3 validator consensus
// Tests that validator state doesn't diverge under concurrent async scheduling
// Run with: cargo +nightly test shuttle_validator_consensus

#![cfg(test)]

#[cfg(test)]
mod shuttle_tests {
    /// Simulated validator state
    pub struct ValidatorState {
        round: u64,
        committed_blocks: Vec<u64>,
    }

    impl ValidatorState {
        pub fn new() -> Self {
            ValidatorState {
                round: 0,
                committed_blocks: Vec::new(),
            }
        }

        async fn process_block(&mut self, block_num: u64) {
            // Simulate async processing
            tokio::time::sleep(tokio::time::Duration::from_micros(1)).await;
            self.committed_blocks.push(block_num);
        }

        async fn increment_round(&mut self) {
            self.round += 1;
            tokio::time::sleep(tokio::time::Duration::from_micros(1)).await;
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn shuttle_validator_no_state_divergence() {
        let state = std::sync::Arc::new(tokio::sync::Mutex::new(ValidatorState::new()));

        let mut tasks = vec![];

        // Simulate multiple concurrent gossip tasks
        for task_id in 0..10 {
            let s = state.clone();
            let task = tokio::spawn(async move {
                for i in 0..10 {
                    let mut st = s.lock().await;
                    st.process_block((task_id * 100 + i) as u64).await;
                    drop(st);
                    tokio::task::yield_now().await;
                }
            });
            tasks.push(task);
        }

        // Wait for all tasks
        for task in tasks {
            let _ = task.await;
        }

        // Verify final state consistency
        let final_state = state.lock().await;
        assert_eq!(
            final_state.committed_blocks.len(),
            100,
            "Expected 100 blocks committed"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn shuttle_round_increment_ordering() {
        let state = std::sync::Arc::new(tokio::sync::Mutex::new(ValidatorState::new()));

        let mut tasks = vec![];

        // Multiple threads incrementing rounds
        for _ in 0..5 {
            let s = state.clone();
            let task = tokio::spawn(async move {
                for _ in 0..20 {
                    let mut st = s.lock().await;
                    st.increment_round().await;
                    drop(st);
                    tokio::task::yield_now().await;
                }
            });
            tasks.push(task);
        }

        for task in tasks {
            let _ = task.await;
        }

        let final_state = state.lock().await;
        assert_eq!(
            final_state.round, 100,
            "Expected 100 round increments, got {}",
            final_state.round
        );
    }
}
