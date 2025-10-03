//! Intelligent caching layer for PLM resources
//!
//! Implements state-aware caching with different TTL policies based on data mutability:
//! - Immutable: Pipeline definitions, task libraries (1 hour)
//! - Completed: Finished runs/tasks (24 hours)
//! - Semi-dynamic: Lists, resources (10 minutes)
//! - Dynamic: Active runs, live events (1 minute)

#![allow(dead_code)]

use serde_json::Value;
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Comprehensive performance report for cache monitoring
#[derive(Debug, Clone)]
pub struct CachePerformanceReport {
    pub total_operations: u64,
    pub hit_rate: f64,
    pub average_access_time_ms: f64,
    pub operations_per_second: f64,
    pub memory_usage_mb: f64,
    pub memory_efficiency: f64,
    pub health_score: f64,
    pub uptime_seconds: u64,
    pub eviction_summary: EvictionSummary,
    pub type_breakdown: HashMap<String, CacheTypePerformance>,
}

/// Summary of cache eviction activity
#[derive(Debug, Default, Clone)]
pub struct EvictionSummary {
    pub total_evictions: u64,
    pub memory_evictions: u64,
    pub size_evictions: u64,
    pub lru_evictions: u64,
}

/// Real-time cache health metrics
#[derive(Debug, Clone)]
pub struct CacheHealthMetrics {
    pub overall_health_score: f64,
    pub memory_usage_mb: f64,
    pub total_operations: u64,
    pub hit_rate: f64,
    pub operations_per_second: f64,
    pub uptime_seconds: u64,
    pub type_health: HashMap<String, CacheTypeHealth>,
    pub alerts: Vec<CacheAlert>,
}

/// Health metrics for specific cache type
#[derive(Debug, Clone)]
pub struct CacheTypeHealth {
    pub hit_rate: f64,
    pub is_active: bool,
    pub memory_pressure: f64,
    pub avg_access_time: f64,
    pub total_operations: u64,
}

/// Cache performance alert
#[derive(Debug, Clone)]
pub struct CacheAlert {
    pub level: AlertLevel,
    pub message: String,
    pub metric: String,
    pub value: f64,
    pub threshold: f64,
}

/// Alert severity levels
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AlertLevel {
    Info,
    Warning,
    Critical,
}

pub mod invalidation_service;
pub mod plm_cache;
pub mod sensitive_filter;

pub use invalidation_service::CacheInvalidationService;
pub use plm_cache::PlmCache;
pub use sensitive_filter::SensitiveDataFilter;

/// User context for cache isolation
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CacheContext {
    /// User identifier (username or user ID)
    pub user_id: String,
    /// Organization/instance identifier
    pub org_id: String,
    /// Environment (dev, staging, prod)
    pub environment: String,
}

impl CacheContext {
    /// Create a new cache context
    pub fn new(user_id: String, org_id: String, environment: String) -> Self {
        Self {
            user_id,
            org_id,
            environment,
        }
    }

    /// Create a cache context from auth credentials
    pub fn from_auth(instance_id: &str, username: &str, environment: &str) -> Self {
        Self {
            user_id: username.to_string(),
            org_id: instance_id.to_string(),
            environment: environment.to_string(),
        }
    }

    /// Generate cache key prefix with user context
    pub fn cache_prefix(&self) -> String {
        format!(
            "user:{}:org:{}:env:{}",
            self.sanitize_key_component(&self.user_id),
            self.sanitize_key_component(&self.org_id),
            self.sanitize_key_component(&self.environment)
        )
    }

    /// Sanitize cache key components to prevent collision
    fn sanitize_key_component(&self, component: &str) -> String {
        component
            .chars()
            .map(|c| match c {
                ':' => '_',
                ' ' => '_',
                '\t' | '\n' | '\r' => '_',
                c if c.is_alphanumeric() || c == '-' || c == '.' => c,
                _ => '_',
            })
            .collect()
    }
}

/// Cache item with metadata
#[derive(Debug, Clone)]
pub struct CachedItem {
    pub data: Value,
    pub cached_at: Instant,
    pub ttl: Duration,
    pub cache_type: CacheType,
    pub access_count: u64,
    pub last_accessed: Instant,
    pub estimated_size_bytes: usize,
}

