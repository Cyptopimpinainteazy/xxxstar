//! Pipeline bridging functions connecting to actual X3 compiler APIs.

use anyhow::{Context, Result};

use x3_backend::bc_format::BytecodeModule;
use x3_backend::{BytecodeCompiler, MirBytecodeCompiler};
use x3_hir::hir::HirModule;
use x3_hir::HirLowerer;
use x3_mir::{MirLowerer, MirModule};
use x3_opt::ssa_lite::global_ssa_opt;
use x3_opt::telemetry::{RunTelemetry, TelemetryObserver};
use x3_opt::{OptLevel, Optimizer, PassObserver};
use x3_parser::parse_program;
use x3_vm::verifier::{opcode_gas_cost, Verifier, VerifyOptions};

/// Full pipeline stats.
#[derive(Debug, Clone, Default)]
pub struct PipelineStats {
    pub instruction_count: usize,
    pub gas_estimate: u64,
    pub bytecode_size: usize,
}

/// Result from compiling through the pipeline.
pub struct CompileResult {
    pub hir: HirModule,
    pub mir: MirModule,
    pub bytecode_module: BytecodeModule,
    pub bytecode_bytes: Vec<u8>,
    pub stats: PipelineStats,
}

// ============================================================================
// Bridge Functions - Connect to real X3 APIs
// ============================================================================

/// Parse X3 source to AST, then lower to HIR.
pub fn bridge_parse_to_hir(source: &str) -> Result<HirModule> {
    // Parse source → AST
    let ast = parse_program(source).context("Failed to parse source")?;

    // Lower AST → HIR
    let hir = HirLowerer::lower(ast).context("Failed to lower AST to HIR")?;

    Ok(hir)
}

/// Lower HIR to MIR.
pub fn bridge_hir_to_mir(hir: &HirModule) -> Result<MirModule> {
    let mir = MirLowerer::lower(hir).context("Failed to lower HIR to MIR")?;
    Ok(mir)
}

/// Compile HIR to bytecode module.
pub fn bridge_hir_to_bytecode(hir: &HirModule) -> Result<BytecodeModule> {
    let bytecode = BytecodeCompiler::compile(hir).context("Failed to compile HIR to bytecode")?;
    Ok(bytecode)
}

/// Compile MIR to bytecode module (uses optimized MIR).
pub fn bridge_mir_to_bytecode(mir: &MirModule) -> Result<BytecodeModule> {
    let bytecode =
        MirBytecodeCompiler::compile(mir).context("Failed to compile MIR to bytecode")?;
    Ok(bytecode)
}

/// Serialize bytecode module to bytes.
pub fn bridge_bytecode_to_bytes(module: &BytecodeModule) -> Vec<u8> {
    module.to_bytes()
}

/// Run optimizer passes on MIR in-place with optional observer.
pub fn bridge_optimize_mir(
    optimizer: &Optimizer,
    mir: &mut MirModule,
    observer: Option<&mut dyn PassObserver>,
) -> Result<()> {
    if let Some(obs) = observer {
        optimizer
            .run_with_observer(mir, obs)
            .context("Optimizer failed")?;
    } else {
        optimizer.run(mir).context("Optimizer failed")?;
    }
    Ok(())
}

/// Count instructions in bytecode using the verifier's decoder.
pub fn bridge_count_instructions(bytecode_bytes: &[u8]) -> Result<usize> {
    // Use the verifier to decode instructions
    let options = VerifyOptions::default();

    // Try to parse the module and get decoded instructions
    match Verifier::verify_module_bytes(bytecode_bytes, &options) {
        Ok(module) => {
            // Count instructions by decoding the code section
            let code = &module.code;
            let count = count_instructions_in_code(code);
            Ok(count)
        }
        Err(_) => {
            // If verification fails, try to count raw opcodes
            // This is a fallback - just count non-zero bytes as a rough estimate
            Ok(bytecode_bytes.iter().filter(|&&b| b != 0).count() / 3)
        }
    }
}

/// Count instructions by walking the code bytes.
fn count_instructions_in_code(code: &[u8]) -> usize {
    use x3_backend::opcode::Opcode;

    let mut count = 0;
    let mut ip = 0;

    while ip < code.len() {
        let opcode_byte = code[ip];

        // Try to decode the opcode
        if let Some(opcode) = Opcode::from_byte(opcode_byte) {
            count += 1;
            // Get instruction size and advance
            let size = get_instruction_size(opcode, &code[ip..]);
            ip += size;
        } else {
            // Unknown opcode, skip one byte
            ip += 1;
        }
    }

    count
}

