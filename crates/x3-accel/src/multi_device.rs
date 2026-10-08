//! Run one batch across several accelerator devices at once.
//!
//! [`MultiDevice`] splits a batch into contiguous parts sized by each device's
//! measured throughput, runs the parts concurrently, and concatenates the
//! results in input order. A part whose device fails (error, or the wrong
//! number of results) is retried on the other devices in turn; the batch
//! errors only if every device fails that part, so callers' existing CPU
//! fallback (`ParityChecked`, `ShadowVerifier`, `*_with_parity`) still
//! applies. Small batches go to the single fastest device, where splitting
//! would cost more in per-dispatch latency than it saves.
//!
//! The output is a pure function of the input whatever the split: every part
//! is an independent slice and the merge is by position.

use std::ops::Range;

use crate::{AccelBackend, AccelError, Ed25519VerifyJob, Secp256k1VerifyJob};

/// Below this many items a batch is not split.
pub const DEFAULT_MIN_SPLIT: usize = 4096;

pub struct MultiDevice<D> {
    devices: Vec<D>,
    /// Relative throughput, same order as `devices`; all positive.
    weights: Vec<f64>,
    min_split: usize,
}

impl<D: AccelBackend> MultiDevice<D> {
    /// `weights` are relative speeds (e.g. measured verifies/s). Devices with a
    /// non-positive or non-finite weight are dropped.
    pub fn new(devices: Vec<D>, weights: Vec<f64>, min_split: usize) -> Result<Self, AccelError> {
        if devices.len() != weights.len() {
            return Err(AccelError::InvalidInput("one weight per device"));
        }
        let (devices, weights): (Vec<D>, Vec<f64>) = devices
            .into_iter()
            .zip(weights)
            .filter(|(_, w)| w.is_finite() && *w > 0.0)
            .unzip();
        if devices.is_empty() {
            return Err(AccelError::InvalidInput("no usable device"));
        }
        Ok(Self {
            devices,
            weights,
            min_split,
        })
    }

    pub fn device_count(&self) -> usize {
        self.devices.len()
    }

    pub fn weights(&self) -> &[f64] {
        &self.weights
    }

    /// The highest-weight device; ties go to the later index, as `max_by` would.
    fn fastest(&self) -> usize {
        let mut best = 0;
        for (index, weight) in self.weights.iter().enumerate().skip(1) {
            if weight.total_cmp(&self.weights[best]).is_ge() {
                best = index;
            }
        }
        best
    }

    /// Contiguous `(device, range)` parts covering `0..len` in order.
    pub fn plan(&self, len: usize) -> Vec<(usize, Range<usize>)> {
        if len == 0 {
            return Vec::new();
        }
        if len < self.min_split || self.devices.len() == 1 {
            return vec![(self.fastest(), 0..len)];
        }
        let total: f64 = self.weights.iter().sum();
        let mut parts = Vec::with_capacity(self.devices.len());
        let mut start = 0;
        let mut cumulative = 0.0;
        for (device, weight) in self.weights.iter().enumerate() {
            cumulative += weight;
            // The last device takes the remainder so rounding never drops items.
            let end = if device + 1 == self.devices.len() {
                len
            } else {
                ((len as f64 * cumulative / total).round() as usize).clamp(start, len)
            };
            if end > start {
                parts.push((device, start..end));
            }
            start = end;
        }
        parts
    }

    /// Run `op` over every part concurrently and merge in order.
    fn run<T, F>(&self, len: usize, op: F) -> Result<Vec<T>, AccelError>
    where
        T: Send,
        F: Fn(&D, Range<usize>) -> Result<Vec<T>, AccelError> + Sync,
    {
        let plan = self.plan(len);
        let attempt = |device: usize, range: &Range<usize>| -> Result<Vec<T>, AccelError> {
            let out = op(&self.devices[device], range.clone())?;
            if out.len() != range.len() {
                return Err(AccelError::InvalidInput(
                    "device returned the wrong number of results",
                ));
            }
            Ok(out)
        };
        let results: Vec<Result<Vec<T>, AccelError>> = std::thread::scope(|scope| {
            let handles: Vec<_> = plan
                .iter()
                .map(|(device, range)| {
                    let attempt = &attempt;
                    scope.spawn(move || attempt(*device, range))
                })
                .collect();
            handles
                .into_iter()
                .map(|handle| {
                    handle
                        .join()
                        .unwrap_or(Err(AccelError::InvalidInput("device thread panicked")))
                })
                .collect()
        });
        let mut merged = Vec::with_capacity(len);
        for ((failed_device, range), result) in plan.iter().zip(results) {
            let part = match result {
                Ok(part) => part,
                Err(first_error) => (0..self.devices.len())
                    .filter(|device| device != failed_device)
                    .find_map(|device| attempt(device, range).ok())
                    .ok_or(first_error)?,
            };
            merged.extend(part);
        }
        Ok(merged)
    }
}

