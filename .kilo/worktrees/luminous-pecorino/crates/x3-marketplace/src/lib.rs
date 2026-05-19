//! X3 SDK Marketplace
//! 
//! Plugin registry, rating system, and fee distribution for SDK extensions

pub mod plugin_registry;
pub mod rating_system;
pub mod fee_distribution;
pub mod ipfs_metadata;

pub use plugin_registry::{PluginRegistry, Plugin, PluginMetadata};
pub use rating_system::{RatingSystem, Rating};
pub use fee_distribution::{FeeDistribution, FeePool};
pub use ipfs_metadata::{IPFSManager, IPFSPin};

use serde::{Deserialize, Serialize};

/// Marketplace error types
#[derive(Debug, thiserror::Error)]
pub enum MarketplaceError {
    #[error("Plugin not found")]
    PluginNotFound,
    
    #[error("Invalid rating: {0}")]
    InvalidRating(String),
    
    #[error("Insufficient balance")]
    InsufficientBalance,
    
    #[error("IPFS error: {0}")]
    IPFSError(String),
    
    #[error("Plugin already exists")]
    PluginExists,
    
    #[error("Invalid metadata")]
    InvalidMetadata,
}

pub type Result<T> = std::result::Result<T, MarketplaceError>;

/// Plugin status
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum PluginStatus {
    Pending,    // Awaiting review
    Approved,   // Listed in marketplace
    Suspended,  // Temporarily delisted
    Deprecated, // No longer maintained
}

/// Marketplace summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketplaceSummary {
    pub total_plugins: u32,
    pub total_downloads: u64,
    pub total_revenue: u128,
    pub average_rating: f64,
    pub active_plugins: u32,
}

/// SDK version info
pub const VERSION: &str = "1.0.0";
