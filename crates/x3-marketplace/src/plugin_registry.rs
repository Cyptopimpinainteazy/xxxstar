//! Plugin Registry — Central registry for marketplace plugins
//!
//! Manages plugin registration, versioning, and metadata tracking

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use crate::{Result, MarketplaceError};

/// Plugin metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetadata {
    pub description: String,
    pub author: String,
    pub repository: String,
    pub documentation_url: String,
    pub license: String,
    pub icon_ipfs_hash: String,
    pub shields_badge_url: String,
}

/// Plugin version info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginVersion {
    pub version: String,
    pub release_date: DateTime<Utc>,
    pub code_hash: String, // SHA-256 of plugin code
    pub breaking_changes: Vec<String>,
    pub compatibility_notes: String,
    pub download_count: u64,
}

/// Core plugin definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plugin {
    pub id: String,
    pub name: String,
    pub metadata: PluginMetadata,
    pub developer: String,
    pub status: crate::PluginStatus,
    pub versions: Vec<PluginVersion>,
    pub latest_version: String,
    pub total_downloads: u64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub category: PluginCategory,
    pub weekly_downloads: u64,
    pub dependencies: Vec<PluginDependency>,
}

impl Plugin {
    /// Latest version info
    pub fn latest(&self) -> Option<&PluginVersion> {
        self.versions
            .iter()
            .find(|v| v.version == self.latest_version)
    }

    /// Version history (newest first)
    pub fn version_history(&self) -> Vec<&PluginVersion> {
        let mut sorted = self.versions.iter().collect::<Vec<_>>();
        sorted.sort_by(|a, b| b.release_date.cmp(&a.release_date));
        sorted
    }

    /// Is plugin actively maintained
    pub fn is_maintained(&self) -> bool {
        let now = Utc::now();
        let days_since_update = (now - self.updated_at).num_days();
        days_since_update < 180 // Updated in last 6 months
    }

    /// Popularity score (0-100)
    pub fn popularity_score(&self) -> f64 {
        if self.total_downloads == 0 {
            return 0.0;
        }
        // Logarithmic scale: 1 download = 10, 100 = 30, 10k = 50, 1M = 100
        ((self.total_downloads as f64).log10() * 10.0).min(100.0)
    }
}

/// Plugin category
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum PluginCategory {
    Authentication,
    Analytics,
    Wallet,
    Trading,
    Governance,
    Staking,
    Bridge,
    Oracle,
    DeFi,
    NFT,
    Social,
    Other,
}

/// Plugin dependency
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginDependency {
    pub plugin_id: String,
    pub min_version: String,
}

/// Plugin Registry Manager
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginRegistry {
    plugins: HashMap<String, Plugin>,
    by_category: HashMap<PluginCategory, Vec<String>>,
    by_developer: HashMap<String, Vec<String>>,
    total_downloads: u64,
}

impl PluginRegistry {
    pub fn new() -> Self {
        PluginRegistry {
            plugins: HashMap::new(),
            by_category: HashMap::new(),
            by_developer: HashMap::new(),
            total_downloads: 0,
        }
    }

    /// Register new plugin
    pub fn register_plugin(&mut self, mut plugin: Plugin) -> Result<String> {
        if self.plugins.contains_key(&plugin.id) {
            return Err(MarketplaceError::PluginExists);
        }

        let plugin_id = plugin.id.clone();
        let category = plugin.category;
        let developer = plugin.developer.clone();

        self.plugins.insert(plugin_id.clone(), plugin);
        self.by_category
            .entry(category)
            .or_insert_with(Vec::new)
            .push(plugin_id.clone());
        self.by_developer
            .entry(developer)
            .or_insert_with(Vec::new)
            .push(plugin_id.clone());

        Ok(plugin_id)
    }

    /// Get plugin by ID
    pub fn get_plugin(&self, plugin_id: &str) -> Result<Plugin> {
        self.plugins
            .get(plugin_id)
            .cloned()
            .ok_or(MarketplaceError::PluginNotFound)
    }

    /// Get all plugins
    pub fn all_plugins(&self) -> Vec<Plugin> {
        self.plugins.values().cloned().collect()
    }

    /// Get plugins by category
    pub fn plugins_by_category(&self, category: PluginCategory) -> Vec<Plugin> {
        self.by_category
            .get(&category)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.plugins.get(id))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get plugins by developer
    pub fn plugins_by_developer(&self, developer: &str) -> Vec<Plugin> {
        self.by_developer
            .get(developer)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.plugins.get(id))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get approved plugins only
    pub fn approved_plugins(&self) -> Vec<Plugin> {
        self.plugins
            .values()
            .filter(|p| p.status == crate::PluginStatus::Approved)
            .cloned()
            .collect()
    }

