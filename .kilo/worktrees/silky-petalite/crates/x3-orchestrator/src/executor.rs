//! VM-level executor trait. Adapters that wrap a real virtual machine
//! (X3VM, EVM, SVM) implement this so the orchestrator can drive them.

use crate::{CrossVmMessage, MessageStatus, Result};

pub trait VmExecutor {
    fn execute_message(&self, msg: &CrossVmMessage) -> Result<MessageStatus>;
}
