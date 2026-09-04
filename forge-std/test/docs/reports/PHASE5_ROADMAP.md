# Phase 5 Roadmap: CLI & Node Integration

## 🎯 Objective

Wire the x3-compiler integration into the X3 Chain node and CLI tools so that:
1. Users can specify optimization level when compiling contracts
2. Node uses configured optimization level for all contract compilations
3. RPC endpoints pass through optimization preferences
4. Real smart contracts show measured gas/bytecode reduction

## 📋 Tasks for Phase 5

### Task 1: CLI Integration (Estimated: 30-45 min)

**File**: `crates/x3-cli/src/main.rs`

**Changes**:
```rust
// Add to compile subcommand
#[derive(Parser)]
pub struct CompileCmd {
    /// Input file path
    #[arg(value_name = "FILE")]
    input: PathBuf,
    
    /// Output bytecode path
    #[arg(short, long)]
    output: Option<PathBuf>,
    
    /// Optimization level: 0, 1, 2 (default), 3
    #[arg(short = 'O', long, value_parser = parse_opt_level)]
    opt_level: Option<OptLevel>,  // ← NEW
    
    /// Debug info
    #[arg(short, long)]
    debug: bool,
    
    /// Verbose output
    #[arg(short, long)]
    verbose: bool,
}

// Usage:
// x3 compile contract.x3 -O 2        (O2, recommended)
// x3 compile contract.x3 -O 3        (O3, aggressive)
// x3 compile contract.x3             (defaults to O2)
```

**Verification**:
```bash
cargo build -p x3-cli
./target/debug/x3 compile --help  # Should show --opt-level flag
./target/debug/x3 compile test.x3 -O 3  # Should work
```

### Task 2: Node Configuration (Estimated: 20-30 min)

**File**: `runtime/src/lib.rs`

**Changes**:
```rust
// Add to runtime config
pub struct CompilerConfig {
    pub optimization_level: OptLevel,
    pub debug_info: bool,
}

impl Default for CompilerConfig {
    fn default() -> Self {
        Self {
            optimization_level: OptLevel::Default,  // O2
            debug_info: false,
        }
    }
}

// Wire into runtime initialization
impl RuntimeConfig {
    pub fn new() -> Self {
        Self {
            compiler: CompilerConfig::default(),
            // ... other fields
        }
    }
}
```

**Verification**:
- Runtime compiles with new config
- Default optimization level is O2
- Can override at node startup

### Task 3: RPC Integration (Estimated: 40-60 min)

**File**: `node/src/rpc.rs`

**Changes**:
```rust
// Update deploy contract RPC method
pub fn deploy_contract(
    &mut self,
    wasm_bytes: Vec<u8>,
    opt_level: Option<OptLevel>,  // ← NEW PARAMETER
) -> RpcResult<DeploymentResult> {
    let opt = opt_level.unwrap_or(OptLevel::Default);
    
    // 1. Parse and check contract
    let hir = parse_and_check_contract(&wasm_bytes)?;
    
    // 2. Lower to MIR
    let mir = lower_hir_to_mir(&hir)?;
    
    // 3. Compile with optimization ← NEW
    let bytecode = Compiler::compile_mir(
        &mir,
        CompilationOptions {
            opt_level: opt,
            debug: self.config.compiler.debug_info,
            verbose: self.config.verbose,
        }
    ).map_err(|e| RpcError::internal(format!("Compilation failed: {}", e)))?;
    
    // 4. Store on-chain
    self.store_contract(&bytecode)?;
    
    Ok(DeploymentResult { /* ... */ })
}

// Update RPC method signature in JSON-RPC handler
// "eth_deployContract" or "svm_deployContract" or "atlasKernel_deployContract"
```

**Verification**:
- RPC endpoint accepts optimization level parameter
- Passes parameter through to Compiler::compile_mir
- Defaults to OptLevel::Default if not specified

### Task 4: End-to-End Test (Estimated: 30-45 min)

**Create**: `tests/optimization_e2e.rs`

**Test**:
```rust
#[tokio::test]
async fn test_contract_optimization_e2e() {
    // 1. Start blockchain node
    let node = start_test_node(OptLevel::Default);
    
    // 2. Deploy test contract
    let contract = deploy_test_contract(&node, OptLevel::Default)?;
    
    // 3. Compile same contract with O0
    let baseline = deploy_test_contract(&node, OptLevel::None)?;
    
    // 4. Compare bytecode sizes
    let optimized_size = contract.bytecode.len();
    let baseline_size = baseline.bytecode.len();
    
    let reduction = (baseline_size - optimized_size) as f64 / baseline_size as f64;
    
    // 5. Assert reduction is measurable (at least 5%)
    assert!(reduction > 0.05, "Expected >5% reduction, got {}%", reduction * 100.0);
    
    println!("✓ Bytecode reduction: {:.1}%", reduction * 100.0);
}
```

**Verification**:
- Deploy contract with different optimization levels
- Measure bytecode size differences
- Assert expected gas reduction

