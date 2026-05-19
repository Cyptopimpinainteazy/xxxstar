//! VM isolation: memory and execution context isolation between contracts.
//!
//! Each contract execution gets an isolated context. Contracts cannot access
//! each other's memory directly — all cross-contract communication goes through
//! the hostcall interface.

use std::collections::BTreeMap;

/// Contract execution context — isolated per call frame.
pub struct IsolationContext {
    /// Contract address owning this context.
    pub contract_addr: [u8; 32],
    /// Linear memory (max 64 KB per context).
    memory: Vec<u8>,
    /// Call depth (for nested call enforcement).
    pub call_depth: u32,
}

/// Maximum linear memory per isolated context.
pub const MAX_MEMORY_BYTES: usize = 65_536;

/// Maximum nested call depth.
pub const MAX_CALL_DEPTH: u32 = 10;

/// Errors from isolation operations.
#[derive(Debug, PartialEq, Eq)]
pub enum IsolationError {
    /// Memory access out of bounds.
    MemoryOutOfBounds { offset: usize, len: usize },
    /// Memory growth would exceed the per-context limit.
    MemoryLimitExceeded,
    /// Call depth would exceed `MAX_CALL_DEPTH`.
    CallDepthExceeded,
}

impl IsolationContext {
    /// Create a new isolated context for the given contract address.
    pub fn new(contract_addr: [u8; 32]) -> Self {
        Self {
            contract_addr,
            memory: Vec::new(),
            call_depth: 0,
        }
    }

    /// Grow the memory by `additional` bytes.
    pub fn grow_memory(&mut self, additional: usize) -> Result<(), IsolationError> {
        let new_len = self.memory.len().saturating_add(additional);
        if new_len > MAX_MEMORY_BYTES {
            return Err(IsolationError::MemoryLimitExceeded);
        }
        self.memory.resize(new_len, 0);
        Ok(())
    }

    /// Read `len` bytes from memory at `offset`.
    pub fn read_memory(&self, offset: usize, len: usize) -> Result<&[u8], IsolationError> {
        let end = offset
            .checked_add(len)
            .ok_or(IsolationError::MemoryOutOfBounds { offset, len })?;
        if end > self.memory.len() {
            return Err(IsolationError::MemoryOutOfBounds { offset, len });
        }
        Ok(&self.memory[offset..end])
    }

    /// Write bytes into memory at `offset`.
    pub fn write_memory(&mut self, offset: usize, data: &[u8]) -> Result<(), IsolationError> {
        let end = offset
            .checked_add(data.len())
            .ok_or(IsolationError::MemoryOutOfBounds {
                offset,
                len: data.len(),
            })?;
        if end > self.memory.len() {
            return Err(IsolationError::MemoryOutOfBounds {
                offset,
                len: data.len(),
            });
        }
        self.memory[offset..end].copy_from_slice(data);
        Ok(())
    }

    /// Enter a nested call. Increments depth.
    pub fn enter_call(&mut self) -> Result<(), IsolationError> {
        if self.call_depth >= MAX_CALL_DEPTH {
            return Err(IsolationError::CallDepthExceeded);
        }
        self.call_depth += 1;
        Ok(())
    }

    /// Exit a nested call. Decrements depth.
    pub fn exit_call(&mut self) {
        self.call_depth = self.call_depth.saturating_sub(1);
    }

    /// Memory size in bytes.
    pub fn memory_size(&self) -> usize {
        self.memory.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn addr(b: u8) -> [u8; 32] {
        [b; 32]
    }

    #[test]
    fn test_grow_and_read_write() {
        let mut ctx = IsolationContext::new(addr(1));
        ctx.grow_memory(256).unwrap();
        ctx.write_memory(0, &[1, 2, 3, 4]).unwrap();
        let data = ctx.read_memory(0, 4).unwrap();
        assert_eq!(data, &[1, 2, 3, 4]);
    }

    #[test]
    fn test_out_of_bounds_read() {
        let ctx = IsolationContext::new(addr(1));
        assert!(matches!(
            ctx.read_memory(0, 10),
            Err(IsolationError::MemoryOutOfBounds { .. })
        ));
    }

    #[test]
    fn test_memory_limit_exceeded() {
        let mut ctx = IsolationContext::new(addr(1));
        assert_eq!(
            ctx.grow_memory(MAX_MEMORY_BYTES + 1),
            Err(IsolationError::MemoryLimitExceeded)
        );
    }

    #[test]
    fn test_call_depth_limit() {
        let mut ctx = IsolationContext::new(addr(1));
        for _ in 0..MAX_CALL_DEPTH {
            ctx.enter_call().unwrap();
        }
        assert_eq!(ctx.enter_call(), Err(IsolationError::CallDepthExceeded));
    }

    #[test]
    fn test_call_depth_exit() {
        let mut ctx = IsolationContext::new(addr(1));
        ctx.enter_call().unwrap();
        ctx.enter_call().unwrap();
        ctx.exit_call();
        assert_eq!(ctx.call_depth, 1);
    }

    #[test]
    fn test_contexts_independent() {
        let mut ctx1 = IsolationContext::new(addr(1));
        let mut ctx2 = IsolationContext::new(addr(2));
        ctx1.grow_memory(64).unwrap();
        ctx2.grow_memory(64).unwrap();
        ctx1.write_memory(0, &[0xAA; 8]).unwrap();
        ctx2.write_memory(0, &[0xBB; 8]).unwrap();
        assert_eq!(ctx1.read_memory(0, 8).unwrap(), &[0xAA; 8]);
        assert_eq!(ctx2.read_memory(0, 8).unwrap(), &[0xBB; 8]);
    }
}
