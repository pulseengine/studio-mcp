#!/usr/bin/env node

/**
 * Integration test using the official MCP SDK
 * Tests the complete flow: MCP Client (SDK) → MCP Server → studio-cli → Mock Server
 */

import { spawn } from 'child_process';
import { Client } from '@modelcontextprotocol/sdk/client/index.js';
import { StdioClientTransport } from '@modelcontextprotocol/sdk/client/stdio.js';
import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';
import fetch from 'node-fetch';
import { setTimeout } from 'timers/promises';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const PROJECT_ROOT = path.resolve(__dirname, '../..');

// Colors for output
const colors = {
    red: '\x1b[31m',
    green: '\x1b[32m',
    yellow: '\x1b[33m',
    reset: '\x1b[0m'
};

class IntegrationTester {
    constructor() {
        this.client = null;
        this.transport = null;
        this.serverProcess = null;
        this.testConfigPath = null;
        this.results = {};
    }

    log(message) {
        console.log(`${colors.green}[${new Date().toLocaleTimeString()}] ${message}${colors.reset}`);
    }

    error(message) {
        console.log(`${colors.red}[${new Date().toLocaleTimeString()}] ERROR: ${message}${colors.reset}`);
    }

    warn(message) {
        console.log(`${colors.yellow}[${new Date().toLocaleTimeString()}] WARNING: ${message}${colors.reset}`);
    }

    async setup() {
        this.log('Setting up integration test environment...');

        // Create test configuration
        const config = {
            connections: {
                test_mock: {
                    name: "Test Mock Server",
                    url: "http://localhost:8080",
                    username: "admin",
                    token: null
                }
            },
            default_connection: "test_mock",
            cli: {
                download_base_url: "https://distro.windriver.com/dist/wrstudio/wrstudio-cli-distro-cd",
                version: "auto",
                install_dir: null,
                timeout: 300,
                timeouts: {
                    quick_operations: 10,
                    medium_operations: 30,
                    long_operations: 60,
                    network_requests: 10
                },
                auto_update: false,
                update_check_interval: 24
            },
            cache: {
                enabled: true,
                ttl: 300,
                max_size: 1000
            },
            logging: {
                level: "debug",
                format: "pretty",
                file_logging: false,
                log_file: null
            }
        };

        // Write config to temporary file
        this.testConfigPath = path.join(__dirname, 'test-config-mcp.json');
        fs.writeFileSync(this.testConfigPath, JSON.stringify(config, null, 2));

        // Check prerequisites
        const mcpServerPath = path.join(PROJECT_ROOT, 'target/release/studio-mcp-server');
        if (!fs.existsSync(mcpServerPath)) {
            this.error(`MCP server binary not found at ${mcpServerPath}`);
            this.error('Run "cargo build --release" first');
            return false;
        }

        // Start mock server
        this.log('Starting mock server...');
        if (!await this.startMockServer()) {
            this.error('Failed to start mock server');
            return false;
        }

        // Start MCP server with official SDK
        this.log('Starting MCP server with official SDK...');
        try {
            this.transport = new StdioClientTransport({
                command: mcpServerPath,
                args: [this.testConfigPath],
                stderr: 'pipe'
            });

            this.client = new Client({
                name: "integration-test",
                version: "1.0.0"
            }, {
                capabilities: {}
            });

            await this.client.connect(this.transport);
            this.log('✅ MCP client connected successfully');
            return true;
        } catch (error) {
            this.error(`Failed to start MCP client: ${error.message}`);
            return false;
        }
    }

    async startMockServer() {
        try {
            const mockServerDir = path.join(PROJECT_ROOT, 'mock-studio-server');
            
            // Start docker-compose
            const dockerProcess = spawn('docker-compose', ['up', '-d'], {
                cwd: mockServerDir,
                stdio: 'pipe'
            });

            return new Promise((resolve) => {
                dockerProcess.on('close', async (code) => {
                    if (code === 0) {
                        // Wait for startup
                        await setTimeout(5000);
                        // Check health
                        resolve(await this.checkMockServerHealth());
                    } else {
                        resolve(false);
                    }
                });
            });
        } catch (error) {
            this.error(`Failed to start mock server: ${error.message}`);
            return false;
        }
    }

    async checkMockServerHealth() {
        try {
            const response = await fetch('http://localhost:8080/api/health', {
                timeout: 10000
            });
            const data = await response.json();
            return response.ok && data.status === 'healthy';
        } catch (error) {
            return false;
        }
    }

    async teardown() {
        this.warn('Cleaning up...');

        if (this.client) {
            try {
                await this.client.close();
            } catch (error) {
                // Ignore cleanup errors
            }
        }

        // Stop mock server
        try {
            const mockServerDir = path.join(PROJECT_ROOT, 'mock-studio-server');
            spawn('docker-compose', ['down', '-v'], {
                cwd: mockServerDir,
                stdio: 'ignore'
            });
        } catch (error) {
            // Ignore cleanup errors
        }

        // Remove test config
        if (this.testConfigPath && fs.existsSync(this.testConfigPath)) {
            fs.unlinkSync(this.testConfigPath);
        }
    }

    async runTest(testName, testFunc) {
        this.log(`Testing ${testName}...`);
        try {
            const result = await testFunc();
            if (result) {
                this.log(`✅ ${testName} passed`);
                this.results[testName] = 'PASS';
                return true;
            } else {
                this.error(`❌ ${testName} failed`);
                this.results[testName] = 'FAIL';
                return false;
            }
        } catch (error) {
            this.error(`❌ ${testName} failed with exception: ${error.message}`);
            this.results[testName] = `ERROR: ${error.message}`;
            return false;
        }
    }

