//! Command-line front end for the X3 state-snapshot verifier.
//!
//! ```text
//! x3-state-snapshot hash-manifest <manifest.json>
//! x3-state-snapshot root --from-raw-spec <spec.json> [--state-version <0|1>]
//! x3-state-snapshot build --from-raw-spec <spec.json> --out <dir>
//!     --chain-id <id> --block-number <n> --block-hash <0x..> --state-root <0x..>
//!     --runtime-version <n> --finality-proof <0x..>
//!     [--database-format <tag>] [--chunk-size <bytes>] [--state-version <0|1>]
//! x3-state-snapshot verify --manifest <manifest.json> --chunks <dir>
//!     [--chain-id <id>] [--block-hash <0x..>] [--state-root <0x..>]
//!     [--runtime-version <n>] [--min-block <n>] [--manifest-hash <0x..>]
//!     [--state-version <0|1>]
//! x3-state-snapshot restore --manifest <manifest.json> --chunks <dir>
//!     --out <spec.json> --chain-id <id> --block-hash <0x..> --state-root <0x..>
//!     --runtime-version <n> [--from-spec <template.json>] [--name <name>]
//!     [--id <id>] [--min-block <n>] [--manifest-hash <0x..>]
//!     [--state-version <0|1>] [--force]
//! ```
//!
//! Chunk files are named `<index>.chunk` (0-based, decimal) inside `--chunks`.
//! Exit code 0 means the snapshot verified, 1 means it was refused, and 2 means
//! the command line itself was wrong.
//!
//! `build` takes its state from the output of the node's own `export-state`
//! (a plain chain spec, whose `genesis.raw.top` is the full state map). That is
//! deliberate: the exporter should not be a second implementation of how to read
//! the database, and a raw spec is already the shape a node can hand out.
//!
//! `restore` is the other direction: it turns a verified snapshot back into a raw
//! chain spec a node boots from. It requires the consensus anchor rather than
//! making it optional, and it writes the spec atomically — a partial file is
//! never visible under `--out`, because a half-written chain spec is a chain spec
//! with missing state.

#![deny(unsafe_code)]

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use x3_state_snapshot::{
    compute_state_root, manifest_hash, restore_snapshot, state_entries_from_raw_spec,
    verify_snapshot_with_state_root, write_snapshot, RestoreRequest, SnapshotBuilder,
    SnapshotManifest, TrieVersion, TrustedAnchor,
};

/// Default chunk size for a freshly built snapshot (4 MiB).
const DEFAULT_CHUNK_SIZE: usize = 4 * 1024 * 1024;

/// Default backend tag for state exported from a Substrate trie over RocksDB.
const DEFAULT_DATABASE_FORMAT: &str = "rocksdb-substrate-trie-v1";

const USAGE: &str = "\
usage:
  x3-state-snapshot hash-manifest <manifest.json>
  x3-state-snapshot root --from-raw-spec <spec.json> [--state-version <0|1>]
  x3-state-snapshot build --from-raw-spec <spec.json> --out <dir>
      --chain-id <id> --block-number <n> --block-hash <0x..> --state-root <0x..>
      --runtime-version <n> --finality-proof <0x..>
      [--database-format <tag>] [--chunk-size <bytes>] [--state-version <0|1>]
  x3-state-snapshot verify --manifest <manifest.json> --chunks <dir>
      [--chain-id <id>] [--block-hash <0x..>] [--state-root <0x..>]
      [--runtime-version <n>] [--min-block <n>] [--manifest-hash <0x..>]
      [--state-version <0|1>]
  x3-state-snapshot restore --manifest <manifest.json> --chunks <dir> --out <spec.json>
      --chain-id <id> --block-hash <0x..> --state-root <0x..> --runtime-version <n>
      [--from-spec <template.json>] [--name <name>] [--id <id>] [--min-block <n>]
      [--manifest-hash <0x..>] [--state-version <0|1>] [--force]

Chunk files inside --chunks are named <index>.chunk (0-based, decimal).

build derives the state root from the exported state; --state-root is optional and,
when given, must agree with that derivation or the export is refused.

