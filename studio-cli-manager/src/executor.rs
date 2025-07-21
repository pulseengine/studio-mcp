//! CLI executor - handles executing CLI commands and parsing output

use studio_mcp_shared::{Result, StudioError};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::process::Command;
use serde_json::Value;

pub struct CliExecutor {
    #[allow(dead_code)]
    install_dir: PathBuf,
}

impl CliExecutor {
    pub fn new(install_dir: PathBuf) -> Self {
        Self { install_dir }
    }

    /// Execute CLI command and return parsed JSON output
    pub async fn execute(
        &self,
        cli_path: &Path,
        args: &[&str],
        working_dir: Option<&Path>,
    ) -> Result<Value> {
        tracing::debug!("Executing CLI: {} {}", cli_path.display(), args.join(" "));

        let mut cmd = Command::new(cli_path);
        cmd.args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        if let Some(dir) = working_dir {
            cmd.current_dir(dir);
        }

        // Add default arguments for JSON output and non-interactive mode
        let mut full_args = vec!["--output", "json", "--non-interactive"];
        full_args.extend_from_slice(args);
        cmd.args(&full_args[2..]); // Skip the first two as they're already added

        let output = cmd.output().await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            
            tracing::error!(
                "CLI command failed with status {}: stderr={}, stdout={}",
                output.status,
                stderr,
                stdout
            );

            return Err(StudioError::Cli(format!(
                "Command failed with status {}: {}",
                output.status,
                stderr
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        tracing::debug!("CLI output: {}", stdout);

        // Try to parse as JSON
        if stdout.trim().is_empty() {
            return Ok(Value::Null);
        }

        serde_json::from_str(&stdout).map_err(|e| {
            tracing::error!("Failed to parse CLI output as JSON: {}", e);
            StudioError::Json(e)
        })
    }

    /// Execute CLI command with streaming output
    pub async fn execute_streaming<F>(
        &self,
        cli_path: &Path,
        args: &[&str],
        working_dir: Option<&Path>,
        mut output_handler: F,
    ) -> Result<()>
    where
        F: FnMut(String) -> Result<()>,
    {
        use tokio::io::{AsyncBufReadExt, BufReader};

        tracing::debug!("Executing CLI with streaming: {} {}", cli_path.display(), args.join(" "));

        let mut cmd = Command::new(cli_path);
        cmd.args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        if let Some(dir) = working_dir {
            cmd.current_dir(dir);
        }

        let mut child = cmd.spawn()?;
        
        if let Some(stdout) = child.stdout.take() {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();
            
            while let Some(line) = lines.next_line().await? {
                output_handler(line)?;
            }
        }

        let status = child.wait().await?;
        
        if !status.success() {
            return Err(StudioError::Cli(format!(
                "Streaming command failed with status {}",
                status
            )));
        }

        Ok(())
    }

    /// Check if CLI is available and working
    pub async fn check_cli(&self, cli_path: &Path) -> Result<bool> {
        match self.execute(cli_path, &["--version"], None).await {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    /// Get CLI version information
    pub async fn get_version(&self, cli_path: &Path) -> Result<String> {
        let output = self.execute(cli_path, &["--version"], None).await?;
        
        // Parse version from output
        if let Some(version_str) = output.as_str() {
            Ok(version_str.to_string())
        } else if let Some(obj) = output.as_object() {
            if let Some(version) = obj.get("version").and_then(|v| v.as_str()) {
                Ok(version.to_string())
            } else {
                Err(StudioError::Cli("Unable to parse version from output".to_string()))
            }
        } else {
            Err(StudioError::Cli("Unexpected version output format".to_string()))
        }
    }

    /// Execute PLM-specific commands
    pub async fn plm_list_pipelines(&self, cli_path: &Path, project_id: Option<&str>) -> Result<Value> {
        let mut args = vec!["plm", "pipeline", "list"];
        
        if let Some(project) = project_id {
            args.extend_from_slice(&["--project", project]);
        }
        
        self.execute(cli_path, &args, None).await
    }

    pub async fn plm_get_pipeline(&self, cli_path: &Path, pipeline_id: &str) -> Result<Value> {
        let args = vec!["plm", "pipeline", "get", pipeline_id];
        self.execute(cli_path, &args, None).await
    }

    pub async fn plm_list_tasks(&self, cli_path: &Path, pipeline_id: &str) -> Result<Value> {
        let args = vec!["plm", "task", "list", "--pipeline", pipeline_id];
        self.execute(cli_path, &args, None).await
    }

    pub async fn plm_get_task(&self, cli_path: &Path, task_id: &str) -> Result<Value> {
        let args = vec!["plm", "task", "get", task_id];
        self.execute(cli_path, &args, None).await
    }

    pub async fn plm_get_task_logs(&self, cli_path: &Path, task_id: &str) -> Result<Value> {
        let args = vec!["plm", "task", "logs", task_id];
        self.execute(cli_path, &args, None).await
    }

    pub async fn plm_run_pipeline(&self, cli_path: &Path, pipeline_id: &str) -> Result<Value> {
        let args = vec!["plm", "pipeline", "run", pipeline_id];
        self.execute(cli_path, &args, None).await
    }

    pub async fn plm_stop_pipeline(&self, cli_path: &Path, pipeline_id: &str) -> Result<Value> {
        let args = vec!["plm", "pipeline", "stop", pipeline_id];
        self.execute(cli_path, &args, None).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;

    #[tokio::test]
    async fn test_executor_creation() {
        let temp_dir = TempDir::new().unwrap();
        let executor = CliExecutor::new(temp_dir.path().to_path_buf());
        assert_eq!(executor.install_dir, temp_dir.path());
    }

    #[tokio::test]
    async fn test_check_cli_with_nonexistent_binary() {
        let temp_dir = TempDir::new().unwrap();
        let executor = CliExecutor::new(temp_dir.path().to_path_buf());
        let fake_cli_path = temp_dir.path().join("nonexistent-cli");
        
        let result = executor.check_cli(&fake_cli_path).await.unwrap();
        assert!(!result);
    }

    // Note: More comprehensive tests would require a mock CLI binary
    // or integration tests with the actual studio-cli
}