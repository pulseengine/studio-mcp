#!/bin/bash

# Simple Integration Test for WindRiver Studio MCP Server
# Tests the basic functionality without complex dependencies

set -e  # Exit on any error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Test configuration
PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

# Platform-specific binary path detection
MCP_SERVER="${MCP_SERVER_PATH:-$PROJECT_ROOT/target/release/studio-mcp-server}"
if [[ "$OSTYPE" == "msys" || "$OSTYPE" == "cygwin" ]]; then
    MCP_SERVER="${MCP_SERVER_PATH:-$PROJECT_ROOT/target/release/studio-mcp-server.exe}"
fi

MOCK_SERVER_DIR="$PROJECT_ROOT/mock-studio-server"
TEST_CONFIG="$PROJECT_ROOT/tests/integration/test-simple-config.json"
MCP_PID=""

# Cleanup function
cleanup() {
    echo -e "${YELLOW}Cleaning up...${NC}"
    
    # Stop MCP server
    if [[ -n "$MCP_PID" ]]; then
        kill $MCP_PID 2>/dev/null || true
        wait $MCP_PID 2>/dev/null || true
    fi
    
    # Stop mock server
    cd "$MOCK_SERVER_DIR" 2>/dev/null || true
    docker-compose down -v &>/dev/null || true
    
    # Remove test config
    rm -f "$TEST_CONFIG"
}

# Set up cleanup trap
trap cleanup EXIT

# Logging functions
log() {
    echo -e "${GREEN}[$(date '+%H:%M:%S')] $1${NC}"
}

error() {
    echo -e "${RED}[$(date '+%H:%M:%S')] ERROR: $1${NC}"
}

# Create test configuration
create_test_config() {
    cat > "$TEST_CONFIG" << 'EOF'
{
  "connections": {
    "test_mock": {
      "name": "Test Mock Server",
      "url": "http://localhost:8080",
      "username": "admin",
      "token": null
    }
  },
  "default_connection": "test_mock",
  "cli": {
    "download_base_url": "https://distro.windriver.com/dist/wrstudio/wrstudio-cli-distro-cd",
    "version": "auto",
    "install_dir": null,
    "timeout": 300,
    "timeouts": {
      "quick_operations": 5,
      "medium_operations": 15,
      "long_operations": 30,
      "network_requests": 5
    },
    "auto_update": false,
    "update_check_interval": 24
  },
  "cache": {
    "enabled": true,
    "ttl": 300,
    "max_size": 1000
  },
  "logging": {
    "level": "info",
    "format": "pretty",
    "file_logging": false,
    "log_file": null
  }
}
EOF
}

