#!/bin/bash

# Integration test script for WindRiver Studio MCP Server
# Tests the complete flow: MCP Server → studio-cli → Mock Server

set -e  # Exit on any error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Test configuration
PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
TEST_CONFIG="$PROJECT_ROOT/tests/integration/test-config.json"
MCP_SERVER="$PROJECT_ROOT/target/release/studio-mcp-server"
MOCK_SERVER_DIR="$PROJECT_ROOT/mock-studio-server"
TEST_RESULTS_DIR="$PROJECT_ROOT/tests/integration/results"
MOCK_SERVER_PID=""
MCP_SERVER_PID=""

# Cleanup function
cleanup() {
    echo -e "${YELLOW}Cleaning up...${NC}"
    
    # Stop MCP server
    if [[ -n "$MCP_SERVER_PID" ]]; then
        kill $MCP_SERVER_PID 2>/dev/null || true
        wait $MCP_SERVER_PID 2>/dev/null || true
    fi
    
    # Stop mock server
    if [[ -n "$MOCK_SERVER_PID" ]]; then
        cd "$MOCK_SERVER_DIR"
        docker-compose down -v &>/dev/null || true
    fi
    
    # Remove test files
    rm -f "$TEST_CONFIG"
    rm -rf "$TEST_RESULTS_DIR"
}

# Set up cleanup trap
trap cleanup EXIT

# Logging function
log() {
    echo -e "${GREEN}[$(date '+%H:%M:%S')] $1${NC}"
}

error() {
    echo -e "${RED}[$(date '+%H:%M:%S')] ERROR: $1${NC}"
}

warn() {
    echo -e "${YELLOW}[$(date '+%H:%M:%S')] WARNING: $1${NC}"
}

# Test functions
test_mock_server_health() {
    log "Testing mock server health..."
    
    local response=$(curl -s http://localhost:8080/api/health | jq -r '.status' 2>/dev/null || echo "error")
    
    if [[ "$response" == "healthy" ]]; then
        log "✅ Mock server is healthy"
        return 0
    else
        error "❌ Mock server health check failed. Response: $response"
        return 1
    fi
}

test_mock_server_auth() {
    log "Testing mock server authentication..."
    
    local response=$(curl -s -X POST http://localhost:8080/api/auth/token \
        -H "Content-Type: application/json" \
        -d '{"username": "admin", "password": "password"}' | \
        jq -r '.access_token' 2>/dev/null || echo "error")
    
    if [[ "$response" != "error" && "$response" != "null" ]]; then
        log "✅ Mock server authentication working"
        return 0
    else
        error "❌ Mock server authentication failed"
        return 1
    fi
}

test_mock_server_pipelines() {
    log "Testing mock server pipelines endpoint..."
    
    local count=$(curl -s http://localhost:8080/api/plm/pipelines | \
        jq 'length' 2>/dev/null || echo "0")
    
    if [[ "$count" -gt 0 ]]; then
        log "✅ Mock server returned $count pipelines"
        return 0
    else
        error "❌ Mock server pipelines endpoint failed"
        return 1
    fi
}

test_mcp_server_startup() {
    log "Testing MCP server startup..."
    
    # Start MCP server in background
    timeout 30s "$MCP_SERVER" "$TEST_CONFIG" > "$TEST_RESULTS_DIR/mcp_server.log" 2>&1 &
    MCP_SERVER_PID=$!
    
    # Wait for startup
    sleep 3
    
    # Check if process is still running
    if kill -0 $MCP_SERVER_PID 2>/dev/null; then
        log "✅ MCP server started successfully (PID: $MCP_SERVER_PID)"
        return 0
    else
        error "❌ MCP server failed to start"
        return 1
    fi
}

send_mcp_request() {
    local method="$1"
    local params="$2"
    local id="${3:-1}"
    
    local request="{
        \"jsonrpc\": \"2.0\",
        \"method\": \"$method\",
        \"params\": $params,
        \"id\": $id
    }"
    
    # Send request to MCP server via stdin
    echo "$request" | timeout 10s socat - EXEC:"$MCP_SERVER $TEST_CONFIG",pty 2>/dev/null || echo '{"error": "timeout"}'
}

test_mcp_tools_list() {
    log "Testing MCP tools list..."
    
    local response=$(send_mcp_request "tools/list" "{}")
    local tool_count=$(echo "$response" | jq '.result.tools | length' 2>/dev/null || echo "0")
    
    if [[ "$tool_count" -gt 0 ]]; then
        log "✅ MCP server returned $tool_count tools"
        echo "$response" > "$TEST_RESULTS_DIR/tools_list.json"
        return 0
    else
        error "❌ MCP tools list failed"
        return 1
    fi
}

test_mcp_resources_list() {
    log "Testing MCP resources list..."
    
    local response=$(send_mcp_request "resources/list" "{}")
    local resource_count=$(echo "$response" | jq '.result.resources | length' 2>/dev/null || echo "0")
    
    if [[ "$resource_count" -gt 0 ]]; then
        log "✅ MCP server returned $resource_count resources"
        echo "$response" > "$TEST_RESULTS_DIR/resources_list.json"
        return 0
    else
        error "❌ MCP resources list failed"
        return 1
    fi
}

