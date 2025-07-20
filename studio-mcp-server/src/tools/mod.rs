//! Tool providers for WindRiver Studio MCP server

use std::sync::Arc;
use pulseengine_mcp_protocol::{Tool, Content};
use studio_mcp_shared::{StudioConfig, Result, StudioError};
use studio_cli_manager::CliManager;
use serde_json::Value;
use tracing::{debug, error, warn};

pub mod plm;

use plm::PlmToolProvider;

pub struct ToolProvider {
    cli_manager: Arc<CliManager>,
    config: StudioConfig,
    plm_provider: PlmToolProvider,
}

impl ToolProvider {
    pub fn new(cli_manager: Arc<CliManager>, config: StudioConfig) -> Self {
        let plm_provider = PlmToolProvider::new(cli_manager.clone(), config.clone());
        
        Self {
            cli_manager,
            config,
            plm_provider,
        }
    }

    pub async fn list_tools(&self) -> Result<Vec<Tool>> {
        let mut tools = Vec::new();

        // Add system tools
        tools.extend(self.list_system_tools());

        // Add PLM tools
        let plm_tools = self.plm_provider.list_tools().await?;
        tools.extend(plm_tools);

        debug!("Listed {} total tools", tools.len());
        Ok(tools)
    }

    pub async fn call_tool(&self, name: &str, arguments: Option<Value>) -> Result<Vec<Content>> {
        debug!("Calling tool: {} with args: {:?}", name, arguments);

        match name {
            // System tools
            "studio_status" => self.get_studio_status().await,
            "studio_version" => self.get_studio_version().await,
            "cli_info" => self.get_cli_info().await,
            
            // PLM tools (delegate to PLM provider)
            name if name.starts_with("plm_") => {
                self.plm_provider.call_tool(name, arguments).await
            }
            
            _ => {
                error!("Unknown tool: {}", name);
                Err(StudioError::InvalidOperation(format!("Tool '{}' not found", name)))
            }
        }
    }

    fn list_system_tools(&self) -> Vec<Tool> {
        vec![
            Tool {
                name: "studio_status".to_string(),
                description: "Get current status of the Studio MCP server and CLI".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {},
                    "required": []
                }),
            },
            Tool {
                name: "studio_version".to_string(),
                description: "Get version information for the Studio CLI and server".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {},
                    "required": []
                }),
            },
            Tool {
                name: "cli_info".to_string(),
                description: "Get detailed information about the Studio CLI installation".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {},
                    "required": []
                }),
            },
        ]
    }

    async fn get_studio_status(&self) -> Result<Vec<Content>> {
        let cli_versions = self.cli_manager.list_installed_versions().unwrap_or_default();
        let default_connection = self.config.get_default_connection();

        let status = serde_json::json!({
            "server": {
                "name": "studio-mcp-server",
                "version": env!("CARGO_PKG_VERSION"),
                "status": "running",
                "uptime": "N/A" // Would need to track start time
            },
            "cli": {
                "installed_versions": cli_versions,
                "auto_update_enabled": self.config.cli.auto_update,
                "base_url": self.config.cli.download_base_url,
                "timeout": self.config.cli.timeout
            },
            "connections": {
                "total_configured": self.config.connections.len(),
                "default_connection": default_connection.map(|c| &c.name),
                "available_connections": self.config.connections.keys().collect::<Vec<_>>()
            },
            "cache": {
                "enabled": self.config.cache.enabled,
                "ttl_seconds": self.config.cache.ttl,
                "max_size": self.config.cache.max_size
            }
        });

        Ok(vec![Content::Text {
            text: serde_json::to_string_pretty(&status)?,
        }])
    }

    async fn get_studio_version(&self) -> Result<Vec<Content>> {
        let mut version_info = serde_json::json!({
            "server": {
                "name": "studio-mcp-server",
                "version": env!("CARGO_PKG_VERSION")
            },
            "cli": {
                "configured_version": self.config.cli.version,
                "installed_versions": self.cli_manager.list_installed_versions().unwrap_or_default()
            }
        });

        // Try to get CLI version if available
        match self.cli_manager.ensure_cli(None).await {
            Ok(_cli_path) => {
                match self.cli_manager.execute(&["--version"], None).await {
                    Ok(cli_version) => {
                        version_info["cli"]["current_version"] = cli_version;
                    }
                    Err(e) => {
                        warn!("Failed to get CLI version: {}", e);
                        version_info["cli"]["version_error"] = Value::String(e.to_string());
                    }
                }
            }
            Err(e) => {
                warn!("CLI not available: {}", e);
                version_info["cli"]["availability_error"] = Value::String(e.to_string());
            }
        }

        Ok(vec![Content::Text {
            text: serde_json::to_string_pretty(&version_info)?,
        }])
    }

    async fn get_cli_info(&self) -> Result<Vec<Content>> {
        let installed_versions = self.cli_manager.list_installed_versions().unwrap_or_default();
        
        let mut info = serde_json::json!({
            "installation": {
                "base_url": self.config.cli.download_base_url,
                "install_directory": self.config.cli.install_dir,
                "auto_update": self.config.cli.auto_update,
                "update_interval_hours": self.config.cli.update_check_interval,
                "timeout_seconds": self.config.cli.timeout
            },
            "versions": {
                "configured": self.config.cli.version,
                "installed": installed_versions,
                "total_installed": installed_versions.len()
            }
        });

        // Try to get more detailed CLI info
        match self.cli_manager.ensure_cli(None).await {
            Ok(cli_path) => {
                info["current"] = serde_json::json!({
                    "path": cli_path.to_string_lossy(),
                    "exists": cli_path.exists(),
                    "executable": cli_path.is_file()
                });

                // Try to get CLI capabilities
                match self.cli_manager.execute(&["--help"], None).await {
                    Ok(help_output) => {
                        info["capabilities"] = help_output;
                    }
                    Err(e) => {
                        warn!("Failed to get CLI help: {}", e);
                        info["help_error"] = Value::String(e.to_string());
                    }
                }
            }
            Err(e) => {
                error!("Failed to ensure CLI availability: {}", e);
                info["error"] = Value::String(e.to_string());
            }
        }

        Ok(vec![Content::Text {
            text: serde_json::to_string_pretty(&info)?,
        }])
    }
}