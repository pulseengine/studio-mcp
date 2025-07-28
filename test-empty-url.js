#!/usr/bin/env node

const { Binary } = require("binary-install");

console.log("Testing Binary constructor with empty URL...");

try {
  const binary = new Binary("studio-mcp-server", "");
  console.log("✅ Success - This shouldn't happen!");
} catch (err) {
  console.log("❌ Failed as expected:", err.message);
}