test_plm_list_pipelines() {
    log "Testing PLM list pipelines tool..."
    
    local params='{
        "name": "plm_list_pipelines",
        "arguments": {}
    }'
    
    local response=$(send_mcp_request "tools/call" "$params")
    local success=$(echo "$response" | jq -r '.result.content[0].text' 2>/dev/null | jq -r '.success' 2>/dev/null || echo "false")
    
    if [[ "$success" == "true" ]]; then
        log "✅ PLM list pipelines tool working"
        echo "$response" > "$TEST_RESULTS_DIR/plm_list_pipelines.json"
        return 0
    else
        error "❌ PLM list pipelines tool failed"
        echo "$response" > "$TEST_RESULTS_DIR/plm_list_pipelines_error.json"
        return 1
    fi
}

test_plm_resolve_run_id() {
    log "Testing PLM resolve run ID tool..."
    
    local params='{
        "name": "plm_resolve_run_id",
        "arguments": {
            "pipeline_name": "build-api-service",
            "run_number": 1
        }
    }'
    
    local response=$(send_mcp_request "tools/call" "$params")
    local success=$(echo "$response" | jq -r '.result.content[0].text' 2>/dev/null | jq -r '.success' 2>/dev/null || echo "false")
    
    if [[ "$success" == "true" ]]; then
        log "✅ PLM resolve run ID tool working"
        echo "$response" > "$TEST_RESULTS_DIR/plm_resolve_run_id.json"
        return 0
    else
        error "❌ PLM resolve run ID tool failed"
        echo "$response" > "$TEST_RESULTS_DIR/plm_resolve_run_id_error.json"
        return 1
    fi
}

test_plm_get_run_logs() {
    log "Testing PLM get run logs with pipeline name/run number..."
    
    local params='{
        "name": "plm_get_run_log",
        "arguments": {
            "pipeline_name": "build-api-service",
            "run_number": 1,
            "errors_only": true
        }
    }'
    
    local response=$(send_mcp_request "tools/call" "$params")
    local success=$(echo "$response" | jq -r '.result.content[0].text' 2>/dev/null | jq -r '.success' 2>/dev/null || echo "false")
    
    if [[ "$success" == "true" ]]; then
        log "✅ PLM get run logs tool working"
        echo "$response" > "$TEST_RESULTS_DIR/plm_get_run_logs.json"
        return 0
    else
        error "❌ PLM get run logs tool failed"
        echo "$response" > "$TEST_RESULTS_DIR/plm_get_run_logs_error.json"
        return 1
    fi
}

# Main test execution
main() {
    log "Starting WindRiver Studio MCP Integration Tests"
    log "Project root: $PROJECT_ROOT"
    
    # Create results directory
    mkdir -p "$TEST_RESULTS_DIR"
    
    # Create test configuration
    cat > "$TEST_CONFIG" << EOF
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
      "quick_operations": 10,
      "medium_operations": 30,
      "long_operations": 60,
      "network_requests": 10
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
    "level": "debug",
    "format": "pretty",
    "file_logging": true,
    "log_file": "$TEST_RESULTS_DIR/mcp_debug.log"
  }
}
EOF
    
    # Check prerequisites
    if [[ ! -f "$MCP_SERVER" ]]; then
        error "MCP server binary not found at $MCP_SERVER"
        error "Run 'cargo build --release' first"
        exit 1
    fi
    
    if ! command -v docker &> /dev/null; then
        error "Docker not found. Please install Docker."
        exit 1
    fi
    
    if ! command -v jq &> /dev/null; then
        error "jq not found. Please install jq for JSON processing."
        exit 1
    fi
    
    # Start mock server
    log "Starting mock server..."
    cd "$MOCK_SERVER_DIR"
    docker-compose up -d
    sleep 5  # Wait for startup
    
    # Test mock server
    local failed_tests=0
    
    test_mock_server_health || ((failed_tests++))
    test_mock_server_auth || ((failed_tests++))
    test_mock_server_pipelines || ((failed_tests++))
    
    # Test MCP server
    test_mcp_server_startup || ((failed_tests++))
    
    if [[ $failed_tests -eq 0 ]]; then
        # Run MCP-specific tests
        sleep 2  # Let MCP server settle
        
        test_mcp_tools_list || ((failed_tests++))
        test_mcp_resources_list || ((failed_tests++))
        test_plm_list_pipelines || ((failed_tests++))
        test_plm_resolve_run_id || ((failed_tests++))
        test_plm_get_run_logs || ((failed_tests++))
    fi
    
    # Summary
    log "Integration test summary:"
    if [[ $failed_tests -eq 0 ]]; then
        log "✅ All tests passed!"
        log "Test results saved to: $TEST_RESULTS_DIR"
    else
        error "❌ $failed_tests test(s) failed"
        error "Check logs in: $TEST_RESULTS_DIR"
        exit 1
    fi
}

# Run main function
main "$@"