#!/usr/bin/env node

const { Binary } = require("binary-install");
const os = require("os");

function getBinaryName() {
  const platform = os.type();
  return platform === "Windows_NT" ? "studio-mcp-server.exe" : "studio-mcp-server";
}

// Create binary instance (URL not needed for running)
const binary = new Binary(getBinaryName(), "");

// Run the binary with all arguments passed through
binary.run().catch(err => {
  console.error("Failed to run studio-mcp-server:", err.message);
  console.error("\n🔧 Try reinstalling: npm install -g @pulseengine/studio-mcp-server");
  process.exit(1);
});