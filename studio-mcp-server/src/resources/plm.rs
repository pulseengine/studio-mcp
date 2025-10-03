//! PLM (Pipeline Management) resource provider

#![allow(dead_code)]

use crate::auth_middleware::AuthMiddleware;
use crate::cache::{CacheContext, CacheInvalidationService, PlmCache};
use pulseengine_mcp_protocol::{Content, Resource};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use studio_cli_manager::{CliManager, OperationHook};
use studio_mcp_shared::{ResourceUri, Result, StudioConfig, StudioError};
use tracing::{debug, warn};

pub struct PlmResourceProvider {
    cli_manager: Arc<CliManager>,
    #[allow(dead_code)]
    config: StudioConfig,
    cache: Arc<PlmCache>,
    auth_middleware: Option<Arc<AuthMiddleware>>,
    invalidation_service: Option<Arc<CacheInvalidationService>>,
}

impl PlmResourceProvider {
    pub fn new(cli_manager: Arc<CliManager>, config: StudioConfig) -> Self {
        Self {
            cli_manager,
            config,
            cache: Arc::new(PlmCache::new()),
            auth_middleware: None,
            invalidation_service: None,
        }
    }

    pub fn with_cache(
        cli_manager: Arc<CliManager>,
        config: StudioConfig,
        cache: Arc<PlmCache>,
    ) -> Self {
        Self {
            cli_manager,
            config,
            cache,
            auth_middleware: None,
            invalidation_service: None,
        }
    }

    /// Set the authentication middleware for user context extraction
    pub fn with_auth(mut self, auth_middleware: Arc<AuthMiddleware>) -> Self {
        self.auth_middleware = Some(auth_middleware);
        self
    }

    /// Enable cache invalidation with automatic CLI operation detection
    pub async fn with_cache_invalidation(mut self) -> Self {
        let invalidation_service = Arc::new(CacheInvalidationService::new(self.cache.clone()));

        // Create a hook that will trigger cache invalidation
        let hook_service = invalidation_service.clone();
        let hook: OperationHook = Arc::new(
            move |operation: &str, args: &[&str], _result: &serde_json::Value| {
                let service = hook_service.clone();
                let operation = operation.to_string();
                let args_vec: Vec<String> = args.iter().map(|s| s.to_string()).collect();

                // Spawn async task to handle invalidation
                tokio::spawn(async move {
                    let context = CacheContext::new(
                        "authenticated_user".to_string(),
                        "default_org".to_string(),
                        "production".to_string(),
                    );

                    // Extract parameters from CLI arguments
                    let mut parameters = HashMap::new();

                    // Extract pipeline ID from various argument patterns
                    if let Some(pipeline_idx) = args_vec
                        .iter()
                        .position(|arg| arg == "--pipeline" || arg == "-p")
                        && let Some(pipeline_id) = args_vec.get(pipeline_idx + 1)
                    {
                        parameters.insert("pipeline_id".to_string(), pipeline_id.clone());
                    }

                    // Extract run ID
                    if let Some(run_idx) = args_vec
                        .iter()
                        .position(|arg| arg == "--run" || arg == "-r")
                        && let Some(run_id) = args_vec.get(run_idx + 1)
                    {
                        parameters.insert("run_id".to_string(), run_id.clone());
                    }

                    // Extract entity name from positional arguments (e.g., "plm pipeline create my-pipeline")
                    if args_vec.len() >= 4
                        && args_vec[0] == "plm"
                        && (args_vec[1] == "pipeline"
                            || (args_vec[1] == "run" && args_vec[2] == "start"))
                    {
                        parameters.insert("pipeline_id".to_string(), args_vec[3].clone());
                    }

                    let result = service
                        .process_operation(&context, &operation, &parameters)
                        .await;
                    debug!(
                        "Cache invalidation for '{}': {} entries invalidated, {} patterns matched",
                        operation,
                        result.entries_invalidated,
                        result.matched_patterns.len()
                    );

                    if !result.errors.is_empty() {
                        warn!(
                            "Cache invalidation errors for '{}': {:?}",
                            operation, result.errors
                        );
                    }
                });
            },
        );

        // Register the hook with the CLI manager
        let cli_manager = self.cli_manager.clone();
        tokio::spawn(async move {
            cli_manager.register_operation_hook(hook).await;
        });

        self.invalidation_service = Some(invalidation_service);
        self
    }

