# Claude Desktop Integration

This guide shows how to integrate the WindRiver Studio MCP Server with Claude Desktop.

## Prerequisites

1. **Claude Desktop**: Download from [claude.ai](https://claude.ai/desktop)
2. **Studio MCP Server**: Install via npm or build from source
3. **Studio Access**: Valid WindRiver Studio credentials

## Configuration Steps

### 1. Create Configuration File

Create a configuration file (e.g., `studio-config.json`):

```json
{
  "connections": {
    "default": {
      "name": "My Studio",
      "url": "https://your-studio-instance.com",
      "username": "your-username"
    }
  },
  "cli": {
    "download_base_url": "https://distro.windriver.com/dist/wrstudio/wrstudio-cli-distro-cd",
    "version": "auto",
    "auto_update": true
  },
  "cache": {
    "enabled": true,
    "max_memory_mb": 50
  }
}
```

### 2. Configure Claude Desktop

#### Option A: Using npm package (Recommended)

Add to your Claude Desktop MCP configuration file:

**macOS**: `~/Library/Application Support/Claude/claude_desktop_config.json`
**Windows**: `%APPDATA%\Claude\claude_desktop_config.json`

```json
{
  "mcpServers": {
    "windrive-studio": {
      "command": "npx",
      "args": [
        "@pulseengine/studio-mcp-server@latest",
        "/path/to/your/studio-config.json"
      ]
    }
  }
}
```

#### Option B: Using compiled binary

```json
{
  "mcpServers": {
    "windrive-studio": {
      "command": "/path/to/studio-mcp-server",
      "args": ["/path/to/your/studio-config.json"]
    }
  }
}
```

### 3. Set Credentials

#### Environment Variables (Recommended)

Set your Studio password as an environment variable:

**macOS/Linux**:
```bash
export STUDIO_PASSWORD="your-password"
```

**Windows**:
```cmd
set STUDIO_PASSWORD=your-password
```

#### Keyring Storage

For persistent credential storage, use keyring authentication:

```json
{
  "connections": {
    "default": {
      "name": "My Studio",
      "url": "https://your-studio-instance.com",
      "username": "your-username",
      "auth_method": "keyring"
    }
  }
}
```

Then set credentials:
```bash
npx @pulseengine/studio-mcp-server@latest --set-password studio-config.json
```

### 4. Restart Claude Desktop

Restart Claude Desktop to load the new MCP server configuration.

## Testing the Integration

### Basic Connection Test

Ask Claude:
```
"Can you check if the Studio MCP server is working?"
```

Claude should respond with server status information.

### Pipeline Queries

Try these example queries:

```
"Show me all my Studio pipelines"
```

```
"What's the status of my recent pipeline runs?"
```

```
"List all active pipelines in my default project"
```

### Resource Exploration

```
"What Studio resources are available through MCP?"
```

```
"Show me the structure of my Studio workspace"
```

## Advanced Configuration

### Multiple Studio Instances

Configure multiple Studio environments:

```json
{
  "connections": {
    "production": {
      "name": "Production Studio",
      "url": "https://prod-studio.company.com",
      "username": "prod-user"
    },
    "staging": {
      "name": "Staging Studio", 
      "url": "https://staging-studio.company.com",
      "username": "staging-user"
    }
  }
}
```

### Performance Tuning

For better performance with frequent queries:

```json
{
  "cache": {
    "enabled": true,
    "max_memory_mb": 100,
    "ttl": {
      "immutable": 7200,
      "completed": 172800,
      "semi_dynamic": 1200,
      "dynamic": 120
    }
  },
  "server": {
    "max_concurrent_requests": 5,
    "timeout_seconds": 45
  }
}
```

## Troubleshooting

### Server Not Connecting

1. **Check configuration**:
   ```bash
   npx @pulseengine/studio-mcp-server@latest --check-config studio-config.json
   ```

2. **Test Studio connection**:
   ```bash
   npx @pulseengine/studio-mcp-server@latest --test-connection studio-config.json
   ```

3. **Check Claude Desktop logs**:
   - **macOS**: `~/Library/Logs/Claude/mcp.log`
   - **Windows**: `%LOCALAPPDATA%\Claude\logs\mcp.log`

### Authentication Issues

1. **Verify credentials**:
   ```bash
   echo $STUDIO_PASSWORD  # Should show your password
   ```

2. **Test manual login**:
   ```bash
   studio-cli login --url https://your-studio-instance.com --username your-username
   ```

3. **Check keyring storage**:
   ```bash
   npx @pulseengine/studio-mcp-server@latest --check-auth studio-config.json
   ```

### Performance Issues

1. **Enable caching**:
   ```json
   {
     "cache": {
       "enabled": true,
       "max_memory_mb": 100
     }
   }
   ```

2. **Increase timeouts**:
   ```json
   {
     "server": {
       "timeout_seconds": 60
     }
   }
   ```

3. **Monitor performance**:
   ```json
   {
     "server": {
       "enable_metrics": true
     }
   }
   ```

## Next Steps

- Explore [workflow examples](pipeline-automation.md)
- Set up [monitoring and alerts](monitoring-setup.md)
- Review [best practices](../docs/best-practices.md)
- Check out [advanced integrations](vscode-integration.md)