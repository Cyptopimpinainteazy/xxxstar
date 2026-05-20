use anyhow::Result;
use ethers::providers::{Http, Middleware, Provider};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tracing::{info, warn};

pub struct RpcPool {
    providers: Vec<Arc<Provider<Http>>>,
    index: AtomicUsize,
}

impl RpcPool {
    pub fn new(urls: Vec<String>) -> Result<Self> {
        let mut providers = Vec::new();
        for url in urls {
            let provider = Provider::<Http>::try_from(url)?;
            providers.push(Arc::new(provider));
        }
        Ok(Self {
            providers,
            index: AtomicUsize::new(0),
        })
    }

    pub fn get_next(&self) -> Arc<Provider<Http>> {
        let idx = self.index.fetch_add(1, Ordering::SeqCst) % self.providers.len();
        self.providers[idx].clone()
    }

    pub async fn health_check(&self) -> bool {
        let mut healthy_count = 0;
        for (i, p) in self.providers.iter().enumerate() {
            match p.get_block_number().await {
                Ok(n) => {
                    info!("RPC {} healthy: block {}", i, n);
                    healthy_count += 1;
                }
                Err(e) => warn!("RPC {} failed health check: {}", i, e),
            }
        }
        healthy_count > 0
    }
}
