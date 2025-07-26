//! Authentication-aware CLI manager that integrates with Studio auth

use crate::CliManager;
use std::collections::HashMap;
use std::sync::Arc;
use studio_mcp_shared::{AuthCredentials, Result, StudioAuthService};
use tokio::sync::RwLock;

/// Authentication-aware CLI manager
pub struct AuthenticatedCliManager {
    /// Base CLI manager
    cli_manager: Arc<CliManager>,
    /// Authentication service
    auth_service: Arc<RwLock<StudioAuthService>>,
    /// Cached credentials by instance
    credentials_cache: Arc<RwLock<HashMap<String, AuthCredentials>>>,
}

impl AuthenticatedCliManager {
    /// Create a new authenticated CLI manager
    pub async fn new(
        download_base_url: String,
        install_dir: Option<std::path::PathBuf>,
    ) -> Result<Self> {
        let cli_manager = Arc::new(CliManager::new(download_base_url, install_dir)?);
        let auth_service = Arc::new(RwLock::new(StudioAuthService::new(300)?)); // 5 minute timeout
        let credentials_cache = Arc::new(RwLock::new(HashMap::new()));

        Ok(Self {
            cli_manager,
            auth_service,
            credentials_cache,
        })
    }

    /// Authenticate with a Studio instance
    pub async fn authenticate(
        &self,
        studio_url: &str,
        username: &str,
        password: &str,
        environment: &str,
    ) -> Result<AuthCredentials> {
        let mut auth_service = self.auth_service.write().await;
        let credentials = auth_service
            .authenticate(studio_url, username, password, environment)
            .await?;

        // Cache credentials
        let mut cache = self.credentials_cache.write().await;
        let cache_key = format!("{}:{}", environment, credentials.instance_id);
        cache.insert(cache_key, credentials.clone());

        Ok(credentials)
    }

    /// Get credentials for an instance
    pub async fn get_credentials(
        &self,
        instance_id: &str,
        environment: &str,
    ) -> Result<AuthCredentials> {
        let cache_key = format!("{environment}:{instance_id}");

        // Check cache first
        {
            let cache = self.credentials_cache.read().await;
            if let Some(credentials) = cache.get(&cache_key) {
                if !credentials.needs_refresh() {
                    return Ok(credentials.clone());
                }
            }
        }

        // Load from auth service (which will handle refresh if needed)
        let mut auth_service = self.auth_service.write().await;
        let credentials = auth_service
            .get_credentials(instance_id, environment)
            .await?;

        // Update cache
        let mut cache = self.credentials_cache.write().await;
        cache.insert(cache_key, credentials.clone());

        Ok(credentials)
    }

    /// Execute CLI command with authentication
    pub async fn execute_authenticated(
        &self,
        args: &[&str],
        instance_id: &str,
        environment: &str,
        working_dir: Option<&std::path::Path>,
    ) -> Result<serde_json::Value> {
        // Get valid credentials
        let credentials = self.get_credentials(instance_id, environment).await?;
        let token = credentials.get_valid_token()?;

        // Build authenticated CLI args
        let mut auth_args = vec![
            "--url",
            &credentials.studio_url,
            "--token",
            &token.access_token,
        ];
        auth_args.extend_from_slice(args);

        // Execute CLI command
        self.cli_manager.execute(&auth_args, working_dir).await
    }

    /// Execute CLI command with explicit credentials
    pub async fn execute_with_credentials(
        &self,
        args: &[&str],
        credentials: &AuthCredentials,
        working_dir: Option<&std::path::Path>,
    ) -> Result<serde_json::Value> {
        let token = credentials.get_valid_token()?;

        let mut auth_args = vec![
            "--url",
            &credentials.studio_url,
            "--token",
            &token.access_token,
        ];
        auth_args.extend_from_slice(args);

        self.cli_manager.execute(&auth_args, working_dir).await
    }

