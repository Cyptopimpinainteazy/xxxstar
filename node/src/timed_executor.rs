//! Attribution for what a node does with a block's time.
//!
//! The storage audit's §3 question — *which stage is the bottleneck?* — could not
//! be answered at all. The node exported one number for a whole import
//! (`substrate_block_verification_and_import_time`) and nothing inside it, so
//! runtime execution and the client's state-root/commit work were the same
//! measurement. Every throughput number in the repository was therefore
//! unattributable: "the chain does 73 TPS" could not be traced to execution, to
//! the database, or to the load generator.
//!
//! This module wraps the code executor, which is the only place on the client
//! side where a *runtime call* is a discrete event. That gives:
//!
//! * `x3_runtime_call_seconds{method}` — wall time of each runtime call. The
//!   method labels are the runtime API method names the client asks for, so
//!   `Core_execute_block` (once per imported block), `Core_initialize_block` and
//!   `BlockBuilder_apply_extrinsic` (per authored block) separate authoring from
//!   import, and `TaggedTransactionQueue_validate_transaction` shows what the
//!   pool is spending on validation;
//! * `x3_runtime_calls_total{method}` — how many calls of each kind;
//! * `x3_runtime_call_errors_total{method}` — calls that returned an error, so a
//!   fast number cannot hide a failing one;
//! * `x3_runtime_version_seconds` — `RuntimeVersionOf::runtime_version`, which is
//!   called before many runtime calls and is a known hidden cost when the version
//!   has to be read out of the wasm blob rather than from an embedded section.
//!
//! # What this deliberately does not claim
//!
//! It does **not** measure the state root computation or the database commit.
//! Both happen in the client's backend *after* the executor returns, and there is
//! no hook for them short of patching `sc-client-db`. The honest reading is
//! therefore a split into three parts:
//!
//! ```text
//! substrate_block_verification_and_import_time   (SDK, whole import)
//!   = x3_runtime_call_seconds{method="Core_execute_block"}   (this module)
//!   + verification + state root + commit + notification     (unattributed)
//! ```
//!
//! A large `Core_execute_block` share means execution; a small one means the time
//! is on the client side, and the next step is a backend patch rather than a
//! runtime one. Storage read/write counts are also absent: they live in the state
//! machine behind the host functions, which is a separate wrapper.

use std::sync::Arc;
use std::time::Instant;

use sc_executor::{RuntimeVersion, RuntimeVersionOf};
use sp_core::traits::{CallContext, CodeExecutor, Externalities, ReadRuntimeVersion, RuntimeCode};
use substrate_prometheus_endpoint::prometheus::{
    Error as PrometheusError, Histogram, HistogramOpts, HistogramVec, IntCounterVec, Opts,
};
use substrate_prometheus_endpoint::Registry;

/// Runtime call timings, registered on the node's Prometheus registry.
pub struct RuntimeCallMetrics {
    call_seconds: HistogramVec,
    calls_total: IntCounterVec,
    call_errors_total: IntCounterVec,
    runtime_version_seconds: Histogram,
}

impl RuntimeCallMetrics {
    /// Register the metrics. Fails only on a malformed registration, which is a
    /// programming error rather than a runtime condition.
    pub fn register(registry: &Registry) -> Result<Self, PrometheusError> {
        // Buckets span the questions that matter: a transaction validation or a
        // `Core_version` call is sub-millisecond, one block's execution on this
        // chain is tens of milliseconds, and a block that takes seconds is the
        // pathological case worth seeing in its own bucket.
        let buckets = vec![
            0.0001, 0.0005, 0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 5.0,
        ];

        let call_seconds = HistogramVec::new(
            HistogramOpts::new(
                "x3_runtime_call_seconds",
                "Wall-clock seconds spent inside each runtime call, by method",
            )
            .buckets(buckets.clone()),
            &["method"],
        )?;
        let calls_total = IntCounterVec::new(
            Opts::new(
                "x3_runtime_calls_total",
                "Runtime calls made by this node, by method",
            ),
            &["method"],
        )?;
        let call_errors_total = IntCounterVec::new(
            Opts::new(
                "x3_runtime_call_errors_total",
                "Runtime calls that returned an error, by method",
            ),
            &["method"],
        )?;
        let runtime_version_seconds = Histogram::with_opts(
            HistogramOpts::new(
                "x3_runtime_version_seconds",
                "Wall-clock seconds spent reading the runtime version",
            )
            .buckets(buckets),
        )?;

        registry.register(Box::new(call_seconds.clone()))?;
        registry.register(Box::new(calls_total.clone()))?;
        registry.register(Box::new(call_errors_total.clone()))?;
        registry.register(Box::new(runtime_version_seconds.clone()))?;

        Ok(Self {
            call_seconds,
            calls_total,
            call_errors_total,
            runtime_version_seconds,
        })
    }

