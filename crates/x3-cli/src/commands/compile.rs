//! Compile command - standalone X3 file compilation without project.
//!
//! This command allows compiling individual X3 source files without
//! needing a full project structure.

use crate::error::{CliError, Result};
use clap::{Args, ValueEnum};
use colored::Colorize;
use std::path::PathBuf;
use x3_compiler::{CompilationOptions, Compiler};

/// Emit format for X3 compilation
#[derive(Clone, ValueEnum, Default)]
pub enum EmitType {
    /// Emit bytecode (default)
    #[default]
    Bytecode,
    /// Emit MIR representation
    Mir,
    /// Emit HIR representation
    Hir,
}

#[derive(Args)]
pub struct CompileArgs {
    /// X3 source file to compile
    #[arg(required = true)]
    pub input: PathBuf,

    /// Output file (defaults to input with .x3b extension)
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Optimization level (0-3)
    #[arg(short = 'O', long = "opt-level", value_parser = clap::value_parser!(u8).range(0..=3), default_value = "2")]
    pub optimization: u8,

    /// Verbose output (shows compilation progress)
    #[arg(short, long)]
    pub verbose: bool,

    /// Emit intermediate representation (mir, hir, bytecode)
    #[arg(long, value_enum)]
    pub emit: Option<EmitType>,

    /// Emit optimization statistics
    #[arg(long)]
    pub stats: bool,

    /// Compile the artifact so its policy demands private submission.
    ///
    /// This is a property of the *artifact*, not of the invocation that runs it: the demanded
    /// capability is compiled in, and a chain with no private channel refuses the program at intake
    /// rather than executing it in the clear.
    #[arg(long)]
    pub require_private_submission: bool,

    /// Emit optimized MIR alongside bytecode
    #[arg(long)]
    pub emit_mir_opt: bool,

    /// Enable debug info in output
    #[arg(short = 'g', long)]
    pub debug: bool,

    /// Sign the artifact: a 64-hex ed25519 seed. Writes a detached attestation next to the
    /// artifact (`<output>.sig.json`) that `x3 verify-artifact` checks. Requires `--key-id` and
    /// `--signing-registry`.
    #[arg(long, value_name = "HEX", requires_all = ["key_id", "signing_registry"])]
    pub sign_key_hex: Option<String>,

    /// The key id the attestation names.
    #[arg(long, requires = "sign_key_hex")]
    pub key_id: Option<String>,

    /// The artifact key registry (`{"keys": [{"key_id", "public_key", "status"}]}`). The artifact is
    /// signed only by a key that is active in it and listed under `--key-id` with this key's public
    /// key; the attestation is verified against it before anything is written.
    #[arg(long, value_name = "FILE", requires = "sign_key_hex")]
    pub signing_registry: Option<PathBuf>,

    /// How the signing registry is trusted (`--registry-root` or `--unsigned-registry`).
    #[command(flatten)]
    pub registry_trust: crate::commands::registry_trust::RegistryTrust,

    /// Disable optimization (shorthand for -O0)
    #[arg(long = "no-opt")]
    pub no_opt: bool,
}

