//! WindRiver Studio MCP Server
//!
//! This server provides Model Context Protocol access to WindRiver Studio CLI functionality,
//! with a focus on Pipeline Management (PLM) features.

use std::collections::HashMap;
use std::env;
use tracing::{error, info};
use tracing_subscriber::{fmt, EnvFilter};

mod auth_middleware;
mod resources;
mod server;
mod tools;

use server::StudioMcpServer;
use studio_mcp_shared::{CacheConfig, CliConfig, LoggingConfig, StudioConfig, StudioConnection};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    init_logging();

    info!("Starting WindRiver Studio MCP Server");

    // Parse command line arguments
    let args: Vec<String> = env::args().collect();

    // Check for --init flag
    if args.contains(&"--init".to_string()) {
        return init_config(&args).await;
    }

    let config_path = args.get(1).map(|s| s.as_str());

    // Load configuration
    let config = match StudioConfig::load_or_default(config_path) {
        Ok(config) => {
            // Validate configuration has at least one connection
            if config.connections.is_empty() {
                error!(
                    "No connections configured. Run with --init to create a default configuration."
                );
                eprintln!("Error: No connections configured.");
                eprintln!(
                    "Run `{} --init` to create a default configuration.",
                    args[0]
                );
                std::process::exit(1);
            }
            info!("Configuration loaded successfully");
            config
        }
        Err(e) => {
            error!("Failed to load configuration: {}", e);
            eprintln!("Error loading configuration: {}", e);
            eprintln!(
                "Run `{} --init` to create a default configuration.",
                args[0]
            );
            std::process::exit(1);
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
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

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

async fn init_config(args: &[String]) -> anyhow::Result<()> {
    // Find the config path, skipping the --init flag
    let config_path = if args.len() > 2 && args[1] == "--init" {
        &args[2]
    } else if args.len() > 2 && args[2] == "--init" {
        &args[1]
    } else {
        "config.json"
    };

    // Check if config already exists
    if std::path::Path::new(config_path).exists() {
        eprintln!("Configuration file '{}' already exists.", config_path);
        eprintln!("Remove it first if you want to create a new one.");
        std::process::exit(1);
    }

    // Create default configuration with mock server connection
    let mut connections = HashMap::new();
    connections.insert(
        "mock_studio".to_string(),
        StudioConnection {
            name: "Mock Studio Server".to_string(),
            url: "http://localhost:8080".to_string(),
            username: Some("admin".to_string()),
            token: None,
        },
    );

    let config = StudioConfig {
        connections,
        default_connection: Some("mock_studio".to_string()),
        cli: CliConfig::default(),
        cache: CacheConfig::default(),
        logging: LoggingConfig::default(),
    };

    // Save configuration
    match config.save(config_path) {
        Ok(_) => {
            println!(
                "✅ Configuration file '{}' created successfully!",
                config_path
            );
            println!();
            println!("Default configuration includes:");
            println!("  • Mock Studio server connection (localhost:8080)");
            println!("  • Optimized timeout settings for different operations");
            println!("  • Debug logging enabled");
            println!();
            println!("To start the mock server for testing:");
            println!("  cd mock-studio-server && docker-compose up -d");
            println!();
            println!("To start the MCP server:");
            println!("  {} {}", args[0], config_path);
            Ok(())
        }
        Err(e) => {
            eprintln!("Failed to create configuration file: {}", e);
            std::process::exit(1);
        }
    }
}
