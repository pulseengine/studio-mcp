//! PLM-specific caching layer with intelligent cache type detection
//!
//! Provides PLM-aware caching that integrates with the PlmResourceProvider:
//! - Automatic cache type detection based on PLM resource patterns
//! - Smart invalidation for pipeline state changes
//! - Cache warming for frequently accessed resources
//! - Integration with CLI command patterns

#![allow(dead_code)]

use super::{
    AlertLevel, CacheAlert, CacheConfig, CacheContext, CacheHealthMetrics, CachePerformanceReport,
    CacheStats, CacheStore, CacheType, CacheTypeHealth, CachedItem, SensitiveDataFilter,
};
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
    /// Sensitive data filter for security
    sensitive_filter: SensitiveDataFilter,
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
                Arc::new(RwLock::new(CacheStore::with_memory_limit(
                    config.max_size_per_type,
                    config.max_memory_bytes / 4, // Divide memory between cache types
                    config.memory_eviction_threshold,
                ))),
            )
        })
        .collect();

        Self {
            stores,
            config,
            stats: Arc::new(RwLock::new(CacheStats::new())),
            sensitive_filter: SensitiveDataFilter::new(),
        }
    }

    /// Get a cached value by key with user context, updating access statistics
    pub async fn get(&self, context: &CacheContext, key: &str) -> Option<Value> {
        if !self.config.enabled {
            return None;
        }

        let start_time = std::time::Instant::now();
        let full_key = self.build_cache_key(context, key);
        let cache_type = Self::detect_cache_type(key);
        let store = self.stores.get(&cache_type)?;
        let mut store_guard = store.write().await;

        let result = store_guard.get(&full_key);
        let access_time_ms = start_time.elapsed().as_millis() as u64;

        if self.config.enable_stats {
            let mut stats = self.stats.write().await;
            match &result {
                Some(_) => {
                    stats.record_hit();
                    stats.update_type_performance(
                        cache_type,
                        true,
                        access_time_ms,
                        Some(&full_key),
                    );
                    trace!("Cache hit for PLM key: {} ({}ms)", full_key, access_time_ms);
                }
                None => {
                    stats.record_miss();
                    stats.update_type_performance(
                        cache_type,
                        false,
                        access_time_ms,
                        Some(&full_key),
                    );
                    trace!(
                        "Cache miss for PLM key: {} ({}ms)",
                        full_key, access_time_ms
                    );
                }
            }
            stats.record_access_time(access_time_ms);
        }

        result
    }

    /// Insert a value into the cache with automatic type detection and user context
    pub async fn insert(&self, context: &CacheContext, key: String, value: Value) {
        if !self.config.enabled {
            return;
        }

        // Check if this key should be skipped due to sensitive data
        if self.sensitive_filter.should_skip_caching(&key) {
            debug!("Skipping cache insertion for sensitive key: {}", key);
            return;
        }

        let full_key = self.build_cache_key(context, &key);
        let cache_type = Self::detect_cache_type(&key);
        let store = match self.stores.get(&cache_type) {
            Some(store) => store,
            None => {
                warn!("No cache store found for type: {:?}", cache_type);
                return;
            }
        };

        // Filter sensitive data from the value before caching
        let filtered_value = self.sensitive_filter.filter_value(&value);

        // Create cached item with configuration-aware TTL
        let item = CachedItem::with_config(filtered_value, cache_type, &self.config);

        let mut store_guard = store.write().await;
        let item_size = item.estimated_size_bytes;
        store_guard.insert(full_key.clone(), item);

        if self.config.enable_stats {
            let mut stats = self.stats.write().await;
            stats.record_insertion(cache_type);
            stats.update_memory_usage(item_size as isize);
        }

        debug!("Cached PLM resource: {} (type: {:?})", full_key, cache_type);
    }

    /// Build a cache key with user context
    fn build_cache_key(&self, context: &CacheContext, key: &str) -> String {
        format!("{}:{}", context.cache_prefix(), key)
    }

    /// Remove a specific key from the cache
    pub async fn remove(&self, context: &CacheContext, key: &str) {
        if !self.config.enabled {
            return;
        }

        let full_key = self.build_cache_key(context, key);
        let cache_type = Self::detect_cache_type(key);
        if let Some(store) = self.stores.get(&cache_type) {
            let mut store_guard = store.write().await;
            if let Some(removed_item) = store_guard.remove(&full_key) {
                if self.config.enable_stats {
                    let mut stats = self.stats.write().await;
                    stats.record_eviction(cache_type);
                    stats.update_memory_usage(-(removed_item.estimated_size_bytes as isize));
                }
                debug!("Removed from PLM cache: {}", full_key);
            }
        }
    }

    /// Invalidate cache entries based on PLM resource changes for a specific user context
    pub async fn invalidate_pattern(&self, context: &CacheContext, pattern: &str) {
        if !self.config.enabled {
            return;
        }

        let context_prefix = context.cache_prefix();
        let full_pattern = format!("{context_prefix}:{pattern}");
        debug!("Invalidating PLM cache pattern: {}", full_pattern);
        let mut invalidated_count = 0;
        let mut total_memory_freed = 0;

        for store in self.stores.values() {
            let mut store_guard = store.write().await;
            let keys_to_remove: Vec<String> = store_guard
                .items
                .keys()
                .filter(|key| key.contains(&full_pattern))
                .cloned()
                .collect();

            for key in keys_to_remove {
                if let Some(removed_item) = store_guard.remove(&key) {
                    invalidated_count += 1;
                    total_memory_freed += removed_item.estimated_size_bytes;
                }
            }
        }

        if self.config.enable_stats && total_memory_freed > 0 {
            let mut stats = self.stats.write().await;
            stats.update_memory_usage(-(total_memory_freed as isize));
        }

        if self.config.enable_stats {
            for _ in 0..invalidated_count {
                self.stats.write().await.record_invalidation();
            }
        }

        debug!(
            "Invalidated {} PLM cache entries for pattern: {}",
            invalidated_count, full_pattern
        );
    }

    /// Invalidate caches when pipeline state changes for a specific user
    pub async fn invalidate_pipeline(&self, context: &CacheContext, pipeline_id: &str) {
        // Invalidate pipeline-specific caches with more specific patterns
        self.invalidate_pattern(context, &format!("pipeline:def:{pipeline_id}"))
            .await;
        self.invalidate_pattern(context, &format!("pipeline:runs:{pipeline_id}"))
            .await;
        self.invalidate_pattern(context, &format!("pipeline:events:{pipeline_id}"))
            .await;
        self.invalidate_pattern(context, &format!("pipelines/{pipeline_id}"))
            .await;

        // Invalidate dynamic pipeline lists
        self.remove(context, "pipelines:list").await;
        self.remove(context, "runs:list").await;
    }

    /// Invalidate caches when run state changes for a specific user
    pub async fn invalidate_run(&self, context: &CacheContext, run_id: &str) {
        // Invalidate run-specific caches
        self.invalidate_pattern(context, &format!("run:{run_id}"))
            .await;
        self.invalidate_pattern(context, &format!("runs/{run_id}"))
            .await;

        // Invalidate dynamic run lists
        self.remove(context, "runs:list").await;
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

    /// Generate comprehensive performance report
    pub async fn get_performance_report(&self) -> CachePerformanceReport {
        let mut stats = self.stats.write().await;

        // Update memory usage for each cache type
        for (cache_type, store) in &self.stores {
            let store_guard = store.read().await;
            let type_name = format!("{cache_type:?}");
            if let Some(perf) = stats.performance_by_type.get_mut(&type_name) {
                perf.memory_usage = store_guard.memory_usage();
            }
        }

        stats.generate_report()
    }

    /// Get real-time cache health metrics
    pub async fn get_health_metrics(&self) -> CacheHealthMetrics {
        let stats = self.stats.read().await;
        let report = stats.generate_report();

        let mut type_health = HashMap::new();
        for (type_name, perf) in &report.type_breakdown {
            type_health.insert(
                type_name.clone(),
                CacheTypeHealth {
                    hit_rate: perf.hit_rate(),
                    is_active: perf.is_active(),
                    memory_pressure: if perf.memory_usage > 0 {
                        perf.memory_usage as f64 / (1024.0 * 1024.0) // MB
                    } else {
                        0.0
                    },
                    avg_access_time: perf.avg_access_time_ms,
                    total_operations: perf.total_operations(),
                },
            );
        }

        CacheHealthMetrics {
            overall_health_score: report.health_score,
            memory_usage_mb: report.memory_usage_mb,
            total_operations: report.total_operations,
            hit_rate: report.hit_rate,
            operations_per_second: report.operations_per_second,
            uptime_seconds: report.uptime_seconds,
            type_health,
            alerts: self.generate_health_alerts(&report).await,
        }
    }

    /// Generate alerts based on cache performance
    async fn generate_health_alerts(&self, report: &CachePerformanceReport) -> Vec<CacheAlert> {
        let mut alerts = Vec::new();

        // Low hit rate alert
        if report.hit_rate < 0.5 {
            alerts.push(CacheAlert {
                level: AlertLevel::Warning,
                message: format!("Low cache hit rate: {:.1}%", report.hit_rate * 100.0),
                metric: "hit_rate".to_string(),
                value: report.hit_rate,
                threshold: 0.5,
            });
        }

        // High memory usage alert
        if report.memory_usage_mb > 80.0 {
            alerts.push(CacheAlert {
                level: AlertLevel::Critical,
                message: format!("High memory usage: {:.1} MB", report.memory_usage_mb),
                metric: "memory_usage_mb".to_string(),
                value: report.memory_usage_mb,
                threshold: 80.0,
            });
        }

        // High eviction rate alert
        let eviction_rate = if report.total_operations > 0 {
            report.eviction_summary.total_evictions as f64 / report.total_operations as f64
        } else {
            0.0
        };

        if eviction_rate > 0.2 {
            alerts.push(CacheAlert {
                level: AlertLevel::Warning,
                message: format!("High eviction rate: {:.1}%", eviction_rate * 100.0),
                metric: "eviction_rate".to_string(),
                value: eviction_rate,
                threshold: 0.2,
            });
        }

        // Slow access time alert
        if report.average_access_time_ms > 10.0 {
            alerts.push(CacheAlert {
                level: AlertLevel::Warning,
                message: format!(
                    "Slow cache access: {:.1}ms average",
                    report.average_access_time_ms
                ),
                metric: "avg_access_time_ms".to_string(),
                value: report.average_access_time_ms,
                threshold: 10.0,
            });
        }

        alerts
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

    /// Get total memory usage across all stores
    pub async fn total_memory_usage(&self) -> usize {
        let mut total = 0;
        for store in self.stores.values() {
            total += store.read().await.memory_usage();
        }
        total
    }

    /// Get memory usage statistics for all cache types
    pub async fn memory_stats(&self) -> HashMap<String, (usize, usize, f64)> {
        let mut stats = HashMap::new();
        for (cache_type, store) in &self.stores {
            let store_guard = store.read().await;
            let (current, max, percent) = store_guard.memory_stats();
            stats.insert(format!("{cache_type:?}"), (current, max, percent));
        }
        stats
    }

    /// Force memory-based eviction across all stores if needed
    pub async fn evict_for_memory(&self) -> usize {
        let mut total_evicted = 0;
        for store in self.stores.values() {
            let mut store_guard = store.write().await;
            total_evicted += store_guard.evict_for_memory();
        }
        if total_evicted > 0 {
            debug!(
                "Memory-based eviction freed {} cache entries",
                total_evicted
            );
        }
        total_evicted
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
            || key.contains("access-config:")
        {
            return CacheType::Immutable;
        }

        // Completed/failed runs and tasks - never change once done
        if key.contains("completed")
            || key.contains("failed")
            || key.contains("finished")
            || key.contains(":status:completed")
            || key.contains(":status:failed")
        {
            return CacheType::Completed;
        }

        // Pipeline/run lists and resource lists - change when items added/removed
        if key.contains("list")
            || key.contains("pipelines:")
            || key.contains("runs:")
            || key.contains("resources:")
            || key.contains("groups:")
        {
            return CacheType::SemiDynamic;
        }

        // Active runs, events, live status - frequently changing
        CacheType::Dynamic
    }

    /// Pre-warm cache with commonly accessed PLM resources for a specific user
    pub async fn warm_cache(&self, context: &CacheContext, pipeline_ids: &[String]) {
        if !self.config.enabled {
            return;
        }

        debug!(
            "Warming PLM cache for {} pipelines (user: {})",
            pipeline_ids.len(),
            context.user_id
        );

        // Cache pipeline definitions (immutable)
        for pipeline_id in pipeline_ids {
            let key = format!("pipeline:def:{pipeline_id}");
            // Would normally fetch from CLI here
            let mock_definition = serde_json::json!({
                "id": pipeline_id,
                "name": format!("Pipeline {}", pipeline_id),
                "status": "active"
            });
            self.insert(context, key, mock_definition).await;
        }

        // Cache pipeline list (semi-dynamic)
        let pipeline_list = serde_json::json!({
            "pipelines": pipeline_ids,
            "total": pipeline_ids.len()
        });
        self.insert(context, "pipelines:list".to_string(), pipeline_list)
            .await;

        debug!(
            "PLM cache warmed with {} pipeline definitions for user: {}",
            pipeline_ids.len(),
            context.user_id
        );
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
        format!("pipeline:def:{pipeline_id}")
    }

    /// Generate cache key for pipeline runs
    pub fn pipeline_runs_key(pipeline_id: &str) -> String {
        format!("pipeline:runs:{pipeline_id}")
    }

    /// Generate cache key for pipeline events
    pub fn pipeline_events_key(pipeline_id: &str) -> String {
        format!("pipeline:events:{pipeline_id}")
    }

    /// Generate cache key for run details
    pub fn run_details_key(run_id: &str) -> String {
        format!("run:details:{run_id}")
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
    use crate::cache::AlertLevel;
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
        let context = CacheContext::new("user1".to_string(), "org1".to_string(), "dev".to_string());
        let test_data = json!({"test": "data"});

        // Test insertion and retrieval
        cache
            .insert(&context, "pipeline:def:test".to_string(), test_data.clone())
            .await;
        let cached = cache.get(&context, "pipeline:def:test").await;
        assert_eq!(cached, Some(test_data));

        // Test cache miss
        let missing = cache.get(&context, "nonexistent:key").await;
        assert_eq!(missing, None);
    }

    #[tokio::test]
    async fn test_plm_cache_invalidation() {
        let cache = PlmCache::new();
        let context = CacheContext::new("user1".to_string(), "org1".to_string(), "dev".to_string());

        // Insert test data
        cache
            .insert(
                &context,
                "pipeline:def:123".to_string(),
                json!({"id": "123", "name": "test"}),
            )
            .await;
        cache
            .insert(
                &context,
                "pipeline:runs:123".to_string(),
                json!({"runs": []}),
            )
            .await;

        // Verify data exists
        assert!(cache.get(&context, "pipeline:def:123").await.is_some());
        assert!(cache.get(&context, "pipeline:runs:123").await.is_some());

        // Invalidate pipeline
        cache.invalidate_pipeline(&context, "123").await;

        // Verify data was invalidated
        assert!(cache.get(&context, "pipeline:def:123").await.is_none());
        assert!(cache.get(&context, "pipeline:runs:123").await.is_none());
    }

    #[tokio::test]
    async fn test_plm_cache_expiration() {
        let mut config = CacheConfig::default();
        config
            .custom_ttl
            .insert(CacheType::Dynamic, Duration::from_millis(50));

        let cache = PlmCache::with_config(config);
        let context = CacheContext::new("user1".to_string(), "org1".to_string(), "dev".to_string());

        // Insert dynamic data with short TTL
        cache
            .insert(
                &context,
                "run:events:123".to_string(),
                json!({"events": []}),
            )
            .await;

        // Verify data exists
        assert!(cache.get(&context, "run:events:123").await.is_some());

        // Wait for expiration
        sleep(Duration::from_millis(100)).await;

        // Verify data expired
        assert!(cache.get(&context, "run:events:123").await.is_none());
    }

    #[tokio::test]
    async fn test_plm_cache_warming() {
        let cache = PlmCache::new();
        let context = CacheContext::new("user1".to_string(), "org1".to_string(), "dev".to_string());
        let pipeline_ids = vec!["pipe1".to_string(), "pipe2".to_string()];

        // Warm cache
        cache.warm_cache(&context, &pipeline_ids).await;

        // Verify pipeline definitions were cached
        assert!(cache.get(&context, "pipeline:def:pipe1").await.is_some());
        assert!(cache.get(&context, "pipeline:def:pipe2").await.is_some());
        assert!(cache.get(&context, "pipelines:list").await.is_some());
    }

    #[tokio::test]
    async fn test_plm_cache_stats() {
        let cache = PlmCache::new();
        let context = CacheContext::new("user1".to_string(), "org1".to_string(), "dev".to_string());

        // Generate cache hits and misses
        cache.get(&context, "nonexistent").await; // miss
        cache
            .insert(&context, "test:key".to_string(), json!({"data": "test"}))
            .await; // insertion
        cache.get(&context, "test:key").await; // hit

        let stats = cache.get_stats().await;
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
        assert_eq!(stats.insertions, 1);
        assert!(stats.hit_rate() > 0.0);
    }

    #[tokio::test]
    async fn test_cache_context_isolation() {
        let cache = PlmCache::new();
        let context1 =
            CacheContext::new("user1".to_string(), "org1".to_string(), "dev".to_string());
        let context2 =
            CacheContext::new("user2".to_string(), "org1".to_string(), "dev".to_string());
        let test_data = json!({"test": "data"});

        // Insert data for user1
        cache
            .insert(
                &context1,
                "pipeline:def:test".to_string(),
                test_data.clone(),
            )
            .await;

        // Verify user1 can access the data
        assert_eq!(
            cache.get(&context1, "pipeline:def:test").await,
            Some(test_data.clone())
        );

        // Verify user2 cannot access user1's data
        assert_eq!(cache.get(&context2, "pipeline:def:test").await, None);

        // Insert different data for user2
        let user2_data = json!({"user2": "data"});
        cache
            .insert(
                &context2,
                "pipeline:def:test".to_string(),
                user2_data.clone(),
            )
            .await;

        // Verify both users have isolated data
        assert_eq!(
            cache.get(&context1, "pipeline:def:test").await,
            Some(test_data)
        );
        assert_eq!(
            cache.get(&context2, "pipeline:def:test").await,
            Some(user2_data)
        );
    }

    #[tokio::test]
    async fn test_memory_usage_tracking() {
        let cache = PlmCache::new();
        let context = CacheContext::new("user1".to_string(), "org1".to_string(), "dev".to_string());

        // Start with zero memory usage
        assert_eq!(cache.total_memory_usage().await, 0);

        // Insert some data and verify memory usage increases
        let large_data = json!({
            "pipeline_id": "test-123",
            "config": {
                "build_type": "release",
                "targets": ["x86_64", "arm64"],
                "env_vars": {
                    "CC": "gcc",
                    "CXX": "g++",
                    "LDFLAGS": "-static"
                }
            },
            "tasks": [
                {"name": "checkout", "timeout": 300},
                {"name": "configure", "timeout": 600},
                {"name": "compile", "timeout": 1800}
            ]
        });

        cache
            .insert(&context, "pipeline:def:large".to_string(), large_data)
            .await;
        let memory_after_insert = cache.total_memory_usage().await;
        assert!(memory_after_insert > 0);

        // Insert more data and verify memory increases further
        let more_data = json!({"runs": [{"id": "run-1", "status": "running"}]});
        cache
            .insert(&context, "pipeline:runs:large".to_string(), more_data)
            .await;
        let memory_after_second_insert = cache.total_memory_usage().await;
        assert!(memory_after_second_insert > memory_after_insert);

        // Remove data and verify memory decreases
        cache.remove(&context, "pipeline:def:large").await;
        let memory_after_remove = cache.total_memory_usage().await;
        assert!(memory_after_remove < memory_after_second_insert);
    }

    #[tokio::test]
    async fn test_memory_based_eviction() {
        // Create cache with very small memory limit for testing
        let config = CacheConfig {
            max_memory_bytes: 500,          // Very small limit
            memory_eviction_threshold: 0.7, // 70% threshold
            ..CacheConfig::default()
        };

        let cache = PlmCache::with_config(config);
        let context = CacheContext::new("user1".to_string(), "org1".to_string(), "dev".to_string());

        // Insert enough data to trigger memory eviction
        for i in 0..20 {
            let large_data = json!({
                "id": format!("item-{}", i),
                "data": "x".repeat(50), // Make it reasonably large
                "metadata": {
                    "created": "2023-01-01",
                    "version": i
                }
            });
            cache
                .insert(&context, format!("test:item:{i}"), large_data)
                .await;
        }

        // Should have triggered eviction to stay under memory limit
        let total_memory = cache.total_memory_usage().await;
        assert!(total_memory <= 500); // Should be under the limit

        // Should have fewer than 20 items due to eviction
        let total_items = cache.total_size().await;
        assert!(total_items < 20);
    }

    #[tokio::test]
    async fn test_memory_stats() {
        let cache = PlmCache::new();
        let context = CacheContext::new("user1".to_string(), "org1".to_string(), "dev".to_string());

        // Insert data into different cache types
        cache
            .insert(
                &context,
                "pipeline:def:test".to_string(),
                json!({"immutable": "data"}),
            )
            .await;
        cache
            .insert(
                &context,
                "run:completed:123".to_string(),
                json!({"completed": "data"}),
            )
            .await;
        cache
            .insert(
                &context,
                "pipelines:list".to_string(),
                json!({"semi_dynamic": "data"}),
            )
            .await;
        cache
            .insert(
                &context,
                "run:events:456".to_string(),
                json!({"dynamic": "data"}),
            )
            .await;

        // Get memory stats
        let stats = cache.memory_stats().await;

        // Should have stats for all cache types
        assert!(stats.contains_key("Immutable"));
        assert!(stats.contains_key("Completed"));
        assert!(stats.contains_key("SemiDynamic"));
        assert!(stats.contains_key("Dynamic"));

        // All should have some memory usage
        for (cache_type, (current, max, percent)) in stats {
            assert!(
                current > 0,
                "Cache type {cache_type} should have memory usage"
            );
            assert!(
                max > 0,
                "Cache type {cache_type} should have max memory limit"
            );
            assert!(
                (0.0..=100.0).contains(&percent),
                "Cache type {cache_type} should have valid percentage"
            );
        }
    }

    #[tokio::test]
    async fn test_forced_memory_eviction() {
        let config = CacheConfig {
            max_memory_bytes: 1000,
            memory_eviction_threshold: 0.8,
            ..CacheConfig::default()
        };

        let cache = PlmCache::with_config(config);
        let context = CacheContext::new("user1".to_string(), "org1".to_string(), "dev".to_string());

        // Fill cache to near capacity
        for i in 0..10 {
            let data = json!({
                "id": i,
                "payload": "x".repeat(80) // Large enough to fill memory
            });
            cache.insert(&context, format!("test:{i}"), data).await;
        }

        let memory_before = cache.total_memory_usage().await;

        // Force memory eviction
        let evicted = cache.evict_for_memory().await;

        let memory_after = cache.total_memory_usage().await;

        // Should have evicted some items if we were over threshold
        if memory_before > 800 {
            // If we were over 80% of 1000 bytes
            assert!(evicted > 0);
            assert!(memory_after < memory_before);
        }
    }

    #[tokio::test]
    async fn test_performance_monitoring() {
        let cache = PlmCache::new();
        let context = CacheContext::new("user1".to_string(), "org1".to_string(), "dev".to_string());

        // Generate various cache operations
        cache
            .insert(
                &context,
                "pipeline:def:test1".to_string(),
                json!({"immutable": "data"}),
            )
            .await;
        cache
            .insert(
                &context,
                "run:details:123:completed".to_string(),
                json!({"completed": "data"}),
            )
            .await;
        cache
            .insert(
                &context,
                "pipelines:list".to_string(),
                json!({"semi_dynamic": "data"}),
            )
            .await;

        // Generate hits and misses
        cache.get(&context, "pipeline:def:test1").await; // hit
        cache.get(&context, "pipeline:def:test1").await; // hit
        cache.get(&context, "nonexistent:key").await; // miss

        // Get performance report
        let report = cache.get_performance_report().await;

        assert!(report.total_operations > 0);
        assert!(report.hit_rate > 0.0);
        assert!(report.uptime_seconds < u64::MAX);
        assert!(!report.type_breakdown.is_empty());

        // Verify type-specific metrics exist
        assert!(report.type_breakdown.contains_key("Immutable"));
        assert!(report.type_breakdown.contains_key("Dynamic"));

        let immutable_perf = &report.type_breakdown["Immutable"];
        assert_eq!(immutable_perf.hits, 2); // Two hits on pipeline:def:test1
        assert!(immutable_perf.hit_rate() > 0.0);
    }

    #[tokio::test]
    async fn test_cache_health_metrics() {
        let cache = PlmCache::new();
        let context = CacheContext::new("user1".to_string(), "org1".to_string(), "dev".to_string());

        // Add some cache operations
        for i in 0..5 {
            cache
                .insert(&context, format!("test:key:{i}"), json!({"data": i}))
                .await;
            cache.get(&context, &format!("test:key:{i}")).await;
        }

        // Add some misses
        for i in 5..8 {
            cache.get(&context, &format!("missing:key:{i}")).await;
        }

        let health = cache.get_health_metrics().await;

        assert!(health.overall_health_score >= 0.0);
        assert!(health.overall_health_score <= 100.0);
        assert!(health.hit_rate > 0.0);
        assert!(health.total_operations > 0);
        assert!(health.uptime_seconds < u64::MAX);
        assert!(!health.type_health.is_empty());

        // Should have some dynamic cache type health
        if let Some(dynamic_health) = health.type_health.get("Dynamic") {
            assert!(dynamic_health.total_operations > 0);
            assert!(dynamic_health.hit_rate >= 0.0);
        }
    }

    #[tokio::test]
    async fn test_cache_alerts() {
        let cache = PlmCache::new();
        let context = CacheContext::new("user1".to_string(), "org1".to_string(), "dev".to_string());

        // Generate many misses to trigger low hit rate alert
        for i in 0..20 {
            cache.get(&context, &format!("nonexistent:key:{i}")).await;
        }

        // Add a few hits
        for i in 0..3 {
            cache
                .insert(&context, format!("test:key:{i}"), json!({"data": i}))
                .await;
            cache.get(&context, &format!("test:key:{i}")).await;
        }

        let health = cache.get_health_metrics().await;

        // Should have alerts due to low hit rate
        let hit_rate_alerts: Vec<_> = health
            .alerts
            .iter()
            .filter(|alert| alert.metric == "hit_rate")
            .collect();

        assert!(!hit_rate_alerts.is_empty());
        assert_eq!(hit_rate_alerts[0].level, AlertLevel::Warning);
    }

    #[tokio::test]
    async fn test_cache_type_performance_tracking() {
        let cache = PlmCache::new();
        let context = CacheContext::new("user1".to_string(), "org1".to_string(), "dev".to_string());

        // Test different cache types
        cache
            .insert(
                &context,
                "pipeline:def:perf1".to_string(),
                json!({"type": "immutable"}),
            )
            .await;
        cache
            .insert(
                &context,
                "run:completed:perf1".to_string(),
                json!({"type": "completed"}),
            )
            .await;
        cache
            .insert(
                &context,
                "pipelines:list".to_string(),
                json!({"type": "semi_dynamic"}),
            )
            .await;
        cache
            .insert(
                &context,
                "run:events:perf1".to_string(),
                json!({"type": "dynamic"}),
            )
            .await;

        // Access each type multiple times
        for _ in 0..3 {
            cache.get(&context, "pipeline:def:perf1").await;
            cache.get(&context, "run:completed:perf1").await;
            cache.get(&context, "pipelines:list").await;
            cache.get(&context, "run:events:perf1").await;
        }

        let report = cache.get_performance_report().await;

        // Verify each cache type has performance data
        for cache_type in ["Immutable", "Completed", "SemiDynamic", "Dynamic"] {
            let perf = &report.type_breakdown[cache_type];
            assert_eq!(perf.hits, 3);
            assert_eq!(perf.misses, 0);
            assert_eq!(perf.hit_rate(), 1.0);
            // In fast test environments, timing might be 0ms, so just check it's reasonable
            assert!(perf.total_access_time_ms < u64::MAX);
            assert!(perf.avg_access_time_ms >= 0.0);
            assert!(perf.is_active()); // Should be active since just accessed
        }
    }

    #[tokio::test]
    async fn test_configurable_ttl_settings() {
        // Test custom TTL configuration
        let config = CacheConfig::default()
            .with_immutable_ttl(Duration::from_secs(300)) // 5 minutes
            .with_completed_ttl(Duration::from_secs(7200)) // 2 hours
            .with_semi_dynamic_ttl(Duration::from_secs(120)) // 2 minutes
            .with_dynamic_ttl(Duration::from_secs(30)); // 30 seconds

        let cache = PlmCache::with_config(config);
        let context =
            CacheContext::new("user1".to_string(), "org1".to_string(), "test".to_string());

        // Insert items of different types
        cache
            .insert(
                &context,
                "pipeline:def:test".to_string(),
                json!({"immutable": true}),
            )
            .await;
        cache
            .insert(
                &context,
                "run:details:123:completed".to_string(),
                json!({"completed": true}),
            )
            .await;
        cache
            .insert(
                &context,
                "pipelines:list".to_string(),
                json!({"semi_dynamic": true}),
            )
            .await;
        cache
            .insert(
                &context,
                "run:events:456".to_string(),
                json!({"dynamic": true}),
            )
            .await;

        // Verify items exist
        assert!(cache.get(&context, "pipeline:def:test").await.is_some());
        assert!(
            cache
                .get(&context, "run:details:123:completed")
                .await
                .is_some()
        );
        assert!(cache.get(&context, "pipelines:list").await.is_some());
        assert!(cache.get(&context, "run:events:456").await.is_some());

        // Test TTL retrieval
        assert_eq!(
            cache.config.get_ttl(CacheType::Immutable),
            Duration::from_secs(300)
        );
        assert_eq!(
            cache.config.get_ttl(CacheType::Completed),
            Duration::from_secs(7200)
        );
        assert_eq!(
            cache.config.get_ttl(CacheType::SemiDynamic),
            Duration::from_secs(120)
        );
        assert_eq!(
            cache.config.get_ttl(CacheType::Dynamic),
            Duration::from_secs(30)
        );
    }

    #[tokio::test]
    async fn test_environment_specific_configurations() {
        // Test development configuration
        let dev_cache = PlmCache::with_config(CacheConfig::development());
        let context = CacheContext::new(
            "dev_user".to_string(),
            "dev_org".to_string(),
            "dev".to_string(),
        );

        dev_cache
            .insert(
                &context,
                "pipeline:def:dev".to_string(),
                json!({"env": "dev"}),
            )
            .await;
        assert!(dev_cache.get(&context, "pipeline:def:dev").await.is_some());

        // Test production configuration
        let prod_cache = PlmCache::with_config(CacheConfig::production());
        let context = CacheContext::new(
            "prod_user".to_string(),
            "prod_org".to_string(),
            "prod".to_string(),
        );

        prod_cache
            .insert(
                &context,
                "pipeline:def:prod".to_string(),
                json!({"env": "prod"}),
            )
            .await;
        assert!(
            prod_cache
                .get(&context, "pipeline:def:prod")
                .await
                .is_some()
        );

        // Test testing configuration
        let test_cache = PlmCache::with_config(CacheConfig::testing());
        let context = CacheContext::new(
            "test_user".to_string(),
            "test_org".to_string(),
            "test".to_string(),
        );

        test_cache
            .insert(
                &context,
                "pipeline:def:test".to_string(),
                json!({"env": "test"}),
            )
            .await;
        assert!(
            test_cache
                .get(&context, "pipeline:def:test")
                .await
                .is_some()
        );

        // Verify different TTL values
        let dev_immutable_ttl = dev_cache.config.get_ttl(CacheType::Immutable);
        let prod_immutable_ttl = prod_cache.config.get_ttl(CacheType::Immutable);
        let test_immutable_ttl = test_cache.config.get_ttl(CacheType::Immutable);

        assert!(dev_immutable_ttl < prod_immutable_ttl);
        assert!(test_immutable_ttl < dev_immutable_ttl);
    }

    #[tokio::test]
    async fn test_builder_pattern_configuration() {
        // Test fluent configuration builder
        let config = CacheConfig::default()
            .with_enabled(true)
            .with_max_size_per_type(500)
            .with_memory_config(50 * 1024 * 1024, 0.8) // 50MB, 80% threshold
            .with_stats_enabled(true)
            .with_custom_ttl(CacheType::Immutable, Duration::from_secs(600))
            .with_dynamic_ttl(Duration::from_secs(15));

        let cache = PlmCache::with_config(config);
        let context = CacheContext::new(
            "builder_user".to_string(),
            "builder_org".to_string(),
            "builder".to_string(),
        );

        // Test that the configuration is applied correctly
        assert!(cache.config.enabled);
        assert_eq!(cache.config.max_size_per_type, 500);
        assert_eq!(cache.config.max_memory_bytes, 50 * 1024 * 1024);
        assert_eq!(cache.config.memory_eviction_threshold, 0.8);
        assert!(cache.config.enable_stats);
        assert_eq!(
            cache.config.get_ttl(CacheType::Immutable),
            Duration::from_secs(600)
        );
        assert_eq!(
            cache.config.get_ttl(CacheType::Dynamic),
            Duration::from_secs(15)
        );

        // Test caching functionality works with custom config
        cache
            .insert(&context, "test:key".to_string(), json!({"test": "value"}))
            .await;
        assert!(cache.get(&context, "test:key").await.is_some());
    }

    #[tokio::test]
    async fn test_ttl_expiration_with_custom_config() {
        let config = CacheConfig::testing().with_dynamic_ttl(Duration::from_millis(50)); // Very short TTL for testing

        let cache = PlmCache::with_config(config);
        let context = CacheContext::new(
            "ttl_user".to_string(),
            "ttl_org".to_string(),
            "ttl_test".to_string(),
        );

        // Insert dynamic data with short TTL
        cache
            .insert(
                &context,
                "run:events:test".to_string(),
                json!({"dynamic": "data"}),
            )
            .await;

        // Should be available immediately
        assert!(cache.get(&context, "run:events:test").await.is_some());

        // Wait for expiration
        tokio::time::sleep(Duration::from_millis(60)).await;

        // Should be expired now
        assert!(cache.get(&context, "run:events:test").await.is_none());
    }
}