    /// Get access to the PLM cache for external invalidation and monitoring
    pub fn cache(&self) -> Arc<PlmCache> {
        self.cache.clone()
    }

    /// Invalidate cache when pipeline state changes (e.g., after run starts/completes)
    pub async fn invalidate_pipeline_cache(&self, pipeline_id: &str) {
        let context = self.get_cache_context();
        self.cache.invalidate_pipeline(&context, pipeline_id).await;
    }

    /// Invalidate cache when run state changes
    pub async fn invalidate_run_cache(&self, run_id: &str) {
        let context = self.get_cache_context();
        self.cache.invalidate_run(&context, run_id).await;
    }

    /// Clean up expired cache entries
    pub async fn cleanup_cache(&self) -> usize {
        self.cache.cleanup_expired().await
    }

    /// Get cache context from authentication middleware or fallback to default
    /// TODO: This should be async and integrate with actual authentication
    fn get_cache_context(&self) -> CacheContext {
        // For now, use a default context with secure defaults
        // Future enhancement: extract from auth_middleware when available
        CacheContext::new(
            "authenticated_user".to_string(),
            "default_org".to_string(),
            "production".to_string(),
        )
    }

    pub async fn list_resources(&self) -> Result<Vec<Resource>> {
        let mut resources = Vec::new();

        // PLM base resources - comprehensive hierarchy
        resources.extend(vec![
            Resource {
                uri: "studio://plm/pipelines/".to_string(),
                name: "Pipelines".to_string(),
                description: Some("List all available pipelines".to_string()),
                mime_type: Some("application/json".to_string()),
                annotations: None,
                raw: None,
            },
            Resource {
                uri: "studio://plm/runs/".to_string(),
                name: "Pipeline Runs".to_string(),
                description: Some("All pipeline execution runs".to_string()),
                mime_type: Some("application/json".to_string()),
                annotations: None,
                raw: None,
            },
            Resource {
                uri: "studio://plm/tasks/".to_string(),
                name: "Tasks".to_string(),
                description: Some("Pipeline tasks and task libraries".to_string()),
                mime_type: Some("application/json".to_string()),
                annotations: None,
                raw: None,
            },
            Resource {
                uri: "studio://plm/resources/".to_string(),
                name: "Pipeline Resources".to_string(),
                description: Some("Available pipeline resources".to_string()),
                mime_type: Some("application/json".to_string()),
                annotations: None,
                raw: None,
            },
            Resource {
                uri: "studio://plm/groups/".to_string(),
                name: "User Groups".to_string(),
                description: Some("Pipeline user groups and access control".to_string()),
                mime_type: Some("application/json".to_string()),
                annotations: None,
                raw: None,
            },
            Resource {
                uri: "studio://plm/secrets/".to_string(),
                name: "Pipeline Secrets".to_string(),
                description: Some("Pipeline secret management".to_string()),
                mime_type: Some("application/json".to_string()),
                annotations: None,
                raw: None,
            },
            Resource {
                uri: "studio://plm/triggers/".to_string(),
                name: "Pipeline Triggers".to_string(),
                description: Some("Pipeline trigger configurations".to_string()),
                mime_type: Some("application/json".to_string()),
                annotations: None,
                raw: None,
            },
            Resource {
                uri: "studio://plm/access-config/".to_string(),
                name: "Access Configurations".to_string(),
                description: Some("Pipeline access control configurations".to_string()),
                mime_type: Some("application/json".to_string()),
                annotations: None,
                raw: None,
            },
        ]);

        // Try to fetch dynamic pipeline resources
        match self.get_pipeline_list().await {
            Ok(pipelines) => {
                for pipeline in pipelines {
                    if let Some(pipeline_id) = pipeline.get("id").and_then(|v| v.as_str()) {
                        let pipeline_name = pipeline
                            .get("name")
                            .and_then(|v| v.as_str())
                            .unwrap_or("Unknown Pipeline");

                        // Pipeline definition resource
                        resources.push(Resource {
                            uri: format!("studio://plm/pipelines/{pipeline_id}"),
                            name: format!("Pipeline: {pipeline_name}"),
                            description: Some(format!(
                                "Pipeline definition (YAML/JSON) for {pipeline_name}"
                            )),
                            mime_type: Some("application/yaml".to_string()),
                            annotations: None,
                            raw: None,
                        });

                        // Pipeline runs resource
                        resources.push(Resource {
                            uri: format!("studio://plm/pipelines/{pipeline_id}/runs"),
                            name: format!("Runs: {pipeline_name}"),
                            description: Some(format!(
                                "Execution runs for pipeline {pipeline_name}"
                            )),
                            mime_type: Some("application/json".to_string()),
                            annotations: None,
                            raw: None,
                        });

                        // Pipeline events resource
                        resources.push(Resource {
                            uri: format!("studio://plm/pipelines/{pipeline_id}/events"),
                            name: format!("Events: {pipeline_name}"),
                            description: Some(format!(
                                "Recent events for pipeline {pipeline_name}"
                            )),
                            mime_type: Some("application/json".to_string()),
                            annotations: None,
                            raw: None,
                        });
                    }
                }
            }
            Err(e) => {
                warn!(
                    "Failed to fetch pipeline list for resource discovery: {}",
                    e
                );
                // Continue with static resources only
            }
        }

        debug!("PLM provider listed {} resources", resources.len());
        Ok(resources)
    }

