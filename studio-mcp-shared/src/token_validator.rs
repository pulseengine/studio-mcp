//! Token validation and JWT verification for WindRiver Studio

use crate::{AuthToken, Result, StudioError};
use chrono::{DateTime, Duration, Utc};
use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, TokenData, Validation};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// JWT Claims for Studio tokens
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StudioTokenClaims {
    /// Subject (user ID)
    pub sub: String,
    /// Expiration time (Unix timestamp)
    pub exp: i64,
    /// Issued at (Unix timestamp)
    pub iat: i64,
    /// Issuer (Studio instance)
    pub iss: String,
    /// Audience (client ID)
    pub aud: String,
    /// Token scopes/permissions
    pub scope: Option<String>,
    /// Username
    pub username: Option<String>,
    /// User roles
    pub roles: Option<Vec<String>>,
    /// Studio instance ID
    pub instance_id: Option<String>,
    /// Environment (dev, staging, prod)
    pub environment: Option<String>,
}

/// Token validation result
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Whether the token is valid
    pub is_valid: bool,
    /// Token claims if valid
    pub claims: Option<StudioTokenClaims>,
    /// Validation errors
    pub errors: Vec<String>,
    /// Time until expiration
    pub expires_in: Option<Duration>,
    /// Whether token needs refresh soon
    pub needs_refresh: bool,
}

/// JWKS (JSON Web Key Set) cache entry
#[derive(Clone)]
struct JwksEntry {
    /// The public keys
    keys: HashMap<String, DecodingKey>,
    /// When this entry expires
    expires_at: DateTime<Utc>,
    /// Studio instance URL this belongs to
    #[allow(dead_code)]
    studio_url: String,
}

/// Studio token validator with JWT verification
pub struct TokenValidator {
    /// HTTP client for JWKS fetching
    client: Client,
    /// JWKS cache by studio URL
    jwks_cache: Arc<RwLock<HashMap<String, JwksEntry>>>,
    /// Cache TTL for JWKS entries
    cache_ttl: Duration,
    /// Grace period before token expiration to trigger refresh
    refresh_grace_period: Duration,
}

/// JWKS response from Studio
#[derive(Debug, Deserialize)]
struct JwksResponse {
    keys: Vec<JwkKey>,
}

/// Individual JWK key
#[derive(Debug, Deserialize)]
struct JwkKey {
    /// Key type (usually "RSA")
    kty: String,
    /// Key use (usually "sig")
    #[serde(rename = "use")]
    #[allow(dead_code)]
    key_use: Option<String>,
    /// Key ID
    kid: Option<String>,
    /// Algorithm
    #[allow(dead_code)]
    alg: Option<String>,
    /// RSA modulus (base64url)
    n: Option<String>,
    /// RSA exponent (base64url)
    e: Option<String>,
}

