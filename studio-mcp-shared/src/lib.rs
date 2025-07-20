//! Shared types and utilities for WindRiver Studio MCP server

pub mod error;
pub mod types;
pub mod config;

pub use error::{StudioError, Result};
pub use types::*;
pub use config::StudioConfig;