    pub async fn read_resource(&self, uri: &ResourceUri) -> Result<Vec<Content>> {
        debug!("PLM provider reading resource: {}", uri.to_string());

        match uri.path.get(1).map(|s| s.as_str()) {
            Some("pipelines") => self.read_pipeline_resource(uri).await,
            Some("runs") => self.read_runs_resource(uri).await,
            Some("tasks") => self.read_tasks_resource(uri).await,
            Some("resources") => self.read_pipeline_resources_resource(uri).await,
            Some("groups") => self.read_groups_resource(uri).await,
            Some("secrets") => self.read_secrets_resource(uri).await,
            Some("triggers") => self.read_triggers_resource(uri).await,
            Some("access-config") => self.read_access_config_resource(uri).await,
            None => {
                // PLM root resource
                self.read_plm_root().await
            }
            Some(resource_type) => Err(StudioError::ResourceNotFound(format!(
                "PLM resource type '{resource_type}' not found"
            ))),
        }
    }

    async fn read_plm_root(&self) -> Result<Vec<Content>> {
        let content = serde_json::json!({
            "name": "Pipeline Management (PLM)",
            "description": "WindRiver Studio Pipeline Management system",
            "version": "1.0",
            "capabilities": [
                "pipeline_management",
                "pipeline_execution",
                "task_management",
                "resource_allocation",
                "access_control",
                "secret_management",
                "trigger_management"
            ],
            "endpoints": {
                "pipelines": "studio://plm/pipelines/",
                "runs": "studio://plm/runs/",
                "tasks": "studio://plm/tasks/",
                "resources": "studio://plm/resources/",
                "groups": "studio://plm/groups/",
                "secrets": "studio://plm/secrets/",
                "triggers": "studio://plm/triggers/",
                "access_config": "studio://plm/access-config/"
            },
            "cli_commands": {
                "pipeline": ["create", "delete", "get", "list", "lock", "unlock", "update", "prettify", "weave"],
                "run": ["start", "cancel", "get", "list", "events", "log"],
                "task": ["manage task definitions and libraries"],
                "resource": ["list", "assign", "revoke"],
                "group": ["assign", "join", "leave", "revoke"],
                "secret": ["manage pipeline secrets"],
                "trigger": ["manage pipeline triggers"],
                "access_config": ["manage pipeline access control"]
            }
        });

        Ok(vec![Content::Text {
            text: content.to_string(),
        }])
    }