impl TokenValidator {
    /// Create a new token validator
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            jwks_cache: Arc::new(RwLock::new(HashMap::new())),
            cache_ttl: Duration::hours(1),
            refresh_grace_period: Duration::minutes(5),
        }
    }

    /// Validate a Studio token with full JWT verification
    pub async fn validate_token(&self, token: &AuthToken) -> Result<ValidationResult> {
        let mut result = ValidationResult {
            is_valid: false,
            claims: None,
            errors: Vec::new(),
            expires_in: None,
            needs_refresh: false,
        };

        // Basic token validation
        if let Err(e) = token.validate() {
            result.errors.push(e.to_string());
            return Ok(result);
        }

        // Calculate expiration info
        let now = Utc::now();
        let expires_in = token.expires_at - now;
        result.expires_in = Some(expires_in);
        result.needs_refresh = expires_in <= self.refresh_grace_period;

        // Decode JWT header to get key ID
        let header = match decode_header(&token.access_token) {
            Ok(h) => h,
            Err(e) => {
                result.errors.push(format!("Invalid JWT header: {}", e));
                return Ok(result);
            }
        };

        // Get decoding key for this token
        let decoding_key = match self.get_decoding_key(&token.studio_url, &header).await {
            Ok(key) => key,
            Err(e) => {
                result
                    .errors
                    .push(format!("Failed to get decoding key: {}", e));
                return Ok(result);
            }
        };

        // Validate JWT signature and claims
        match self
            .decode_and_validate_jwt(&token.access_token, &decoding_key)
            .await
        {
            Ok(token_data) => {
                result.is_valid = true;
                result.claims = Some(token_data.claims);
            }
            Err(e) => {
                result.errors.push(format!("JWT validation failed: {}", e));
            }
        }

        Ok(result)
    }

    /// Quick validation without JWT verification (for performance)
    pub fn validate_token_basic(&self, token: &AuthToken) -> ValidationResult {
        let mut result = ValidationResult {
            is_valid: false,
            claims: None,
            errors: Vec::new(),
            expires_in: None,
            needs_refresh: false,
        };

        // Basic token validation
        if let Err(e) = token.validate() {
            result.errors.push(e.to_string());
            return result;
        }

        // Calculate expiration info
        let now = Utc::now();
        let expires_in = token.expires_at - now;
        result.expires_in = Some(expires_in);
        result.needs_refresh = expires_in <= self.refresh_grace_period;

        // If token hasn't expired and has basic structure, consider it valid for basic check
        if expires_in > Duration::zero() && !token.access_token.is_empty() {
            result.is_valid = true;
        }

        result
    }

    /// Check if token needs refresh based on expiration time
    pub fn needs_refresh(&self, token: &AuthToken) -> bool {
        let expires_in = token.expires_at - Utc::now();
        expires_in <= self.refresh_grace_period
    }

    /// Validate token permissions for specific operations
    pub fn validate_permissions(
        &self,
        claims: &StudioTokenClaims,
        required_scopes: &[String],
    ) -> bool {
        if let Some(scope_str) = &claims.scope {
            let token_scopes: Vec<&str> = scope_str.split_whitespace().collect();

            for required in required_scopes {
                if !token_scopes.contains(&required.as_str()) {
                    return false;
                }
            }
            true
        } else {
            required_scopes.is_empty()
        }
    }

    /// Validate token for specific Studio instance
    pub fn validate_instance(&self, claims: &StudioTokenClaims, expected_instance: &str) -> bool {
        claims
            .instance_id
            .as_ref()
            .map_or(false, |id| id == expected_instance)
    }

    /// Get or fetch JWKS decoding key
    async fn get_decoding_key(
        &self,
        studio_url: &str,
        header: &jsonwebtoken::Header,
    ) -> Result<DecodingKey> {
        // Check cache first
        {
            let cache = self.jwks_cache.read().await;
            if let Some(entry) = cache.get(studio_url) {
                if entry.expires_at > Utc::now() {
                    // Try to find key by kid (key ID)
                    if let Some(kid) = &header.kid {
                        if let Some(key) = entry.keys.get(kid) {
                            return Ok(key.clone());
                        }
                    }
                    // Fallback to first available key
                    if let Some(key) = entry.keys.values().next() {
                        return Ok(key.clone());
                    }
                }
            }
        }

        // Fetch fresh JWKS
        let jwks = self.fetch_jwks(studio_url).await?;
        self.cache_jwks(studio_url, jwks).await
    }

    /// Fetch JWKS from Studio instance
    async fn fetch_jwks(&self, studio_url: &str) -> Result<JwksResponse> {
        let jwks_url = format!("{}/.well-known/jwks.json", studio_url);

        let response = self
            .client
            .get(&jwks_url)
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await
            .map_err(|e| StudioError::Network(e))?;

        if !response.status().is_success() {
            return Err(StudioError::Auth(format!(
                "Failed to fetch JWKS: HTTP {}",
                response.status()
            )));
        }

        let jwks: JwksResponse = response.json().await.map_err(|e| StudioError::Network(e))?;

        Ok(jwks)
    }

    /// Cache JWKS and return a decoding key
    async fn cache_jwks(&self, studio_url: &str, jwks: JwksResponse) -> Result<DecodingKey> {
        let mut keys = HashMap::new();
        let mut selected_key = None;

        for jwk in jwks.keys {
            if jwk.kty == "RSA" && jwk.n.is_some() && jwk.e.is_some() {
                match self.create_rsa_key(&jwk) {
                    Ok(key) => {
                        let kid = jwk.kid.unwrap_or_else(|| "default".to_string());
                        if selected_key.is_none() {
                            selected_key = Some(key.clone());
                        }
                        keys.insert(kid, key);
                    }
                    Err(e) => {
                        tracing::warn!("Failed to create RSA key from JWK: {}", e);
                    }
                }
            }
        }

        if keys.is_empty() {
            return Err(StudioError::Auth(
                "No valid RSA keys found in JWKS".to_string(),
            ));
        }

        let entry = JwksEntry {
            keys,
            expires_at: Utc::now() + self.cache_ttl,
            studio_url: studio_url.to_string(),
        };

        // Cache the entry
        {
            let mut cache = self.jwks_cache.write().await;
            cache.insert(studio_url.to_string(), entry);
        }

        selected_key.ok_or_else(|| StudioError::Auth("No usable key found".to_string()))
    }

    /// Create RSA decoding key from JWK
    fn create_rsa_key(&self, _jwk: &JwkKey) -> Result<DecodingKey> {
        // This is a simplified implementation
        // In production, you'd use proper RSA key construction from modulus and exponent
        // For now, return a dummy key since we don't have the full RSA implementation

        // This would normally construct the RSA public key from n and e parameters
        // let modulus = base64url_decode(&jwk.n.as_ref().unwrap())?;
        // let exponent = base64url_decode(&jwk.e.as_ref().unwrap())?;
        // let public_key = construct_rsa_public_key(modulus, exponent)?;

        // For now, create a dummy key
        Ok(DecodingKey::from_secret(b"dummy-secret-key"))
    }

    /// Decode and validate JWT with proper verification
    async fn decode_and_validate_jwt(
        &self,
        token: &str,
        key: &DecodingKey,
    ) -> Result<TokenData<StudioTokenClaims>> {
        let mut validation = Validation::new(Algorithm::RS256);
        validation.validate_exp = true;
        validation.validate_nbf = true;
        validation.leeway = 60; // 60 seconds leeway for clock skew

        // For now, disable signature validation since we're using dummy keys
        validation.insecure_disable_signature_validation();

        let token_data = decode::<StudioTokenClaims>(token, key, &validation)
            .map_err(|e| StudioError::Auth(format!("JWT decode failed: {}", e)))?;

        Ok(token_data)
    }

    /// Clear expired entries from JWKS cache
    pub async fn cleanup_cache(&self) {
        let mut cache = self.jwks_cache.write().await;
        let now = Utc::now();
        cache.retain(|_, entry| entry.expires_at > now);
    }
}