/// Encoded size of the instruction at the start of `code`, from the canonical opcode spec
/// (`x3bc_core::spec`), so this walker cannot drift from the decoder. A truncated instruction
/// consumes the rest of the code.
fn get_instruction_size(opcode: x3_backend::opcode::Opcode, code: &[u8]) -> usize {
    x3bc_core::spec::of(opcode)
        .encoded_len(code)
        .unwrap_or(code.len().max(1))
}

/// Simulate gas by summing instruction gas costs.
pub fn bridge_simulate_gas(bytecode_bytes: &[u8]) -> Result<u64> {
    use x3_backend::opcode::Opcode;

    let mut total_gas: u64 = 0;
    let mut ip = 0;

    // Try to parse actual bytecode module first
    let options = VerifyOptions::default();
    if let Ok(module) = Verifier::verify_module_bytes(bytecode_bytes, &options) {
        let code = &module.code;

        while ip < code.len() {
            let opcode_byte = code[ip];
            total_gas += opcode_gas_cost(opcode_byte);

            if let Some(opcode) = Opcode::from_byte(opcode_byte) {
                ip += get_instruction_size(opcode, &code[ip..]);
            } else {
                ip += 1;
            }
        }
    } else {
        // Fallback: estimate based on bytecode size
        // Assume average instruction size of 4 bytes, average gas cost of 3
        total_gas = (bytecode_bytes.len() as u64 / 4) * 3;
    }

    Ok(total_gas)
}

/// Full compilation pipeline without optimization.
pub fn compile_unoptimized(source: &str) -> Result<CompileResult> {
    // Parse → HIR
    let hir = bridge_parse_to_hir(source)?;

    // HIR → MIR (keep copy for stats, though we don't use it for unoptimized path)
    let mir = bridge_hir_to_mir(&hir)?;

    // HIR → Bytecode (directly, no MIR optimization)
    let bytecode_module = bridge_hir_to_bytecode(&hir)?;
    let bytecode_bytes = bridge_bytecode_to_bytes(&bytecode_module);

    // Calculate stats
    let instruction_count = bridge_count_instructions(&bytecode_bytes).unwrap_or(0);
    let gas_estimate = bridge_simulate_gas(&bytecode_bytes).unwrap_or(0);
    let bytecode_size = bytecode_bytes.len();

    Ok(CompileResult {
        hir,
        mir,
        bytecode_module,
        bytecode_bytes,
        stats: PipelineStats {
            instruction_count,
            gas_estimate,
            bytecode_size,
        },
    })
}