    async fn read_pipeline_resource(&self, uri: &ResourceUri) -> Result<Vec<Content>> {
        match uri.path.get(2) {
            None => {
                // List all pipelines
                let pipelines = self.get_pipeline_list().await?;
                let content = serde_json::json!({
                    "pipelines": pipelines,
                    "total": pipelines.len(),
                    "description": "All available pipeline definitions"
                });

                Ok(vec![Content::Text {
                    text: content.to_string(),
                }])
            }
            Some(pipeline_id) => {
                match uri.path.get(3).map(|s| s.as_str()) {
                    Some("runs") => {
                        // Pipeline runs
                        let runs = self.get_pipeline_runs(pipeline_id).await?;
                        let content = serde_json::json!({
                            "pipeline_id": pipeline_id,
                            "runs": runs,
                            "total": runs.as_array().map(|arr| arr.len()).unwrap_or(0)
                        });

                        Ok(vec![Content::Text {
                            text: content.to_string(),
                        }])
                    }
                    Some("events") => {
                        // Pipeline events (recent activity)
                        let events = self.get_pipeline_events(pipeline_id).await?;
                        let content = serde_json::json!({
                            "pipeline_id": pipeline_id,
                            "events": events,
                            "description": "Recent pipeline events and activity"
                        });

                        Ok(vec![Content::Text {
                            text: content.to_string(),
                        }])
                    }
                    None => {
                        // Individual pipeline definition (YAML/JSON)
                        let pipeline_def = self.get_pipeline_definition(pipeline_id).await?;
                        Ok(vec![Content::Text {
                            text: pipeline_def.to_string(),
                        }])
                    }
                    Some(run_id) => {
                        // Specific run details
                        let run_details = self.get_run_details(pipeline_id, run_id).await?;
                        Ok(vec![Content::Text {
                            text: run_details.to_string(),
                        }])
                    }
                }
            }
        }
    }

    async fn read_runs_resource(&self, uri: &ResourceUri) -> Result<Vec<Content>> {
        match uri.path.get(2) {
            None => {
                // List all recent runs across all pipelines
                let runs = self.get_all_runs().await?;
                let content = serde_json::json!({
                    "runs": runs,
                    "total": runs.as_array().map(|arr| arr.len()).unwrap_or(0),
                    "description": "All pipeline execution runs"
                });

                Ok(vec![Content::Text {
                    text: content.to_string(),
                }])
            }
            Some(run_id) => {
                // Specific run details
                let run_details = self.get_run_by_id(run_id).await?;
                Ok(vec![Content::Text {
                    text: run_details.to_string(),
                }])
            }
        }
    }

    async fn read_tasks_resource(&self, uri: &ResourceUri) -> Result<Vec<Content>> {
        match uri.path.get(2) {
            None => {
                // List all available tasks
                let tasks = self.get_all_tasks().await?;
                let content = serde_json::json!({
                    "tasks": tasks,
                    "description": "All available pipeline tasks and task libraries"
                });

                Ok(vec![Content::Text {
                    text: content.to_string(),
                }])
            }
            Some(task_id) => {
                // Specific task details
                let task_details = self.get_task_details(task_id).await?;
                Ok(vec![Content::Text {
                    text: task_details.to_string(),
                }])
            }
        }
    }

    async fn read_pipeline_resources_resource(&self, _uri: &ResourceUri) -> Result<Vec<Content>> {
        let resources = self.get_pipeline_resources().await?;
        let content = serde_json::json!({
            "resources": resources,
            "description": "Available pipeline resources for assignment"
        });

        Ok(vec![Content::Text {
            text: content.to_string(),
        }])
    }

    async fn read_groups_resource(&self, _uri: &ResourceUri) -> Result<Vec<Content>> {
        let groups = self.get_pipeline_groups().await?;
        let content = serde_json::json!({
            "groups": groups,
            "description": "User groups with pipeline access"
        });

        Ok(vec![Content::Text {
            text: content.to_string(),
        }])
    }