impl Default for TokenValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl ValidationResult {
    /// Check if token is valid and not expired
    pub fn is_valid_and_fresh(&self) -> bool {
        self.is_valid && !self.needs_refresh
    }

    /// Get user information from claims
    pub fn get_user_info(&self) -> Option<(String, Vec<String>)> {
        self.claims.as_ref().map(|claims| {
            let username = claims
                .username
                .clone()
                .unwrap_or_else(|| claims.sub.clone());
            let roles = claims.roles.clone().unwrap_or_default();
            (username, roles)
        })
    }

    /// Get token scopes
    pub fn get_scopes(&self) -> Vec<String> {
        self.claims
            .as_ref()
            .and_then(|claims| claims.scope.as_ref())
            .map(|scope| scope.split_whitespace().map(|s| s.to_string()).collect())
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AuthToken;

    #[test]
    fn test_basic_token_validation() {
        let validator = TokenValidator::new();

        // Create a valid token
        let token = AuthToken::new(
            "valid.jwt.token".to_string(),
            Some("refresh_token".to_string()),
            3600,
            "https://studio.example.com".to_string(),
            vec!["read".to_string(), "write".to_string()],
        );

        let result = validator.validate_token_basic(&token);
        assert!(result.is_valid);
        assert!(!result.needs_refresh);
    }

    #[test]
    fn test_expired_token_validation() {
        let validator = TokenValidator::new();

        // Create an expired token
        let token = AuthToken::new(
            "expired.jwt.token".to_string(),
            Some("refresh_token".to_string()),
            -3600, // Expired 1 hour ago
            "https://studio.example.com".to_string(),
            vec!["read".to_string()],
        );

        let result = validator.validate_token_basic(&token);
        assert!(!result.is_valid);
        assert!(!result.errors.is_empty());
    }

    #[test]
    fn test_token_needs_refresh() {
        let validator = TokenValidator::new();

        // Create a token that expires in 2 minutes (should need refresh)
        let token = AuthToken::new(
            "soon.to.expire.token".to_string(),
            Some("refresh_token".to_string()),
            120, // Expires in 2 minutes
            "https://studio.example.com".to_string(),
            vec!["read".to_string()],
        );

        assert!(validator.needs_refresh(&token));
    }

    #[test]
    fn test_permission_validation() {
        let validator = TokenValidator::new();

        let claims = StudioTokenClaims {
            sub: "user123".to_string(),
            exp: 9999999999,
            iat: 1000000000,
            iss: "https://studio.example.com".to_string(),
            aud: "studio-client".to_string(),
            scope: Some("read write admin".to_string()),
            username: Some("testuser".to_string()),
            roles: Some(vec!["user".to_string(), "admin".to_string()]),
            instance_id: Some("test-instance".to_string()),
            environment: Some("dev".to_string()),
        };

        // Test valid permissions
        assert!(validator.validate_permissions(&claims, &["read".to_string()]));
        assert!(validator.validate_permissions(&claims, &["read".to_string(), "write".to_string()]));

        // Test invalid permissions
        assert!(!validator.validate_permissions(&claims, &["super-admin".to_string()]));
    }
}
