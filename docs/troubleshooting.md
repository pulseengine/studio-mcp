# Troubleshooting Guide

Common issues and solutions for the WindRiver Studio MCP Server.

## Installation Issues

### npm Installation Fails

**Problem**: `npm install -g @pulseengine/studio-mcp-server` fails

**Solutions**:

1. **Permission Issues (macOS/Linux)**:
   ```bash
   # Use sudo (not recommended)
   sudo npm install -g @pulseengine/studio-mcp-server
   
   # Better: Configure npm to use different directory
   mkdir ~/.npm-global
   npm config set prefix '~/.npm-global'
   echo 'export PATH=~/.npm-global/bin:$PATH' >> ~/.bashrc
   source ~/.bashrc
   ```

2. **Network Issues**:
   ```bash
   # Use different registry
   npm install -g @pulseengine/studio-mcp-server --registry https://registry.npmjs.org/
   
   # Clear npm cache
   npm cache clean --force
   ```

3. **Corporate Proxy**:
   ```bash
   npm config set proxy http://proxy.company.com:8080
   npm config set https-proxy http://proxy.company.com:8080
   ```

### Binary Not Found

**Problem**: `studio-mcp-server binary not found`

**Solutions**:

1. **Check Installation**:
   ```bash
   which studio-mcp-server
   npm list -g @pulseengine/studio-mcp-server
   ```

2. **Path Issues**:
   ```bash
   # Add npm global bin to PATH
   echo 'export PATH=$(npm config get prefix)/bin:$PATH' >> ~/.bashrc
   source ~/.bashrc
   ```

3. **Use npx Instead**:
   ```bash
   npx @pulseengine/studio-mcp-server@latest config.json
   ```

## Configuration Issues

### Invalid Configuration

**Problem**: Server fails to start with configuration errors

**Solutions**:

1. **Validate JSON**:
   ```bash
   # Check JSON syntax
   python -m json.tool config.json
   
   # Or use online JSON validator
   ```

2. **Check Required Fields**:
   ```json
   {
     "connections": {
       "default": {
         "url": "https://required-field.com",
         "username": "required-field"
       }
     }
   }
   ```

3. **Test Configuration**:
   ```bash
   studio-mcp-server --check-config config.json
   ```

### Authentication Failures

**Problem**: Cannot authenticate with Studio instance

**Solutions**:

1. **Verify Credentials**:
   ```bash
   # Test manual login
   curl -u username:password https://your-studio-instance.com/api/v1/auth/login
   ```

2. **Check URL Format**:
   ```json
   {
     "connections": {
       "default": {
         "url": "https://studio.company.com", // Correct
         "url": "studio.company.com",         // Wrong - missing protocol  
         "url": "https://studio.company.com/" // Wrong - trailing slash
       }
     }
   }
   ```

3. **Environment Variables**:
   ```bash
   # Set password
   export STUDIO_PASSWORD="your-password"
   
   # Or use token
   export STUDIO_TOKEN="your-api-token"
   ```

4. **Keyring Issues**:
   ```bash
   # Clear stored credentials
   studio-mcp-server --clear-keyring config.json
   
   # Reset and re-enter credentials
   studio-mcp-server --set-password config.json
   ```

## Connection Issues

### Cannot Connect to Studio

**Problem**: Server cannot reach Studio instance

**Solutions**:

1. **Network Connectivity**:
   ```bash
   # Test basic connectivity
   ping studio.company.com
   curl -I https://studio.company.com
   ```

2. **Proxy Configuration**:
   ```bash
   # Set proxy environment variables
   export HTTP_PROXY=http://proxy.company.com:8080
   export HTTPS_PROXY=http://proxy.company.com:8080
   export NO_PROXY=localhost,127.0.0.1
   ```

3. **DNS Issues**:
   ```bash
   # Check DNS resolution
   nslookup studio.company.com
   
   # Try alternative DNS
   export DNS_SERVER=8.8.8.8
   ```

4. **Certificate Issues**:
   ```bash
   # Test SSL certificate
   openssl s_client -connect studio.company.com:443
   
   # For self-signed certificates (NOT RECOMMENDED for production)
   export NODE_TLS_REJECT_UNAUTHORIZED=0
   ```

### CLI Download Fails

**Problem**: Cannot download Studio CLI

**Solutions**:

1. **Check Download URL**:
   ```json
   {
     "cli": {
       "download_base_url": "https://distro.windriver.com/dist/wrstudio/wrstudio-cli-distro-cd"
     }
   }
   ```

2. **Manual Download**:
   ```bash
   # Download manually and specify path
   mkdir -p ~/.studio-cli
   # Download CLI to ~/.studio-cli/
   ```

3. **Proxy Issues**:
   ```json
   {
     "cli": {
       "proxy": "http://proxy.company.com:8080"
     }
   }
   ```

## Performance Issues

### Slow Response Times

