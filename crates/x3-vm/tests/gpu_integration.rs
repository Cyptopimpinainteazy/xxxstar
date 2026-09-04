//! Integration tests: X3 VM → GPU CUDA Hostcall Bridge
//!
//! Tests the full pipeline:
//!   X3 bytecode → VM dispatch (0xD0-0xD5) → HostcallRegistry → gpu_hostcalls → CUDA FFI
//!
//! These tests require CUDA .so libraries to be present at runtime.
//! If libraries are not found, hostcalls return errors (which we test for).
//!
//! Invariant: INV-GPU-001 (GPU hostcall dispatch correctness)

use x3_backend::bc_format::*;
use x3_backend::opcode::Opcode;
use x3_vm::{GpuHostcalls, Value, VM};

/// Build a minimal BytecodeModule with given code and one function at offset 0.
fn make_gpu_module(code: Vec<u8>) -> BytecodeModule {
    BytecodeModule {
        version: VersionInfo::new(1, 0, 0),
        min_version: VersionInfo::new(1, 0, 0),
        flags: ModuleFlags::default(),
        features: FeatureFlags(0),
        const_pool: ConstPool::new(),
        functions: vec![FunctionEntry {
            name: "main".into(),
            entry_point: 0,
            param_count: 0,
            local_count: 0,
            max_stack: 0,
            return_type_tag: 1, // int
        }],
        globals: vec![],
        code,
        debug_info: None,
        metadata: None,
    }
}

/// Helper: Register GPU hostcalls on a VM.
fn register_gpu_on(vm: &mut VM) {
    let gpu = GpuHostcalls::new();
    gpu.register_on_vm(vm);
}

// ═══════════════════════════════════════════════════════════════════════
// Test 1: gpu_device_count opcode (0xD4)
//
// Bytecode:
//   0xD4 r0        ; r0 = gpu_device_count()
//   Ret  r0        ; return r0
// ═══════════════════════════════════════════════════════════════════════
#[test]
fn gpu_device_count_opcode() {
    // Encoding: [0xD4, dst:u8] [Ret, src:u8]
    let code = vec![
        Opcode::GpuDeviceCount.to_byte(),
        0x00, // r0 = gpu_device_count()
        Opcode::Ret.to_byte(),
        0x00, // return r0
    ];

    let module = make_gpu_module(code);
    let mut vm = VM::new(module);
    register_gpu_on(&mut vm);

    let result = match vm.call_function(0, &[]) {
        Ok(result) => result,
        Err(err) => {
            let err_text = format!("{err:?}").to_lowercase();
            if err_text.contains("invalid device ordinal")
                || err_text.contains("cuda error code -1")
            {
                println!(
                    "[GPU Test] Skipping real SHA-256 test — CUDA device unavailable: {err:?}"
                );
                return;
            }
            panic!("execution should succeed: {err:?}");
        }
    };

    // If CUDA libraries loaded, count should be > 0 (3 for the GTX 1070s)
    // If not loaded, the hostcall still returns a value (0)
    match result.value {
        Some(Value::I64(count)) => {
            println!("[GPU Test] Device count: {}", count);
            assert!(count >= 0, "device count should be non-negative");
        }
        other => {
            // On systems without GPUs, hostcall may error
            println!("[GPU Test] Device count result: {:?}", other);
        }
    }
    assert!(result.gas_used > 0, "should have used gas");
}

