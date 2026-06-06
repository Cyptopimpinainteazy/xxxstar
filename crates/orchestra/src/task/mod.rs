//! Task subsystem — .md spec parser, queue management, execution engine.

pub mod executor;
pub mod queue;
pub mod spec;

pub use executor::{ExecutionResult, TaskExecutor};
pub use queue::TaskQueue;
pub use spec::{TaskMetadata, TaskSpec, TaskType};
