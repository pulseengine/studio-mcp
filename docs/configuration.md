# Configuration Reference

Complete reference for configuring the WindRiver Studio MCP Server.

## Configuration File

The server uses JSON configuration files. Create a `config.json` file:

```json
{
  "connections": {
    "default": {
      "name": "production",
      "url": "https://your-studio-instance.com",
      "username": "your-username"
    },
    "staging": {
      "name": "staging",
      "url": "https://staging.your-studio-instance.com", 
      "username": "your-username"
    }
  },
  "cli": {
    "download_base_url": "https://distro.windriver.com/dist/wrstudio/wrstudio-cli-distro-cd",
    "version": "auto",
    "auto_update": true,
    "install_dir": null
  },
  "cache": {
    "enabled": true,
    "max_memory_mb": 100,
    "ttl": {
      "immutable": 3600,
      "completed": 86400,
      "semi_dynamic": 600,
      "dynamic": 60
    }
  },
  "server": {
    "timeout_seconds": 30,
    "max_concurrent_requests": 10,
    "enable_metrics": true
  }
}
```

## Configuration Sections

### Connections

Define Studio instances and credentials:

```json
{
  "connections": {
    "connection-name": {
      "name": "display-name",
      "url": "https://studio-instance.com",
      "username": "username",
      "auth_method": "password|token|keyring"
    }
  }
}
```

**Fields:**
- `name`: Display name for the connection
- `url`: Studio instance URL (required)
- `username`: Username for authentication (required)
- `auth_method`: Authentication method (default: "password")

### CLI Configuration

Control Studio CLI management:

```json
{
  "cli": {
    "download_base_url": "https://distro.windriver.com/...",
    "version": "auto|latest|1.2.3",
    "auto_update": true,
    "install_dir": "/custom/path",
    "timeout_seconds": 300,
    "retry_attempts": 3
  }
}
```

**Fields:**
- `download_base_url`: CLI distribution URL
- `version`: CLI version to use ("auto" for latest compatible)
- `auto_update`: Automatically update CLI when available
- `install_dir`: Custom installation directory (null for default)
- `timeout_seconds`: CLI operation timeout
- `retry_attempts`: Number of retry attempts for failed operations

### Cache Configuration

Control caching behavior:

```json
{
  "cache": {
    "enabled": true,
    "max_memory_mb": 100,
    "max_items_per_type": 1000,
    "memory_eviction_threshold": 0.9,
    "ttl": {
      "immutable": 3600,
      "completed": 86400, 
      "semi_dynamic": 600,
      "dynamic": 60
    },
    "presets": {
      "development": {
        "ttl": {
          "immutable": 300,
          "completed": 3600,
          "semi_dynamic": 60,
          "dynamic": 10
        }
      }
    }
  }
}
```

**Cache Types:**
- `immutable`: Pipeline definitions, task libraries (rarely change)
- `completed`: Finished runs/tasks (stable historical data)
- `semi_dynamic`: Lists and resources (change when items added/removed)
- `dynamic`: Active runs and live events (frequently changing)

**Fields:**
- `enabled`: Enable/disable caching
- `max_memory_mb`: Maximum memory usage in MB
- `max_items_per_type`: Item limit per cache type
- `memory_eviction_threshold`: Memory threshold for eviction (0.0-1.0)
- `ttl`: Time-to-live in seconds for each cache type

### Server Configuration

Server-level settings:

```json
{
  "server": {
    "timeout_seconds": 30,
    "max_concurrent_requests": 10,
    "enable_metrics": true,
    "log_level": "info",
    "sensitive_data_filter": true
  }
}
```

**Fields:**
- `timeout_seconds`: Request timeout
- `max_concurrent_requests`: Concurrent request limit
- `enable_metrics`: Enable performance metrics
- `log_level`: Logging level (error, warn, info, debug, trace)
- `sensitive_data_filter`: Filter sensitive data from logs/cache

## Environment Variables

Override configuration with environment variables:

```bash
# Connection settings
STUDIO_URL=https://your-studio-instance.com
STUDIO_USERNAME=your-username
STUDIO_PASSWORD=your-password

# CLI settings  
STUDIO_CLI_VERSION=auto
STUDIO_CLI_AUTO_UPDATE=true

# Cache settings
STUDIO_CACHE_ENABLED=true
STUDIO_CACHE_MAX_MEMORY_MB=100

# Server settings
STUDIO_TIMEOUT_SECONDS=30
STUDIO_LOG_LEVEL=info
```

## Authentication

### Password Authentication

Prompted at startup or use environment variable:
```bash
STUDIO_PASSWORD=your-password studio-mcp-server config.json
```

### Token Authentication

Use API tokens:
```json
{
  "connections": {
    "default": {
      "auth_method": "token",
      "username": "your-username"
    }
  }
}
```

Set token via environment:
```bash
STUDIO_TOKEN=your-api-token
```

### Keyring Authentication

Store credentials in system keyring:
```json
{
  "connections": {
    "default": {
      "auth_method": "keyring",
      "username": "your-username"
    }
  }
}
```

Credentials are stored using the keyring library.

## Configuration Validation

Test your configuration:

```bash
# Validate configuration file
studio-mcp-server --check-config config.json

# Test connection
studio-mcp-server --test-connection config.json

# Show effective configuration
studio-mcp-server --show-config config.json
```

## Examples

See the [examples directory](../examples/) for complete configuration examples:
- [Basic configuration](../examples/basic-config.json)
- [Multi-instance setup](../examples/multi-instance-config.json)
- [Development configuration](../examples/dev-config.json)
- [Production configuration](../examples/prod-config.json)