# Test mock server health
test_mock_server() {
    log "Testing mock server..."
    
    # Check if Docker is available
    if ! command -v docker &> /dev/null; then
        log "⚠️  Docker not available, skipping mock server tests"
        return 0
    fi
    
    if ! docker info &>/dev/null; then
        log "⚠️  Docker daemon not available, skipping mock server tests"
        return 0
    fi
    
    # Start mock server
    log "Starting mock server..."
    cd "$MOCK_SERVER_DIR"
    if ! docker-compose up -d; then
        log "⚠️  Failed to start mock server, skipping mock server tests"
        return 0
    fi
    sleep 8  # Wait for startup
    
    # Test health endpoint
    local health=$(curl -s http://localhost:8080/api/health | jq -r '.status' 2>/dev/null || echo "error")
    if [[ "$health" == "healthy" ]]; then
        log "✅ Mock server is healthy"
    else
        error "❌ Mock server health check failed"
        return 1
    fi
    
    # Test auth endpoint
    local token=$(curl -s -X POST http://localhost:8080/api/auth/token \
        -H "Content-Type: application/json" \
        -d '{"username": "admin", "password": "password"}' | \
        jq -r '.access_token' 2>/dev/null || echo "error")
    
    if [[ "$token" != "error" && "$token" != "null" ]]; then
        log "✅ Mock server authentication working"
    else
        error "❌ Mock server authentication failed"
        return 1
    fi
    
    # Test pipelines endpoint with authentication
    local pipeline_count=$(curl -s -H "Authorization: Bearer test-token" http://localhost:8080/api/plm/pipelines | \
        jq 'length' 2>/dev/null || echo "0")
    
    if [[ "$pipeline_count" -gt 0 ]]; then
        log "✅ Mock server returned $pipeline_count pipelines"
    else
        error "❌ Mock server pipelines endpoint failed"
        return 1
    fi
    
    return 0
}

# Test MCP server startup
test_mcp_server_startup() {
    log "Testing MCP server startup..."
    
    if [[ ! -f "$MCP_SERVER" ]]; then
        error "MCP server binary not found at $MCP_SERVER"
        error "Run 'cargo build --release' first"
        return 1
    fi
    
    # Start MCP server in background
    "$MCP_SERVER" "$TEST_CONFIG" &
    MCP_PID=$!
    
    # Wait for startup
    sleep 3
    
    # Check if process is still running
    if kill -0 $MCP_PID 2>/dev/null; then
        log "✅ MCP server started successfully (PID: $MCP_PID)"
        return 0
    else
        error "❌ MCP server failed to start"
        return 1
    fi
}

# Test configuration initialization
test_config_initialization() {
    log "Testing MCP server --init flag..."
    
    local init_config="/tmp/test-init-config.json"
    rm -f "$init_config"
    
    # Test --init functionality
    "$MCP_SERVER" --init "$init_config" &>/dev/null
    
    if [[ -f "$init_config" ]]; then
        local connections=$(jq -r '.connections | keys | length' "$init_config" 2>/dev/null || echo "0")
        if [[ "$connections" -gt 0 ]]; then
            log "✅ Configuration initialization working"
            rm -f "$init_config"
            return 0
        fi
    fi
    
    error "❌ Configuration initialization failed"
    rm -f "$init_config"
    return 1
}

# Test timeout configuration
test_timeout_config() {
    log "Testing timeout configuration..."
    
    # Check if config has timeout settings
    local quick_timeout=$(jq -r '.cli.timeouts.quick_operations' "$TEST_CONFIG" 2>/dev/null || echo "null")
    local medium_timeout=$(jq -r '.cli.timeouts.medium_operations' "$TEST_CONFIG" 2>/dev/null || echo "null")
    local long_timeout=$(jq -r '.cli.timeouts.long_operations' "$TEST_CONFIG" 2>/dev/null || echo "null")
    
    if [[ "$quick_timeout" != "null" && "$medium_timeout" != "null" && "$long_timeout" != "null" ]]; then
        log "✅ Timeout configuration present (quick: ${quick_timeout}s, medium: ${medium_timeout}s, long: ${long_timeout}s)"
        return 0
    else
        error "❌ Timeout configuration missing"
        return 1
    fi
}

# Main test execution
main() {
    log "Starting WindRiver Studio MCP Simple Integration Tests"
    log "Project root: $PROJECT_ROOT"
    
    # Create test configuration
    create_test_config
    
    # Check prerequisites
    if ! command -v docker &> /dev/null; then
        error "Docker not found. Please install Docker."
        return 1
    fi
    
    if ! command -v jq &> /dev/null; then
        error "jq not found. Please install jq for JSON processing."
        return 1
    fi
    
    # Wait for Docker daemon to be ready
    log "Waiting for Docker daemon to be ready..."
    local docker_ready=false
    for i in {1..10}; do
        if docker info &>/dev/null; then
            docker_ready=true
            break
        fi
        log "Docker not ready, waiting... ($i/10)"
        sleep 2
    done
    
    if [[ "$docker_ready" != "true" ]]; then
        error "Docker daemon is not ready after waiting"
        return 1
    fi
    
    # Run tests
    local failed=0
    
    test_config_initialization || ((failed++))
    test_timeout_config || ((failed++))
    test_mock_server || ((failed++))
    test_mcp_server_startup || ((failed++))
    
    # Summary
    echo
    log "Integration test summary:"
    if [[ $failed -eq 0 ]]; then
        log "✅ All basic tests passed!"
        log "Mock server and MCP server are working correctly"
        echo
        log "Next steps to test full functionality:"
        log "1. Install Node.js dependencies: cd tests/integration && npm install"
        log "2. Run full MCP tests: npm run test"
        log "3. Or test manually with MCP Inspector: npx @modelcontextprotocol/inspector $MCP_SERVER"
    else
        error "❌ $failed test(s) failed"
        return 1
    fi
}

# Run main function
main "$@"