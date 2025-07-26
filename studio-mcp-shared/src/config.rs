//! Configuration management for WindRiver Studio MCP server

use crate::types::StudioConnection;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Main configuration for the Studio MCP server
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StudioConfig {
    /// Studio connections
    pub connections: HashMap<String, StudioConnection>,

    /// Default connection name
    pub default_connection: Option<String>,

    /// CLI configuration
    pub cli: CliConfig,

    /// Cache configuration
    pub cache: CacheConfig,

    /// Logging configuration
    pub logging: LoggingConfig,
}

/// CLI-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CliConfig {
    /// Base URL for CLI downloads
    pub download_base_url: String,

    /// CLI version to use (auto for latest)
    pub version: String,

    /// Directory to store CLI binaries
    pub install_dir: Option<String>,

    /// Timeout for CLI operations (seconds) - deprecated, use timeouts instead
    pub timeout: u64,

    /// Operation-specific timeouts
    pub timeouts: TimeoutConfig,

    /// Whether to auto-update CLI
    pub auto_update: bool,

    /// Update check interval (hours)
    pub update_check_interval: u64,
}

/// Timeout configuration for different operation types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeoutConfig {
    /// Quick operations like list, get (seconds)
    pub quick_operations: u64,

    /// Medium operations like run, cancel (seconds)
    pub medium_operations: u64,

    /// Long operations like logs, streaming (seconds)
    pub long_operations: u64,

    /// Network requests (seconds)
    pub network_requests: u64,
}

/// Cache configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// Enable caching
    pub enabled: bool,

    /// Cache TTL in seconds
    pub ttl: u64,

    /// Maximum cache size (items)
    pub max_size: usize,
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level (trace, debug, info, warn, error)
    pub level: String,

    /// Log format (json, pretty)
    pub format: String,

    /// Enable file logging
    pub file_logging: bool,

    /// Log file path
    pub log_file: Option<String>,
}

impl Default for CliConfig {
    fn default() -> Self {
        Self {
            download_base_url: "https://distro.windriver.com/dist/wrstudio/wrstudio-cli-distro-cd"
                .to_string(),
            version: "auto".to_string(),
            install_dir: None,
            timeout: 300, // 5 minutes - deprecated
            timeouts: TimeoutConfig::default(),
            auto_update: true,
            update_check_interval: 24, // 24 hours
        }
    }
}

impl Default for TimeoutConfig {
    fn default() -> Self {
        Self {
            quick_operations: 30,   // 30 seconds for list, get operations
            medium_operations: 300, // 5 minutes for run, cancel operations
            long_operations: 600,   // 10 minutes for logs, streaming operations
            network_requests: 30,   // 30 seconds for HTTP requests
        }
    }
}

/// Operation types for timeout selection
#[derive(Debug, Clone, Copy)]
pub enum OperationType {
    /// Quick operations like list, get
    Quick,
    /// Medium operations like run, cancel
    Medium,
    /// Long operations like logs, streaming
    Long,
    /// Network requests
    Network,
}

impl TimeoutConfig {
    /// Get timeout for specific operation type
    pub fn get_timeout(&self, operation_type: OperationType) -> u64 {
        match operation_type {
            OperationType::Quick => self.quick_operations,
            OperationType::Medium => self.medium_operations,
            OperationType::Long => self.long_operations,
            OperationType::Network => self.network_requests,
        }
    }
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            ttl: 300, // 5 minutes
            max_size: 1000,
        }
    }
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            format: "pretty".to_string(),
            file_logging: false,
            log_file: None,
        }
    }
}

impl StudioConfig {
    /// Load configuration from file or create default
    pub fn load_or_default(config_path: Option<&str>) -> crate::Result<Self> {
        match config_path {
            Some(path) => {
                let content = std::fs::read_to_string(path)?;
                let config: StudioConfig = serde_json::from_str(&content)?;
                Ok(config)
            }
            None => Ok(Self::default()),
        }
    }

    /// Save configuration to file
    pub fn save(&self, config_path: &str) -> crate::Result<()> {
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(config_path, content)?;
        Ok(())
    }

    /// Get the default connection
    pub fn get_default_connection(&self) -> Option<&StudioConnection> {
        self.default_connection
            .as_ref()
            .and_then(|name| self.connections.get(name))
    }

    /// Add or update a connection
    pub fn add_connection(&mut self, name: String, connection: StudioConnection) {
        self.connections.insert(name.clone(), connection);

        // Set as default if it's the first connection
        if self.default_connection.is_none() {
            self.default_connection = Some(name);
        }
    }
}
