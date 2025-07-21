# WindRiver Studio Mock Server

A comprehensive mock server implementation using WireMock to simulate the WindRiver Studio APIs for testing the MCP server integration.

## Quick Start

### Prerequisites
- Docker and Docker Compose
- Working WindRiver Studio CLI installation

### Starting the Mock Server

```bash
cd mock-studio-server
docker-compose up -d
```

This will start:
- **Mock Server**: `http://localhost:8080` - Main API endpoints
- **Admin UI**: `http://localhost:8081` - WireMock admin interface for debugging

### Testing the Integration

1. **Start Mock Server**:
   ```bash
   docker-compose up -d
   ```

2. **Configure MCP Server**:
   ```bash
   cp ../config-mock.json ../config.json
   ```

3. **Test Authentication**:
   ```bash
   curl -X POST http://localhost:8080/api/auth/token \
     -H "Content-Type: application/json" \
     -d '{"username": "admin", "password": "password"}'
   ```

4. **Run MCP Server**:
   ```bash
   cd ..
   ./target/release/studio-mcp-server
   ```

5. **Test with MCP Inspector**:
   ```bash
   npx @modelcontextprotocol/inspector ./target/release/studio-mcp-server --cli
   ```

## Mock Server Features

### Authentication APIs
- `POST /api/auth/token` - Login with username/password
- `POST /api/auth/refresh` - Refresh access token
- `POST /api/auth/revoke` - Revoke token
- `GET /.well-known/jwks.json` - JWT public keys
- `GET /api/health` - Health check

### PLM (Pipeline Management) APIs
- `GET /api/plm/pipelines` - List pipelines
- `GET /api/plm/pipelines/{id}` - Get pipeline definition (YAML)
- `POST /api/plm/runs` - Start pipeline run
- `DELETE /api/plm/runs/{id}` - Cancel pipeline run
- `GET /api/plm/runs` - List pipeline runs
- `GET /api/plm/runs/{id}` - Get run details
- `GET /api/plm/runs/{id}/logs` - Get run logs (with error data for testing)
- `GET /api/plm/runs/{id}/events` - Get run events
- `GET /api/plm/tasks` - List available tasks
- `GET /api/plm/resources` - List pipeline resources

### Test Data Features

#### Realistic Pipeline Data
- 4 sample pipelines with different statuses
- Build, deploy, data processing, and security scan pipelines
- Comprehensive pipeline definitions in YAML format

#### Rich Log Data for Error Testing
- 27 log entries with various severity levels
- 4 ERROR level entries for testing error filtering
- 6 WARNING level entries for comprehensive testing
- Realistic timestamps and stage progression
- Different tasks: git-clone, npm-install, build, test, docker-build, kubectl-deploy

#### Event Tracking
- 13 pipeline events covering full lifecycle
- Error events: test_timeout, network_error, health_check_failed
- Success events: stage completion, deployment success
- Metadata for each event with contextual information

### Error Scenarios for Testing

The mock server includes realistic error conditions to test our log filtering:

1. **Network Errors**: Connection timeouts to registry
2. **Test Failures**: Test timeout scenarios
3. **Health Check Failures**: Service unavailability during deployment
4. **Build Warnings**: Bundle size warnings, deprecated packages
5. **Deployment Issues**: Pod startup delays, health check failures

## Configuration

### Test Credentials
- **Username**: `admin`
- **Password**: `password` (any password works)
- **Token**: Auto-generated JWT with appropriate scopes

### Customizing Mock Data

Edit files in `wiremock/__files/` to customize responses:
- `pipelines/` - Pipeline definitions and lists
- `runs/` - Run details, logs, and events  
- `tasks/` - Available tasks and definitions
- `resources/` - Pipeline resources and configurations
- `auth/` - Authentication responses and tokens

### Adding New Endpoints

Create new mapping files in `wiremock/mappings/`:

```json
{
  "request": {
    "method": "GET",
    "urlPattern": "/api/new-endpoint.*"
  },
  "response": {
    "status": 200,
    "bodyFileName": "new-endpoint-response.json"
  }
}
```

## Debugging

### View Admin Interface
Open `http://localhost:8081` to see:
- Request logs
- Response mappings
- Request/response debugging
- Scenario state management

### Check Logs
```bash
docker-compose logs -f wiremock
```

### Reset State
```bash
docker-compose restart
```

## Integration Testing

### Test MCP Tools
```bash
# Test pipeline listing
echo '{"name": "plm_list_pipelines", "arguments": {}}' | mcp-client

# Test pipeline error analysis
echo '{"name": "plm_get_pipeline_errors", "arguments": {"pipeline_name": "build-api-service"}}' | mcp-client

# Test log filtering
echo '{"name": "plm_get_run_log", "arguments": {"run_id": "run-abc123", "errors_only": true}}' | mcp-client
```

### Test MCP Resources
```bash
# Test pipeline resources
echo '{"uri": "studio://plm/pipelines/"}' | mcp-client

# Test run details
echo '{"uri": "studio://plm/runs/run-abc123"}' | mcp-client
```

## Architecture

```
MCP Server → studio-cli → Mock Server (WireMock)
     ↓           ↓              ↓
  Resources    HTTP Calls    JSON/YAML
   & Tools      with Auth     Responses
```

The mock server enables complete end-to-end testing without requiring an actual WindRiver Studio deployment, providing realistic API responses for comprehensive MCP server validation.