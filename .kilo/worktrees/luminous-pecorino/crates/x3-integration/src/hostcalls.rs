//! Hostcall implementations for X3 VM in Substrate context
//!
//! Hostcalls allow X3 programs to interact with:
//! - Substrate storage (read/write)
//! - Event emission
//! - Cross-VM communication (EVM/SVM bridge)
//! - Cryptographic primitives

#[cfg(not(feature = "std"))]
use alloc::{string::String, vec::Vec};

use sp_core::H256;

#[cfg(feature = "std")]
use std::sync::Arc;

/// Hostcall IDs for X3 VM
pub mod hostcall_ids {
    // Storage operations (0x00-0x0F)
    pub const STORAGE_READ: u32 = 0x00;
    pub const STORAGE_WRITE: u32 = 0x01;
    pub const STORAGE_DELETE: u32 = 0x02;
    pub const STORAGE_EXISTS: u32 = 0x03;

    // Event/logging (0x10-0x1F)
    pub const EMIT_EVENT: u32 = 0x10;
    pub const LOG_DEBUG: u32 = 0x11;

    // Crypto (0x20-0x2F)
    pub const BLAKE2_256: u32 = 0x20;
    pub const KECCAK_256: u32 = 0x21;
    pub const SHA2_256: u32 = 0x22;
    pub const VERIFY_SIGNATURE: u32 = 0x23;

    // Cross-VM (0x30-0x3F)
    pub const EVM_CALL: u32 = 0x30;
    pub const SVM_CALL: u32 = 0x31;
    pub const BRIDGE_TRANSFER: u32 = 0x32;

    // Environment (0x40-0x4F)
    pub const GET_BLOCK_NUMBER: u32 = 0x40;
    pub const GET_TIMESTAMP: u32 = 0x41;
    pub const GET_CALLER: u32 = 0x42;
    pub const GET_SELF_ADDRESS: u32 = 0x43;
    pub const GET_GAS_LEFT: u32 = 0x44;

    // Balance/Assets (0x50-0x5F)
    pub const GET_BALANCE: u32 = 0x50;
    pub const TRANSFER: u32 = 0x51;
}

/// Cross-VM execution backends and execution context.
///
/// These are *not* constructed inside this crate so we can stay lightweight.
/// Callers (node service / integration harness) provide the real executors.
#[cfg(feature = "std")]
#[derive(Clone)]
pub struct CrossVmContext {
    pub evm: Option<Arc<dyn x3_evm_integration::EvmExecutor + Send + Sync>>,
    pub svm: Option<Arc<dyn x3_svm_integration::SvmExecutor + Send + Sync>>,

    pub evm_config: x3_evm_integration::EvmConfig,
    pub svm_config: x3_svm_integration::SvmConfig,

    pub evm_caller: sp_core::H160,
    pub svm_payer: [u8; 32],
}

#[cfg(feature = "std")]
impl Default for CrossVmContext {
    fn default() -> Self {
        Self {
            evm: None,
            svm: None,
            evm_config: x3_evm_integration::EvmConfig::default(),
            svm_config: x3_svm_integration::SvmConfig::default(),
            evm_caller: sp_core::H160::zero(),
            svm_payer: [0u8; 32],
        }
    }
}

#[cfg(feature = "std")]
fn vm_hostcall_error(msg: impl Into<String>) -> x3_vm::VMError {
    x3_vm::VMError::without_ip(x3_vm::VMErrorKind::HostcallError(msg.into()))
}

#[cfg(feature = "std")]
fn expect_bytes(arg: Option<&x3_vm::Value>, name: &str) -> Result<Vec<u8>, x3_vm::VMError> {
    match arg {
        Some(x3_vm::Value::Bytes(b)) => Ok(b.clone()),
        Some(v) => Err(vm_hostcall_error(format!(
            "{} requires bytes, got {:?}",
            name, v
        ))),
        None => Err(vm_hostcall_error(format!("{} missing argument", name))),
    }
}

#[cfg(feature = "std")]
fn expect_i64(arg: Option<&x3_vm::Value>, name: &str) -> Result<i64, x3_vm::VMError> {
    match arg {
        Some(x3_vm::Value::I64(v)) => Ok(*v),
        Some(v) => Err(vm_hostcall_error(format!(
            "{} requires i64, got {:?}",
            name, v
        ))),
        None => Err(vm_hostcall_error(format!("{} missing argument", name))),
    }
}