### Task 5: Performance Benchmarking (Estimated: 60-90 min)

**Create**: `benches/blockchain_optimization_bench.rs`

**Benchmark**:
```rust
// For various contract types:
// - Simple ERC-20-like token
// - Complex DeFi logic
// - Loop-heavy computation
// - Mixed patterns

#[bench]
fn bench_erc20_compilation(b: &mut Bencher) {
    let source = include_str!("test_contracts/erc20.x3");
    
    b.iter(|| {
        Compiler::compile_mir(
            &mir,
            CompilationOptions::opt2()
        )
    });
}
```

**Reporting**:
- Create OPTIMIZATION_RESULTS.md with:
  - Bytecode size reduction per contract type
  - Compilation time comparison
  - Gas cost estimation
  - Network bandwidth savings

## 📊 Success Criteria (Phase 5)

- ✅ CLI accepts `--opt-level` / `-O` flag
- ✅ Node starts with configurable compiler settings
- ✅ RPC endpoints pass through optimization preference
- ✅ End-to-end test shows measurable bytecode reduction
- ✅ Real smart contracts compiled with visible gas savings
- ✅ Documentation updated with usage examples

## 🔗 Dependencies Between Tasks

```
Task 1 (CLI)                ← Independent, can start first
Task 2 (Node Config)        ← Can start in parallel with Task 1
Task 3 (RPC Integration)    ← Depends on Task 2 being available
Task 4 (E2E Test)           ← Depends on Tasks 1-3
Task 5 (Benchmarking)       ← Depends on Task 4 working
```

**Recommended Order**:
1. Start Tasks 1 & 2 in parallel
2. When both ready, do Task 3
3. Then Task 4 (E2E test)
4. Finally Task 5 (comprehensive benchmarking)

## 📦 What's Already Available

From Phase 4:
- ✅ `crates/x3-compiler/` - Fully implemented and tested
- ✅ `Compiler::compile_mir()` - Public API ready to use
- ✅ `CompilationOptions` - Config system ready
- ✅ `OptLevel` enum - O0/O1/O2/O3 levels
- ✅ Documentation - Architecture guide included

From Phase 3:
- ✅ `x3-opt` - 14-pass optimizer (110 tests passing)
- ✅ `Benchmark harness` - Gas reduction measured (20-50%)
- ✅ Test suite - Comprehensive (119 tests)

## 💡 Implementation Tips

### For CLI Integration
- Use `clap::ValueEnum` for OptLevel parsing
- Default to OptLevel::Default when not specified
- Show optimization level in verbose output

### For Node Config
- Use environment variable fallback (e.g., `X3_OPT_LEVEL`)
- Support both short form (0-3) and long form (none, basic, default, aggressive)
- Log chosen optimization level at startup

### For RPC Integration
- Make optimization level optional in RPC call
- Use node's default if not specified
- Return optimization stats in response if verbose mode

### For Testing
- Use simple test contracts first (loop patterns)
- Graduate to complex contracts (DeFi)
- Measure both time and space (bytecode size)

## 📚 Reference Code

Available in workspace:
```
crates/x3-cli/              ← Where to add --opt-level
runtime/src/lib.rs         ← Where to add CompilerConfig
node/src/rpc.rs            ← Where to integrate Compiler
tests/                      ← Where to add E2E tests
benches/                    ← Where to add performance benchmarks
```

## 🎯 Phase 5 Success Looks Like

```
$ x3 compile --help
  -O, --opt-level <OPT_LEVEL>  Optimization level: 0, 1, 2 (default), 3

$ x3 compile contract.x3 -O 3
🔧 Compiling with optimization level: Aggressive
  → 14 passes, up to 20 iterations
  📊 Optimization stats:
     • Passes executed: 14
     • Passes changed code: 12
     • Total transformations: 247
     • Iterations to fixpoint: 3
  ✓ Optimization complete
  ✓ Bytecode generation complete
✨ Compiled: contract.wasm (1.2 KB, 42.3% smaller than baseline)
```

## 🚀 After Phase 5

Once CLI/node integration is complete:
1. Document usage patterns
2. Create example contracts
3. Publish performance metrics
4. Consider upstream of YOLO/Loop-Pack to Substrate ecosystem
5. Plan Phase 6: Adaptive optimization selection

---

## 📖 Quick Reference

### Key Files to Modify
- `crates/x3-cli/src/main.rs` - Add CLI flags
- `runtime/src/lib.rs` - Add node config
- `node/src/rpc.rs` - Add RPC integration

### Key Functions to Call
```rust
Compiler::compile_mir(mir, options)  // From x3-compiler
```

### Key Types to Use
```rust
CompilationOptions                   // Configure optimization
OptLevel { None, Basic, Default, Aggressive }
```

### Testing Commands
```bash
cargo build -p x3-cli
cargo test  # Should include new E2E tests
cargo bench # Should show compilation benchmarks
```

---

**Status**: Ready for Phase 5
**Estimated Duration**: 3-5 hours
**Start Condition**: Phase 4 complete (✅ done)