impl CachedItem {
    pub fn new(data: Value, cache_type: CacheType) -> Self {
        let ttl = cache_type.default_ttl();
        let now = Instant::now();
        let estimated_size = Self::estimate_size(&data);

        Self {
            data,
            cached_at: now,
            ttl,
            cache_type,
            access_count: 0,
            last_accessed: now,
            estimated_size_bytes: estimated_size,
        }
    }

    /// Create a new cached item with custom TTL from configuration
    pub fn with_config(data: Value, cache_type: CacheType, config: &CacheConfig) -> Self {
        let ttl = config.get_ttl(cache_type);
        let now = Instant::now();
        let estimated_size = Self::estimate_size(&data);

        Self {
            data,
            cached_at: now,
            ttl,
            cache_type,
            access_count: 0,
            last_accessed: now,
            estimated_size_bytes: estimated_size,
        }
    }

    /// Estimate memory usage of a JSON value in bytes
    fn estimate_size(value: &Value) -> usize {
        match value {
            Value::Null => 4,
            Value::Bool(_) => 1,
            Value::Number(_) => 8,
            Value::String(s) => s.len() + 24, // String overhead
            Value::Array(arr) => {
                24 + arr.iter().map(Self::estimate_size).sum::<usize>() // Vec overhead
            }
            Value::Object(obj) => {
                32 + obj
                    .iter()
                    .map(|(k, v)| {
                        k.len() + 24 + Self::estimate_size(v) // HashMap overhead + key + value
                    })
                    .sum::<usize>()
            }
        }
    }

    pub fn is_expired(&self) -> bool {
        // All cache types now respect their TTL
        self.cached_at.elapsed() > self.ttl
    }

    pub fn access(&mut self) -> &Value {
        self.access_count += 1;
        self.last_accessed = Instant::now();
        &self.data
    }
}

/// Cache type determines TTL and invalidation behavior
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CacheType {
    /// Pipeline definitions, task libraries - rarely change
    Immutable,
    /// Finished runs/tasks - stable data that expires after 24 hours
    Completed,
    /// Lists, resources - change when items added/removed
    SemiDynamic,
    /// Active runs, events - frequently changing
    Dynamic,
}

impl CacheType {
    pub fn default_ttl(&self) -> Duration {
        match self {
            CacheType::Immutable => Duration::from_secs(3600), // 1 hour
            CacheType::Completed => Duration::from_secs(86400), // 24 hours
            CacheType::SemiDynamic => Duration::from_secs(600), // 10 minutes
            CacheType::Dynamic => Duration::from_secs(60),     // 1 minute
        }
    }

    pub fn from_key(key: &str) -> Self {
        if key.contains("definition") || key.contains("task_lib") {
            CacheType::Immutable
        } else if key.contains("completed") || key.contains("failed") {
            CacheType::Completed
        } else if key.contains("list") || key.contains("resources") {
            CacheType::SemiDynamic
        } else {
            CacheType::Dynamic
        }
    }
}

/// Cache statistics for monitoring
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub insertions: u64,
    pub evictions: u64,
    pub invalidations: u64,
    pub size_by_type: HashMap<String, usize>,
    pub memory_usage_bytes: usize,
    pub memory_evictions: u64,
    pub size_evictions: u64,
    pub total_access_time_ms: u64,
    pub max_access_time_ms: u64,
    pub started_at: std::time::Instant,
    pub performance_by_type: HashMap<String, CacheTypePerformance>,
}

impl Default for CacheStats {
    fn default() -> Self {
        Self::new()
    }
}

/// Performance metrics for specific cache types
#[derive(Debug, Default, Clone)]
pub struct CacheTypePerformance {
    pub hits: u64,
    pub misses: u64,
    pub avg_access_time_ms: f64,
    pub total_access_time_ms: u64,
    pub evictions: u64,
    pub memory_usage: usize,
    pub hottest_keys: Vec<String>,
    pub last_access: Option<std::time::Instant>,
}

impl CacheTypePerformance {
    pub fn hit_rate(&self) -> f64 {
        if self.hits + self.misses == 0 {
            0.0
        } else {
            self.hits as f64 / (self.hits + self.misses) as f64
        }
    }

