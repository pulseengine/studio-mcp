//! Main MCP server implementation using PulseEngine MCP framework

use async_trait::async_trait;
use std::sync::Arc;
use tracing::{debug, info};

use pulseengine_mcp_protocol::{
    CallToolRequestParam, CallToolResult, Content, GetPromptRequestParam, GetPromptResult,
    Implementation, ListPromptsResult, ListResourcesResult, ListToolsResult, PaginatedRequestParam,
    ProtocolVersion, ReadResourceRequestParam, ReadResourceResult, ResourceContents,
    ResourcesCapability, ServerCapabilities, ServerInfo, ToolsCapability,
};
use pulseengine_mcp_server::{AuthConfig, McpBackend, McpServer, ServerConfig, TransportConfig};

use studio_cli_manager::CliManager;
use studio_mcp_shared::{Result, StudioConfig, StudioError};

use crate::resources::ResourceProvider;
use crate::tools::ToolProvider;

pub struct StudioMcpServer {
    #[allow(dead_code)]
    config: StudioConfig,
    cli_manager: Arc<CliManager>,
    resource_provider: Arc<ResourceProvider>,
    tool_provider: Arc<ToolProvider>,
}

impl StudioMcpServer {
    pub async fn new(config: StudioConfig) -> Result<Self> {
        info!("Initializing Studio MCP Server with PulseEngine framework");

        // Initialize CLI manager
        let cli_manager = Arc::new(CliManager::new(
            config.cli.download_base_url.clone(),
            config
                .cli
                .install_dir
                .as_ref()
                .map(std::path::PathBuf::from),
        )?);

        // Ensure CLI is available
        cli_manager
            .ensure_cli(if config.cli.version == "auto" {
                None
            } else {
                Some(&config.cli.version)
            })
            .await?;

        // Initialize providers
        let resource_provider =
            Arc::new(ResourceProvider::new(cli_manager.clone(), config.clone()));

        let tool_provider = Arc::new(ToolProvider::new(cli_manager.clone(), config.clone()));

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

        // Create server with memory-based auth and stdio transport
        let server_config = ServerConfig {
            auth_config: AuthConfig::memory(),
            transport_config: TransportConfig::stdio(),
            ..Default::default()
        };
        let mut server = McpServer::new(backend, server_config)
            .await
            .map_err(|e| StudioError::Mcp(format!("Failed to create server: {e}")))?;

        info!("Starting PulseEngine MCP server with stdio transport");

        server
            .run()
            .await
            .map_err(|e| StudioError::Mcp(format!("Server run error: {e}")))
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
        Err(StudioError::Config(
            "Use StudioMcpServer::new() instead".to_string(),
        ))
    }

    fn get_server_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::default(),
            capabilities: ServerCapabilities {
                resources: Some(ResourcesCapability {
                    subscribe: Some(false),
                    list_changed: Some(false),
                }),
                tools: Some(ToolsCapability {
                    list_changed: Some(false),
                }),
                prompts: None,
                sampling: None,
                logging: None,
            },
            server_info: Implementation {
                name: "studio-mcp-server".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
            instructions: Some("WindRiver Studio MCP Server providing access to Studio CLI functionality, focusing on Pipeline Management (PLM) features.".to_string()),
        }
    }

    async fn list_tools(
        &self,
        _request: PaginatedRequestParam,
    ) -> std::result::Result<ListToolsResult, Self::Error> {
        debug!("Listing tools");

        let tools = self.inner.tool_provider.list_tools().await?;

        debug!("Found {} tools", tools.len());
        Ok(ListToolsResult {
            tools,
            next_cursor: None,
        })
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParam,
    ) -> std::result::Result<CallToolResult, Self::Error> {
        debug!("Calling tool: {}", request.name);

        let content = self
            .inner
            .tool_provider
            .call_tool(&request.name, request.arguments)
            .await?;

        debug!("Successfully called tool: {}", request.name);
        Ok(CallToolResult {
            content,
            is_error: Some(false),
        })
    }

    async fn list_resources(
        &self,
        _request: PaginatedRequestParam,
    ) -> std::result::Result<ListResourcesResult, Self::Error> {
        debug!("Listing resources");

        let resources = self.inner.resource_provider.list_resources().await?;

        debug!("Found {} resources", resources.len());
        Ok(ListResourcesResult {
            resources,
            next_cursor: None,
        })
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParam,
    ) -> std::result::Result<ReadResourceResult, Self::Error> {
        debug!("Reading resource: {}", request.uri);

        let content = self
            .inner
            .resource_provider
            .read_resource(&request.uri)
            .await?;

        // Convert Content to ResourceContents
        let contents = content
            .into_iter()
            .map(|c| match c {
                Content::Text { text } => ResourceContents {
                    uri: request.uri.clone(),
                    mime_type: Some("text/plain".to_string()),
                    text: Some(text),
                    blob: None,
                },
                Content::Image { data, mime_type } => ResourceContents {
                    uri: request.uri.clone(),
                    mime_type: Some(mime_type),
                    text: None,
                    blob: Some(data),
                },
                Content::Resource { resource, text } => ResourceContents {
                    uri: resource,
                    mime_type: Some("application/json".to_string()),
                    text,
                    blob: None,
                },
            })
            .collect();

        debug!("Successfully read resource: {}", request.uri);
        Ok(ReadResourceResult { contents })
    }

    async fn health_check(&self) -> std::result::Result<(), Self::Error> {
        // Basic health check - verify CLI is available
        match self.inner.cli_manager.ensure_cli(None).await {
            Ok(_) => Ok(()),
            Err(e) => Err(StudioError::Cli(format!("Health check failed: {e}"))),
        }
    }

    async fn list_prompts(
        &self,
        _request: PaginatedRequestParam,
    ) -> std::result::Result<ListPromptsResult, Self::Error> {
        // Not implemented yet - return empty list
        Ok(ListPromptsResult {
            prompts: vec![],
            next_cursor: None,
        })
    }

    async fn get_prompt(
        &self,
        _request: GetPromptRequestParam,
    ) -> std::result::Result<GetPromptResult, Self::Error> {
        // Not implemented yet
        Err(StudioError::InvalidOperation(
            "Prompts not yet implemented".to_string(),
        ))
    }
}
