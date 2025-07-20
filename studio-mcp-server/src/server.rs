//! Main MCP server implementation using PulseEngine MCP framework

use std::sync::Arc;
use tracing::{info, debug};
use async_trait::async_trait;

use pulseengine_mcp_server::{McpServer, McpBackend, ServerConfig};
use pulseengine_mcp_protocol::*;

use studio_mcp_shared::{StudioConfig, Result, StudioError};
use studio_cli_manager::CliManager;

use crate::resources::ResourceProvider;
use crate::tools::ToolProvider;

pub struct StudioMcpServer {
    config: StudioConfig,
    cli_manager: Arc<CliManager>,
    resource_provider: Arc<ResourceProvider>,
    tool_provider: Arc<ToolProvider>,
}

impl StudioMcpServer {
    pub async fn new(config: StudioConfig) -> Result<Self> {
        info!("Initializing Studio MCP Server with PulseEngine framework");
        
        // Initialize CLI manager
        let cli_manager = Arc::new(
            CliManager::new(
                config.cli.download_base_url.clone(),
                config.cli.install_dir.as_ref().map(std::path::PathBuf::from),
            )?
        );

        // Ensure CLI is available
        cli_manager.ensure_cli(
            if config.cli.version == "auto" { 
                None 
            } else { 
                Some(&config.cli.version) 
            }
        ).await?;

        // Initialize providers
        let resource_provider = Arc::new(ResourceProvider::new(
            cli_manager.clone(),
            config.clone(),
        ));

        let tool_provider = Arc::new(ToolProvider::new(
            cli_manager.clone(),
            config.clone(),
        ));

        Ok(Self {
            config,
            cli_manager,
            resource_provider,
            tool_provider,
        })
    }

    pub async fn run(self) -> Result<()> {
        let backend = StudioMcpBackend {
            inner: Arc::new(self),
        };

        // Create server with default config and stdio transport
        let server_config = ServerConfig::default();
        let server = McpServer::new(backend, server_config)
            .map_err(|e| StudioError::Mcp(format!("Failed to create server: {}", e)))?;
        
        info!("Starting PulseEngine MCP server with stdio transport");
        
        server.run().await.map_err(|e| {
            StudioError::Mcp(format!("Server run error: {}", e))
        })
    }
}

#[derive(Clone)]
struct StudioMcpBackend {
    inner: Arc<StudioMcpServer>,
}

#[async_trait]
impl McpBackend for StudioMcpBackend {
    type Config = ();
    type Error = StudioError;

    async fn initialize(_: Self::Config) -> std::result::Result<Self, Self::Error> {
        Err(StudioError::Config("Use StudioMcpServer::new() instead".to_string()))
    }

    fn get_server_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::default(),
            name: "studio-mcp-server".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            capabilities: ServerCapabilities {
                resources: Some(ResourcesCapability {
                    subscribe: Some(false),
                    list_changed: Some(false),
                }),
                tools: Some(ToolsCapability {}),
                prompts: None,
                sampling: None,
                experimental: None,
            },
            instructions: Some("WindRiver Studio MCP Server providing access to Studio CLI functionality, focusing on Pipeline Management (PLM) features.".to_string()),
        }
    }

    async fn list_tools(&self, _params: ListToolsParams) -> std::result::Result<ListToolsResult, Self::Error> {
        debug!("Listing tools");
        
        let tools = self.inner.tool_provider.list_tools().await?;
        
        debug!("Found {} tools", tools.len());
        Ok(ListToolsResult {
            tools,
            next_cursor: None,
        })
    }

    async fn call_tool(&self, params: CallToolParams) -> std::result::Result<CallToolResult, Self::Error> {
        debug!("Calling tool: {}", params.name);
        
        let content = self.inner.tool_provider.call_tool(&params.name, params.arguments).await?;
        
        debug!("Successfully called tool: {}", params.name);
        Ok(CallToolResult {
            content,
            is_error: Some(false),
        })
    }

    async fn list_resources(&self, _params: ListResourcesParams) -> std::result::Result<ListResourcesResult, Self::Error> {
        debug!("Listing resources");
        
        let resources = self.inner.resource_provider.list_resources().await?;
        
        debug!("Found {} resources", resources.len());
        Ok(ListResourcesResult {
            resources,
            next_cursor: None,
        })
    }

    async fn read_resource(&self, params: ReadResourceParams) -> std::result::Result<ReadResourceResult, Self::Error> {
        debug!("Reading resource: {}", params.uri);
        
        let contents = self.inner.resource_provider.read_resource(&params.uri).await?;
        
        debug!("Successfully read resource: {}", params.uri);
        Ok(ReadResourceResult {
            contents,
        })
    }
}