//! PLM (Pipeline Management) tool provider

use std::sync::Arc;
use std::collections::HashMap;
use pulseengine_mcp_protocol::{Tool, ToolInputSchema, Content, TextContent};
use studio_mcp_shared::{StudioConfig, Result, StudioError};
use studio_cli_manager::CliManager;
use serde_json::{Value, json};
use tracing::{debug, error};

pub struct PlmToolProvider {
    cli_manager: Arc<CliManager>,
    config: StudioConfig,
}

impl PlmToolProvider {
    pub fn new(cli_manager: Arc<CliManager>, config: StudioConfig) -> Self {
        Self {
            cli_manager,
            config,
        }
    }

    pub async fn list_tools(&self) -> Result<Vec<Tool>> {
        let tools = vec![
            // Pipeline management tools
            Tool {
                name: "plm_list_pipelines".to_string(),
                description: "List all available pipelines, optionally filtered by project".to_string(),
                input_schema: ToolInputSchema {
                    schema_type: "object".to_string(),
                    properties: Some({
                        let mut props = HashMap::new();
                        props.insert("project_id".to_string(), json!({
                            "type": "string",
                            "description": "Optional project ID to filter pipelines"
                        }));
                        props
                    }),
                    required: Some(vec![]),
                    additional_properties: None,
                },
            },
            Tool {
                name: "plm_get_pipeline".to_string(),
                description: "Get detailed information about a specific pipeline".to_string(),
                input_schema: ToolInputSchema {
                    schema_type: "object".to_string(),
                    properties: Some({
                        let mut props = HashMap::new();
                        props.insert("pipeline_id".to_string(), json!({
                            "type": "string",
                            "description": "ID of the pipeline to retrieve"
                        }));
                        props
                    }),
                    required: Some(vec!["pipeline_id".to_string()]),
                    additional_properties: None,
                },
            },
            Tool {
                name: "plm_run_pipeline".to_string(),
                description: "Start execution of a pipeline".to_string(),
                input_schema: ToolInputSchema {
                    schema_type: "object".to_string(),
                    properties: Some({
                        let mut props = HashMap::new();
                        props.insert("pipeline_id".to_string(), json!({
                            "type": "string",
                            "description": "ID of the pipeline to run"
                        }));
                        props.insert("parameters".to_string(), json!({
                            "type": "object",
                            "description": "Optional parameters to pass to the pipeline",
                            "additionalProperties": true
                        }));
                        props
                    }),
                    required: Some(vec!["pipeline_id".to_string()]),
                    additional_properties: None,
                },
            },
            Tool {
                name: "plm_stop_pipeline".to_string(),
                description: "Stop a running pipeline".to_string(),
                input_schema: ToolInputSchema {
                    schema_type: "object".to_string(),
                    properties: Some({
                        let mut props = HashMap::new();
                        props.insert("pipeline_id".to_string(), json!({
                            "type": "string",
                            "description": "ID of the pipeline to stop"
                        }));
                        props
                    }),
                    required: Some(vec!["pipeline_id".to_string()]),
                    additional_properties: None,
                },
            },
            
            // Task management tools
            Tool {
                name: "plm_list_tasks".to_string(),
                description: "List tasks for a specific pipeline".to_string(),
                input_schema: ToolInputSchema {
                    schema_type: "object".to_string(),
                    properties: Some({
                        let mut props = HashMap::new();
                        props.insert("pipeline_id".to_string(), json!({
                            "type": "string",
                            "description": "ID of the pipeline to list tasks for"
                        }));
                        props
                    }),
                    required: Some(vec!["pipeline_id".to_string()]),
                    additional_properties: None,
                },
            },
            Tool {
                name: "plm_get_task".to_string(),
                description: "Get detailed information about a specific task".to_string(),
                input_schema: ToolInputSchema {
                    schema_type: "object".to_string(),
                    properties: Some({
                        let mut props = HashMap::new();
                        props.insert("task_id".to_string(), json!({
                            "type": "string",
                            "description": "ID of the task to retrieve"
                        }));
                        props
                    }),
                    required: Some(vec!["task_id".to_string()]),
                    additional_properties: None,
                },
            },
            Tool {
                name: "plm_get_task_logs".to_string(),
                description: "Get logs for a specific task".to_string(),
                input_schema: ToolInputSchema {
                    schema_type: "object".to_string(),
                    properties: Some({
                        let mut props = HashMap::new();
                        props.insert("task_id".to_string(), json!({
                            "type": "string",
                            "description": "ID of the task to get logs for"
                        }));
                        props.insert("lines".to_string(), json!({
                            "type": "integer",
                            "description": "Number of log lines to retrieve (default: 100)",
                            "minimum": 1,
                            "maximum": 10000
                        }));
                        props
                    }),
                    required: Some(vec!["task_id".to_string()]),
                    additional_properties: None,
                },
            },
            
            // Project management tools
            Tool {
                name: "plm_list_projects".to_string(),
                description: "List all PLM projects".to_string(),
                input_schema: ToolInputSchema {
                    schema_type: "object".to_string(),
                    properties: Some(HashMap::new()),
                    required: Some(vec![]),
                    additional_properties: None,
                },
            },
        ];

        debug!("PLM provider listed {} tools", tools.len());
        Ok(tools)
    }

