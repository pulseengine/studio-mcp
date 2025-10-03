#!/usr/bin/env node

const os = require("os");
const path = require("path");
const { existsSync } = require("fs");

function getBinaryName() {
  const platform = os.type();
  return platform === "Windows_NT" ? "studio-mcp-server.exe" : "studio-mcp-server";
}

function getPlatformPackageName() {
  const platform = os.platform();
  const arch = os.arch();

  if (platform === "darwin") {
    return arch === "arm64" ? "@pulseengine/studio-mcp-server-darwin-arm64" : "@pulseengine/studio-mcp-server-darwin-x64";
  } else if (platform === "linux") {
    return "@pulseengine/studio-mcp-server-linux-x64";
  } else if (platform === "win32") {
    return "@pulseengine/studio-mcp-server-win32-x64";
  }

  throw new Error(`Unsupported platform: ${platform}-${arch}`);
}

function getBinaryPath() {
  try {
    const platformPackage = getPlatformPackageName();
    const binaryName = getBinaryName();

    // Try to find the platform-specific package
    try {
      const platformPackagePath = require.resolve(platformPackage);
      const binaryPath = path.join(path.dirname(platformPackagePath), binaryName);
      if (existsSync(binaryPath)) {
        return binaryPath;
      }
    } catch (err) {
      // Platform package not found, continue to fallback
    }

    // Fallback to local bin directory (for GitHub releases fallback)
    const fallbackBinaryPath = path.join(__dirname, "bin", binaryName);
    if (existsSync(fallbackBinaryPath)) {
      return fallbackBinaryPath;
    }

    throw new Error(`Binary not found. Install with: npm install ${platformPackage}`);
  } catch (err) {
    throw new Error(`${err.message}

🔧 Installation options:
1. npm install -g @pulseengine/studio-mcp-server
2. Manual download: https://github.com/pulseengine/studio-mcp/releases
3. Build from source: cargo install --git https://github.com/pulseengine/studio-mcp.git studio-mcp-server`);
  }
}

module.exports = {
  getBinaryPath,
  getBinaryName,
  getPlatformPackageName
};

// If called directly, run the binary
if (require.main === module) {
  try {
    const { spawn } = require("child_process");
    const binaryPath = getBinaryPath();
    const [, , ...args] = process.argv;

    const child = spawn(binaryPath, args, {
      stdio: "inherit",
      cwd: process.cwd()
    });

    child.on("close", (code) => {
      process.exit(code || 0);
    });

    child.on("error", (err) => {
      console.error("Failed to run studio-mcp-server:", err.message);
      process.exit(1);
    });
  } catch (err) {
    console.error(err.message);
    process.exit(1);
  }
}