restore requires the full anchor (--chain-id, --block-hash, --state-root,
--runtime-version): the snapshot is verified against it before a spec is written.
--from-spec contributes name, id, chain type, bootnodes, protocol id, telemetry and
properties only; its state, if any, is replaced. The output is written to a
temporary file and renamed, and an existing --out is refused without --force.";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();

    match args.first().map(String::as_str) {
        Some("hash-manifest") => match args.get(1) {
            Some(path) => hash_manifest(Path::new(path)),
            None => fail_usage("hash-manifest needs a manifest path"),
        },
        Some("build") => build(&args[1..]),
        Some("root") => root(&args[1..]),
        Some("verify") => verify(&args[1..]),
        Some("restore") => restore(&args[1..]),
        _ => fail_usage("expected a subcommand"),
    }
}

/// Print the state root a raw spec's genesis state produces.
///
/// Separate from `build` on purpose: this answers "what does this state hash
/// to?" without writing a manifest, so a root probe does not need to invent a
/// finality proof. It is the piece that lets the derived root be compared with a
/// chain-produced one.
fn root(args: &[String]) -> ExitCode {
    let mut spec_path: Option<PathBuf> = None;
    let mut state_version: Option<u8> = None;
    let mut index = 0;

    while index < args.len() {
        let flag = args[index].clone();
        match flag.as_str() {
            "--from-raw-spec" => {
                spec_path = Some(PathBuf::from(match value_at(args, &mut index, &flag) {
                    Ok(value) => value,
                    Err(message) => return fail_usage(&message),
                }));
            }
            "--state-version" => {
                let raw = match value_at(args, &mut index, &flag) {
                    Ok(value) => value,
                    Err(message) => return fail_usage(&message),
                };
                match raw.parse::<u8>() {
                    Ok(version) => state_version = Some(version),
                    Err(err) => return fail_usage(&format!("{flag} {raw:?}: {err}")),
                }
            }
            other => return fail_usage(&format!("unexpected argument {other:?}")),
        }
        index += 1;
    }

    let Some(spec_path) = spec_path else {
        return fail_usage("root needs --from-raw-spec");
    };

    let raw = match std::fs::read_to_string(&spec_path) {
        Ok(raw) => raw,
        Err(err) => {
            eprintln!("error: cannot read {}: {err}", spec_path.display());
            return ExitCode::from(1);
        }
    };
    let spec: serde_json::Value = match serde_json::from_str(&raw) {
        Ok(spec) => spec,
        Err(err) => {
            eprintln!("error: cannot parse {}: {err}", spec_path.display());
            return ExitCode::from(1);
        }
    };
    let entries = match state_entries_from_raw_spec(&spec) {
        Ok(entries) => entries,
        Err(err) => {
            eprintln!("error: {err}");
            return ExitCode::from(1);
        }
    };
    let version = match TrieVersion::from_u8(state_version.unwrap_or(1)) {
        Ok(version) => version,
        Err(err) => {
            eprintln!("error: {err}");
            return ExitCode::from(1);
        }
    };

    match compute_state_root(&entries, version) {
        Ok(root) => {
            println!("{root}");
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("error: {err}");
            ExitCode::from(1)
        }
    }
}

fn fail_usage(message: &str) -> ExitCode {
    eprintln!("error: {message}");
    eprintln!("{USAGE}");
    ExitCode::from(2)
}

fn load_manifest(path: &Path) -> Result<SnapshotManifest, String> {
    let raw = std::fs::read_to_string(path)
        .map_err(|err| format!("cannot read manifest {}: {err}", path.display()))?;
    serde_json::from_str(&raw)
        .map_err(|err| format!("cannot parse manifest {}: {err}", path.display()))
}

fn hash_manifest(path: &Path) -> ExitCode {
    match load_manifest(path).and_then(|m| manifest_hash(&m).map_err(|err| err.to_string())) {
        Ok(hash) => {
            println!("{hash}");
            ExitCode::SUCCESS
        }
        Err(message) => {
            eprintln!("error: {message}");
            ExitCode::from(1)
        }
    }
}