/// Full compilation pipeline with MIR optimization.
///
/// This pipeline runs the optimizer on MIR, then compiles the optimized
/// MIR directly to bytecode using the MirBytecodeCompiler.
pub fn compile_optimized(
    source: &str,
    max_opt_iters: usize,
    telemetry: Option<&mut RunTelemetry>,
    sample_name: Option<&str>,
) -> Result<CompileResult> {
    // Parse → HIR
    let hir = bridge_parse_to_hir(source)?;

    // HIR → MIR
    let mut mir = bridge_hir_to_mir(&hir)?;

    for func in &mut mir.functions {
        *func = global_ssa_opt(func);
    }

    let mut optimizer = Optimizer::new(OptLevel::Aggressive).with_max_iterations(max_opt_iters);

    if let Some(tx) = telemetry {
        let order: Vec<String> = optimizer
            .pass_names()
            .into_iter()
            .map(String::from)
            .collect();
        tx.set_pass_order(order);
        let sample = sample_name.unwrap_or("sample");
        let mut observer = TelemetryObserver::new(tx, sample);
        bridge_optimize_mir(&optimizer, &mut mir, Some(&mut observer))?;
    } else {
        bridge_optimize_mir(&optimizer, &mut mir, None)?;
    }

    // MIR → Bytecode (using the new MIR lowerer!)
    let bytecode_module = bridge_mir_to_bytecode(&mir)?;
    let bytecode_bytes = bridge_bytecode_to_bytes(&bytecode_module);

    // Calculate stats from bytecode
    let instruction_count = bridge_count_instructions(&bytecode_bytes).unwrap_or(0);
    let gas_estimate = bridge_simulate_gas(&bytecode_bytes).unwrap_or(0);
    let bytecode_size = bytecode_bytes.len();

    Ok(CompileResult {
        hir,
        mir,
        bytecode_module,
        bytecode_bytes,
        stats: PipelineStats {
            instruction_count,
            gas_estimate,
            bytecode_size,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SIMPLE_SOURCE: &str = r#"
fn main() -> i64 {
    let x = 1 + 2;
    return x;
}
"#;

    /// The measured instruction count is the production decoder's count: the walker reads widths
    /// from the canonical spec, not from a table of its own.
    #[test]
    fn the_instruction_count_matches_the_production_decoder() {
        for source in [
            SIMPLE_SOURCE,
            "fn f(a: i64, b: i64) -> i64 { if a > b { return a - b; } return f(b, a); }",
        ] {
            let compiled = compile_unoptimized(source).expect("compiles");
            let decoded = Verifier::decode_all_instructions(&compiled.bytecode_module.code)
                .expect("the production decoder reads it");
            assert_eq!(
                bridge_count_instructions(&compiled.bytecode_bytes).expect("counts"),
                decoded.len()
            );
        }
    }

    #[test]
    fn test_parse_to_hir() {
        let result = bridge_parse_to_hir(SIMPLE_SOURCE);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
    }

    #[test]
    fn test_hir_to_mir() {
        let hir = bridge_parse_to_hir(SIMPLE_SOURCE).unwrap();
        let result = bridge_hir_to_mir(&hir);
        assert!(result.is_ok(), "Failed to lower to MIR: {:?}", result.err());
    }

    #[test]
    fn test_compile_unoptimized() {
        let result = compile_unoptimized(SIMPLE_SOURCE);
        assert!(result.is_ok(), "Compile failed: {:?}", result.err());

        let res = result.unwrap();
        assert!(res.stats.bytecode_size > 0);
    }

    #[test]
    fn test_compile_optimized() {
        let result = compile_optimized(SIMPLE_SOURCE, 3, None, None);
        assert!(result.is_ok(), "Compile failed: {:?}", result.err());
    }

    /// The blocker this answers: the tests above assert the pipeline *completes*,
    /// not that a number is right, so a pipeline that compiled and measured
    /// nothing would pass them. This asserts the measurement moves the way the
    /// optimizer is supposed to move it — on a sample with redundant work,
    /// optimizing produces fewer instructions than not optimizing — and that the
    /// reported numbers are internally consistent.
    #[test]
    fn the_optimizer_moves_the_measured_instruction_count_down() {
        const REDUNDANT: &str = r#"
fn sum() -> i64 {
    let a = 1 + 2;
    let b = a + 0;
    let c = b * 1;
    let d = c + (2 + 3);
    let e = d - 0;
    return e;
}
"#;
        let baseline = compile_unoptimized(REDUNDANT).expect("the baseline compiles");
        let optimized =
            compile_optimized(REDUNDANT, 6, None, None).expect("the optimized build compiles");

        // A measurement of zero is not a measurement: the pipeline must have
        // emitted instructions, charged for them and produced bytes.
        for (label, stats) in [
            ("baseline", &baseline.stats),
            ("optimized", &optimized.stats),
        ] {
            assert!(
                stats.instruction_count > 0,
                "{label} reported no instructions"
            );
            assert!(stats.gas_estimate > 0, "{label} reported no gas");
            assert!(stats.bytecode_size > 0, "{label} reported no bytes");
        }
        // `stats.bytecode_size` has to be the length of the bytes that result
        // actually produced, or the published number is a claim about nothing.
        assert_eq!(
            baseline.stats.bytecode_size,
            baseline.bytecode_bytes.len(),
            "the baseline's byte count does not match its bytes"
        );
        assert_eq!(
            optimized.stats.bytecode_size,
            optimized.bytecode_bytes.len(),
            "the optimized build's byte count does not match its bytes"
        );

        assert!(
            optimized.stats.instruction_count < baseline.stats.instruction_count,
            "the optimizer did not reduce the instruction count: {} -> {}",
            baseline.stats.instruction_count,
            optimized.stats.instruction_count
        );
        assert!(
            optimized.stats.bytecode_size <= baseline.stats.bytecode_size,
            "optimizing grew the bytecode: {} -> {}",
            baseline.stats.bytecode_size,
            optimized.stats.bytecode_size
        );
    }
}
