//! Authentication service that integrates with WindRiver Studio CLI

use crate::{AuthCredentials, AuthManager, AuthToken, Result, StudioError, TokenStorage};
use jsonwebtoken::{decode_header, Algorithm, DecodingKey, Validation};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Studio authentication service
pub struct StudioAuthService {
    /// Authentication manager for token storage
    auth_manager: AuthManager,
    /// HTTP client for API requests
    client: Client,
    /// Default request timeout
    #[allow(dead_code)]
    timeout: Duration,
}

/// Studio API authentication request
#[derive(Debug, Serialize)]
struct AuthRequest {
    username: String,
    password: String,
    grant_type: String,
    client_id: String,
}

/// Studio API authentication response
#[derive(Debug, Deserialize)]
struct AuthResponse {
    access_token: String,
    refresh_token: Option<String>,
    #[allow(dead_code)]
    token_type: String,
    expires_in: i64,
    scope: Option<String>,
}

/// Studio API error response
#[derive(Debug, Deserialize)]
struct ApiErrorResponse {
    error: String,
    error_description: Option<String>,
}

/// JWT token claims for validation
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct TokenClaims {
    sub: String,
    exp: i64,
    iat: i64,
    iss: String,
    aud: String,
    scope: Option<String>,
    username: Option<String>,
    roles: Option<Vec<String>>,
}

/// Studio instance information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StudioInstance {
    pub instance_id: String,
    pub name: String,
    pub url: String,
    pub environment: String,
    pub version: Option<String>,
    pub status: InstanceStatus,
}

/// Studio instance status
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InstanceStatus {
    Online,
    Offline,
    Unknown,
}

impl StudioAuthService {
    /// Create a new authentication service
    pub fn new(timeout_seconds: u64) -> Result<Self> {
        let auth_manager = AuthManager::new()?;
        let client = Client::builder()
            .timeout(Duration::from_secs(timeout_seconds))
            .build()
            .map_err(StudioError::Network)?;

        Ok(Self {
            auth_manager,
            client,
            timeout: Duration::from_secs(timeout_seconds),
        })
    }

    /// Authenticate with a Studio instance using username/password
    pub async fn authenticate(
        &mut self,
        studio_url: &str,
        username: &str,
        password: &str,
        environment: &str,
    ) -> Result<AuthCredentials> {
        // Validate inputs
        if studio_url.is_empty() || username.is_empty() || password.is_empty() {
            return Err(StudioError::Auth(
                "Invalid authentication parameters".to_string(),
            ));
        }

        // Normalize studio URL
        let normalized_url = self.normalize_studio_url(studio_url)?;

        // Check if instance is reachable
        self.verify_studio_instance(&normalized_url).await?;

        // Attempt authentication
        let auth_response = self
            .perform_authentication(&normalized_url, username, password)
            .await?;

        // Parse and validate token
        let token = self.create_auth_token(auth_response, &normalized_url)?;

        // Create credentials and store securely
        let mut credentials = AuthCredentials::new(
            self.generate_instance_id(&normalized_url, environment),
            normalized_url,
            username.to_string(),
            None,
            environment.to_string(),
        );

        // Extract additional info from token if possible
        if let Ok(claims) = self.decode_token_claims(&token.access_token) {
            credentials.display_name = claims.username.clone();
            credentials.roles = claims.roles.unwrap_or_default();
        }

        credentials.set_token(token);

        // Store credentials
        self.auth_manager.store_credentials(&credentials).await?;

        Ok(credentials)
    }

    /// Get cached credentials or load from storage
    pub async fn get_credentials(
        &mut self,
        instance_id: &str,
        environment: &str,
    ) -> Result<AuthCredentials> {
        let mut credentials = self
            .auth_manager
            .get_credentials(instance_id, environment)?;

        // Check if token needs refresh
        if credentials.needs_refresh() {
            credentials = self.refresh_credentials(credentials).await?;
        }

        Ok(credentials)
    }

