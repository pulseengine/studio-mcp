#!/usr/bin/env node

const { Binary } = require("binary-install");
const os = require("os");

// Copy the exact logic from install.js
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
    throw new Error(`Unsupported architecture: ${arch}. Supported architectures: x64 (Intel), arm64 (Apple Silicon).`);
  }

  return `${archSuffix}-${platform}`;
}

function getBinaryName() {
  const platform = os.type();
  return platform === "Windows_NT" ? "studio-mcp-server.exe" : "studio-mcp-server";
}

function getDownloadUrl() {
  const version = "0.2.5"; // Current version
  const platform = getPlatform();
  
  // Determine archive format based on platform
  const archiveExtension = os.type() === "Windows_NT" ? "zip" : "tar.gz";
  
  // Use GitHub releases for binary distribution
  return `https://github.com/pulseengine/studio-mcp/releases/download/v${version}/studio-mcp-server-v${version}-${platform}.${archiveExtension}`;
}

const binaryName = getBinaryName();
const downloadUrl = getDownloadUrl();

console.log("=== Testing binary-install ===");
console.log(`Platform detected: ${os.type()} ${os.arch()}`);
console.log(`Target: ${getPlatform()}`);
console.log(`Binary: ${binaryName}`);
console.log(`Download URL: ${downloadUrl}`);
console.log("");

// Test URL validity
try {
  const url = new URL(downloadUrl);
  console.log("✅ URL is valid in Node.js");
  console.log(`Protocol: ${url.protocol}`);
  console.log(`Host: ${url.host}`);
  console.log(`Pathname: ${url.pathname}`);
} catch (err) {
  console.log("❌ URL is INVALID in Node.js");
  console.log(`Error: ${err.message}`);
  process.exit(1);
}

console.log("");
console.log("=== Testing Binary constructor ===");

// Test Binary constructor directly
try {
  console.log("Creating Binary instance...");
  const binary = new Binary(binaryName, downloadUrl);
  console.log("✅ Binary constructor succeeded!");
  console.log("Binary instance:", {
    name: binary.name,
    url: binary.url,
    installDirectory: binary.installDirectory
  });
  
  console.log("");
  console.log("=== Testing install (dry run) ===");
  // Just test the install method without actually downloading
  binary.install().then(() => {
    console.log("✅ Install completed successfully!");
  }).catch(err => {
    console.log("❌ Install failed:");
    console.log(`Error: ${err.message}`);
    console.log(`Stack: ${err.stack}`);
  });
  
} catch (err) {
  console.log("❌ Binary constructor failed:");
  console.log(`Error: ${err.message}`);
  console.log(`Error type: ${err.constructor.name}`);
  console.log(`Stack: ${err.stack}`);
}