#[cfg(feature = "std")]
fn bytes_to_h160(bytes: &[u8]) -> Result<sp_core::H160, x3_vm::VMError> {
    if bytes.len() != 20 {
        return Err(vm_hostcall_error(format!(
            "address must be 20 bytes, got {}",
            bytes.len()
        )));
    }
    Ok(sp_core::H160::from_slice(bytes))
}

/// Substrate-aware hostcall handler
#[cfg(feature = "std")]
pub struct SubstrateHostcalls {
    /// Accumulated state changes
    state_changes: Vec<(H256, Vec<u8>)>,
    /// Accumulated events
    events: Vec<(H256, Vec<u8>)>,
    /// Current block number
    block_number: u64,
    /// Current timestamp
    timestamp: u64,
    /// Caller address
    caller: Vec<u8>,
}

#[cfg(feature = "std")]
impl SubstrateHostcalls {
    /// Create new hostcall handler
    pub fn new(block_number: u64, timestamp: u64, caller: Vec<u8>) -> Self {
        Self {
            state_changes: Vec::new(),
            events: Vec::new(),
            block_number,
            timestamp,
            caller,
        }
    }

    /// Get accumulated state changes
    pub fn state_changes(&self) -> &[(H256, Vec<u8>)] {
        &self.state_changes
    }

    /// Get accumulated events
    pub fn events(&self) -> &[(H256, Vec<u8>)] {
        &self.events
    }

    /// Handle a hostcall
    pub fn handle(&mut self, id: u32, args: &[u8]) -> Result<Vec<u8>, String> {
        match id {
            hostcall_ids::STORAGE_READ => self.storage_read(args),
            hostcall_ids::STORAGE_WRITE => self.storage_write(args),
            hostcall_ids::EMIT_EVENT => self.emit_event(args),
            hostcall_ids::BLAKE2_256 => self.blake2_256(args),
            hostcall_ids::KECCAK_256 => self.keccak_256(args),
            hostcall_ids::GET_BLOCK_NUMBER => Ok(self.block_number.to_le_bytes().to_vec()),
            hostcall_ids::GET_TIMESTAMP => Ok(self.timestamp.to_le_bytes().to_vec()),
            hostcall_ids::GET_CALLER => Ok(self.caller.clone()),
            _ => Err(format!("Unknown hostcall: 0x{:02X}", id)),
        }
    }

    fn storage_read(&self, key: &[u8]) -> Result<Vec<u8>, String> {
        if key.len() != 32 {
            return Err("Storage key must be 32 bytes".to_string());
        }
        // In real implementation, read from Substrate storage
        // For now, return empty
        Ok(Vec::new())
    }

    fn storage_write(&mut self, args: &[u8]) -> Result<Vec<u8>, String> {
        if args.len() < 32 {
            return Err("Storage write requires key (32 bytes) + value".to_string());
        }
        let key = H256::from_slice(&args[0..32]);
        let value = args[32..].to_vec();
        self.state_changes.push((key, value));
        Ok(Vec::new())
    }

    fn emit_event(&mut self, args: &[u8]) -> Result<Vec<u8>, String> {
        if args.len() < 32 {
            return Err("Event requires topic (32 bytes) + data".to_string());
        }
        let topic = H256::from_slice(&args[0..32]);
        let data = args[32..].to_vec();
        self.events.push((topic, data));
        Ok(Vec::new())
    }

    fn blake2_256(&self, data: &[u8]) -> Result<Vec<u8>, String> {
        Ok(sp_io::hashing::blake2_256(data).to_vec())
    }

    fn keccak_256(&self, data: &[u8]) -> Result<Vec<u8>, String> {
        Ok(sp_io::hashing::keccak_256(data).to_vec())
    }
}