    fn observe(&self, method: &str, seconds: f64, failed: bool) {
        self.call_seconds.with_label_values(&[method]).observe(seconds);
        self.calls_total.with_label_values(&[method]).inc();
        if failed {
            self.call_errors_total.with_label_values(&[method]).inc();
        }
    }
}

/// A code executor that reports what it was asked to run and how long it took.
///
/// Every method delegates to the wrapped executor unchanged: the result, the
/// error type and the "did native execution happen" flag are the inner
/// executor's. Timing cannot change an outcome here, which is why it is
/// acceptable to have it in the block-import path at all.
pub struct TimedExecutor<E> {
    inner: E,
    metrics: Option<Arc<RuntimeCallMetrics>>,
}

impl<E> TimedExecutor<E> {
    /// Wrap `inner`, recording into `metrics` when the node has a registry.
    ///
    /// A node started without Prometheus (`--no-prometheus`) gets `None` and the
    /// wrapper becomes a pure pass-through rather than a source of contention.
    pub fn new(inner: E, metrics: Option<Arc<RuntimeCallMetrics>>) -> Self {
        Self { inner, metrics }
    }
}

impl<E: Clone> Clone for TimedExecutor<E> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            metrics: self.metrics.clone(),
        }
    }
}

impl<E: ReadRuntimeVersion> ReadRuntimeVersion for TimedExecutor<E> {
    fn read_runtime_version(
        &self,
        wasm_code: &[u8],
        ext: &mut dyn Externalities,
    ) -> Result<Vec<u8>, String> {
        self.inner.read_runtime_version(wasm_code, ext)
    }
}

impl<E: RuntimeVersionOf> RuntimeVersionOf for TimedExecutor<E> {
    fn runtime_version(
        &self,
        ext: &mut dyn Externalities,
        runtime_code: &RuntimeCode<'_>,
    ) -> sc_executor::error::Result<RuntimeVersion> {
        let started = Instant::now();
        let result = self.inner.runtime_version(ext, runtime_code);

        if let Some(metrics) = &self.metrics {
            metrics
                .runtime_version_seconds
                .observe(started.elapsed().as_secs_f64());
        }

        result
    }
}

impl<E: CodeExecutor> CodeExecutor for TimedExecutor<E> {
    type Error = E::Error;