#[derive(Default)]
struct VerifyArgs {
    manifest: Option<PathBuf>,
    chunks: Option<PathBuf>,
    chain_id: Option<String>,
    block_hash: Option<String>,
    state_root: Option<String>,
    runtime_version: Option<u32>,
    min_block: u64,
    manifest_hash: Option<String>,
    state_version: Option<u8>,
}

#[derive(Default)]
struct BuildArgs {
    from_raw_spec: Option<PathBuf>,
    out: Option<PathBuf>,
    chain_id: Option<String>,
    block_number: Option<u64>,
    block_hash: Option<String>,
    state_root: Option<String>,
    runtime_version: Option<u32>,
    finality_proof: Option<String>,
    database_format: Option<String>,
    chunk_size: Option<usize>,
    state_version: Option<u8>,
}

fn parse_build_args(args: &[String]) -> Result<BuildArgs, String> {
    let mut parsed = BuildArgs::default();
    let mut index = 0;

    while index < args.len() {
        let flag = args[index].clone();
        match flag.as_str() {
            "--from-raw-spec" => {
                parsed.from_raw_spec = Some(PathBuf::from(value_at(args, &mut index, &flag)?));
            }
            "--out" => parsed.out = Some(PathBuf::from(value_at(args, &mut index, &flag)?)),
            "--chain-id" => parsed.chain_id = Some(value_at(args, &mut index, &flag)?),
            "--block-number" => {
                let raw = value_at(args, &mut index, &flag)?;
                parsed.block_number = Some(
                    raw.parse()
                        .map_err(|err| format!("{flag} {raw:?}: {err}"))?,
                );
            }
            "--block-hash" => parsed.block_hash = Some(value_at(args, &mut index, &flag)?),
            "--state-root" => parsed.state_root = Some(value_at(args, &mut index, &flag)?),
            "--runtime-version" => {
                let raw = value_at(args, &mut index, &flag)?;
                parsed.runtime_version = Some(
                    raw.parse()
                        .map_err(|err| format!("{flag} {raw:?}: {err}"))?,
                );
            }
            "--finality-proof" => parsed.finality_proof = Some(value_at(args, &mut index, &flag)?),
            "--database-format" => {
                parsed.database_format = Some(value_at(args, &mut index, &flag)?);
            }
            "--chunk-size" => {
                let raw = value_at(args, &mut index, &flag)?;
                parsed.chunk_size = Some(
                    raw.parse()
                        .map_err(|err| format!("{flag} {raw:?}: {err}"))?,
                );
            }
            "--state-version" => {
                let raw = value_at(args, &mut index, &flag)?;
                parsed.state_version = Some(
                    raw.parse()
                        .map_err(|err| format!("{flag} {raw:?}: {err}"))?,
                );
            }
            other => return Err(format!("unexpected argument {other:?}")),
        }
        index += 1;
    }

    Ok(parsed)
}

fn require<T>(value: Option<T>, flag: &str) -> Result<T, String> {
    value.ok_or_else(|| format!("build needs {flag}"))
}

