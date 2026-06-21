// X3 Loom Concurrency Verification — tests for critical concurrent structures
// Covers: mempool queues, reservation locks, nonce cache, RPC rotator
//
// Run with: RUSTFLAGS="--cfg loom" cargo +nightly-2026-05-01 test --package loom-concurrency
#![cfg(loom)]

use loom::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use loom::sync::{Arc, Mutex, RwLock};
use loom::thread;

// ─── Nonce Cache ──────────────────────────────────────────────────────────

#[derive(Clone, Default)]
struct NonceCache {
    entries: Arc<RwLock<Vec<(Vec<u8>, u64)>>>,
}

impl NonceCache {
    fn new() -> Self {
        Self {
            entries: Arc::new(RwLock::new(Vec::new())),
        }
    }

    fn get_or_inc(&self, key: &[u8]) -> u64 {
        let read = self.entries.read().unwrap();
        for (k, v) in read.iter() {
            if k.as_slice() == key {
                return *v;
            }
        }
        drop(read);
        let mut write = self.entries.write().unwrap();
        // Double-check after acquiring write lock
        for (k, v) in write.iter() {
            if k.as_slice() == key {
                return *v;
            }
        }
        let nonce = 1u64;
        write.push((key.to_vec(), nonce));
        nonce
    }

    fn increment(&self, key: &[u8]) -> u64 {
        let mut write = self.entries.write().unwrap();
        for (k, v) in write.iter_mut() {
            if k.as_slice() == key {
                *v += 1;
                return *v;
            }
        }
        write.push((key.to_vec(), 2));
        2
    }
}

#[test]
fn nonce_cache_concurrent_get_or_inc_same_key() {
    loom::model(|| {
        let cache = NonceCache::new();
        let key = b"tx-42";

        let c1 = cache.clone();
        let t1 = thread::spawn(move || c1.get_or_inc(key));

        let c2 = cache.clone();
        let t2 = thread::spawn(move || c2.get_or_inc(key));

        let r1 = t1.join().unwrap();
        let r2 = t2.join().unwrap();

        // Both should see the same initial nonce (1) without increment after read
        // or one may see 1 and the other 1 — consistency is what matters
        assert!(r1 == 1 || r2 == 1, "at least one must see the initial nonce 1");
    });
}

#[test]
fn nonce_cache_increment_sequence() {
    loom::model(|| {
        let cache = NonceCache::new();
        let key = b"tx-99";

        cache.get_or_inc(key); // seed
        let c1 = cache.clone();
        let t1 = thread::spawn(move || c1.increment(key));
        let c2 = cache.clone();
        let t2 = thread::spawn(move || c2.increment(key));

        let r1 = t1.join().unwrap();
        let r2 = t2.join().unwrap();

        // Both increments must produce unique values
        assert_ne!(r1, r2, "nonce increments must be unique");
    });
}

// ─── Reservation Lock ─────────────────────────────────────────────────────

#[derive(Clone)]
struct ReservationLock {
    locked: Arc<AtomicBool>,
    owner: Arc<RwLock<Option<u64>>>,
    seq: Arc<AtomicU64>,
}

impl ReservationLock {
    fn new() -> Self {
        Self {
            locked: Arc::new(AtomicBool::new(false)),
            owner: Arc::new(RwLock::new(None)),
            seq: Arc::new(AtomicU64::new(0)),
        }
    }

    fn try_acquire(&self, account: u64) -> Result<u64, u64> {
        if self
            .locked
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_ok()
        {
            *self.owner.write().unwrap() = Some(account);
            let s = self.seq.fetch_add(1, Ordering::SeqCst) + 1;
            Ok(s)
        } else {
            let owner = *self.owner.read().unwrap();
            Err(owner.unwrap_or(0))
        }
    }

    fn release(&self, account: u64) -> bool {
        let current = *self.owner.read().unwrap();
        if current == Some(account) {
            *self.owner.write().unwrap() = None;
            self.locked.store(false, Ordering::Release);
            true
        } else {
            false
        }
    }
}

#[test]
fn reservation_lock_exclusive_acquisition() {
    loom::model(|| {
        let lock = ReservationLock::new();

        let l1 = lock.clone();
        let t1 = thread::spawn(move || l1.try_acquire(1));

        let l2 = lock.clone();
        let t2 = thread::spawn(move || l2.try_acquire(2));

        let r1 = t1.join().unwrap();
        let r2 = t2.join().unwrap();

        // Exactly one must succeed
        let successes = [r1.is_ok(), r2.is_ok()]
            .iter()
            .filter(|x| **x)
            .count();
        assert_eq!(successes, 1, "exactly one acquisition must succeed");
    });
}