// ═══════════════════════════════════════════════════════════════════════
// Test 2: Verify GPU opcodes charge correct gas
// ═══════════════════════════════════════════════════════════════════════
#[test]
fn gpu_opcodes_charge_gas() {
    // GpuDeviceCount uses 10 gas, Ret uses 2 gas
    let code = vec![
        Opcode::GpuDeviceCount.to_byte(),
        0x00, // 10 gas
        Opcode::Ret.to_byte(),
        0x00, // 2 gas
    ];

    let module = make_gpu_module(code);
    let mut vm = VM::new(module);
    register_gpu_on(&mut vm);

    let result = match vm.call_function(0, &[]) {
        Ok(result) => result,
        Err(err) => {
            let err_text = format!("{err:?}").to_lowercase();
            if err_text.contains("invalid device ordinal")
                || err_text.contains("cuda error code -1")
            {
                println!(
                    "[GPU Test] Skipping real SHA-256 test — CUDA device unavailable: {err:?}"
                );
                return;
            }
            panic!("execution should succeed: {err:?}");
        }
    };
    // GpuDeviceCount = 10 gas, Ret = 2 gas = 12 total
    assert_eq!(
        result.gas_used, 12,
        "GPU device count should cost 10 gas + 2 for ret"
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Test 3: GPU SHA-256 batch opcode (0xD0) — dispatches to CUDA
//
// Bytecode:
//   LoadImm r1, 0      ; count = 0 (empty batch = safe on any system)
//   GpuSha256Batch r0, r1, r1  ; r0 = gpu_sha256_batch(r1=bytes, r1=count)
//   Ret r0
// ═══════════════════════════════════════════════════════════════════════
#[test]
fn gpu_sha256_batch_empty() {
    // For empty batch (count=0), the handler returns empty Bytes without calling CUDA.
    // This works even without GPU hardware.
    let code = vec![
        Opcode::GpuSha256Batch.to_byte(),
        0x00,
        0x01,
        0x02, // r0 = gpu_sha256_batch(r1, r2)
        Opcode::Ret.to_byte(),
        0x00,
    ];

    let module = make_gpu_module(code);
    let mut vm = VM::new(module);
    register_gpu_on(&mut vm);

    // Pre-set registers: r1 = empty Bytes, r2 = I64(0) [count = 0]
    vm.set_register(1, Value::Bytes(vec![]));
    vm.set_register(2, Value::I64(0));

    let result = vm.call_function(0, &[]).expect("execution should succeed");
    assert_eq!(result.value, Some(Value::Bytes(vec![])));
}

// ═══════════════════════════════════════════════════════════════════════
// Test 4: GPU Ed25519 verify empty batch (0xD1)
// ═══════════════════════════════════════════════════════════════════════
#[test]
fn gpu_ed25519_verify_empty() {
    let code = vec![
        Opcode::GpuEd25519Verify.to_byte(),
        0x00,
        0x01,
        0x02,
        Opcode::Ret.to_byte(),
        0x00,
    ];

    let module = make_gpu_module(code);
    let mut vm = VM::new(module);
    register_gpu_on(&mut vm);

    vm.set_register(1, Value::Bytes(vec![]));
    vm.set_register(2, Value::I64(0));

    let result = vm.call_function(0, &[]).expect("execution should succeed");
    assert_eq!(result.value, Some(Value::Bytes(vec![])));
}

// ═══════════════════════════════════════════════════════════════════════
// Test 5: GPU PoH chain empty (0xD2)
// ═══════════════════════════════════════════════════════════════════════
#[test]
fn gpu_poh_chain_empty() {
    let code = vec![
        Opcode::GpuPohChain.to_byte(),
        0x00,
        0x01,
        0x02,
        0x03,
        Opcode::Ret.to_byte(),
        0x00,
    ];

    let module = make_gpu_module(code);
    let mut vm = VM::new(module);
    register_gpu_on(&mut vm);

    vm.set_register(1, Value::Bytes(vec![]));
    vm.set_register(2, Value::I64(0));
    vm.set_register(3, Value::I64(0));

    let result = vm.call_function(0, &[]).expect("execution should succeed");
    assert_eq!(result.value, Some(Value::Bytes(vec![])));
}

// ═══════════════════════════════════════════════════════════════════════
// Test 6: Multiple GPU ops in sequence
// ═══════════════════════════════════════════════════════════════════════
#[test]
fn gpu_multi_op_sequence() {
    let code = vec![
        // r10 = gpu_device_count() → 2 bytes
        Opcode::GpuDeviceCount.to_byte(),
        0x0A,
        // r0 = gpu_sha256_batch(r1, r2) → 4 bytes
        Opcode::GpuSha256Batch.to_byte(),
        0x00,
        0x01,
        0x02,
        // r5 = gpu_ed25519_verify(r3, r4) → 4 bytes
        Opcode::GpuEd25519Verify.to_byte(),
        0x05,
        0x03,
        0x04,
        // return r10
        Opcode::Ret.to_byte(),
        0x0A,
    ];

    let module = make_gpu_module(code);
    let mut vm = VM::new(module);
    register_gpu_on(&mut vm);

    // Set up registers for empty batches
    vm.set_register(1, Value::Bytes(vec![]));
    vm.set_register(2, Value::I64(0));
    vm.set_register(3, Value::Bytes(vec![]));
    vm.set_register(4, Value::I64(0));

    let result = vm.call_function(0, &[]).expect("execution should succeed");

    // r10 should hold the device count
    match result.value {
        Some(Value::I64(count)) => {
            println!("[GPU Test] Multi-op: device count = {}", count);
            assert!(count >= 0);
        }
        other => panic!("Expected I64, got {:?}", other),
    }

    // Gas: GpuDeviceCount(10) + GpuSha256Batch(500) + GpuEd25519Verify(500) + Ret(2) = 1012
    assert_eq!(result.gas_used, 1012);
}

// ═══════════════════════════════════════════════════════════════════════
// Test 7: Hostcall registration (registry API)
// ═══════════════════════════════════════════════════════════════════════
#[test]
fn gpu_hostcalls_register_all() {
    use x3_vm::hostcall::HostcallRegistry;

    let gpu = GpuHostcalls::new();
    let mut registry = HostcallRegistry::new();
    gpu.register_all(&mut registry);

    // Should have all 6 GPU hostcalls registered
    // Invoke device count with no args
    let result = registry.invoke(0xD4, &[]);
    assert!(result.is_ok());
    let val = result.unwrap();
    match val {
        Some(Value::I64(count)) => {
            println!("[GPU Test] Registry API device count: {}", count);
        }
        other => panic!("Expected Some(I64), got {:?}", other),
    }
}

// ═══════════════════════════════════════════════════════════════════════
// Test 8: GPU SHA-256 batch with real data (only runs when CUDA libs available)
// ═══════════════════════════════════════════════════════════════════════
#[test]
fn gpu_sha256_batch_real_data() {
    let gpu = GpuHostcalls::new();
    if !gpu.is_available() {
        println!("[GPU Test] Skipping real SHA-256 test — no CUDA libraries found");
        return;
    }

    // Prepare 4 × 32-byte inputs
    let count = 4;
    let mut inputs = vec![0u8; count * 32];
    for i in 0..count {
        inputs[i * 32] = i as u8; // Different seed per block
    }

    let code = vec![
        Opcode::GpuSha256Batch.to_byte(),
        0x00,
        0x01,
        0x02,
        Opcode::Ret.to_byte(),
        0x00,
    ];

    let module = make_gpu_module(code);
    let mut vm = VM::new(module);
    gpu.register_on_vm(&mut vm);

    vm.set_register(1, Value::Bytes(inputs));
    vm.set_register(2, Value::I64(count as i64));

    let result = match vm.call_function(0, &[]) {
        Ok(result) => result,
        Err(err) => {
            let err_text = format!("{err:?}").to_lowercase();
            if err_text.contains("invalid device ordinal")
                || err_text.contains("cuda error code -1")
            {
                println!(
                    "[GPU Test] Skipping real SHA-256 test — CUDA device unavailable: {err:?}"
                );
                return;
            }
            panic!("execution should succeed: {err:?}");
        }
    };
    match result.value {
        Some(Value::Bytes(hashes)) => {
            assert_eq!(
                hashes.len(),
                count * 32,
                "should return {} bytes",
                count * 32
            );
            println!(
                "[GPU Test] SHA-256 batch first hash: {:02x?}",
                &hashes[..32]
            );
            // Verify hashes are non-zero (actual correctness tested in CUDA unit tests)
            assert!(
                hashes.iter().any(|&b| b != 0),
                "hashes should not be all zeros"
            );
        }
        other => panic!("Expected Bytes, got {:?}", other),
    }
}