fn build(args: &[String]) -> ExitCode {
    let parsed = match parse_build_args(args) {
        Ok(parsed) => parsed,
        Err(message) => return fail_usage(&message),
    };

    let (spec_path, out_dir, chain_id, block_number, block_hash, runtime_version) = match (
        parsed.from_raw_spec.as_deref(),
        parsed.out.as_deref(),
        parsed.chain_id.as_deref(),
        parsed.block_number,
        parsed.block_hash.as_deref(),
        parsed.runtime_version,
    ) {
        (Some(spec), Some(out), Some(chain), Some(number), Some(block), Some(version)) => {
            (spec, out, chain, number, block, version)
        }
        _ => {
            return fail_usage(
                "build needs --from-raw-spec, --out, --chain-id, --block-number, --block-hash \
                 and --runtime-version",
            )
        }
    };

    // A snapshot that does not name its finalized block is not a snapshot, so
    // the proof is required rather than defaulted to something empty.
    let finality_proof = match require(parsed.finality_proof.clone(), "--finality-proof") {
        Ok(proof) => proof,
        Err(message) => return fail_usage(&message),
    };

    let raw = match std::fs::read_to_string(spec_path) {
        Ok(raw) => raw,
        Err(err) => {
            eprintln!("error: cannot read {}: {err}", spec_path.display());
            return ExitCode::from(1);
        }
    };
    let spec: serde_json::Value = match serde_json::from_str(&raw) {
        Ok(spec) => spec,
        Err(err) => {
            eprintln!("error: cannot parse {}: {err}", spec_path.display());
            return ExitCode::from(1);
        }
    };

    let entries = match state_entries_from_raw_spec(&spec) {
        Ok(entries) => entries,
        Err(err) => {
            eprintln!("error: {err}");
            return ExitCode::from(1);
        }
    };

    // The root is a property of the state, not of what the operator typed, so
    // derive it. An explicit `--state-root` is treated as a claim to check
    // against that derivation rather than as something to write down: a
    // mismatch means the exported state is not the state at the block whose
    // root was quoted, and writing that manifest would produce a snapshot that
    // can never verify.
    let version = match TrieVersion::from_u8(parsed.state_version.unwrap_or(1)) {
        Ok(version) => version,
        Err(err) => {
            eprintln!("error: {err}");
            return ExitCode::from(1);
        }
    };
    let derived_root = match compute_state_root(&entries, version) {
        Ok(root) => root,
        Err(err) => {
            eprintln!("error: {err}");
            return ExitCode::from(1);
        }
    };
    let state_root = match parsed.state_root.as_deref() {
        Some(declared) if !declared.eq_ignore_ascii_case(&derived_root) => {
            eprintln!(
                "error: the state read from {} recomputes to {derived_root}, but --state-root \
                 says {declared}. Those are different states; refusing to write a manifest whose \
                 own bytes do not produce the root it declares.",
                spec_path.display()
            );
            return ExitCode::from(1);
        }
        Some(declared) => declared.to_string(),
        None => derived_root.clone(),
    };

    let builder = SnapshotBuilder {
        chain_id: chain_id.to_string(),
        block_number,
        block_hash: block_hash.to_string(),
        state_root: state_root.clone(),
        runtime_spec_version: runtime_version,
        database_format: parsed
            .database_format
            .clone()
            .unwrap_or_else(|| DEFAULT_DATABASE_FORMAT.to_string()),
        chunk_size: parsed.chunk_size.unwrap_or(DEFAULT_CHUNK_SIZE),
        finality_proof,
    };

    let (manifest, chunks) = match builder.build(&entries) {
        Ok(built) => built,
        Err(err) => {
            eprintln!("error: {err}");
            return ExitCode::from(1);
        }
    };

    if let Err(err) = write_snapshot(out_dir, &manifest, &chunks) {
        eprintln!("error: {err}");
        return ExitCode::from(1);
    }

    match manifest_hash(&manifest) {
        Ok(hash) => {
            println!(
                "wrote {} chunk(s), {} state entries, to {} (state root {state_root})",
                manifest.chunk_count,
                entries.len(),
                out_dir.display()
            );
            println!("{hash}");
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("error: {err}");
            ExitCode::from(1)
        }
    }
}

fn value_at(args: &[String], index: &mut usize, flag: &str) -> Result<String, String> {
    *index += 1;
    args.get(*index)
        .cloned()
        .ok_or_else(|| format!("{flag} needs a value"))
}

#[derive(Default)]
struct RestoreArgs {
    manifest: Option<PathBuf>,
    chunks: Option<PathBuf>,
    out: Option<PathBuf>,
    from_spec: Option<PathBuf>,
    name: Option<String>,
    id: Option<String>,
    chain_id: Option<String>,
    block_hash: Option<String>,
    state_root: Option<String>,
    runtime_version: Option<u32>,
    min_block: u64,
    manifest_hash: Option<String>,
    state_version: Option<u8>,
    force: bool,
}