    /// Record download
    pub fn record_download(&mut self, plugin_id: &str) -> Result<()> {
        if let Some(plugin) = self.plugins.get_mut(plugin_id) {
            plugin.total_downloads += 1;
            plugin.weekly_downloads += 1;
            self.total_downloads += 1;
            Ok(())
        } else {
            Err(MarketplaceError::PluginNotFound)
        }
    }

    /// Release new plugin version
    pub fn release_version(
        &mut self,
        plugin_id: &str,
        version: String,
        code_hash: String,
        breaking_changes: Vec<String>,
        compatibility_notes: String,
    ) -> Result<()> {
        if let Some(plugin) = self.plugins.get_mut(plugin_id) {
            let new_version = PluginVersion {
                version: version.clone(),
                release_date: Utc::now(),
                code_hash,
                breaking_changes,
                compatibility_notes,
                download_count: 0,
            };

            plugin.versions.push(new_version);
            plugin.latest_version = version;
            plugin.updated_at = Utc::now();
            Ok(())
        } else {
            Err(MarketplaceError::PluginNotFound)
        }
    }

    /// Top plugins by downloads
    pub fn top_plugins(&self, limit: usize) -> Vec<Plugin> {
        let mut plugins = self.approved_plugins();
        plugins.sort_by(|a, b| b.total_downloads.cmp(&a.total_downloads));
        plugins.into_iter().take(limit).collect()
    }

    /// Trending plugins (high weekly downloads)
    pub fn trending_plugins(&self, limit: usize) -> Vec<Plugin> {
        let mut plugins = self.approved_plugins();
        plugins.sort_by(|a, b| b.weekly_downloads.cmp(&a.weekly_downloads));
        plugins.into_iter().take(limit).collect()
    }

    /// Search plugins by name
    pub fn search_by_name(&self, query: &str) -> Vec<Plugin> {
        let query_lower = query.to_lowercase();
        self.approved_plugins()
            .into_iter()
            .filter(|p| {
                p.name.to_lowercase().contains(&query_lower) ||
                p.metadata.description.to_lowercase().contains(&query_lower)
            })
            .collect()
    }

    /// Update plugin status
    pub fn update_status(
        &mut self,
        plugin_id: &str,
        status: crate::PluginStatus,
    ) -> Result<()> {
        if let Some(plugin) = self.plugins.get_mut(plugin_id) {
            plugin.status = status;
            plugin.updated_at = Utc::now();
            Ok(())
        } else {
            Err(MarketplaceError::PluginNotFound)
        }
    }

    /// Total plugins count
    pub fn count(&self) -> u32 {
        self.plugins.len() as u32
    }

    /// Get plugin count by category
    pub fn category_count(&self, category: PluginCategory) -> u32 {
        self.by_category
            .get(&category)
            .map(|p| p.len() as u32)
            .unwrap_or(0)
    }

    /// Total downloads across all plugins
    pub fn total_downloads(&self) -> u64 {
        self.total_downloads
    }

    /// Get maintained plugins
    pub fn maintained_plugins(&self) -> Vec<Plugin> {
        self.approved_plugins()
            .into_iter()
            .filter(|p| p.is_maintained())
            .collect()
    }

