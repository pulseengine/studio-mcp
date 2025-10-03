//! WindRiver Studio CLI manager - handles downloading, updating, and executing the CLI

pub mod auth_cli;
pub mod downloader;
pub mod executor;
pub mod version;

pub use auth_cli::{AuthenticatedCliManager, AuthenticatedCommand};
pub use downloader::CliDownloader;
pub use executor::CliExecutor;
pub use version::VersionManager;

use directories::ProjectDirs;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use studio_mcp_shared::Result;
use tokio::sync::RwLock;

/// Hook function type for CLI operation callbacks
pub type OperationHook = Arc<dyn Fn(&str, &[&str], &serde_json::Value) + Send + Sync>;

/// Main CLI manager that orchestrates CLI operations
pub struct CliManager {
    downloader: CliDownloader,
    executor: CliExecutor,
    version_manager: VersionManager,
    install_dir: PathBuf,
    /// Hooks that are called after CLI operations complete
    operation_hooks: Arc<RwLock<Vec<OperationHook>>>,
}

impl CliManager {
    pub fn new(base_url: String, install_dir: Option<PathBuf>) -> Result<Self> {
        let install_dir = install_dir.unwrap_or_else(|| {
            ProjectDirs::from("com", "pulseengine", "studio-mcp")
                .expect("Failed to get project directories")
                .data_dir()
                .join("cli")
        });

        std::fs::create_dir_all(&install_dir)?;

        Ok(Self {
            downloader: CliDownloader::new(base_url),
            executor: CliExecutor::new(install_dir.clone()),
            version_manager: VersionManager::new(install_dir.clone()),
            install_dir,
            operation_hooks: Arc::new(RwLock::new(Vec::new())),
        })
    }

    /// Ensure CLI is available and up-to-date
    pub async fn ensure_cli(&self, version: Option<&str>) -> Result<PathBuf> {
        let target_version = match version {
            Some(v) if v != "auto" => v.to_string(),
            _ => self.version_manager.get_latest_version().await?,
        };

        let cli_path = self.get_cli_path(&target_version);

        if !cli_path.exists() || self.version_manager.should_update(&target_version).await? {
            tracing::info!("Downloading/updating CLI version: {}", target_version);
            self.download_cli(&target_version).await?;
        }

        Ok(cli_path)
    }

