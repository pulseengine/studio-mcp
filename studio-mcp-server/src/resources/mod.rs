//! Resource providers for WindRiver Studio MCP server

use std::sync::Arc;
use pulseengine_mcp_protocol::{Resource, ResourceContents, TextResourceContents};
use studio_mcp_shared::{StudioConfig, Result, StudioError, ResourceUri};
use studio_cli_manager::CliManager;
use tracing::{debug, warn};

pub mod plm;

use plm::PlmResourceProvider;

pub struct ResourceProvider {
    cli_manager: Arc<CliManager>,
    config: StudioConfig,
    plm_provider: PlmResourceProvider,
}

impl ResourceProvider {
    pub fn new(cli_manager: Arc<CliManager>, config: StudioConfig) -> Self {
        let plm_provider = PlmResourceProvider::new(cli_manager.clone(), config.clone());
        
        Self {
            cli_manager,
            config,
            plm_provider,
        }
    }

    pub async fn list_resources(&self) -> Result<Vec<Resource>> {
        let mut resources = Vec::new();

        // Add root resources
        resources.push(Resource {
            uri: "studio://".to_string(),
            name: "WindRiver Studio".to_string(),
            description: Some("Root resource for WindRiver Studio CLI access".to_string()),
            mime_type: Some("application/json".to_string()),
        });

        resources.push(Resource {
            uri: "studio://plm/".to_string(),
            name: "Pipeline Management".to_string(),
            description: Some("Pipeline Management (PLM) resources and operations".to_string()),
            mime_type: Some("application/json".to_string()),
        });

        // Add PLM-specific resources
        let plm_resources = self.plm_provider.list_resources().await?;
        resources.extend(plm_resources);

        // Add other service areas (placeholder for future expansion)
        resources.extend(self.list_placeholder_resources());

        debug!("Listed {} total resources", resources.len());
        Ok(resources)
    }

    pub async fn read_resource(&self, uri: &str) -> Result<Vec<ResourceContents>> {
        debug!("Reading resource: {}", uri);

        let parsed_uri = ResourceUri::parse(uri)?;
        
        match parsed_uri.path.first().map(|s| s.as_str()) {
            Some("plm") => {
                self.plm_provider.read_resource(&parsed_uri).await
            }
            Some("config") => {
                self.read_config_resource(&parsed_uri).await
            }
            Some("status") => {
                self.read_status_resource().await
            }
            None => {
                // Root resource
                self.read_root_resource().await
            }
            Some(service) => {
                warn!("Unknown service area: {}", service);
                Err(StudioError::ResourceNotFound(format!("Service '{}' not implemented", service)))
            }
        }
    }

    async fn read_root_resource(&self) -> Result<Vec<ResourceContents>> {
        let content = serde_json::json!({
            "name": "WindRiver Studio MCP Server",
            "version": env!("CARGO_PKG_VERSION"),
            "description": "Model Context Protocol server for WindRiver Studio CLI",
            "services": [
                {
                    "name": "plm",
                    "description": "Pipeline Management",
                    "uri": "studio://plm/"
                },
                {
                    "name": "artifacts",
                    "description": "Artifact Management",
                    "uri": "studio://artifacts/",
                    "status": "planned"
                },
                {
                    "name": "vlab",
                    "description": "Virtual Lab Management",
                    "uri": "studio://vlab/",
                    "status": "planned"
                }
            ],
            "cli_info": {
                "installed_versions": self.cli_manager.list_installed_versions().unwrap_or_default(),
                "config": {
                    "auto_update": self.config.cli.auto_update,
                    "version": self.config.cli.version,
                    "timeout": self.config.cli.timeout
                }
            }
        });

        Ok(vec![ResourceContents::Text(TextResourceContents {
            text: content.to_string(),
            mime_type: Some("application/json".to_string()),
        })])
    }

    async fn read_config_resource(&self, uri: &ResourceUri) -> Result<Vec<ResourceContents>> {
        match uri.path.get(1).map(|s| s.as_str()) {
            Some("connections") => {
                let content = serde_json::to_string_pretty(&self.config.connections)?;
                Ok(vec![ResourceContents::Text(TextResourceContents {
                    text: content,
                    mime_type: Some("application/json".to_string()),
                })])
            }
            Some("cli") => {
                let content = serde_json::to_string_pretty(&self.config.cli)?;
                Ok(vec![ResourceContents::Text(TextResourceContents {
                    text: content,
                    mime_type: Some("application/json".to_string()),
                })])
            }
            None => {
                let content = serde_json::to_string_pretty(&self.config)?;
                Ok(vec![ResourceContents::Text(TextResourceContents {
                    text: content,
                    mime_type: Some("application/json".to_string()),
                })])
            }
            Some(config_type) => {
                Err(StudioError::ResourceNotFound(format!("Config type '{}' not found", config_type)))
            }
        }
    }

    async fn read_status_resource(&self) -> Result<Vec<ResourceContents>> {
        let cli_versions = self.cli_manager.list_installed_versions().unwrap_or_default();
        let default_connection = self.config.get_default_connection();

        let content = serde_json::json!({
            "server": {
                "name": "studio-mcp-server",
                "version": env!("CARGO_PKG_VERSION"),
                "status": "running"
            },
            "cli": {
                "installed_versions": cli_versions,
                "auto_update": self.config.cli.auto_update
            },
            "connections": {
                "total": self.config.connections.len(),
                "default": default_connection.map(|c| &c.name),
                "configured": self.config.connections.keys().collect::<Vec<_>>()
            },
            "cache": {
                "enabled": self.config.cache.enabled,
                "ttl": self.config.cache.ttl,
                "max_size": self.config.cache.max_size
            }
        });

        Ok(vec![ResourceContents::Text(TextResourceContents {
            text: content.to_string(),
            mime_type: Some("application/json".to_string()),
        })])
    }

    fn list_placeholder_resources(&self) -> Vec<Resource> {
        vec![
            Resource {
                uri: "studio://artifacts/".to_string(),
                name: "Artifact Management".to_string(),
                description: Some("Artifact storage and retrieval (coming soon)".to_string()),
                mime_type: Some("application/json".to_string()),
            },
            Resource {
                uri: "studio://vlab/".to_string(),
                name: "Virtual Lab".to_string(),
                description: Some("Virtual and physical lab management (coming soon)".to_string()),
                mime_type: Some("application/json".to_string()),
            },
            Resource {
                uri: "studio://config/".to_string(),
                name: "Configuration".to_string(),
                description: Some("Server and CLI configuration".to_string()),
                mime_type: Some("application/json".to_string()),
            },
            Resource {
                uri: "studio://status".to_string(),
                name: "Server Status".to_string(),
                description: Some("Current server status and health information".to_string()),
                mime_type: Some("application/json".to_string()),
            },
        ]
    }
}