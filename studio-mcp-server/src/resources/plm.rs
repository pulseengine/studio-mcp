//! PLM (Pipeline Management) resource provider

use std::sync::Arc;
use pulseengine_mcp_protocol::{Resource, ResourceContents, TextResourceContents};
use studio_mcp_shared::{StudioConfig, Result, StudioError, ResourceUri};
use studio_cli_manager::CliManager;
use serde_json::Value;
use tracing::{debug, warn};

pub struct PlmResourceProvider {
    cli_manager: Arc<CliManager>,
    config: StudioConfig,
}

impl PlmResourceProvider {
    pub fn new(cli_manager: Arc<CliManager>, config: StudioConfig) -> Self {
        Self {
            cli_manager,
            config,
        }
    }

    pub async fn list_resources(&self) -> Result<Vec<Resource>> {
        let mut resources = Vec::new();

        // PLM base resources
        resources.extend(vec![
            Resource {
                uri: "studio://plm/pipelines/".to_string(),
                name: "Pipelines".to_string(),
                description: Some("List all available pipelines".to_string()),
                mime_type: Some("application/json".to_string()),
            },
            Resource {
                uri: "studio://plm/projects/".to_string(),
                name: "Projects".to_string(),
                description: Some("List all PLM projects".to_string()),
                mime_type: Some("application/json".to_string()),
            },
            Resource {
                uri: "studio://plm/templates/".to_string(),
                name: "Pipeline Templates".to_string(),
                description: Some("Available pipeline templates".to_string()),
                mime_type: Some("application/json".to_string()),
            },
        ]);

        // Try to fetch dynamic pipeline resources
        match self.get_pipeline_list().await {
            Ok(pipelines) => {
                for pipeline in pipelines {
                    if let Some(pipeline_id) = pipeline.get("id").and_then(|v| v.as_str()) {
                        let pipeline_name = pipeline.get("name")
                            .and_then(|v| v.as_str())
                            .unwrap_or("Unknown Pipeline");

                        // Pipeline info resource
                        resources.push(Resource {
                            uri: format!("studio://plm/pipelines/{}/info", pipeline_id),
                            name: format!("Pipeline: {}", pipeline_name),
                            description: Some(format!("Detailed information for pipeline {}", pipeline_id)),
                            mime_type: Some("application/json".to_string()),
                        });

                        // Pipeline tasks resource
                        resources.push(Resource {
                            uri: format!("studio://plm/pipelines/{}/tasks/", pipeline_id),
                            name: format!("Tasks: {}", pipeline_name),
                            description: Some(format!("Tasks for pipeline {}", pipeline_id)),
                            mime_type: Some("application/json".to_string()),
                        });

                        // Pipeline history resource
                        resources.push(Resource {
                            uri: format!("studio://plm/pipelines/{}/history", pipeline_id),
                            name: format!("History: {}", pipeline_name),
                            description: Some(format!("Execution history for pipeline {}", pipeline_id)),
                            mime_type: Some("application/json".to_string()),
                        });
                    }
                }
            }
            Err(e) => {
                warn!("Failed to fetch pipeline list for resource discovery: {}", e);
                // Continue with static resources only
            }
        }

        debug!("PLM provider listed {} resources", resources.len());
        Ok(resources)
    }

    pub async fn read_resource(&self, uri: &ResourceUri) -> Result<Vec<ResourceContents>> {
        debug!("PLM provider reading resource: {}", uri.to_string());

        match uri.path.get(1).map(|s| s.as_str()) {
            Some("pipelines") => {
                self.read_pipeline_resource(uri).await
            }
            Some("projects") => {
                self.read_project_resource(uri).await
            }
            Some("templates") => {
                self.read_templates_resource().await
            }
            None => {
                // PLM root resource
                self.read_plm_root().await
            }
            Some(resource_type) => {
                Err(StudioError::ResourceNotFound(format!("PLM resource type '{}' not found", resource_type)))
            }
        }
    }

    async fn read_plm_root(&self) -> Result<Vec<ResourceContents>> {
        let content = serde_json::json!({
            "name": "Pipeline Management (PLM)",
            "description": "WindRiver Studio Pipeline Management system",
            "version": "1.0",
            "capabilities": [
                "pipeline_management",
                "task_execution",
                "artifact_handling",
                "build_automation"
            ],
            "endpoints": {
                "pipelines": "studio://plm/pipelines/",
                "projects": "studio://plm/projects/",
                "templates": "studio://plm/templates/"
            }
        });

        Ok(vec![ResourceContents::Text(TextResourceContents {
            text: content.to_string(),
            mime_type: Some("application/json".to_string()),
        })])
    }

