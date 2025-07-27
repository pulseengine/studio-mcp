#!/usr/bin/env node

// Main entry point for programmatic usage
const { Binary } = require("binary-install");
const os = require("os");

function getBinaryName() {
  const platform = os.type();
  return platform === "Windows_NT" ? "studio-mcp-server.exe" : "studio-mcp-server";
}

function createBinary() {
  return new Binary(getBinaryName(), "");
}

module.exports = {
  createBinary,
  Binary,
  getBinaryName
};

// If called directly, run the binary
if (require.main === module) {
  const binary = createBinary();
  binary.run().catch(err => {
    console.error("Failed to run studio-mcp-server:", err.message);
    process.exit(1);
  });
}