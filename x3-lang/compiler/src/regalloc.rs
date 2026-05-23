//! Register allocation for the lowering pipeline (naive version)

use crate::lowering::LoweredInstr;

/// A simple, deterministic coloring register allocator that assigns registers to temporaries in order.
pub fn allocate(instrs: &[LoweredInstr]) -> Vec<LoweredInstr> {
    // For v0.1 we return the input sequence unchanged. This is a placeholder for a deterministic allocator.
    instrs.to_vec()
}
