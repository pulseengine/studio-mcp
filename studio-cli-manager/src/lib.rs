//! WindRiver Studio CLI manager - handles downloading, updating, and executing the CLI

pub mod auth_cli;
pub mod downloader;
pub mod executor;
pub mod version;

pub use auth_cli::{AuthenticatedCliManager, AuthenticatedCommand};
pub use downloader::CliDownloader;
pub use executor::CliExecutor;
pub use version::VersionManager;

use studio_mcp_shared::Result;
use std::path::{Path, PathBuf};
use directories::ProjectDirs;

/// Main CLI manager that orchestrates CLI operations
pub struct CliManager {
    downloader: CliDownloader,
    executor: CliExecutor,
    version_manager: VersionManager,
    install_dir: PathBuf,
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
        
        self.downloader.download_and_install(&cli_version, &cli_path).await?;
        
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

    /// Execute a CLI command
    pub async fn execute(&self, args: &[&str], working_dir: Option<&Path>) -> Result<serde_json::Value> {
        let cli_path = self.ensure_cli(None).await?;
        self.executor.execute(&cli_path, args, working_dir).await
    }

    /// Execute a CLI command with custom timeout
    pub async fn execute_with_timeout(
        &self, 
        args: &[&str], 
        working_dir: Option<&Path>,
        timeout_duration: std::time::Duration,
    ) -> Result<serde_json::Value> {
        let cli_path = self.ensure_cli(None).await?;
        self.executor.execute_with_timeout(&cli_path, args, working_dir, timeout_duration).await
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
                if entry.file_type()?.is_dir() {
                    if let Some(name) = entry.file_name().to_str() {
                        let cli_path = self.get_cli_path(name);
                        if cli_path.exists() {
                            versions.push(name.to_string());
                        }
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