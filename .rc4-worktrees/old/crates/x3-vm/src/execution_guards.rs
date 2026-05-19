use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceVector {
    pub cpu_cycles: u64,
    pub gpu_cycles: u64,
    pub memory_bytes: u64,
    pub io_ops: u64,
    pub storage_reads: u64,
    pub storage_writes: u64,
}

pub struct AgentContext {
    pub memory_cap: u64,
    pub resource_limit: ResourceVector,
}

pub struct AgentInput {
    pub data: Vec<u8>,
}

pub enum AgentAction {
    Continue,
    Yield,
}

pub enum AgentExit {
    Success,
    ResourceExhausted,
    Fault,
}

pub trait Agent {
    fn init(&mut self, ctx: AgentContext) -> Result<(), &'static str>;
    fn step(&mut self, input: AgentInput) -> Result<AgentAction, &'static str>;
    fn halt(&mut self) -> AgentExit;
}

pub struct ExecutionGuard {
    pub current_usage: ResourceVector,
    pub limits: ResourceVector,
}

impl ExecutionGuard {
    pub fn new(limits: ResourceVector) -> Self {
        Self {
            current_usage: ResourceVector {
                cpu_cycles: 0,
                gpu_cycles: 0,
                memory_bytes: 0,
                io_ops: 0,
                storage_reads: 0,
                storage_writes: 0,
            },
            limits,
        }
    }

    pub fn consume_cpu(&mut self, amount: u64) -> Result<(), &'static str> {
        self.current_usage.cpu_cycles = self.current_usage.cpu_cycles.saturating_add(amount);
        if self.current_usage.cpu_cycles > self.limits.cpu_cycles {
            return Err("CPU limit exceeded");
        }
        Ok(())
    }

    pub fn consume_memory(&mut self, amount: u64) -> Result<(), &'static str> {
        self.current_usage.memory_bytes = self.current_usage.memory_bytes.saturating_add(amount);
        if self.current_usage.memory_bytes > self.limits.memory_bytes {
            return Err("Memory limit exceeded");
        }
        Ok(())
    }

    pub fn consume_io(&mut self, amount: u64) -> Result<(), &'static str> {
        self.current_usage.io_ops = self.current_usage.io_ops.saturating_add(amount);
        if self.current_usage.io_ops > self.limits.io_ops {
            return Err("I/O limit exceeded");
        }
        Ok(())
    }
}
