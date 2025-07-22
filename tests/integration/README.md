# WindRiver Studio MCP Integration Tests

This directory contains integration tests for the WindRiver Studio MCP Server. The tests validate the complete flow: MCP Client → MCP Server → studio-cli → Mock Server.

## Test Files

### 1. Simple Integration Test (`simple_integration_test.sh`)
Basic integration test that validates:
- ✅ MCP server `--init` configuration flag
- ✅ Configurable timeout settings
- ✅ Mock server health, authentication, and pipelines endpoints
- ✅ MCP server startup and basic functionality

**Usage:**
```bash
./simple_integration_test.sh
```

### 2. Python Integration Test (`test_mcp_client.py`)
Comprehensive test using a custom MCP client that validates:
- Mock server functionality
- MCP server JSON-RPC protocol compliance
- PLM tools functionality
- Pipeline name/run number to ID resolution

**Requirements:**
```bash
python3 -m pip install requests  # or use --user flag
```

### 3. Node.js Integration Test (`test_mcp_official.js`)
Official MCP SDK-based test that validates:
- MCP protocol compliance using official SDK
- All MCP tools and resources
- PLM pipeline management functionality

**Setup:**
```bash
npm install
npm test
```

### 4. Manual Testing with MCP Inspector
Use the official MCP Inspector for interactive testing:
```bash
npx @modelcontextprotocol/inspector target/release/studio-mcp-server config.json
```
Then visit http://127.0.0.1:6274 to interact with the MCP server.

## Test Results Summary

### ✅ Working Features
1. **Timeout Configuration**: Configurable timeouts for different operation types
2. **Configuration Initialization**: `--init` flag creates proper configuration
3. **Mock Server Integration**: WireMock server provides realistic API responses
4. **MCP Protocol Compliance**: Server follows MCP 2024-11-05 specification
5. **Pipeline Name/Run ID Resolution**: Convert pipeline name + run number to run ID
6. **PLM Tools**: List pipelines, resolve IDs, get logs with error filtering
7. **Authentication Simulation**: Mock server requires proper authorization headers

### 🔧 Tools Verified
- `plm_list_pipelines`: Lists available pipelines
- `plm_resolve_run_id`: Converts pipeline name + run number to ID
- `plm_get_run`: Gets run details by ID or name/number
- `plm_get_run_log`: Gets logs with error filtering and name/number support
- `plm_get_run_events`: Gets events by ID or name/number
- `plm_start_pipeline`: Starts pipeline runs
- `plm_cancel_run`: Cancels running pipelines

### 📊 Mock Data Available
- **4 Pipelines**: build-api-service, deploy-frontend, data-processing, security-scan
- **Run Logs**: 27 entries with 4 ERROR and 6 WARNING levels for testing filters
- **Events**: 13 pipeline events including error scenarios
- **Resources**: 6 pipeline resources (registries, clusters, storage, notifications)
- **Tasks**: 5 task types (git-clone, npm-build, docker-build, kubectl-deploy, security-scan)

## Running All Tests

```bash
# Simple test (recommended first)
./simple_integration_test.sh

# Python test (requires requests module)
python3 test_mcp_client.py

# Node.js test (requires npm install)
npm test

# Interactive testing
npx @modelcontextprotocol/inspector target/release/studio-mcp-server config.json
```

## What Was Tested

1. **Core Infrastructure**:
   - MCP server builds and starts correctly
   - Configuration system works with timeouts and connections
   - Mock server provides realistic WindRiver Studio API simulation

2. **Protocol Compliance**:
   - MCP JSON-RPC protocol implementation
   - Tools and resources listing
   - Proper error handling and timeout behavior

3. **PLM Features**:
   - Pipeline listing and information retrieval
   - Run management with ID resolution
   - Log filtering and error analysis
   - Event tracking and monitoring

4. **User Experience Improvements**:
   - Pipeline name + run number instead of requiring run IDs
   - Configurable timeouts prevent hanging operations
   - Proper configuration initialization with helpful guidance

## Next Steps

The integration tests demonstrate that the core MCP server functionality is working correctly with the mock server. The system is ready for:

1. **Real CLI Integration**: Test with actual WindRiver Studio CLI downloads
2. **Extended PLM Features**: Add artifacts and vlab resource providers
3. **Performance Testing**: Test with larger datasets and concurrent operations
4. **Error Scenarios**: Test timeout handling and network failure recovery