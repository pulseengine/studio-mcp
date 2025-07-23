#!/bin/bash

# Minimal Integration Test - Docker-free validation
# Tests basic MCP server functionality without requiring mock server

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

# Test configuration
PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
MCP_BINARY_PATH="${MCP_SERVER_PATH:-$PROJECT_ROOT/target/release/studio-mcp-server}"

# Platform-specific binary path detection
if [[ "$OSTYPE" == "msys" || "$OSTYPE" == "cygwin" ]]; then
    MCP_BINARY_PATH="${MCP_SERVER_PATH:-$PROJECT_ROOT/target/release/studio-mcp-server.exe}"
fi

# Logging functions
log() {
    echo -e "${GREEN}[$(date '+%H:%M:%S')] $1${NC}"
}

error() {
    echo -e "${RED}[$(date '+%H:%M:%S')] ERROR: $1${NC}"
}

# Test binary exists
test_binary_exists() {
    log "Testing MCP server binary exists..."
    
    if [[ -f "$MCP_BINARY_PATH" ]]; then
        log "✅ MCP server binary found at $MCP_BINARY_PATH"
        return 0
    else
        error "❌ MCP server binary not found at $MCP_BINARY_PATH"
        return 1
    fi
}

# Test configuration initialization
test_config_initialization() {
    log "Testing configuration initialization..."
    
    local temp_config="/tmp/test-minimal-config-$$.json"
    
    if "$MCP_BINARY_PATH" --init "$temp_config" &>/dev/null; then
        if [[ -f "$temp_config" ]]; then
            log "✅ Configuration initialization working"
            rm -f "$temp_config"
            return 0
        else
            error "❌ Configuration file not created"
            return 1
        fi
    else
        error "❌ Configuration initialization failed"
        return 1
    fi
}

# Test binary basic functionality
test_basic_functionality() {
    log "Testing basic MCP server functionality..."
    
    # Create a temporary config for testing
    local temp_config="/tmp/test-basic-config-$$.json"
    "$MCP_BINARY_PATH" --init "$temp_config" &>/dev/null
    
    # Test that the server binary can be executed (it will wait for stdio input)
    # We'll start it in background and kill it quickly
    "$MCP_BINARY_PATH" "$temp_config" &
    local server_pid=$!
    sleep 1
    
    # Check if process is still running (means it started successfully)
    if kill -0 "$server_pid" 2>/dev/null; then
        log "✅ Basic MCP server functionality working"
        kill "$server_pid" 2>/dev/null
        wait "$server_pid" 2>/dev/null || true
        rm -f "$temp_config"
        return 0
    else
        error "❌ Basic MCP server functionality failed"
        rm -f "$temp_config"
        return 1
    fi
}

# Test version check (if available)
test_version_check() {
    log "Testing version check..."
    
    if "$MCP_BINARY_PATH" --version &>/dev/null; then
        log "✅ Version check working"
        return 0
    else
        log "⚠️  Version check not available (this is okay)"
        return 0
    fi
}

# Main test execution
main() {
    log "Starting Minimal MCP Integration Test"
    log "Binary path: $MCP_BINARY_PATH"
    
    local failed=0
    
    test_binary_exists || ((failed++))
    test_config_initialization || ((failed++))
    test_basic_functionality || ((failed++))
    test_version_check || ((failed++))
    
    echo
    if [[ $failed -eq 0 ]]; then
        log "✅ All minimal tests passed!"
        log "MCP server basic functionality is working"
        return 0
    else
        error "❌ $failed test(s) failed"
        return 1
    fi
}

# Run main function
main "$@"