//! CLI downloader - handles downloading and verifying CLI binaries

use flate2::read::GzDecoder;
use reqwest::Client;
use sha1::{Digest, Sha1};
use std::io::{Read, Write};
use std::path::Path;
use studio_mcp_shared::{CliVersion, Result, StudioError};

pub struct CliDownloader {
    client: Client,
    base_url: String,
}

impl CliDownloader {
    pub fn new(base_url: String) -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(300)) // 5 minutes
            .build()
            .expect("Failed to create HTTP client");

        Self { client, base_url }
    }

    /// Download and install CLI binary
    pub async fn download_and_install(
        &self,
        cli_version: &CliVersion,
        target_path: &Path,
    ) -> Result<()> {
        tracing::info!("Downloading CLI from: {}", cli_version.url);

        // Create parent directory
        if let Some(parent) = target_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Download file
        let response = self.client.get(&cli_version.url).send().await?;

        if !response.status().is_success() {
            return Err(StudioError::Network(
                response.error_for_status().unwrap_err(),
            ));
        }

        let bytes = response.bytes().await?;

        // Verify checksum
        self.verify_checksum(&bytes, &cli_version.checksum)?;

        // Decompress if it's a gzip file
        let decompressed_data = if cli_version.url.ends_with(".gz") {
            self.decompress_gzip(&bytes)?
        } else {
            bytes.to_vec()
        };

        // Write to target file
        let mut file = std::fs::File::create(target_path)?;
        file.write_all(&decompressed_data)?;
        file.sync_all()?;

        tracing::info!("CLI installed to: {}", target_path.display());
        Ok(())
    }

    /// Verify file checksum
    fn verify_checksum(&self, data: &[u8], expected_checksum: &str) -> Result<()> {
        let mut hasher = Sha1::new();
        hasher.update(data);
        let computed = hex::encode(hasher.finalize());

        if computed != expected_checksum {
            tracing::error!(
                "Checksum mismatch. Expected: {}, Got: {}",
                expected_checksum,
                computed
            );
            return Err(StudioError::ChecksumMismatch);
        }

        tracing::debug!("Checksum verified: {}", computed);
        Ok(())
    }

    /// Decompress gzip data
    fn decompress_gzip(&self, data: &[u8]) -> Result<Vec<u8>> {
        let mut decoder = GzDecoder::new(data);
        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed)?;
        Ok(decompressed)
    }

    /// Get download URL for a specific version and platform
    pub fn get_download_url(&self, version: &str, platform: &str) -> String {
        let (platform_dir, file_extension) = match platform {
            "windows" => ("win64", ".exe.gz"),
            "linux" => ("linux", ".gz"),
            "macos" => ("darwin", ".gz"),
            _ => ("linux", ".gz"), // default to linux
        };

        format!(
            "{}/{}/{}/studio-cli{}",
            self.base_url, version, platform_dir, file_extension
        )
    }

    /// Detect current platform
    pub fn detect_platform() -> &'static str {
        match std::env::consts::OS {
            "windows" => "windows",
            "linux" => "linux",
            "macos" => "macos",
            _ => "linux", // default to linux for unknown platforms
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_platform_detection() {
        let platform = CliDownloader::detect_platform();
        assert!(["windows", "linux", "macos"].contains(&platform));
    }

    #[test]
    fn test_download_url_generation() {
        let downloader = CliDownloader::new("https://example.com/cli".to_string());

        let url = downloader.get_download_url("1.0.0", "linux");
        assert_eq!(url, "https://example.com/cli/1.0.0/linux/studio-cli.gz");

        let url = downloader.get_download_url("1.0.0", "windows");
        assert_eq!(url, "https://example.com/cli/1.0.0/win64/studio-cli.exe.gz");
    }

    #[test]
    fn test_checksum_verification() {
        let downloader = CliDownloader::new("https://example.com/cli".to_string());
        let data = b"test data";

        // Calculate correct checksum
        let mut hasher = Sha1::new();
        hasher.update(data);
        let correct_checksum = hex::encode(hasher.finalize());

        // Should succeed with correct checksum
        assert!(downloader.verify_checksum(data, &correct_checksum).is_ok());

        // Should fail with incorrect checksum
        assert!(downloader.verify_checksum(data, "wrong_checksum").is_err());
    }

    #[test]
    fn test_gzip_decompression() {
        use flate2::write::GzEncoder;
        use flate2::Compression;

        let downloader = CliDownloader::new("https://example.com/cli".to_string());
        let original_data = b"test data for compression";

        // Compress data
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(original_data).unwrap();
        let compressed = encoder.finish().unwrap();

        // Decompress using our function
        let decompressed = downloader.decompress_gzip(&compressed).unwrap();

        assert_eq!(decompressed, original_data);
    }
}
