//! Authentication middleware for MCP server operations

use std::collections::HashMap;
use std::sync::Arc;
use studio_mcp_shared::{
    AuthCredentials, Result, StudioAuthService, StudioError, TokenValidator, ValidationResult,
};
use tokio::sync::RwLock;
use tracing::{debug, error};

/// Authentication context for MCP operations
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct AuthContext {
    /// Authenticated user credentials
    pub credentials: AuthCredentials,
    /// Token validation result
    pub validation: ValidationResult,
    /// User information
    pub user_info: Option<(String, Vec<String>)>,
    /// Available scopes
    pub scopes: Vec<String>,
}

/// Authentication middleware for MCP server
#[allow(dead_code)]
pub struct AuthMiddleware {
    /// Token validator
    validator: Arc<TokenValidator>,
    /// Authentication service
    auth_service: Arc<RwLock<StudioAuthService>>,
    /// Cached auth contexts by instance
    auth_cache: Arc<RwLock<HashMap<String, AuthContext>>>,
    /// Default instance configuration
    default_instance: Option<String>,
    /// Default environment
    default_environment: String,
}

#[allow(dead_code)]
impl AuthMiddleware {
    /// Create new authentication middleware
    pub fn new(default_environment: String) -> Result<Self> {
        let validator = Arc::new(TokenValidator::new());
        let auth_service = Arc::new(RwLock::new(StudioAuthService::new(300)?));
        let auth_cache = Arc::new(RwLock::new(HashMap::new()));

        Ok(Self {
            validator,
            auth_service,
            auth_cache,
            default_instance: None,
            default_environment,
        })
    }

    /// Set default Studio instance for operations
    pub fn set_default_instance(&mut self, instance_id: String) {
        self.default_instance = Some(instance_id);
    }

    /// Authenticate and get auth context for default instance
    pub async fn get_default_auth_context(&self) -> Result<AuthContext> {
        let instance_id = self
            .default_instance
            .as_ref()
            .ok_or_else(|| StudioError::Auth("No default instance configured".to_string()))?;

        self.get_auth_context(instance_id, &self.default_environment)
            .await
    }

    /// Get authentication context for specific instance
    pub async fn get_auth_context(
        &self,
        instance_id: &str,
        environment: &str,
    ) -> Result<AuthContext> {
        let cache_key = format!("{}:{}", environment, instance_id);

        // Check cache first
        {
            let cache = self.auth_cache.read().await;
            if let Some(context) = cache.get(&cache_key) {
                // Validate cached context is still fresh
                if context.validation.is_valid_and_fresh() {
                    debug!(
                        "Using cached auth context for {}:{}",
                        environment, instance_id
                    );
                    return Ok(context.clone());
                }
            }
        }

        // Load or refresh credentials
        let credentials = {
            let mut auth_service = self.auth_service.write().await;
            auth_service
                .get_credentials(instance_id, environment)
                .await?
        };

        // Validate token
        let token = credentials.get_valid_token()?;
        let validation = self.validator.validate_token(token).await?;

        if !validation.is_valid {
            error!(
                "Token validation failed for {}:{}: {:?}",
                environment, instance_id, validation.errors
            );
            return Err(StudioError::Auth(
                "Invalid authentication token".to_string(),
            ));
        }

        // Create auth context
        let user_info = validation.get_user_info();
        let scopes = validation.get_scopes();

        let context = AuthContext {
            credentials,
            validation,
            user_info,
            scopes,
        };

        // Cache the context
        {
            let mut cache = self.auth_cache.write().await;
            cache.insert(cache_key, context.clone());
        }

        debug!(
            "Created new auth context for {}:{}",
            environment, instance_id
        );
        Ok(context)
    }

    /// Validate permissions for specific operation
    pub fn validate_operation_permissions(
        &self,
        context: &AuthContext,
        required_scopes: &[String],
    ) -> Result<()> {
        if let Some(claims) = &context.validation.claims {
            if self.validator.validate_permissions(claims, required_scopes) {
                Ok(())
            } else {
                Err(StudioError::Auth(format!(
                    "Insufficient permissions. Required: {:?}, Available: {:?}",
                    required_scopes, context.scopes
                )))
            }
        } else {
            Err(StudioError::Auth(
                "No token claims available for permission validation".to_string(),
            ))
        }
    }

    /// Check if user has specific role
    pub fn has_role(&self, context: &AuthContext, role: &str) -> bool {
        context
            .user_info
            .as_ref()
            .map(|(_, roles)| roles.contains(&role.to_string()))
            .unwrap_or(false)
    }

    /// Force refresh authentication for instance
    pub async fn refresh_auth(&self, instance_id: &str, environment: &str) -> Result<AuthContext> {
        let cache_key = format!("{}:{}", environment, instance_id);

        // Remove from cache to force refresh
        {
            let mut cache = self.auth_cache.write().await;
            cache.remove(&cache_key);
        }

        // Get fresh context
        self.get_auth_context(instance_id, environment).await
    }

    /// Authenticate with new credentials
    pub async fn authenticate(
        &self,
        studio_url: &str,
        username: &str,
        password: &str,
        environment: &str,
    ) -> Result<AuthContext> {
        let credentials = {
            let mut auth_service = self.auth_service.write().await;
            auth_service
                .authenticate(studio_url, username, password, environment)
                .await?
        };

        // Create and cache auth context
        let instance_id = &credentials.instance_id;
        let environment = &credentials.environment;

        self.get_auth_context(instance_id, environment).await
    }

