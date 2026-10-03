//! Sampled shadow verification: let an accelerator's signature verdicts be
//! used without recomputing every one on the CPU.
//!
//! [`crate::ParityChecked`] recomputes everything on the CPU, which is right
//! for consensus but means an accelerator can never save CPU time. This
//! wrapper trades a bounded, *detectable* risk for throughput:
//!
//! * **Rejections are always re-checked.** Every signature the accelerator
//!   calls invalid is verified again on the CPU, so an accelerator bug can
//!   never make a valid transaction be refused.
//! * **Acceptances are sampled.** Each accepted signature is re-verified on the
//!   CPU with probability `sample_per_million / 1e6`. Selection is keyed with a
//!   per-process random key (`RandomState`), so a submitter cannot predict
//!   which signatures escape the check.
//! * **Any disagreement disables the accelerator for good.** The batch is
//!   answered from the CPU, the evidence is kept in [`ShadowStats`], and every
//!   later batch goes straight to the CPU. There is no automatic re-enable.
//! * **Accelerator errors** answer that batch from the CPU without disabling.
//!
//! # Where this may be used
//!
//! Admission paths whose decisions are re-checked later — mempool/tx-pool
//! admission, gossip filtering, RPC pre-validation. **Not block import or
//! finality**: a deterministic accelerator bug that wrongly *accepts* a
//! signature is only caught when a sample lands on it, so a few bad
//! signatures could pass first. On a consensus path that is a fork, so those
//! paths must keep using `ParityChecked` / `CpuBackend`.

use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hash, Hasher};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Mutex;

use crate::{AccelBackend, CpuBackend, Secp256k1VerifyJob};

/// Counters and evidence. All counts are signatures, not batches.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ShadowStats {
    pub accelerated: u64,
    pub accepted_unchecked: u64,
    pub accepted_sampled: u64,
    pub rejections_rechecked: u64,
    pub cpu_only: u64,
    pub accelerator_errors: u64,
    pub disabled: bool,
    /// First disagreement seen, if any: what the accelerator and the CPU said.
    pub mismatch: Option<String>,
}

pub struct ShadowVerifier<B> {
    accelerator: B,
    sample_per_million: u32,
    key: RandomState,
    sample_nonce: AtomicU64,
    disabled: AtomicBool,
    accelerated: AtomicU64,
    accepted_unchecked: AtomicU64,
    accepted_sampled: AtomicU64,
    rejections_rechecked: AtomicU64,
    cpu_only: AtomicU64,
    accelerator_errors: AtomicU64,
    mismatch: Mutex<Option<String>>,
}

impl<B: AccelBackend> ShadowVerifier<B> {
    /// `sample_per_million` of accepted signatures are re-verified on the CPU
    /// (1_000_000 = all of them, equivalent to full parity for acceptances).
    pub fn new(accelerator: B, sample_per_million: u32) -> Self {
        Self {
            accelerator,
            sample_per_million: sample_per_million.min(1_000_000),
            key: RandomState::new(),
            sample_nonce: AtomicU64::new(0),
            disabled: AtomicBool::new(false),
            accelerated: AtomicU64::new(0),
            accepted_unchecked: AtomicU64::new(0),
            accepted_sampled: AtomicU64::new(0),
            rejections_rechecked: AtomicU64::new(0),
            cpu_only: AtomicU64::new(0),
            accelerator_errors: AtomicU64::new(0),
            mismatch: Mutex::new(None),
        }
    }

    pub fn stats(&self) -> ShadowStats {
        ShadowStats {
            accelerated: self.accelerated.load(Ordering::Relaxed),
            accepted_unchecked: self.accepted_unchecked.load(Ordering::Relaxed),
            accepted_sampled: self.accepted_sampled.load(Ordering::Relaxed),
            rejections_rechecked: self.rejections_rechecked.load(Ordering::Relaxed),
            cpu_only: self.cpu_only.load(Ordering::Relaxed),
            accelerator_errors: self.accelerator_errors.load(Ordering::Relaxed),
            disabled: self.disabled.load(Ordering::SeqCst),
            mismatch: self.mismatch.lock().map(|m| m.clone()).unwrap_or(None),
        }
    }

    fn sampled(&self, job: &Secp256k1VerifyJob) -> bool {
        if self.sample_per_million == 0 {
            return false;
        }
        let mut hasher = self.key.build_hasher();
        job.message_hash.hash(&mut hasher);
        job.signature.hash(&mut hasher);
        job.public_key.hash(&mut hasher);
        // Fresh per-call entropy prevents replaying the same unsampled acceptance forever.
        self.sample_nonce.fetch_add(1, Ordering::Relaxed).hash(&mut hasher);
        hasher.finish() % 1_000_000 < u64::from(self.sample_per_million)
    }