    async fn read_secrets_resource(&self, _uri: &ResourceUri) -> Result<Vec<Content>> {
        let secrets = self.get_pipeline_secrets().await?;
        let content = serde_json::json!({
            "secrets": secrets,
            "description": "Pipeline secrets (values hidden for security)"
        });

        Ok(vec![Content::Text {
            text: content.to_string(),
        }])
    }

    async fn read_triggers_resource(&self, _uri: &ResourceUri) -> Result<Vec<Content>> {
        let triggers = self.get_pipeline_triggers().await?;
        let content = serde_json::json!({
            "triggers": triggers,
            "description": "Pipeline trigger configurations"
        });

        Ok(vec![Content::Text {
            text: content.to_string(),
        }])
    }

    async fn read_access_config_resource(&self, _uri: &ResourceUri) -> Result<Vec<Content>> {
        let access_configs = self.get_access_configs().await?;
        let content = serde_json::json!({
            "access_configs": access_configs,
            "description": "Pipeline access control configurations"
        });

        Ok(vec![Content::Text {
            text: content.to_string(),
        }])
    }

    // CLI interaction methods
    async fn get_pipeline_list(&self) -> Result<Vec<Value>> {
        let context = self.get_cache_context();
        let cache_key = PlmCache::pipeline_list_key();

        // Try cache first
        if let Some(cached_value) = self.cache.get(&context, &cache_key).await
            && let Some(pipelines) = cached_value.get("pipelines").and_then(|v| v.as_array())
        {
            debug!(
                "Returning cached pipeline list ({} pipelines)",
                pipelines.len()
            );
            return Ok(pipelines.clone());
        }

        // Cache miss - fetch from CLI
        match self
            .cli_manager
            .execute(&["plm", "pipeline", "list", "--output", "json"], None)
            .await
        {
            Ok(result) => {
                let pipelines = if let Some(pipelines) = result.as_array() {
                    pipelines.clone()
                } else if let Some(obj) = result.as_object() {
                    if let Some(pipelines) = obj.get("pipelines").and_then(|v| v.as_array()) {
                        pipelines.clone()
                    } else {
                        vec![result]
                    }
                } else {
                    vec![]
                };

                // Cache the result
                let cache_data = serde_json::json!({
                    "pipelines": pipelines,
                    "total": pipelines.len(),
                    "cached_at": chrono::Utc::now().to_rfc3339()
                });
                self.cache.insert(&context, cache_key, cache_data).await;

                debug!("Fetched and cached {} pipelines", pipelines.len());
                Ok(pipelines)
            }
            Err(e) => {
                warn!("Failed to list pipelines: {}", e);
                Ok(vec![])
            }
        }
    }

    async fn get_pipeline_definition(&self, pipeline_id: &str) -> Result<Value> {
        let context = self.get_cache_context();
        let cache_key = PlmCache::pipeline_definition_key(pipeline_id);

        // Try cache first (pipeline definitions are immutable)
        if let Some(cached_value) = self.cache.get(&context, &cache_key).await {
            debug!("Returning cached pipeline definition for: {}", pipeline_id);
            return Ok(cached_value);
        }

        // Cache miss - fetch from CLI
        match self
            .cli_manager
            .execute(
                &["plm", "pipeline", "get", pipeline_id, "--output", "yaml"],
                None,
            )
            .await
        {
            Ok(result) => {
                // Cache the result (immutable data)
                self.cache.insert(&context, cache_key, result.clone()).await;
                debug!(
                    "Fetched and cached pipeline definition for: {}",
                    pipeline_id
                );
                Ok(result)
            }
            Err(e) => Err(e),
        }
    }

