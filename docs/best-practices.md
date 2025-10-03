# Best Practices

Production deployment and usage guidelines for the WindRiver Studio MCP Server.

## Security Best Practices

### Credential Management

#### DO: Use Environment Variables
```bash
# Set credentials securely
export STUDIO_PASSWORD="$(read -s -p 'Password: ' pwd; echo $pwd)"
export STUDIO_TOKEN="your-api-token"
```

#### DO: Use Keyring Storage
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

#### DON'T: Store Credentials in Configuration Files
```json
{
  "connections": {
    "default": {
      "password": "hardcoded-password"  // ❌ Never do this
    }
  }
}
```

### Network Security

#### Use HTTPS Only
```json
{
  "connections": {
    "default": {
      "url": "https://studio.company.com",  // ✅ Secure
      "url": "http://studio.company.com"    // ❌ Insecure
    }
  }
}
```

#### Certificate Validation
```bash
# ✅ Verify certificates
openssl s_client -connect studio.company.com:443 -verify 5

# ❌ Never disable certificate validation in production
export NODE_TLS_REJECT_UNAUTHORIZED=0  # Only for development/testing
```

### Data Protection

#### Enable Sensitive Data Filtering
```json
{
  "server": {
    "sensitive_data_filter": true,  // Removes credentials from logs/cache
    "log_level": "info"            // Avoid debug logs in production
  }
}
```

#### Cache Security
```json
{
  "cache": {
    "enabled": true,
    "max_memory_mb": 100,
    "user_isolation": true  // Isolate cache between users
  }
}
```

## Performance Optimization

### Caching Configuration

#### Production Settings
```json
{
  "cache": {
    "enabled": true,
    "max_memory_mb": 200,
    "memory_eviction_threshold": 0.85,
    "ttl": {
      "immutable": 86400,     // 24 hours for stable data
      "completed": 604800,    // 7 days for historical data
      "semi_dynamic": 1800,   // 30 minutes for lists
      "dynamic": 120          // 2 minutes for live data
    }
  }
}
```

#### Development Settings
```json
{
  "cache": {
    "enabled": true,
    "max_memory_mb": 50,
    "ttl": {
      "immutable": 300,       // 5 minutes for rapid iteration
      "completed": 3600,      // 1 hour
      "semi_dynamic": 60,     // 1 minute
      "dynamic": 10           // 10 seconds
    }
  }
}
```

### Resource Management

#### Concurrent Request Limits
```json
{
  "server": {
    "max_concurrent_requests": 5,    // Adjust based on Studio capacity
    "timeout_seconds": 45            // Balance responsiveness vs reliability
  }
}
```

#### CLI Resource Management
```json
{
  "cli": {
    "timeout_seconds": 300,          // 5 minutes for complex operations
    "retry_attempts": 3,             // Retry failed operations
    "cleanup_temp_files": true       // Clean up temporary files
  }
}
```

### Memory Management

#### Monitor Memory Usage
```bash
# Check current memory usage
studio-mcp-server --health | grep memory

# Set up monitoring alerts
studio-mcp-server --enable-metrics config.json
```

#### Memory-Conscious Configuration
```json
{
  "cache": {
    "max_memory_mb": 100,           // Limit total cache memory
    "max_items_per_type": 1000,     // Limit items per cache type
    "memory_eviction_threshold": 0.8 // Start evicting early
  }
}
```

## Deployment Patterns

### Single Studio Instance

#### Basic Production Setup
```json
{
  "connections": {
    "default": {
      "name": "Production Studio",
      "url": "https://studio.company.com",
      "username": "service-account",
      "auth_method": "keyring"
    }
  },
  "cache": {
    "enabled": true,
    "max_memory_mb": 200
  },
  "server": {
    "timeout_seconds": 45,
    "max_concurrent_requests": 5,
    "enable_metrics": true,
    "log_level": "info"
  }
}
```

### Multi-Instance Setup

#### Environment-Specific Configuration
```json
{
  "connections": {
    "production": {
      "name": "Production",
      "url": "https://prod-studio.company.com",
      "username": "prod-service-account"
    },
    "staging": {
      "name": "Staging", 
      "url": "https://staging-studio.company.com",
      "username": "staging-service-account"
    },
    "development": {
      "name": "Development",
      "url": "https://dev-studio.company.com", 
      "username": "dev-user"
    }
  }
}
```

### High-Availability Setup

#### Load Distribution
```json
{
  "server": {
    "max_concurrent_requests": 3,   // Lower per-instance limit
    "timeout_seconds": 60,          // Higher timeout for reliability
    "retry_attempts": 2             // Retry failed requests
  },
  "cache": {
    "enabled": true,
    "max_memory_mb": 150,
    "persistence": true             // Enable cache persistence
  }
}
```

## Monitoring and Observability

### Health Monitoring

#### Regular Health Checks
```bash
#!/bin/bash
# health-check.sh
HEALTH=$(studio-mcp-server --health --json)
if [ $? -ne 0 ]; then
    echo "Server unhealthy: $HEALTH"
    exit 1
fi
```

