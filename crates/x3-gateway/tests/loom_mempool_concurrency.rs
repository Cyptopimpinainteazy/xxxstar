// Loom concurrency tests for X3 mempool ordering
// Tests that concurrent mempool operations maintain FIFO ordering
// Run with: cargo +nightly test -p x3-gateway --test loom_mempool_concurrency --features loom-tests

#![cfg_attr(feature = "loom-tests", allow(dead_code))]

#[cfg(feature = "loom-tests")]
mod loom_mempool {
    use std::sync::Mutex;

    /// Simulated mempool entry for testing
    #[derive(Debug, Clone, Eq, PartialEq)]
    pub struct TxEntry {
        pub nonce: u64,
        pub sender: u32,
    }

    /// Simple FIFO mempool with ordering guarantee
    pub struct FifoMempool {
        queue: Mutex<Vec<TxEntry>>,
    }

    impl FifoMempool {
        pub fn new() -> Self {
            FifoMempool {
                queue: Mutex::new(Vec::new()),
            }
        }

        pub fn enqueue(&self, entry: TxEntry) {
            let mut q = self.queue.lock().unwrap();
            q.push(entry);
        }

        pub fn dequeue(&self) -> Option<TxEntry> {
            let mut q = self.queue.lock().unwrap();
            if q.is_empty() {
                None
            } else {
                Some(q.remove(0))
            }
        }

        pub fn len(&self) -> usize {
            self.queue.lock().unwrap().len()
        }
    }

    #[test]
    fn loom_fifo_ordering() {
        loom::model(|| {
            let mempool = std::sync::Arc::new(FifoMempool::new());

            let m1 = mempool.clone();
            let h1 = loom::thread::spawn(move || {
                m1.enqueue(TxEntry {
                    nonce: 1,
                    sender: 1,
                });
                m1.enqueue(TxEntry {
                    nonce: 2,
                    sender: 1,
                });
            });

            let m2 = mempool.clone();
            let h2 = loom::thread::spawn(move || {
                m2.enqueue(TxEntry {
                    nonce: 1,
                    sender: 2,
                });
                m2.enqueue(TxEntry {
                    nonce: 2,
                    sender: 2,
                });
            });

            h1.join().unwrap();
            h2.join().unwrap();

            // Dequeue all and verify we got 4 entries
            let mut results = Vec::new();
            while let Some(entry) = mempool.dequeue() {
                results.push(entry);
            }

            assert_eq!(results.len(), 4, "Expected 4 entries");

            // Verify sender 1 maintains nonce order
            let sender1: Vec<_> = results.iter().filter(|e| e.sender == 1).collect();
            for i in 1..sender1.len() {
                assert!(
                    sender1[i - 1].nonce <= sender1[i].nonce,
                    "Sender 1 nonce order broken"
                );
            }

            // Verify sender 2 maintains nonce order
            let sender2: Vec<_> = results.iter().filter(|e| e.sender == 2).collect();
            for i in 1..sender2.len() {
                assert!(
                    sender2[i - 1].nonce <= sender2[i].nonce,
                    "Sender 2 nonce order broken"
                );
            }
        });
    }

    /// Test: Multiple enqueuers, single dequeuer
    #[test]
    fn loom_single_dequeuer_coherency() {
        loom::model(|| {
            let mempool = std::sync::Arc::new(FifoMempool::new());
            let mut handles = vec![];

            // 3 threads enqueuing
            for sender in 0..3 {
                let m = mempool.clone();
                let h = loom::thread::spawn(move || {
                    for nonce in 0..2 {
                        m.enqueue(TxEntry {
                            nonce: nonce as u64,
                            sender: sender as u32,
                        });
                    }
                });
                handles.push(h);
            }

            // 1 thread dequeuing
            let m = mempool.clone();
            let deq_handle = loom::thread::spawn(move || {
                let mut dequeued = Vec::new();
                // Bounded dequeue loop for deterministic Loom exploration.
                for _ in 0..6 {
                    if let Some(entry) = m.dequeue() {
                        dequeued.push(entry);
                    }
                    loom::thread::yield_now();
                }
                dequeued
            });

            for h in handles {
                h.join().unwrap();
            }

            let dequeued = deq_handle.join().unwrap();

            // Dequeuer may miss some due to scheduling, but it cannot exceed total enqueued.
            assert!(dequeued.len() <= 6, "Dequeued more than enqueued");

            // Remaining queue size + dequeued size must equal total produced.
            let remaining = mempool.len();
            assert_eq!(
                dequeued.len() + remaining,
                6,
                "Queue accounting mismatch under concurrency"
            );

            // No panics = success
        });
    }

