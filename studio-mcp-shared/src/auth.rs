//! Authentication and token management for WindRiver Studio

use crate::{Result, StudioError};
use aes_gcm::{AeadInPlace, Aes256Gcm, KeyInit, Nonce};
use base64::{engine::general_purpose, Engine as _};
use chrono::{DateTime, Duration, Utc};
use keyring::Entry;
use rand::{rngs::OsRng, RngCore};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Authentication token information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthToken {
    /// JWT access token
    pub access_token: String,
    /// JWT refresh token
    pub refresh_token: Option<String>,
    /// Token type (usually "Bearer")
    pub token_type: String,
    /// Token expiration time
    pub expires_at: DateTime<Utc>,
    /// Token scopes/permissions
    pub scopes: Vec<String>,
    /// Studio instance URL this token belongs to
    pub studio_url: String,
}

/// Authentication credentials for a Studio instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthCredentials {
    /// Studio instance identifier
    pub instance_id: String,
    /// Studio instance URL
    pub studio_url: String,
    /// Username/email
    pub username: String,
    /// User's full name
    pub display_name: Option<String>,
    /// User roles/permissions
    pub roles: Vec<String>,
    /// Environment (dev, staging, prod)
    pub environment: String,
    /// Last authentication time
    pub last_auth: DateTime<Utc>,
    /// Encrypted authentication token
    #[serde(skip)]
    pub token: Option<AuthToken>,
}

/// Token storage manager using OS keyring for secure storage
pub struct TokenStorage {
    /// Service name for keyring entries
    service_name: String,
    /// Encryption key for additional security
    encryption_key: [u8; 32],
}

/// Authentication manager for Studio instances
pub struct AuthManager {
    /// Token storage backend
    pub(crate) storage: TokenStorage,
    /// In-memory cache of credentials
    pub(crate) credentials_cache: HashMap<String, AuthCredentials>,
}

impl AuthToken {
    /// Create a new auth token
    pub fn new(
        access_token: String,
        refresh_token: Option<String>,
        expires_in: i64,
        studio_url: String,
        scopes: Vec<String>,
    ) -> Self {
        let expires_at = Utc::now() + Duration::seconds(expires_in);

        Self {
            access_token,
            refresh_token,
            token_type: "Bearer".to_string(),
            expires_at,
            scopes,
            studio_url,
        }
    }

    /// Check if the token is expired
    pub fn is_expired(&self) -> bool {
        Utc::now() >= self.expires_at
    }

    /// Check if the token will expire within the given duration
    pub fn expires_within(&self, duration: Duration) -> bool {
        Utc::now() + duration >= self.expires_at
    }

    /// Get the authorization header value
    pub fn authorization_header(&self) -> String {
        format!("{} {}", self.token_type, self.access_token)
    }

    /// Validate token format and basic structure
    pub fn validate(&self) -> Result<()> {
        if self.access_token.is_empty() {
            return Err(StudioError::Auth("Access token is empty".to_string()));
        }

        if self.studio_url.is_empty() {
            return Err(StudioError::Auth("Studio URL is empty".to_string()));
        }

        if self.is_expired() {
            return Err(StudioError::Auth("Token has expired".to_string()));
        }

        Ok(())
    }
}

impl AuthCredentials {
    /// Create new authentication credentials
    pub fn new(
        instance_id: String,
        studio_url: String,
        username: String,
        display_name: Option<String>,
        environment: String,
    ) -> Self {
        Self {
            instance_id,
            studio_url,
            username,
            display_name,
            roles: Vec::new(),
            environment,
            last_auth: Utc::now(),
            token: None,
        }
    }

    /// Set the authentication token
    pub fn set_token(&mut self, token: AuthToken) {
        self.last_auth = Utc::now();
        self.token = Some(token);
    }

    /// Get the current token if valid
    pub fn get_valid_token(&self) -> Result<&AuthToken> {
        match &self.token {
            Some(token) => {
                token.validate()?;
                Ok(token)
            }
            None => Err(StudioError::Auth("No token available".to_string())),
        }
    }

    /// Check if credentials need refresh
    pub fn needs_refresh(&self) -> bool {
        match &self.token {
            Some(token) => token.expires_within(Duration::minutes(5)),
            None => true,
        }
    }

    /// Generate a unique key for storage
    pub fn storage_key(&self) -> String {
        format!("studio-mcp:{}:{}", self.environment, self.instance_id)
    }
}

impl TokenStorage {
    /// Create a new token storage manager
    pub fn new(service_name: String) -> Result<Self> {
        // Generate or load encryption key
        let encryption_key = Self::get_or_create_encryption_key(&service_name)?;

        Ok(Self {
            service_name,
            encryption_key,
        })
    }

