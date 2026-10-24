#![no_main]

//! Deliberately-defective fuzz target used ONLY to validate the fuzzing
//! toolchain (libFuzzer + cargo-fuzz + ASAN) on this host.
//!
//! Contract of the (fictional) "record" being fuzzed: a well-formed record
//! never contains the 14-byte magic `X3CRASHFIXTURE`. The injected defect
//! panics when the magic appears, which gives libFuzzer a real crash to find
//! and report. The magic is deliberately long enough that libFuzzer cannot
//! guess it from benign seeds inside the short negative-control run, so the
//! known-good and known-bad cases stay distinguishable. It is not wired into
//! anything X3 ships.

use libfuzzer_sys::fuzz_target;

const FORBIDDEN_TAG: &[u8] = b"X3CRASHFIXTURE";

fuzz_target!(|data: &[u8]| {
    if data.windows(FORBIDDEN_TAG.len()).any(|w| w == FORBIDDEN_TAG) {
        panic!("tool-validation fixture: injected defect triggered by X3CRASH tag");
    }
});
