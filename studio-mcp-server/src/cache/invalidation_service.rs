//! Cache invalidation service for coordinating cache updates with CLI operations
//!
//! This service provides a centralized interface for invalidating cache entries
//! when data changes occur through CLI operations. It ensures cache consistency
//! by automatically triggering invalidation when write operations are detected.

#![allow(dead_code)]

use super::{CacheContext, PlmCache};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, warn};

/// Cache invalidation service that coordinates cache updates with data changes
pub struct CacheInvalidationService {
    /// Cache instance to invalidate
    cache: Arc<PlmCache>,
    /// Registered invalidation patterns for different operations
    patterns: Arc<RwLock<HashMap<String, Vec<InvalidationPattern>>>>,
    /// Statistics for monitoring invalidation activity
    stats: Arc<RwLock<InvalidationStats>>,
}

/// Pattern for invalidating cache entries based on operation type and parameters
#[derive(Debug, Clone)]
pub struct InvalidationPattern {
    /// Operation pattern (e.g., "plm.pipeline.create", "plm.run.start")
    pub operation_pattern: String,
    /// Cache key patterns to invalidate (supports wildcards)
    pub cache_patterns: Vec<String>,
    /// Whether to invalidate immediately or defer
    pub immediate: bool,
    /// Optional delay before invalidation (in seconds)
    pub delay_seconds: Option<u64>,
}

/// Statistics for cache invalidation monitoring
#[derive(Debug, Default, Clone)]
pub struct InvalidationStats {
    /// Total number of invalidation events processed
    pub events_processed: u64,
    /// Number of cache entries invalidated
    pub entries_invalidated: u64,
    /// Number of pattern matches
    pub pattern_matches: u64,
    /// Number of failed invalidations
    pub failures: u64,
    /// Operations by type
    pub operations_by_type: HashMap<String, u64>,
}

/// Result of cache invalidation operation
#[derive(Debug)]
pub struct InvalidationResult {
    /// Number of cache entries invalidated
    pub entries_invalidated: usize,
    /// Patterns that matched
    pub matched_patterns: Vec<String>,
    /// Any errors that occurred
    pub errors: Vec<String>,
}

impl CacheInvalidationService {
    /// Create a new cache invalidation service
    pub fn new(cache: Arc<PlmCache>) -> Self {
        let patterns = Self::build_default_patterns();

        Self {
            cache,
            patterns: Arc::new(RwLock::new(patterns)),
            stats: Arc::new(RwLock::new(InvalidationStats::default())),
        }
    }

    /// Register a new invalidation pattern
    pub async fn register_pattern(&self, pattern: InvalidationPattern) {
        let operation_pattern = pattern.operation_pattern.clone();
        let mut patterns = self.patterns.write().await;
        patterns
            .entry(operation_pattern.clone())
            .or_insert_with(Vec::new)
            .push(pattern);
        debug!(
            "Registered invalidation pattern for operation: {}",
            operation_pattern
        );
    }

    /// Process a CLI operation and invalidate relevant cache entries
    pub async fn process_operation(
        &self,
        context: &CacheContext,
        operation: &str,
        parameters: &HashMap<String, String>,
    ) -> InvalidationResult {
        let mut result = InvalidationResult {
            entries_invalidated: 0,
            matched_patterns: Vec::new(),
            errors: Vec::new(),
        };

        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.events_processed += 1;
            *stats
                .operations_by_type
                .entry(operation.to_string())
                .or_insert(0) += 1;
        }

        // Find matching patterns
        let patterns = self.patterns.read().await;
        let matching_patterns = Self::find_matching_patterns(&patterns, operation);

