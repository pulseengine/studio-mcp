# API Documentation

Complete reference for WindRiver Studio MCP Server resources and tools.

## Resources

Resources provide read-only access to Studio data through the MCP protocol.

### Resource URI Structure

```
studio://
├── plm/                          # Pipeline Management
│   ├── pipelines/               # Pipeline listings
│   │   ├── {pipeline-id}/info   # Pipeline details
│   │   ├── {pipeline-id}/tasks/ # Pipeline tasks
│   │   └── {pipeline-id}/history # Execution history
│   ├── projects/                # PLM projects
│   └── templates/               # Pipeline templates
├── config/                      # Server configuration
└── status                       # Server health status
```

### Pipeline Resources

#### List Pipelines
**URI**: `studio://plm/pipelines`

Returns all accessible pipelines with basic information.

**Response Format**:
```json
{
  "pipelines": [
    {
      "id": "pipeline-123",
      "name": "My Pipeline",
      "project": "MyProject",
      "status": "idle|running|completed|failed",
      "created": "2024-01-15T10:30:00Z",
      "last_run": "2024-01-20T14:22:00Z"
    }
  ]
}
```

#### Pipeline Details
**URI**: `studio://plm/pipelines/{pipeline-id}/info`

Detailed information about a specific pipeline.

**Response Format**:
```json
{
  "id": "pipeline-123",
  "name": "My Pipeline",
  "description": "Pipeline description",
  "project": "MyProject",
  "status": "idle",
  "definition": {
    "stages": [...],
    "parameters": {...},
    "triggers": [...]
  },
  "statistics": {
    "total_runs": 45,
    "success_rate": 0.89,
    "avg_duration_minutes": 12.5
  }
}
```

#### Pipeline Tasks
**URI**: `studio://plm/pipelines/{pipeline-id}/tasks`

List all tasks in a pipeline with their current status.

**Response Format**:
```json
{
  "pipeline_id": "pipeline-123",
  "tasks": [
    {
      "id": "task-456",
      "name": "Build Task",
      "stage": "build",
      "status": "completed|running|failed|pending",
      "duration_seconds": 145,
      "started": "2024-01-20T14:22:00Z",
      "completed": "2024-01-20T14:24:25Z"
    }
  ]
}
```

#### Pipeline History
**URI**: `studio://plm/pipelines/{pipeline-id}/history`

Execution history for a pipeline.

**Response Format**:
```json
{
  "pipeline_id": "pipeline-123",
  "runs": [
    {
      "run_id": "run-789",
      "status": "completed",
      "started": "2024-01-20T14:22:00Z",
      "completed": "2024-01-20T14:35:12Z",
      "duration_seconds": 792,
      "trigger": "manual|schedule|webhook",
      "triggered_by": "user@company.com"
    }
  ]
}
```

### Project Resources

#### List Projects
**URI**: `studio://plm/projects`

All accessible PLM projects.

**Response Format**:
```json
{
  "projects": [
    {
      "id": "project-123",
      "name": "MyProject", 
      "description": "Project description",
      "pipeline_count": 12,
      "active_pipelines": 3
    }
  ]
}
```

### Status Resources

#### Server Status
**URI**: `studio://status`

Current server health and performance metrics.

**Response Format**:
```json
{
  "status": "healthy|degraded|unhealthy",
  "version": "0.2.15",
  "uptime_seconds": 86400,
  "cli_status": {
    "version": "1.2.3",
    "available": true,
    "last_check": "2024-01-20T15:00:00Z"
  },
  "cache_stats": {
    "hit_rate": 0.87,
    "memory_usage_mb": 45.2,
    "total_operations": 1234
  },
  "connections": {
    "default": {
      "status": "connected|disconnected|error",
      "last_success": "2024-01-20T14:58:00Z"
    }
  }
}
```

## Tools

Tools enable actions and operations through the MCP protocol.

### Pipeline Management Tools

