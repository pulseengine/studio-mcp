#!/usr/bin/env node

const { Binary } = require("binary-install");
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

  // Determine architecture - only support x64 for now
  if (arch === "x64") {
    archSuffix = "x86_64";
  } else {
    throw new Error(`Unsupported architecture: ${arch}. Only x64 is currently supported. Please install from source: cargo install --git https://github.com/pulseengine/studio-mcp.git studio-mcp-server`);
  }

  return `${archSuffix}-${platform}`;
}

function getBinaryName() {
  const platform = os.type();
  return platform === "Windows_NT" ? "studio-mcp-server.exe" : "studio-mcp-server";
}

function getDownloadUrl() {
  const version = require("./package.json").version;
  const platform = getPlatform();
  const binaryName = getBinaryName();
  
  // Use GitHub releases for binary distribution
  return `https://github.com/pulseengine/studio-mcp/releases/download/v${version}/studio-mcp-server-v${version}-${platform}.tar.gz`;
}

const binary = new Binary(getBinaryName(), getDownloadUrl());

binary.install().catch(err => {
  console.error("Failed to install studio-mcp-server binary:", err.message);
  
  // Provide helpful error message
  console.error("\n📋 Installation failed. You can:");
  console.error("1. Install Rust and build from source:");
  console.error("   git clone https://github.com/pulseengine/studio-mcp.git");
  console.error("   cd studio-mcp/studio-mcp-server");
  console.error("   cargo install --path .");
  console.error("");
  console.error("2. Download binary manually from:");
  console.error("   https://github.com/pulseengine/studio-mcp/releases");
  
  process.exit(1);
});