    /// Reset weekly download counts (call weekly)
    pub fn reset_weekly_counts(&mut self) {
        for plugin in self.plugins.values_mut() {
            plugin.weekly_downloads = 0;
        }
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_plugin(id: &str, name: &str) -> Plugin {
        Plugin {
            id: id.to_string(),
            name: name.to_string(),
            metadata: PluginMetadata {
                description: "Test plugin".to_string(),
                author: "Test Author".to_string(),
                repository: "https://github.com/test".to_string(),
                documentation_url: "https://docs.example.com".to_string(),
                license: "Apache-2.0".to_string(),
                icon_ipfs_hash: "QmTest".to_string(),
                shields_badge_url: "https://shields.io/test".to_string(),
            },
            developer: "test_dev".to_string(),
            status: crate::PluginStatus::Approved,
            versions: vec![PluginVersion {
                version: "1.0.0".to_string(),
                release_date: Utc::now(),
                code_hash: "hash123".to_string(),
                breaking_changes: vec![],
                compatibility_notes: "Initial release".to_string(),
                download_count: 0,
            }],
            latest_version: "1.0.0".to_string(),
            total_downloads: 0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            category: PluginCategory::Analytics,
            weekly_downloads: 0,
            dependencies: vec![],
        }
    }

    #[test]
    fn test_register_plugin() {
        let mut registry = PluginRegistry::new();
        let plugin = create_test_plugin("auth_plugin", "Auth Plugin");
        let id = registry.register_plugin(plugin).unwrap();

        assert_eq!(id, "auth_plugin");
        assert_eq!(registry.count(), 1);
    }

    #[test]
    fn test_duplicate_plugin_error() {
        let mut registry = PluginRegistry::new();
        let plugin = create_test_plugin("auth_plugin", "Auth Plugin");
        registry.register_plugin(plugin.clone()).unwrap();

        let result = registry.register_plugin(plugin);
        assert!(result.is_err());
    }

    #[test]
    fn test_get_plugin() {
        let mut registry = PluginRegistry::new();
        let plugin = create_test_plugin("auth_plugin", "Auth Plugin");
        registry.register_plugin(plugin.clone()).unwrap();

        let retrieved = registry.get_plugin("auth_plugin").unwrap();
        assert_eq!(retrieved.name, "Auth Plugin");
    }

    #[test]
    fn test_plugins_by_category() {
        let mut registry = PluginRegistry::new();
        let plugin1 = create_test_plugin("auth1", "Auth Plugin 1");
        let mut plugin2 = create_test_plugin("analytics1", "Analytics Plugin");
        plugin2.category = PluginCategory::Analytics;

        registry.register_plugin(plugin1).unwrap();
        registry.register_plugin(plugin2).unwrap();

        let analytics = registry.plugins_by_category(PluginCategory::Analytics);
        assert_eq!(analytics.len(), 1);
    }

    #[test]
    fn test_record_download() {
        let mut registry = PluginRegistry::new();
        let plugin = create_test_plugin("auth_plugin", "Auth Plugin");
        registry.register_plugin(plugin).unwrap();

        registry.record_download("auth_plugin").unwrap();
        let plugin = registry.get_plugin("auth_plugin").unwrap();
        assert_eq!(plugin.total_downloads, 1);
    }

    #[test]
    fn test_top_plugins() {
        let mut registry = PluginRegistry::new();
        for i in 0..3 {
            let plugin = create_test_plugin(&format!("plugin{}", i), &format!("Plugin {}", i));
            registry.register_plugin(plugin).unwrap();
        }

        registry.record_download("plugin0").unwrap();
        registry.record_download("plugin0").unwrap();
        registry.record_download("plugin1").unwrap();

        let top = registry.top_plugins(2);
        assert_eq!(top.len(), 2);
        assert_eq!(top[0].id, "plugin0");
    }

    #[test]
    fn test_search_by_name() {
        let mut registry = PluginRegistry::new();
        let plugin = create_test_plugin("auth_plugin", "Authentication Plugin");
        registry.register_plugin(plugin).unwrap();

        let results = registry.search_by_name("auth");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_update_status() {
        let mut registry = PluginRegistry::new();
        let plugin = create_test_plugin("auth_plugin", "Auth Plugin");
        registry.register_plugin(plugin).unwrap();

        registry
            .update_status("auth_plugin", crate::PluginStatus::Suspended)
            .unwrap();

        let plugin = registry.get_plugin("auth_plugin").unwrap();
        assert_eq!(plugin.status, crate::PluginStatus::Suspended);
    }

    #[test]
    fn test_release_version() {
        let mut registry = PluginRegistry::new();
        let plugin = create_test_plugin("auth_plugin", "Auth Plugin");
        registry.register_plugin(plugin).unwrap();

        registry
            .release_version(
                "auth_plugin",
                "1.1.0".to_string(),
                "newhash".to_string(),
                vec!["Breaking change 1".to_string()],
                "Improved performance".to_string(),
            )
            .unwrap();

        let plugin = registry.get_plugin("auth_plugin").unwrap();
        assert_eq!(plugin.latest_version, "1.1.0");
        assert_eq!(plugin.versions.len(), 2);
    }

    #[test]
    fn test_plugin_status_check() {
        let plugin = create_test_plugin("auth_plugin", "Auth Plugin");
        assert_eq!(plugin.status, crate::PluginStatus::Approved);
    }

    #[test]
    fn test_plugin_popularity_score() {
        let mut plugin = create_test_plugin("auth_plugin", "Auth Plugin");
        plugin.total_downloads = 1000;

        let score = plugin.popularity_score();
        assert!(score > 0.0 && score < 100.0);
    }

    #[test]
    fn test_plugins_by_developer() {
        let mut registry = PluginRegistry::new();
        let plugin1 = create_test_plugin("plugin1", "Plugin 1");
        let plugin2 = create_test_plugin("plugin2", "Plugin 2");

        registry.register_plugin(plugin1).unwrap();
        registry.register_plugin(plugin2).unwrap();

        let dev_plugins = registry.plugins_by_developer("test_dev");
        assert_eq!(dev_plugins.len(), 2);
    }

    #[test]
    fn test_approved_plugins_only() {
        let mut registry = PluginRegistry::new();
        let mut plugin1 = create_test_plugin("plugin1", "Plugin 1");
        let mut plugin2 = create_test_plugin("plugin2", "Plugin 2");
        plugin2.status = crate::PluginStatus::Pending;

        registry.register_plugin(plugin1).unwrap();
        registry.register_plugin(plugin2).unwrap();

        let approved = registry.approved_plugins();
        assert_eq!(approved.len(), 1);
    }
}
