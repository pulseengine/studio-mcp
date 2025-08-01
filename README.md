# WindRiver Studio MCP Server

A production-ready Model Context Protocol (MCP) server providing AI assistants with secure access to WindRiver Studio CLI functionality, focusing on Pipeline Management (PLM) features.

**Current Version: 0.2.15** - Built with PulseEngine MCP 0.7.0

## Features

### Core Capabilities
- **🏗️ Pipeline Management**: Complete PLM workflow integration
- **🤖 CLI Automation**: Automatic Studio CLI download and version management  
- **🔐 Secure Authentication**: Multi-instance Studio credential management
- **⚡ Intelligent Caching**: High-performance caching with smart invalidation
- **📊 Resource Hierarchy**: Structured MCP resource access
- **🛠️ Comprehensive Tools**: Full pipeline lifecycle management

### Production Features
- **Multi-platform Support**: Native binaries for macOS, Linux, and Windows
- **User Isolation**: Secure multi-tenant cache and authentication
- **Performance Monitoring**: Detailed cache metrics and health monitoring
- **Sensitive Data Protection**: Automatic credential filtering and sanitization
- **Error Recovery**: Robust error handling with detailed diagnostics

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

### Quick Start with npm (Recommended)

Install the latest version globally:
```bash
npm install -g @pulseengine/studio-mcp-server@latest
```

Or run directly without installation:
```bash
npx @pulseengine/studio-mcp-server@latest [config-file]
```

### Building from Source

#### Prerequisites
- Rust 1.70+ with Cargo
- WindRiver Studio access credentials

#### Build Process
```bash
git clone https://github.com/pulseengine/studio-mcp.git
cd studio-mcp
cargo build --release
```

The binary will be available at `target/release/studio-mcp-server`

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

#### Using npm installation:
Add to your Claude Desktop MCP configuration:

```json
{
  "mcpServers": {
    "windrive-studio": {
      "command": "npx",
      "args": ["@pulseengine/studio-mcp-server@latest", "/path/to/config.json"]
    }
  }
}
```

#### Using compiled binary:
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

### MCP Client Integration

The server works with any MCP-compatible client:
- **Claude Desktop**: Official Claude app with MCP support
- **VS Code MCP Extension**: Development environment integration
- **Custom MCP Clients**: Using MCP protocol libraries

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

**Current Release: v0.2.15** - Production-ready with PulseEngine MCP 0.7.0

### ✅ Completed Features
- **Core MCP Server**: Full PulseEngine MCP 0.7.0 integration
- **CLI Management**: Automatic download, version management, and execution
- **Pipeline Management**: Complete PLM resource and tool providers
- **Intelligent Caching**: Multi-layer caching with performance monitoring
- **Authentication**: Secure multi-instance credential management
- **Multi-platform Distribution**: npm packages for all major platforms
- **Automated CI/CD**: GitHub Actions with automated version management

### 🚀 Production Ready
- **Performance**: Optimized caching reduces API calls by 80%+
- **Security**: Credential isolation and sensitive data filtering
- **Reliability**: Comprehensive error handling and recovery
- **Monitoring**: Built-in health checks and performance metrics
- **Documentation**: Complete API documentation and examples

### 📋 Planned Enhancements
- **Artifact Management**: Direct artifact storage and retrieval
- **Virtual Lab Integration**: VLab resource management and automation
- **Build System Support**: LXBS/VXBS integration
- **Advanced Monitoring**: OpenTelemetry integration and dashboards
- **Plugin System**: Extensible architecture for custom integrations

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