    pub async fn call_tool(&self, name: &str, arguments: Option<Value>) -> Result<Vec<Content>> {
        debug!("PLM provider calling tool: {} with args: {:?}", name, arguments);

        let args = arguments.unwrap_or(Value::Object(serde_json::Map::new()));

        match name {
            "plm_list_pipelines" => self.list_pipelines(args).await,
            "plm_get_pipeline" => self.get_pipeline(args).await,
            "plm_run_pipeline" => self.run_pipeline(args).await,
            "plm_stop_pipeline" => self.stop_pipeline(args).await,
            "plm_list_tasks" => self.list_tasks(args).await,
            "plm_get_task" => self.get_task(args).await,
            "plm_get_task_logs" => self.get_task_logs(args).await,
            "plm_list_projects" => self.list_projects(args).await,
            _ => {
                error!("Unknown PLM tool: {}", name);
                Err(StudioError::InvalidOperation(format!("PLM tool '{}' not found", name)))
            }
        }
    }

    async fn list_pipelines(&self, args: Value) -> Result<Vec<Content>> {
        let project_id = args.get("project_id").and_then(|v| v.as_str());
        
        let mut cli_args = vec!["plm", "pipeline", "list"];
        if let Some(project) = project_id {
            cli_args.extend_from_slice(&["--project", project]);
        }

        match self.cli_manager.execute(&cli_args, None).await {
            Ok(result) => {
                let response = json!({
                    "success": true,
                    "data": result,
                    "project_filter": project_id
                });

                Ok(vec![Content::Text(TextContent {
                    text: serde_json::to_string_pretty(&response)?,
                    ..Default::default()
                })])
            }
            Err(e) => {
                error!("Failed to list pipelines: {}", e);
                let error_response = json!({
                    "success": false,
                    "error": e.to_string(),
                    "message": "Failed to retrieve pipeline list"
                });

                Ok(vec![Content::Text(TextContent {
                    text: serde_json::to_string_pretty(&error_response)?,
                    ..Default::default()
                })])
            }
        }
    }

