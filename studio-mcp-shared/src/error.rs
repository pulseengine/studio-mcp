//! Error types for WindRiver Studio MCP server

use thiserror::Error;

pub type Result<T> = std::result::Result<T, StudioError>;

#[derive(Error, Debug)]
pub enum StudioError {
    #[error("CLI error: {0}")]
    Cli(String),

    #[error("Authentication error: {0}")]
    Auth(String),

    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("URL parse error: {0}")]
    UrlParse(#[from] url::ParseError),

    #[error("MCP protocol error: {0}")]
    Mcp(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Resource not found: {0}")]
    ResourceNotFound(String),

    #[error("Invalid operation: {0}")]
    InvalidOperation(String),

    #[error("Timeout error: {0}")]
    Timeout(String),

    #[error("Checksum verification failed")]
    ChecksumMismatch,

    #[error("Unknown error: {0}")]
    Unknown(String),
}