    async testMockServerHealth() {
        return await this.checkMockServerHealth();
    }

    async testMockServerAuth() {
        try {
            const response = await fetch('http://localhost:8080/api/auth/token', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ username: 'admin', password: 'password' }),
                timeout: 10000
            });
            const data = await response.json();
            return response.ok && data.access_token;
        } catch (error) {
            return false;
        }
    }

    async testMockServerPipelines() {
        try {
            const response = await fetch('http://localhost:8080/api/plm/pipelines', {
                timeout: 10000
            });
            const data = await response.json();
            return response.ok && Array.isArray(data) && data.length > 0;
        } catch (error) {
            return false;
        }
    }

    async testMcpToolsList() {
        try {
            const response = await this.client.listTools();
            this.log(`Found ${response.tools.length} tools`);
            
            // Check for expected PLM tools
            const toolNames = response.tools.map(tool => tool.name);
            const expectedTools = [
                'plm_list_pipelines',
                'plm_resolve_run_id',
                'plm_get_run_log'
            ];

            for (const tool of expectedTools) {
                if (!toolNames.includes(tool)) {
                    this.error(`Missing expected tool: ${tool}`);
                    return false;
                }
            }

            return response.tools.length > 0;
        } catch (error) {
            this.error(`Tools list error: ${error.message}`);
            return false;
        }
    }

    async testMcpResourcesList() {
        try {
            const response = await this.client.listResources();
            this.log(`Found ${response.resources.length} resources`);
            return response.resources.length > 0;
        } catch (error) {
            this.error(`Resources list error: ${error.message}`);
            return false;
        }
    }

    async testPlmListPipelines() {
        try {
            const response = await this.client.callTool({
                name: 'plm_list_pipelines',
                arguments: {}
            });

            if (response.content && response.content.length > 0) {
                const toolResult = JSON.parse(response.content[0].text);
                if (toolResult.success) {
                    const pipelines = toolResult.data;
                    this.log(`Found ${pipelines.length} pipelines`);
                    return pipelines.length > 0;
                }
            }
            return false;
        } catch (error) {
            this.error(`PLM list pipelines error: ${error.message}`);
            return false;
        }
    }

    async testPlmResolveRunId() {
        try {
            const response = await this.client.callTool({
                name: 'plm_resolve_run_id',
                arguments: {
                    pipeline_name: 'build-api-service',
                    run_number: 1
                }
            });

            if (response.content && response.content.length > 0) {
                const toolResult = JSON.parse(response.content[0].text);
                if (toolResult.success) {
                    this.log(`Resolved run ID: ${toolResult.run_id}`);
                    return toolResult.run_id != null;
                }
            }
            return false;
        } catch (error) {
            this.error(`PLM resolve run ID error: ${error.message}`);
            return false;
        }
    }

    async testPlmGetRunLogsWithName() {
        try {
            const response = await this.client.callTool({
                name: 'plm_get_run_log',
                arguments: {
                    pipeline_name: 'build-api-service',
                    run_number: 1,
                    errors_only: true
                }
            });

            if (response.content && response.content.length > 0) {
                const toolResult = JSON.parse(response.content[0].text);
                if (toolResult.success) {
                    const logs = toolResult.logs || [];
                    this.log(`Retrieved ${logs.length} log entries`);
                    return true;
                }
            }
            return false;
        } catch (error) {
            this.error(`PLM get run logs error: ${error.message}`);
            return false;
        }
    }

    async runAllTests() {
        this.log('Starting WindRiver Studio MCP Integration Tests (Official SDK)');

        if (!await this.setup()) {
            this.error('Failed to set up test environment');
            return false;
        }

        try {
            const tests = [
                ['Mock Server Health', () => this.testMockServerHealth()],
                ['Mock Server Auth', () => this.testMockServerAuth()],
                ['Mock Server Pipelines', () => this.testMockServerPipelines()],
                ['MCP Tools List', () => this.testMcpToolsList()],
                ['MCP Resources List', () => this.testMcpResourcesList()],
                ['PLM List Pipelines', () => this.testPlmListPipelines()],
                ['PLM Resolve Run ID', () => this.testPlmResolveRunId()],
                ['PLM Get Run Logs with Name', () => this.testPlmGetRunLogsWithName()],
            ];

            let passed = 0;
            const total = tests.length;

            for (const [testName, testFunc] of tests) {
                if (await this.runTest(testName, testFunc)) {
                    passed++;
                }
            }

            // Print summary
            console.log(`\n${colors.green}Integration Test Summary:${colors.reset}`);
            console.log(`Passed: ${passed}/${total}`);

            if (passed === total) {
                console.log(`${colors.green}✅ All tests passed!${colors.reset}`);
                return true;
            } else {
                console.log(`${colors.red}❌ ${total - passed} test(s) failed${colors.reset}`);
                return false;
            }
        } finally {
            await this.teardown();
        }
    }
}

async function main() {
    const tester = new IntegrationTester();
    const success = await tester.runAllTests();
    process.exit(success ? 0 : 1);
}

main().catch(error => {
    console.error(`${colors.red}Unhandled error: ${error.message}${colors.reset}`);
    process.exit(1);
});