//! Intelligent caching layer for PLM resources
//!
//! Implements state-aware caching with different TTL policies based on data mutability:
//! - Immutable: Pipeline definitions, task libraries (1+ hours)
//! - Completed: Finished runs/tasks (permanent until manual invalidation)
//! - Semi-dynamic: Lists, resources (5-15 minutes)
//! - Dynamic: Active runs, live events (30 seconds - 2 minutes)

use serde_json::Value;
use std::collections::HashMap;
use std::time::{Duration, Instant};

pub mod plm_cache;
pub use plm_cache::PlmCache;

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
}

impl CachedItem {
    pub fn new(data: Value, cache_type: CacheType) -> Self {
        let ttl = cache_type.default_ttl();
        let now = Instant::now();

        Self {
            data,
            cached_at: now,
            ttl,
            cache_type,
            access_count: 0,
            last_accessed: now,
        }
    }

    pub fn is_expired(&self) -> bool {
        match self.cache_type {
            CacheType::Completed => false, // Never expires until manually invalidated
            _ => self.cached_at.elapsed() > self.ttl,
        }
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
    /// Finished runs/tasks - never change once completed
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
            CacheType::Completed => Duration::from_secs(86400), // 24 hours (but never expires)
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
#[derive(Debug, Default, Clone)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub insertions: u64,
    pub evictions: u64,
    pub invalidations: u64,
    pub size_by_type: HashMap<String, usize>,
}

impl CacheStats {
    pub fn hit_rate(&self) -> f64 {
        if self.hits + self.misses == 0 {
            0.0
        } else {
            self.hits as f64 / (self.hits + self.misses) as f64
        }
    }

    pub fn record_hit(&mut self) {
        self.hits += 1;
    }

    pub fn record_miss(&mut self) {
        self.misses += 1;
    }

    pub fn record_insertion(&mut self, cache_type: CacheType) {
        self.insertions += 1;
        let type_name = format!("{:?}", cache_type);
        *self.size_by_type.entry(type_name).or_insert(0) += 1;
    }

    pub fn record_eviction(&mut self, cache_type: CacheType) {
        self.evictions += 1;
        let type_name = format!("{:?}", cache_type);
        if let Some(size) = self.size_by_type.get_mut(&type_name) {
            *size = size.saturating_sub(1);
        }
    }

    pub fn record_invalidation(&mut self) {
        self.invalidations += 1;
    }
}

/// Configuration for cache behavior
#[derive(Debug, Clone)]
pub struct CacheConfig {
    pub enabled: bool,
    pub max_size_per_type: usize,
    pub custom_ttl: HashMap<CacheType, Duration>,
    pub enable_stats: bool,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_size_per_type: 1000,
            custom_ttl: HashMap::new(),
            enable_stats: true,
        }
    }
}

/// Generic cache store with LRU eviction
pub struct CacheStore {
    items: HashMap<String, CachedItem>,
    access_order: Vec<String>, // For LRU eviction
    max_size: usize,
}

impl CacheStore {
    pub fn new(max_size: usize) -> Self {
        Self {
            items: HashMap::new(),
            access_order: Vec::new(),
            max_size,
        }
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
        // Remove if exists
        if let Some(old_item) = self.remove(&key) {
            // Reinsert at end
            self.access_order.push(key.clone());
            self.items.insert(key, item);
            return Some(old_item);
        }

        // Check size limit and evict LRU if needed
        while self.items.len() >= self.max_size && !self.access_order.is_empty() {
            let lru_key = self.access_order.remove(0);
            self.items.remove(&lru_key);
        }

        self.access_order.push(key.clone());
        self.items.insert(key, item);
        None
    }

    pub fn remove(&mut self, key: &str) -> Option<CachedItem> {
        if let Some(pos) = self.access_order.iter().position(|k| k == key) {
            self.access_order.remove(pos);
        }
        self.items.remove(key)
    }

    pub fn clear(&mut self) {
        self.items.clear();
        self.access_order.clear();
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
    fn test_completed_never_expires() {
        let mut item = CachedItem::new(json!({"test": "data"}), CacheType::Completed);
        item.cached_at = Instant::now() - Duration::from_secs(86400 * 365); // 1 year ago
        assert!(!item.is_expired());
    }

    #[test]
    fn test_cache_store_lru() {
        let mut store = CacheStore::new(2);

        store.insert(
            "key1".to_string(),
            CachedItem::new(json!(1), CacheType::Dynamic),
        );
        store.insert(
            "key2".to_string(),
            CachedItem::new(json!(2), CacheType::Dynamic),
        );
        assert_eq!(store.len(), 2);

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
}