fn parse_restore_args(args: &[String]) -> Result<RestoreArgs, String> {
    let mut parsed = RestoreArgs::default();
    let mut index = 0;

    while index < args.len() {
        let flag = args[index].clone();
        match flag.as_str() {
            "--manifest" => {
                parsed.manifest = Some(PathBuf::from(value_at(args, &mut index, &flag)?));
            }
            "--chunks" => {
                parsed.chunks = Some(PathBuf::from(value_at(args, &mut index, &flag)?));
            }
            "--out" => parsed.out = Some(PathBuf::from(value_at(args, &mut index, &flag)?)),
            "--from-spec" => {
                parsed.from_spec = Some(PathBuf::from(value_at(args, &mut index, &flag)?));
            }
            "--name" => parsed.name = Some(value_at(args, &mut index, &flag)?),
            "--id" => parsed.id = Some(value_at(args, &mut index, &flag)?),
            "--chain-id" => parsed.chain_id = Some(value_at(args, &mut index, &flag)?),
            "--block-hash" => parsed.block_hash = Some(value_at(args, &mut index, &flag)?),
            "--state-root" => parsed.state_root = Some(value_at(args, &mut index, &flag)?),
            "--runtime-version" => {
                let raw = value_at(args, &mut index, &flag)?;
                parsed.runtime_version = Some(
                    raw.parse()
                        .map_err(|err| format!("{flag} {raw:?}: {err}"))?,
                );
            }
            "--min-block" => {
                let raw = value_at(args, &mut index, &flag)?;
                parsed.min_block = raw
                    .parse()
                    .map_err(|err| format!("{flag} {raw:?}: {err}"))?;
            }
            "--manifest-hash" => parsed.manifest_hash = Some(value_at(args, &mut index, &flag)?),
            "--state-version" => {
                let raw = value_at(args, &mut index, &flag)?;
                parsed.state_version = Some(
                    raw.parse()
                        .map_err(|err| format!("{flag} {raw:?}: {err}"))?,
                );
            }
            "--force" => parsed.force = true,
            other => return Err(format!("unexpected argument {other:?}")),
        }
        index += 1;
    }

    Ok(parsed)
}

/// Write the spec through a temporary file and one rename.
///
/// A chain spec that exists under its final name is a chain spec an operator
/// will hand to a node, so it must never be observable half-written.
fn write_spec_atomically(path: &Path, spec: &serde_json::Value, force: bool) -> Result<(), String> {
    if path.exists() && !force {
        return Err(format!(
            "{} already exists; refusing to overwrite a chain spec (pass --force to replace it)",
            path.display()
        ));
    }

    let partial = PathBuf::from(format!("{}.partial", path.display()));
    let encoded = serde_json::to_string_pretty(spec)
        .map_err(|err| format!("cannot serialise the restored spec: {err}"))?;
    std::fs::write(&partial, format!("{encoded}\n"))
        .map_err(|err| format!("cannot write {}: {err}", partial.display()))?;
    std::fs::rename(&partial, path).map_err(|err| {
        let _ = std::fs::remove_file(&partial);
        format!(
            "cannot move {} to {}: {err}",
            partial.display(),
            path.display()
        )
    })
}