    /// Store encrypted credentials in the OS keyring
    pub fn store_credentials(&self, credentials: &AuthCredentials) -> Result<()> {
        let key = credentials.storage_key();
        let entry = Entry::new(&self.service_name, &key)
            .map_err(|e| StudioError::Auth(format!("Failed to create keyring entry: {e}")))?;

        // Serialize and encrypt credentials
        let serialized = serde_json::to_vec(credentials).map_err(StudioError::Json)?;

        let encrypted = self.encrypt_data(&serialized)?;
        let encoded = general_purpose::STANDARD.encode(&encrypted);

        entry
            .set_password(&encoded)
            .map_err(|e| StudioError::Auth(format!("Failed to store credentials: {e}")))?;

        Ok(())
    }

    /// Retrieve and decrypt credentials from the OS keyring
    pub fn load_credentials(
        &self,
        instance_id: &str,
        environment: &str,
    ) -> Result<AuthCredentials> {
        let key = format!("studio-mcp:{environment}:{instance_id}");
        let entry = Entry::new(&self.service_name, &key)
            .map_err(|e| StudioError::Auth(format!("Failed to create keyring entry: {e}")))?;

        let encoded = entry
            .get_password()
            .map_err(|e| StudioError::Auth(format!("Failed to retrieve credentials: {e}")))?;

        let encrypted = general_purpose::STANDARD
            .decode(encoded)
            .map_err(|e| StudioError::Auth(format!("Failed to decode credentials: {e}")))?;

        let decrypted = self.decrypt_data(&encrypted)?;

        let credentials: AuthCredentials =
            serde_json::from_slice(&decrypted).map_err(StudioError::Json)?;

        Ok(credentials)
    }

    /// Remove credentials from storage
    pub fn remove_credentials(&self, instance_id: &str, environment: &str) -> Result<()> {
        let key = format!("studio-mcp:{environment}:{instance_id}");
        let entry = Entry::new(&self.service_name, &key)
            .map_err(|e| StudioError::Auth(format!("Failed to create keyring entry: {e}")))?;

        entry
            .delete_credential()
            .map_err(|e| StudioError::Auth(format!("Failed to remove credentials: {e}")))?;

        Ok(())
    }

    /// List all stored credentials
    pub fn list_stored_instances(&self) -> Result<Vec<(String, String)>> {
        // Note: This is a limitation of most keyring APIs - we can't list entries
        // So we'll need to maintain a registry of instances separately
        // For now, return empty list and rely on configuration file
        Ok(Vec::new())
    }

    /// Encrypt data using AES-256-GCM
    fn encrypt_data(&self, data: &[u8]) -> Result<Vec<u8>> {
        let cipher = Aes256Gcm::new(&self.encryption_key.into());

        // Generate random nonce
        let mut nonce_bytes = [0u8; 12];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let mut buffer = data.to_vec();
        cipher
            .encrypt_in_place(nonce, b"", &mut buffer)
            .map_err(|e| StudioError::Auth(format!("Encryption failed: {e}")))?;

        // Prepend nonce to encrypted data
        let mut result = nonce_bytes.to_vec();
        result.extend_from_slice(&buffer);
        Ok(result)
    }

    /// Decrypt data using AES-256-GCM
    fn decrypt_data(&self, encrypted_data: &[u8]) -> Result<Vec<u8>> {
        if encrypted_data.len() < 12 {
            return Err(StudioError::Auth("Invalid encrypted data".to_string()));
        }

        let (nonce_bytes, ciphertext) = encrypted_data.split_at(12);
        let nonce = Nonce::from_slice(nonce_bytes);

        let cipher = Aes256Gcm::new(&self.encryption_key.into());

        let mut buffer = ciphertext.to_vec();
        cipher
            .decrypt_in_place(nonce, b"", &mut buffer)
            .map_err(|e| StudioError::Auth(format!("Decryption failed: {e}")))?;

        Ok(buffer)
    }

    /// Get or create encryption key
    fn get_or_create_encryption_key(service_name: &str) -> Result<[u8; 32]> {
        let key_entry = Entry::new(service_name, "encryption-key")
            .map_err(|e| StudioError::Auth(format!("Failed to create key entry: {e}")))?;

        match key_entry.get_password() {
            Ok(encoded_key) => {
                let key_bytes = general_purpose::STANDARD.decode(encoded_key).map_err(|e| {
                    StudioError::Auth(format!("Failed to decode encryption key: {e}"))
                })?;

                if key_bytes.len() != 32 {
                    return Err(StudioError::Auth(
                        "Invalid encryption key length".to_string(),
                    ));
                }

                let mut key = [0u8; 32];
                key.copy_from_slice(&key_bytes);
                Ok(key)
            }
            Err(_) => {
                // Generate new key
                let mut key = [0u8; 32];
                OsRng.fill_bytes(&mut key);

                let encoded_key = general_purpose::STANDARD.encode(key);
                key_entry.set_password(&encoded_key).map_err(|e| {
                    StudioError::Auth(format!("Failed to store encryption key: {e}"))
                })?;

                Ok(key)
            }
        }
    }
}