    fn call(
        &self,
        ext: &mut dyn Externalities,
        runtime_code: &RuntimeCode<'_>,
        method: &str,
        data: &[u8],
        context: CallContext,
    ) -> (Result<Vec<u8>, Self::Error>, bool) {
        let started = Instant::now();
        let (result, native) = self.inner.call(ext, runtime_code, method, data, context);

        if let Some(metrics) = &self.metrics {
            metrics.observe(method, started.elapsed().as_secs_f64(), result.is_err());
        }

        (result, native)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A test double for the executor, so the wrapper's behaviour (delegation,
    /// error propagation, and metric recording) can be checked without a
    /// runtime.
    #[derive(Clone)]
    struct Recorder {
        calls: Arc<std::sync::Mutex<Vec<String>>>,
        fail_with: Option<String>,
    }

    impl ReadRuntimeVersion for Recorder {
        fn read_runtime_version(
            &self,
            _wasm_code: &[u8],
            _ext: &mut dyn Externalities,
        ) -> Result<Vec<u8>, String> {
            Err("not used in these tests".to_string())
        }
    }

    impl RuntimeVersionOf for Recorder {
        fn runtime_version(
            &self,
            _ext: &mut dyn Externalities,
            _runtime_code: &RuntimeCode<'_>,
        ) -> sc_executor::error::Result<RuntimeVersion> {
            Err(sc_executor::error::Error::ApiError("not used".into()))
        }
    }

    impl CodeExecutor for Recorder {
        type Error = String;

        fn call(
            &self,
            _ext: &mut dyn Externalities,
            _runtime_code: &RuntimeCode<'_>,
            method: &str,
            _data: &[u8],
            _context: CallContext,
        ) -> (Result<Vec<u8>, Self::Error>, bool) {
            self.calls.lock().expect("lock").push(method.to_string());
            match &self.fail_with {
                Some(message) if message == method => (Err(message.clone()), false),
                _ => (Ok(vec![1, 2, 3]), false),
            }
        }
    }

    fn registry() -> Registry {
        Registry::new()
    }

    fn externalities() -> sp_io::TestExternalities {
        sp_io::TestExternalities::default()
    }

    /// The wrapper has to be invisible to the client: same result, same error,
    /// same native flag, same arguments passed through.
    #[test]
    fn the_wrapper_passes_every_call_through_unchanged() {
        let recorder = Recorder {
            calls: Arc::new(std::sync::Mutex::new(Vec::new())),
            fail_with: None,
        };
        let executor = TimedExecutor::new(recorder.clone(), None);

        let code = sp_core::traits::RuntimeCode::empty();
        let mut storage = externalities();
        let mut ext = storage.ext();

        let (result, native) = executor.call(
            &mut ext,
            &code,
            "Core_execute_block",
            &[],
            CallContext::Onchain,
        );
        assert_eq!(result.expect("call succeeds"), vec![1, 2, 3]);
        assert!(!native);
        assert_eq!(
            recorder.calls.lock().expect("lock").as_slice(),
            ["Core_execute_block"]
        );
    }

    #[test]
    fn an_error_from_the_inner_executor_comes_back_as_an_error() {
        let recorder = Recorder {
            calls: Arc::new(std::sync::Mutex::new(Vec::new())),
            fail_with: Some("Core_execute_block".to_string()),
        };
        let executor = TimedExecutor::new(recorder, None);

        let code = sp_core::traits::RuntimeCode::empty();
        let mut storage = externalities();
        let mut ext = storage.ext();

        let (result, _) = executor.call(
            &mut ext,
            &code,
            "Core_execute_block",
            &[],
            CallContext::Onchain,
        );
        assert!(result.is_err(), "an inner error must not be swallowed");
    }

    #[test]
    fn calls_are_recorded_per_method_with_their_failures() {
        let metrics = Arc::new(RuntimeCallMetrics::register(&registry()).expect("register"));
        let recorder = Recorder {
            calls: Arc::new(std::sync::Mutex::new(Vec::new())),
            fail_with: Some("BlockBuilder_apply_extrinsic".to_string()),
        };
        let executor = TimedExecutor::new(recorder, Some(metrics.clone()));

        let code = sp_core::traits::RuntimeCode::empty();
        let mut storage = externalities();
        let mut ext = storage.ext();

        for _ in 0..3 {
            let _ = executor.call(&mut ext, &code, "Core_execute_block", &[], CallContext::Onchain);
        }
        let _ = executor.call(
            &mut ext,
            &code,
            "BlockBuilder_apply_extrinsic",
            &[],
            CallContext::Onchain,
        );

        assert_eq!(
            metrics
                .calls_total
                .with_label_values(&["Core_execute_block"])
                .get(),
            3,
            "every call of a method has to be counted"
        );
        assert_eq!(
            metrics
                .call_seconds
                .with_label_values(&["Core_execute_block"])
                .get_sample_count(),
            3,
            "every timed call has to contribute one observation"
        );
        assert_eq!(
            metrics
                .call_errors_total
                .with_label_values(&["BlockBuilder_apply_extrinsic"])
                .get(),
            1,
            "the failing call has to be visible as an error"
        );
        assert_eq!(
            metrics
                .call_errors_total
                .with_label_values(&["Core_execute_block"])
                .get(),
            0,
            "a call that succeeded must not be counted as an error"
        );
    }
}
