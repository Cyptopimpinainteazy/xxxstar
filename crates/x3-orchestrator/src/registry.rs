//! Chain adapter registry.

use std::{collections::HashMap, sync::Arc};

use parking_lot::RwLock;

use crate::{ChainId, CrossVmMessage, ExecutionProof, Result};

/// Trait implemented by every chain adapter (EVM, SVM, X3VM, Bitcoin, ...).
pub trait ChainAdapter: Send + Sync {
    fn chain_id(&self) -> ChainId;
    fn send(&self, msg: &CrossVmMessage) -> Result<String>;
    fn verify(&self, proof: &ExecutionProof) -> Result<bool>;
    fn execute(&self, msg: &CrossVmMessage) -> Result<()>;
}

#[derive(Default)]
pub struct AdapterRegistry {
    adapters: RwLock<HashMap<ChainId, Arc<dyn ChainAdapter>>>,
}

impl AdapterRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&self, adapter: Arc<dyn ChainAdapter>) {
        self.adapters.write().insert(adapter.chain_id(), adapter);
    }

    pub fn get(&self, chain_id: &ChainId) -> Option<Arc<dyn ChainAdapter>> {
        self.adapters.read().get(chain_id).cloned()
    }
}