/// Register hostcalls with X3 VM
#[cfg(feature = "std")]
pub fn register_substrate_hostcalls(registry: &mut x3_vm::HostcallRegistry) {
    use hostcall_ids::*;
    use x3_vm::{VMError, VMErrorKind};

    // Register storage operations
    registry.register(STORAGE_READ as u8, "storage_read", 1, |_args| Ok(None));
    registry.register(STORAGE_WRITE as u8, "storage_write", 2, |_args| Ok(None));
    registry.register(STORAGE_DELETE as u8, "storage_delete", 1, |_args| Ok(None));
    registry.register(STORAGE_EXISTS as u8, "storage_exists", 1, |_args| {
        Ok(Some(x3_vm::Value::Bool(false)))
    });

    // Register event/logging
    registry.register(EMIT_EVENT as u8, "emit_event", 2, |_args| Ok(None));
    registry.register(LOG_DEBUG as u8, "log_debug", 1, |_args| Ok(None));

    // Register crypto
    registry.register(BLAKE2_256 as u8, "blake2_256", 1, |args| {
        if let Some(x3_vm::Value::Bytes(data)) = args.first() {
            let hash = sp_io::hashing::blake2_256(data);
            Ok(Some(x3_vm::Value::Bytes(hash.to_vec())))
        } else {
            Err(VMError::without_ip(VMErrorKind::HostcallError(
                "blake2_256 requires bytes argument".into(),
            )))
        }
    });
    registry.register(KECCAK_256 as u8, "keccak_256", 1, |args| {
        if let Some(x3_vm::Value::Bytes(data)) = args.first() {
            let hash = sp_io::hashing::keccak_256(data);
            Ok(Some(x3_vm::Value::Bytes(hash.to_vec())))
        } else {
            Err(VMError::without_ip(VMErrorKind::HostcallError(
                "keccak_256 requires bytes argument".into(),
            )))
        }
    });
    registry.register(SHA2_256 as u8, "sha2_256", 1, |_args| Ok(None));
    registry.register(VERIFY_SIGNATURE as u8, "verify_signature", 3, |_args| {
        Ok(Some(x3_vm::Value::Bool(false)))
    });

    // Register cross-VM
    // Fail-closed by default: cross-VM wiring must be explicitly provided.
    registry.register(EVM_CALL as u8, "evm_call", 4, |_args| {
        Err(VMError::without_ip(VMErrorKind::HostcallError(
            "evm_call not wired (provide CrossVmContext via register_substrate_hostcalls_with_cross_vm)"
                .into(),
        )))
    });
    registry.register(SVM_CALL as u8, "svm_call", 3, |_args| {
        Err(VMError::without_ip(VMErrorKind::HostcallError(
            "svm_call not wired (provide CrossVmContext via register_substrate_hostcalls_with_cross_vm)"
                .into(),
        )))
    });
    registry.register(BRIDGE_TRANSFER as u8, "bridge_transfer", 3, |_args| {
        Err(VMError::without_ip(VMErrorKind::HostcallError(
            "bridge_transfer not wired (provide CrossVmContext via register_substrate_hostcalls_with_cross_vm)"
                .into(),
        )))
    });

    // Register environment
    registry.register(GET_BLOCK_NUMBER as u8, "get_block_number", 0, |_args| {
        Ok(Some(x3_vm::Value::I64(0)))
    });
    registry.register(GET_TIMESTAMP as u8, "get_timestamp", 0, |_args| {
        Ok(Some(x3_vm::Value::I64(0)))
    });
    registry.register(GET_CALLER as u8, "get_caller", 0, |_args| {
        Ok(Some(x3_vm::Value::Bytes(vec![0u8; 32])))
    });
    registry.register(GET_SELF_ADDRESS as u8, "get_self_address", 0, |_args| {
        Ok(Some(x3_vm::Value::Bytes(vec![0u8; 32])))
    });
    registry.register(GET_GAS_LEFT as u8, "get_gas_left", 0, |_args| {
        Ok(Some(x3_vm::Value::I64(0)))
    });

    // Register balance/assets
    registry.register(GET_BALANCE as u8, "get_balance", 1, |_args| {
        Ok(Some(x3_vm::Value::I64(0)))
    });
    registry.register(TRANSFER as u8, "transfer", 3, |_args| {
        Ok(Some(x3_vm::Value::Bool(false)))
    });
}