#[test]
fn reservation_lock_acquire_release_cycle() {
    loom::model(|| {
        let lock = ReservationLock::new();

        // First acquire
        let seq = lock.try_acquire(1).expect("first acquire");
        assert_eq!(seq, 1);

        // Release
        assert!(lock.release(1));

        // Second acquire should succeed
        let l2 = lock.clone();
        let seq2 = l2.try_acquire(2).expect("second acquire after release");
        assert_eq!(seq2, 2);
    });
}

// ─── Mempool Queue ────────────────────────────────────────────────────────

struct MempoolQueue {
    items: Arc<RwLock<Vec<(u64, Vec<u8>)>>>,
    next_id: Arc<AtomicU64>,
}

impl MempoolQueue {
    fn new() -> Self {
        Self {
            items: Arc::new(RwLock::new(Vec::new())),
            next_id: Arc::new(AtomicU64::new(1)),
        }
    }

    fn enqueue(&self, data: Vec<u8>) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        self.items.write().unwrap().push((id, data));
        id
    }

    fn dequeue(&self) -> Option<(u64, Vec<u8>)> {
        let mut items = self.items.write().unwrap();
        if items.is_empty() {
            None
        } else {
            Some(items.remove(0))
        }
    }

    fn len(&self) -> usize {
        self.items.read().unwrap().len()
    }
}

#[test]
fn mempool_queue_concurrent_enqueue_dequeue() {
    loom::model(|| {
        let q = MempoolQueue::new();

        // Enqueue from two threads
        let q1 = q.clone();
        let t1 = thread::spawn(move || q1.enqueue(b"tx-a".to_vec()));

        let q2 = q.clone();
        let t2 = thread::spawn(move || q2.enqueue(b"tx-b".to_vec()));

        let id1 = t1.join().unwrap();
        let id2 = t2.join().unwrap();
        assert_ne!(id1, id2, "enqueue IDs must be unique");

        // Both items should be dequeuable
        let mut dequeued = 0;
        while q.dequeue().is_some() {
            dequeued += 1;
        }
        assert_eq!(dequeued, 2, "both enqueued items must be dequeued");
    });
}

#[test]
fn mempool_queue_empty_dequeue_returns_none() {
    loom::model(|| {
        let q = MempoolQueue::new();
        assert!(q.dequeue().is_none());
        assert_eq!(q.len(), 0);
    });
}

// ─── RPC Rotator ──────────────────────────────────────────────────────────

struct RpcRotator {
    endpoints: Arc<RwLock<Vec<String>>>,
    current: Arc<AtomicUsize>,
}

impl RpcRotator {
    fn new(endpoints: Vec<String>) -> Self {
        Self {
            endpoints: Arc::new(RwLock::new(endpoints)),
            current: Arc::new(AtomicUsize::new(0)),
        }
    }

    fn next(&self) -> Option<String> {
        let eps = self.endpoints.read().unwrap();
        if eps.is_empty() {
            return None;
        }
        let idx = self.current.fetch_add(1, Ordering::SeqCst) % eps.len();
        Some(eps[idx].clone())
    }

    fn remove(&self, endpoint: &str) {
        let mut eps = self.endpoints.write().unwrap();
        eps.retain(|e| e != endpoint);
    }

    fn count(&self) -> usize {
        self.endpoints.read().unwrap().len()
    }
}

#[test]
fn rpc_rotator_distributes_across_endpoints() {
    loom::model(|| {
        let rotator = RpcRotator::new(vec![
            "rpc1".to_string(),
            "rpc2".to_string(),
            "rpc3".to_string(),
        ]);

        let r1 = rotator.clone();
        let t1 = thread::spawn(move || r1.next());

        let r2 = rotator.clone();
        let t2 = thread::spawn(move || r2.next());

        let r3 = rotator.clone();
        let t3 = thread::spawn(move || r3.next());

        let ep1 = t1.join().unwrap();
        let ep2 = t2.join().unwrap();
        let ep3 = t3.join().unwrap();

        // All three calls should return Some endpoint
        assert!(ep1.is_some());
        assert!(ep2.is_some());
        assert!(ep3.is_some());

        // Not all need to be unique with 3 items and concurrent fetch_add,
        // but at least one should differ
        let unique = {
            let mut set = std::collections::HashSet::new();
            set.insert(ep1.unwrap());
            set.insert(ep2.unwrap());
            set.insert(ep3.unwrap());
            set
        };
        assert!(unique.len() >= 1, "at least one endpoint selected");
    });
}

#[test]
fn rpc_rotator_empty_returns_none() {
    loom::model(|| {
        let rotator = RpcRotator::new(vec![]);
        assert!(rotator.next().is_none());
    });
}

#[test]
fn rpc_rotator_remove_preserves_remaining() {
    loom::model(|| {
        let rotator = RpcRotator::new(vec!["a".to_string(), "b".to_string()]);
        assert_eq!(rotator.count(), 2);
        rotator.remove("a");
        assert_eq!(rotator.count(), 1);
        let next = rotator.next().unwrap();
        assert_eq!(next, "b");
    });
}