    /// Logout from instance
    pub async fn logout(&self, instance_id: &str, environment: &str) -> Result<()> {
        let cache_key = format!("{}:{}", environment, instance_id);

        // Remove from cache
        {
            let mut cache = self.auth_cache.write().await;
            cache.remove(&cache_key);
        }

        // Logout from auth service
        {
            let mut auth_service = self.auth_service.write().await;
            auth_service.logout(instance_id, environment).await?;
        }

        debug!("Logged out from {}:{}", environment, instance_id);
        Ok(())
    }

    /// Get current user information
    pub fn get_user_info(&self, context: &AuthContext) -> Option<UserInfo> {
        context
            .user_info
            .as_ref()
            .map(|(username, roles)| UserInfo {
                username: username.clone(),
                roles: roles.clone(),
                scopes: context.scopes.clone(),
                instance_id: context.credentials.instance_id.clone(),
                environment: context.credentials.environment.clone(),
                studio_url: context.credentials.studio_url.clone(),
            })
    }

    /// Clean up expired cache entries
    pub async fn cleanup_cache(&self) {
        {
            let mut cache = self.auth_cache.write().await;
            cache.retain(|_, context| context.validation.is_valid_and_fresh());
        }

        // Also cleanup validator cache
        self.validator.cleanup_cache().await;
    }

    /// Get authentication statistics
    pub async fn get_auth_stats(&self) -> AuthStats {
        let cache = self.auth_cache.read().await;
        let total_contexts = cache.len();
        let valid_contexts = cache
            .values()
            .filter(|context| context.validation.is_valid_and_fresh())
            .count();
        let expired_contexts = total_contexts - valid_contexts;

        AuthStats {
            total_contexts,
            valid_contexts,
            expired_contexts,
            instances: cache.keys().map(|key| key.clone()).collect(),
        }
    }
}

/// User information extracted from authentication context
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct UserInfo {
    pub username: String,
    pub roles: Vec<String>,
    pub scopes: Vec<String>,
    pub instance_id: String,
    pub environment: String,
    pub studio_url: String,
}

/// Authentication statistics
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct AuthStats {
    pub total_contexts: usize,
    pub valid_contexts: usize,
    pub expired_contexts: usize,
    pub instances: Vec<String>,
}

#[allow(dead_code)]
impl AuthContext {
    /// Check if context has specific scope
    pub fn has_scope(&self, scope: &str) -> bool {
        self.scopes.contains(&scope.to_string())
    }

    /// Check if context has any of the specified scopes
    pub fn has_any_scope(&self, scopes: &[String]) -> bool {
        scopes.iter().any(|scope| self.has_scope(scope))
    }

    /// Check if context has all specified scopes
    pub fn has_all_scopes(&self, scopes: &[String]) -> bool {
        scopes.iter().all(|scope| self.has_scope(scope))
    }

    /// Get time until token expires
    pub fn time_until_expiry(&self) -> Option<chrono::Duration> {
        self.validation.expires_in
    }

    /// Check if token needs refresh soon
    pub fn needs_refresh(&self) -> bool {
        self.validation.needs_refresh
    }
}

/// Common scopes for Studio operations
pub mod scopes {
    pub const READ: &str = "read";
    pub const WRITE: &str = "write";
    pub const ADMIN: &str = "admin";
    pub const PLM_READ: &str = "plm:read";
    pub const PLM_WRITE: &str = "plm:write";
    pub const PLM_ADMIN: &str = "plm:admin";
    pub const ARTIFACTS_READ: &str = "artifacts:read";
    pub const ARTIFACTS_WRITE: &str = "artifacts:write";
    pub const VLAB_READ: &str = "vlab:read";
    pub const VLAB_WRITE: &str = "vlab:write";
}

/// Common roles for Studio users
pub mod roles {
    pub const USER: &str = "user";
    pub const ADMIN: &str = "admin";
    pub const DEVELOPER: &str = "developer";
    pub const OPERATOR: &str = "operator";
    pub const READONLY: &str = "readonly";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_auth_middleware_creation() {
        let middleware = AuthMiddleware::new("dev".to_string());
        assert!(middleware.is_ok());
    }

    #[test]
    fn test_auth_context_scopes() {
        use studio_mcp_shared::AuthCredentials;

        let credentials = AuthCredentials::new(
            "test_instance".to_string(),
            "https://studio.example.com".to_string(),
            "testuser".to_string(),
            None,
            "dev".to_string(),
        );

        let validation = ValidationResult {
            is_valid: true,
            claims: None,
            errors: Vec::new(),
            expires_in: Some(chrono::Duration::hours(1)),
            needs_refresh: false,
        };

        let context = AuthContext {
            credentials,
            validation,
            user_info: Some(("testuser".to_string(), vec!["user".to_string()])),
            scopes: vec!["read".to_string(), "write".to_string()],
        };

        assert!(context.has_scope("read"));
        assert!(context.has_scope("write"));
        assert!(!context.has_scope("admin"));
        assert!(context.has_any_scope(&["read".to_string()]));
        assert!(context.has_all_scopes(&["read".to_string(), "write".to_string()]));
        assert!(!context.has_all_scopes(&["read".to_string(), "admin".to_string()]));
    }
}
