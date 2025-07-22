//! Shared types and utilities for WindRiver Studio MCP server

pub mod auth;
pub mod auth_service;
pub mod error;
pub mod token_validator;
pub mod types;
pub mod config;

pub use auth::{AuthManager, AuthToken, AuthCredentials, TokenStorage};
pub use auth_service::{StudioAuthService, StudioInstance, InstanceStatus};
pub use error::{StudioError, Result};
pub use token_validator::{TokenValidator, StudioTokenClaims, ValidationResult};
pub use types::*;
pub use config::{StudioConfig, OperationType, TimeoutConfig, CliConfig, CacheConfig, LoggingConfig};