impl AuthManager {
    /// Create a new authentication manager
    pub fn new() -> Result<Self> {
        let storage = TokenStorage::new("studio-mcp".to_string())?;

        Ok(Self {
            storage,
            credentials_cache: HashMap::new(),
        })
    }

    /// Authenticate with a Studio instance using username/password
    pub async fn authenticate(
        &mut self,
        studio_url: &str,
        username: &str,
        _password: &str,
        environment: &str,
    ) -> Result<AuthCredentials> {
        // This would typically make an HTTP request to the Studio auth endpoint
        // For now, we'll create a mock implementation

        let instance_id = self.generate_instance_id(studio_url, environment);
        let mut credentials = AuthCredentials::new(
            instance_id.clone(),
            studio_url.to_string(),
            username.to_string(),
            None,
            environment.to_string(),
        );

        // Mock token creation (in real implementation, this would come from Studio API)
        let token = AuthToken::new(
            "mock_access_token".to_string(),
            Some("mock_refresh_token".to_string()),
            3600, // 1 hour
            studio_url.to_string(),
            vec!["read".to_string(), "write".to_string()],
        );

        credentials.set_token(token);

        // Store credentials securely
        self.storage.store_credentials(&credentials)?;

        // Cache credentials
        self.credentials_cache
            .insert(instance_id, credentials.clone());

        Ok(credentials)
    }

    /// Get cached or stored credentials for an instance
    pub fn get_credentials(
        &mut self,
        instance_id: &str,
        environment: &str,
    ) -> Result<AuthCredentials> {
        // Check cache first
        let cache_key = format!("{environment}:{instance_id}");
        if let Some(credentials) = self.credentials_cache.get(&cache_key) {
            return Ok(credentials.clone());
        }

        // Load from storage
        let credentials = self.storage.load_credentials(instance_id, environment)?;
        self.credentials_cache
            .insert(cache_key, credentials.clone());

        Ok(credentials)
    }

    /// Refresh an expired token
    pub async fn refresh_token(
        &mut self,
        instance_id: &str,
        environment: &str,
    ) -> Result<AuthToken> {
        let mut credentials = self.get_credentials(instance_id, environment)?;

        if let Some(token) = &credentials.token {
            if let Some(refresh_token) = &token.refresh_token {
                // Make refresh request to Studio API
                // For now, create a new mock token
                let new_token = AuthToken::new(
                    "refreshed_access_token".to_string(),
                    Some(refresh_token.clone()),
                    3600,
                    token.studio_url.clone(),
                    token.scopes.clone(),
                );

                // Update stored credentials
                credentials.set_token(new_token.clone());
                self.storage.store_credentials(&credentials)?;

                // Update cache
                let cache_key = format!("{environment}:{instance_id}");
                self.credentials_cache.insert(cache_key, credentials);

                return Ok(new_token);
            }
        }

        Err(StudioError::Auth("No refresh token available".to_string()))
    }

    /// Logout and remove stored credentials
    pub fn logout(&mut self, instance_id: &str, environment: &str) -> Result<()> {
        // Remove from cache
        let cache_key = format!("{environment}:{instance_id}");
        self.credentials_cache.remove(&cache_key);

        // Remove from storage
        self.storage.remove_credentials(instance_id, environment)?;

        Ok(())
    }

    /// Generate a unique instance ID
    fn generate_instance_id(&self, studio_url: &str, environment: &str) -> String {
        use sha1::{Digest, Sha1};

        let mut hasher = Sha1::new();
        hasher.update(studio_url.as_bytes());
        hasher.update(environment.as_bytes());
        let result = hasher.finalize();

        hex::encode(&result[..8]) // Use first 8 bytes for shorter ID
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_auth_token_validation() {
        let token = AuthToken::new(
            "test_token".to_string(),
            None,
            3600,
            "https://studio.example.com".to_string(),
            vec!["read".to_string()],
        );

        assert!(!token.is_expired());
        assert!(token.validate().is_ok());
        assert_eq!(token.authorization_header(), "Bearer test_token");
    }

    #[test]
    fn test_credentials_storage_key() {
        let creds = AuthCredentials::new(
            "test_instance".to_string(),
            "https://studio.example.com".to_string(),
            "user@example.com".to_string(),
            None,
            "dev".to_string(),
        );

        assert_eq!(creds.storage_key(), "studio-mcp:dev:test_instance");
    }
}
