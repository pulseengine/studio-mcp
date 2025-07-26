//! PLM-specific caching layer with intelligent cache type detection
//! 
//! Provides PLM-aware caching that integrates with the PlmResourceProvider:
//! - Automatic cache type detection based on PLM resource patterns
//! - Smart invalidation for pipeline state changes
//! - Cache warming for frequently accessed resources
//! - Integration with CLI command patterns

use super::{CacheConfig, CacheStats, CacheStore, CacheType, CachedItem};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, trace, warn};

/// PLM-specific cache with intelligent type detection and invalidation
pub struct PlmCache {
    /// Cache stores organized by type for optimal performance
    stores: HashMap<CacheType, Arc<RwLock<CacheStore>>>,
    /// Configuration for cache behavior
    config: CacheConfig,
    /// Statistics tracking
    stats: Arc<RwLock<CacheStats>>,
}

impl PlmCache {
    /// Create a new PLM cache with default configuration
    pub fn new() -> Self {
        Self::with_config(CacheConfig::default())
    }

    /// Create a new PLM cache with custom configuration
    pub fn with_config(config: CacheConfig) -> Self {
        let stores = [
            CacheType::Immutable,
            CacheType::Completed,
            CacheType::SemiDynamic,
            CacheType::Dynamic,
        ]
        .into_iter()
        .map(|cache_type| {
            (
                cache_type,
                Arc::new(RwLock::new(CacheStore::new(config.max_size_per_type))),
            )
        })
        .collect();

        Self {
            stores,
            config,
            stats: Arc::new(RwLock::new(CacheStats::default())),
        }
    }

    /// Get a cached value by key, updating access statistics
    pub async fn get(&self, key: &str) -> Option<Value> {
        if !self.config.enabled {
            return None;
        }

        let cache_type = Self::detect_cache_type(key);
        let store = self.stores.get(&cache_type)?;
        let mut store_guard = store.write().await;

        match store_guard.get(key) {
            Some(value) => {
                if self.config.enable_stats {
                    self.stats.write().await.record_hit();
                }
                trace!("Cache hit for PLM key: {}", key);
                Some(value.clone())
            }
            None => {
                if self.config.enable_stats {
                    self.stats.write().await.record_miss();
                }
                trace!("Cache miss for PLM key: {}", key);
                None
            }
        }
    }

    /// Insert a value into the cache with automatic type detection
    pub async fn insert(&self, key: String, value: Value) {
        if !self.config.enabled {
            return;
        }

        let cache_type = Self::detect_cache_type(&key);
        let store = match self.stores.get(&cache_type) {
            Some(store) => store,
            None => {
                warn!("No cache store found for type: {:?}", cache_type);
                return;
            }
        };

        // Check for custom TTL override
        let mut item = CachedItem::new(value, cache_type);
        if let Some(custom_ttl) = self.config.custom_ttl.get(&cache_type) {
            item.ttl = *custom_ttl;
        }

        let mut store_guard = store.write().await;
        store_guard.insert(key.clone(), item);

        if self.config.enable_stats {
            self.stats.write().await.record_insertion(cache_type);
        }

        debug!("Cached PLM resource: {} (type: {:?})", key, cache_type);
    }

    /// Remove a specific key from the cache
    pub async fn remove(&self, key: &str) {
        if !self.config.enabled {
            return;
        }

        let cache_type = Self::detect_cache_type(key);
        if let Some(store) = self.stores.get(&cache_type) {
            let mut store_guard = store.write().await;
            if store_guard.remove(key).is_some() {
                if self.config.enable_stats {
                    self.stats.write().await.record_eviction(cache_type);
                }
                debug!("Removed from PLM cache: {}", key);
            }
        }
    }

    /// Invalidate cache entries based on PLM resource changes
    pub async fn invalidate_pattern(&self, pattern: &str) {
        if !self.config.enabled {
            return;
        }

        debug!("Invalidating PLM cache pattern: {}", pattern);
        let mut invalidated_count = 0;

        for store in self.stores.values() {
            let mut store_guard = store.write().await;
            let keys_to_remove: Vec<String> = store_guard
                .items
                .keys()
                .filter(|key| key.contains(pattern))
                .cloned()
                .collect();

            for key in keys_to_remove {
                store_guard.remove(&key);
                invalidated_count += 1;
            }
        }

        if self.config.enable_stats {
            for _ in 0..invalidated_count {
                self.stats.write().await.record_invalidation();
            }
        }

        debug!("Invalidated {} PLM cache entries", invalidated_count);
    }