    pub fn total_operations(&self) -> u64 {
        self.hits + self.misses
    }

    pub fn is_active(&self) -> bool {
        self.last_access
            .map(|last| last.elapsed().as_secs() < 300) // Active if accessed in last 5 minutes
            .unwrap_or(false)
    }
}

impl CacheStats {
    pub fn new() -> Self {
        Self {
            hits: 0,
            misses: 0,
            insertions: 0,
            evictions: 0,
            invalidations: 0,
            size_by_type: HashMap::new(),
            memory_usage_bytes: 0,
            memory_evictions: 0,
            size_evictions: 0,
            total_access_time_ms: 0,
            max_access_time_ms: 0,
            started_at: std::time::Instant::now(),
            performance_by_type: HashMap::new(),
        }
    }

    pub fn hit_rate(&self) -> f64 {
        if self.hits + self.misses == 0 {
            0.0
        } else {
            self.hits as f64 / (self.hits + self.misses) as f64
        }
    }

    pub fn average_access_time_ms(&self) -> f64 {
        if self.hits + self.misses == 0 {
            0.0
        } else {
            self.total_access_time_ms as f64 / (self.hits + self.misses) as f64
        }
    }

    pub fn operations_per_second(&self) -> f64 {
        let elapsed = self.started_at.elapsed().as_secs_f64();
        if elapsed == 0.0 {
            0.0
        } else {
            (self.hits + self.misses + self.insertions) as f64 / elapsed
        }
    }

    pub fn memory_efficiency(&self) -> f64 {
        if self.memory_usage_bytes == 0 {
            0.0
        } else {
            (self.hits as f64) / (self.memory_usage_bytes as f64 / 1024.0) // Hits per KB
        }
    }

    pub fn uptime_seconds(&self) -> u64 {
        self.started_at.elapsed().as_secs()
    }

    pub fn record_hit(&mut self) {
        self.hits += 1;
    }

    pub fn record_miss(&mut self) {
        self.misses += 1;
    }

    pub fn record_access_time(&mut self, duration_ms: u64) {
        self.total_access_time_ms += duration_ms;
        self.max_access_time_ms = self.max_access_time_ms.max(duration_ms);
    }

    pub fn record_insertion(&mut self, cache_type: CacheType) {
        self.insertions += 1;
        let type_name = format!("{cache_type:?}");
        *self.size_by_type.entry(type_name).or_insert(0) += 1;
    }

    pub fn record_eviction(&mut self, cache_type: CacheType) {
        self.evictions += 1;
        let type_name = format!("{cache_type:?}");
        if let Some(size) = self.size_by_type.get_mut(&type_name) {
            *size = size.saturating_sub(1);
        }
    }

    pub fn record_memory_eviction(&mut self, memory_freed: usize) {
        self.memory_evictions += 1;
        self.memory_usage_bytes = self.memory_usage_bytes.saturating_sub(memory_freed);
    }

    pub fn record_size_eviction(&mut self) {
        self.size_evictions += 1;
    }

    pub fn update_memory_usage(&mut self, delta: isize) {
        if delta >= 0 {
            self.memory_usage_bytes = self.memory_usage_bytes.saturating_add(delta as usize);
        } else {
            self.memory_usage_bytes = self.memory_usage_bytes.saturating_sub((-delta) as usize);
        }
    }

    pub fn record_invalidation(&mut self) {
        self.invalidations += 1;
    }

    pub fn update_type_performance(
        &mut self,
        cache_type: CacheType,
        hit: bool,
        access_time_ms: u64,
        key: Option<&str>,
    ) {
        let type_name = format!("{cache_type:?}");
        let perf = self
            .performance_by_type
            .entry(type_name.clone())
            .or_default();

        if hit {
            perf.hits += 1;
        } else {
            perf.misses += 1;
        }

        perf.total_access_time_ms += access_time_ms;
        perf.last_access = Some(std::time::Instant::now());

        if perf.hits + perf.misses > 0 {
            perf.avg_access_time_ms =
                perf.total_access_time_ms as f64 / (perf.hits + perf.misses) as f64;
        }

        // Track hottest keys (most accessed)
        if let Some(key) = key
            && hit
            && !perf.hottest_keys.contains(&key.to_string())
        {
            perf.hottest_keys.push(key.to_string());
            // Keep only top 10 hottest keys
            if perf.hottest_keys.len() > 10 {
                perf.hottest_keys.remove(0);
            }
        }
    }