    async fn get_pipeline_runs(&self, pipeline_id: &str) -> Result<Value> {
        let context = self.get_cache_context();
        let cache_key = PlmCache::pipeline_runs_key(pipeline_id);

        // Try cache first (semi-dynamic data)
        if let Some(cached_value) = self.cache.get(&context, &cache_key).await {
            debug!("Returning cached pipeline runs for: {}", pipeline_id);
            return Ok(cached_value);
        }

        // Cache miss - fetch from CLI
        match self
            .cli_manager
            .execute(
                &[
                    "plm",
                    "run",
                    "list",
                    "--pipeline",
                    pipeline_id,
                    "--output",
                    "json",
                ],
                None,
            )
            .await
        {
            Ok(result) => {
                // Cache the result (semi-dynamic data)
                self.cache.insert(&context, cache_key, result.clone()).await;
                debug!("Fetched and cached pipeline runs for: {}", pipeline_id);
                Ok(result)
            }
            Err(e) => Err(e),
        }
    }

    async fn get_pipeline_events(&self, pipeline_id: &str) -> Result<Value> {
        let context = self.get_cache_context();
        let cache_key = PlmCache::pipeline_events_key(pipeline_id);

        // Try cache first (dynamic data - short TTL)
        if let Some(cached_value) = self.cache.get(&context, &cache_key).await {
            debug!("Returning cached pipeline events for: {}", pipeline_id);
            return Ok(cached_value);
        }

        // Cache miss - fetch from CLI
        match self
            .cli_manager
            .execute(
                &[
                    "plm",
                    "run",
                    "events",
                    "--pipeline",
                    pipeline_id,
                    "--output",
                    "json",
                ],
                None,
            )
            .await
        {
            Ok(result) => {
                // Cache the result (dynamic data)
                self.cache.insert(&context, cache_key, result.clone()).await;
                debug!("Fetched and cached pipeline events for: {}", pipeline_id);
                Ok(result)
            }
            Err(e) => Err(e),
        }
    }

    async fn get_run_details(&self, _pipeline_id: &str, run_id: &str) -> Result<Value> {
        let context = self.get_cache_context();
        let cache_key = PlmCache::run_details_key(run_id);

        // Try cache first - check if run is completed for better caching
        if let Some(cached_value) = self.cache.get(&context, &cache_key).await {
            debug!("Returning cached run details for: {}", run_id);
            return Ok(cached_value);
        }

        // Cache miss - fetch from CLI
        match self
            .cli_manager
            .execute(&["plm", "run", "get", run_id, "--output", "json"], None)
            .await
        {
            Ok(result) => {
                // Cache the result - let cache type detection handle TTL based on run status
                self.cache.insert(&context, cache_key, result.clone()).await;
                debug!("Fetched and cached run details for: {}", run_id);
                Ok(result)
            }
            Err(e) => Err(e),
        }
    }

    async fn get_all_runs(&self) -> Result<Value> {
        let context = self.get_cache_context();
        let cache_key = PlmCache::all_runs_key();

        // Try cache first (semi-dynamic data)
        if let Some(cached_value) = self.cache.get(&context, &cache_key).await {
            debug!("Returning cached all runs list");
            return Ok(cached_value);
        }

        // Cache miss - fetch from CLI
        match self
            .cli_manager
            .execute(&["plm", "run", "list", "--output", "json"], None)
            .await
        {
            Ok(result) => {
                // Cache the result (semi-dynamic data)
                self.cache.insert(&context, cache_key, result.clone()).await;
                debug!("Fetched and cached all runs list");
                Ok(result)
            }
            Err(e) => Err(e),
        }
    }

    async fn get_run_by_id(&self, run_id: &str) -> Result<Value> {
        // Reuse the run details caching logic
        self.get_run_details("", run_id).await
    }

    async fn get_all_tasks(&self) -> Result<Value> {
        let context = self.get_cache_context();
        let cache_key = PlmCache::tasks_key();

        // Try cache first (immutable/semi-static data)
        if let Some(cached_value) = self.cache.get(&context, &cache_key).await {
            debug!("Returning cached tasks list");
            return Ok(cached_value);
        }

        // Cache miss - fetch from CLI
        match self
            .cli_manager
            .execute(&["plm", "task", "list", "--output", "json"], None)
            .await
        {
            Ok(result) => {
                // Cache the result (task libraries are relatively static)
                self.cache.insert(&context, cache_key, result.clone()).await;
                debug!("Fetched and cached tasks list");
                Ok(result)
            }
            Err(e) => Err(e),
        }
    }

