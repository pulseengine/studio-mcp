#!/usr/bin/env python3

"""
Integration test client for WindRiver Studio MCP Server.
Tests the complete flow: MCP Client → MCP Server → studio-cli → Mock Server
"""

import json
import subprocess
import time
import os
import sys
import signal
import requests
from typing import Dict, Any, Optional
import tempfile

# Test configuration
PROJECT_ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "../.."))
MCP_SERVER = os.path.join(PROJECT_ROOT, "target/release/studio-mcp-server")
MOCK_SERVER_DIR = os.path.join(PROJECT_ROOT, "mock-studio-server")

class Colors:
    RED = '\033[0;31m'
    GREEN = '\033[0;32m'
    YELLOW = '\033[1;33m'
    NC = '\033[0m'  # No Color

class MCPClient:
    """Simple MCP client for testing"""
    
    def __init__(self, server_path: str, config_path: str):
        self.server_path = server_path
        self.config_path = config_path
        self.process = None
        self.request_id = 1
        
    def start(self):
        """Start the MCP server process"""
        try:
            self.process = subprocess.Popen(
                [self.server_path, self.config_path],
                stdin=subprocess.PIPE,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True,
                bufsize=0
            )
            time.sleep(2)  # Let server start
            return True
        except Exception as e:
            print(f"{Colors.RED}Failed to start MCP server: {e}{Colors.NC}")
            return False
    
    def stop(self):
        """Stop the MCP server process"""
        if self.process:
            self.process.terminate()
            try:
                self.process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                self.process.kill()
            self.process = None
    
    def send_request(self, method: str, params: Optional[Dict[str, Any]] = None) -> Dict[str, Any]:
        """Send a JSON-RPC request to the MCP server"""
        if not self.process:
            return {"error": "Server not started"}
        
        request = {
            "jsonrpc": "2.0",
            "method": method,
            "id": self.request_id
        }
        
        if params is not None:
            request["params"] = params
        
        self.request_id += 1
        
        try:
            # Send request
            request_line = json.dumps(request) + "\n"
            self.process.stdin.write(request_line)
            self.process.stdin.flush()
            
            # Read response
            response_line = self.process.stdout.readline()
            if not response_line:
                return {"error": "No response from server"}
            
            return json.loads(response_line.strip())
        except Exception as e:
            return {"error": f"Communication error: {e}"}

class MockServerManager:
    """Manages the WireMock mock server"""
    
    def __init__(self, mock_dir: str):
        self.mock_dir = mock_dir
        
    def start(self):
        """Start the mock server using docker-compose"""
        try:
            subprocess.run(
                ["docker-compose", "up", "-d"],
                cwd=self.mock_dir,
                check=True,
                capture_output=True
            )
            time.sleep(5)  # Wait for startup
            return self.health_check()
        except subprocess.CalledProcessError as e:
            print(f"{Colors.RED}Failed to start mock server: {e}{Colors.NC}")
            return False
    
    def stop(self):
        """Stop the mock server"""
        try:
            subprocess.run(
                ["docker-compose", "down", "-v"],
                cwd=self.mock_dir,
                check=True,
                capture_output=True
            )
        except subprocess.CalledProcessError:
            pass  # Ignore errors during cleanup
    
    def health_check(self) -> bool:
        """Check if mock server is healthy"""
        try:
            response = requests.get("http://localhost:8080/api/health", timeout=10)
            return response.status_code == 200 and response.json().get("status") == "healthy"
        except Exception:
            return False

