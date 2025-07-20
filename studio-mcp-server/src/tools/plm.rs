//! PLM (Pipeline Management) tool provider

use std::sync::Arc;
use pulseengine_mcp_protocol::{Tool, Content};
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
                description: "List all available pipelines with optional filtering".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "name": {
                            "type": "string",
                            "description": "Filter by pipeline name"
                        },
                        "user": {
                            "type": "string",
                            "description": "Filter by user who created the pipeline"
                        },
                        "limit": {
                            "type": "integer",
                            "description": "Limit number of results (default 10, 0 for all)",
                            "minimum": 0
                        },
                        "offset": {
                            "type": "integer",
                            "description": "Starting offset for results (default 1)",
                            "minimum": 1
                        }
                    },
                    "required": []
                }),
            },
            Tool {
                name: "plm_get_pipeline".to_string(),
                description: "Get pipeline definition (YAML/JSON) for a specific pipeline".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "pipeline_id": {
                            "type": "string",
                            "description": "Name or ID of the pipeline to retrieve"
                        }
                    },
                    "required": ["pipeline_id"]
                }),
            },
            Tool {
                name: "plm_start_pipeline".to_string(),
                description: "Start execution of a pipeline with optional parameters".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "pipeline_name": {
                            "type": "string",
                            "description": "Name of the pipeline to run"
                        },
                        "pipeline_id": {
                            "type": "string",
                            "description": "ID of the pipeline to run (alternative to name)"
                        },
                        "parameters": {
                            "type": "array",
                            "description": "Pipeline parameters as key=value pairs",
                            "items": {
                                "type": "string",
                                "pattern": "^[^=]+=.*$"
                            }
                        },
                        "config": {
                            "type": "array", 
                            "description": "Pipeline config settings as key=value pairs",
                            "items": {
                                "type": "string",
                                "pattern": "^[^=]+=.*$"
                            }
                        },
                        "env": {
                            "type": "array",
                            "description": "Environment variables as key=value pairs", 
                            "items": {
                                "type": "string",
                                "pattern": "^[^=]+=.*$"
                            }
                        },
                        "follow": {
                            "type": "boolean",
                            "description": "Stream logs until completion"
                        }
                    },
                    "required": []
                }),
            },
            Tool {
                name: "plm_cancel_run".to_string(),
                description: "Cancel a running pipeline execution".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "run_id": {
                            "type": "string",
                            "description": "ID of the pipeline run to cancel"
                        }
                    },
                    "required": ["run_id"]
                }),
            },
            
            // Pipeline run management tools
            Tool {
                name: "plm_list_runs".to_string(),
                description: "List pipeline runs, optionally filtered by pipeline".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "pipeline_name": {
                            "type": "string",
                            "description": "Filter runs by pipeline name"
                        },
                        "pipeline_id": {
                            "type": "string", 
                            "description": "Filter runs by pipeline ID"
                        }
                    },
                    "required": []
                }),
            },
            Tool {
                name: "plm_get_run".to_string(),
                description: "Get detailed information about a specific pipeline run".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "run_id": {
                            "type": "string",
                            "description": "ID of the pipeline run to retrieve"
                        }
                    },
                    "required": ["run_id"]
                }),
            },
            Tool {
                name: "plm_get_run_log".to_string(),
                description: "Get logs for a specific pipeline run".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "run_id": {
                            "type": "string",
                            "description": "ID of the pipeline run to get logs for"
                        }
                    },
                    "required": ["run_id"]
                }),
            },
            Tool {
                name: "plm_get_run_events".to_string(),
                description: "Get events for a specific pipeline run".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "run_id": {
                            "type": "string",
                            "description": "ID of the pipeline run to get events for"
                        }
                    },
                    "required": ["run_id"]
                }),
            },
            
            // Resource management tools
            Tool {
                name: "plm_list_resources".to_string(),
                description: "List available pipeline resources".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "pipeline": {
                            "type": "string",
                            "description": "Filter by pipeline name or ID"
                        },
                        "access_config": {
                            "type": "string",
                            "description": "Filter by access config name or WRRN"
                        }
                    },
                    "required": []
                }),
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
            "plm_start_pipeline" => self.start_pipeline(args).await,
            "plm_cancel_run" => self.cancel_run(args).await,
            "plm_list_runs" => self.list_runs(args).await,
            "plm_get_run" => self.get_run(args).await,
            "plm_get_run_log" => self.get_run_log(args).await,
            "plm_get_run_events" => self.get_run_events(args).await,
            "plm_list_resources" => self.list_resources(args).await,
            _ => {
                error!("Unknown PLM tool: {}", name);
                Err(StudioError::InvalidOperation(format!("PLM tool '{}' not found", name)))
            }
        }
    }

    async fn list_pipelines(&self, args: Value) -> Result<Vec<Content>> {
        let mut cli_args = vec!["plm", "pipeline", "list", "--output", "json"];
        
        // Add optional filters
        let mut name_filter = None;
        let mut user_filter = None;
        let mut limit_str = String::new();
        let mut offset_str = String::new();
        
        if let Some(name) = args.get("name").and_then(|v| v.as_str()) {
            cli_args.extend_from_slice(&["--name", name]);
            name_filter = Some(name);
        }
        
        if let Some(user) = args.get("user").and_then(|v| v.as_str()) {
            cli_args.extend_from_slice(&["--user", user]);
            user_filter = Some(user);
        }
        
        if let Some(limit) = args.get("limit").and_then(|v| v.as_u64()) {
            limit_str = limit.to_string();
            cli_args.extend_from_slice(&["--limit", &limit_str]);
        }
        
        if let Some(offset) = args.get("offset").and_then(|v| v.as_u64()) {
            offset_str = offset.to_string();
            cli_args.extend_from_slice(&["--offset", &offset_str]);
        }

        match self.cli_manager.execute(&cli_args, None).await {
            Ok(result) => {
                let response = json!({
                    "success": true,
                    "data": result,
                    "filters": {
                        "name": name_filter,
                        "user": user_filter,
                        "limit": args.get("limit"),
                        "offset": args.get("offset")
                    }
                });

                Ok(vec![Content::Text {
                    text: serde_json::to_string_pretty(&response)?,
                }])
            }
            Err(e) => {
                error!("Failed to list pipelines: {}", e);
                let error_response = json!({
                    "success": false,
                    "error": e.to_string(),
                    "message": "Failed to retrieve pipeline list"
                });

                Ok(vec![Content::Text {
                    text: serde_json::to_string_pretty(&error_response)?,
                }])
            }
        }
    }

    async fn get_pipeline(&self, args: Value) -> Result<Vec<Content>> {
        let pipeline_id = args.get("pipeline_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| StudioError::InvalidOperation("pipeline_id is required".to_string()))?;

        match self.cli_manager.execute(&["plm", "pipeline", "get", pipeline_id, "--output", "yaml"], None).await {
            Ok(result) => {
                let response = json!({
                    "success": true,
                    "pipeline_id": pipeline_id,
                    "format": "yaml",
                    "data": result
                });

                Ok(vec![Content::Text {
                    text: serde_json::to_string_pretty(&response)?,
                }])
            }
            Err(e) => {
                error!("Failed to get pipeline {}: {}", pipeline_id, e);
                let error_response = json!({
                    "success": false,
                    "pipeline_id": pipeline_id,
                    "error": e.to_string(),
                    "message": "Failed to retrieve pipeline definition"
                });

                Ok(vec![Content::Text {
                    text: serde_json::to_string_pretty(&error_response)?,
                }])
            }
        }
    }

    async fn start_pipeline(&self, args: Value) -> Result<Vec<Content>> {
        let mut cli_args = vec!["plm", "run", "start", "--output", "json"];
        
        // Either pipeline name or ID is required
        let pipeline_identifier = if let Some(name) = args.get("pipeline_name").and_then(|v| v.as_str()) {
            cli_args.extend_from_slice(&["--name", name]);
            name
        } else if let Some(id) = args.get("pipeline_id").and_then(|v| v.as_str()) {
            cli_args.extend_from_slice(&["--id", id]);
            id
        } else {
            return Err(StudioError::InvalidOperation("Either pipeline_name or pipeline_id is required".to_string()));
        };
        
        // Add parameters if provided
        if let Some(parameters) = args.get("parameters").and_then(|v| v.as_array()) {
            for param in parameters {
                if let Some(param_str) = param.as_str() {
                    cli_args.extend_from_slice(&["--param", param_str]);
                }
            }
        }
        
        // Add config settings if provided
        if let Some(config) = args.get("config").and_then(|v| v.as_array()) {
            for conf in config {
                if let Some(conf_str) = conf.as_str() {
                    cli_args.extend_from_slice(&["--config", conf_str]);
                }
            }
        }
        
        // Add environment variables if provided
        if let Some(env) = args.get("env").and_then(|v| v.as_array()) {
            for env_var in env {
                if let Some(env_str) = env_var.as_str() {
                    cli_args.extend_from_slice(&["--env", env_str]);
                }
            }
        }
        
        // Add follow flag if requested
        if args.get("follow").and_then(|v| v.as_bool()).unwrap_or(false) {
            cli_args.push("--follow");
        }

        match self.cli_manager.execute(&cli_args, None).await {
            Ok(result) => {
                let response = json!({
                    "success": true,
                    "pipeline": pipeline_identifier,
                    "action": "started",
                    "data": result,
                    "parameters": args.get("parameters"),
                    "config": args.get("config"),
                    "env": args.get("env")
                });

                Ok(vec![Content::Text {
                    text: serde_json::to_string_pretty(&response)?,
                }])
            }
            Err(e) => {
                error!("Failed to start pipeline {}: {}", pipeline_identifier, e);
                let error_response = json!({
                    "success": false,
                    "pipeline": pipeline_identifier,
                    "action": "start_failed",
                    "error": e.to_string(),
                    "message": "Failed to start pipeline execution"
                });

                Ok(vec![Content::Text {
                    text: serde_json::to_string_pretty(&error_response)?,
                }])
            }
        }
    }

    async fn cancel_run(&self, args: Value) -> Result<Vec<Content>> {
        let run_id = args.get("run_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| StudioError::InvalidOperation("run_id is required".to_string()))?;

        match self.cli_manager.execute(&["plm", "run", "cancel", run_id, "--output", "json"], None).await {
            Ok(result) => {
                let response = json!({
                    "success": true,
                    "run_id": run_id,
                    "action": "cancelled",
                    "data": result
                });

                Ok(vec![Content::Text {
                    text: serde_json::to_string_pretty(&response)?,
                }])
            }
            Err(e) => {
                error!("Failed to cancel run {}: {}", run_id, e);
                let error_response = json!({
                    "success": false,
                    "run_id": run_id,
                    "action": "cancel_failed",
                    "error": e.to_string(),
                    "message": "Failed to cancel pipeline run"
                });

                Ok(vec![Content::Text {
                    text: serde_json::to_string_pretty(&error_response)?,
                }])
            }
        }
    }

    async fn list_runs(&self, args: Value) -> Result<Vec<Content>> {
        let mut cli_args = vec!["plm", "run", "list", "--output", "json"];
        
        // Add pipeline filter if provided
        let pipeline_filter = if let Some(name) = args.get("pipeline_name").and_then(|v| v.as_str()) {
            cli_args.extend_from_slice(&["--pipeline", name]);
            Some(format!("name: {}", name))
        } else if let Some(id) = args.get("pipeline_id").and_then(|v| v.as_str()) {
            cli_args.extend_from_slice(&["--pipeline", id]);
            Some(format!("id: {}", id))
        } else {
            None
        };

        match self.cli_manager.execute(&cli_args, None).await {
            Ok(result) => {
                let response = json!({
                    "success": true,
                    "data": result,
                    "pipeline_filter": pipeline_filter
                });

                Ok(vec![Content::Text {
                    text: serde_json::to_string_pretty(&response)?,
                }])
            }
            Err(e) => {
                error!("Failed to list runs: {}", e);
                let error_response = json!({
                    "success": false,
                    "error": e.to_string(),
                    "message": "Failed to retrieve pipeline runs"
                });

                Ok(vec![Content::Text {
                    text: serde_json::to_string_pretty(&error_response)?,
                }])
            }
        }
    }

    async fn get_run(&self, args: Value) -> Result<Vec<Content>> {
        let run_id = args.get("run_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| StudioError::InvalidOperation("run_id is required".to_string()))?;

        match self.cli_manager.execute(&["plm", "run", "get", run_id, "--output", "json"], None).await {
            Ok(result) => {
                let response = json!({
                    "success": true,
                    "run_id": run_id,
                    "data": result
                });

                Ok(vec![Content::Text {
                    text: serde_json::to_string_pretty(&response)?,
                }])
            }
            Err(e) => {
                error!("Failed to get run {}: {}", run_id, e);
                let error_response = json!({
                    "success": false,
                    "run_id": run_id,
                    "error": e.to_string(),
                    "message": "Failed to retrieve run information"
                });

                Ok(vec![Content::Text {
                    text: serde_json::to_string_pretty(&error_response)?,
                }])
            }
        }
    }

    async fn get_run_log(&self, args: Value) -> Result<Vec<Content>> {
        let run_id = args.get("run_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| StudioError::InvalidOperation("run_id is required".to_string()))?;

        match self.cli_manager.execute(&["plm", "run", "log", run_id, "--output", "json"], None).await {
            Ok(result) => {
                let response = json!({
                    "success": true,
                    "run_id": run_id,
                    "data": result
                });

                Ok(vec![Content::Text {
                    text: serde_json::to_string_pretty(&response)?,
                }])
            }
            Err(e) => {
                error!("Failed to get logs for run {}: {}", run_id, e);
                let error_response = json!({
                    "success": false,
                    "run_id": run_id,
                    "error": e.to_string(),
                    "message": "Failed to retrieve run logs"
                });

                Ok(vec![Content::Text {
                    text: serde_json::to_string_pretty(&error_response)?,
                }])
            }
        }
    }

    async fn get_run_events(&self, args: Value) -> Result<Vec<Content>> {
        let run_id = args.get("run_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| StudioError::InvalidOperation("run_id is required".to_string()))?;

        match self.cli_manager.execute(&["plm", "run", "events", run_id, "--output", "json"], None).await {
            Ok(result) => {
                let response = json!({
                    "success": true,
                    "run_id": run_id,
                    "data": result
                });

                Ok(vec![Content::Text {
                    text: serde_json::to_string_pretty(&response)?,
                }])
            }
            Err(e) => {
                error!("Failed to get events for run {}: {}", run_id, e);
                let error_response = json!({
                    "success": false,
                    "run_id": run_id,
                    "error": e.to_string(),
                    "message": "Failed to retrieve run events"
                });

                Ok(vec![Content::Text {
                    text: serde_json::to_string_pretty(&error_response)?,
                }])
            }
        }
    }

    async fn list_resources(&self, args: Value) -> Result<Vec<Content>> {
        let mut cli_args = vec!["plm", "resource", "list", "--output", "json"];
        
        // Add filters if provided
        let mut filters = json!({});
        
        if let Some(pipeline) = args.get("pipeline").and_then(|v| v.as_str()) {
            cli_args.extend_from_slice(&["--pipeline", pipeline]);
            filters["pipeline"] = json!(pipeline);
        }
        
        if let Some(access_config) = args.get("access_config").and_then(|v| v.as_str()) {
            cli_args.extend_from_slice(&["--access-config", access_config]);
            filters["access_config"] = json!(access_config);
        }

        match self.cli_manager.execute(&cli_args, None).await {
            Ok(result) => {
                let response = json!({
                    "success": true,
                    "data": result,
                    "filters": filters
                });

                Ok(vec![Content::Text {
                    text: serde_json::to_string_pretty(&response)?,
                }])
            }
            Err(e) => {
                error!("Failed to list resources: {}", e);
                let error_response = json!({
                    "success": false,
                    "error": e.to_string(),
                    "message": "Failed to retrieve pipeline resources"
                });

                Ok(vec![Content::Text {
                    text: serde_json::to_string_pretty(&error_response)?,
                }])
            }
        }
    }
}