    /// Logout from a Studio instance
    pub async fn logout(&self, instance_id: &str, environment: &str) -> Result<()> {
        // Remove from cache
        let cache_key = format!("{environment}:{instance_id}");
        {
            let mut cache = self.credentials_cache.write().await;
            cache.remove(&cache_key);
        }

        // Remove from auth service
        let mut auth_service = self.auth_service.write().await;
        auth_service.logout(instance_id, environment).await?;

        Ok(())
    }

    /// List authenticated Studio instances
    pub async fn list_authenticated_instances(
        &self,
    ) -> Result<Vec<studio_mcp_shared::StudioInstance>> {
        let auth_service = self.auth_service.read().await;
        auth_service.list_instances().await
    }

    /// Verify Studio instance connectivity
    pub async fn verify_instance(&self, studio_url: &str) -> Result<()> {
        let auth_service = self.auth_service.read().await;
        auth_service.verify_studio_instance(studio_url).await
    }

    /// Get the underlying CLI manager for non-authenticated operations
    pub fn cli_manager(&self) -> &Arc<CliManager> {
        &self.cli_manager
    }

    /// Check if instance is authenticated
    pub async fn is_authenticated(&self, instance_id: &str, environment: &str) -> bool {
        self.get_credentials(instance_id, environment).await.is_ok()
    }

    /// Refresh credentials for an instance
    pub async fn refresh_credentials(
        &self,
        instance_id: &str,
        environment: &str,
    ) -> Result<AuthCredentials> {
        let mut auth_service = self.auth_service.write().await;

        // Get current credentials
        let credentials = auth_service
            .get_credentials(instance_id, environment)
            .await?;

        // Force refresh
        let refreshed = auth_service.refresh_credentials(credentials).await?;

        // Update cache
        let cache_key = format!("{environment}:{instance_id}");
        let mut cache = self.credentials_cache.write().await;
        cache.insert(cache_key, refreshed.clone());

        Ok(refreshed)
    }
}

/// CLI command builder for authenticated operations
pub struct AuthenticatedCommand {
    cli_manager: Arc<AuthenticatedCliManager>,
    instance_id: String,
    environment: String,
    args: Vec<String>,
    working_dir: Option<std::path::PathBuf>,
}

impl AuthenticatedCommand {
    /// Create a new authenticated command builder
    pub fn new(
        cli_manager: Arc<AuthenticatedCliManager>,
        instance_id: String,
        environment: String,
    ) -> Self {
        Self {
            cli_manager,
            instance_id,
            environment,
            args: Vec::new(),
            working_dir: None,
        }
    }

    /// Add arguments to the command
    pub fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        self.args
            .extend(args.into_iter().map(|s| s.as_ref().to_string()));
        self
    }

    /// Set working directory
    pub fn working_dir<P: Into<std::path::PathBuf>>(mut self, dir: P) -> Self {
        self.working_dir = Some(dir.into());
        self
    }

    /// Execute the command
    pub async fn execute(self) -> Result<serde_json::Value> {
        let args: Vec<&str> = self.args.iter().map(|s| s.as_str()).collect();

        self.cli_manager
            .execute_authenticated(
                &args,
                &self.instance_id,
                &self.environment,
                self.working_dir.as_deref(),
            )
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_authenticated_cli_manager_creation() {
        let temp_dir = TempDir::new().unwrap();
        let manager = AuthenticatedCliManager::new(
            "https://test.example.com".to_string(),
            Some(temp_dir.path().to_path_buf()),
        )
        .await;

        assert!(manager.is_ok());
    }

    #[tokio::test]
    async fn test_command_builder() {
        let temp_dir = TempDir::new().unwrap();
        let manager = Arc::new(
            AuthenticatedCliManager::new(
                "https://test.example.com".to_string(),
                Some(temp_dir.path().to_path_buf()),
            )
            .await
            .unwrap(),
        );

        let command =
            AuthenticatedCommand::new(manager, "test_instance".to_string(), "dev".to_string())
                .args(["plm", "pipeline", "list"])
                .working_dir(temp_dir.path());

        // This would fail since we don't have real credentials, but tests the builder
        assert_eq!(command.args, vec!["plm", "pipeline", "list"]);
        assert_eq!(command.instance_id, "test_instance");
        assert_eq!(command.environment, "dev");
    }
}
