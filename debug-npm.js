#!/usr/bin/env node

// Test script to debug npm package URL generation
const os = require("os");

function getPlatform() {
  const type = os.type();
  const arch = os.arch();

  let platform;
  let archSuffix;

  // Determine platform
  if (type === "Windows_NT") {
    platform = "pc-windows-msvc";
  } else if (type === "Linux") {
    platform = "unknown-linux-gnu";
  } else if (type === "Darwin") {
    platform = "apple-darwin";
  } else {
    throw new Error(`Unsupported platform: ${type}`);
  }

  // Determine architecture
  if (arch === "x64") {
    archSuffix = "x86_64";
  } else if (arch === "arm64") {
    archSuffix = "aarch64";
  } else {
    throw new Error(`Unsupported architecture: ${arch}. Supported architectures: x64 (Intel), arm64 (Apple Silicon). Please install from source: cargo install --git https://github.com/pulseengine/studio-mcp.git studio-mcp-server`);
  }

  return `${archSuffix}-${platform}`;
}

function getBinaryName() {
  const platform = os.type();
  return platform === "Windows_NT" ? "studio-mcp-server.exe" : "studio-mcp-server";
}

function getDownloadUrl() {
  const version = "0.2.5"; // Test version
  const platform = getPlatform();
  const binaryName = getBinaryName();
  
  // Determine archive format based on platform
  const archiveExtension = os.type() === "Windows_NT" ? "zip" : "tar.gz";
  
  // Use GitHub releases for binary distribution
  return `https://github.com/pulseengine/studio-mcp/releases/download/v${version}/studio-mcp-server-v${version}-${platform}.${archiveExtension}`;
}

// Debug output
console.log("=== DEBUG INFO ===");
console.log(`Platform detected: ${os.type()} ${os.arch()}`);
console.log(`Target: ${getPlatform()}`);
console.log(`Binary: ${getBinaryName()}`);
console.log(`Download URL: ${getDownloadUrl()}`);
console.log("");

// Test if URL is valid
try {
  const url = new URL(getDownloadUrl());
  console.log("✅ URL is valid!");
  console.log(`Protocol: ${url.protocol}`);
  console.log(`Host: ${url.host}`);
  console.log(`Pathname: ${url.pathname}`);
} catch (err) {
  console.log("❌ URL is INVALID!");
  console.log(`Error: ${err.message}`);
}

// Test if the release actually exists
console.log("\n=== TESTING RELEASE ===");
const testUrl = getDownloadUrl();
console.log(`Testing: ${testUrl}`);