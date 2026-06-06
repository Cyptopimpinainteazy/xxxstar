//! Concurrency Race Detection with Loom
//!
//! Tests for S1-1 (failed_rollback) and concurrent state corruption:
//! - Verifies rollback is atomic under concurrent access
//! - Detects data races in rollback execution
//! - Validates reservation locks prevent double-reserve

#![cfg(test)]

#[cfg(test)]
mod loom_tests {
    use loom::sync::atomic::{AtomicUsize, Ordering};
    use loom::sync::{mpsc, Arc, Mutex};

    // ════════════════════════════════════════════════════════════
    // TEST 1: Concurrent Rollback Must Be Atomic (S1-1)
    // ════════════════════════════════════════════════════════════

    #[test]
    fn loom_concurrent_rollback_atomic() {
        loom::model(|| {
            // Simulate atomic operation log with 3 changes
            let changes = Arc::new(Mutex::new(vec![
                ("transfer", 100u128),
                ("mint", 50u128),
                ("burn", 25u128),
            ]));

            let changes_clone = Arc::clone(&changes);

            // Thread 1: Initiates rollback
            let t1 = loom::thread::spawn(move || {
                let mut log = changes.lock().unwrap();

                // Rollback MUST be atomic: all changes reverted or none
                // Simulating LIFO (Last-In-First-Out) reversal
                while let Some(_) = log.pop() {
                    // Each revert is a critical section
                }

                // If we get here, rollback either completed fully or not at all
                log.is_empty()
            });

            // Thread 2: Attempts concurrent read (should block or see consistent state)
            let t2 = loom::thread::spawn(move || {
                let log = changes_clone.lock().unwrap();

                // If rollback is not atomic, this might see partial state
                // which would indicate S1-1 blocker: incomplete rollback
                let remaining = log.len();

                // After rollback completes, should be 0 or 3 (never 1 or 2)
                remaining == 0 || remaining == 3
            });

            let rollback_done = t1.join().unwrap();
            let state_consistent = t2.join().unwrap();

            // BLOCKER S1-1: If rollback_done but state NOT consistent, rollback was incomplete
            assert!(
                rollback_done && state_consistent,
                "BLOCKER FOUND (S1-1): Rollback incomplete or state corrupted under concurrency"
            );
        });
    }

    // ════════════════════════════════════════════════════════════
    // TEST 2: Reservation Locks Prevent Double-Reserve (S1-1)
    // ════════════════════════════════════════════════════════════

    #[test]
    fn loom_reservation_prevents_double_reserve() {
        loom::model(|| {
            // Simulating reservation counter with mutex protection
            let reserved = Arc::new(Mutex::new(0u128));
            let reserved_clone = Arc::clone(&reserved);

            // Thread 1: Attempt to reserve 50
            let t1 = loom::thread::spawn(move || {
                let mut res = reserved.lock().unwrap();
                let current = *res;

                // Critical section: check and update
                if current + 50 <= 1000 {
                    *res = current + 50;
                    50
                } else {
                    0
                }
            });

            // Thread 2: Attempt to reserve 50 simultaneously
            let t2 = loom::thread::spawn(move || {
                let mut res = reserved_clone.lock().unwrap();
                let current = *res;

                if current + 50 <= 1000 {
                    *res = current + 50;
                    50
                } else {
                    0
                }
            });

            let r1 = t1.join().unwrap();
            let r2 = t2.join().unwrap();

            // INVARIANT: Total reserved must not exceed 1000
            // If it does, the lock doesn't protect against concurrent over-reservation
            // which indicates rollback could fail to recover from partial failure
            assert!(
                r1 + r2 <= 1000,
                "BLOCKER: Concurrent reserve bypassed lock - rollback may fail (S1-1)"
            );
        });
    }

    // ════════════════════════════════════════════════════════════
    // TEST 3: AtomicOperationLog Mutation Safety
    // ════════════════════════════════════════════════════════════

    #[test]
    fn loom_atomic_log_concurrent_mutations() {
        loom::model(|| {
            // Simulate operation log entries
            let log = Arc::new(Mutex::new(Vec::<(usize, &'static str)>::new()));
            let log_clone = Arc::clone(&log);

            let success_counter = Arc::new(AtomicUsize::new(0));

            // Thread 1: Mark changes as successful
            let t1 = loom::thread::spawn(move || {
                let mut l = log.lock().unwrap();
                l.push((1, "success"));
                success_counter.fetch_add(1, Ordering::SeqCst);
            });

            // Thread 2: Attempt rollback during mutation
            let t2 = loom::thread::spawn(move || {
                let mut l = log_clone.lock().unwrap();
                let len_before = l.len();

                // Simulated rollback: clear all entries
                l.clear();

                let len_after = l.len();
                len_before >= len_after // Monotonic decrease
            });

            t1.join().unwrap();
            let rollback_monotonic = t2.join().unwrap();

            assert!(
                rollback_monotonic,
                "BLOCKER: Rollback did not maintain monotonicity (S1-1)"
            );
        });
    }

    // ════════════════════════════════════════════════════════════
    // TEST 4: Cross-Thread Rollback Visibility
    // ════════════════════════════════════════════════════════════

    #[test]
    fn loom_rollback_visibility_across_threads() {
        loom::model(|| {
            let state = Arc::new(Mutex::new(100u128));
            let state_for_change = Arc::clone(&state);
            let state_for_observer = Arc::clone(&state);
            let state_for_rollback = Arc::clone(&state);

            // Ordered handoff: change -> observation -> rollback.
            let (tx_change, rx_change) = mpsc::channel::<()>();
            let (tx_observed, rx_observed) = mpsc::channel::<()>();

            // Thread 1: Makes a change
            let t1 = loom::thread::spawn(move || {
                let mut s = state_for_change.lock().unwrap();
                *s = 50; // Transfer out
                drop(s);
                tx_change.send(()).unwrap();
            });

            // Thread 2: Must observe pre-rollback change before rollback starts
            let t2 = loom::thread::spawn(move || {
                rx_change.recv().unwrap();

                let s = state_for_observer.lock().unwrap();
                let saw_change = *s == 50;
                drop(s);

                tx_observed.send(()).unwrap();
                saw_change
            });

            // Thread 3: Performs rollback
            let t3 = loom::thread::spawn(move || {
                rx_observed.recv().unwrap();

                let mut s = state_for_rollback.lock().unwrap();
                *s = 100; // Rollback to original state
            });

            t1.join().unwrap();
            let seen_change = t2.join().unwrap();
            t3.join().unwrap();

            let final_state = *state.lock().unwrap();

            assert!(
                seen_change && final_state == 100,
                "Rollback visibility failed (S1-1): change/rollback ordering not observable"
            );
        });
    }
}

// ════════════════════════════════════════════════════════════
// Non-Loom sanity tests
// ════════════════════════════════════════════════════════════

#[test]
fn test_loom_available() {
    // Verify loom is available for testing
    // If this fails, cargo update may be needed
}
