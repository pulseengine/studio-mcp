#!/bin/bash

# Run all integration tests for WindRiver Studio MCP Server
# This script executes all available tests and provides a summary

set -e

# Ensure we have bash 4+ for associative arrays
if [[ ${BASH_VERSION%%.*} -lt 4 ]]; then
    echo "This script requires bash 4.0 or later for associative arrays"
    echo "Current version: $BASH_VERSION"
    exit 1
fi

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

log() {
    echo -e "${GREEN}[$(date '+%H:%M:%S')] $1${NC}"
}

error() {
    echo -e "${RED}[$(date '+%H:%M:%S')] ERROR: $1${NC}"
}

info() {
    echo -e "${BLUE}[$(date '+%H:%M:%S')] INFO: $1${NC}"
}

echo -e "${BLUE}"
echo "================================================="
echo "  WindRiver Studio MCP Integration Test Suite   "
echo "================================================="
echo -e "${NC}"

# Check prerequisites
log "Checking prerequisites..."

# Check for Windows binary path from environment variable or detect platform
MCP_BINARY_PATH="${MCP_SERVER_PATH:-target/release/studio-mcp-server}"

# Platform-specific binary detection
if [[ "$OSTYPE" == "msys" || "$OSTYPE" == "cygwin" ]]; then
    MCP_BINARY_PATH="${MCP_SERVER_PATH:-target/release/studio-mcp-server.exe}"
fi

if [[ ! -f "$MCP_BINARY_PATH" ]]; then
    error "MCP server binary not found at $MCP_BINARY_PATH. Building..."
    cargo build --release
    
    # Re-check after build
    if [[ ! -f "$MCP_BINARY_PATH" ]]; then
        error "Failed to build MCP server binary at $MCP_BINARY_PATH"
        exit 1
    fi
fi

if ! command -v docker &> /dev/null; then
    error "Docker not found. Please install Docker."
    exit 1
fi

if ! command -v jq &> /dev/null; then
    error "jq not found. Please install jq."
    exit 1
fi

log "✅ Prerequisites checked"

# Test results tracking
declare -A test_results
total_tests=0
passed_tests=0

run_test() {
    local test_name="$1"
    local test_command="$2"
    
    info "Running: $test_name"
    ((total_tests++))
    
    if eval "$test_command" &>/dev/null; then
        log "✅ $test_name PASSED"
        test_results["$test_name"]="PASS"
        ((passed_tests++))
        return 0
    else
        error "❌ $test_name FAILED"
        test_results["$test_name"]="FAIL"
        return 1
    fi
}

# Run tests
echo
log "Running integration tests..."
echo

# 1. Simple Integration Test
run_test "Simple Integration Test" "cd tests/integration && ./simple_integration_test.sh"

# 2. Configuration Tests
run_test "Configuration Init Test" "\"$MCP_BINARY_PATH\" --init /tmp/test-config-$$.json && rm -f /tmp/test-config-$$.json"

# 3. MCP Inspector Test (if Node.js is available)
if command -v node &> /dev/null; then
    run_test "MCP Inspector Compatibility" "echo 'q' | npx --yes @modelcontextprotocol/inspector \"$MCP_BINARY_PATH\" config.json --stdio 2>/dev/null"
fi

# 4. Mock Server Standalone Test
run_test "Mock Server Standalone" "cd mock-studio-server && docker-compose up -d && sleep 5 && curl -s -H 'Authorization: Bearer test' http://localhost:8080/api/plm/pipelines | jq length > /dev/null && docker-compose down"

echo
echo -e "${BLUE}================================================="
echo "               TEST RESULTS SUMMARY              "
echo -e "=================================================${NC}"
echo
echo "Total tests: $total_tests"
echo "Passed: $passed_tests"
echo "Failed: $((total_tests - passed_tests))"
echo

# Detailed results
for test_name in "${!test_results[@]}"; do
    result="${test_results[$test_name]}"
    if [[ "$result" == "PASS" ]]; then
        echo -e "${GREEN}✅ $test_name${NC}"
    else
        echo -e "${RED}❌ $test_name${NC}"
    fi
done

echo
if [[ $passed_tests -eq $total_tests ]]; then
    log "🎉 ALL TESTS PASSED! The MCP server is ready for use."
    echo
    echo "Quick start guide:"
    echo "1. Start mock server: cd mock-studio-server && docker-compose up -d"
    echo "2. Create config: ./target/release/studio-mcp-server --init"
    echo "3. Start MCP server: ./target/release/studio-mcp-server config.json"
    echo "4. Test with Inspector: npx @modelcontextprotocol/inspector target/release/studio-mcp-server config.json"
    echo
else
    error "Some tests failed. Check the logs above for details."
    echo
    echo "Debug steps:"
    echo "1. Check Docker is running: docker ps"
    echo "2. Build server: cargo build --release"
    echo "3. Check logs in tests/integration/"
    echo "4. Run individual tests manually"
    exit 1
fi