    async fn get_pipeline(&self, args: Value) -> Result<Vec<Content>> {
        let pipeline_id = args.get("pipeline_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| StudioError::InvalidOperation("pipeline_id is required".to_string()))?;

        match self.cli_manager.execute(&["plm", "pipeline", "get", pipeline_id], None).await {
            Ok(result) => {
                let response = json!({
                    "success": true,
                    "pipeline_id": pipeline_id,
                    "data": result
                });

                Ok(vec![Content::Text(TextContent {
                    text: serde_json::to_string_pretty(&response)?,
                    ..Default::default()
                })])
            }
            Err(e) => {
                error!("Failed to get pipeline {}: {}", pipeline_id, e);
                let error_response = json!({
                    "success": false,
                    "pipeline_id": pipeline_id,
                    "error": e.to_string(),
                    "message": "Failed to retrieve pipeline information"
                });

                Ok(vec![Content::Text(TextContent {
                    text: serde_json::to_string_pretty(&error_response)?,
                    ..Default::default()
                })])
            }
        }
    }

    async fn run_pipeline(&self, args: Value) -> Result<Vec<Content>> {
        let pipeline_id = args.get("pipeline_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| StudioError::InvalidOperation("pipeline_id is required".to_string()))?;

        let mut cli_args = vec!["plm", "pipeline", "run", pipeline_id];
        
        // Add parameters if provided
        if let Some(parameters) = args.get("parameters") {
            if let Some(params_obj) = parameters.as_object() {
                for (key, value) in params_obj {
                    if let Some(val_str) = value.as_str() {
                        cli_args.extend_from_slice(&["--param", &format!("{}={}", key, val_str)]);
                    }
                }
            }
        }

        match self.cli_manager.execute(&cli_args, None).await {
            Ok(result) => {
                let response = json!({
                    "success": true,
                    "pipeline_id": pipeline_id,
                    "action": "started",
                    "data": result
                });

                Ok(vec![Content::Text(TextContent {
                    text: serde_json::to_string_pretty(&response)?,
                    ..Default::default()
                })])
            }
            Err(e) => {
                error!("Failed to run pipeline {}: {}", pipeline_id, e);
                let error_response = json!({
                    "success": false,
                    "pipeline_id": pipeline_id,
                    "action": "start_failed",
                    "error": e.to_string(),
                    "message": "Failed to start pipeline"
                });

                Ok(vec![Content::Text(TextContent {
                    text: serde_json::to_string_pretty(&error_response)?,
                    ..Default::default()
                })])
            }
        }
    }

    async fn stop_pipeline(&self, args: Value) -> Result<Vec<Content>> {
        let pipeline_id = args.get("pipeline_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| StudioError::InvalidOperation("pipeline_id is required".to_string()))?;

        match self.cli_manager.execute(&["plm", "pipeline", "stop", pipeline_id], None).await {
            Ok(result) => {
                let response = json!({
                    "success": true,
                    "pipeline_id": pipeline_id,
                    "action": "stopped",
                    "data": result
                });

                Ok(vec![Content::Text(TextContent {
                    text: serde_json::to_string_pretty(&response)?,
                    ..Default::default()
                })])
            }
            Err(e) => {
                error!("Failed to stop pipeline {}: {}", pipeline_id, e);
                let error_response = json!({
                    "success": false,
                    "pipeline_id": pipeline_id,
                    "action": "stop_failed",
                    "error": e.to_string(),
                    "message": "Failed to stop pipeline"
                });

                Ok(vec![Content::Text(TextContent {
                    text: serde_json::to_string_pretty(&error_response)?,
                    ..Default::default()
                })])
            }
        }
    }

    async fn list_tasks(&self, args: Value) -> Result<Vec<Content>> {
        let pipeline_id = args.get("pipeline_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| StudioError::InvalidOperation("pipeline_id is required".to_string()))?;

        match self.cli_manager.execute(&["plm", "task", "list", "--pipeline", pipeline_id], None).await {
            Ok(result) => {
                let response = json!({
                    "success": true,
                    "pipeline_id": pipeline_id,
                    "data": result
                });

                Ok(vec![Content::Text(TextContent {
                    text: serde_json::to_string_pretty(&response)?,
                    ..Default::default()
                })])
            }
            Err(e) => {
                error!("Failed to list tasks for pipeline {}: {}", pipeline_id, e);
                let error_response = json!({
                    "success": false,
                    "pipeline_id": pipeline_id,
                    "error": e.to_string(),
                    "message": "Failed to retrieve task list"
                });

                Ok(vec![Content::Text(TextContent {
                    text: serde_json::to_string_pretty(&error_response)?,
                    ..Default::default()
                })])
            }
        }
    }

    async fn get_task(&self, args: Value) -> Result<Vec<Content>> {
        let task_id = args.get("task_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| StudioError::InvalidOperation("task_id is required".to_string()))?;

        match self.cli_manager.execute(&["plm", "task", "get", task_id], None).await {
            Ok(result) => {
                let response = json!({
                    "success": true,
                    "task_id": task_id,
                    "data": result
                });

                Ok(vec![Content::Text(TextContent {
                    text: serde_json::to_string_pretty(&response)?,
                    ..Default::default()
                })])
            }
            Err(e) => {
                error!("Failed to get task {}: {}", task_id, e);
                let error_response = json!({
                    "success": false,
                    "task_id": task_id,
                    "error": e.to_string(),
                    "message": "Failed to retrieve task information"
                });

                Ok(vec![Content::Text(TextContent {
                    text: serde_json::to_string_pretty(&error_response)?,
                    ..Default::default()
                })])
            }
        }
    }

    async fn get_task_logs(&self, args: Value) -> Result<Vec<Content>> {
        let task_id = args.get("task_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| StudioError::InvalidOperation("task_id is required".to_string()))?;

        let lines = args.get("lines")
            .and_then(|v| v.as_u64())
            .unwrap_or(100);

        let mut cli_args = vec!["plm", "task", "logs", task_id];
        if lines != 100 {
            cli_args.extend_from_slice(&["--lines", &lines.to_string()]);
        }

        match self.cli_manager.execute(&cli_args, None).await {
            Ok(result) => {
                let response = json!({
                    "success": true,
                    "task_id": task_id,
                    "lines_requested": lines,
                    "data": result
                });

                Ok(vec![Content::Text(TextContent {
                    text: serde_json::to_string_pretty(&response)?,
                    ..Default::default()
                })])
            }
            Err(e) => {
                error!("Failed to get logs for task {}: {}", task_id, e);
                let error_response = json!({
                    "success": false,
                    "task_id": task_id,
                    "error": e.to_string(),
                    "message": "Failed to retrieve task logs"
                });

                Ok(vec![Content::Text(TextContent {
                    text: serde_json::to_string_pretty(&error_response)?,
                    ..Default::default()
                })])
            }
        }
    }

    async fn list_projects(&self, _args: Value) -> Result<Vec<Content>> {
        match self.cli_manager.execute(&["plm", "project", "list"], None).await {
            Ok(result) => {
                let response = json!({
                    "success": true,
                    "data": result
                });

                Ok(vec![Content::Text(TextContent {
                    text: serde_json::to_string_pretty(&response)?,
                    ..Default::default()
                })])
            }
            Err(e) => {
                error!("Failed to list projects: {}", e);
                let error_response = json!({
                    "success": false,
                    "error": e.to_string(),
                    "message": "Failed to retrieve project list"
                });

                Ok(vec![Content::Text(TextContent {
                    text: serde_json::to_string_pretty(&error_response)?,
                    ..Default::default()
                })])
            }
        }
    }
}