/// Register Substrate hostcalls with cross-VM execution wired to real adapters.
///
/// This stays fail-closed if an executor is missing.
#[cfg(feature = "std")]
pub fn register_substrate_hostcalls_with_cross_vm(
    registry: &mut x3_vm::HostcallRegistry,
    ctx: CrossVmContext,
) {
    use hostcall_ids::*;

    let ctx = Arc::new(ctx);

    registry.register(EVM_CALL as u8, "evm_call", 4, {
        let ctx = Arc::clone(&ctx);
        move |args| {
            let executor = ctx
                .evm
                .as_ref()
                .ok_or_else(|| vm_hostcall_error("evm executor not configured"))?;

            let gas = expect_i64(args.first(), "evm_call.gas")?;
            let addr_bytes = expect_bytes(args.get(1), "evm_call.addr")?;
            let value_i64 = expect_i64(args.get(2), "evm_call.value")?;
            let data = expect_bytes(args.get(3), "evm_call.data")?;

            let target = bytes_to_h160(&addr_bytes)?;
            let mut cfg = ctx.evm_config.clone();
            cfg.gas_limit = gas.max(0) as u64;

            let result = executor
                .call(
                    &data,
                    ctx.evm_caller,
                    target,
                    sp_core::U256::from(value_i64.max(0) as u64),
                    &cfg,
                )
                .map_err(|e| vm_hostcall_error(format!("evm_call failed: {:?}", e)))?;

            Ok(Some(x3_vm::Value::Bytes(result.output)))
        }
    });

    registry.register(SVM_CALL as u8, "svm_call", 3, {
        let ctx = Arc::clone(&ctx);
        move |args| {
            let executor = ctx
                .svm
                .as_ref()
                .ok_or_else(|| vm_hostcall_error("svm executor not configured"))?;

            let program = expect_bytes(args.first(), "svm_call.program")?;
            let input = expect_bytes(args.get(1), "svm_call.input")?;
            let compute_units = expect_i64(args.get(2), "svm_call.compute_units")?;

            let mut cfg = ctx.svm_config.clone();
            cfg.compute_unit_limit = compute_units.max(0) as u64;

            let result = executor
                .execute_bpf(&program, &input, &cfg)
                .map_err(|e| vm_hostcall_error(format!("svm_call failed: {:?}", e)))?;

            Ok(Some(x3_vm::Value::Bytes(result.output)))
        }
    });

    // Cross-domain token transfer is intentionally left as fail-closed until
    // canonical ledger / escrow semantics are wired.
    registry.register(BRIDGE_TRANSFER as u8, "bridge_transfer", 3, |_args| {
        Err(vm_hostcall_error(
            "bridge_transfer not implemented (wire canonical bridge + ledger semantics)",
        ))
    });
}

#[cfg(all(test, feature = "std"))]
mod cross_vm_tests {
    use super::*;

    #[test]
    fn cross_vm_hostcalls_fail_closed_by_default() {
        let mut registry = x3_vm::HostcallRegistry::new();
        register_substrate_hostcalls(&mut registry);

        // evm_call: 4 args
        let evm_res = registry.invoke(
            hostcall_ids::EVM_CALL as u8,
            &[
                x3_vm::Value::I64(21_000),
                x3_vm::Value::Bytes(vec![0u8; 20]),
                x3_vm::Value::I64(0),
                x3_vm::Value::Bytes(vec![]),
            ],
        );
        assert!(evm_res.is_err());

        // svm_call: 3 args
        let svm_res = registry.invoke(
            hostcall_ids::SVM_CALL as u8,
            &[
                x3_vm::Value::Bytes(vec![0x7f, 0x45, 0x4c, 0x46]),
                x3_vm::Value::Bytes(vec![]),
                x3_vm::Value::I64(100_000),
            ],
        );
        assert!(svm_res.is_err());
    }

    #[test]
    fn cross_vm_hostcalls_can_use_executor_traits() {
        let mut registry = x3_vm::HostcallRegistry::new();

        let ctx = CrossVmContext {
            evm: Some(Arc::new(x3_evm_integration::MockEvmExecutor)),
            svm: Some(Arc::new(x3_svm_integration::MockSvmExecutor::new())),
            ..CrossVmContext::default()
        };

        register_substrate_hostcalls_with_cross_vm(&mut registry, ctx);

        let evm_res = registry.invoke(
            hostcall_ids::EVM_CALL as u8,
            &[
                x3_vm::Value::I64(21_000),
                x3_vm::Value::Bytes(vec![0u8; 20]),
                x3_vm::Value::I64(0),
                x3_vm::Value::Bytes(vec![0x60, 0x00]),
            ],
        );
        assert!(matches!(evm_res, Ok(Some(x3_vm::Value::Bytes(_)))));

        let svm_res = registry.invoke(
            hostcall_ids::SVM_CALL as u8,
            &[
                x3_vm::Value::Bytes(vec![0x79, 0x00]),
                x3_vm::Value::Bytes(vec![0x01]),
                x3_vm::Value::I64(200_000),
            ],
        );
        assert!(matches!(svm_res, Ok(Some(x3_vm::Value::Bytes(_)))));
    }
}