    /// Download and install specific CLI version
    pub async fn download_cli(&self, version: &str) -> Result<PathBuf> {
        let cli_version = self.version_manager.get_version_info(version).await?;
        let cli_path = self.get_cli_path(version);

        self.downloader
            .download_and_install(&cli_version, &cli_path)
            .await?;

        // Make executable on Unix-like systems
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&cli_path)?.permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&cli_path, perms)?;
        }

        Ok(cli_path)
    }

    /// Register an operation hook that will be called after CLI operations
    pub async fn register_operation_hook(&self, hook: OperationHook) {
        let mut hooks = self.operation_hooks.write().await;
        hooks.push(hook);
    }

    /// Extract operation name and parameters from CLI arguments
    fn extract_operation_info(args: &[&str]) -> (String, HashMap<String, String>) {
        let mut operation = String::new();
        let mut parameters = HashMap::new();

        if args.is_empty() {
            return (operation, parameters);
        }

        // Build operation name from command structure
        let mut operation_parts = Vec::new();
        let mut param_key: Option<String> = None;

        for arg in args {
            if arg.starts_with("--") {
                // This is a parameter flag
                param_key = Some(arg.trim_start_matches("--").to_string());
            } else if let Some(key) = param_key.take() {
                // This is a parameter value
                parameters.insert(key, arg.to_string());
            } else {
                // This is part of the operation
                operation_parts.push(arg.to_string());
            }
        }

        operation = operation_parts.join(".");

        // Extract common parameters that might be useful for cache invalidation
        if let Some(pipeline_idx) = args
            .iter()
            .position(|&arg| arg == "--pipeline" || arg == "-p")
            && let Some(pipeline_id) = args.get(pipeline_idx + 1)
        {
            parameters.insert("pipeline_id".to_string(), pipeline_id.to_string());
        }

        if let Some(run_idx) = args.iter().position(|&arg| arg == "--run" || arg == "-r")
            && let Some(run_id) = args.get(run_idx + 1)
        {
            parameters.insert("run_id".to_string(), run_id.to_string());
        }

        // For commands like "plm pipeline create my-pipeline", extract the pipeline name
        if operation_parts.len() >= 3
            && operation_parts[0] == "plm"
            && operation_parts[1] == "pipeline"
            && let Some(pipeline_name) = operation_parts.get(3)
        {
            parameters.insert("pipeline_name".to_string(), pipeline_name.to_string());
        }

        (operation, parameters)
    }

    /// Check if an operation is a write operation that should trigger cache invalidation
    fn is_write_operation(operation: &str) -> bool {
        let write_operations = [
            "create",
            "update",
            "delete",
            "start",
            "stop",
            "cancel",
            "complete",
            "assign",
            "revoke",
            "lock",
            "unlock",
            "import",
            "export",
            "deploy",
            "install",
            "uninstall",
            "enable",
            "disable",
            "restart",
        ];

        write_operations
            .iter()
            .any(|&write_op| operation.contains(write_op))
    }

    /// Execute a CLI command
    pub async fn execute(
        &self,
        args: &[&str],
        working_dir: Option<&Path>,
    ) -> Result<serde_json::Value> {
        let cli_path = self.ensure_cli(None).await?;
        let result = self.executor.execute(&cli_path, args, working_dir).await?;

        // Extract operation information for hooks
        let (operation, _parameters) = Self::extract_operation_info(args);

        // Only trigger hooks for write operations
        if Self::is_write_operation(&operation) {
            self.trigger_operation_hooks(&operation, args, &result)
                .await;
        }

        Ok(result)
    }

    /// Trigger all registered operation hooks
    async fn trigger_operation_hooks(
        &self,
        operation: &str,
        args: &[&str],
        result: &serde_json::Value,
    ) {
        let hooks = self.operation_hooks.read().await;
        for hook in hooks.iter() {
            hook(operation, args, result);
        }
    }

    /// Execute a CLI command with custom timeout
    pub async fn execute_with_timeout(
        &self,
        args: &[&str],
        working_dir: Option<&Path>,
        timeout_duration: std::time::Duration,
    ) -> Result<serde_json::Value> {
        let cli_path = self.ensure_cli(None).await?;
        self.executor
            .execute_with_timeout(&cli_path, args, working_dir, timeout_duration)
            .await
    }

    /// Get the path where CLI should be installed for a given version
    fn get_cli_path(&self, version: &str) -> PathBuf {
        let filename = if cfg!(windows) {
            "studio-cli.exe"
        } else {
            "studio-cli"
        };

        self.install_dir.join(version).join(filename)
    }

    /// List installed CLI versions
    pub fn list_installed_versions(&self) -> Result<Vec<String>> {
        let mut versions = Vec::new();

        if self.install_dir.exists() {
            for entry in std::fs::read_dir(&self.install_dir)? {
                let entry = entry?;
                if entry.file_type()?.is_dir()
                    && let Some(name) = entry.file_name().to_str()
                {
                    let cli_path = self.get_cli_path(name);
                    if cli_path.exists() {
                        versions.push(name.to_string());
                    }
                }
            }
        }

        versions.sort();
        Ok(versions)
    }

    /// Remove old CLI versions, keeping only the latest N versions
    pub fn cleanup_old_versions(&self, keep_count: usize) -> Result<()> {
        let mut versions = self.list_installed_versions()?;

        if versions.len() <= keep_count {
            return Ok(());
        }

        versions.sort();
        let to_remove = &versions[..versions.len() - keep_count];

        for version in to_remove {
            let version_dir = self.install_dir.join(version);
            if version_dir.exists() {
                tracing::info!("Removing old CLI version: {}", version);
                std::fs::remove_dir_all(version_dir)?;
            }
        }

        Ok(())
    }
}
