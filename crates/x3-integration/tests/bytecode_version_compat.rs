//! Old artifacts, new artifacts, and the loader the chain actually runs.
//!
//! The X3Lang row asks for "old artifacts executed against upgraded VM/runtime versions". The gates
//! for that exist in two places, and they are different pipelines: `x3-lang/`'s version byte rules
//! (TICKET-097, TICKET-137) govern the canonical compiler, while the loader *this chain executes*
//! is `x3-integration::mini_x3` — reached through `pallets/x3-kernel`'s adapter. This file exercises
//! the second one, on artifacts the real compiler produced, and asserts every verdict **by name**:
//! a test that only said `is_err()` would keep passing if the version gate were deleted.
//!
//! The envelope is one format with one definition (`x3-common::bytecode`), so the fixtures here are
//! as valid as compiler output — they rewrite header fields and recompute nothing, because the
//! checksum covers the body and the body is untouched. That distinction matters: an earlier
//! hand-assembled fixture with a zeroed checksum hid a reader that never checked it (TICKET-108).

#![cfg(all(feature = "std", feature = "compile"))]

use x3_common::bytecode as bc;
use x3_x3_integration::mini_x3::{execute_x3bc, MiniValue, X3Error};

/// Header offsets, in the order the writer emits them and both readers check them.
const VERSION_OFFSET: usize = 4;
const CHECKSUM_OFFSET: usize = bc::CHECKSUM_OFFSET;
const MIN_VERSION_OFFSET: usize = 16;

/// A program compiled now, by the same bridge the node uses.
fn compiled_now() -> Vec<u8> {
    x3_x3_integration::compiler_bridge::compile_source("fn main() -> i64 {\n    return 7;\n}\n")
        .expect("the fixture must compile")
}

fn with_u32_at(bytes: &[u8], offset: usize, value: u32) -> Vec<u8> {
    let mut out = bytes.to_vec();
    out[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    out
}

/// The row this whole file is about: the current artifact runs, and returns what the source says.
#[test]
fn the_current_artifact_runs_on_the_loader_the_chain_uses() {
    let artifact = compiled_now();
    assert_eq!(
        &artifact[..4],
        bc::MAGIC,
        "the writer emits the shared magic"
    );

    let result = execute_x3bc(&artifact, 1_000_000).expect("a current artifact must execute");
    assert_eq!(
        result.return_val,
        MiniValue::I64(7),
        "and it must return the value the source states"
    );
    assert!(result.gas_used > 0, "execution must be metered");
}

/// A patch difference is compatible by the format's own versioning rules, and the loader agrees.
#[test]
fn a_patch_bump_stays_readable() {
    let artifact = with_u32_at(&compiled_now(), VERSION_OFFSET, bc::VERSION + 1);
    assert!(
        execute_x3bc(&artifact, 1_000_000).is_ok(),
        "1.0.1 must be readable by a 1.0.0 loader — TICKET-137 is the bug where it was not"
    );
}

/// A newer minor is not readable, and the refusal names the version it found.
#[test]
fn a_newer_minor_is_refused_by_name() {
    let future = bc::VERSION + 0x100; // 1.1.0
    let artifact = with_u32_at(&compiled_now(), VERSION_OFFSET, future);
    assert_eq!(
        execute_x3bc(&artifact, 1_000_000).unwrap_err(),
        X3Error::UnsupportedVersion(future),
        "the loader must name the version it refused"
    );
}

/// The next major is not readable — a loader does not guess at a format it does not know.
#[test]
fn the_next_major_is_refused_by_name() {
    let next_major = bc::MAX_SUPPORTED_VERSION;
    let artifact = with_u32_at(&compiled_now(), VERSION_OFFSET, next_major);
    assert_eq!(
        execute_x3bc(&artifact, 1_000_000).unwrap_err(),
        X3Error::UnsupportedVersion(next_major)
    );
}

/// An artifact that *demands* a newer loader is refused by that demand.
///
/// This is the field an older chain uses to refuse a newer artifact: the loader reads
/// `min_version` and answers "I am not new enough", rather than executing a format it predates.
#[test]
fn an_artifact_that_demands_a_newer_loader_is_refused_by_name() {
    let demanded = bc::VERSION + 0x100;
    let artifact = with_u32_at(&compiled_now(), MIN_VERSION_OFFSET, demanded);
    assert_eq!(
        execute_x3bc(&artifact, 1_000_000).unwrap_err(),
        X3Error::UnsupportedVersion(demanded),
        "a module requiring a newer loader must be refused by the version it requires"
    );
}

/// A body edited after compilation is refused with both checksums, not as a parse error.
#[test]
fn a_body_edit_is_refused_with_the_two_checksums() {
    let mut artifact = compiled_now();
    let declared = u32::from_le_bytes(
        artifact[CHECKSUM_OFFSET..CHECKSUM_OFFSET + 4]
            .try_into()
            .expect("the header carries a checksum"),
    );
    let last = artifact.len() - 1;
    artifact[last] ^= 0xFF;
    let recomputed = bc::checksum(&artifact[bc::HEADER_LEN..]);

    assert_ne!(
        declared, recomputed,
        "the fixture must actually change the body"
    );
    assert_eq!(
        execute_x3bc(&artifact, 1_000_000).unwrap_err(),
        X3Error::ChecksumMismatch {
            expected: recomputed,
            found: declared,
        },
        "the refusal must report which checksum it expected and which it found"
    );
}

/// The version gate runs *before* execution: a refused artifact never reaches the interpreter.
#[test]
fn the_version_gate_runs_before_execution() {
    let next_major = bc::MAX_SUPPORTED_VERSION;
    let artifact = with_u32_at(&compiled_now(), VERSION_OFFSET, next_major);
    let error = execute_x3bc(&artifact, 1_000_000).unwrap_err();
    assert!(
        matches!(error, X3Error::UnsupportedVersion(_)),
        "a future artifact must be refused by the version gate, not by the interpreter: {error:?}"
    );
    assert!(
        !matches!(error, X3Error::GasExhausted | X3Error::InvalidOpcode(_)),
        "and it must not have started executing"
    );
}

/// The bounds the matrix above depends on, checked at *compile* time.
///
/// These are constants, so a runtime `assert!` about their order cannot fail and would be optimized
/// out — clippy said exactly that (`assertions_on_constants`) and it was right. A `const` assertion
/// is the honest form: widening the loader's range now fails the build here rather than leaving a
/// test that passes vacuously.
const _: () = {
    assert!(bc::MIN_SUPPORTED_VERSION <= bc::VERSION);
    assert!(bc::VERSION < bc::MAX_SUPPORTED_VERSION);
};

/// And the predicates callers actually use, evaluated on those bounds.
#[test]
fn the_loader_bounds_are_what_the_loader_claims() {
    let min = bc::MIN_SUPPORTED_VERSION;
    let max = bc::MAX_SUPPORTED_VERSION;
    let version = bc::VERSION;

    let min_is_readable = bc::version_is_readable(min);
    let max_is_readable = bc::version_is_readable(max);
    let loader_satisfies_itself = bc::loader_satisfies(version);

    assert!(
        min_is_readable,
        "a loader that cannot read its own minimum is misconfigured"
    );
    assert!(
        !max_is_readable,
        "the exclusive upper bound must not be readable"
    );
    assert!(
        loader_satisfies_itself,
        "the loader must satisfy what it writes"
    );
}