impl<D: AccelBackend> AccelBackend for MultiDevice<D> {
    fn name(&self) -> &'static str {
        "multi-device"
    }

    fn verify_secp256k1_batch(
        &self,
        batch: &[Secp256k1VerifyJob],
    ) -> Result<Vec<bool>, AccelError> {
        self.run(batch.len(), |device, range| {
            device.verify_secp256k1_batch(&batch[range])
        })
    }

    fn verify_ed25519_batch(&self, batch: &[Ed25519VerifyJob]) -> Result<Vec<bool>, AccelError> {
        self.run(batch.len(), |device, range| {
            device.verify_ed25519_batch(&batch[range])
        })
    }

    fn keccak256_batch(&self, inputs: &[Vec<u8>]) -> Result<Vec<[u8; 32]>, AccelError> {
        self.run(inputs.len(), |device, range| {
            device.keccak256_batch(&inputs[range])
        })
    }

    fn sha256_batch(&self, inputs: &[Vec<u8>]) -> Result<Vec<[u8; 32]>, AccelError> {
        self.run(inputs.len(), |device, range| {
            device.sha256_batch(&inputs[range])
        })
    }

    fn blake2b256_batch(&self, inputs: &[Vec<u8>]) -> Result<Vec<[u8; 32]>, AccelError> {
        self.run(inputs.len(), |device, range| {
            device.blake2b256_batch(&inputs[range])
        })
    }

    /// A Merkle root is one value over all leaves; it cannot be split by range.
    fn build_merkle_root(&self, leaves: &[[u8; 32]]) -> Result<[u8; 32], AccelError> {
        self.devices[self.fastest()].build_merkle_root(leaves)
    }
}

#[cfg(feature = "wgpu")]
impl MultiDevice<crate::WgpuBackend> {
    /// Every hardware adapter, weighted by a measured secp256k1 verification
    /// rate. Adapters that fail to initialize or calibrate are left out.
    ///
    /// The first run on a machine also pays the one-time shader compile
    /// (~30 s per device); the driver caches it afterwards.
    pub fn calibrated_wgpu(min_split: usize) -> Result<Self, AccelError> {
        let jobs = calibration_jobs(8192)?;
        let mut devices = Vec::new();
        let mut weights = Vec::new();
        for index in 0..x3_accel_wgpu::WgpuBackend::hardware_adapters().len() {
            let Ok(device) = crate::WgpuBackend::from_adapter(index) else {
                continue;
            };
            // Warm-up compiles the pipeline and sizes the buffers.
            if device.verify_secp256k1_batch(&jobs[..64]).is_err() {
                continue;
            }
            let started = std::time::Instant::now();
            match device.verify_secp256k1_batch(&jobs) {
                Ok(verdicts) if verdicts.iter().all(|v| *v) => {
                    weights.push(jobs.len() as f64 / started.elapsed().as_secs_f64());
                    devices.push(device);
                }
                // A device that rejects known-good signatures is not used at all.
                _ => continue,
            }
        }
        if devices.is_empty() {
            return Err(AccelError::BackendUnavailable(crate::BackendKind::Wgpu));
        }
        Self::new(devices, weights, min_split)
    }
}

