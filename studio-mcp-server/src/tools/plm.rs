//! PLM (Pipeline Management) tool provider

use pulseengine_mcp_protocol::{Content, Tool};
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Duration;
use studio_cli_manager::CliManager;
use studio_mcp_shared::{OperationType, Result, StudioConfig, StudioError};
use tracing::{debug, error};

pub struct PlmToolProvider {
    cli_manager: Arc<CliManager>,
    #[allow(dead_code)]
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
                        "pipeline_id": {
                            "type": "string",
                            "description": "Filter by specific pipeline ID"
                        },
                        "created_by": {
                            "type": "string",
                            "description": "Filter by user who created the pipeline"
                        },
                        "modified_by": {
                            "type": "string",
                            "description": "Filter by user who last modified the pipeline"
                        },
                        "include_tasks": {
                            "type": "boolean",
                            "description": "Include task definitions in pipeline list"
                        },
                        "is_archived": {
                            "type": "boolean",
                            "description": "Show/hide archived pipelines"
                        },
                        "is_template": {
                            "type": "boolean",
                            "description": "Show/hide pipeline templates"
                        },
                        "sort_column": {
                            "type": "string",
                            "description": "Column to sort by (name, created_at, modified_by, etc.)"
                        },
                        "sort_direction": {
                            "type": "string",
                            "description": "Sort direction",
                            "enum": ["ASC", "DESC", "ascending", "descending"]
                        },
                        "limit": {
                            "type": "integer",
                            "description": "Limit number of results (default 10, 0 for all)",
                            "minimum": 0
                        },
                        "page_size": {
                            "type": "integer",
                            "description": "Page size for pagination (alternative to limit)",
                            "minimum": 1
                        },
                        "page_number": {
                            "type": "integer",
                            "description": "Page number for pagination",
                            "minimum": 1
                        },
                        "offset": {
                            "type": "integer",
                            "description": "Starting offset for results (default 1)",
                            "minimum": 1
                        }
                    },
                    "required": []
                }),
                output_schema: Some(json!({
                    "type": "object",
                    "properties": {
                        "pipelines": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "id": {"type": "string"},
                                    "name": {"type": "string"},
                                    "status": {"type": "string"},
                                    "created_by": {"type": "string"},
                                    "created_at": {"type": "string"}
                                }
                            }
                        },
                        "total": {"type": "integer"},
                        "offset": {"type": "integer"},
                        "limit": {"type": "integer"}
                    }
                })),
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
                output_schema: Some(json!({
                    "type": "object",
                    "properties": {
                        "success": {"type": "boolean"},
                        "pipeline_id": {"type": "string"},
                        "format": {"type": "string"},
                        "data": {"type": "string", "description": "Pipeline definition in YAML/JSON format"},
                        "error": {"type": "string"},
                        "message": {"type": "string"}
                    },
                    "required": ["success"]
                })),
            },
            Tool {
                name: "plm_start_pipeline".to_string(),
                description: "Start execution of a pipeline with optional parameters. Either pipeline_name or pipeline_id is required.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "pipeline_name": {
                            "type": "string",
                            "description": "Name of the pipeline to run (mutually exclusive with pipeline_id)"
                        },
                        "pipeline_id": {
                            "type": "string",
                            "description": "ID of the pipeline to run (mutually exclusive with pipeline_name)"
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
                            "description": "Stream logs until completion (uses extended timeout)"
                        }
                    },
                    "anyOf": [
                        {"required": ["pipeline_name"]},
                        {"required": ["pipeline_id"]}
                    ]
                }),
                output_schema: Some(json!({
                    "type": "object",
                    "properties": {
                        "success": {"type": "boolean"},
                        "pipeline": {"type": "string"},
                        "action": {"type": "string"},
                        "data": {"type": "object", "description": "Pipeline execution result"},
                        "parameters": {"type": "array"},
                        "config": {"type": "array"},
                        "env": {"type": "array"},
                        "error": {"type": "string"},
                        "message": {"type": "string"}
                    },
                    "required": ["success", "action"]
                })),
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
                output_schema: Some(json!({
                    "type": "object",
                    "properties": {
                        "success": {"type": "boolean"},
                        "run_id": {"type": "string"},
                        "action": {"type": "string"},
                        "data": {"type": "object", "description": "Cancellation result"},
                        "error": {"type": "string"},
                        "message": {"type": "string"}
                    },
                    "required": ["success", "run_id", "action"]
                })),
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
                output_schema: Some(json!({
                    "type": "object",
                    "properties": {
                        "success": {"type": "boolean"},
                        "run_id": {"type": "string"},
                        "pipeline": {"type": "string"},
                        "run_number": {"type": "integer"},
                        "run_details": {"type": "object"},
                        "error": {"type": "string"},
                        "available_runs": {"type": "integer"},
                        "message": {"type": "string"}
                    },
                    "required": ["success"]
                })),
            },

            // Pipeline run management tools
            Tool {
                name: "plm_list_runs".to_string(),
                description: "List pipeline runs with comprehensive filtering options".to_string(),
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
                        },
                        "run_number": {
                            "type": "integer",
                            "description": "Filter by specific run number",
                            "minimum": 1
                        },
                        "status": {
                            "type": "string",
                            "description": "Filter by run status (running, completed, failed, etc.)"
                        },
                        "created_by": {
                            "type": "string",
                            "description": "Filter by user who created the run"
                        },
                        "start_time": {
                            "type": "string",
                            "description": "Filter runs started after this timestamp (ISO 8601 format)"
                        },
                        "end_time": {
                            "type": "string",
                            "description": "Filter runs started before this timestamp (ISO 8601 format)"
                        },
                        "from_failure": {
                            "type": "boolean",
                            "description": "Show runs from failure point"
                        },
                        "compile_only": {
                            "type": "boolean",
                            "description": "Show only compile-only runs"
                        },
                        "sort_column": {
                            "type": "string",
                            "description": "Column to sort by (start_time, status, pipeline_name, etc.)"
                        },
                        "sort_direction": {
                            "type": "string",
                            "description": "Sort direction",
                            "enum": ["ASC", "DESC", "ascending", "descending"]
                        },
                        "limit": {
                            "type": "integer",
                            "description": "Limit number of results",
                            "minimum": 1
                        },
                        "offset": {
                            "type": "integer",
                            "description": "Starting offset for results",
                            "minimum": 0
                        }
                    },
                    "required": []
                }),
                output_schema: Some(json!({
                    "type": "object",
                    "properties": {
                        "success": {"type": "boolean"},
                        "data": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "id": {"type": "string"},
                                    "pipeline_id": {"type": "string"},
                                    "status": {"type": "string"},
                                    "created_at": {"type": "string"}
                                }
                            }
                        },
                        "pipeline_filter": {"type": "string"},
                        "error": {"type": "string"},
                        "message": {"type": "string"}
                    },
                    "required": ["success"]
                })),
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
                        },
                        "run_config": {
                            "type": "boolean",
                            "description": "Include run configuration details"
                        },
                        "detailed_info": {
                            "type": "boolean",
                            "description": "Include detailed run information"
                        },
                        "include_tasks": {
                            "type": "boolean",
                            "description": "Include task definitions and details"
                        },
                        "execution_logs": {
                            "type": "boolean",
                            "description": "Include execution logs in the response"
                        }
                    },
                    "anyOf": [
                        {"required": ["run_id"]},
                        {"required": ["pipeline_name", "run_number"]},
                        {"required": ["pipeline_id", "run_number"]}
                    ]
                }),
                output_schema: Some(json!({
                    "type": "object",
                    "properties": {
                        "success": {"type": "boolean"},
                        "run_id": {"type": "string"},
                        "data": {
                            "type": "object",
                            "properties": {
                                "id": {"type": "string"},
                                "pipeline_id": {"type": "string"},
                                "status": {"type": "string"},
                                "created_at": {"type": "string"},
                                "updated_at": {"type": "string"},
                                "tasks": {"type": "array"}
                            }
                        },
                        "error": {"type": "string"},
                        "message": {"type": "string"}
                    },
                    "required": ["success"]
                })),
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
                        },
                        "query_since": {
                            "type": "string", 
                            "description": "Query logs since timestamp (more precise than since)"
                        },
                        "query_until": {
                            "type": "string",
                            "description": "Query logs until timestamp"
                        },
                        "log_type": {
                            "type": "string",
                            "description": "Filter by log type (error, warning, info, debug)"
                        },
                        "sort_column": {
                            "type": "string",
                            "description": "Sort logs by column (timestamp, level, task, etc.)"
                        },
                        "raw_field": {
                            "type": "boolean",
                            "description": "Return raw log fields without formatting"
                        }
                    },
                    "anyOf": [
                        {"required": ["run_id"]},
                        {"required": ["pipeline_name", "run_number"]},
                        {"required": ["pipeline_id", "run_number"]}
                    ]
                }),
                output_schema: Some(json!({
                    "type": "object",
                    "properties": {
                        "success": {"type": "boolean"},
                        "run_id": {"type": "string"},
                        "data": {"type": "string", "description": "Log content"},
                        "filters_applied": {
                            "type": "object",
                            "properties": {
                                "lines": {"type": "integer"},
                                "tail": {"type": "boolean"},
                                "errors_only": {"type": "boolean"},
                                "task_name": {"type": "string"},
                                "since": {"type": "string"}
                            }
                        },
                        "error": {"type": "string"},
                        "message": {"type": "string"}
                    },
                    "required": ["success"]
                })),
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
                output_schema: Some(json!({
                    "type": "object",
                    "properties": {
                        "success": {"type": "boolean"},
                        "data": {
                            "type": "object",
                            "properties": {
                                "pipeline": {"type": "string"},
                                "analyzed_runs": {"type": "integer"},
                                "total_errors": {"type": "integer"},
                                "error_patterns": {"type": "object"},
                                "recent_errors": {
                                    "type": "array",
                                    "items": {
                                        "type": "object",
                                        "properties": {
                                            "run_id": {"type": "string"},
                                            "error_count": {"type": "integer"},
                                            "timestamp": {"type": "string"}
                                        }
                                    }
                                }
                            }
                        },
                        "pipeline": {"type": "string"},
                        "error": {"type": "string"},
                        "message": {"type": "string"}
                    },
                    "required": ["success"]
                })),
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
                output_schema: Some(json!({
                    "type": "object",
                    "properties": {
                        "success": {"type": "boolean"},
                        "run_id": {"type": "string"},
                        "task_name": {"type": "string"},
                        "context_lines": {"type": "integer"},
                        "data": {
                            "type": "object",
                            "properties": {
                                "total_errors": {"type": "integer"},
                                "error_blocks": {
                                    "type": "array",
                                    "items": {
                                        "type": "object",
                                        "properties": {
                                            "error_line": {"type": "integer"},
                                            "error_text": {"type": "string"},
                                            "context": {"type": "string"}
                                        }
                                    }
                                },
                                "analysis": {
                                    "type": "object",
                                    "properties": {
                                        "common_patterns": {"type": "object"},
                                        "severity": {"type": "string"}
                                    }
                                }
                            }
                        },
                        "error": {"type": "string"},
                        "message": {"type": "string"}
                    },
                    "required": ["success"]
                })),
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
                output_schema: Some(json!({
                    "type": "object",
                    "properties": {
                        "success": {"type": "boolean"},
                        "run_id": {"type": "string"},
                        "data": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "event_id": {"type": "string"},
                                    "event_type": {"type": "string"},
                                    "timestamp": {"type": "string"},
                                    "task_name": {"type": "string"},
                                    "message": {"type": "string"},
                                    "data": {"type": "object"}
                                }
                            }
                        },
                        "error": {"type": "string"},
                        "message": {"type": "string"}
                    },
                    "required": ["success"]
                })),
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
                output_schema: Some(json!({
                    "type": "object",
                    "properties": {
                        "success": {"type": "boolean"},
                        "data": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "id": {"type": "string"},
                                    "name": {"type": "string"},
                                    "type": {"type": "string"},
                                    "pipeline_id": {"type": "string"},
                                    "access_config": {"type": "string"},
                                    "status": {"type": "string"}
                                }
                            }
                        },
                        "filters": {"type": "object"},
                        "error": {"type": "string"},
                        "message": {"type": "string"}
                    },
                    "required": ["success"]
                })),
            },

            // Task management tools
            Tool {
                name: "plm_create_task".to_string(),
                description: "Create a new task from YAML/JSON definition or parameters".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "task_definition": {
                            "type": "string",
                            "description": "Task definition in YAML or JSON format"
                        },
                        "definition_file": {
                            "type": "string", 
                            "description": "Path to YAML/JSON file containing task definition"
                        },
                        "name": {
                            "type": "string",
                            "description": "Name of the task (alternative to task_definition)"
                        },
                        "category": {
                            "type": "string",
                            "description": "Task category (required with name)"
                        },
                        "task_lib": {
                            "type": "string",
                            "description": "Task library (required with name)"
                        },
                        "version": {
                            "type": "string",
                            "description": "Task version (optional)"
                        }
                    },
                    "anyOf": [
                        {"required": ["task_definition"]},
                        {"required": ["definition_file"]},
                        {"required": ["name", "category", "task_lib"]}
                    ]
                }),
                output_schema: Some(json!({
                    "type": "object",
                    "properties": {
                        "success": {"type": "boolean"},
                        "task_name": {"type": "string"},
                        "action": {"type": "string"},
                        "data": {"type": "object"},
                        "error": {"type": "string"},
                        "message": {"type": "string"}
                    },
                    "required": ["success"]
                })),
            },
            Tool {
                name: "plm_update_task".to_string(),
                description: "Update an existing task with new definition".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "task_name": {
                            "type": "string",
                            "description": "Name of the task to update"
                        },
                        "task_definition": {
                            "type": "string",
                            "description": "Updated task definition in YAML or JSON format"
                        },
                        "definition_file": {
                            "type": "string",
                            "description": "Path to YAML/JSON file containing updated task definition"
                        }
                    },
                    "required": ["task_name"],
                    "anyOf": [
                        {"required": ["task_name", "task_definition"]},
                        {"required": ["task_name", "definition_file"]}
                    ]
                }),
                output_schema: Some(json!({
                    "type": "object",
                    "properties": {
                        "success": {"type": "boolean"},
                        "task_name": {"type": "string"},
                        "action": {"type": "string"},
                        "data": {"type": "object"},
                        "error": {"type": "string"},
                        "message": {"type": "string"}
                    },
                    "required": ["success"]
                })),
            },
            Tool {
                name: "plm_delete_task".to_string(),
                description: "Delete a task by name".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "task_name": {
                            "type": "string",
                            "description": "Name of the task to delete"
                        }
                    },
                    "required": ["task_name"]
                }),
                output_schema: Some(json!({
                    "type": "object",
                    "properties": {
                        "success": {"type": "boolean"},
                        "task_name": {"type": "string"},
                        "action": {"type": "string"},
                        "message": {"type": "string"},
                        "error": {"type": "string"}
                    },
                    "required": ["success"]
                })),
            },
            Tool {
                name: "plm_rename_task".to_string(),
                description: "Rename a task from old name to new name".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "old_task_name": {
                            "type": "string",
                            "description": "Current name of the task"
                        },
                        "new_task_name": {
                            "type": "string",
                            "description": "New name for the task"
                        }
                    },
                    "required": ["old_task_name", "new_task_name"]
                }),
                output_schema: Some(json!({
                    "type": "object",
                    "properties": {
                        "success": {"type": "boolean"},
                        "old_task_name": {"type": "string"},
                        "new_task_name": {"type": "string"},
                        "action": {"type": "string"},
                        "message": {"type": "string"},
                        "error": {"type": "string"}
                    },
                    "required": ["success"]
                })),
            },
            Tool {
                name: "plm_list_tasks".to_string(),
                description: "List all available tasks with optional filtering".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "category": {
                            "type": "string",
                            "description": "Filter tasks by category"
                        },
                        "task_lib": {
                            "type": "string",
                            "description": "Filter tasks by task library"
                        },
                        "include_tasks": {
                            "type": "boolean",
                            "description": "Include detailed task definitions"
                        },
                        "limit": {
                            "type": "integer",
                            "description": "Limit number of results",
                            "minimum": 1
                        },
                        "offset": {
                            "type": "integer",
                            "description": "Starting offset for results",
                            "minimum": 0
                        }
                    },
                    "required": []
                }),
                output_schema: Some(json!({
                    "type": "object",
                    "properties": {
                        "success": {"type": "boolean"},
                        "data": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "name": {"type": "string"},
                                    "category": {"type": "string"},
                                    "task_lib": {"type": "string"},
                                    "version": {"type": "string"},
                                    "definition": {"type": "object"}
                                }
                            }
                        },
                        "filters": {"type": "object"},
                        "error": {"type": "string"},
                        "message": {"type": "string"}
                    },
                    "required": ["success"]
                })),
            },
            Tool {
                name: "plm_get_task".to_string(),
                description: "Get detailed information about a specific task".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "task_name": {
                            "type": "string",
                            "description": "Name of the task to retrieve"
                        },
                        "category": {
                            "type": "string",
                            "description": "Task category (alternative identifier)"
                        },
                        "version": {
                            "type": "string",
                            "description": "Specific version to retrieve"
                        }
                    },
                    "anyOf": [
                        {"required": ["task_name"]},
                        {"required": ["category", "task_name"]}
                    ]
                }),
                output_schema: Some(json!({
                    "type": "object",
                    "properties": {
                        "success": {"type": "boolean"},
                        "task_name": {"type": "string"},
                        "data": {
                            "type": "object",
                            "properties": {
                                "name": {"type": "string"},
                                "category": {"type": "string"},
                                "task_lib": {"type": "string"},
                                "version": {"type": "string"},
                                "definition": {"type": "object"},
                                "dependencies": {"type": "array"}
                            }
                        },
                        "error": {"type": "string"},
                        "message": {"type": "string"}
                    },
                    "required": ["success"]
                })),
            },
            Tool {
                name: "plm_unlock_task".to_string(),
                description: "Unlock a task that may be locked by another process".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "task_name": {
                            "type": "string",
                            "description": "Name of the task to unlock"
                        }
                    },
                    "required": ["task_name"]
                }),
                output_schema: Some(json!({
                    "type": "object",
                    "properties": {
                        "success": {"type": "boolean"},
                        "task_name": {"type": "string"},
                        "action": {"type": "string"},
                        "message": {"type": "string"},
                        "error": {"type": "string"}
                    },
                    "required": ["success"]
                })),
            },
            Tool {
                name: "plm_rename_param".to_string(),
                description: "Rename a pipeline parameter by specifying the old name and new name".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "pipeline_name": {
                            "type": "string",
                            "description": "Name of the pipeline containing the parameter to rename"
                        },
                        "old_param_name": {
                            "type": "string",
                            "description": "Current name of the parameter to rename"
                        },
                        "new_param_name": {
                            "type": "string",
                            "description": "New name for the parameter"
                        },
                        "file": {
                            "type": "string",
                            "description": "Path to pipeline YAML/JSON file (alternative to pipeline name)"
                        }
                    },
                    "anyOf": [
                        {"required": ["pipeline_name", "old_param_name", "new_param_name"]},
                        {"required": ["file", "old_param_name", "new_param_name"]}
                    ]
                }),
                output_schema: Some(json!({
                    "type": "object",
                    "properties": {
                        "success": {"type": "boolean"},
                        "pipeline_name": {"type": "string"},
                        "old_param_name": {"type": "string"},
                        "new_param_name": {"type": "string"},
                        "action": {"type": "string"},
                        "data": {"type": "object"},
                        "message": {"type": "string"},
                        "error": {"type": "string"}
                    },
                    "required": ["success"]
                })),
            },
            Tool {
                name: "plm_create_access_config".to_string(),
                description: "Create a new pipeline access configuration with optional user credentials".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "name": {
                            "type": "string",
                            "description": "Name of the access configuration"
                        },
                        "username": {
                            "type": "string",
                            "description": "Username of access user (optional, creates bot if not provided)"
                        },
                        "password": {
                            "type": "string",
                            "description": "Password of access user (optional)"
                        },
                        "group": {
                            "type": "string",
                            "description": "Group name or ID for the access config"
                        },
                        "create_ssh": {
                            "type": "boolean",
                            "description": "Enable SSH key creation (default: true)",
                            "default": true
                        }
                    },
                    "required": ["name"]
                }),
                output_schema: Some(json!({
                    "type": "object",
                    "properties": {
                        "success": {"type": "boolean"},
                        "name": {"type": "string"},
                        "action": {"type": "string"},
                        "data": {"type": "object"},
                        "message": {"type": "string"},
                        "error": {"type": "string"}
                    },
                    "required": ["success"]
                })),
            },
            Tool {
                name: "plm_list_access_configs".to_string(),
                description: "List all pipeline access configurations".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {},
                    "additionalProperties": false
                }),
                output_schema: Some(json!({
                    "type": "object",
                    "properties": {
                        "success": {"type": "boolean"},
                        "data": {"type": "array"},
                        "total": {"type": "number"},
                        "message": {"type": "string"},
                        "error": {"type": "string"}
                    },
                    "required": ["success"]
                })),
            },
            Tool {
                name: "plm_get_access_config".to_string(),
                description: "Get detailed information about a specific access configuration".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "name": {
                            "type": "string",
                            "description": "Name of the access configuration"
                        }
                    },
                    "required": ["name"]
                }),
                output_schema: Some(json!({
                    "type": "object",
                    "properties": {
                        "success": {"type": "boolean"},
                        "name": {"type": "string"},
                        "data": {"type": "object"},
                        "message": {"type": "string"},
                        "error": {"type": "string"}
                    },
                    "required": ["success"]
                })),
            },
            Tool {
                name: "plm_delete_access_config".to_string(),
                description: "Delete a pipeline access configuration".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "name": {
                            "type": "string",
                            "description": "Name of the access configuration to delete"
                        }
                    },
                    "required": ["name"]
                }),
                output_schema: Some(json!({
                    "type": "object",
                    "properties": {
                        "success": {"type": "boolean"},
                        "name": {"type": "string"},
                        "action": {"type": "string"},
                        "data": {"type": "object"},
                        "message": {"type": "string"},
                        "error": {"type": "string"}
                    },
                    "required": ["success"]
                })),
            },
        ];

        debug!("PLM provider listed {} tools", tools.len());
        Ok(tools)
    }

    pub async fn call_tool(&self, name: &str, arguments: Option<Value>) -> Result<Vec<Content>> {
        debug!(
            "PLM provider calling tool: {} with args: {:?}",
            name, arguments
        );

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
            "plm_create_task" => self.create_task(args).await,
            "plm_update_task" => self.update_task(args).await,
            "plm_delete_task" => self.delete_task(args).await,
            "plm_rename_task" => self.rename_task(args).await,
            "plm_list_tasks" => self.list_tasks(args).await,
            "plm_get_task" => self.get_task(args).await,
            "plm_unlock_task" => self.unlock_task(args).await,
            "plm_rename_param" => self.rename_param(args).await,
            "plm_create_access_config" => self.create_access_config(args).await,
            "plm_list_access_configs" => self.list_access_configs(args).await,
            "plm_get_access_config" => self.get_access_config(args).await,
            "plm_delete_access_config" => self.delete_access_config(args).await,
            _ => {
                error!("Unknown PLM tool: {}", name);
                Err(StudioError::InvalidOperation(format!(
                    "PLM tool '{name}' not found"
                )))
            }
        }
    }

    async fn list_pipelines(&self, args: Value) -> Result<Vec<Content>> {
        let mut cli_args = vec!["plm", "pipeline", "list", "--output", "json"];

        // Add optional filters
        let mut filters = json!({});

        if let Some(name) = args.get("name").and_then(|v| v.as_str()) {
            cli_args.extend_from_slice(&["--name", name]);
            filters["name"] = json!(name);
        }

        if let Some(pipeline_id) = args.get("pipeline_id").and_then(|v| v.as_str()) {
            cli_args.extend_from_slice(&["--id", pipeline_id]);
            filters["pipeline_id"] = json!(pipeline_id);
        }

        if let Some(created_by) = args.get("created_by").and_then(|v| v.as_str()) {
            cli_args.extend_from_slice(&["--created-by", created_by]);
            filters["created_by"] = json!(created_by);
        }

        if let Some(modified_by) = args.get("modified_by").and_then(|v| v.as_str()) {
            cli_args.extend_from_slice(&["--modified-by", modified_by]);
            filters["modified_by"] = json!(modified_by);
        }

        if let Some(include_tasks) = args.get("include_tasks").and_then(|v| v.as_bool()) {
            if include_tasks {
                cli_args.push("--include-tasks");
            }
            filters["include_tasks"] = json!(include_tasks);
        }

        if let Some(is_archived) = args.get("is_archived").and_then(|v| v.as_bool()) {
            if is_archived {
                cli_args.push("--is-archived");
            }
            filters["is_archived"] = json!(is_archived);
        }

        if let Some(is_template) = args.get("is_template").and_then(|v| v.as_bool()) {
            if is_template {
                cli_args.push("--is-template");
            }
            filters["is_template"] = json!(is_template);
        }

        if let Some(sort_column) = args.get("sort_column").and_then(|v| v.as_str()) {
            cli_args.extend_from_slice(&["--sort-column", sort_column]);
            filters["sort_column"] = json!(sort_column);
        }

        if let Some(sort_direction) = args.get("sort_direction").and_then(|v| v.as_str()) {
            cli_args.extend_from_slice(&["--sort-direction", sort_direction]);
            filters["sort_direction"] = json!(sort_direction);
        }

        // Handle pagination - prefer page_size/page_number over limit/offset
        let page_size_str;
        let limit_str;
        let page_number_str;
        let offset_str;

        if let Some(page_size) = args.get("page_size").and_then(|v| v.as_u64()) {
            page_size_str = page_size.to_string();
            cli_args.extend_from_slice(&["--page-size", &page_size_str]);
            filters["page_size"] = json!(page_size);
        } else if let Some(limit) = args.get("limit").and_then(|v| v.as_u64()) {
            limit_str = limit.to_string();
            cli_args.extend_from_slice(&["--limit", &limit_str]);
            filters["limit"] = json!(limit);
        }

        if let Some(page_number) = args.get("page_number").and_then(|v| v.as_u64()) {
            page_number_str = page_number.to_string();
            cli_args.extend_from_slice(&["--page-number", &page_number_str]);
            filters["page_number"] = json!(page_number);
        } else if let Some(offset) = args.get("offset").and_then(|v| v.as_u64()) {
            offset_str = offset.to_string();
            cli_args.extend_from_slice(&["--offset", &offset_str]);
            filters["offset"] = json!(offset);
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
        let pipeline_id = args
            .get("pipeline_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| StudioError::InvalidOperation("pipeline_id is required".to_string()))?;

        match self
            .cli_manager
            .execute(
                &["plm", "pipeline", "get", pipeline_id, "--output", "yaml"],
                None,
            )
            .await
        {
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
        let pipeline_identifier =
            if let Some(name) = args.get("pipeline_name").and_then(|v| v.as_str()) {
                cli_args.extend_from_slice(&["--name", name]);
                name
            } else if let Some(id) = args.get("pipeline_id").and_then(|v| v.as_str()) {
                cli_args.extend_from_slice(&["--id", id]);
                id
            } else {
                return Err(StudioError::InvalidOperation(
                    "Either pipeline_name or pipeline_id is required".to_string(),
                ));
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
        let is_follow = args
            .get("follow")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        if is_follow {
            cli_args.push("--follow");
        }

        // Use appropriate timeout based on operation type
        let timeout_duration = if is_follow {
            Duration::from_secs(
                self.config
                    .cli
                    .timeouts
                    .get_timeout(OperationType::PipelineFollow),
            )
        } else {
            Duration::from_secs(
                self.config
                    .cli
                    .timeouts
                    .get_timeout(OperationType::PipelineStart),
            )
        };

        match self
            .cli_manager
            .execute_with_timeout(&cli_args, None, timeout_duration)
            .await
        {
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
        let run_id = args
            .get("run_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| StudioError::InvalidOperation("run_id is required".to_string()))?;

        match self
            .cli_manager
            .execute(&["plm", "run", "cancel", run_id, "--output", "json"], None)
            .await
        {
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

        // Add comprehensive filters
        let mut filters = json!({});

        // Pipeline filters
        if let Some(name) = args.get("pipeline_name").and_then(|v| v.as_str()) {
            cli_args.extend_from_slice(&["--pipeline-name", name]);
            filters["pipeline_name"] = json!(name);
        } else if let Some(id) = args.get("pipeline_id").and_then(|v| v.as_str()) {
            cli_args.extend_from_slice(&["--pipeline-id", id]);
            filters["pipeline_id"] = json!(id);
        }

        // Run-specific filters
        let run_number_str;
        if let Some(run_number) = args.get("run_number").and_then(|v| v.as_u64()) {
            run_number_str = run_number.to_string();
            cli_args.extend_from_slice(&["--run-number", &run_number_str]);
            filters["run_number"] = json!(run_number);
        }

        if let Some(status) = args.get("status").and_then(|v| v.as_str()) {
            cli_args.extend_from_slice(&["--status", status]);
            filters["status"] = json!(status);
        }

        if let Some(created_by) = args.get("created_by").and_then(|v| v.as_str()) {
            cli_args.extend_from_slice(&["--created-by", created_by]);
            filters["created_by"] = json!(created_by);
        }

        // Time-based filters
        if let Some(start_time) = args.get("start_time").and_then(|v| v.as_str()) {
            cli_args.extend_from_slice(&["--start-time", start_time]);
            filters["start_time"] = json!(start_time);
        }

        if let Some(end_time) = args.get("end_time").and_then(|v| v.as_str()) {
            cli_args.extend_from_slice(&["--end-time", end_time]);
            filters["end_time"] = json!(end_time);
        }

        // Boolean flags
        if let Some(from_failure) = args.get("from_failure").and_then(|v| v.as_bool()) {
            if from_failure {
                cli_args.push("--from-failure");
            }
            filters["from_failure"] = json!(from_failure);
        }

        if let Some(compile_only) = args.get("compile_only").and_then(|v| v.as_bool()) {
            if compile_only {
                cli_args.push("--compile-only");
            }
            filters["compile_only"] = json!(compile_only);
        }

        // Sorting and pagination
        if let Some(sort_column) = args.get("sort_column").and_then(|v| v.as_str()) {
            cli_args.extend_from_slice(&["--sort-column", sort_column]);
            filters["sort_column"] = json!(sort_column);
        }

        if let Some(sort_direction) = args.get("sort_direction").and_then(|v| v.as_str()) {
            cli_args.extend_from_slice(&["--sort-direction", sort_direction]);
            filters["sort_direction"] = json!(sort_direction);
        }

        let limit_str;
        let offset_str;

        if let Some(limit) = args.get("limit").and_then(|v| v.as_u64()) {
            limit_str = limit.to_string();
            cli_args.extend_from_slice(&["--limit", &limit_str]);
            filters["limit"] = json!(limit);
        }

        if let Some(offset) = args.get("offset").and_then(|v| v.as_u64()) {
            offset_str = offset.to_string();
            cli_args.extend_from_slice(&["--offset", &offset_str]);
            filters["offset"] = json!(offset);
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

        let mut cli_args = vec!["plm", "run", "get", &run_id, "--output", "json"];

        // Add additional options based on parameters
        if let Some(run_config) = args.get("run_config").and_then(|v| v.as_bool()) {
            if run_config {
                cli_args.push("--run-config");
            }
        }

        if let Some(detailed_info) = args.get("detailed_info").and_then(|v| v.as_bool()) {
            if detailed_info {
                cli_args.push("--detailed-info");
            }
        }

        if let Some(include_tasks) = args.get("include_tasks").and_then(|v| v.as_bool()) {
            if include_tasks {
                cli_args.push("--include-tasks");
            }
        }

        if let Some(execution_logs) = args.get("execution_logs").and_then(|v| v.as_bool()) {
            if execution_logs {
                cli_args.push("--execution-logs");
            }
        }

        match self.cli_manager.execute(&cli_args, None).await {
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

        if let Some(query_since) = args.get("query_since").and_then(|v| v.as_str()) {
            additional_args.push("--query-since".to_string());
            additional_args.push(query_since.to_string());
        }

        if let Some(query_until) = args.get("query_until").and_then(|v| v.as_str()) {
            additional_args.push("--query-until".to_string());
            additional_args.push(query_until.to_string());
        }

        if let Some(log_type) = args.get("log_type").and_then(|v| v.as_str()) {
            additional_args.push("--log-type".to_string());
            additional_args.push(log_type.to_string());
        }

        if let Some(sort_column) = args.get("sort_column").and_then(|v| v.as_str()) {
            additional_args.push("--sort-column".to_string());
            additional_args.push(sort_column.to_string());
        }

        if args
            .get("raw_field")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
        {
            additional_args.push("--raw-field".to_string());
        }

        // Add additional args as string references
        for arg in &additional_args {
            cli_args.push(arg.as_str());
        }

        match self.cli_manager.execute(&cli_args, None).await {
            Ok(mut result) => {
                // Apply client-side error filtering if requested
                if args
                    .get("errors_only")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false)
                {
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

        match self
            .cli_manager
            .execute(&["plm", "run", "events", &run_id, "--output", "json"], None)
            .await
        {
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
        let pipeline_identifier =
            if let Some(name) = args.get("pipeline_name").and_then(|v| v.as_str()) {
                name
            } else if let Some(id) = args.get("pipeline_id").and_then(|v| v.as_str()) {
                id
            } else {
                return Err(StudioError::InvalidOperation(
                    "Either pipeline_name or pipeline_id is required".to_string(),
                ));
            };

        let recent_runs = args
            .get("recent_runs")
            .and_then(|v| v.as_u64())
            .unwrap_or(5) as usize;

        // Get recent runs for this pipeline
        let runs_result = match self
            .cli_manager
            .execute(
                &[
                    "plm",
                    "run",
                    "list",
                    "--pipeline",
                    pipeline_identifier,
                    "--output",
                    "json",
                ],
                None,
            )
            .await
        {
            Ok(result) => result,
            Err(e) => {
                error!(
                    "Failed to get runs for pipeline {}: {}",
                    pipeline_identifier, e
                );
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
                    if let Ok(log_result) = self
                        .cli_manager
                        .execute(&["plm", "run", "log", run_id, "--output", "json"], None)
                        .await
                    {
                        let filtered_errors = self.filter_error_logs(log_result);

                        // Count and categorize errors (simplified implementation)
                        if let Some(log_text) = filtered_errors.as_str() {
                            let error_count = log_text
                                .lines()
                                .filter(|line| {
                                    line.to_lowercase().contains("error")
                                        || line.to_lowercase().contains("fail")
                                })
                                .count();

                            error_summary["total_errors"] = json!(
                                error_summary["total_errors"].as_u64().unwrap_or(0)
                                    + error_count as u64
                            );

                            if error_count > 0 {
                                let recent_errors =
                                    error_summary["recent_errors"].as_array_mut().unwrap();
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
        let run_id = args
            .get("run_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| StudioError::InvalidOperation("run_id is required".to_string()))?;

        let task_name = args
            .get("task_name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| StudioError::InvalidOperation("task_name is required".to_string()))?;

        let context_lines = args
            .get("context_lines")
            .and_then(|v| v.as_u64())
            .unwrap_or(10);

        // Get logs for the specific task
        let mut cli_args = vec![
            "plm", "run", "log", run_id, "--task", task_name, "--output", "json",
        ];

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
                error!(
                    "Failed to get task errors for run {} task {}: {}",
                    run_id, task_name, e
                );
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
                    lower.contains("error")
                        || lower.contains("fail")
                        || lower.contains("exception")
                        || lower.contains("panic")
                        || lower.contains("fatal")
                        || lower.contains("warn")
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
                if lower.contains("error") || lower.contains("fail") || lower.contains("exception")
                {
                    // Get context around error
                    let start = i.saturating_sub(context_lines);
                    let end = std::cmp::min(i + context_lines + 1, lines.len());

                    let context_block: Vec<String> = lines[start..end]
                        .iter()
                        .enumerate()
                        .map(|(idx, l)| {
                            let line_num = start + idx;
                            if line_num == i {
                                format!(">>> {line_num} ERROR: {l}") // Mark error line
                            } else {
                                format!("    {line_num} {l}")
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
        let pipeline_filter = if let Some(name) = args.get("pipeline_name").and_then(|v| v.as_str())
        {
            name.to_string()
        } else if let Some(id) = args.get("pipeline_id").and_then(|v| v.as_str()) {
            id.to_string()
        } else {
            return Err(StudioError::InvalidOperation(
                "Either pipeline_name or pipeline_id is required".to_string(),
            ));
        };

        let run_number = args
            .get("run_number")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| StudioError::InvalidOperation("run_number is required".to_string()))?
            as usize;

        // Get runs for the pipeline
        let cli_args = vec![
            "plm",
            "run",
            "list",
            "--pipeline",
            &pipeline_filter,
            "--output",
            "json",
        ];

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
                    let run_id = run.get("id").and_then(|v| v.as_str()).ok_or_else(|| {
                        StudioError::InvalidOperation("Run ID not found in response".to_string())
                    })?;

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
                error!(
                    "Failed to list runs for pipeline {}: {}",
                    pipeline_filter, e
                );
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
        let pipeline_filter = if let Some(name) = args.get("pipeline_name").and_then(|v| v.as_str())
        {
            name.to_string()
        } else if let Some(id) = args.get("pipeline_id").and_then(|v| v.as_str()) {
            id.to_string()
        } else {
            return Err(StudioError::InvalidOperation(
                "Either run_id or (pipeline_name/pipeline_id + run_number) is required".to_string(),
            ));
        };

        let run_number = args
            .get("run_number")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| {
                StudioError::InvalidOperation(
                    "run_number is required when not using run_id".to_string(),
                )
            })? as usize;

        // Get runs for the pipeline
        let cli_args = vec![
            "plm",
            "run",
            "list",
            "--pipeline",
            &pipeline_filter,
            "--output",
            "json",
        ];

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
            let run_id = run.get("id").and_then(|v| v.as_str()).ok_or_else(|| {
                StudioError::InvalidOperation("Run ID not found in response".to_string())
            })?;

            Ok(run_id.to_string())
        } else {
            Err(StudioError::InvalidOperation(
                "Invalid response format from CLI".to_string(),
            ))
        }
    }

    // Task management methods
    async fn create_task(&self, args: Value) -> Result<Vec<Content>> {
        let mut cli_args = vec!["plm", "task", "create", "--output", "json"];

        // Determine input method
        if let Some(task_definition) = args.get("task_definition").and_then(|v| v.as_str()) {
            // Create task from inline definition
            cli_args.extend_from_slice(&["--definition", task_definition]);
        } else if let Some(definition_file) = args.get("definition_file").and_then(|v| v.as_str()) {
            // Create task from file
            cli_args.extend_from_slice(&["--file", definition_file]);
        } else {
            // Create task from parameters
            if let Some(name) = args.get("name").and_then(|v| v.as_str()) {
                cli_args.extend_from_slice(&["--name", name]);
            }
            if let Some(category) = args.get("category").and_then(|v| v.as_str()) {
                cli_args.extend_from_slice(&["--category", category]);
            }
            if let Some(task_lib) = args.get("task_lib").and_then(|v| v.as_str()) {
                cli_args.extend_from_slice(&["--task-lib", task_lib]);
            }
            if let Some(version) = args.get("version").and_then(|v| v.as_str()) {
                cli_args.extend_from_slice(&["--version", version]);
            }
        }

        match self.cli_manager.execute(&cli_args, None).await {
            Ok(result) => {
                let response = json!({
                    "success": true,
                    "action": "created",
                    "data": result
                });

                Ok(vec![Content::Text {
                    text: serde_json::to_string_pretty(&response)?,
                }])
            }
            Err(e) => {
                error!("Failed to create task: {}", e);
                let error_response = json!({
                    "success": false,
                    "error": e.to_string(),
                    "message": "Failed to create task"
                });

                Ok(vec![Content::Text {
                    text: serde_json::to_string_pretty(&error_response)?,
                }])
            }
        }
    }

    async fn update_task(&self, args: Value) -> Result<Vec<Content>> {
        let mut cli_args = vec!["plm", "task", "update", "--output", "json"];

        // Task name is required
        if let Some(task_name) = args.get("task_name").and_then(|v| v.as_str()) {
            cli_args.extend_from_slice(&["--name", task_name]);
        }

        // Add definition source
        if let Some(task_definition) = args.get("task_definition").and_then(|v| v.as_str()) {
            cli_args.extend_from_slice(&["--definition", task_definition]);
        } else if let Some(definition_file) = args.get("definition_file").and_then(|v| v.as_str()) {
            cli_args.extend_from_slice(&["--file", definition_file]);
        }

        match self.cli_manager.execute(&cli_args, None).await {
            Ok(result) => {
                let response = json!({
                    "success": true,
                    "action": "updated",
                    "task_name": args.get("task_name"),
                    "data": result
                });

                Ok(vec![Content::Text {
                    text: serde_json::to_string_pretty(&response)?,
                }])
            }
            Err(e) => {
                error!("Failed to update task: {}", e);
                let error_response = json!({
                    "success": false,
                    "task_name": args.get("task_name"),
                    "error": e.to_string(),
                    "message": "Failed to update task"
                });

                Ok(vec![Content::Text {
                    text: serde_json::to_string_pretty(&error_response)?,
                }])
            }
        }
    }

    async fn delete_task(&self, args: Value) -> Result<Vec<Content>> {
        let mut cli_args = vec!["plm", "task", "delete", "--output", "json"];

        if let Some(task_name) = args.get("task_name").and_then(|v| v.as_str()) {
            cli_args.extend_from_slice(&["--name", task_name]);
        }

        match self.cli_manager.execute(&cli_args, None).await {
            Ok(result) => {
                let response = json!({
                    "success": true,
                    "action": "deleted",
                    "task_name": args.get("task_name"),
                    "data": result
                });

                Ok(vec![Content::Text {
                    text: serde_json::to_string_pretty(&response)?,
                }])
            }
            Err(e) => {
                error!("Failed to delete task: {}", e);
                let error_response = json!({
                    "success": false,
                    "task_name": args.get("task_name"),
                    "error": e.to_string(),
                    "message": "Failed to delete task"
                });

                Ok(vec![Content::Text {
                    text: serde_json::to_string_pretty(&error_response)?,
                }])
            }
        }
    }

    async fn rename_task(&self, args: Value) -> Result<Vec<Content>> {
        let mut cli_args = vec!["plm", "task", "rename", "--output", "json"];

        if let Some(old_name) = args.get("old_task_name").and_then(|v| v.as_str()) {
            cli_args.extend_from_slice(&["--old-task-name", old_name]);
        }

        if let Some(new_name) = args.get("new_task_name").and_then(|v| v.as_str()) {
            cli_args.extend_from_slice(&["--new-task-name", new_name]);
        }

        match self.cli_manager.execute(&cli_args, None).await {
            Ok(result) => {
                let response = json!({
                    "success": true,
                    "action": "renamed",
                    "old_task_name": args.get("old_task_name"),
                    "new_task_name": args.get("new_task_name"),
                    "data": result
                });

                Ok(vec![Content::Text {
                    text: serde_json::to_string_pretty(&response)?,
                }])
            }
            Err(e) => {
                error!("Failed to rename task: {}", e);
                let error_response = json!({
                    "success": false,
                    "old_task_name": args.get("old_task_name"),
                    "new_task_name": args.get("new_task_name"),
                    "error": e.to_string(),
                    "message": "Failed to rename task"
                });

                Ok(vec![Content::Text {
                    text: serde_json::to_string_pretty(&error_response)?,
                }])
            }
        }
    }

    async fn list_tasks(&self, args: Value) -> Result<Vec<Content>> {
        let mut cli_args = vec!["plm", "task", "list", "--output", "json"];

        let mut filters = json!({});

        if let Some(category) = args.get("category").and_then(|v| v.as_str()) {
            cli_args.extend_from_slice(&["--category", category]);
            filters["category"] = json!(category);
        }

        if let Some(task_lib) = args.get("task_lib").and_then(|v| v.as_str()) {
            cli_args.extend_from_slice(&["--task-lib", task_lib]);
            filters["task_lib"] = json!(task_lib);
        }

        if let Some(include_tasks) = args.get("include_tasks").and_then(|v| v.as_bool()) {
            if include_tasks {
                cli_args.push("--include-tasks");
            }
            filters["include_tasks"] = json!(include_tasks);
        }

        let limit_str;
        let offset_str;

        if let Some(limit) = args.get("limit").and_then(|v| v.as_u64()) {
            limit_str = limit.to_string();
            cli_args.extend_from_slice(&["--limit", &limit_str]);
            filters["limit"] = json!(limit);
        }

        if let Some(offset) = args.get("offset").and_then(|v| v.as_u64()) {
            offset_str = offset.to_string();
            cli_args.extend_from_slice(&["--offset", &offset_str]);
            filters["offset"] = json!(offset);
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
                error!("Failed to list tasks: {}", e);
                let error_response = json!({
                    "success": false,
                    "error": e.to_string(),
                    "message": "Failed to list tasks"
                });

                Ok(vec![Content::Text {
                    text: serde_json::to_string_pretty(&error_response)?,
                }])
            }
        }
    }

    async fn get_task(&self, args: Value) -> Result<Vec<Content>> {
        let mut cli_args = vec!["plm", "task", "get", "--output", "json"];

        if let Some(task_name) = args.get("task_name").and_then(|v| v.as_str()) {
            cli_args.extend_from_slice(&["--name", task_name]);
        }

        if let Some(category) = args.get("category").and_then(|v| v.as_str()) {
            cli_args.extend_from_slice(&["--category", category]);
        }

        if let Some(version) = args.get("version").and_then(|v| v.as_str()) {
            cli_args.extend_from_slice(&["--version", version]);
        }

        match self.cli_manager.execute(&cli_args, None).await {
            Ok(result) => {
                let response = json!({
                    "success": true,
                    "task_name": args.get("task_name"),
                    "data": result
                });

                Ok(vec![Content::Text {
                    text: serde_json::to_string_pretty(&response)?,
                }])
            }
            Err(e) => {
                error!("Failed to get task: {}", e);
                let error_response = json!({
                    "success": false,
                    "task_name": args.get("task_name"),
                    "error": e.to_string(),
                    "message": "Failed to retrieve task information"
                });

                Ok(vec![Content::Text {
                    text: serde_json::to_string_pretty(&error_response)?,
                }])
            }
        }
    }

    async fn unlock_task(&self, args: Value) -> Result<Vec<Content>> {
        let mut cli_args = vec!["plm", "task", "unlock", "--output", "json"];

        if let Some(task_name) = args.get("task_name").and_then(|v| v.as_str()) {
            cli_args.extend_from_slice(&["--name", task_name]);
        }

        match self.cli_manager.execute(&cli_args, None).await {
            Ok(result) => {
                let response = json!({
                    "success": true,
                    "action": "unlocked",
                    "task_name": args.get("task_name"),
                    "data": result
                });

                Ok(vec![Content::Text {
                    text: serde_json::to_string_pretty(&response)?,
                }])
            }
            Err(e) => {
                error!("Failed to unlock task: {}", e);
                let error_response = json!({
                    "success": false,
                    "task_name": args.get("task_name"),
                    "error": e.to_string(),
                    "message": "Failed to unlock task"
                });

                Ok(vec![Content::Text {
                    text: serde_json::to_string_pretty(&error_response)?,
                }])
            }
        }
    }

    async fn rename_param(&self, args: Value) -> Result<Vec<Content>> {
        let mut cli_args = vec!["plm", "pipeline", "rename-param", "--output", "json"];

        let old_param_name = args
            .get("old_param_name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| StudioError::InvalidOperation("old_param_name is required".to_string()))?;

        let new_param_name = args
            .get("new_param_name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| StudioError::InvalidOperation("new_param_name is required".to_string()))?;

        cli_args.extend_from_slice(&["--old-param-name", old_param_name]);
        cli_args.extend_from_slice(&["--new-param-name", new_param_name]);

        // Either pipeline name or file is required (validated by anyOf schema)
        if let Some(pipeline_name) = args.get("pipeline_name").and_then(|v| v.as_str()) {
            cli_args.extend_from_slice(&["--name", pipeline_name]);
        } else if let Some(file) = args.get("file").and_then(|v| v.as_str()) {
            cli_args.extend_from_slice(&["--file", file]);
        } else {
            return Err(StudioError::InvalidOperation(
                "Either pipeline_name or file is required".to_string(),
            ));
        }

        match self.cli_manager.execute(&cli_args, None).await {
            Ok(result) => {
                let response = json!({
                    "success": true,
                    "action": "renamed_parameter",
                    "pipeline_name": args.get("pipeline_name"),
                    "old_param_name": old_param_name,
                    "new_param_name": new_param_name,
                    "data": result
                });

                Ok(vec![Content::Text {
                    text: serde_json::to_string_pretty(&response)?,
                }])
            }
            Err(e) => {
                error!("Failed to rename parameter: {}", e);
                let error_response = json!({
                    "success": false,
                    "pipeline_name": args.get("pipeline_name"),
                    "old_param_name": old_param_name,
                    "new_param_name": new_param_name,
                    "error": e.to_string(),
                    "message": "Failed to rename pipeline parameter"
                });

                Ok(vec![Content::Text {
                    text: serde_json::to_string_pretty(&error_response)?,
                }])
            }
        }
    }

    async fn create_access_config(&self, args: Value) -> Result<Vec<Content>> {
        let mut cli_args = vec!["plm", "access-config", "create", "--output", "json"];

        let name = args
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| StudioError::InvalidOperation("name is required".to_string()))?;

        cli_args.extend_from_slice(&["--name", name]);

        if let Some(username) = args.get("username").and_then(|v| v.as_str()) {
            cli_args.extend_from_slice(&["--username", username]);
        }

        if let Some(password) = args.get("password").and_then(|v| v.as_str()) {
            cli_args.extend_from_slice(&["--password", password]);
        }

        if let Some(group) = args.get("group").and_then(|v| v.as_str()) {
            cli_args.extend_from_slice(&["--group", group]);
        }

        // Handle create_ssh flag (default is true)
        let create_ssh = args.get("create_ssh").and_then(|v| v.as_bool()).unwrap_or(true);
        if !create_ssh {
            cli_args.push("--create-ssh=false");
        }

        match self.cli_manager.execute(&cli_args, None).await {
            Ok(result) => {
                let response = json!({
                    "success": true,
                    "action": "created",
                    "name": name,
                    "data": result
                });

                Ok(vec![Content::Text {
                    text: serde_json::to_string_pretty(&response)?,
                }])
            }
            Err(e) => {
                error!("Failed to create access config: {}", e);
                let error_response = json!({
                    "success": false,
                    "name": name,
                    "error": e.to_string(),
                    "message": "Failed to create access configuration"
                });

                Ok(vec![Content::Text {
                    text: serde_json::to_string_pretty(&error_response)?,
                }])
            }
        }
    }

    async fn list_access_configs(&self, _args: Value) -> Result<Vec<Content>> {
        let cli_args = vec!["plm", "access-config", "list", "--output", "json"];

        match self.cli_manager.execute(&cli_args, None).await {
            Ok(result) => {
                let configs = if let Some(array) = result.as_array() {
                    array.clone()
                } else if let Some(obj) = result.as_object() {
                    if let Some(configs) = obj.get("access_configs").and_then(|v| v.as_array()) {
                        configs.clone()
                    } else {
                        vec![result]
                    }
                } else {
                    vec![]
                };

                let response = json!({
                    "success": true,
                    "data": configs,
                    "total": configs.len()
                });

                Ok(vec![Content::Text {
                    text: serde_json::to_string_pretty(&response)?,
                }])
            }
            Err(e) => {
                error!("Failed to list access configs: {}", e);
                let error_response = json!({
                    "success": false,
                    "error": e.to_string(),
                    "message": "Failed to list access configurations"
                });

                Ok(vec![Content::Text {
                    text: serde_json::to_string_pretty(&error_response)?,
                }])
            }
        }
    }

    async fn get_access_config(&self, args: Value) -> Result<Vec<Content>> {
        let name = args
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| StudioError::InvalidOperation("name is required".to_string()))?;

        let cli_args = vec!["plm", "access-config", "get", name, "--output", "json"];

        match self.cli_manager.execute(&cli_args, None).await {
            Ok(result) => {
                let response = json!({
                    "success": true,
                    "name": name,
                    "data": result
                });

                Ok(vec![Content::Text {
                    text: serde_json::to_string_pretty(&response)?,
                }])
            }
            Err(e) => {
                error!("Failed to get access config: {}", e);
                let error_response = json!({
                    "success": false,
                    "name": name,
                    "error": e.to_string(),
                    "message": "Failed to get access configuration"
                });

                Ok(vec![Content::Text {
                    text: serde_json::to_string_pretty(&error_response)?,
                }])
            }
        }
    }

    async fn delete_access_config(&self, args: Value) -> Result<Vec<Content>> {
        let name = args
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| StudioError::InvalidOperation("name is required".to_string()))?;

        let cli_args = vec!["plm", "access-config", "delete", name, "--output", "json"];

        match self.cli_manager.execute(&cli_args, None).await {
            Ok(result) => {
                let response = json!({
                    "success": true,
                    "action": "deleted",
                    "name": name,
                    "data": result
                });

                Ok(vec![Content::Text {
                    text: serde_json::to_string_pretty(&response)?,
                }])
            }
            Err(e) => {
                error!("Failed to delete access config: {}", e);
                let error_response = json!({
                    "success": false,
                    "name": name,
                    "error": e.to_string(),
                    "message": "Failed to delete access configuration"
                });

                Ok(vec![Content::Text {
                    text: serde_json::to_string_pretty(&error_response)?,
                }])
            }
        }
    }
}
