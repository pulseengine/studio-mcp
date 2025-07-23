//! Shared types and utilities for WindRiver Studio MCP server

pub mod auth;
pub mod auth_service;
pub mod config;
pub mod error;
pub mod token_validator;
pub mod types;

pub use auth::{AuthCredentials, AuthManager, AuthToken, TokenStorage};
pub use auth_service::{InstanceStatus, StudioAuthService, StudioInstance};
pub use config::{
    CacheConfig, CliConfig, LoggingConfig, OperationType, StudioConfig, TimeoutConfig,
};
pub use error::{Result, StudioError};
pub use token_validator::{StudioTokenClaims, TokenValidator, ValidationResult};
pub use types::*;