class IntegrationTester:
    """Main integration test runner"""
    
    def __init__(self):
        self.mock_manager = MockServerManager(MOCK_SERVER_DIR)
        self.mcp_client = None
        self.test_config_path = None
        self.results = {}
        
    def setup(self):
        """Set up the test environment"""
        print(f"{Colors.GREEN}Setting up integration test environment...{Colors.NC}")
        
        # Create test configuration
        config = {
            "connections": {
                "test_mock": {
                    "name": "Test Mock Server",
                    "url": "http://localhost:8080",
                    "username": "admin",
                    "token": None
                }
            },
            "default_connection": "test_mock",
            "cli": {
                "download_base_url": "https://distro.windriver.com/dist/wrstudio/wrstudio-cli-distro-cd",
                "version": "auto",
                "install_dir": None,
                "timeout": 300,
                "timeouts": {
                    "quick_operations": 10,
                    "medium_operations": 30,
                    "long_operations": 60,
                    "network_requests": 10
                },
                "auto_update": False,
                "update_check_interval": 24
            },
            "cache": {
                "enabled": True,
                "ttl": 300,
                "max_size": 1000
            },
            "logging": {
                "level": "debug",
                "format": "pretty",
                "file_logging": False,
                "log_file": None
            }
        }
        
        # Write config to temporary file
        with tempfile.NamedTemporaryFile(mode='w', suffix='.json', delete=False) as f:
            json.dump(config, f, indent=2)
            self.test_config_path = f.name
        
        # Check prerequisites
        if not os.path.exists(MCP_SERVER):
            print(f"{Colors.RED}MCP server binary not found at {MCP_SERVER}{Colors.NC}")
            print(f"{Colors.RED}Run 'cargo build --release' first{Colors.NC}")
            return False
        
        # Start mock server
        print("Starting mock server...")
        if not self.mock_manager.start():
            print(f"{Colors.RED}Failed to start mock server{Colors.NC}")
            return False
        
        # Start MCP server
        print("Starting MCP server...")
        self.mcp_client = MCPClient(MCP_SERVER, self.test_config_path)
        if not self.mcp_client.start():
            return False
        
        return True
    
    def teardown(self):
        """Clean up test environment"""
        print(f"{Colors.YELLOW}Cleaning up...{Colors.NC}")
        
        if self.mcp_client:
            self.mcp_client.stop()
        
        self.mock_manager.stop()
        
        if self.test_config_path and os.path.exists(self.test_config_path):
            os.unlink(self.test_config_path)
    
    def run_test(self, test_name: str, test_func) -> bool:
        """Run a single test and record results"""
        print(f"Testing {test_name}...")
        try:
            result = test_func()
            if result:
                print(f"{Colors.GREEN}✅ {test_name} passed{Colors.NC}")
                self.results[test_name] = "PASS"
                return True
            else:
                print(f"{Colors.RED}❌ {test_name} failed{Colors.NC}")
                self.results[test_name] = "FAIL"
                return False
        except Exception as e:
            print(f"{Colors.RED}❌ {test_name} failed with exception: {e}{Colors.NC}")
            self.results[test_name] = f"ERROR: {e}"
            return False
    
    def test_mock_server_health(self) -> bool:
        """Test mock server health endpoint"""
        return self.mock_manager.health_check()
    
    def test_mock_server_auth(self) -> bool:
        """Test mock server authentication"""
        try:
            response = requests.post(
                "http://localhost:8080/api/auth/token",
                json={"username": "admin", "password": "password"},
                timeout=10
            )
            return response.status_code == 200 and "access_token" in response.json()
        except Exception:
            return False
    
    def test_mock_server_pipelines(self) -> bool:
        """Test mock server pipelines endpoint"""
        try:
            response = requests.get("http://localhost:8080/api/plm/pipelines", timeout=10)
            return response.status_code == 200 and len(response.json()) > 0
        except Exception:
            return False
    
    def test_mcp_server_initialization(self) -> bool:
        """Test MCP server initialization"""
        response = self.mcp_client.send_request("initialize", {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {
                "name": "integration-test",
                "version": "1.0.0"
            }
        })
        return "error" not in response and "result" in response
    
    def test_mcp_tools_list(self) -> bool:
        """Test MCP tools list"""
        response = self.mcp_client.send_request("tools/list")
        if "error" in response:
            print(f"Tools list error: {response['error']}")
            return False
        
        tools = response.get("result", {}).get("tools", [])
        print(f"Found {len(tools)} tools")
        
        # Check for expected PLM tools
        tool_names = [tool["name"] for tool in tools]
        expected_tools = [
            "plm_list_pipelines",
            "plm_resolve_run_id",
            "plm_get_run_log"
        ]
        
        for tool in expected_tools:
            if tool not in tool_names:
                print(f"Missing expected tool: {tool}")
                return False
        
        return len(tools) > 0
    
    def test_mcp_resources_list(self) -> bool:
        """Test MCP resources list"""
        response = self.mcp_client.send_request("resources/list")
        if "error" in response:
            print(f"Resources list error: {response['error']}")
            return False
        
        resources = response.get("result", {}).get("resources", [])
        print(f"Found {len(resources)} resources")
        return len(resources) > 0
    
    def test_plm_list_pipelines(self) -> bool:
        """Test PLM list pipelines tool"""
        response = self.mcp_client.send_request("tools/call", {
            "name": "plm_list_pipelines",
            "arguments": {}
        })
        
        if "error" in response:
            print(f"PLM list pipelines error: {response['error']}")
            return False
        
        # Parse the tool response
        content = response.get("result", {}).get("content", [])
        if not content:
            return False
        
        try:
            tool_result = json.loads(content[0].get("text", "{}"))
            success = tool_result.get("success", False)
            if success:
                pipelines = tool_result.get("data", [])
                print(f"Found {len(pipelines)} pipelines")
                return len(pipelines) > 0
        except json.JSONDecodeError:
            pass
        
        return False
    
    def test_plm_resolve_run_id(self) -> bool:
        """Test PLM resolve run ID tool"""
        response = self.mcp_client.send_request("tools/call", {
            "name": "plm_resolve_run_id",
            "arguments": {
                "pipeline_name": "build-api-service",
                "run_number": 1
            }
        })
        
        if "error" in response:
            print(f"PLM resolve run ID error: {response['error']}")
            return False
        
        # Parse the tool response
        content = response.get("result", {}).get("content", [])
        if not content:
            return False
        
        try:
            tool_result = json.loads(content[0].get("text", "{}"))
            success = tool_result.get("success", False)
            if success:
                run_id = tool_result.get("run_id")
                print(f"Resolved run ID: {run_id}")
                return run_id is not None
        except json.JSONDecodeError:
            pass
        
        return False
    
    def test_plm_get_run_logs_with_name(self) -> bool:
        """Test PLM get run logs using pipeline name and run number"""
        response = self.mcp_client.send_request("tools/call", {
            "name": "plm_get_run_log",
            "arguments": {
                "pipeline_name": "build-api-service",
                "run_number": 1,
                "errors_only": True
            }
        })
        
        if "error" in response:
            print(f"PLM get run logs error: {response['error']}")
            return False
        
        # Parse the tool response
        content = response.get("result", {}).get("content", [])
        if not content:
            return False
        
        try:
            tool_result = json.loads(content[0].get("text", "{}"))
            success = tool_result.get("success", False)
            if success:
                logs = tool_result.get("logs", [])
                print(f"Retrieved {len(logs)} log entries")
                return True
        except json.JSONDecodeError:
            pass
        
        return False
    
    def run_all_tests(self):
        """Run all integration tests"""
        print(f"{Colors.GREEN}Starting WindRiver Studio MCP Integration Tests{Colors.NC}")
        
        if not self.setup():
            print(f"{Colors.RED}Failed to set up test environment{Colors.NC}")
            return False
        
        try:
            # Test mock server
            tests = [
                ("Mock Server Health", self.test_mock_server_health),
                ("Mock Server Auth", self.test_mock_server_auth),
                ("Mock Server Pipelines", self.test_mock_server_pipelines),
                
                # Test MCP server
                ("MCP Server Initialization", self.test_mcp_server_initialization),
                ("MCP Tools List", self.test_mcp_tools_list),
                ("MCP Resources List", self.test_mcp_resources_list),
                
                # Test PLM tools
                ("PLM List Pipelines", self.test_plm_list_pipelines),
                ("PLM Resolve Run ID", self.test_plm_resolve_run_id),
                ("PLM Get Run Logs with Name", self.test_plm_get_run_logs_with_name),
            ]
            
            passed = 0
            total = len(tests)
            
            for test_name, test_func in tests:
                if self.run_test(test_name, test_func):
                    passed += 1
            
            # Print summary
            print(f"\n{Colors.GREEN}Integration Test Summary:{Colors.NC}")
            print(f"Passed: {passed}/{total}")
            
            if passed == total:
                print(f"{Colors.GREEN}✅ All tests passed!{Colors.NC}")
                return True
            else:
                print(f"{Colors.RED}❌ {total - passed} test(s) failed{Colors.NC}")
                return False
        
        finally:
            self.teardown()

def main():
    """Main entry point"""
    tester = IntegrationTester()
    success = tester.run_all_tests()
    sys.exit(0 if success else 1)

if __name__ == "__main__":
    main()