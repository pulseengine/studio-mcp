#!/usr/bin/env python3
"""
Simple test script for Studio MCP Server
Tests basic MCP protocol functionality via stdin/stdout
"""

import json
import subprocess
import sys
import time

def send_mcp_request(process, request):
    """Send MCP request and get response"""
    request_json = json.dumps(request) + "\n"
    print(f"Sending: {request_json.strip()}")
    
    process.stdin.write(request_json.encode())
    process.stdin.flush()
    
    # Read response
    response_line = process.stdout.readline()
    if response_line:
        try:
            response = json.loads(response_line.decode().strip())
            print(f"Received: {json.dumps(response, indent=2)}")
            return response
        except json.JSONDecodeError as e:
            print(f"Failed to decode response: {e}")
            print(f"Raw response: {response_line}")
            return None
    else:
        print("No response received")
        return None

def test_mcp_server():
    """Test the MCP server basic functionality"""
    server_path = "./target/release/studio-mcp-server"
    
    print("Starting Studio MCP Server test...")
    print("=" * 50)
    
    try:
        # Start the server process
        process = subprocess.Popen(
            [server_path],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=False
        )
        
        # Give server time to initialize
        time.sleep(2)
        
        # Test 1: Initialize
        print("\n1. Testing MCP initialization...")
        init_request = {
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "roots": {"listChanged": True},
                    "sampling": {}
                },
                "clientInfo": {
                    "name": "test-client",
                    "version": "1.0.0"
                }
            }
        }
        
        response = send_mcp_request(process, init_request)
        if response and "result" in response:
            print("✓ Initialization successful")
            server_info = response["result"]
            print(f"  Server: {server_info.get('serverInfo', {}).get('name', 'unknown')}")
            print(f"  Version: {server_info.get('serverInfo', {}).get('version', 'unknown')}")
        else:
            print("✗ Initialization failed")
            return False
            
        # Test 2: List Resources
        print("\n2. Testing resource listing...")
        list_resources_request = {
            "jsonrpc": "2.0", 
            "id": 2,
            "method": "resources/list",
            "params": {}
        }
        
        response = send_mcp_request(process, list_resources_request)
        if response and "result" in response:
            resources = response["result"].get("resources", [])
            print(f"✓ Found {len(resources)} resources:")
            for resource in resources[:5]:  # Show first 5
                print(f"  - {resource.get('name', 'unknown')}: {resource.get('uri', 'no uri')}")
            if len(resources) > 5:
                print(f"  ... and {len(resources) - 5} more")
        else:
            print("✗ Resource listing failed")
            
        # Test 3: List Tools  
        print("\n3. Testing tool listing...")
        list_tools_request = {
            "jsonrpc": "2.0",
            "id": 3, 
            "method": "tools/list",
            "params": {}
        }
        
        response = send_mcp_request(process, list_tools_request)
        if response and "result" in response:
            tools = response["result"].get("tools", [])
            print(f"✓ Found {len(tools)} tools:")
            for tool in tools[:5]:  # Show first 5
                print(f"  - {tool.get('name', 'unknown')}: {tool.get('description', 'no description')}")
            if len(tools) > 5:
                print(f"  ... and {len(tools) - 5} more")
        else:
            print("✗ Tool listing failed")
            
        # Test 4: Read a resource
        print("\n4. Testing resource reading...")
        read_resource_request = {
            "jsonrpc": "2.0",
            "id": 4,
            "method": "resources/read", 
            "params": {
                "uri": "studio://plm/"
            }
        }
        
        response = send_mcp_request(process, read_resource_request)
        if response and "result" in response:
            print("✓ Resource reading successful")
            contents = response["result"].get("contents", [])
            print(f"  Got {len(contents)} content items")
        else:
            print("✗ Resource reading failed")
            
        # Test 5: Call a tool (if CLI is available)
        print("\n5. Testing tool calling...")
        call_tool_request = {
            "jsonrpc": "2.0",
            "id": 5,
            "method": "tools/call",
            "params": {
                "name": "plm_list_pipelines",
                "arguments": {
                    "limit": 1
                }
            }
        }
        
        response = send_mcp_request(process, call_tool_request)
        if response and "result" in response:
            print("✓ Tool calling successful")
            content = response["result"].get("content", [])
            print(f"  Got {len(content)} content items")
        else:
            print("✗ Tool calling failed (this is expected if CLI is not available)")
            
        print("\n" + "=" * 50)
        print("Test completed!")
        
    except Exception as e:
        print(f"Error during testing: {e}")
        return False
    finally:
        if 'process' in locals():
            process.terminate()
            process.wait()
            
    return True

if __name__ == "__main__":
    success = test_mcp_server()
    sys.exit(0 if success else 1)