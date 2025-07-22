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
            
            // ID resolution tool
            Tool {
                name: "plm_resolve_run_id".to_string(),
                description: "Convert pipeline name and run number to run ID".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "pipeline_name": {
                            "type": "string",
                            "description": "Name of the pipeline"
                        },
                        "pipeline_id": {
                            "type": "string",
                            "description": "ID of the pipeline (alternative to pipeline_name)"
                        },
                        "run_number": {
                            "type": "integer",
                            "description": "Run number within the pipeline (1 = latest, 2 = second latest, etc.)",
                            "minimum": 1
                        }
                    },
                    "anyOf": [
                        {"required": ["pipeline_name", "run_number"]},
                        {"required": ["pipeline_id", "run_number"]}
                    ]
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
                description: "Get detailed information about a specific pipeline run by ID or pipeline name/run number".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "run_id": {
                            "type": "string",
                            "description": "ID of the pipeline run to retrieve"
                        },
                        "pipeline_name": {
                            "type": "string",
                            "description": "Name of the pipeline (alternative to run_id)"
                        },
                        "pipeline_id": {
                            "type": "string",
                            "description": "ID of the pipeline (alternative to run_id)"
                        },
                        "run_number": {
                            "type": "integer",
                            "description": "Run number within the pipeline (1 = latest, 2 = second latest, etc.)",
                            "minimum": 1
                        }
                    },
                    "anyOf": [
                        {"required": ["run_id"]},
                        {"required": ["pipeline_name", "run_number"]},
                        {"required": ["pipeline_id", "run_number"]}
                    ]
                }),
            },
            Tool {
                name: "plm_get_run_log".to_string(),
                description: "Get logs for a specific pipeline run by ID or pipeline name/run number with optional filtering".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "run_id": {
                            "type": "string",
                            "description": "ID of the pipeline run to get logs for"
                        },
                        "pipeline_name": {
                            "type": "string",
                            "description": "Name of the pipeline (alternative to run_id)"
                        },
                        "pipeline_id": {
                            "type": "string",
                            "description": "ID of the pipeline (alternative to run_id)"
                        },
                        "run_number": {
                            "type": "integer",
                            "description": "Run number within the pipeline (1 = latest, 2 = second latest, etc.)",
                            "minimum": 1
                        },
                        "lines": {
                            "type": "integer",
                            "description": "Number of lines to retrieve (default: all)",
                            "minimum": 1
                        },
                        "tail": {
                            "type": "boolean",
                            "description": "Get logs from the end (tail mode)"
                        },
                        "errors_only": {
                            "type": "boolean",
                            "description": "Filter to show only error/warning lines"
                        },
                        "task_name": {
                            "type": "string",
                            "description": "Filter logs for specific task"
                        },
                        "since": {
                            "type": "string",
                            "description": "Show logs since timestamp (ISO format)"
                        }
                    },
                    "anyOf": [
                        {"required": ["run_id"]},
                        {"required": ["pipeline_name", "run_number"]},
                        {"required": ["pipeline_id", "run_number"]}
                    ]
                }),
            },
            Tool {
                name: "plm_get_pipeline_errors".to_string(),
                description: "Get error summary and recent errors for a pipeline".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "pipeline_name": {
                            "type": "string",
                            "description": "Name of the pipeline to analyze"
                        },
                        "pipeline_id": {
                            "type": "string",
                            "description": "ID of the pipeline to analyze"
                        },
                        "recent_runs": {
                            "type": "integer",
                            "description": "Number of recent runs to analyze (default: 5)",
                            "minimum": 1,
                            "maximum": 50
                        }
                    },
                    "required": []
                }),
            },
            Tool {
                name: "plm_get_task_errors".to_string(),
                description: "Get detailed error information for a specific pipeline task".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "run_id": {
                            "type": "string",
                            "description": "Pipeline run ID containing the task"
                        },
                        "task_name": {
                            "type": "string",
                            "description": "Name of the task to analyze"
                        },
                        "context_lines": {
                            "type": "integer",
                            "description": "Number of context lines around errors (default: 10)",
                            "minimum": 1,
                            "maximum": 100
                        }
                    },
                    "required": ["run_id", "task_name"]
                }),
            },
            Tool {
                name: "plm_get_run_events".to_string(),
                description: "Get events for a specific pipeline run by ID or pipeline name/run number".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "run_id": {
                            "type": "string",
                            "description": "ID of the pipeline run to get events for"
                        },
                        "pipeline_name": {
                            "type": "string",
                            "description": "Name of the pipeline (alternative to run_id)"
                        },
                        "pipeline_id": {
                            "type": "string",
                            "description": "ID of the pipeline (alternative to run_id)"
                        },
                        "run_number": {
                            "type": "integer",
                            "description": "Run number within the pipeline (1 = latest, 2 = second latest, etc.)",
                            "minimum": 1
                        }
                    },
                    "anyOf": [
                        {"required": ["run_id"]},
                        {"required": ["pipeline_name", "run_number"]},
                        {"required": ["pipeline_id", "run_number"]}
                    ]
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
            "plm_resolve_run_id" => self.resolve_run_id(args).await,
            "plm_list_runs" => self.list_runs(args).await,
            "plm_get_run" => self.get_run(args).await,
            "plm_get_run_log" => self.get_run_log(args).await,
            "plm_get_run_events" => self.get_run_events(args).await,
            "plm_list_resources" => self.list_resources(args).await,
            "plm_get_pipeline_errors" => self.get_pipeline_errors(args).await,
            "plm_get_task_errors" => self.get_task_errors(args).await,
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
        let run_id = self.resolve_run_id_from_args(&args).await?;

        match self.cli_manager.execute(&["plm", "run", "get", &run_id, "--output", "json"], None).await {
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
        let run_id = self.resolve_run_id_from_args(&args).await?;

        let mut cli_args = vec!["plm", "run", "log", &run_id, "--output", "json"];
        
        // Build CLI arguments based on filtering parameters
        let mut additional_args = Vec::new();
        
        if let Some(lines) = args.get("lines").and_then(|v| v.as_u64()) {
            additional_args.push("--lines".to_string());
            additional_args.push(lines.to_string());
        }
        
        if args.get("tail").and_then(|v| v.as_bool()).unwrap_or(false) {
            additional_args.push("--tail".to_string());
        }
        
        if let Some(task_name) = args.get("task_name").and_then(|v| v.as_str()) {
            additional_args.push("--task".to_string());
            additional_args.push(task_name.to_string());
        }
        
        if let Some(since) = args.get("since").and_then(|v| v.as_str()) {
            additional_args.push("--since".to_string());
            additional_args.push(since.to_string());
        }

        // Add additional args as string references
        for arg in &additional_args {
            cli_args.push(arg.as_str());
        }

        match self.cli_manager.execute(&cli_args, None).await {
            Ok(mut result) => {
                // Apply client-side error filtering if requested
                if args.get("errors_only").and_then(|v| v.as_bool()).unwrap_or(false) {
                    result = self.filter_error_logs(result);
                }

                let response = json!({
                    "success": true,
                    "run_id": run_id,
                    "data": result,
                    "filters_applied": {
                        "lines": args.get("lines"),
                        "tail": args.get("tail").and_then(|v| v.as_bool()).unwrap_or(false),
                        "errors_only": args.get("errors_only").and_then(|v| v.as_bool()).unwrap_or(false),
                        "task_name": args.get("task_name"),
                        "since": args.get("since")
                    }
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
        let run_id = self.resolve_run_id_from_args(&args).await?;

        match self.cli_manager.execute(&["plm", "run", "events", &run_id, "--output", "json"], None).await {
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

    async fn get_pipeline_errors(&self, args: Value) -> Result<Vec<Content>> {
        // Get pipeline identifier
        let pipeline_identifier = if let Some(name) = args.get("pipeline_name").and_then(|v| v.as_str()) {
            name
        } else if let Some(id) = args.get("pipeline_id").and_then(|v| v.as_str()) {
            id
        } else {
            return Err(StudioError::InvalidOperation("Either pipeline_name or pipeline_id is required".to_string()));
        };

        let recent_runs = args.get("recent_runs")
            .and_then(|v| v.as_u64())
            .unwrap_or(5) as usize;

        // Get recent runs for this pipeline
        let runs_result = match self.cli_manager.execute(&["plm", "run", "list", "--pipeline", pipeline_identifier, "--output", "json"], None).await {
            Ok(result) => result,
            Err(e) => {
                error!("Failed to get runs for pipeline {}: {}", pipeline_identifier, e);
                return Ok(vec![Content::Text {
                    text: serde_json::to_string_pretty(&json!({
                        "success": false,
                        "pipeline": pipeline_identifier,
                        "error": e.to_string(),
                        "message": "Failed to retrieve pipeline runs"
                    }))?,
                }]);
            }
        };

        // Extract run IDs and analyze errors
        let mut error_summary = json!({
            "pipeline": pipeline_identifier,
            "analyzed_runs": 0,
            "total_errors": 0,
            "error_patterns": {},
            "recent_errors": []
        });

        if let Some(runs) = runs_result.as_array() {
            let limited_runs: Vec<_> = runs.iter().take(recent_runs).collect();
            error_summary["analyzed_runs"] = json!(limited_runs.len());

            for run in limited_runs {
                if let Some(run_id) = run.get("id").and_then(|v| v.as_str()) {
                    // Get logs for this run and analyze errors
                    if let Ok(log_result) = self.cli_manager.execute(&["plm", "run", "log", run_id, "--output", "json"], None).await {
                        let filtered_errors = self.filter_error_logs(log_result);
                        
                        // Count and categorize errors (simplified implementation)
                        if let Some(log_text) = filtered_errors.as_str() {
                            let error_count = log_text.lines()
                                .filter(|line| line.to_lowercase().contains("error") || line.to_lowercase().contains("fail"))
                                .count();
                            
                            error_summary["total_errors"] = json!(
                                error_summary["total_errors"].as_u64().unwrap_or(0) + error_count as u64
                            );

                            if error_count > 0 {
                                let recent_errors = error_summary["recent_errors"].as_array_mut().unwrap();
                                recent_errors.push(json!({
                                    "run_id": run_id,
                                    "error_count": error_count,
                                    "timestamp": run.get("created_at").unwrap_or(&json!("unknown"))
                                }));
                            }
                        }
                    }
                }
            }
        }

        Ok(vec![Content::Text {
            text: serde_json::to_string_pretty(&json!({
                "success": true,
                "data": error_summary
            }))?,
        }])
    }

    async fn get_task_errors(&self, args: Value) -> Result<Vec<Content>> {
        let run_id = args.get("run_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| StudioError::InvalidOperation("run_id is required".to_string()))?;

        let task_name = args.get("task_name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| StudioError::InvalidOperation("task_name is required".to_string()))?;

        let context_lines = args.get("context_lines")
            .and_then(|v| v.as_u64())
            .unwrap_or(10);

        // Get logs for the specific task
        let mut cli_args = vec!["plm", "run", "log", run_id, "--task", task_name, "--output", "json"];
        
        // Add context lines if the CLI supports it
        let context_str = context_lines.to_string();
        cli_args.extend_from_slice(&["--lines", &context_str]);

        match self.cli_manager.execute(&cli_args, None).await {
            Ok(result) => {
                // Filter for error lines and add context
                let error_analysis = self.analyze_task_errors(result, context_lines as usize);

                let response = json!({
                    "success": true,
                    "run_id": run_id,
                    "task_name": task_name,
                    "context_lines": context_lines,
                    "data": error_analysis
                });

                Ok(vec![Content::Text {
                    text: serde_json::to_string_pretty(&response)?,
                }])
            }
            Err(e) => {
                error!("Failed to get task errors for run {} task {}: {}", run_id, task_name, e);
                let error_response = json!({
                    "success": false,
                    "run_id": run_id,
                    "task_name": task_name,
                    "error": e.to_string(),
                    "message": "Failed to retrieve task error information"
                });

                Ok(vec![Content::Text {
                    text: serde_json::to_string_pretty(&error_response)?,
                }])
            }
        }
    }

    // Helper methods for error filtering and analysis
    fn filter_error_logs(&self, logs: Value) -> Value {
        if let Some(log_str) = logs.as_str() {
            let error_lines: Vec<&str> = log_str
                .lines()
                .filter(|line| {
                    let lower = line.to_lowercase();
                    lower.contains("error") || 
                    lower.contains("fail") || 
                    lower.contains("exception") ||
                    lower.contains("panic") ||
                    lower.contains("fatal") ||
                    lower.contains("warn")
                })
                .collect();
            
            json!(error_lines.join("\n"))
        } else {
            logs
        }
    }

    fn analyze_task_errors(&self, logs: Value, context_lines: usize) -> Value {
        if let Some(log_str) = logs.as_str() {
            let lines: Vec<&str> = log_str.lines().collect();
            let mut error_blocks = Vec::new();

            for (i, line) in lines.iter().enumerate() {
                let lower = line.to_lowercase();
                if lower.contains("error") || lower.contains("fail") || lower.contains("exception") {
                    // Get context around error
                    let start = i.saturating_sub(context_lines);
                    let end = std::cmp::min(i + context_lines + 1, lines.len());
                    
                    let context_block: Vec<String> = lines[start..end]
                        .iter()
                        .enumerate()
                        .map(|(idx, l)| {
                            let line_num = start + idx;
                            if line_num == i {
                                format!(">>> {} ERROR: {}", line_num, l) // Mark error line
                            } else {
                                format!("    {} {}", line_num, l)
                            }
                        })
                        .collect();

                    error_blocks.push(json!({
                        "error_line": i,
                        "error_text": line,
                        "context": context_block.join("\n")
                    }));
                }
            }

            json!({
                "total_errors": error_blocks.len(),
                "error_blocks": error_blocks,
                "analysis": {
                    "common_patterns": self.extract_error_patterns(&error_blocks),
                    "severity": if error_blocks.len() > 5 { "high" } else if error_blocks.len() > 2 { "medium" } else { "low" }
                }
            })
        } else {
            json!({
                "total_errors": 0,
                "error_blocks": [],
                "message": "No text logs available for analysis"
            })
        }
    }

    fn extract_error_patterns(&self, error_blocks: &[Value]) -> Value {
        let mut patterns = std::collections::HashMap::new();
        
        for block in error_blocks {
            if let Some(error_text) = block.get("error_text").and_then(|v| v.as_str()) {
                let lower = error_text.to_lowercase();
                
                // Simple pattern matching
                if lower.contains("connection") || lower.contains("network") {
                    *patterns.entry("network_errors").or_insert(0) += 1;
                } else if lower.contains("permission") || lower.contains("access") {
                    *patterns.entry("permission_errors").or_insert(0) += 1;
                } else if lower.contains("timeout") {
                    *patterns.entry("timeout_errors").or_insert(0) += 1;
                } else if lower.contains("not found") || lower.contains("missing") {
                    *patterns.entry("missing_resource_errors").or_insert(0) += 1;
                } else {
                    *patterns.entry("other_errors").or_insert(0) += 1;
                }
            }
        }
        
        json!(patterns)
    }

    /// Resolve run ID from pipeline name/ID and run number
    async fn resolve_run_id(&self, args: Value) -> Result<Vec<Content>> {
        let pipeline_filter = if let Some(name) = args.get("pipeline_name").and_then(|v| v.as_str()) {
            name.to_string()
        } else if let Some(id) = args.get("pipeline_id").and_then(|v| v.as_str()) {
            id.to_string()
        } else {
            return Err(StudioError::InvalidOperation("Either pipeline_name or pipeline_id is required".to_string()));
        };

        let run_number = args.get("run_number")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| StudioError::InvalidOperation("run_number is required".to_string()))? as usize;

        // Get runs for the pipeline
        let cli_args = vec!["plm", "run", "list", "--pipeline", &pipeline_filter, "--output", "json"];
        
        match self.cli_manager.execute(&cli_args, None).await {
            Ok(result) => {
                if let Some(runs) = result.as_array() {
                    if run_number == 0 || run_number > runs.len() {
                        let error_response = json!({
                            "success": false,
                            "error": format!("Run number {} is out of range (1-{})", run_number, runs.len()),
                            "pipeline": pipeline_filter,
                            "available_runs": runs.len()
                        });
                        return Ok(vec![Content::Text {
                            text: serde_json::to_string_pretty(&error_response)?,
                        }]);
                    }

                    // Get the run by index (run_number 1 = index 0 = latest)
                    let run = &runs[run_number - 1];
                    let run_id = run.get("id")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| StudioError::InvalidOperation("Run ID not found in response".to_string()))?;

                    let response = json!({
                        "success": true,
                        "run_id": run_id,
                        "pipeline": pipeline_filter,
                        "run_number": run_number,
                        "run_details": run
                    });

                    Ok(vec![Content::Text {
                        text: serde_json::to_string_pretty(&response)?,
                    }])
                } else {
                    let error_response = json!({
                        "success": false,
                        "error": "Invalid response format from CLI",
                        "pipeline": pipeline_filter
                    });
                    Ok(vec![Content::Text {
                        text: serde_json::to_string_pretty(&error_response)?,
                    }])
                }
            }
            Err(e) => {
                error!("Failed to list runs for pipeline {}: {}", pipeline_filter, e);
                let error_response = json!({
                    "success": false,
                    "pipeline": pipeline_filter,
                    "error": e.to_string(),
                    "message": "Failed to retrieve runs for pipeline"
                });

                Ok(vec![Content::Text {
                    text: serde_json::to_string_pretty(&error_response)?,
                }])
            }
        }
    }

    /// Helper to resolve run ID from various input formats
    async fn resolve_run_id_from_args(&self, args: &Value) -> Result<String> {
        // If run_id is provided directly, use it
        if let Some(run_id) = args.get("run_id").and_then(|v| v.as_str()) {
            return Ok(run_id.to_string());
        }

        // Otherwise, resolve from pipeline name/ID and run number
        let pipeline_filter = if let Some(name) = args.get("pipeline_name").and_then(|v| v.as_str()) {
            name.to_string()
        } else if let Some(id) = args.get("pipeline_id").and_then(|v| v.as_str()) {
            id.to_string()
        } else {
            return Err(StudioError::InvalidOperation("Either run_id or (pipeline_name/pipeline_id + run_number) is required".to_string()));
        };

        let run_number = args.get("run_number")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| StudioError::InvalidOperation("run_number is required when not using run_id".to_string()))? as usize;

        // Get runs for the pipeline
        let cli_args = vec!["plm", "run", "list", "--pipeline", &pipeline_filter, "--output", "json"];
        
        let result = self.cli_manager.execute(&cli_args, None).await?;
        
        if let Some(runs) = result.as_array() {
            if run_number == 0 || run_number > runs.len() {
                return Err(StudioError::InvalidOperation(format!(
                    "Run number {} is out of range (1-{})", 
                    run_number, 
                    runs.len()
                )));
            }

            // Get the run by index (run_number 1 = index 0 = latest)
            let run = &runs[run_number - 1];
            let run_id = run.get("id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| StudioError::InvalidOperation("Run ID not found in response".to_string()))?;

            Ok(run_id.to_string())
        } else {
            Err(StudioError::InvalidOperation("Invalid response format from CLI".to_string()))
        }
    }
}