    /// Invalidate caches when pipeline state changes
    pub async fn invalidate_pipeline(&self, pipeline_id: &str) {
        // Invalidate pipeline-specific caches with more specific patterns
        self.invalidate_pattern(&format!("pipeline:def:{}", pipeline_id))
            .await;
        self.invalidate_pattern(&format!("pipeline:runs:{}", pipeline_id))
            .await;
        self.invalidate_pattern(&format!("pipeline:events:{}", pipeline_id))
            .await;
        self.invalidate_pattern(&format!("pipelines/{}", pipeline_id))
            .await;

        // Invalidate dynamic pipeline lists
        self.remove("pipelines:list").await;
        self.remove("runs:list").await;
    }

    /// Invalidate caches when run state changes
    pub async fn invalidate_run(&self, run_id: &str) {
        // Invalidate run-specific caches
        self.invalidate_pattern(&format!("run:{}", run_id)).await;
        self.invalidate_pattern(&format!("runs/{}", run_id)).await;

        // Invalidate dynamic run lists
        self.remove("runs:list").await;
    }

    /// Clean up expired entries across all cache stores
    pub async fn cleanup_expired(&self) -> usize {
        if !self.config.enabled {
            return 0;
        }

        let mut total_cleaned = 0;

        for store in self.stores.values() {
            let mut store_guard = store.write().await;
            total_cleaned += store_guard.cleanup_expired();
        }

        if total_cleaned > 0 {
            debug!("Cleaned up {} expired PLM cache entries", total_cleaned);
        }

        total_cleaned
    }

    /// Get cache statistics
    pub async fn get_stats(&self) -> CacheStats {
        self.stats.read().await.clone()
    }

    /// Clear all caches
    pub async fn clear_all(&self) {
        if !self.config.enabled {
            return;
        }

        for store in self.stores.values() {
            store.write().await.clear();
        }

        debug!("Cleared all PLM cache stores");
    }

    /// Get total cache size across all stores
    pub async fn total_size(&self) -> usize {
        let mut total = 0;
        for store in self.stores.values() {
            total += store.read().await.len();
        }
        total
    }

    /// Detect cache type based on PLM resource key patterns
    fn detect_cache_type(key: &str) -> CacheType {
        // Pipeline definitions and task libraries - rarely change
        if key.contains("definition") 
            || key.contains("task_lib") 
            || key.contains("pipeline:def:")
            || key.contains("tasks:")
            || key.contains("secrets:")
            || key.contains("triggers:")
            || key.contains("access-config:") {
            return CacheType::Immutable;
        }

        // Completed/failed runs and tasks - never change once done
        if key.contains("completed") 
            || key.contains("failed") 
            || key.contains("finished")
            || key.contains(":status:completed")
            || key.contains(":status:failed") {
            return CacheType::Completed;
        }

        // Pipeline/run lists and resource lists - change when items added/removed
        if key.contains("list") 
            || key.contains("pipelines:")
            || key.contains("runs:")
            || key.contains("resources:")
            || key.contains("groups:") {
            return CacheType::SemiDynamic;
        }

        // Active runs, events, live status - frequently changing
        CacheType::Dynamic
    }

    /// Pre-warm cache with commonly accessed PLM resources
    pub async fn warm_cache(&self, pipeline_ids: &[String]) {
        if !self.config.enabled {
            return;
        }

        debug!("Warming PLM cache for {} pipelines", pipeline_ids.len());

        // Cache pipeline definitions (immutable)
        for pipeline_id in pipeline_ids {
            let key = format!("pipeline:def:{}", pipeline_id);
            // Would normally fetch from CLI here
            let mock_definition = serde_json::json!({
                "id": pipeline_id,
                "name": format!("Pipeline {}", pipeline_id),
                "status": "active"
            });
            self.insert(key, mock_definition).await;
        }

        // Cache pipeline list (semi-dynamic)
        let pipeline_list = serde_json::json!({
            "pipelines": pipeline_ids,
            "total": pipeline_ids.len()
        });
        self.insert("pipelines:list".to_string(), pipeline_list).await;

        debug!("PLM cache warmed with {} pipeline definitions", pipeline_ids.len());
    }
}

impl Default for PlmCache {
    fn default() -> Self {
        Self::new()
    }
}

// Helper methods for integration with PlmResourceProvider
impl PlmCache {
    /// Generate cache key for pipeline list
    pub fn pipeline_list_key() -> String {
        "pipelines:list".to_string()
    }

    /// Generate cache key for specific pipeline definition
    pub fn pipeline_definition_key(pipeline_id: &str) -> String {
        format!("pipeline:def:{}", pipeline_id)
    }

    /// Generate cache key for pipeline runs
    pub fn pipeline_runs_key(pipeline_id: &str) -> String {
        format!("pipeline:runs:{}", pipeline_id)
    }