    pub fn get_cache_health_score(&self) -> f64 {
        let hit_rate = self.hit_rate();
        let memory_efficiency = self.memory_efficiency();
        let eviction_rate = if self.insertions == 0 {
            0.0
        } else {
            (self.evictions + self.memory_evictions + self.size_evictions) as f64
                / self.insertions as f64
        };

        // Health score: weighted combination of metrics (0-100)
        let hit_score = hit_rate * 40.0; // 40% weight
        let efficiency_score = memory_efficiency.min(1.0) * 30.0; // 30% weight
        let eviction_score = (1.0 - eviction_rate.min(1.0)) * 30.0; // 30% weight (lower eviction is better)

        hit_score + efficiency_score + eviction_score
    }

    pub fn generate_report(&self) -> CachePerformanceReport {
        CachePerformanceReport {
            total_operations: self.hits + self.misses + self.insertions,
            hit_rate: self.hit_rate(),
            average_access_time_ms: self.average_access_time_ms(),
            operations_per_second: self.operations_per_second(),
            memory_usage_mb: self.memory_usage_bytes as f64 / (1024.0 * 1024.0),
            memory_efficiency: self.memory_efficiency(),
            health_score: self.get_cache_health_score(),
            uptime_seconds: self.uptime_seconds(),
            eviction_summary: EvictionSummary {
                total_evictions: self.evictions + self.memory_evictions + self.size_evictions,
                memory_evictions: self.memory_evictions,
                size_evictions: self.size_evictions,
                lru_evictions: self.evictions,
            },
            type_breakdown: self.performance_by_type.clone(),
        }
    }
}

/// Configuration for cache behavior
#[derive(Debug, Clone)]
pub struct CacheConfig {
    pub enabled: bool,
    pub max_size_per_type: usize,
    pub custom_ttl: HashMap<CacheType, Duration>,
    pub enable_stats: bool,
    pub max_memory_bytes: usize,
    pub memory_eviction_threshold: f64,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_size_per_type: 1000,
            custom_ttl: HashMap::new(),
            enable_stats: true,
            max_memory_bytes: 100 * 1024 * 1024, // 100MB default
            memory_eviction_threshold: 0.9,      // Start evicting at 90% memory usage
        }
    }
}

impl CacheConfig {
    /// Create a new cache configuration with custom TTL settings
    pub fn with_custom_ttl(mut self, cache_type: CacheType, ttl: Duration) -> Self {
        self.custom_ttl.insert(cache_type, ttl);
        self
    }

    /// Set TTL for immutable data (pipeline definitions, task libraries)
    pub fn with_immutable_ttl(mut self, ttl: Duration) -> Self {
        self.custom_ttl.insert(CacheType::Immutable, ttl);
        self
    }

    /// Set TTL for completed data (finished runs/tasks)
    pub fn with_completed_ttl(mut self, ttl: Duration) -> Self {
        self.custom_ttl.insert(CacheType::Completed, ttl);
        self
    }

    /// Set TTL for semi-dynamic data (lists, resources)
    pub fn with_semi_dynamic_ttl(mut self, ttl: Duration) -> Self {
        self.custom_ttl.insert(CacheType::SemiDynamic, ttl);
        self
    }

    /// Set TTL for dynamic data (active runs, events)
    pub fn with_dynamic_ttl(mut self, ttl: Duration) -> Self {
        self.custom_ttl.insert(CacheType::Dynamic, ttl);
        self
    }

    /// Set memory configuration
    pub fn with_memory_config(mut self, max_memory_bytes: usize, eviction_threshold: f64) -> Self {
        self.max_memory_bytes = max_memory_bytes;
        self.memory_eviction_threshold = eviction_threshold;
        self
    }

    /// Set cache size per type
    pub fn with_max_size_per_type(mut self, max_size: usize) -> Self {
        self.max_size_per_type = max_size;
        self
    }

    /// Enable or disable cache
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Enable or disable statistics tracking
    pub fn with_stats_enabled(mut self, enable_stats: bool) -> Self {
        self.enable_stats = enable_stats;
        self
    }

