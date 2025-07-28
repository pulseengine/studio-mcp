#!/usr/bin/env node

// Patch the Binary constructor to see what it's receiving
const binaryInstall = require("binary-install");
const originalBinary = binaryInstall.Binary;

binaryInstall.Binary = function(name, url, ...args) {
  console.log("=== Binary Constructor Called ===");
  console.log("Name:", JSON.stringify(name));
  console.log("URL:", JSON.stringify(url));
  console.log("Args:", JSON.stringify(args));
  console.log("Name type:", typeof name);
  console.log("URL type:", typeof url);
  console.log("Name length:", name ? name.length : 'null/undefined');
  console.log("URL length:", url ? url.length : 'null/undefined');
  
  // Test URL validity
  try {
    const urlObj = new URL(url);
    console.log("✅ URL is valid");
    console.log("Protocol:", urlObj.protocol);
    console.log("Host:", urlObj.host);
    console.log("Pathname:", urlObj.pathname);
  } catch (err) {
    console.log("❌ URL is invalid:", err.message);
  }
  
  // Call original constructor
  return new originalBinary(name, url, ...args);
};

// Now require the package install script
const os = require("os");
const path = require("path");

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
  const version = "0.2.5"; // hardcode for testing
  const platform = getPlatform();
  const binaryName = getBinaryName();
  
  // Determine archive format based on platform
  const archiveExtension = os.type() === "Windows_NT" ? "zip" : "tar.gz";
  
  // Use GitHub releases for binary distribution
  return `https://github.com/pulseengine/studio-mcp/releases/download/v${version}/studio-mcp-server-v${version}-${platform}.${archiveExtension}`;
}

const binaryName = getBinaryName();
const downloadUrl = getDownloadUrl();

console.log("=== Pre-Binary Constructor ===");
console.log(`Platform detected: ${os.type()} ${os.arch()}`);
console.log(`Target: ${getPlatform()}`);
console.log(`Binary: ${binaryName}`);
console.log(`Download URL: ${downloadUrl}`);
console.log("");

const { Binary } = binaryInstall;
const binary = new Binary(binaryName, downloadUrl);