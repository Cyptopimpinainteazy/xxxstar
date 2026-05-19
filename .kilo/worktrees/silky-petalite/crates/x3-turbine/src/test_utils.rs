//! Test utilities for Turbine testing

use crate::blockstore::BlockstoreConfig;
use crate::config::{ShredConfig, TurbineConfig};
use crate::peer::{PeerInfo, PeerRole};
use crate::shred::{Shred, ShredPayload};
use crate::Turbine;

/// Create a test turbine instance
pub fn create_test_turbine() -> Turbine {
    let config = TurbineConfig {
        shred_size: 16384,
        num_data_shreds: 8,
        num_coding_shreds: 4,
        ..Default::default()
    };

    Turbine::new(config)
}

/// Create test shred config
pub fn create_test_shred_config() -> ShredConfig {
    ShredConfig {
        shred_size: 1024,
        coding_shreds: 4,
        data_shreds: 8,
    }
}

/// Create test blockstore config
pub fn create_test_blockstore_config() -> BlockstoreConfig {
    BlockstoreConfig {
        max_pending_blocks: 10,
        shred_recovery_timeout: std::time::Duration::from_secs(1),
        enable_recovery: true,
    }
}

/// Create a test peer
pub fn create_test_peer(id: &str) -> PeerInfo {
    PeerInfo::new(
        id.to_string(),
        format!("/ip4/127.0.0.1/tcp/{}", 8000 + (id.len() as u16)),
        PeerRole::Validator,
    )
}

/// Generate test block data
pub fn generate_test_block_data(size: usize) -> Vec<u8> {
    (0..size).map(|i| (i % 256) as u8).collect()
}

/// Create test shred for a given slot
pub fn create_test_shred(slot: u64, index: u32, num_shreds: u32) -> Shred {
    let data = format!("test-shred-data-{}", index);
    let payload = ShredPayload::new(data.into_bytes());
    let reference = [0u8; 32];

    if index < num_shreds / 2 {
        Shred::new_data(slot, index, num_shreds, reference, payload)
    } else {
        Shred::new_coding(slot, index, num_shreds, index - num_shreds / 2, payload)
    }
}

/// Create multiple test shreds for a slot
pub fn create_test_shreds(slot: u64, num_shreds: u32) -> Vec<Shred> {
    (0..num_shreds)
        .map(|i| create_test_shred(slot, i, num_shreds))
        .collect()
}

/// Mock peer manager for testing
pub mod mock_peer_manager {
    use crate::config::TurbineConfig;
    use crate::peer::{PeerInfo, PeerManager, PeerRole};

    pub fn create() -> PeerManager {
        let config = TurbineConfig::default();
        PeerManager::new(config)
    }

    pub fn add_test_peers(manager: &PeerManager, count: usize) {
        for i in 0..count {
            let peer = PeerInfo::new(
                format!("test-peer-{}", i),
                format!("/ip4/127.0.0.1/tcp/{}", 8000 + i),
                PeerRole::Validator,
            );
            manager.add_peer(peer);
        }
    }
}