    /// Get the TTL for a cache type, using custom value if set, otherwise default
    pub fn get_ttl(&self, cache_type: CacheType) -> Duration {
        self.custom_ttl
            .get(&cache_type)
            .copied()
            .unwrap_or_else(|| cache_type.default_ttl())
    }

    /// Create a configuration optimized for development environment
    pub fn development() -> Self {
        Self::default()
            .with_immutable_ttl(Duration::from_secs(300)) // 5 minutes
            .with_completed_ttl(Duration::from_secs(3600)) // 1 hour
            .with_semi_dynamic_ttl(Duration::from_secs(60)) // 1 minute
            .with_dynamic_ttl(Duration::from_secs(10)) // 10 seconds
    }

    /// Create a configuration optimized for production environment
    pub fn production() -> Self {
        Self::default()
            .with_immutable_ttl(Duration::from_secs(86400)) // 24 hours
            .with_completed_ttl(Duration::from_secs(604800)) // 7 days
            .with_semi_dynamic_ttl(Duration::from_secs(1800)) // 30 minutes
            .with_dynamic_ttl(Duration::from_secs(120)) // 2 minutes
            .with_memory_config(500 * 1024 * 1024, 0.85) // 500MB, 85% threshold
    }

    /// Create a configuration optimized for testing environment
    pub fn testing() -> Self {
        Self::default()
            .with_immutable_ttl(Duration::from_millis(100))
            .with_completed_ttl(Duration::from_millis(200))
            .with_semi_dynamic_ttl(Duration::from_millis(50))
            .with_dynamic_ttl(Duration::from_millis(25))
            .with_memory_config(10 * 1024 * 1024, 0.8) // 10MB for testing
            .with_max_size_per_type(50)
    }
}

/// Generic cache store with LRU eviction and memory management
pub struct CacheStore {
    items: HashMap<String, CachedItem>,
    access_order: Vec<String>, // For LRU eviction
    max_size: usize,
    current_memory_bytes: usize,
    max_memory_bytes: usize,
    memory_eviction_threshold: f64,
}

impl CacheStore {
    pub fn new(max_size: usize) -> Self {
        Self::with_memory_limit(max_size, 100 * 1024 * 1024, 0.9) // 100MB default
    }

    pub fn with_memory_limit(
        max_size: usize,
        max_memory_bytes: usize,
        memory_eviction_threshold: f64,
    ) -> Self {
        Self {
            items: HashMap::new(),
            access_order: Vec::new(),
            max_size,
            current_memory_bytes: 0,
            max_memory_bytes,
            memory_eviction_threshold,
        }
    }

    pub fn memory_usage(&self) -> usize {
        self.current_memory_bytes
    }

    pub fn memory_usage_percent(&self) -> f64 {
        if self.max_memory_bytes == 0 {
            0.0
        } else {
            (self.current_memory_bytes as f64 / self.max_memory_bytes as f64) * 100.0
        }
    }

    pub fn should_evict_for_memory(&self) -> bool {
        let usage_ratio = self.current_memory_bytes as f64 / self.max_memory_bytes as f64;
        usage_ratio >= self.memory_eviction_threshold
    }

    pub fn get(&mut self, key: &str) -> Option<Value> {
        // Check if item exists and is expired
        let is_expired = self
            .items
            .get(key)
            .map(|item| item.is_expired())
            .unwrap_or(false);

        if is_expired {
            self.remove(key);
            return None;
        }

        if let Some(item) = self.items.get_mut(key) {
            // Update access order for LRU
            if let Some(pos) = self.access_order.iter().position(|k| k == key) {
                self.access_order.remove(pos);
            }
            self.access_order.push(key.to_string());

            Some(item.access().clone())
        } else {
            None
        }
    }