#[cfg(feature = "wgpu")]
fn calibration_jobs(count: usize) -> Result<Vec<Secp256k1VerifyJob>, AccelError> {
    use secp256k1::{Message, PublicKey, Secp256k1, SecretKey};
    let secp = Secp256k1::new();
    let sk = SecretKey::from_slice(&[0x5a; 32])
        .map_err(|_| AccelError::InvalidInput("calibration secret key"))?;
    let public_key = PublicKey::from_secret_key(&secp, &sk).serialize().to_vec();
    Ok((0..count as u32)
        .map(|i| {
            let mut z = [0x33u8; 32];
            z[..4].copy_from_slice(&i.to_le_bytes());
            Secp256k1VerifyJob {
                message_hash: z,
                signature: secp
                    .sign_ecdsa(&Message::from_digest(z), &sk)
                    .serialize_compact(),
                public_key: public_key.clone(),
            }
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CpuBackend;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// Test-only device: CPU answers, optionally failing or truncating, and
    /// counting the items it was given.
    struct Device {
        fail: bool,
        truncate: bool,
        seen: AtomicUsize,
    }

    impl Device {
        fn ok() -> Self {
            Self {
                fail: false,
                truncate: false,
                seen: AtomicUsize::new(0),
            }
        }
        fn failing() -> Self {
            Self {
                fail: true,
                ..Self::ok()
            }
        }
        fn truncating() -> Self {
            Self {
                truncate: true,
                ..Self::ok()
            }
        }
        fn answer<T>(&self, n: usize, mut out: Vec<T>) -> Result<Vec<T>, AccelError> {
            self.seen.fetch_add(n, Ordering::SeqCst);
            if self.fail {
                return Err(AccelError::InvalidInput("injected"));
            }
            if self.truncate {
                out.pop();
            }
            Ok(out)
        }
    }

    impl AccelBackend for Device {
        fn name(&self) -> &'static str {
            "test-device"
        }
        fn verify_secp256k1_batch(
            &self,
            b: &[Secp256k1VerifyJob],
        ) -> Result<Vec<bool>, AccelError> {
            self.answer(b.len(), CpuBackend::new().verify_secp256k1_batch(b)?)
        }
        fn verify_ed25519_batch(&self, b: &[Ed25519VerifyJob]) -> Result<Vec<bool>, AccelError> {
            self.answer(b.len(), CpuBackend::new().verify_ed25519_batch(b)?)
        }
        fn keccak256_batch(&self, i: &[Vec<u8>]) -> Result<Vec<[u8; 32]>, AccelError> {
            self.answer(i.len(), CpuBackend::new().keccak256_batch(i)?)
        }
        fn sha256_batch(&self, i: &[Vec<u8>]) -> Result<Vec<[u8; 32]>, AccelError> {
            self.answer(i.len(), CpuBackend::new().sha256_batch(i)?)
        }
        fn blake2b256_batch(&self, i: &[Vec<u8>]) -> Result<Vec<[u8; 32]>, AccelError> {
            self.answer(i.len(), CpuBackend::new().blake2b256_batch(i)?)
        }
        fn build_merkle_root(&self, l: &[[u8; 32]]) -> Result<[u8; 32], AccelError> {
            CpuBackend::new().build_merkle_root(l)
        }
    }

    fn inputs(n: usize) -> Vec<Vec<u8>> {
        (0..n).map(|i| (i as u32).to_le_bytes().to_vec()).collect()
    }

    #[test]
    fn plan_is_proportional_contiguous_and_complete() {
        let multi =
            MultiDevice::new(vec![Device::ok(), Device::ok()], vec![105.0, 75.0], 100).unwrap();
        let plan = multi.plan(10_000);
        assert_eq!(plan, vec![(0, 0..5833), (1, 5833..10_000)]);
        for len in [100, 101, 999, 4097, 65_536] {
            let plan = multi.plan(len);
            assert_eq!(plan.first().unwrap().1.start, 0);
            assert_eq!(plan.last().unwrap().1.end, len);
            assert!(plan.windows(2).all(|w| w[0].1.end == w[1].1.start));
        }
    }

    #[test]
    fn small_batches_go_to_the_fastest_device_only() {
        let multi =
            MultiDevice::new(vec![Device::ok(), Device::ok()], vec![1.0, 3.0], 4096).unwrap();
        assert_eq!(multi.plan(4095), vec![(1, 0..4095)]);
        assert!(multi.plan(0).is_empty());
    }

    #[test]
    fn split_results_are_identical_and_in_order() {
        let multi = MultiDevice::new(
            vec![Device::ok(), Device::ok(), Device::ok()],
            vec![1.0, 2.0, 3.0],
            10,
        )
        .unwrap();
        let data = inputs(3001);
        assert_eq!(
            multi.keccak256_batch(&data).unwrap(),
            CpuBackend::new().keccak256_batch(&data).unwrap()
        );
        let seen: Vec<_> = multi
            .devices
            .iter()
            .map(|d| d.seen.load(Ordering::SeqCst))
            .collect();
        assert_eq!(seen.iter().sum::<usize>(), 3001);
        assert!(
            seen.iter().all(|s| *s > 0),
            "every device got work: {seen:?}"
        );
    }

    #[test]
    fn a_failing_or_truncating_device_is_covered_by_the_others() {
        let data = inputs(2000);
        let expected = CpuBackend::new().sha256_batch(&data).unwrap();
        for bad in [Device::failing(), Device::truncating()] {
            let multi = MultiDevice::new(vec![Device::ok(), bad], vec![1.0, 1.0], 10).unwrap();
            assert_eq!(multi.sha256_batch(&data).unwrap(), expected);
        }
    }

    #[test]
    fn all_devices_failing_is_an_error_for_the_caller_to_fall_back_on() {
        let multi = MultiDevice::new(
            vec![Device::failing(), Device::failing()],
            vec![1.0, 1.0],
            10,
        )
        .unwrap();
        assert!(multi.sha256_batch(&inputs(100)).is_err());
    }

    #[test]
    fn unusable_weights_are_rejected() {
        assert!(MultiDevice::new(vec![Device::ok()], vec![0.0], 1).is_err());
        assert!(MultiDevice::new(vec![Device::ok(), Device::ok()], vec![1.0], 1).is_err());
        let multi =
            MultiDevice::new(vec![Device::ok(), Device::ok()], vec![f64::NAN, 2.0], 1).unwrap();
        assert_eq!(multi.device_count(), 1);
    }
}
