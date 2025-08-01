# Installation Guide

This guide covers all methods to install and run the WindRiver Studio MCP Server.

## Quick Start (Recommended)

### npm Installation

The easiest way to get started:

```bash
# Install globally
npm install -g @pulseengine/studio-mcp-server@latest

# Or run directly
npx @pulseengine/studio-mcp-server@latest config.json
```

### Platform Support

Pre-built binaries are available for:
- **macOS**: Intel (x64) and Apple Silicon (arm64)
- **Linux**: x64 
- **Windows**: x64

## Advanced Installation

### Building from Source

#### Prerequisites

- **Rust**: 1.70 or later with Cargo
- **Git**: For cloning the repository
- **Studio Access**: Valid WindRiver Studio credentials

#### Build Steps

```bash
# Clone repository
git clone https://github.com/pulseengine/studio-mcp.git
cd studio-mcp

# Build release version
cargo build --release

# Binary location
./target/release/studio-mcp-server
```

#### Development Build

```bash
# For development with debug symbols
cargo build

# Run directly
cargo run -- config.json
```

### Docker Installation

```dockerfile
FROM rust:1.70 as builder
WORKDIR /usr/src/app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
COPY --from=builder /usr/src/app/target/release/studio-mcp-server /usr/local/bin/
CMD ["studio-mcp-server"]
```

## Verification

Verify your installation:

```bash
# Check version
studio-mcp-server --version

# Test configuration
studio-mcp-server --check-config config.json

# Health check
studio-mcp-server --health
```

## Next Steps

1. [Configure the server](configuration.md)
2. [Set up MCP client integration](../examples/claude-desktop.md)
3. [Test with example workflows](../examples/)

## Troubleshooting

See the [troubleshooting guide](troubleshooting.md) for common installation issues.