    pub fn insert(&mut self, key: String, item: CachedItem) -> Option<CachedItem> {
        let item_size = item.estimated_size_bytes;

        // Remove if exists to get accurate memory accounting
        let old_item = self.remove(&key);

        // Check if we need to evict for memory BEFORE adding the new item
        let projected_memory = self.current_memory_bytes + item_size;
        while (projected_memory > self.max_memory_bytes || self.should_evict_for_memory())
            && !self.access_order.is_empty()
        {
            let lru_key = self.access_order.remove(0);
            if let Some(evicted_item) = self.items.remove(&lru_key) {
                self.current_memory_bytes = self
                    .current_memory_bytes
                    .saturating_sub(evicted_item.estimated_size_bytes);
            }
            // Recalculate projected memory after eviction
            let new_projected = self.current_memory_bytes + item_size;
            if new_projected <= self.max_memory_bytes && !self.should_evict_for_memory() {
                break;
            }
        }

        // Check size limit and evict LRU if needed
        while self.items.len() >= self.max_size && !self.access_order.is_empty() {
            let lru_key = self.access_order.remove(0);
            if let Some(evicted_item) = self.items.remove(&lru_key) {
                self.current_memory_bytes = self
                    .current_memory_bytes
                    .saturating_sub(evicted_item.estimated_size_bytes);
            }
        }

        // Only insert if we can fit it
        if self.current_memory_bytes + item_size <= self.max_memory_bytes {
            self.access_order.push(key.clone());
            self.current_memory_bytes += item_size;
            self.items.insert(key, item);
        }

        old_item
    }

    pub fn remove(&mut self, key: &str) -> Option<CachedItem> {
        if let Some(pos) = self.access_order.iter().position(|k| k == key) {
            self.access_order.remove(pos);
        }
        if let Some(item) = self.items.remove(key) {
            self.current_memory_bytes = self
                .current_memory_bytes
                .saturating_sub(item.estimated_size_bytes);
            Some(item)
        } else {
            None
        }
    }