    async fn get_task_details(&self, task_id: &str) -> Result<Value> {
        let context = self.get_cache_context();
        let cache_key = format!("task:details:{task_id}");

        // Try cache first (task details are immutable)
        if let Some(cached_value) = self.cache.get(&context, &cache_key).await {
            debug!("Returning cached task details for: {}", task_id);
            return Ok(cached_value);
        }

        // Cache miss - fetch from CLI
        match self
            .cli_manager
            .execute(&["plm", "task", "get", task_id, "--output", "json"], None)
            .await
        {
            Ok(result) => {
                // Cache the result (task definitions are immutable)
                self.cache.insert(&context, cache_key, result.clone()).await;
                debug!("Fetched and cached task details for: {}", task_id);
                Ok(result)
            }
            Err(e) => Err(e),
        }
    }

    async fn get_pipeline_resources(&self) -> Result<Value> {
        let context = self.get_cache_context();
        let cache_key = PlmCache::pipeline_resources_key();

        // Try cache first (semi-dynamic data)
        if let Some(cached_value) = self.cache.get(&context, &cache_key).await {
            debug!("Returning cached pipeline resources");
            return Ok(cached_value);
        }

        // Cache miss - fetch from CLI
        match self
            .cli_manager
            .execute(&["plm", "resource", "list", "--output", "json"], None)
            .await
        {
            Ok(result) => {
                // Cache the result (resource assignments change semi-frequently)
                self.cache.insert(&context, cache_key, result.clone()).await;
                debug!("Fetched and cached pipeline resources");
                Ok(result)
            }
            Err(e) => Err(e),
        }
    }

    async fn get_pipeline_groups(&self) -> Result<Value> {
        // Groups might require specific access config or pipeline context
        match self
            .cli_manager
            .execute(&["plm", "group", "list", "--output", "json"], None)
            .await
        {
            Ok(result) => Ok(result),
            Err(_) => {
                // Fallback to placeholder if command structure is different
                Ok(serde_json::json!({
                    "message": "Group listing requires specific pipeline or access config context",
                    "suggestion": "Use pipeline-specific group queries"
                }))
            }
        }
    }

    async fn get_pipeline_secrets(&self) -> Result<Value> {
        // Secrets listing might require specific pipeline context
        match self
            .cli_manager
            .execute(&["plm", "secret", "list", "--output", "json"], None)
            .await
        {
            Ok(result) => Ok(result),
            Err(_) => {
                // Fallback to placeholder if command structure is different
                Ok(serde_json::json!({
                    "message": "Secret listing requires specific pipeline context",
                    "suggestion": "Use pipeline-specific secret queries"
                }))
            }
        }
    }

    async fn get_pipeline_triggers(&self) -> Result<Value> {
        // Triggers might require specific pipeline context
        match self
            .cli_manager
            .execute(&["plm", "trigger", "list", "--output", "json"], None)
            .await
        {
            Ok(result) => Ok(result),
            Err(_) => {
                // Fallback to placeholder if command structure is different
                Ok(serde_json::json!({
                    "message": "Trigger listing requires specific pipeline context",
                    "suggestion": "Use pipeline-specific trigger queries"
                }))
            }
        }
    }

    async fn get_access_configs(&self) -> Result<Value> {
        // Access config might require specific context
        match self
            .cli_manager
            .execute(&["plm", "access-config", "list", "--output", "json"], None)
            .await
        {
            Ok(result) => Ok(result),
            Err(_) => {
                // Fallback to placeholder if command structure is different
                Ok(serde_json::json!({
                    "message": "Access config listing requires specific context",
                    "suggestion": "Use specific access config queries"
                }))
            }
        }
    }
}