    fn cpu(&self, batch: &[Secp256k1VerifyJob]) -> Vec<bool> {
        self.cpu_only
            .fetch_add(batch.len() as u64, Ordering::Relaxed);
        CpuBackend::new()
            .verify_secp256k1_batch(batch)
            .expect("the CPU backend is infallible")
    }

    /// Verify a batch. Never fails: every path ends in a CPU answer or an
    /// accelerator answer that survived the checks above.
    pub fn verify_secp256k1(&self, batch: &[Secp256k1VerifyJob]) -> Vec<bool> {
        if self.disabled.load(Ordering::SeqCst) {
            return self.cpu(batch);
        }
        let verdicts = match self.accelerator.verify_secp256k1_batch(batch) {
            Ok(verdicts) if verdicts.len() == batch.len() => verdicts,
            _ => {
                self.accelerator_errors
                    .fetch_add(batch.len() as u64, Ordering::Relaxed);
                return self.cpu(batch);
            }
        };
        // Everything rejected, plus the sampled acceptances, goes back to the CPU.
        let (recheck, sampled): (Vec<usize>, Vec<bool>) = verdicts
            .iter()
            .enumerate()
            .filter_map(|(index, verdict)| {
                if !verdict {
                    Some((index, false))
                } else if self.sampled(&batch[index]) {
                    Some((index, true))
                } else {
                    None
                }
            })
            .unzip();
        let jobs: Vec<_> = recheck.iter().map(|&index| batch[index].clone()).collect();
        let truth = CpuBackend::new()
            .verify_secp256k1_batch(&jobs)
            .expect("the CPU backend is infallible");
        for (&index, &expected) in recheck.iter().zip(&truth) {
            if verdicts[index] != expected {
                self.disable(index, verdicts[index], expected);
                return self.cpu(batch);
            }
        }
        // Another concurrent batch may have disabled the accelerator while this
        // one was being checked. Never publish unsampled accelerator verdicts
        // after disablement becomes visible.
        if self.disabled.load(Ordering::SeqCst) {
            return self.cpu(batch);
        }
        let sampled_count = sampled.iter().filter(|s| **s).count() as u64;
        let accepted = verdicts.iter().filter(|v| **v).count() as u64;
        self.accelerated
            .fetch_add(batch.len() as u64, Ordering::Relaxed);
        self.accepted_sampled
            .fetch_add(sampled_count, Ordering::Relaxed);
        self.accepted_unchecked
            .fetch_add(accepted - sampled_count, Ordering::Relaxed);
        self.rejections_rechecked
            .fetch_add(recheck.len() as u64 - sampled_count, Ordering::Relaxed);
        verdicts
    }