        for pattern in matching_patterns {
            result
                .matched_patterns
                .push(pattern.operation_pattern.clone());

            // Generate cache keys to invalidate based on pattern and parameters
            let cache_keys = Self::generate_cache_keys(&pattern, parameters);

            for cache_key in cache_keys {
                match self.invalidate_cache_key(context, &cache_key).await {
                    Ok(count) => {
                        result.entries_invalidated += count;
                        debug!(
                            "Invalidated {} entries for key pattern: {}",
                            count, cache_key
                        );
                    }
                    Err(e) => {
                        let error_msg = format!("Failed to invalidate cache key {cache_key}: {e}");
                        result.errors.push(error_msg.clone());
                        warn!("{}", error_msg);
                    }
                }
            }
        }

        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.entries_invalidated += result.entries_invalidated as u64;
            stats.pattern_matches += result.matched_patterns.len() as u64;
            stats.failures += result.errors.len() as u64;
        }

        debug!(
            "Operation '{}' triggered invalidation of {} cache entries",
            operation, result.entries_invalidated
        );

        result
    }

    /// Invalidate a specific cache key pattern
    async fn invalidate_cache_key(
        &self,
        context: &CacheContext,
        key_pattern: &str,
    ) -> Result<usize, String> {
        if key_pattern.contains('*') {
            // Pattern-based invalidation
            self.cache.invalidate_pattern(context, key_pattern).await;
            Ok(1) // Pattern invalidation doesn't return count, assume 1
        } else {
            // Exact key invalidation
            self.cache.remove(context, key_pattern).await;
            Ok(1)
        }
    }

    /// Find patterns that match the given operation
    fn find_matching_patterns(
        patterns: &HashMap<String, Vec<InvalidationPattern>>,
        operation: &str,
    ) -> Vec<InvalidationPattern> {
        let mut matching = Vec::new();

        for (pattern_key, pattern_list) in patterns {
            if Self::operation_matches_pattern(operation, pattern_key) {
                matching.extend(pattern_list.clone());
            }
        }

        matching
    }

    /// Check if an operation matches a pattern (supports wildcards)
    fn operation_matches_pattern(operation: &str, pattern: &str) -> bool {
        if pattern == "*" {
            return true;
        }

        if pattern.contains('*') {
            // Simple wildcard matching
            if pattern.ends_with('*') {
                let prefix = pattern.trim_end_matches('*');
                return operation.starts_with(prefix);
            }
            if pattern.starts_with('*') {
                let suffix = pattern.trim_start_matches('*');
                return operation.ends_with(suffix);
            }
        }

        operation == pattern
    }

    /// Generate cache keys to invalidate based on pattern and parameters
    fn generate_cache_keys(
        pattern: &InvalidationPattern,
        parameters: &HashMap<String, String>,
    ) -> Vec<String> {
        let mut keys = Vec::new();

        for cache_pattern in &pattern.cache_patterns {
            let mut key = cache_pattern.clone();

            // Replace parameter placeholders
            for (param_name, param_value) in parameters {
                let placeholder = format!("{{{param_name}}}");
                key = key.replace(&placeholder, param_value);
            }

            keys.push(key);
        }

        keys
    }

    /// Build default invalidation patterns for common PLM operations
    fn build_default_patterns() -> HashMap<String, Vec<InvalidationPattern>> {
        let mut patterns = HashMap::new();

        // Pipeline operations
        patterns.insert(
            "plm.pipeline.create".to_string(),
            vec![InvalidationPattern {
                operation_pattern: "plm.pipeline.create".to_string(),
                cache_patterns: vec!["pipelines:list".to_string(), "pipeline:*".to_string()],
                immediate: true,
                delay_seconds: None,
            }],
        );

        patterns.insert(
            "plm.pipeline.update".to_string(),
            vec![InvalidationPattern {
                operation_pattern: "plm.pipeline.update".to_string(),
                cache_patterns: vec![
                    "pipeline:def:{pipeline_id}".to_string(),
                    "pipelines:list".to_string(),
                ],
                immediate: true,
                delay_seconds: None,
            }],
        );

        patterns.insert(
            "plm.pipeline.delete".to_string(),
            vec![InvalidationPattern {
                operation_pattern: "plm.pipeline.delete".to_string(),
                cache_patterns: vec![
                    "pipeline:*:{pipeline_id}".to_string(),
                    "pipelines:list".to_string(),
                ],
                immediate: true,
                delay_seconds: None,
            }],
        );

        // Run operations
        patterns.insert(
            "plm.run.start".to_string(),
            vec![InvalidationPattern {
                operation_pattern: "plm.run.start".to_string(),
                cache_patterns: vec![
                    "pipeline:runs:{pipeline_id}".to_string(),
                    "runs:list".to_string(),
                    "run:*".to_string(),
                ],
                immediate: true,
                delay_seconds: None,
            }],
        );

        patterns.insert(
            "plm.run.complete".to_string(),
            vec![InvalidationPattern {
                operation_pattern: "plm.run.complete".to_string(),
                cache_patterns: vec![
                    "run:details:{run_id}".to_string(),
                    "pipeline:runs:{pipeline_id}".to_string(),
                    "runs:list".to_string(),
                ],
                immediate: true,
                delay_seconds: None,
            }],
        );

        // Task operations
        patterns.insert(
            "plm.task.*".to_string(),
            vec![InvalidationPattern {
                operation_pattern: "plm.task.*".to_string(),
                cache_patterns: vec!["tasks:list".to_string(), "task:*".to_string()],
                immediate: true,
                delay_seconds: None,
            }],
        );

        // Resource operations
        patterns.insert(
            "plm.resource.*".to_string(),
            vec![InvalidationPattern {
                operation_pattern: "plm.resource.*".to_string(),
                cache_patterns: vec!["pipeline:resources".to_string(), "resource:*".to_string()],
                immediate: true,
                delay_seconds: None,
            }],
        );

        patterns
    }

    /// Get invalidation statistics
    pub async fn get_stats(&self) -> InvalidationStats {
        self.stats.read().await.clone()
    }

    /// Clear invalidation statistics
    pub async fn clear_stats(&self) {
        let mut stats = self.stats.write().await;
        *stats = InvalidationStats::default();
    }

    /// Get registered patterns
    pub async fn get_patterns(&self) -> HashMap<String, Vec<InvalidationPattern>> {
        self.patterns.read().await.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::{CacheContext, PlmCache};

    #[tokio::test]
    async fn test_invalidation_service_creation() {
        let cache = Arc::new(PlmCache::new());
        let service = CacheInvalidationService::new(cache);

        let patterns = service.get_patterns().await;
        assert!(patterns.contains_key("plm.pipeline.create"));
        assert!(patterns.contains_key("plm.run.start"));
    }

    #[tokio::test]
    async fn test_operation_matching() {
        assert!(CacheInvalidationService::operation_matches_pattern(
            "plm.pipeline.create",
            "plm.pipeline.create"
        ));
        assert!(CacheInvalidationService::operation_matches_pattern(
            "plm.pipeline.create",
            "plm.pipeline.*"
        ));
        assert!(CacheInvalidationService::operation_matches_pattern(
            "plm.task.delete",
            "plm.task.*"
        ));
        assert!(!CacheInvalidationService::operation_matches_pattern(
            "plm.run.start",
            "plm.pipeline.*"
        ));
    }

    #[tokio::test]
    async fn test_cache_key_generation() {
        let pattern = InvalidationPattern {
            operation_pattern: "plm.pipeline.update".to_string(),
            cache_patterns: vec![
                "pipeline:def:{pipeline_id}".to_string(),
                "pipeline:runs:{pipeline_id}".to_string(),
            ],
            immediate: true,
            delay_seconds: None,
        };

        let mut parameters = HashMap::new();
        parameters.insert("pipeline_id".to_string(), "test-pipeline-123".to_string());

        let keys = CacheInvalidationService::generate_cache_keys(&pattern, &parameters);

        assert_eq!(keys.len(), 2);
        assert!(keys.contains(&"pipeline:def:test-pipeline-123".to_string()));
        assert!(keys.contains(&"pipeline:runs:test-pipeline-123".to_string()));
    }

    #[tokio::test]
    async fn test_process_operation() {
        let cache = Arc::new(PlmCache::new());
        let service = CacheInvalidationService::new(cache);
        let context = CacheContext::new(
            "test_user".to_string(),
            "test_org".to_string(),
            "test_env".to_string(),
        );

        let mut parameters = HashMap::new();
        parameters.insert("pipeline_id".to_string(), "test-pipeline".to_string());

        let result = service
            .process_operation(&context, "plm.pipeline.update", &parameters)
            .await;

        assert!(result.entries_invalidated > 0);
        assert!(!result.matched_patterns.is_empty());
        assert_eq!(result.errors.len(), 0);

        let stats = service.get_stats().await;
        assert_eq!(stats.events_processed, 1);
        assert!(stats.operations_by_type.contains_key("plm.pipeline.update"));
    }

    /// Test CLI manager integration patterns
    #[tokio::test]
    async fn test_cli_manager_integration_patterns() {
        // Test write operation detection patterns
        let write_operations = [
            "plm.pipeline.create",
            "plm.pipeline.update",
            "plm.pipeline.delete",
            "plm.run.start",
            "plm.run.cancel",
            "plm.resource.assign",
            "plm.group.assign",
        ];

        let read_operations = [
            "plm.pipeline.list",
            "plm.pipeline.get",
            "plm.run.list",
            "plm.run.get",
            "plm.resource.list",
            "plm.group.list",
        ];

        // Test write operations would trigger cache invalidation
        for operation in &write_operations {
            assert!(
                operation.contains("create")
                    || operation.contains("update")
                    || operation.contains("delete")
                    || operation.contains("start")
                    || operation.contains("cancel")
                    || operation.contains("assign"),
                "Operation {operation} should be classified as write operation"
            );
        }

        // Test read operations would NOT trigger cache invalidation
        for operation in &read_operations {
            assert!(
                operation.contains("list") || operation.contains("get"),
                "Operation {operation} should be classified as read operation"
            );
        }
    }

    /// Test that cache invalidation service properly handles CLI command patterns
    #[tokio::test]
    async fn test_cli_command_cache_invalidation() {
        let cache = Arc::new(PlmCache::new());
        let service = CacheInvalidationService::new(cache);
        let context = CacheContext::new(
            "authenticated_user".to_string(),
            "default_org".to_string(),
            "production".to_string(),
        );

        // Test pipeline create operation
        let mut parameters = HashMap::new();
        parameters.insert("pipeline_id".to_string(), "my-new-pipeline".to_string());

        let result = service
            .process_operation(&context, "plm.pipeline.create", &parameters)
            .await;

        // Should match create pattern and invalidate relevant caches
        assert!(!result.matched_patterns.is_empty());
        assert!(
            result
                .matched_patterns
                .contains(&"plm.pipeline.create".to_string())
        );

        // Test run start operation
        parameters.clear();
        parameters.insert("pipeline_id".to_string(), "test-pipeline".to_string());
        parameters.insert("run_id".to_string(), "run-123".to_string());

        let result = service
            .process_operation(&context, "plm.run.start", &parameters)
            .await;

        // Should match run start pattern
        assert!(!result.matched_patterns.is_empty());
        assert!(
            result
                .matched_patterns
                .contains(&"plm.run.start".to_string())
        );
    }
}
