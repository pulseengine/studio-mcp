//! Error types for WindRiver Studio MCP server

use thiserror::Error;

pub type Result<T> = std::result::Result<T, StudioError>;

// Re-export BackendError and Error for compatibility
pub use pulseengine_mcp_server::{BackendError, Error};

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

// Implement required traits for PulseEngine MCP compatibility
impl From<BackendError> for StudioError {
    fn from(err: BackendError) -> Self {
        match err {
            BackendError::NotInitialized => StudioError::Mcp("Backend not initialized".to_string()),
            BackendError::Configuration(msg) => StudioError::Config(msg),
            BackendError::Connection(msg) => StudioError::Mcp(format!("Connection error: {}", msg)),
            BackendError::NotSupported(msg) => StudioError::InvalidOperation(msg),
            BackendError::Internal(msg) => StudioError::Mcp(msg),
            BackendError::Custom(err) => StudioError::Unknown(err.to_string()),
        }
    }
}

impl From<StudioError> for Error {
    fn from(err: StudioError) -> Self {
        match err {
            StudioError::Cli(msg) => Error::internal_error(format!("CLI error: {}", msg)),
            StudioError::Auth(msg) => {
                Error::invalid_params(format!("Authentication error: {}", msg))
            }
            StudioError::Network(err) => Error::internal_error(format!("Network error: {}", err)),
            StudioError::Io(err) => Error::internal_error(format!("IO error: {}", err)),
            StudioError::Json(err) => Error::invalid_params(format!("JSON error: {}", err)),
            StudioError::UrlParse(err) => {
                Error::invalid_params(format!("URL parse error: {}", err))
            }
            StudioError::Mcp(msg) => Error::internal_error(msg),
            StudioError::Config(msg) => {
                Error::invalid_params(format!("Configuration error: {}", msg))
            }
            StudioError::ResourceNotFound(msg) => {
                Error::invalid_request(format!("Resource not found: {}", msg))
            }
            StudioError::InvalidOperation(msg) => Error::method_not_found(msg),
            StudioError::Timeout(msg) => Error::internal_error(format!("Timeout: {}", msg)),
            StudioError::ChecksumMismatch => Error::internal_error("Checksum verification failed"),
            StudioError::Unknown(msg) => Error::internal_error(format!("Unknown error: {}", msg)),
        }
    }
}
