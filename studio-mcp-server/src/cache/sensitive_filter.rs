//! Sensitive data filtering for cache security
//!
//! Prevents sensitive information from being cached by detecting and filtering
//! common patterns in PLM resource data including tokens, passwords, secrets,
//! and other authentication-related information.

use regex::Regex;
use serde_json::{Map, Value};
use std::collections::HashSet;
use tracing::{debug, warn};

/// Comprehensive filter for sensitive data in cache values
pub struct SensitiveDataFilter {
    /// Compiled regex patterns for sensitive data detection
    patterns: Vec<CompiledPattern>,
    /// Field names that should never be cached
    sensitive_fields: HashSet<String>,
    /// Keywords that indicate sensitive content
    sensitive_keywords: HashSet<String>,
}

/// Compiled regex pattern with metadata
struct CompiledPattern {
    regex: Regex,
    name: String,
    severity: Severity,
}

/// Severity level for sensitive data detection
#[derive(Debug, Clone, Copy)]
enum Severity {
    /// Critical - must never be cached (tokens, passwords)
    Critical,
    /// High - should not be cached (API keys, secrets)
    High,
    /// Medium - consider filtering (internal IDs, paths)
    Medium,
}

impl SensitiveDataFilter {
    /// Create a new sensitive data filter with comprehensive patterns
    pub fn new() -> Self {
        let patterns = Self::build_patterns();
        let sensitive_fields = Self::build_sensitive_fields();
        let sensitive_keywords = Self::build_sensitive_keywords();

        Self {
            patterns,
            sensitive_fields,
            sensitive_keywords,
        }
    }

    /// Check if a cache key indicates sensitive data that should not be cached
    pub fn should_skip_caching(&self, key: &str) -> bool {
        // Check for auth/secret-related cache keys
        let sensitive_key_patterns = [
            "auth",
            "token",
            "secret",
            "password",
            "credential",
            "login",
            "session",
            "jwt",
            "oauth",
            "api_key",
            "private_key",
            "cert",
            "certificate",
            "keystore",
            "vault",
            "encryption",
        ];

        let key_lower = key.to_lowercase();
        for pattern in &sensitive_key_patterns {
            if key_lower.contains(pattern) {
                debug!(
                    "Skipping cache for sensitive key pattern '{}': {}",
                    pattern, key
                );
                return true;
            }
        }

        // Check for auth command patterns
        if key_lower.contains("plm")
            && (key_lower.contains("auth")
                || key_lower.contains("login")
                || key_lower.contains("credential"))
        {
            debug!("Skipping cache for PLM auth operation: {}", key);
            return true;
        }

        false
    }

    /// Filter sensitive data from a JSON value before caching
    pub fn filter_value(&self, value: &Value) -> Value {
        match value {
            Value::Object(obj) => {
                let mut filtered = Map::new();
                for (key, val) in obj {
                    if self.is_sensitive_field(key) {
                        // Replace sensitive field with placeholder
                        filtered.insert(key.clone(), Value::String("[FILTERED]".to_string()));
                        warn!("Filtered sensitive field from cache: {}", key);
                    } else {
                        // Recursively filter nested objects/arrays, including string pattern filtering
                        filtered.insert(key.clone(), self.filter_value(val));
                    }
                }
                Value::Object(filtered)
            }
            Value::Array(arr) => Value::Array(arr.iter().map(|v| self.filter_value(v)).collect()),
            Value::String(s) => self.filter_string_value(s),
            _ => value.clone(),
        }
    }

    /// Filter sensitive patterns from string values
    fn filter_string_value(&self, value: &str) -> Value {
        let mut filtered = value.to_string();
        let mut _was_filtered = false;

        for pattern in &self.patterns {
            if pattern.regex.is_match(value) {
                match pattern.severity {
                    Severity::Critical | Severity::High => {
                        // Replace entire match with placeholder
                        filtered = pattern
                            .regex
                            .replace_all(&filtered, "[REDACTED]")
                            .to_string();
                        _was_filtered = true;
                        warn!(
                            "Filtered {} pattern '{}' from cache value",
                            match pattern.severity {
                                Severity::Critical => "critical",
                                Severity::High => "high",
                                _ => "medium",
                            },
                            pattern.name
                        );
                    }
                    Severity::Medium => {
                        // For medium severity, just log but don't filter
                        debug!(
                            "Detected medium sensitivity pattern '{}' in cache value",
                            pattern.name
                        );
                    }
                }
            }
        }

        Value::String(filtered)
    }

