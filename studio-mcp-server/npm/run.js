#!/usr/bin/env node

const { spawnSync } = require("child_process");
const { join } = require("path");
const { existsSync } = require("fs");
const os = require("os");

function getBinaryName() {
  const platform = os.type();
  return platform === "Windows_NT" ? "studio-mcp-server.exe" : "studio-mcp-server";
}

function findBinaryPath() {
  // Look for the binary in the expected install location
  const binaryName = getBinaryName();
  const possiblePaths = [
    join(__dirname, "node_modules", ".bin", binaryName),
    join(__dirname, "..", ".bin", binaryName),
    join(__dirname, "..", "node_modules", ".bin", binaryName),
  ];
  
  for (const path of possiblePaths) {
    if (existsSync(path)) {
      return path;
    }
  }
  
  return null;
}

const binaryPath = findBinaryPath();

if (!binaryPath) {
  console.error("studio-mcp-server binary not found. Please run 'npm install' to install it.");
  console.error("\n🔧 If the issue persists, try reinstalling: npm install -g @pulseengine/studio-mcp-server");
  process.exit(1);
}

// Run the binary directly with all arguments passed through
const [, , ...args] = process.argv;
const result = spawnSync(binaryPath, args, { 
  cwd: process.cwd(), 
  stdio: "inherit" 
});

if (result.error) {
  console.error("Failed to run studio-mcp-server:", result.error.message);
  console.error("\n🔧 Try reinstalling: npm install -g @pulseengine/studio-mcp-server");
  process.exit(1);
}

process.exit(result.status || 0);