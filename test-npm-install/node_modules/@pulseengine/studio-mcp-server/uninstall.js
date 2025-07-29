#!/usr/bin/env node

const { Binary } = require("binary-install");
const os = require("os");

function getBinaryName() {
  const platform = os.type();
  return platform === "Windows_NT" ? "studio-mcp-server.exe" : "studio-mcp-server";
}

const binary = new Binary(getBinaryName(), "");

binary.uninstall().catch(err => {
  // Don't fail uninstall if binary cleanup fails
  console.warn("Warning: Failed to clean up binary:", err.message);
});