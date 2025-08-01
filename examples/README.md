# Examples

This directory contains working examples and configuration samples for the WindRiver Studio MCP Server.

## Quick Start Examples

### Basic Setup
- [basic-config.json](basic-config.json) - Minimal working configuration
- [claude-desktop.md](claude-desktop.md) - Claude Desktop integration guide
- [first-pipeline.md](first-pipeline.md) - Running your first pipeline query

### Configuration Examples
- [dev-config.json](dev-config.json) - Development environment setup
- [prod-config.json](prod-config.json) - Production configuration with security
- [multi-instance-config.json](multi-instance-config.json) - Multiple Studio instances

### Integration Examples
- [vscode-integration.md](vscode-integration.md) - VS Code MCP extension setup
- [api-usage.py](api-usage.py) - Direct MCP protocol usage
- [monitoring-setup.md](monitoring-setup.md) - Performance monitoring

### Workflow Examples
- [pipeline-automation.md](pipeline-automation.md) - Automated pipeline management
- [status-monitoring.md](status-monitoring.md) - Pipeline status dashboards
- [troubleshooting-workflow.md](troubleshooting-workflow.md) - Debugging pipelines

## Usage Patterns

### Common Queries

Ask Claude to help with these typical Studio operations:

```
"Show me all my active pipelines"
"What's the status of pipeline XYZ-123?"
"List all failed tasks in my recent pipelines"
"Show me the logs for task ABC in pipeline XYZ-123"
"Start the deployment pipeline for project MyApp"
```

### Advanced Workflows

```
"Create a summary of all pipeline failures this week"
"Compare the performance of pipelines between staging and production"
"Help me diagnose why pipeline XYZ-123 is failing repeatedly"
"Set up monitoring for my critical production pipelines"
```

## Testing Your Setup

1. **Configuration Test**:
   ```bash
   npx @pulseengine/studio-mcp-server@latest --check-config basic-config.json
   ```

2. **Connection Test**:
   ```bash
   npx @pulseengine/studio-mcp-server@latest --test-connection basic-config.json
   ```

3. **Claude Desktop Test**:
   - Use configuration from [claude-desktop.md](claude-desktop.md)
   - Ask Claude: "Can you list my Studio pipelines?"

## Getting Help

- Check [troubleshooting guide](../docs/troubleshooting.md) for common issues
- Review [configuration reference](../docs/configuration.md) for all options
- Open [GitHub issues](https://github.com/pulseengine/studio-mcp/issues) for bugs
- Use [GitHub discussions](https://github.com/pulseengine/studio-mcp/discussions) for questions