    /// Refresh expired credentials
    pub async fn refresh_credentials(
        &mut self,
        mut credentials: AuthCredentials,
    ) -> Result<AuthCredentials> {
        if let Some(token) = &credentials.token {
            if let Some(refresh_token) = &token.refresh_token {
                // Attempt token refresh
                match self
                    .refresh_token_with_api(&credentials.studio_url, refresh_token)
                    .await
                {
                    Ok(new_token) => {
                        credentials.set_token(new_token);
                        self.auth_manager.store_credentials(&credentials).await?;
                        return Ok(credentials);
                    }
                    Err(e) => {
                        // If refresh fails, credentials are invalid
                        self.logout(&credentials.instance_id, &credentials.environment)
                            .await?;
                        return Err(StudioError::Auth(format!("Token refresh failed: {e}")));
                    }
                }
            }
        }

        Err(StudioError::Auth(
            "Cannot refresh credentials - no refresh token available".to_string(),
        ))
    }

    /// Logout and remove stored credentials
    pub async fn logout(&mut self, instance_id: &str, environment: &str) -> Result<()> {
        // Get credentials to notify server
        if let Ok(credentials) = self.auth_manager.get_credentials(instance_id, environment) {
            if let Ok(token) = credentials.get_valid_token() {
                // Attempt to revoke token on server (best effort)
                let _ = self
                    .revoke_token(&credentials.studio_url, &token.access_token)
                    .await;
            }
        }

        // Remove from local storage
        self.auth_manager.logout(instance_id, environment)?;

        Ok(())
    }

    /// List available Studio instances
    pub async fn list_instances(&self) -> Result<Vec<StudioInstance>> {
        // This would typically query a registry or configuration
        // For now, return instances from stored credentials
        // Implementation would depend on how Studio instances are discovered
        Ok(Vec::new())
    }

