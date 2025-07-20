# WindRiver Studio MCP Server

A Rust-based Model Context Protocol (MCP) server providing AI assistants with access to WindRiver Studio CLI functionality, with a focus on Pipeline Management (PLM) features.

## Features

### Current Implementation
- **CLI Management**: Automatic download and version management of WindRiver Studio CLI
- **Resource Hierarchy**: Structured access to Studio resources via MCP protocol
- **PLM Integration**: Pipeline and task management capabilities
- **PulseEngine Framework**: Built using the production-proven PulseEngine MCP framework

### Supported Operations
- List and inspect pipelines
- View pipeline tasks and their status
- Access task logs and artifacts
- Pipeline execution controls (run, stop)
- Project and template management

## Architecture

The server follows a modular architecture with three main components:

### 1. CLI Manager (`studio-cli-manager`)
- **Downloader**: Handles CLI binary downloads with checksum verification
- **Executor**: Manages CLI command execution and output parsing
- **Version Manager**: Tracks available versions and handles updates

### 2. MCP Server (`studio-mcp-server`)
- **Resource Providers**: Expose Studio data as MCP resources
- **Tool Providers**: Enable actions through MCP tools
- **Server Handler**: Implements PulseEngine MCP backend interface

### 3. Shared Types (`studio-mcp-shared`)
- **Configuration**: Server and CLI configuration management
- **Error Handling**: Comprehensive error types and handling
- **Protocol Types**: WindRiver Studio specific data structures

## Resource Hierarchy

```
studio://
├── plm/                          # Pipeline Management
│   ├── pipelines/               # Pipeline listings
│   │   ├── {pipeline-id}/info   # Pipeline details
│   │   ├── {pipeline-id}/tasks/ # Pipeline tasks
│   │   └── {pipeline-id}/history # Execution history
│   ├── projects/                # PLM projects
│   └── templates/               # Pipeline templates
├── artifacts/                   # Artifact management (planned)
├── vlab/                       # Virtual lab (planned)
├── config/                     # Server configuration
└── status                      # Server health status
```

## Installation

### Prerequisites
- Rust 1.70+ with Cargo
- WindRiver Studio access credentials

### Building
```bash
git clone https://github.com/pulseengine/studio-mcp.git
cd studio-mcp
cargo build --release
```

### Configuration
Create a configuration file or set environment variables:

```json
{
  "connections": {
    "default": {
      "name": "production",
      "url": "https://your-studio-instance.com",
      "username": "your-username"
    }
  },
  "cli": {
    "download_base_url": "https://distro.windriver.com/dist/wrstudio/wrstudio-cli-distro-cd",
    "version": "auto",
    "auto_update": true
  }
}
```

## Usage

### With Claude Desktop
Add to your Claude Desktop MCP configuration:

```json
{
  "mcpServers": {
    "windrive-studio": {
      "command": "/path/to/studio-mcp-server",
      "args": ["/path/to/config.json"]
    }
  }
}
```

### Available Tools

#### Pipeline Management
- `plm_list_pipelines` - List all pipelines, optionally filtered by project
- `plm_get_pipeline` - Get detailed pipeline information
- `plm_run_pipeline` - Start pipeline execution
- `plm_stop_pipeline` - Stop running pipeline
- `plm_list_tasks` - List tasks for a pipeline
- `plm_get_task` - Get task details and status
- `plm_get_task_logs` - Retrieve task execution logs

#### System Information
- `studio_status` - Get server and CLI status
- `studio_version` - Version information
- `cli_info` - Detailed CLI installation info

## Development Status

This project is currently in active development. The core infrastructure is complete, with the MCP server framework requiring final integration adjustments for the PulseEngine MCP protocol.

### Completed Components
- ✅ Project structure and workspace setup
- ✅ CLI download and management system
- ✅ Resource hierarchy design
- ✅ Tool provider implementations
- ✅ GitHub issues and project planning

### In Progress
- 🔄 PulseEngine MCP protocol integration
- 🔄 Server handler implementation

### Planned Features
- 📋 Artifact management integration
- 📋 Virtual lab (VLab) support
- 📋 Build system integration (LXBS/VXBS)
- 📋 Authentication and security enhancements

## Contributing

1. Check the [GitHub Issues](https://github.com/pulseengine/studio-mcp/issues) for planned work
2. Follow the established code patterns and error handling
3. Add tests for new functionality
4. Update documentation as needed

## License

MIT License - see LICENSE file for details.

## Acknowledgments

- Built with the [PulseEngine MCP framework](https://github.com/pulseengine/mcp)
- Designed for [WindRiver Studio](https://windriver.com) integration
- Implements the [Model Context Protocol](https://modelcontextprotocol.io/)