    fn disable(&self, index: usize, accelerator: bool, cpu: bool) {
        if !self.disabled.swap(true, Ordering::SeqCst) {
            if let Ok(mut slot) = self.mismatch.lock() {
                *slot = Some(format!(
                    "backend {} said {accelerator} where the CPU says {cpu} (batch index {index}); accelerator disabled",
                    self.accelerator.name()
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AccelError, Ed25519VerifyJob};
    use secp256k1::{Message, PublicKey, Secp256k1, SecretKey};

    /// Test-only accelerator: CPU answers with chosen indices flipped, or an error.
    struct Faulty {
        flip: Vec<usize>,
        fail: bool,
    }

    impl AccelBackend for Faulty {
        fn name(&self) -> &'static str {
            "faulty-test"
        }
        fn verify_secp256k1_batch(
            &self,
            batch: &[Secp256k1VerifyJob],
        ) -> Result<Vec<bool>, AccelError> {
            if self.fail {
                return Err(AccelError::InvalidInput("injected"));
            }
            let mut out = CpuBackend::new().verify_secp256k1_batch(batch)?;
            for &i in &self.flip {
                if i < out.len() {
                    out[i] = !out[i];
                }
            }
            Ok(out)
        }
        fn verify_ed25519_batch(&self, b: &[Ed25519VerifyJob]) -> Result<Vec<bool>, AccelError> {
            CpuBackend::new().verify_ed25519_batch(b)
        }
        fn keccak256_batch(&self, i: &[Vec<u8>]) -> Result<Vec<[u8; 32]>, AccelError> {
            CpuBackend::new().keccak256_batch(i)
        }
        fn sha256_batch(&self, i: &[Vec<u8>]) -> Result<Vec<[u8; 32]>, AccelError> {
            CpuBackend::new().sha256_batch(i)
        }
        fn blake2b256_batch(&self, i: &[Vec<u8>]) -> Result<Vec<[u8; 32]>, AccelError> {
            CpuBackend::new().blake2b256_batch(i)
        }
        fn build_merkle_root(&self, l: &[[u8; 32]]) -> Result<[u8; 32], AccelError> {
            CpuBackend::new().build_merkle_root(l)
        }
    }

    /// Even indices valid, odd indices signed over a different message.
    fn batch(count: u32) -> Vec<Secp256k1VerifyJob> {
        let secp = Secp256k1::new();
        (0..count)
            .map(|i| {
                let mut seed = [7u8; 32];
                seed[..4].copy_from_slice(&i.to_le_bytes());
                let sk = SecretKey::from_slice(&seed).unwrap();
                let z = [i as u8 ^ 0x5a; 32];
                let signed = if i % 2 == 0 { z } else { [0x11; 32] };
                Secp256k1VerifyJob {
                    message_hash: z,
                    signature: secp
                        .sign_ecdsa(&Message::from_digest(signed), &sk)
                        .serialize_compact(),
                    public_key: PublicKey::from_secret_key(&secp, &sk).serialize().to_vec(),
                }
            })
            .collect()
    }

    fn truth(jobs: &[Secp256k1VerifyJob]) -> Vec<bool> {
        CpuBackend::new().verify_secp256k1_batch(jobs).unwrap()
    }

    #[test]
    fn honest_accelerator_is_used_and_rejections_rechecked() {
        let jobs = batch(40);
        let shadow = ShadowVerifier::new(
            Faulty {
                flip: vec![],
                fail: false,
            },
            0,
        );
        assert_eq!(shadow.verify_secp256k1(&jobs), truth(&jobs));
        let stats = shadow.stats();
        assert_eq!(stats.accelerated, 40);
        assert_eq!(stats.accepted_unchecked, 20);
        assert_eq!(stats.rejections_rechecked, 20);
        assert_eq!(stats.cpu_only, 0);
        assert!(!stats.disabled);
    }

    #[test]
    fn false_rejection_is_always_caught_and_disables() {
        let jobs = batch(40);
        // Index 0 is valid; the accelerator calls it invalid.
        let shadow = ShadowVerifier::new(
            Faulty {
                flip: vec![0],
                fail: false,
            },
            0,
        );
        assert_eq!(shadow.verify_secp256k1(&jobs), truth(&jobs));
        let stats = shadow.stats();
        assert!(stats.disabled);
        assert!(stats
            .mismatch
            .unwrap()
            .contains("said false where the CPU says true"));
        // Disabled for good: later batches never touch the accelerator.
        assert_eq!(shadow.verify_secp256k1(&jobs), truth(&jobs));
        assert_eq!(shadow.stats().cpu_only, 80);
    }

    #[test]
    fn false_acceptance_is_caught_when_sampled() {
        let jobs = batch(40);
        // Index 1 is invalid; the accelerator calls it valid. Sample everything.
        let shadow = ShadowVerifier::new(
            Faulty {
                flip: vec![1],
                fail: false,
            },
            1_000_000,
        );
        assert_eq!(shadow.verify_secp256k1(&jobs), truth(&jobs));
        assert!(shadow.stats().disabled);
    }

    #[test]
    fn false_acceptance_unsampled_is_the_documented_risk() {
        // With no sampling a false acceptance passes. This pins the reason the
        // module forbids consensus paths, so changing it is a deliberate act.
        let jobs = batch(40);
        let shadow = ShadowVerifier::new(
            Faulty {
                flip: vec![1],
                fail: false,
            },
            0,
        );
        let got = shadow.verify_secp256k1(&jobs);
        assert!(got[1] && !truth(&jobs)[1]);
        assert!(!shadow.stats().disabled);
    }

    #[test]
    fn sampling_rate_is_roughly_honoured() {
        let jobs: Vec<_> = batch(4000).into_iter().step_by(2).collect(); // all valid
        let shadow = ShadowVerifier::new(
            Faulty {
                flip: vec![],
                fail: false,
            },
            100_000,
        );
        shadow.verify_secp256k1(&jobs);
        let sampled = shadow.stats().accepted_sampled;
        assert!(
            (120..=290).contains(&sampled),
            "10% of 2000 sampled {sampled}"
        );
    }

    #[test]
    fn accelerator_error_falls_back_without_disabling() {
        let jobs = batch(10);
        let shadow = ShadowVerifier::new(
            Faulty {
                flip: vec![],
                fail: true,
            },
            0,
        );
        assert_eq!(shadow.verify_secp256k1(&jobs), truth(&jobs));
        let stats = shadow.stats();
        assert_eq!(
            (stats.accelerator_errors, stats.cpu_only, stats.disabled),
            (1, 10, false)
        );
    }
}