    /// Verify that a Studio instance is reachable
    pub async fn verify_studio_instance(&self, studio_url: &str) -> Result<()> {
        let health_url = format!("{studio_url}/api/health");

        let response = self
            .client
            .get(&health_url)
            .timeout(Duration::from_secs(10))
            .send()
            .await
            .map_err(StudioError::Network)?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(StudioError::Auth(format!(
                "Studio instance not reachable: HTTP {}",
                response.status()
            )))
        }
    }

    /// Perform authentication with Studio API
    async fn perform_authentication(
        &self,
        studio_url: &str,
        username: &str,
        password: &str,
    ) -> Result<AuthResponse> {
        let auth_url = format!("{studio_url}/api/auth/token");

        let request = AuthRequest {
            username: username.to_string(),
            password: password.to_string(),
            grant_type: "password".to_string(),
            client_id: "studio-mcp-client".to_string(),
        };

        let response = self
            .client
            .post(&auth_url)
            .json(&request)
            .send()
            .await
            .map_err(StudioError::Network)?;

        if response.status().is_success() {
            let auth_response: AuthResponse =
                response.json().await.map_err(StudioError::Network)?;

            Ok(auth_response)
        } else {
            // Save status for error message
            let status = response.status();

            // Try to parse error response
            let error_text = if let Ok(error_response) = response.json::<ApiErrorResponse>().await {
                error_response
                    .error_description
                    .unwrap_or(error_response.error)
            } else {
                format!("Authentication failed with status: {status}")
            };

            Err(StudioError::Auth(error_text))
        }
    }

    /// Refresh token using Studio API
    async fn refresh_token_with_api(
        &self,
        studio_url: &str,
        refresh_token: &str,
    ) -> Result<AuthToken> {
        let refresh_url = format!("{studio_url}/api/auth/refresh");

        let mut refresh_request = std::collections::HashMap::new();
        refresh_request.insert("grant_type", "refresh_token");
        refresh_request.insert("refresh_token", refresh_token);

        let response = self
            .client
            .post(&refresh_url)
            .json(&refresh_request)
            .send()
            .await
            .map_err(StudioError::Network)?;

        if response.status().is_success() {
            let auth_response: AuthResponse =
                response.json().await.map_err(StudioError::Network)?;

            Ok(self.create_auth_token(auth_response, studio_url)?)
        } else {
            Err(StudioError::Auth("Token refresh failed".to_string()))
        }
    }

    /// Revoke token on server
    async fn revoke_token(&self, studio_url: &str, access_token: &str) -> Result<()> {
        let revoke_url = format!("{studio_url}/api/auth/revoke");

        let mut revoke_request = std::collections::HashMap::new();
        revoke_request.insert("token", access_token);

        let _response = self
            .client
            .post(&revoke_url)
            .bearer_auth(access_token)
            .json(&revoke_request)
            .send()
            .await
            .map_err(StudioError::Network)?;

        // Don't fail if revocation fails - just log it
        Ok(())
    }

    /// Create AuthToken from API response
    fn create_auth_token(&self, response: AuthResponse, studio_url: &str) -> Result<AuthToken> {
        let scopes = response
            .scope
            .map(|s| s.split_whitespace().map(|s| s.to_string()).collect())
            .unwrap_or_default();

        Ok(AuthToken::new(
            response.access_token,
            response.refresh_token,
            response.expires_in,
            studio_url.to_string(),
            scopes,
        ))
    }

    /// Decode JWT token claims for validation
    fn decode_token_claims(&self, token: &str) -> Result<TokenClaims> {
        // For now, just decode without validation since we don't have the public key
        // In production, you'd validate with the proper key from Studio
        let _header = decode_header(token)
            .map_err(|e| StudioError::Auth(format!("Invalid token header: {e}")))?;

        // Use a dummy key for now - in production, fetch from Studio's JWKS endpoint
        let _key = DecodingKey::from_secret(b"dummy-key");
        let mut validation = Validation::new(Algorithm::HS256);
        validation.validate_exp = false; // We handle expiry separately
        validation.validate_aud = false;
        validation.validate_nbf = false;

        // This will fail with dummy key, so just return basic claims
        // In production implementation, proper JWT validation would be done
        Err(StudioError::Auth(
            "JWT validation not implemented with dummy key".to_string(),
        ))
    }

    /// Normalize Studio URL for consistent storage
    fn normalize_studio_url(&self, url: &str) -> Result<String> {
        let mut normalized = url.trim_end_matches('/').to_string();

        if !normalized.starts_with("http://") && !normalized.starts_with("https://") {
            normalized = format!("https://{normalized}");
        }

        // Validate URL format
        url::Url::parse(&normalized).map_err(StudioError::UrlParse)?;

        Ok(normalized)
    }

    /// Generate instance ID from URL and environment
    fn generate_instance_id(&self, studio_url: &str, environment: &str) -> String {
        use sha1::{Digest, Sha1};

        let mut hasher = Sha1::new();
        hasher.update(studio_url.as_bytes());
        hasher.update(environment.as_bytes());
        let result = hasher.finalize();

        hex::encode(&result[..8])
    }
}

impl AuthManager {
    /// Store credentials (async wrapper)
    pub async fn store_credentials(&mut self, credentials: &AuthCredentials) -> Result<()> {
        // Clone what we need for the blocking task
        let creds = credentials.clone();
        let storage_clone = TokenStorage::new("studio-mcp".to_string())?;

        tokio::task::spawn_blocking(move || storage_clone.store_credentials(&creds))
            .await
            .map_err(|e| StudioError::Unknown(format!("Task join error: {e}")))??;

        // Update cache
        let cache_key = format!("{}:{}", credentials.environment, credentials.instance_id);
        self.credentials_cache
            .insert(cache_key, credentials.clone());

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_studio_url_normalization() {
        let service = StudioAuthService::new(30).unwrap();

        assert_eq!(
            service.normalize_studio_url("studio.example.com").unwrap(),
            "https://studio.example.com"
        );

        assert_eq!(
            service
                .normalize_studio_url("https://studio.example.com/")
                .unwrap(),
            "https://studio.example.com"
        );
    }

    #[test]
    fn test_instance_id_generation() {
        let service = StudioAuthService::new(30).unwrap();

        let id1 = service.generate_instance_id("https://studio.example.com", "dev");
        let id2 = service.generate_instance_id("https://studio.example.com", "dev");
        let id3 = service.generate_instance_id("https://studio.example.com", "prod");

        assert_eq!(id1, id2); // Same inputs should produce same ID
        assert_ne!(id1, id3); // Different environment should produce different ID
    }
}