    /// Nonce cache: concurrent updates must not race
    pub struct NonceCache {
        nonces: Mutex<std::collections::HashMap<u32, u64>>,
    }

    impl NonceCache {
        pub fn new() -> Self {
            NonceCache {
                nonces: Mutex::new(std::collections::HashMap::new()),
            }
        }

        pub fn get(&self, sender: u32) -> u64 {
            self.nonces
                .lock()
                .unwrap()
                .get(&sender)
                .copied()
                .unwrap_or(0)
        }

        pub fn set(&self, sender: u32, nonce: u64) {
            self.nonces.lock().unwrap().insert(sender, nonce);
        }

        pub fn increment(&self, sender: u32) -> u64 {
            let mut nonces = self.nonces.lock().unwrap();
            let current = nonces.get(&sender).copied().unwrap_or(0);
            let next = current + 1;
            nonces.insert(sender, next);
            next
        }
    }

    #[test]
    fn loom_nonce_cache_no_lost_increments() {
        loom::model(|| {
            let cache = std::sync::Arc::new(NonceCache::new());

            let c1 = cache.clone();
            let h1 = loom::thread::spawn(move || {
                c1.increment(1);
                c1.increment(1);
            });

            let c2 = cache.clone();
            let h2 = loom::thread::spawn(move || {
                c2.increment(1);
                c2.increment(1);
            });

            h1.join().unwrap();
            h2.join().unwrap();

            // Final nonce should be 4 (no lost increments)
            let final_nonce = cache.get(1);
            assert_eq!(
                final_nonce, 4,
                "Lost increments! Expected 4, got {}",
                final_nonce
            );
        });
    }

    /// Reservation lock: ensuring no overlapping reservations for same account
    pub struct ReservationLock {
        reservations: Mutex<std::collections::HashSet<u32>>,
    }

    impl ReservationLock {
        pub fn new() -> Self {
            ReservationLock {
                reservations: Mutex::new(std::collections::HashSet::new()),
            }
        }

        pub fn try_reserve(&self, account: u32) -> bool {
            let mut res = self.reservations.lock().unwrap();
            if res.contains(&account) {
                false
            } else {
                res.insert(account);
                true
            }
        }

        pub fn release(&self, account: u32) {
            self.reservations.lock().unwrap().remove(&account);
        }
    }

    #[test]
    fn loom_no_overlapping_reservations() {
        loom::model(|| {
            let lock = std::sync::Arc::new(ReservationLock::new());

            let l1 = lock.clone();
            let h1 = loom::thread::spawn(move || {
                if l1.try_reserve(1) {
                    loom::thread::yield_now(); // Give other thread a chance
                    l1.release(1);
                    true
                } else {
                    false
                }
            });

            let l2 = lock.clone();
            let h2 = loom::thread::spawn(move || {
                if l2.try_reserve(1) {
                    loom::thread::yield_now();
                    l2.release(1);
                    true
                } else {
                    false
                }
            });

            let r1 = h1.join().unwrap();
            let r2 = h2.join().unwrap();

            // At least one succeeds.
            assert!(r1 || r2, "At least one thread should get reservation");

            // After both threads finish, the reservation must be released.
            assert!(
                lock.try_reserve(1),
                "Reservation leaked after concurrent release"
            );
        });
    }
}
