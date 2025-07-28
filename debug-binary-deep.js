#!/usr/bin/env node

// Deep debugging of the Binary constructor
const os = require("os");

// Test the URL generation first
function getPlatform() {
  const type = os.type();
  const arch = os.arch();

  let platform;
  let archSuffix;

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

console.log("=== URL Generation Test ===");
console.log("Binary name:", JSON.stringify(binaryName));
console.log("Download URL:", JSON.stringify(downloadUrl));
console.log("Binary name type:", typeof binaryName);
console.log("Download URL type:", typeof downloadUrl);
console.log("URL length:", downloadUrl.length);

// Test URL parsing directly
console.log("\n=== Direct URL Test ===");
try {
  const urlObj = new URL(downloadUrl);
  console.log("✅ Direct URL parsing succeeded");
  console.log("Protocol:", urlObj.protocol);
  console.log("Host:", urlObj.host);
  console.log("Pathname:", urlObj.pathname);
} catch (err) {
  console.log("❌ Direct URL parsing failed:", err);
  console.log("Error type:", err.constructor.name);
  console.log("Error message:", err.message);
}

// Now test exactly what the Binary constructor does
console.log("\n=== Binary Constructor Logic Test ===");
let errors = [];

// Test name parameter
if (binaryName && typeof binaryName !== "string") {
  errors.push("name must be a string");
}
if (!binaryName) {
  errors.push("You must specify the name of your binary");
}

// Test URL parameter  
if (typeof downloadUrl !== "string") {
  errors.push("url must be a string");
} else {
  try {
    new URL(downloadUrl);
    console.log("✅ Binary constructor URL test passed");
  } catch (e) {
    console.log("❌ Binary constructor URL test failed:", e);
    console.log("Error type:", e.constructor.name);
    console.log("Error message:", e.message);
    console.log("Error stack:", e.stack);
    errors.push(e);
  }
}

console.log("Errors array:", errors);
console.log("Errors length:", errors.length);

if (errors.length > 0) {
  console.log("\n=== Error Message Construction ===");
  let errorMsg = "One or more of the parameters you passed to the Binary constructor are invalid:\n";
  errors.forEach((error, index) => {
    console.log(`Error ${index}:`, error);
    console.log(`Error ${index} type:`, typeof error);
    console.log(`Error ${index} toString():`, error.toString());
    errorMsg += error;
  });
  errorMsg += '\n\nCorrect usage: new Binary("my-binary", "https://example.com/binary/download.tar.gz")';
  console.log("Final error message:", errorMsg);
} else {
  console.log("✅ No errors - Binary constructor should succeed");
}