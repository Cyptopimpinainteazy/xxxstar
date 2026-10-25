//! Canonical production compilation and an explicit legacy X3BC compatibility API.
use crate::{X3IntegrationError, X3Result};
use x3_compiler::{CompilationOptions, Compiler};

#[cfg(not(feature = "std"))]
use alloc::{format, vec::Vec};

/// The capabilities a deployment compiles into the artifact it submits.
///
/// The chain's intake path reads these demands out of the module header and refuses the program
/// when the chain cannot meet them (`pallet-x3-kernel`), and both interpreters refuse to run it
/// (`mini_x3` on a block, `x3-vm` off it). Compiling the demand is therefore the only thing a
/// caller has to do — there is deliberately no second, caller-supplied flag at submission time,
/// which could silently override the compiled policy (AGENTS.md §11).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CompilationPolicy {
    /// Compile the artifact so its policy requires private submission.
    pub require_private_submission: bool,
}

impl CompilationPolicy {
    /// The artifact may only run where a private submission channel exists.
    pub fn private_submission_required() -> Self {
        Self {
            require_private_submission: true,
        }
    }
}

/// Compile the legacy contract grammar into the legacy X3BC module format.
///
/// The returned bytes use the versioned `X3BC` format implemented by
/// `x3_backend::BytecodeModule`. Invalid source and every compiler pipeline
/// failure are returned as `CompilationFailed`; this function never substitutes
/// empty or synthetic bytecode.
///
/// This is the default policy: a program that does not ask for privacy is compiled without a
/// demand. Use [`compile_legacy_source_with_policy`] when the deployment requires one.
pub fn compile_legacy_source(source: &str) -> X3Result<Vec<u8>> {
    compile_legacy_source_with_policy(source, CompilationPolicy::default())
}

/// Compile valid `.x3` source with an explicit deployment policy.
///
/// Everything [`compile_legacy_source`] proves still holds; the only difference is that the artifact's
/// header records the capabilities `policy` demands, so a runtime that cannot offer them refuses
/// the program instead of running it in the clear.
pub fn compile_legacy_source_with_policy(
    source: &str,
    policy: CompilationPolicy,
) -> X3Result<Vec<u8>> {
    let mut options = CompilationOptions::contract_mode();
    if policy.require_private_submission {
        options = options.with_private_submission_required();
    }

    let output = Compiler::compile(source, options)
        .map_err(|error| X3IntegrationError::CompilationFailed(format!("{error:?}")))?;

    let bytes = output.bytecode.to_bytes();
    if bytes.is_empty() {
        return Err(X3IntegrationError::CompilationFailed(
            "compiler returned an empty bytecode module".into(),
        ));
    }

    Ok(bytes)
}

/// Compile through the canonical language workspace in Testnet mode.
/// There is no fallback to legacy grammar or legacy bytecode.
pub fn compile_source(source: &str) -> X3Result<Vec<u8>> {
    compile_source_with_mode(source, x3_lang_compiler::CompilationMode::Testnet)
}

pub fn compile_source_with_mode(
    source: &str,
    mode: x3_lang_compiler::CompilationMode,
) -> X3Result<Vec<u8>> {
    x3_lang_compiler::compile_with_mode(source, mode)
        .map_err(|error| X3IntegrationError::CompilationFailed(format!("{error}")))
}

/// A deployment requirement may strengthen the source policy, never replace it.
/// If the source did not compile a private-submission demand, require the author
/// to encode it in the source rather than mutating the artifact after compilation.
pub fn compile_source_with_policy(source: &str, policy: CompilationPolicy) -> X3Result<Vec<u8>> {
    let bytes = compile_source(source)?;
    if policy.require_private_submission {
        let artifact = x3_lang_execution::artifact::decode_artifact(&bytes)
            .map_err(X3IntegrationError::CompilationFailed)?;
        if !artifact.requires_private_submission {
            return Err(X3IntegrationError::CompilationFailed(
                "source policy must require private submission".into(),
            ));
        }
    }
    Ok(bytes)
}

/// Provenance for a compilation performed by the canonical compiler.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalCompilation {
    pub bytecode: Vec<u8>,
    pub source_hash: [u8; 32],
    pub artifact_hash: [u8; 32],
    pub compiler_version: &'static str,
    pub mode: x3_lang_compiler::CompilationMode,
}

pub fn compile_artifact(
    source: &str,
    mode: x3_lang_compiler::CompilationMode,
) -> X3Result<CanonicalCompilation> {
    let bytecode = compile_source_with_mode(source, mode)?;
    Ok(CanonicalCompilation {
        source_hash: sp_io::hashing::sha2_256(source.as_bytes()),
        artifact_hash: sp_io::hashing::sha2_256(&bytecode),
        compiler_version: x3_lang_compiler::COMPILER_VERSION,
        mode,
        bytecode,
    })
}
