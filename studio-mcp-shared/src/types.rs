//! Shared types for WindRiver Studio

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Studio CLI version information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CliVersion {
    pub version: String,
    pub platform: String,
    pub url: String,
    pub checksum: String,
    pub file_name: String,
}

/// Studio connection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StudioConnection {
    pub name: String,
    pub url: String,
    pub username: Option<String>,
    pub token: Option<String>,
}

/// Pipeline information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pipeline {
    pub id: String,
    pub name: String,
    pub project_id: String,
    pub status: PipelineStatus,
    pub created_at: String,
    pub updated_at: String,
    pub config: Option<PipelineConfig>,
}

/// Pipeline status
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PipelineStatus {
    Running,
    Stopped,
    Failed,
    Success,
    Pending,
    Aborted,
}

/// Pipeline configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineConfig {
    pub stages: Vec<PipelineStage>,
    pub variables: HashMap<String, String>,
    pub triggers: Vec<PipelineTrigger>,
}

/// Pipeline stage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineStage {
    pub name: String,
    pub tasks: Vec<PipelineTask>,
}

/// Pipeline task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineTask {
    pub id: String,
    pub name: String,
    pub status: TaskStatus,
    pub stage: String,
    pub created_at: String,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    pub duration: Option<u64>,
    pub logs_url: Option<String>,
    pub artifacts: Vec<TaskArtifact>,
}

/// Task status
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    Running,
    Pending,
    Success,
    Failed,
    Cancelled,
    Skipped,
}

/// Task artifact
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskArtifact {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub created_at: String,
    pub download_url: Option<String>,
}

/// Pipeline trigger
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineTrigger {
    pub name: String,
    pub trigger_type: TriggerType,
    pub config: HashMap<String, String>,
}

/// Trigger type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TriggerType {
    Manual,
    Schedule,
    Webhook,
    GitPush,
    GitTag,
}

/// Project information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub owner: String,
    pub visibility: ProjectVisibility,
}

/// Project visibility
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProjectVisibility {
    Public,
    Private,
    Internal,
}

/// MCP Resource URI components
#[derive(Debug, Clone)]
pub struct ResourceUri {
    pub scheme: String,
    pub path: Vec<String>,
    pub query: HashMap<String, String>,
}

impl ResourceUri {
    pub fn parse(uri: &str) -> crate::Result<Self> {
        let parsed = url::Url::parse(uri)?;
        
        if parsed.scheme() != "studio" {
            return Err(crate::StudioError::InvalidOperation(
                format!("Invalid scheme: {}", parsed.scheme())
            ));
        }

        let path: Vec<String> = parsed
            .path()
            .trim_start_matches('/')
            .split('/')
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect();

        let query: HashMap<String, String> = parsed
            .query_pairs()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();

        Ok(Self {
            scheme: parsed.scheme().to_string(),
            path,
            query,
        })
    }

    pub fn to_string(&self) -> String {
        let path = self.path.join("/");
        let query = if self.query.is_empty() {
            String::new()
        } else {
            let query_string: Vec<String> = self.query
                .iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect();
            format!("?{}", query_string.join("&"))
        };
        
        format!("{}:/{}{}", self.scheme, path, query)
    }
}