pub async fn execute(args: CompileArgs) -> Result<()> {
    // Validate input file
    if !args.input.exists() {
        return Err(CliError::Build(format!(
            "Input file not found: {}",
            args.input.display()
        )));
    }

    if args.input.extension().map_or(true, |ext| ext != "x3") {
        return Err(CliError::Build(format!(
            "Input file must have .x3 extension: {}",
            args.input.display()
        )));
    }

    // Read source
    let source = std::fs::read_to_string(&args.input)?;
    let file_stem = args.input.file_stem().unwrap().to_string_lossy();

    // Build compilation options
    let opt_level = if args.no_opt {
        x3_compiler::OptLevel::None
    } else {
        match args.optimization {
            0 => x3_compiler::OptLevel::None,
            1 => x3_compiler::OptLevel::Basic,
            2 => x3_compiler::OptLevel::Default,
            _ => x3_compiler::OptLevel::Aggressive,
        }
    };

    let emit_format = match args.emit.as_ref().unwrap_or(&EmitType::Bytecode) {
        EmitType::Bytecode => x3_compiler::options::EmitFormat::Bytecode,
        EmitType::Mir => x3_compiler::options::EmitFormat::Mir,
        EmitType::Hir => x3_compiler::options::EmitFormat::Hir,
    };

    let options = CompilationOptions {
        opt_level,
        debug: args.debug,
        verbose: args.verbose,
        emit_hir: matches!(args.emit, Some(EmitType::Hir)),
        emit_mir: matches!(args.emit, Some(EmitType::Mir)),
        emit_mir_opt: args.emit_mir_opt || args.stats,
        emit_stats: args.stats,
        emit_format,
        analyze_gas: false,
        verify_contract: false,
        require_private_submission: args.require_private_submission,
    };

    if args.verbose {
        println!("{} X3 Compiler v0.1.0", "🔧".blue());
        println!("  → Input: {}", args.input.display());
        println!("  → Optimization: {:?}", opt_level);
    }

    // Compile
    let output = Compiler::compile(&source, options.clone())
        .map_err(|e| CliError::Build(format!("Compilation failed: {:?}", e)))?;

    // Determine output path
    let out_dir = args.input.parent().unwrap_or(std::path::Path::new("."));
    let bytecode_file = args
        .output
        .clone()
        .unwrap_or_else(|| out_dir.join(format!("{}.x3b", file_stem)));

    // The artifact is the whole X3BC envelope: magic, header, function table, constant pool,
    // globals and code, with its checksum. This wrote `output.bytecode.code` — the code section
    // alone, with no magic and no function table — so a `.x3b` from `x3 compile` could be loaded
    // by no reader of the format (`mini_x3::validate_x3bc` refused it as `InvalidMagic`), and a
    // program with more than one function lost the table that says where each begins.
    let artifact = output.bytecode.to_bytes();

    // Sign before writing anything, so a refused signature leaves no half-produced output.
    let attestation = match &args.sign_key_hex {
        Some(seed_hex) => {
            let signing_registry = args.signing_registry.as_ref().ok_or_else(|| {
                CliError::InvalidArgument("--sign-key-hex requires --signing-registry".to_string())
            })?;
            Some(sign_artifact(
                &artifact,
                seed_hex,
                args.key_id.as_deref().unwrap_or_default(),
                signing_registry,
                &args.registry_trust,
            )?)
        }
        None => None,
    };

    std::fs::write(&bytecode_file, &artifact)?;
    if let Some(attestation) = &attestation {
        let sidecar = attestation_path(&bytecode_file);
        let json = serde_json::to_string_pretty(attestation)
            .map_err(|e| CliError::Build(format!("encode attestation: {e}")))?;
        std::fs::write(&sidecar, json)?;
        println!(
            "{} Signed as '{}' → {}",
            "✓".green(),
            attestation.key_id,
            sidecar.display()
        );
    }

    println!(
        "{} Compiled: {} → {} ({} bytes)",
        "✓".green(),
        args.input.display(),
        bytecode_file.display(),
        artifact.len()
    );

    // Write stats if requested
    if args.stats {
        if let Some(ref artifacts) = output.artifacts {
            if let Some(ref stats) = artifacts.opt_stats {
                println!("\n{}", "📊 Optimization Statistics:".blue().bold());
                println!("   Passes run:        {}", stats.passes_run);
                println!("   Passes changed:    {}", stats.passes_changed);
                println!("   Transformations:   {}", stats.total_transformations);
                println!("   Iterations:        {}", stats.iterations);
            }
        }
    }

    // Write MIR if requested
    if options.emit_mir {
        if let Some(ref artifacts) = output.artifacts {
            if let Some(ref mir) = artifacts.mir_unoptimized {
                let mir_file = out_dir.join(format!("{}.mir", file_stem));
                std::fs::write(&mir_file, format!("{:#?}", mir))?;
                println!("   → MIR: {}", mir_file.display());
            }
        }
    }

    if options.emit_mir_opt {
        if let Some(ref artifacts) = output.artifacts {
            if let Some(ref mir) = artifacts.mir_optimized {
                let mir_file = out_dir.join(format!("{}.mir.opt", file_stem));
                std::fs::write(&mir_file, format!("{:#?}", mir))?;
                println!("   → Optimized MIR: {}", mir_file.display());
            }
        }
    }

    Ok(())
}

/// Where the detached attestation for `artifact` lives: `<artifact>.sig.json`.
pub fn attestation_path(artifact: &std::path::Path) -> PathBuf {
    let mut name = artifact.as_os_str().to_owned();
    name.push(".sig.json");
    PathBuf::from(name)
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct ArtifactRegistryFile {
    keys: Vec<x3_common::artifact::RegisteredArtifactKey>,
}

/// Load an artifact key registry from its JSON file.
pub fn load_artifact_registry(path: &PathBuf) -> Result<x3_common::artifact::ArtifactKeyRegistry> {
    let body = std::fs::read_to_string(path)
        .map_err(|e| CliError::Build(format!("read {}: {e}", path.display())))?;
    let file: ArtifactRegistryFile = serde_json::from_str(&body)
        .map_err(|e| CliError::Build(format!("artifact key registry {}: {e}", path.display())))?;
    x3_common::artifact::ArtifactKeyRegistry::from_entries(file.keys).map_err(CliError::Build)
}

/// Sign `artifact` as `key_id`, refusing a key the registry says may not sign, and verify the
/// result against the registry before returning it (which also refuses a signing key that is not
/// the public key the registry lists under `key_id`).
fn sign_artifact(
    artifact: &[u8],
    seed_hex: &str,
    key_id: &str,
    registry_path: &PathBuf,
    trust: &crate::commands::registry_trust::RegistryTrust,
) -> Result<x3_common::artifact::ArtifactAttestation> {
    use sp_core::Pair as _;

    let registry = crate::commands::registry_trust::load_trusted_registry(registry_path, trust)?;
    if !registry.may_sign(key_id) {
        let status = registry
            .status(key_id)
            .map(|status| format!("{status:?}").to_lowercase())
            .unwrap_or_else(|| "not listed".to_string());
        return Err(CliError::Build(format!(
            "key '{key_id}' may not sign artifacts: the key registry has it as {status}"
        )));
    }
    let seed: [u8; 32] = hex::decode(seed_hex.trim())
        .ok()
        .and_then(|bytes| bytes.try_into().ok())
        .ok_or_else(|| CliError::Build("--sign-key-hex must be 64 hex characters".to_string()))?;
    let pair = sp_core::ed25519::Pair::from_seed(&seed);
    let attestation = x3_common::artifact::ArtifactAttestation::sign(artifact, key_id, &pair);
    attestation.verify(artifact, &registry).map_err(|e| {
        CliError::Build(format!(
            "the new attestation does not verify against the registry: {e}"
        ))
    })?;
    Ok(attestation)
}