    async fn read_pipeline_resource(&self, uri: &ResourceUri) -> Result<Vec<ResourceContents>> {
        match uri.path.get(2) {
            None => {
                // List all pipelines
                let pipelines = self.get_pipeline_list().await?;
                let content = serde_json::json!({
                    "pipelines": pipelines,
                    "total": pipelines.len()
                });

                Ok(vec![ResourceContents::Text(TextResourceContents {
                    text: content.to_string(),
                    mime_type: Some("application/json".to_string()),
                })])
            }
            Some(pipeline_id) => {
                match uri.path.get(3).map(|s| s.as_str()) {
                    Some("info") => {
                        let pipeline_info = self.get_pipeline_info(pipeline_id).await?;
                        Ok(vec![ResourceContents::Text(TextResourceContents {
                            text: pipeline_info.to_string(),
                            mime_type: Some("application/json".to_string()),
                        })])
                    }
                    Some("tasks") => {
                        let tasks = self.get_pipeline_tasks(pipeline_id).await?;
                        let content = serde_json::json!({
                            "pipeline_id": pipeline_id,
                            "tasks": tasks,
                            "total": tasks.as_array().map(|arr| arr.len()).unwrap_or(0)
                        });

                        Ok(vec![ResourceContents::Text(TextResourceContents {
                            text: content.to_string(),
                            mime_type: Some("application/json".to_string()),
                        })])
                    }
                    Some("history") => {
                        // Placeholder for pipeline history
                        let content = serde_json::json!({
                            "pipeline_id": pipeline_id,
                            "history": [],
                            "message": "Pipeline history not yet implemented"
                        });

                        Ok(vec![ResourceContents::Text(TextResourceContents {
                            text: content.to_string(),
                            mime_type: Some("application/json".to_string()),
                        })])
                    }
                    Some(task_id) => {
                        // Individual task info
                        let task_info = self.get_task_info(task_id).await?;
                        Ok(vec![ResourceContents::Text(TextResourceContents {
                            text: task_info.to_string(),
                            mime_type: Some("application/json".to_string()),
                        })])
                    }
                    None => {
                        // Individual pipeline info (shorthand)
                        let pipeline_info = self.get_pipeline_info(pipeline_id).await?;
                        Ok(vec![ResourceContents::Text(TextResourceContents {
                            text: pipeline_info.to_string(),
                            mime_type: Some("application/json".to_string()),
                        })])
                    }
                }
            }
        }
    }

    async fn read_project_resource(&self, _uri: &ResourceUri) -> Result<Vec<ResourceContents>> {
        // Placeholder for project listing
        let content = serde_json::json!({
            "projects": [],
            "message": "Project listing not yet implemented",
            "note": "Use CLI command 'studio-cli plm project list' for current projects"
        });

        Ok(vec![ResourceContents::Text(TextResourceContents {
            text: content.to_string(),
            mime_type: Some("application/json".to_string()),
        })])
    }

    async fn read_templates_resource(&self) -> Result<Vec<ResourceContents>> {
        // Placeholder for pipeline templates
        let content = serde_json::json!({
            "templates": [
                {
                    "name": "basic-build",
                    "description": "Basic build pipeline template",
                    "stages": ["checkout", "build", "test", "package"]
                },
                {
                    "name": "ci-cd",
                    "description": "Complete CI/CD pipeline template",
                    "stages": ["checkout", "build", "test", "security-scan", "deploy"]
                }
            ],
            "message": "Pipeline templates are placeholder data"
        });

        Ok(vec![ResourceContents::Text(TextResourceContents {
            text: content.to_string(),
            mime_type: Some("application/json".to_string()),
        })])
    }

    // CLI interaction methods
    async fn get_pipeline_list(&self) -> Result<Vec<Value>> {
        match self.cli_manager.execute(&["plm", "pipeline", "list"], None).await {
            Ok(result) => {
                if let Some(pipelines) = result.as_array() {
                    Ok(pipelines.clone())
                } else if let Some(obj) = result.as_object() {
                    if let Some(pipelines) = obj.get("pipelines").and_then(|v| v.as_array()) {
                        Ok(pipelines.clone())
                    } else {
                        Ok(vec![result])
                    }
                } else {
                    Ok(vec![])
                }
            }
            Err(e) => {
                warn!("Failed to list pipelines: {}", e);
                // Return empty list instead of failing
                Ok(vec![])
            }
        }
    }

    async fn get_pipeline_info(&self, pipeline_id: &str) -> Result<Value> {
        self.cli_manager.execute(&["plm", "pipeline", "get", pipeline_id], None).await
    }

    async fn get_pipeline_tasks(&self, pipeline_id: &str) -> Result<Value> {
        self.cli_manager.execute(&["plm", "task", "list", "--pipeline", pipeline_id], None).await
    }

    async fn get_task_info(&self, task_id: &str) -> Result<Value> {
        self.cli_manager.execute(&["plm", "task", "get", task_id], None).await
    }
}