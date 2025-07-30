#!/usr/bin/env node

const { spawnSync } = require("child_process");
const { getBinaryPath } = require("./index.js");

// Get the binary path using the same logic as index.js
let binaryPath;
try {
  binaryPath = getBinaryPath();
} catch (err) {
  console.error(err.message);
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