**Problem**: Queries take too long to complete

**Solutions**:

1. **Enable Caching**:
   ```json
   {
     "cache": {
       "enabled": true,
       "max_memory_mb": 100
     }
   }
   ```

2. **Increase Timeouts**:
   ```json
   {
     "server": {
       "timeout_seconds": 60
     },
     "cli": {
       "timeout_seconds": 300
     }
   }
   ```

3. **Reduce Concurrent Requests**:
   ```json
   {
     "server": {
       "max_concurrent_requests": 3
     }
   }
   ```

### High Memory Usage

**Problem**: Server using too much memory

**Solutions**:

1. **Limit Cache Size**:
   ```json
   {
     "cache": {
       "max_memory_mb": 50,
       "max_items_per_type": 500
     }
   }
   ```

2. **Tune Eviction**:
   ```json
   {
     "cache": {
       "memory_eviction_threshold": 0.8
     }
   }
   ```

3. **Monitor Memory Usage**:
   ```bash
   # Check server status
   studio-mcp-server --health
   ```

## Claude Desktop Integration Issues

### Server Not Appearing

**Problem**: Studio server doesn't show up in Claude Desktop

**Solutions**:

1. **Check Configuration File Location**:
   - **macOS**: `~/Library/Application Support/Claude/claude_desktop_config.json`
   - **Windows**: `%APPDATA%\Claude\claude_desktop_config.json`

2. **Validate MCP Configuration**:
   ```json
   {
     "mcpServers": {
       "windrive-studio": {
         "command": "npx",
         "args": ["@pulseengine/studio-mcp-server@latest", "/full/path/to/config.json"]
       }
     }
   }
   ```

3. **Check File Paths**:
   ```bash
   # Use absolute paths only
   "args": ["/Users/username/studio-config.json"]  // ✓ Correct
   "args": ["~/studio-config.json"]                 // ✗ Won't work
   "args": ["./studio-config.json"]                 // ✗ Won't work
   ```

4. **Restart Claude Desktop**:
   - Completely quit and restart Claude Desktop
   - Check logs for error messages

### Authentication Prompts

**Problem**: Server keeps asking for credentials

**Solutions**:

1. **Set Environment Variables**:
   ```bash
   # Add to shell profile (.bashrc, .zshrc)
   export STUDIO_PASSWORD="your-password"
   ```

2. **Use Keyring Authentication**:
   ```json
   {
     "connections": {
       "default": {
         "auth_method": "keyring"
       }
     }
   }
   ```

3. **Check Credential Storage**:
   ```bash
   studio-mcp-server --check-auth config.json
   ```

## Logging and Debugging

### Enable Debug Logging

**Problem**: Need more detailed error information

**Solutions**:

1. **Server Logging**:
   ```json
   {
     "server": {
       "log_level": "debug"
     }
   }
   ```

2. **Environment Variable**:
   ```bash
   export RUST_LOG=debug
   studio-mcp-server config.json
   ```

3. **Claude Desktop Logs**:
   - **macOS**: `~/Library/Logs/Claude/mcp.log`
   - **Windows**: `%LOCALAPPDATA%\Claude\logs\mcp.log`

### Health Checks

**Problem**: Server appears unhealthy

**Solutions**:

1. **Run Health Check**:
   ```bash
   studio-mcp-server --health
   ```

2. **Test Individual Components**:
   ```bash
   # Test configuration
   studio-mcp-server --check-config config.json
   
   # Test Studio connection
   studio-mcp-server --test-connection config.json
   
   # Test CLI availability
   studio-mcp-server --check-cli config.json
   ```

3. **Performance Metrics**:
   ```bash
   # Enable metrics collection
   studio-mcp-server --enable-metrics config.json
   ```

## Getting Help

### Support Channels

1. **GitHub Issues**: [Report bugs and request features](https://github.com/pulseengine/studio-mcp/issues)
2. **Discussions**: [Ask questions and get help](https://github.com/pulseengine/studio-mcp/discussions) 
3. **Documentation**: [Complete documentation](../docs/)

### Information to Include

When reporting issues, include:

1. **System Information**:
   ```bash
   studio-mcp-server --version
   node --version
   npm --version
   ```

2. **Configuration** (sanitized):
   ```bash
   studio-mcp-server --show-config config.json
   ```

3. **Log Output**:
   ```bash
   RUST_LOG=debug studio-mcp-server config.json 2>&1 | head -100
   ```

4. **Health Status**:
   ```bash
   studio-mcp-server --health
   ```

### Known Issues

1. **Issue**: Windows PowerShell execution policy
   - **Solution**: `Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser`

2. **Issue**: macOS Gatekeeper blocking binary
   - **Solution**: `xattr -d com.apple.quarantine /path/to/studio-mcp-server`

3. **Issue**: Linux GLIBC version compatibility
   - **Solution**: Build from source or use npm package
