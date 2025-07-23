//! Version management - handles CLI version discovery and updates

use reqwest::Client;
use std::path::PathBuf;
use studio_mcp_shared::{CliVersion, Result, StudioError};

pub struct VersionManager {
    #[allow(dead_code)]
    client: Client,
    install_dir: PathBuf,
    cache: tokio::sync::RwLock<Option<(std::time::Instant, Vec<CliVersion>)>>,
}

impl VersionManager {
    pub fn new(install_dir: PathBuf) -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            install_dir,
            cache: tokio::sync::RwLock::new(None),
        }
    }

    /// Get the latest available version
    pub async fn get_latest_version(&self) -> Result<String> {
        let versions = self.fetch_available_versions().await?;

        versions
            .into_iter()
            .map(|v| v.version)
            .max_by(|a, b| match version_compare::compare(a, b) {
                Ok(version_compare::Cmp::Gt) => std::cmp::Ordering::Greater,
                Ok(version_compare::Cmp::Lt) => std::cmp::Ordering::Less,
                Ok(version_compare::Cmp::Eq) => std::cmp::Ordering::Equal,
                _ => std::cmp::Ordering::Equal,
            })
            .ok_or_else(|| StudioError::Config("No versions available".to_string()))
    }

    /// Get version information for a specific version
    pub async fn get_version_info(&self, version: &str) -> Result<CliVersion> {
        // For now, we'll construct the version info based on the pattern
        // In a real implementation, this might come from an API or manifest file
        let platform = self.detect_platform();
        let (platform_dir, file_extension) = self.get_platform_info(platform);

        let base_url = "https://distro.windriver.com/dist/wrstudio/wrstudio-cli-distro-cd";
        let url = format!(
            "{}/{}/{}/studio-cli{}",
            base_url, version, platform_dir, file_extension
        );

        Ok(CliVersion {
            version: version.to_string(),
            platform: platform.to_string(),
            url,
            checksum: self.get_checksum_for_version(version, platform),
            file_name: format!(
                "studio-cli{}",
                if platform == "windows" { ".exe" } else { "" }
            ),
        })
    }

    /// Check if CLI should be updated
    pub async fn should_update(&self, current_version: &str) -> Result<bool> {
        let latest = self.get_latest_version().await?;

        match version_compare::compare(&latest, current_version) {
            Ok(version_compare::Cmp::Gt) => Ok(true),
            Ok(_) => Ok(false),
            Err(_) => Ok(false), // If comparison fails, don't update
        }
    }

    /// Fetch available versions (with caching)
    async fn fetch_available_versions(&self) -> Result<Vec<CliVersion>> {
        const CACHE_DURATION: std::time::Duration = std::time::Duration::from_secs(3600); // 1 hour

        {
            let cache = self.cache.read().await;
            if let Some((timestamp, versions)) = cache.as_ref() {
                if timestamp.elapsed() < CACHE_DURATION {
                    return Ok(versions.clone());
                }
            }
        }

        // For now, return a hardcoded list of known versions
        // In a real implementation, this would fetch from an API or parse directory listings
        let versions = self.get_known_versions();

        {
            let mut cache = self.cache.write().await;
            *cache = Some((std::time::Instant::now(), versions.clone()));
        }

        Ok(versions)
    }

    /// Get known versions (hardcoded for now)
    fn get_known_versions(&self) -> Vec<CliVersion> {
        let platform = self.detect_platform();
        let (platform_dir, file_extension) = self.get_platform_info(platform);
        let base_url = "https://distro.windriver.com/dist/wrstudio/wrstudio-cli-distro-cd";

        // Known versions - in a real implementation, this would be dynamic
        let versions = vec!["24.3.0", "24.2.0", "24.1.0"];

        versions
            .into_iter()
            .map(|version| {
                let url = format!(
                    "{}/{}/{}/studio-cli{}",
                    base_url, version, platform_dir, file_extension
                );

                CliVersion {
                    version: version.to_string(),
                    platform: platform.to_string(),
                    url,
                    checksum: self.get_checksum_for_version(version, platform),
                    file_name: format!(
                        "studio-cli{}",
                        if platform == "windows" { ".exe" } else { "" }
                    ),
                }
            })
            .collect()
    }

    /// Get checksum for a version (hardcoded for now)
    fn get_checksum_for_version(&self, version: &str, platform: &str) -> String {
        // These would normally come from a manifest file
        match (version, platform) {
            ("24.3.0", "linux") => "84a03899b5818de24a398f5c7718db00bf2f4439".to_string(),
            ("24.3.0", "windows") => "b12fbf72a24cd31cfbf23975060c061db881300b".to_string(),
            ("24.3.0", "macos") => "ee5e90a3d838739b57ff8804b489b97499210ef4".to_string(),
            _ => String::new(), // Unknown checksum
        }
    }

    /// Detect current platform
    fn detect_platform(&self) -> &'static str {
        match std::env::consts::OS {
            "windows" => "windows",
            "linux" => "linux",
            "macos" => "macos",
            _ => "linux",
        }
    }

    /// Get platform-specific information
    fn get_platform_info(&self, platform: &str) -> (&'static str, &'static str) {
        match platform {
            "windows" => ("win64", ".exe.gz"),
            "linux" => ("linux", ".gz"),
            "macos" => ("macos", ".gz"),
            _ => ("linux", ".gz"),
        }
    }

    /// Clear version cache
    pub async fn clear_cache(&self) {
        let mut cache = self.cache.write().await;
        *cache = None;
    }

    /// Check if a specific version is available
    pub async fn is_version_available(&self, version: &str) -> Result<bool> {
        let versions = self.fetch_available_versions().await?;
        Ok(versions.iter().any(|v| v.version == version))
    }

    /// Get installed version from CLI binary
    pub async fn get_installed_version(&self, cli_path: &std::path::Path) -> Result<String> {
        use crate::executor::CliExecutor;

        let executor = CliExecutor::new(self.install_dir.clone());
        executor.get_version(cli_path).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_version_manager_creation() {
        let temp_dir = TempDir::new().unwrap();
        let version_manager = VersionManager::new(temp_dir.path().to_path_buf());
        assert_eq!(version_manager.install_dir, temp_dir.path());
    }

    #[tokio::test]
    async fn test_platform_detection() {
        let temp_dir = TempDir::new().unwrap();
        let version_manager = VersionManager::new(temp_dir.path().to_path_buf());
        let platform = version_manager.detect_platform();
        assert!(["windows", "linux", "macos"].contains(&platform));
    }

    #[tokio::test]
    async fn test_platform_info() {
        let temp_dir = TempDir::new().unwrap();
        let version_manager = VersionManager::new(temp_dir.path().to_path_buf());

        let (dir, ext) = version_manager.get_platform_info("linux");
        assert_eq!(dir, "linux");
        assert_eq!(ext, ".gz");

        let (dir, ext) = version_manager.get_platform_info("windows");
        assert_eq!(dir, "win64");
        assert_eq!(ext, ".exe.gz");

        let (dir, ext) = version_manager.get_platform_info("macos");
        assert_eq!(dir, "macos");
        assert_eq!(ext, ".gz");
    }

    #[tokio::test]
    async fn test_version_info_generation() {
        let temp_dir = TempDir::new().unwrap();
        let version_manager = VersionManager::new(temp_dir.path().to_path_buf());

        let version_info = version_manager.get_version_info("24.3.0").await.unwrap();
        assert_eq!(version_info.version, "24.3.0");
        assert!(version_info.url.contains("24.3.0"));
        assert!(!version_info.file_name.is_empty());
    }

    #[tokio::test]
    async fn test_cache_functionality() {
        let temp_dir = TempDir::new().unwrap();
        let version_manager = VersionManager::new(temp_dir.path().to_path_buf());

        // Clear cache first
        version_manager.clear_cache().await;

        // Fetch versions (should populate cache)
        let versions1 = version_manager.fetch_available_versions().await.unwrap();

        // Fetch again (should use cache)
        let versions2 = version_manager.fetch_available_versions().await.unwrap();

        assert_eq!(versions1.len(), versions2.len());
    }
}