    /// Check if a field name indicates sensitive data
    fn is_sensitive_field(&self, field_name: &str) -> bool {
        let field_lower = field_name.to_lowercase();

        // Check exact matches
        if self.sensitive_fields.contains(&field_lower) {
            return true;
        }

        // Check for sensitive keywords in field name
        for keyword in &self.sensitive_keywords {
            if field_lower.contains(keyword) {
                return true;
            }
        }

        false
    }

    /// Build comprehensive regex patterns for sensitive data detection
    fn build_patterns() -> Vec<CompiledPattern> {
        let mut patterns = Vec::new();

        // JWT tokens
        if let Ok(regex) = Regex::new(r"eyJ[A-Za-z0-9+/=]+\.eyJ[A-Za-z0-9+/=]+\.[A-Za-z0-9+/=_-]+")
        {
            patterns.push(CompiledPattern {
                regex,
                name: "JWT_TOKEN".to_string(),
                severity: Severity::Critical,
            });
        }

        // Generic API keys (various formats)
        if let Ok(regex) =
            Regex::new(r#"(?i)(api[_-]?key|apikey)[=:\s]+['"]?([a-zA-Z0-9_-]{20,})['"]?"#)
        {
            patterns.push(CompiledPattern {
                regex,
                name: "API_KEY".to_string(),
                severity: Severity::Critical,
            });
        }

        // AWS keys
        if let Ok(regex) = Regex::new(r"AKIA[0-9A-Z]{16}") {
            patterns.push(CompiledPattern {
                regex,
                name: "AWS_ACCESS_KEY".to_string(),
                severity: Severity::Critical,
            });
        }

        // Bearer tokens
        if let Ok(regex) = Regex::new(r"(?i)bearer\s+[a-zA-Z0-9_-]{8,}") {
            patterns.push(CompiledPattern {
                regex,
                name: "BEARER_TOKEN".to_string(),
                severity: Severity::Critical,
            });
        }

        // Basic auth (base64 encoded)
        if let Ok(regex) = Regex::new(r"(?i)basic\s+[a-zA-Z0-9+/=]{20,}") {
            patterns.push(CompiledPattern {
                regex,
                name: "BASIC_AUTH".to_string(),
                severity: Severity::Critical,
            });
        }

        // Password patterns
        if let Ok(regex) = Regex::new(r#"(?i)(password|passwd|pwd)[=:\s]+['"]?([^\s'"]{8,})['"]?"#)
        {
            patterns.push(CompiledPattern {
                regex,
                name: "PASSWORD".to_string(),
                severity: Severity::Critical,
            });
        }

        // Private keys
        if let Ok(regex) = Regex::new(r"-----BEGIN [A-Z ]*PRIVATE KEY-----") {
            patterns.push(CompiledPattern {
                regex,
                name: "PRIVATE_KEY".to_string(),
                severity: Severity::Critical,
            });
        }

        // Generic tokens
        if let Ok(regex) = Regex::new(r#"(?i)(token|secret)[=:\s]+['"]?([a-zA-Z0-9_-]{16,})['"]?"#)
        {
            patterns.push(CompiledPattern {
                regex,
                name: "GENERIC_TOKEN".to_string(),
                severity: Severity::High,
            });
        }

        // Database connection strings
        if let Ok(regex) = Regex::new(r#"(?i)(mongodb|mysql|postgresql|redis)://[^\s'"]++"#) {
            patterns.push(CompiledPattern {
                regex,
                name: "DB_CONNECTION".to_string(),
                severity: Severity::High,
            });
        }

        // Internal system paths (medium sensitivity)
        if let Ok(regex) =
            Regex::new(r#"/etc/[a-zA-Z0-9_/-]+|/var/[a-zA-Z0-9_/-]+|C:\\[a-zA-Z0-9_\\-]+"#)
        {
            patterns.push(CompiledPattern {
                regex,
                name: "SYSTEM_PATH".to_string(),
                severity: Severity::Medium,
            });
        }

        patterns
    }

    /// Build set of sensitive field names
    fn build_sensitive_fields() -> HashSet<String> {
        [
            // Authentication fields
            "password",
            "passwd",
            "pwd",
            "secret",
            "token",
            "auth_token",
            "access_token",
            "refresh_token",
            "api_key",
            "apikey",
            "auth_key",
            "private_key",
            "public_key",
            "key",
            "certificate",
            "cert",
            // Session and auth
            "session",
            "session_id",
            "session_token",
            "csrf_token",
            "authorization",
            "credentials",
            "credential",
            // Encryption and security
            "encryption_key",
            "decrypt_key",
            "salt",
            "hash",
            "signature",
            "keystore",
            "truststore",
            "vault_token",
            // Database and connection
            "connection_string",
            "database_url",
            "db_password",
            "db_user",
            "jdbc_url",
            "redis_password",
            "mongo_uri",
            // Personal information
            "ssn",
            "social_security",
            "credit_card",
            "bank_account",
            "email_password",
            "phone",
            "address",
            // Cloud provider secrets
            "aws_access_key",
            "aws_secret_key",
            "azure_key",
            "gcp_key",
            "service_account_key",
            "client_secret",
            "client_id",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect()
    }

    /// Build set of sensitive keywords for field name detection
    fn build_sensitive_keywords() -> HashSet<String> {
        [
            "password",
            "secret",
            "token",
            "key",
            "auth",
            "credential",
            "private",
            "confidential",
            "sensitive",
            "encrypted",
            "secure",
            "cert",
            "signature",
            "hash",
            "vault",
            "keystore",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect()
    }
}

impl Default for SensitiveDataFilter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_sensitive_key_detection() {
        let filter = SensitiveDataFilter::new();

        // Should skip caching
        assert!(filter.should_skip_caching("auth:login:user123"));
        assert!(filter.should_skip_caching("plm:secret:vault"));
        assert!(filter.should_skip_caching("token:refresh:abc123"));
        assert!(filter.should_skip_caching("user:credentials:store"));

        // Should allow caching
        assert!(!filter.should_skip_caching("pipeline:list:all"));
        assert!(!filter.should_skip_caching("run:details:123"));
        assert!(!filter.should_skip_caching("task:definition:build"));
    }

    #[test]
    fn test_sensitive_field_filtering() {
        let filter = SensitiveDataFilter::new();

        let input = json!({
            "name": "test-pipeline",
            "status": "active",
            "password": "secret123",
            "api_key": "key_12345",
            "description": "A test pipeline",
            "auth_token": "bearer abc123",
            "config": {
                "timeout": 300,
                "secret": "hidden_value",
                "database_url": "postgres://user:pass@host/db"
            }
        });

        let filtered = filter.filter_value(&input);

        // Check that sensitive fields are filtered
        assert_eq!(filtered["password"], "[FILTERED]");
        assert_eq!(filtered["api_key"], "[FILTERED]");
        assert_eq!(filtered["auth_token"], "[FILTERED]");
        assert_eq!(filtered["config"]["secret"], "[FILTERED]");

        // Check that non-sensitive fields remain
        assert_eq!(filtered["name"], "test-pipeline");
        assert_eq!(filtered["status"], "active");
        assert_eq!(filtered["description"], "A test pipeline");
        assert_eq!(filtered["config"]["timeout"], 300);
    }

    #[test]
    fn test_jwt_token_filtering() {
        let filter = SensitiveDataFilter::new();

        let jwt = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c";
        let input = json!({
            "data": jwt,
            "user": "john.doe"
        });

        let filtered = filter.filter_value(&input);

        // JWT should be filtered from string values
        let token_str = filtered["data"].as_str().unwrap();
        assert!(token_str.contains("[REDACTED]"));
        assert!(!token_str.contains("eyJhbGciOiJIUzI1NiI"));

        // Non-sensitive field should remain
        assert_eq!(filtered["user"], "john.doe");
    }

    #[test]
    fn test_nested_filtering() {
        let filter = SensitiveDataFilter::new();

        let input = json!({
            "pipeline": {
                "name": "test",
                "auth": {
                    "username": "user",
                    "password": "secret123",
                    "tokens": ["token1", "token2"]
                }
            },
            "runs": [
                {
                    "id": "run1",
                    "config_data": "key123"
                }
            ]
        });

        let filtered = filter.filter_value(&input);

        // Check nested sensitive fields are filtered
        // The "auth" field itself is sensitive and should be filtered
        assert_eq!(filtered["pipeline"]["auth"], "[FILTERED]");
        assert_eq!(filtered["runs"][0]["config_data"], "key123");

        // Check non-sensitive fields remain
        assert_eq!(filtered["pipeline"]["name"], "test");
        assert_eq!(filtered["runs"][0]["id"], "run1");
    }

    #[test]
    fn test_bearer_token_filtering() {
        let filter = SensitiveDataFilter::new();

        let input = json!({
            "authorization": "Bearer abc123def456",
            "description": "Normal text with Bearer token123 embedded"
        });

        let filtered = filter.filter_value(&input);

        // Authorization field should be completely filtered
        assert_eq!(filtered["authorization"], "[FILTERED]");

        // Bearer token in description should be redacted
        let desc = filtered["description"].as_str().unwrap();
        assert!(desc.contains("[REDACTED]"));
        assert!(!desc.contains("Bearer token123"));
    }
}