fn restore(args: &[String]) -> ExitCode {
    let parsed = match parse_restore_args(args) {
        Ok(parsed) => parsed,
        Err(message) => return fail_usage(&message),
    };

    let Some(manifest_path) = parsed.manifest.as_deref() else {
        return fail_usage("restore needs --manifest");
    };
    let Some(chunks_dir) = parsed.chunks.as_deref() else {
        return fail_usage("restore needs --chunks");
    };
    let Some(out_path) = parsed.out.as_deref() else {
        return fail_usage("restore needs --out");
    };

    // Unlike `verify`, restore has no self-consistency-only mode: the bytes it
    // produces become a validator's database, so the chain, block, root and
    // runtime all have to come from consensus rather than from the snapshot.
    let (chain_id, block_hash, state_root, runtime_version) =
        match (
            parsed.chain_id.as_deref(),
            parsed.block_hash.as_deref(),
            parsed.state_root.as_deref(),
            parsed.runtime_version,
        ) {
            (Some(chain), Some(block), Some(root), Some(version)) => (chain, block, root, version),
            _ => return fail_usage(
                "restore needs --chain-id, --block-hash, --state-root and --runtime-version; a \
                 restore that cannot name the block it is rebuilding is not a restore",
            ),
        };

    let manifest = match load_manifest(manifest_path) {
        Ok(manifest) => manifest,
        Err(message) => {
            eprintln!("error: {message}");
            return ExitCode::from(1);
        }
    };

    let template = match parsed.from_spec.as_deref() {
        Some(path) => match std::fs::read_to_string(path) {
            Ok(raw) => match serde_json::from_str::<serde_json::Value>(&raw) {
                Ok(spec) => Some(spec),
                Err(err) => {
                    eprintln!("error: cannot parse {}: {err}", path.display());
                    return ExitCode::from(1);
                }
            },
            Err(err) => {
                eprintln!("error: cannot read {}: {err}", path.display());
                return ExitCode::from(1);
            }
        },
        None => None,
    };

    let chunks = match read_chunks(chunks_dir, manifest.chunk_count) {
        Ok(chunks) => chunks,
        Err(message) => {
            eprintln!("error: {message}");
            return ExitCode::from(1);
        }
    };

    let version = match TrieVersion::from_u8(parsed.state_version.unwrap_or(1)) {
        Ok(version) => version,
        Err(err) => {
            eprintln!("error: {err}");
            return ExitCode::from(1);
        }
    };

    let anchor = TrustedAnchor {
        chain_id,
        block_hash,
        state_root,
        runtime_spec_version: runtime_version,
        minimum_block_number: parsed.min_block,
        manifest_hash: parsed.manifest_hash.as_deref(),
    };

    let request = RestoreRequest {
        manifest: &manifest,
        anchor: &anchor,
        chunks: &chunks,
        version,
        chain_name: parsed
            .name
            .clone()
            .unwrap_or_else(|| format!("X3 restored state at block {}", manifest.block_number)),
        spec_id: parsed
            .id
            .clone()
            .unwrap_or_else(|| manifest.chain_id.clone()),
        template: template.as_ref(),
    };

    let (spec, provenance) = match restore_snapshot(&request) {
        Ok(restored) => restored,
        Err(err) => {
            eprintln!("refused: {err}");
            return ExitCode::from(1);
        }
    };

    if let Err(message) = write_spec_atomically(out_path, &spec, parsed.force) {
        eprintln!("error: {message}");
        return ExitCode::from(1);
    }

    println!(
        "restored {} entries from {} chunk(s) into {}",
        provenance.state_entries,
        provenance.chunk_count,
        out_path.display()
    );
    println!(
        "  chain {} at block {} (runtime spec_version {}, trie layout {})",
        provenance.chain_id,
        provenance.block_number,
        provenance.runtime_spec_version,
        provenance.trie_layout
    );
    println!("  block hash  {}", provenance.block_hash);
    println!(
        "  state root  {} (recomputed from the snapshot's own bytes)",
        provenance.state_root
    );
    println!("  manifest    {}", provenance.manifest_hash);
    println!(
        "  boot a node with `--chain {}` to build a database from this state",
        out_path.display()
    );

    ExitCode::SUCCESS
}

fn parse_verify_args(args: &[String]) -> Result<VerifyArgs, String> {
    let mut parsed = VerifyArgs::default();
    let mut index = 0;

    while index < args.len() {
        let flag = args[index].clone();
        match flag.as_str() {
            "--manifest" => {
                parsed.manifest = Some(PathBuf::from(value_at(args, &mut index, &flag)?));
            }
            "--chunks" => {
                parsed.chunks = Some(PathBuf::from(value_at(args, &mut index, &flag)?));
            }
            "--chain-id" => parsed.chain_id = Some(value_at(args, &mut index, &flag)?),
            "--block-hash" => parsed.block_hash = Some(value_at(args, &mut index, &flag)?),
            "--state-root" => parsed.state_root = Some(value_at(args, &mut index, &flag)?),
            "--runtime-version" => {
                let raw = value_at(args, &mut index, &flag)?;
                parsed.runtime_version = Some(
                    raw.parse()
                        .map_err(|err| format!("{flag} {raw:?}: {err}"))?,
                );
            }
            "--min-block" => {
                let raw = value_at(args, &mut index, &flag)?;
                parsed.min_block = raw
                    .parse()
                    .map_err(|err| format!("{flag} {raw:?}: {err}"))?;
            }
            "--manifest-hash" => parsed.manifest_hash = Some(value_at(args, &mut index, &flag)?),
            "--state-version" => {
                let raw = value_at(args, &mut index, &flag)?;
                parsed.state_version = Some(
                    raw.parse()
                        .map_err(|err| format!("{flag} {raw:?}: {err}"))?,
                );
            }
            other => return Err(format!("unexpected argument {other:?}")),
        }
        index += 1;
    }

    Ok(parsed)
}