#### Performance Metrics
```json
{
  "server": {
    "enable_metrics": true,
    "metrics_interval_seconds": 60,
    "export_prometheus": true       // Export to Prometheus
  }
}
```

### Logging Best Practices

#### Production Logging
```json
{
  "server": {
    "log_level": "info",           // Balance detail vs noise
    "log_format": "json",          // Structured logging
    "sensitive_data_filter": true, // Remove credentials
    "log_rotation": true           // Rotate log files
  }
}
```

#### Log Analysis
```bash
# Monitor error rates
grep "ERROR" studio-mcp.log | wc -l

# Check performance metrics
grep "cache_hit_rate" studio-mcp.log | tail -10

# Monitor authentication failures
grep "auth_failed" studio-mcp.log
```

### Alerting

#### Key Metrics to Monitor
- **Health Status**: Server and CLI availability
- **Response Times**: 95th percentile response times
- **Error Rates**: Authentication and operation failures
- **Cache Performance**: Hit rates and memory usage
- **Resource Usage**: Memory and CPU consumption

#### Sample Alert Rules
```yaml
# Prometheus/Grafana alerts
- alert: StudioMCPServerDown
  expr: studio_mcp_server_up == 0
  for: 1m
  
- alert: HighErrorRate
  expr: rate(studio_mcp_errors_total[5m]) > 0.1
  for: 2m
  
- alert: LowCacheHitRate
  expr: studio_mcp_cache_hit_rate < 0.5
  for: 5m
```

## Capacity Planning

### Sizing Guidelines

#### Small Deployment (1-10 users)
```json
{
  "cache": {
    "max_memory_mb": 50
  },
  "server": {
    "max_concurrent_requests": 3
  }
}
```

#### Medium Deployment (10-50 users)
```json
{
  "cache": {
    "max_memory_mb": 150
  },
  "server": {
    "max_concurrent_requests": 8
  }
}
```

#### Large Deployment (50+ users)
```json
{
  "cache": {
    "max_memory_mb": 500
  },
  "server": {
    "max_concurrent_requests": 15
  }
}
```

### Scaling Considerations

#### Horizontal Scaling
- Deploy multiple server instances behind load balancer
- Use shared cache storage (Redis) for consistency
- Implement session affinity for user context

#### Vertical Scaling
- Increase memory limits for larger caches
- Adjust concurrent request limits based on CPU cores
- Monitor Studio instance capacity limits

## Maintenance

### Regular Tasks

#### Daily
- Check server health status
- Monitor error logs for issues
- Verify authentication is working

#### Weekly  
- Review performance metrics
- Check cache hit rates and memory usage
- Update CLI versions if available

#### Monthly
- Review and rotate credentials
- Update server configuration as needed
- Analyze usage patterns and optimize

### Updates and Upgrades

#### Safe Update Process
1. **Test in Development**:
   ```bash
   # Install new version in test environment
   npm install -g @pulseengine/studio-mcp-server@latest
   ```

2. **Backup Configuration**:
   ```bash
   cp config.json config.json.backup
   ```

3. **Rolling Update**:
   ```bash
   # Update one instance at a time
   # Verify health before updating next instance
   ```

4. **Rollback Plan**:
   ```bash
   # Keep previous version available
   npm install -g @pulseengine/studio-mcp-server@0.2.14
   ```

### Troubleshooting Workflow

1. **Check Server Health**: `studio-mcp-server --health`
2. **Review Recent Logs**: Check error logs for patterns
3. **Test Individual Components**: CLI, authentication, Studio connectivity
4. **Monitor Resource Usage**: Memory, CPU, network
5. **Escalate if Needed**: Collect diagnostics and file issue

## Integration Patterns

### Claude Desktop Integration

#### Recommended Configuration
```json
{
  "mcpServers": {
    "windrive-studio": {
      "command": "npx",
      "args": [
        "@pulseengine/studio-mcp-server@latest",
        "/absolute/path/to/config.json"
      ],
      "env": {
        "STUDIO_PASSWORD": "${STUDIO_PASSWORD}",
        "RUST_LOG": "info"
      }
    }
  }
}
```

### Custom Client Integration

#### MCP Protocol Usage
```python
# Example Python client integration
import mcp

client = mcp.Client()
client.connect("studio-mcp-server", ["config.json"])

# Use tools
result = client.call_tool("plm_list_pipelines", {"project": "MyProject"})
```

## Common Anti-Patterns

### ❌ Things to Avoid

1. **Storing Passwords in Config Files**
2. **Using HTTP for Production**
3. **Disabling Certificate Validation**
4. **Running with Debug Logging in Production**
5. **Not Monitoring Server Health**
6. **Setting Unlimited Concurrent Requests**
7. **Disabling Caching for Performance**
8. **Using Shared Accounts Instead of Service Accounts**
9. **Not Setting Appropriate Timeouts**
10. **Ignoring Memory Limits**