    /// Generate cache key for pipeline events
    pub fn pipeline_events_key(pipeline_id: &str) -> String {
        format!("pipeline:events:{}", pipeline_id)
    }

    /// Generate cache key for run details
    pub fn run_details_key(run_id: &str) -> String {
        format!("run:details:{}", run_id)
    }

    /// Generate cache key for all runs list
    pub fn all_runs_key() -> String {
        "runs:list".to_string()
    }

    /// Generate cache key for tasks list
    pub fn tasks_key() -> String {
        "tasks:list".to_string()
    }

    /// Generate cache key for pipeline resources
    pub fn pipeline_resources_key() -> String {
        "pipeline:resources".to_string()
    }

    /// Generate cache key for user groups
    pub fn groups_key() -> String {
        "groups:list".to_string()
    }

    /// Generate cache key for secrets (metadata only)
    pub fn secrets_key() -> String {
        "secrets:list".to_string()
    }

    /// Generate cache key for triggers
    pub fn triggers_key() -> String {
        "triggers:list".to_string()
    }

    /// Generate cache key for access configs
    pub fn access_config_key() -> String {
        "access-config:list".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::time::Duration;
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_plm_cache_type_detection() {
        assert_eq!(
            PlmCache::detect_cache_type("pipeline:def:123"),
            CacheType::Immutable
        );
        assert_eq!(
            PlmCache::detect_cache_type("run:status:completed:456"),
            CacheType::Completed
        );
        assert_eq!(
            PlmCache::detect_cache_type("pipelines:list"),
            CacheType::SemiDynamic
        );
        assert_eq!(
            PlmCache::detect_cache_type("run:events:789"),
            CacheType::Dynamic
        );
    }

    #[tokio::test]
    async fn test_plm_cache_operations() {
        let cache = PlmCache::new();
        let test_data = json!({"test": "data"});

        // Test insertion and retrieval
        cache
            .insert("pipeline:def:test".to_string(), test_data.clone())
            .await;
        let cached = cache.get("pipeline:def:test").await;
        assert_eq!(cached, Some(test_data));

        // Test cache miss
        let missing = cache.get("nonexistent:key").await;
        assert_eq!(missing, None);
    }

    #[tokio::test]
    async fn test_plm_cache_invalidation() {
        let cache = PlmCache::new();

        // Insert test data
        cache
            .insert(
                "pipeline:def:123".to_string(),
                json!({"id": "123", "name": "test"}),
            )
            .await;
        cache
            .insert(
                "pipeline:runs:123".to_string(),
                json!({"runs": []}),
            )
            .await;

        // Verify data exists
        assert!(cache.get("pipeline:def:123").await.is_some());
        assert!(cache.get("pipeline:runs:123").await.is_some());

        // Invalidate pipeline
        cache.invalidate_pipeline("123").await;

        // Verify data was invalidated
        assert!(cache.get("pipeline:def:123").await.is_none());
        assert!(cache.get("pipeline:runs:123").await.is_none());
    }

    #[tokio::test]
    async fn test_plm_cache_expiration() {
        let mut config = CacheConfig::default();
        config.custom_ttl.insert(CacheType::Dynamic, Duration::from_millis(50));
        
        let cache = PlmCache::with_config(config);
        
        // Insert dynamic data with short TTL
        cache
            .insert("run:events:123".to_string(), json!({"events": []}))
            .await;

        // Verify data exists
        assert!(cache.get("run:events:123").await.is_some());

        // Wait for expiration
        sleep(Duration::from_millis(100)).await;

        // Verify data expired
        assert!(cache.get("run:events:123").await.is_none());
    }

    #[tokio::test]
    async fn test_plm_cache_warming() {
        let cache = PlmCache::new();
        let pipeline_ids = vec!["pipe1".to_string(), "pipe2".to_string()];

        // Warm cache
        cache.warm_cache(&pipeline_ids).await;

        // Verify pipeline definitions were cached
        assert!(cache.get("pipeline:def:pipe1").await.is_some());
        assert!(cache.get("pipeline:def:pipe2").await.is_some());
        assert!(cache.get("pipelines:list").await.is_some());
    }

    #[tokio::test]
    async fn test_plm_cache_stats() {
        let cache = PlmCache::new();

        // Generate cache hits and misses
        cache.get("nonexistent").await; // miss
        cache
            .insert("test:key".to_string(), json!({"data": "test"}))
            .await; // insertion
        cache.get("test:key").await; // hit

        let stats = cache.get_stats().await;
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
        assert_eq!(stats.insertions, 1);
        assert!(stats.hit_rate() > 0.0);
    }
}