fn read_chunks(dir: &Path, count: u64) -> Result<Vec<Option<Vec<u8>>>, String> {
    let mut chunks = Vec::with_capacity(count as usize);
    for index in 0..count {
        let path = dir.join(format!("{index}.chunk"));
        match std::fs::read(&path) {
            Ok(bytes) => chunks.push(Some(bytes)),
            // A chunk file that is not there at all is reported as missing so
            // the verifier produces its own "incomplete snapshot" verdict.
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => chunks.push(None),
            Err(err) => return Err(format!("cannot read chunk {}: {err}", path.display())),
        }
    }
    Ok(chunks)
}

fn verify(args: &[String]) -> ExitCode {
    let parsed = match parse_verify_args(args) {
        Ok(parsed) => parsed,
        Err(message) => return fail_usage(&message),
    };

    let Some(manifest_path) = parsed.manifest.as_deref() else {
        return fail_usage("verify needs --manifest");
    };
    let Some(chunks_dir) = parsed.chunks.as_deref() else {
        return fail_usage("verify needs --chunks");
    };

    let manifest = match load_manifest(manifest_path) {
        Ok(manifest) => manifest,
        Err(message) => {
            eprintln!("error: {message}");
            return ExitCode::from(1);
        }
    };

    // Without an anchor the caller is only checking self-consistency, which is
    // still worth doing but must not look like a trust decision.
    let (chain_id, block_hash, state_root, runtime_version) =
        match (
            parsed.chain_id.as_deref(),
            parsed.block_hash.as_deref(),
            parsed.state_root.as_deref(),
            parsed.runtime_version,
        ) {
            (Some(chain), Some(block), Some(root), Some(version)) => (chain, block, root, version),
            (None, None, None, None) => {
                eprintln!(
                    "warning: no --chain-id/--block-hash/--state-root/--runtime-version given; \
                 checking chunk integrity, internal consistency and that the state root \
                 recomputed from the snapshot's own bytes matches the manifest. This is NOT \
                 a statement that the snapshot matches the chain — pass the finalized anchor \
                 from consensus to get that."
                );
                (
                    manifest.chain_id.as_str(),
                    manifest.block_hash.as_str(),
                    manifest.state_root.as_str(),
                    manifest.runtime_spec_version,
                )
            }
            _ => return fail_usage(
                "the anchor is all-or-nothing: give --chain-id, --block-hash, --state-root and \
                 --runtime-version together, or none of them",
            ),
        };

    let chunks = match read_chunks(chunks_dir, manifest.chunk_count) {
        Ok(chunks) => chunks,
        Err(message) => {
            eprintln!("error: {message}");
            return ExitCode::from(1);
        }
    };

    let anchor = TrustedAnchor {
        chain_id,
        block_hash,
        state_root,
        runtime_spec_version: runtime_version,
        minimum_block_number: parsed.min_block,
        manifest_hash: parsed.manifest_hash.as_deref(),
    };

    let version = match TrieVersion::from_u8(parsed.state_version.unwrap_or(1)) {
        Ok(version) => version,
        Err(err) => {
            eprintln!("error: {err}");
            return ExitCode::from(1);
        }
    };

    match verify_snapshot_with_state_root(&manifest, &anchor, &chunks, version) {
        Ok(()) => {
            println!(
                "verified: {} chunk(s), block {}, state root {} recomputed from the snapshot's own \
                 bytes (trie layout {version:?})",
                manifest.chunk_count, manifest.block_number, manifest.state_root
            );
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("refused: {err}");
            ExitCode::from(1)
        }
    }
}