#### List Pipelines
**Tool**: `plm_list_pipelines`

List pipelines with optional filtering.

**Parameters**:
```json
{
  "project": "string (optional)",
  "status": "idle|running|completed|failed (optional)",
  "limit": "number (optional, default: 50)"
}
```

**Response**:
```json
{
  "content": [
    {
      "type": "text",
      "text": "Found 3 pipelines:\n\n1. My Pipeline (pipeline-123) - Status: idle\n2. Deploy Pipeline (pipeline-456) - Status: running\n3. Test Pipeline (pipeline-789) - Status: completed"
    }
  ]
}
```

#### Get Pipeline Details
**Tool**: `plm_get_pipeline`

Get detailed information about a specific pipeline.

**Parameters**:
```json
{
  "pipeline_id": "string (required)"
}
```

#### Run Pipeline
**Tool**: `plm_run_pipeline`

Start pipeline execution.

**Parameters**:
```json
{
  "pipeline_id": "string (required)",
  "parameters": "object (optional)",
  "wait_for_completion": "boolean (optional, default: false)"
}
```

**Response**:
```json
{
  "content": [
    {
      "type": "text", 
      "text": "Pipeline 'My Pipeline' (pipeline-123) started successfully.\nRun ID: run-987\nStatus: running"
    }
  ]
}
```

#### Stop Pipeline
**Tool**: `plm_stop_pipeline`

Stop a running pipeline.

**Parameters**:
```json
{
  "pipeline_id": "string (required)",
  "run_id": "string (optional, stops latest run if not specified)"
}
```

#### List Tasks
**Tool**: `plm_list_tasks`

List tasks for a pipeline.

**Parameters**:
```json
{
  "pipeline_id": "string (required)",
  "run_id": "string (optional, uses latest run if not specified)",
  "status": "completed|running|failed|pending (optional)"
}
```

#### Get Task Details
**Tool**: `plm_get_task`

Get detailed task information.

**Parameters**:
```json
{
  "pipeline_id": "string (required)",
  "task_id": "string (required)",
  "run_id": "string (optional)"
}
```

#### Get Task Logs
**Tool**: `plm_get_task_logs`

Retrieve task execution logs.

**Parameters**:
```json
{
  "pipeline_id": "string (required)",
  "task_id": "string (required)", 
  "run_id": "string (optional)",
  "lines": "number (optional, default: 100)",
  "follow": "boolean (optional, default: false)"
}
```

### System Tools

#### Server Status
**Tool**: `studio_status`

Get comprehensive server status.

**Parameters**: None

#### Version Information
**Tool**: `studio_version`

Get version information for server and CLI.

**Parameters**: None

#### CLI Information  
**Tool**: `cli_info`

Get detailed CLI installation and status information.

**Parameters**: None

## Error Responses

All tools return error information in a consistent format:

```json
{
  "content": [
    {
      "type": "text",
      "text": "Error: Pipeline 'invalid-123' not found"
    }
  ],
  "is_error": true
}
```

Common error types:
- **Authentication Error**: Invalid credentials or expired session
- **Not Found**: Resource or pipeline doesn't exist
- **Permission Denied**: Insufficient access rights
- **Server Error**: Studio CLI or server issues
- **Timeout**: Operation took too long to complete

## Rate Limiting

The server implements intelligent rate limiting:
- **Pipeline Operations**: 10 requests per minute
- **Resource Queries**: 100 requests per minute  
- **Status Checks**: Unlimited

Cached responses don't count toward rate limits.

## Caching Behavior

Resources are cached based on data mutability:

- **Immutable** (1 hour): Pipeline definitions, task libraries
- **Completed** (24 hours): Finished runs and tasks
- **Semi-dynamic** (10 minutes): Pipeline lists, project information
- **Dynamic** (1 minute): Active runs, live task status

Cache can be bypassed by including `"bypass_cache": true` in tool parameters.