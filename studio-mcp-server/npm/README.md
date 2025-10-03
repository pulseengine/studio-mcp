# @pulseengine/studio-mcp-server

A Model Context Protocol (MCP) server for WindRiver Studio CLI integration, packaged for npm/npx usage.

## 🚀 Quick Start with npx (No Installation Required)

```bash
# Run directly with npx
npx @pulseengine/studio-mcp-server --help

# Start MCP server
npx @pulseengine/studio-mcp-server --port 8080

# Use with specific Studio instance
npx @pulseengine/studio-mcp-server --instance my-studio --user developer
```

## 📦 Global Installation

```bash
# Install globally
npm install -g @pulseengine/studio-mcp-server

# Then run directly
studio-mcp-server --help
studio-mcp # Short alias
```

## 🔧 MCP Client Configuration

Add to your MCP client configuration (e.g., Claude Desktop):

### Using npx (Recommended)
```json
{
  "mcpServers": {
    "studio": {
      "command": "npx",
      "args": ["@pulseengine/studio-mcp-server", "--instance", "your-studio-instance"]
    }
  }
}
```

### Using global installation
```json
{
  "mcpServers": {
    "studio": {
      "command": "studio-mcp-server",
      "args": ["--instance", "your-studio-instance"]
    }
  }
}
```

## ⚡ Features

- **Zero-config MCP server** for WindRiver Studio
- **Intelligent caching** with multi-tier TTL policies
- **User context isolation** for secure multi-tenant usage
- **PLM integration** with pipeline, run, and resource management
- **Cross-platform support** (Windows, macOS, Linux)
- **Pre-compiled binaries** - no Rust installation required

## 🛠️ Development

This npm package wraps a Rust binary. For development:

```bash
git clone https://github.com/pulseengine/studio-mcp.git
cd studio-mcp/studio-mcp-server
cargo build --release
```

## 📋 Requirements

- Node.js >= 14
- One of: Windows (x64), macOS (x64/ARM64), Linux (x64/ARM64)

## 🐛 Troubleshooting

If installation fails:

1. **Manual installation from source:**
   ```bash
   git clone https://github.com/pulseengine/studio-mcp.git
   cd studio-mcp/studio-mcp-server
   cargo install --path .
   ```

2. **Download binary manually:**
   - Visit: https://github.com/pulseengine/studio-mcp/releases
   - Download for your platform and add to PATH

## 📄 License

MIT License - see the [LICENSE](https://github.com/pulseengine/studio-mcp/blob/main/LICENSE) file for details.
