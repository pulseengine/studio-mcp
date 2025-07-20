//! WindRiver Studio MCP Server
//! 
//! This server provides Model Context Protocol access to WindRiver Studio CLI functionality,
//! with a focus on Pipeline Management (PLM) features.

use std::env;
use tracing::{info, error};
use tracing_subscriber::{EnvFilter, fmt};

mod auth_middleware;
mod server;
mod resources;
mod tools;

use server::StudioMcpServer;
use studio_mcp_shared::StudioConfig;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    init_logging();

    info!("Starting WindRiver Studio MCP Server");

    // Parse command line arguments
    let args: Vec<String> = env::args().collect();
    let config_path = args.get(1).map(|s| s.as_str());

    // Load configuration
    let config = match StudioConfig::load_or_default(config_path) {
        Ok(config) => {
            info!("Configuration loaded successfully");
            config
        }
        Err(e) => {
            error!("Failed to load configuration: {}", e);
            StudioConfig::default()
        }
    };

    // Create and run the MCP server
    let server = StudioMcpServer::new(config).await?;
    
    info!("Studio MCP Server initialized, starting main loop");
    
    match server.run().await {
        Ok(_) => {
            info!("Studio MCP Server shut down gracefully");
            Ok(())
        }
        Err(e) => {
            error!("Studio MCP Server error: {}", e);
            Err(e.into())
        }
    }
}

fn init_logging() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));

    let subscriber = fmt::Subscriber::builder()
        .with_env_filter(filter)
        .with_target(false)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .finish();

    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set global logging subscriber");
}