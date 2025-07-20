//! Configuration management for WindRiver Studio MCP server

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::types::StudioConnection;

/// Main configuration for the Studio MCP server
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    
    /// Timeout for CLI operations (seconds)
    pub timeout: u64,
    
    /// Whether to auto-update CLI
    pub auto_update: bool,
    
    /// Update check interval (hours)
    pub update_check_interval: u64,
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

impl Default for StudioConfig {
    fn default() -> Self {
        Self {
            connections: HashMap::new(),
            default_connection: None,
            cli: CliConfig::default(),
            cache: CacheConfig::default(),
            logging: LoggingConfig::default(),
        }
    }
}

impl Default for CliConfig {
    fn default() -> Self {
        Self {
            download_base_url: "https://distro.windriver.com/dist/wrstudio/wrstudio-cli-distro-cd".to_string(),
            version: "auto".to_string(),
            install_dir: None,
            timeout: 300, // 5 minutes
            auto_update: true,
            update_check_interval: 24, // 24 hours
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