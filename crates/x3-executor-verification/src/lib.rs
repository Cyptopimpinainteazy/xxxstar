//! Execution gates for the production Wasmtime executor; no runtime component.
#![cfg(test)]

use sc_executor_wasmtime::{
    create_runtime, Config, HeapAllocStrategy, InstantiationStrategy, RuntimeBlob, Semantics,
    WasmModule,
};

fn strategies() -> [InstantiationStrategy; 4] {
    [
        InstantiationStrategy::RecreateInstanceCopyOnWrite,
        InstantiationStrategy::RecreateInstance,
        InstantiationStrategy::PoolingCopyOnWrite,
        InstantiationStrategy::Pooling,
    ]
}

fn execute(body: &str, strategy: InstantiationStrategy) -> Result<Vec<u8>, String> {
    let code = wat::parse_str(format!(
        r#"(module
        (memory (export "memory") 1 4)
        (global (export "__heap_base") i32 (i32.const 16))
        (data (i32.const 0) "ok")
        (func (export "main") (param i32 i32) (result i64) {body}))"#
    ))
    .unwrap();
    let blob = RuntimeBlob::uncompress_if_needed(&code).unwrap();
    let config = Config {
        allow_missing_func_imports: false,
        cache_path: None,
        semantics: Semantics {
            instantiation_strategy: strategy,
            deterministic_stack_limit: None,
            canonicalize_nans: true,
            parallel_compilation: false,
            heap_alloc_strategy: HeapAllocStrategy::Dynamic {
                maximum_pages: Some(4),
            },
            wasm_multi_value: false,
            wasm_bulk_memory: false,
            wasm_reference_types: false,
            wasm_simd: false,
        },
    };
    let runtime = create_runtime::<()>(blob, config).unwrap();
    let mut instance = runtime.new_instance().unwrap();
    instance
        .call_export("main", &[])
        .map_err(|error| format!("{error:?}"))
}

#[test]
fn all_instantiation_strategies_execute_actual_wasm() {
    for strategy in strategies() {
        assert_eq!(execute("i64.const 8589934592", strategy).unwrap(), b"ok");
    }
}

#[test]
fn all_instantiation_strategies_trap_out_of_bounds_loads() {
    for strategy in strategies() {
        let error = execute("i32.const 2147483647 i32.load drop i64.const 0", strategy)
            .expect_err("an out-of-bounds load must trap");
        assert!(error.contains("AbortedDueToTrap"), "{error}");
    }
}

#[test]
fn nan_payload_is_canonical_across_instantiation_strategies() {
    let body = r#"(i32.store (i32.const 0)
        (i32.reinterpret_f32 (f32.add
            (f32.reinterpret_i32 (i32.const 0x7f812345)) (f32.const 1))))
        i64.const 17179869184"#;
    for strategy in strategies() {
        let result = execute(body, strategy).unwrap();
        assert_eq!(result, 0x7fc00000_u32.to_le_bytes());
    }
}
