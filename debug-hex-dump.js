#!/usr/bin/env node

const { Binary } = require("binary-install");
const os = require("os");

function getPlatform() {
  const type = os.type();
  const arch = os.arch();
  let platform, archSuffix;

  if (type === "Windows_NT") {
    platform = "pc-windows-msvc";
  } else if (type === "Linux") {
    platform = "unknown-linux-gnu";
  } else if (type === "Darwin") {
    platform = "apple-darwin";
  } else {
    throw new Error(`Unsupported platform: ${type}`);
  }

  if (arch === "x64") {
    archSuffix = "x86_64";
  } else if (arch === "arm64") {
    archSuffix = "aarch64";
  } else {
    throw new Error(`Unsupported architecture: ${arch}`);
  }

  return `${archSuffix}-${platform}`;
}

function getBinaryName() {
  const platform = os.type();
  return platform === "Windows_NT" ? "studio-mcp-server.exe" : "studio-mcp-server";
}

function getDownloadUrl() {
  const version = "0.2.5";
  const platform = getPlatform();
  const archiveExtension = os.type() === "Windows_NT" ? "zip" : "tar.gz";
  return `https://github.com/pulseengine/studio-mcp/releases/download/v${version}/studio-mcp-server-v${version}-${platform}.${archiveExtension}`;
}

const binaryName = getBinaryName();
const downloadUrl = getDownloadUrl();

console.log("=== Hex Dump Analysis ===");
console.log("Binary name:", JSON.stringify(binaryName));
console.log("Binary name hex:", Buffer.from(binaryName, 'utf8').toString('hex'));
console.log("Download URL:", JSON.stringify(downloadUrl));
console.log("Download URL hex:", Buffer.from(downloadUrl, 'utf8').toString('hex'));

// Check for any invisible/control characters
console.log("\n=== Character Analysis ===");
console.log("Binary name char codes:", [...binaryName].map(c => c.charCodeAt(0)));
console.log("URL char codes (first 50):", [...downloadUrl.slice(0, 50)].map(c => c.charCodeAt(0)));
console.log("URL char codes (last 20):", [...downloadUrl.slice(-20)].map(c => c.charCodeAt(0)));

// Test URL parsing step by step
console.log("\n=== Step by Step URL Test ===");
console.log("1. URL string length:", downloadUrl.length);
console.log("2. URL starts with https:", downloadUrl.startsWith('https://'));
console.log("3. URL contains github.com:", downloadUrl.includes('github.com'));

try {
  const url = new URL(downloadUrl);
  console.log("4. ✅ URL construction succeeded");
} catch (err) {
  console.log("4. ❌ URL construction failed:", err);
}

// Now try Binary constructor with extreme debugging
console.log("\n=== Binary Constructor with Error Catching ===");
try {
  console.log("About to call new Binary...");
  const binary = new Binary(binaryName, downloadUrl);
  console.log("✅ Binary construction succeeded!");
  console.log("Binary url property:", binary.url);
  console.log("Binary name property:", binary.name);
} catch (err) {
  console.log("❌ Binary construction failed:");
  console.log("Error:", err);
  console.log("Error message:", err.message);
  console.log("Error stack:", err.stack);
}