    pub fn clear(&mut self) {
        self.items.clear();
        self.access_order.clear();
        self.current_memory_bytes = 0;
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn cleanup_expired(&mut self) -> usize {
        let expired_keys: Vec<String> = self
            .items
            .iter()
            .filter(|(_, item)| item.is_expired())
            .map(|(key, _)| key.clone())
            .collect();

        let count = expired_keys.len();
        for key in expired_keys {
            self.remove(&key);
        }

        count
    }

    /// Force memory-based eviction to get under threshold
    pub fn evict_for_memory(&mut self) -> usize {
        let mut evicted_count = 0;

        while self.should_evict_for_memory() && !self.access_order.is_empty() {
            let lru_key = self.access_order.remove(0);
            if self.items.remove(&lru_key).is_some() {
                evicted_count += 1;
            }
        }

        evicted_count
    }

    /// Get memory usage statistics
    pub fn memory_stats(&self) -> (usize, usize, f64) {
        (
            self.current_memory_bytes,
            self.max_memory_bytes,
            self.memory_usage_percent(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_cache_item_expiration() {
        let item = CachedItem::new(json!({"test": "data"}), CacheType::Dynamic);
        assert!(!item.is_expired());

        let mut expired_item = CachedItem::new(json!({"test": "data"}), CacheType::Dynamic);
        expired_item.cached_at = Instant::now() - Duration::from_secs(120);
        assert!(expired_item.is_expired());
    }

    #[test]
    fn test_completed_expires_after_24_hours() {
        // Test that completed items don't expire within 24 hours
        let item = CachedItem::new(json!({"test": "data"}), CacheType::Completed);
        assert!(!item.is_expired());

        // Test that completed items expire after 24 hours
        let mut expired_item = CachedItem::new(json!({"test": "data"}), CacheType::Completed);
        expired_item.cached_at = Instant::now() - Duration::from_secs(86400 + 1); // 24 hours + 1 second ago
        assert!(expired_item.is_expired());
    }

    #[test]
    fn test_cache_store_lru() {
        let mut store = CacheStore::new(2);
        let initial_memory = store.memory_usage();

        store.insert(
            "key1".to_string(),
            CachedItem::new(json!(1), CacheType::Dynamic),
        );
        store.insert(
            "key2".to_string(),
            CachedItem::new(json!(2), CacheType::Dynamic),
        );
        assert_eq!(store.len(), 2);
        assert!(store.memory_usage() > initial_memory);

        // Access key1 to make it more recent
        store.get("key1");

        // Insert key3, should evict key2 (LRU)
        store.insert(
            "key3".to_string(),
            CachedItem::new(json!(3), CacheType::Dynamic),
        );
        assert_eq!(store.len(), 2);
        assert!(store.get("key1").is_some());
        assert!(store.get("key2").is_none());
        assert!(store.get("key3").is_some());

        // Memory should still be tracked correctly
        assert!(store.memory_usage() > initial_memory);
    }

    #[test]
    fn test_cache_type_from_key() {
        assert_eq!(
            CacheType::from_key("pipeline_definition:123"),
            CacheType::Immutable
        );
        assert_eq!(
            CacheType::from_key("run_details:456:completed"),
            CacheType::Completed
        );
        assert_eq!(
            CacheType::from_key("pipeline_list:all"),
            CacheType::SemiDynamic
        );
        assert_eq!(CacheType::from_key("run_events:789"), CacheType::Dynamic);
    }

    #[test]
    fn test_memory_size_estimation() {
        let string_val = json!("hello world");
        let number_val = json!(42);
        let array_val = json!([1, 2, 3]);
        let object_val = json!({"key": "value", "count": 10});

        let string_item = CachedItem::new(string_val, CacheType::Dynamic);
        let number_item = CachedItem::new(number_val, CacheType::Dynamic);
        let array_item = CachedItem::new(array_val, CacheType::Dynamic);
        let object_item = CachedItem::new(object_val, CacheType::Dynamic);

        // Basic sanity checks - strings should be larger than numbers
        assert!(string_item.estimated_size_bytes > number_item.estimated_size_bytes);
        assert!(array_item.estimated_size_bytes > number_item.estimated_size_bytes);
        assert!(object_item.estimated_size_bytes > string_item.estimated_size_bytes);

        // Verify all sizes are reasonable (not zero, not huge)
        assert!(string_item.estimated_size_bytes > 10);
        assert!(string_item.estimated_size_bytes < 1000);
    }

    #[test]
    fn test_memory_aware_cache_store() {
        let mut store = CacheStore::with_memory_limit(10, 200, 0.8); // Larger memory limit for testing

        // Create items of known size
        let item1 = CachedItem::new(json!({"data": "small"}), CacheType::Dynamic);
        let item2 = CachedItem::new(json!({"data": "also_small"}), CacheType::Dynamic);

        // Insert first item
        store.insert("key1".to_string(), item1);
        assert_eq!(store.len(), 1);
        assert!(store.memory_usage() > 0);
        assert!(store.memory_usage() <= store.max_memory_bytes);

        // Insert second item
        store.insert("key2".to_string(), item2);

        // Should be able to fit both small items
        assert!(store.len() <= 2);
        assert!(store.memory_usage() <= store.max_memory_bytes);

        // Test with a very large item that exceeds memory limit
        let huge_item = CachedItem::new(json!({"data": "x".repeat(300)}), CacheType::Dynamic);
        let huge_size = huge_item.estimated_size_bytes;

        // If the huge item is larger than max memory, it shouldn't be inserted
        if huge_size > store.max_memory_bytes {
            let _items_before = store.len();
            store.insert("huge_key".to_string(), huge_item);
            // Should either not increase or should have evicted others
            assert!(store.memory_usage() <= store.max_memory_bytes);
        }
    }

    #[test]
    fn test_memory_eviction_threshold() {
        let mut store = CacheStore::with_memory_limit(10, 1000, 0.5); // 50% threshold, larger memory pool

        // Add items gradually and verify memory stays reasonable
        let mut total_attempted_size = 0;

        for i in 0..20 {
            let test_item =
                CachedItem::new(json!({"id": i, "data": "test_data"}), CacheType::Dynamic);
            let item_size = test_item.estimated_size_bytes;
            total_attempted_size += item_size;

            store.insert(format!("key{i}"), test_item);

            // Memory should never exceed the limit
            assert!(
                store.memory_usage() <= store.max_memory_bytes,
                "Memory usage {} exceeded limit {} at iteration {}",
                store.memory_usage(),
                store.max_memory_bytes,
                i
            );
        }

        // Should have triggered some eviction if we attempted to add more than the limit
        if total_attempted_size > 1000 {
            assert!(store.memory_usage() <= 1000);
            assert!(store.len() < 20); // Should have evicted some items
        }
    }
}
