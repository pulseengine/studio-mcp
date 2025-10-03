//! Comprehensive mock WindRiver Studio server based on reverse engineering analysis
//!
//! This mock server simulates the complete WindRiver Studio MCP protocol including:
//! - OAuth 2.0/OIDC authentication with JWT tokens
//! - Multi-service resource providers (MCP, VLAB, Artifacts, Schedule)
//! - Versioned REST API endpoints (/api/v1/ through /api/v5/)
//! - JSON-RPC 2.0 message format compliance

use serde_json::{Value, json};
use std::collections::HashMap;
use tokio::sync::RwLock;
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{header, method, path, path_regex},
};

/// Mock WindRiver Studio server with complete protocol simulation
pub struct MockStudioServer {
    pub server: MockServer,
    pub base_url: String,
    /// JWT tokens for authentication simulation
    #[allow(dead_code)]
    pub tokens: RwLock<HashMap<String, JwtToken>>,
    /// Resource state for different providers
    pub resources: RwLock<StudioResources>,
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct JwtToken {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub expires_in: u64,
    pub scopes: Vec<String>,
}

#[derive(Default)]
pub struct StudioResources {
    pub artifacts: Vec<Artifact>,
    pub vlab_reservations: Vec<VlabReservation>,
    pub mcp_resources: Vec<McpResource>,
    pub scheduled_jobs: Vec<ScheduledJob>,
    pub users: Vec<User>,
    pub groups: Vec<Group>,
    #[allow(dead_code)]
    pub licenses: Vec<License>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Artifact {
    pub id: String,
    pub name: String,
    pub path: String,
    pub size: u64,
    pub created_by: String,
    pub created_date: String,
    pub artifact_type: String,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct VlabReservation {
    pub id: String,
    pub target_name: String,
    pub target_type: String,
    pub status: String,
    pub user_id: String,
    pub created_at: String,
    pub expires_at: String,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct McpResource {
    pub id: String,
    pub name: String,
    pub resource_type: String,
    pub wrrn: String,
    pub status: String,
    pub metadata: Value,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ScheduledJob {
    pub id: String,
    pub name: String,
    pub owner: String,
    pub job_type: i32,
    pub description: String,
    pub cron: String,
    pub endpoint: String,
    pub http_method: String,
    pub http_payload: String,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct User {
    pub id: String,
    pub username: String,
    pub email: String,
    pub first_name: String,
    pub last_name: String,
    pub roles: Vec<String>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Group {
    pub id: String,
    pub name: String,
    pub description: String,
    pub members: Vec<String>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct License {
    pub id: String,
    pub license_type: String,
    pub assigned_to: String,
    pub expiration_date: String,
    pub status: String,
}

impl MockStudioServer {
    /// Create a new mock WindRiver Studio server with all endpoints configured
    pub async fn new() -> Self {
        let server = MockServer::start().await;
        let base_url = server.uri();

        let mock_server = Self {
            server,
            base_url,
            tokens: RwLock::new(HashMap::new()),
            resources: RwLock::new(StudioResources::default()),
        };

        // Initialize mock data
        mock_server.initialize_mock_data().await;

        // Setup all API endpoints
        mock_server.setup_auth_endpoints().await;
        mock_server.setup_mcp_endpoints().await;
        // Setup error scenarios first so specific mocks take precedence
        mock_server.setup_error_scenarios().await;
        mock_server.setup_artifacts_endpoints().await;
        mock_server.setup_vlab_endpoints().await;
        mock_server.setup_schedule_endpoints().await;
        mock_server.setup_user_management_endpoints().await;

        mock_server
    }

    /// Initialize mock data for testing
    async fn initialize_mock_data(&self) {
        let mut resources = self.resources.write().await;

        // Sample artifacts
        resources.artifacts.push(Artifact {
            id: "artifact-001".to_string(),
            name: "libvxworks.so".to_string(),
            path: "/artifacts/vxworks/lib/libvxworks.so".to_string(),
            size: 1024000,
            created_by: "developer@windriver.com".to_string(),
            created_date: "2024-01-15T10:30:00Z".to_string(),
            artifact_type: "library".to_string(),
        });

        // Sample VLAB reservations
        resources.vlab_reservations.push(VlabReservation {
            id: "vlab-res-001".to_string(),
            target_name: "vxworks-sim-x86".to_string(),
            target_type: "simulator".to_string(),
            status: "active".to_string(),
            user_id: "user-001".to_string(),
            created_at: "2024-01-15T09:00:00Z".to_string(),
            expires_at: "2024-01-15T17:00:00Z".to_string(),
        });

        // Sample MCP resources
        resources.mcp_resources.push(McpResource {
            id: "mcp-res-001".to_string(),
            name: "VxWorks Build System".to_string(),
            resource_type: "build_system".to_string(),
            wrrn: "wr:build:vxworks:main".to_string(),
            status: "available".to_string(),
            metadata: json!({
                "version": "24.03",
                "architecture": "x86_64",
                "features": ["smp", "rtp", "networking"]
            }),
        });

        // Sample scheduled jobs
        resources.scheduled_jobs.push(ScheduledJob {
            id: "job-001".to_string(),
            name: "Nightly Build".to_string(),
            owner: "build-system".to_string(),
            job_type: 1,
            description: "Daily VxWorks kernel build".to_string(),
            cron: "0 2 * * *".to_string(),
            endpoint: "/api/v3/builds/vxworks".to_string(),
            http_method: "POST".to_string(),
            http_payload: json!({"config": "release", "target": "x86_64"}).to_string(),
        });

        // Sample users and groups
        resources.users.push(User {
            id: "user-001".to_string(),
            username: "developer".to_string(),
            email: "developer@windriver.com".to_string(),
            first_name: "John".to_string(),
            last_name: "Developer".to_string(),
            roles: vec!["mcp-developer".to_string(), "vlab-user".to_string()],
        });

        resources.groups.push(Group {
            id: "group-001".to_string(),
            name: "mcp-developers".to_string(),
            description: "MCP Development Team".to_string(),
            members: vec!["user-001".to_string()],
        });
    }

    /// Setup OAuth 2.0/OIDC authentication endpoints
    async fn setup_auth_endpoints(&self) {
        // OIDC Discovery endpoint
        Mock::given(method("GET"))
            .and(path("/.well-known/openid_configuration"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "issuer": self.base_url,
                "authorization_endpoint": format!("{}/auth/realms/studio/protocol/openid-connect/auth", self.base_url),
                "token_endpoint": format!("{}/auth/realms/studio/protocol/openid-connect/token", self.base_url),
                "userinfo_endpoint": format!("{}/auth/realms/studio/protocol/openid-connect/userinfo", self.base_url),
                "jwks_uri": format!("{}/auth/realms/studio/protocol/openid-connect/certs", self.base_url),
                "response_types_supported": ["code", "token", "id_token"],
                "subject_types_supported": ["public"],
                "id_token_signing_alg_values_supported": ["RS256"]
            })))
            .mount(&self.server)
            .await;

        // Token endpoint for OAuth 2.0 flow
        Mock::given(method("POST"))
            .and(path("/auth/realms/studio/protocol/openid-connect/token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "access_token": "eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9.mock_token",
                "refresh_token": "refresh_mock_token",
                "id_token": "eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9.mock_id_token",
                "token_type": "Bearer",
                "expires_in": 3600,
                "scope": "openid profile email mcp:read mcp:write vlab:access artifacts:manage"
            })))
            .mount(&self.server)
            .await;

        // User info endpoint
        Mock::given(method("GET"))
            .and(path("/auth/realms/studio/protocol/openid-connect/userinfo"))
            .and(header(
                "authorization",
                "Bearer eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9.mock_token",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "sub": "user-001",
                "username": "developer",
                "email": "developer@windriver.com",
                "given_name": "John",
                "family_name": "Developer",
                "realm_access": {
                    "roles": ["mcp-developer", "vlab-user", "artifacts-user"]
                }
            })))
            .mount(&self.server)
            .await;

        // Admin user management endpoints
        Mock::given(method("GET"))
            .and(path_regex(r"^/auth/admin/realms/studio/users"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([
                {
                    "id": "user-001",
                    "username": "developer",
                    "email": "developer@windriver.com",
                    "firstName": "John",
                    "lastName": "Developer",
                    "enabled": true
                }
            ])))
            .mount(&self.server)
            .await;
    }

    /// Setup MCP resource provider endpoints
    async fn setup_mcp_endpoints(&self) {
        // MCP resources list endpoint - only with valid authorization
        Mock::given(method("GET"))
            .and(path_regex(r"^/api/v[1-5]/resources"))
            .and(header(
                "authorization",
                "Bearer eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9.mock_token",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "data": [
                    {
                        "id": "mcp-res-001",
                        "name": "VxWorks Build System",
                        "type": "build_system",
                        "wrrn": "wr:build:vxworks:main",
                        "status": "available",
                        "metadata": {
                            "version": "24.03",
                            "architecture": "x86_64",
                            "features": ["smp", "rtp", "networking"]
                        }
                    }
                ],
                "totalRows": 1,
                "status": "success"
            })))
            .mount(&self.server)
            .await;

        // MCP resource creation
        Mock::given(method("POST"))
            .and(path_regex(r"^/api/v[1-5]/resources"))
            .respond_with(ResponseTemplate::new(201).set_body_json(json!({
                "data": {
                    "id": "mcp-res-002",
                    "status": "created"
                },
                "status": "success",
                "message": "Resource created successfully"
            })))
            .mount(&self.server)
            .await;

        // License management endpoints
        Mock::given(method("POST"))
            .and(path("/license/assign"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "status": "success",
                "message": "License assigned successfully",
                "data": {
                    "license_id": "lic-001",
                    "assigned_to": "user-001"
                }
            })))
            .mount(&self.server)
            .await;

        Mock::given(method("POST"))
            .and(path("/license/revoke"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "status": "success",
                "message": "License revoked successfully"
            })))
            .mount(&self.server)
            .await;
    }

    /// Setup artifacts management endpoints
    async fn setup_artifacts_endpoints(&self) {
        // List artifacts
        Mock::given(method("GET"))
            .and(path_regex(r"^/api/artifacts"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "data": [
                    {
                        "id": "artifact-001",
                        "name": "libvxworks.so",
                        "path": "/artifacts/vxworks/lib/libvxworks.so",
                        "size": 1024000,
                        "created_by": "developer@windriver.com",
                        "created_date": "2024-01-15T10:30:00Z",
                        "type": "library"
                    }
                ],
                "totalRows": 1,
                "status": "success"
            })))
            .mount(&self.server)
            .await;

        // Upload artifact
        Mock::given(method("POST"))
            .and(path("/api/artifacts"))
            .respond_with(ResponseTemplate::new(201).set_body_json(json!({
                "data": {
                    "id": "artifact-002",
                    "upload_url": format!("{}/api/artifacts/upload/artifact-002", self.base_url)
                },
                "status": "success",
                "message": "Artifact upload initiated"
            })))
            .mount(&self.server)
            .await;

        // Get artifact token
        Mock::given(method("GET"))
            .and(path("/artifacts/token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "data": {
                    "token": "artifacts_access_token_12345",
                    "expires_in": 3600
                },
                "status": "success"
            })))
            .mount(&self.server)
            .await;
    }

    /// Setup VLAB (Virtual Lab) endpoints
    async fn setup_vlab_endpoints(&self) {
        // List VLAB targets
        Mock::given(method("GET"))
            .and(path("/api/vlab/targets"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "data": [
                    {
                        "id": "target-001",
                        "name": "vxworks-sim-x86",
                        "type": "simulator",
                        "architecture": "x86_64",
                        "status": "available",
                        "capabilities": ["debug", "profiling", "network"]
                    },
                    {
                        "id": "target-002",
                        "name": "linux-qemu-arm",
                        "type": "emulator",
                        "architecture": "aarch64",
                        "status": "busy",
                        "capabilities": ["debug", "graphics"]
                    }
                ],
                "status": "success"
            })))
            .mount(&self.server)
            .await;

        // Create VLAB reservation
        Mock::given(method("POST"))
            .and(path("/api/vlab/reservations"))
            .respond_with(ResponseTemplate::new(201).set_body_json(json!({
                "data": {
                    "id": "vlab-res-002",
                    "target_id": "target-001",
                    "reservation_url": format!("{}/vlab/connect/vlab-res-002", self.base_url),
                    "expires_at": "2024-01-15T17:00:00Z"
                },
                "status": "success",
                "message": "VLAB reservation created"
            })))
            .mount(&self.server)
            .await;

        // List reservations
        Mock::given(method("GET"))
            .and(path("/api/vlab/reservations"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "data": [
                    {
                        "id": "vlab-res-001",
                        "target_name": "vxworks-sim-x86",
                        "target_type": "simulator",
                        "status": "active",
                        "user_id": "user-001",
                        "created_at": "2024-01-15T09:00:00Z",
                        "expires_at": "2024-01-15T17:00:00Z"
                    }
                ],
                "status": "success"
            })))
            .mount(&self.server)
            .await;
    }

    /// Setup scheduled job management endpoints
    async fn setup_schedule_endpoints(&self) {
        // List scheduled jobs
        Mock::given(method("GET"))
            .and(path("/schedule/jobs"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "data": [
                    {
                        "id": "job-001",
                        "name": "Nightly Build",
                        "owner": "build-system",
                        "type": 1,
                        "description": "Daily VxWorks kernel build",
                        "cron": "0 2 * * *",
                        "scheduleOptions": {
                            "endpoint": "/api/v3/builds/vxworks",
                            "httpMethod": "POST",
                            "httpPayload": "{\"config\":\"release\",\"target\":\"x86_64\"}"
                        }
                    }
                ],
                "status": "success"
            })))
            .mount(&self.server)
            .await;

        // Create scheduled job
        Mock::given(method("POST"))
            .and(path("/schedule/jobs"))
            .respond_with(ResponseTemplate::new(201).set_body_json(json!({
                "data": {
                    "id": "job-002",
                    "status": "created"
                },
                "status": "success",
                "message": "Scheduled job created successfully"
            })))
            .mount(&self.server)
            .await;

        // Job execution endpoints
        Mock::given(method("GET"))
            .and(path_regex(r"^/schedule/executions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "data": [
                    {
                        "id": "exec-001",
                        "job_id": "job-001",
                        "status": "completed",
                        "started_at": "2024-01-15T02:00:00Z",
                        "completed_at": "2024-01-15T02:45:00Z",
                        "result": "success"
                    }
                ],
                "status": "success"
            })))
            .mount(&self.server)
            .await;
    }

    /// Setup user management endpoints
    async fn setup_user_management_endpoints(&self) {
        // User operations
        Mock::given(method("GET"))
            .and(path_regex(r"^/auth/users"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "data": [
                    {
                        "id": "user-001",
                        "username": "developer",
                        "email": "developer@windriver.com",
                        "first_name": "John",
                        "last_name": "Developer",
                        "roles": ["mcp-developer", "vlab-user"]
                    }
                ],
                "status": "success"
            })))
            .mount(&self.server)
            .await;

        // Group operations
        Mock::given(method("GET"))
            .and(path_regex(r"^/auth/groups"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "data": [
                    {
                        "id": "group-001",
                        "name": "mcp-developers",
                        "description": "MCP Development Team",
                        "members": ["user-001"]
                    }
                ],
                "status": "success"
            })))
            .mount(&self.server)
            .await;

        // Role assignment
        Mock::given(method("POST"))
            .and(path_regex(r"^/auth/roles/.*/users"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "status": "success",
                "message": "Role assigned successfully"
            })))
            .mount(&self.server)
            .await;
    }

    /// Get a mock JWT token for testing
    pub async fn get_mock_token(&self) -> String {
        "eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9.mock_token".to_string()
    }

    /// Simulate specific API responses for testing edge cases
    pub async fn setup_error_scenarios(&self) {
        // Unauthorized access for specific invalid token
        Mock::given(method("GET"))
            .and(path("/api/v1/resources"))
            .and(header("authorization", "Bearer invalid_token"))
            .respond_with(ResponseTemplate::new(401).set_body_json(json!({
                "error": "invalid_token",
                "error_description": "The access token is invalid or expired",
                "status": "error"
            })))
            .mount(&self.server)
            .await;

        // Rate limiting
        Mock::given(method("POST"))
            .and(path("/api/artifacts"))
            .and(header("x-test-scenario", "rate_limit"))
            .respond_with(ResponseTemplate::new(429).set_body_json(json!({
                "error": "rate_limit_exceeded",
                "message": "Too many requests. Please try again later.",
                "retry_after": 60
            })))
            .mount(&self.server)
            .await;

        // Server errors
        Mock::given(method("POST"))
            .and(path("/schedule/jobs"))
            .and(header("x-test-scenario", "server_error"))
            .respond_with(ResponseTemplate::new(500).set_body_json(json!({
                "error": "internal_server_error",
                "message": "An unexpected error occurred",
                "request_id": "req-12345"
            })))
            .mount(&self.server)
            .await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::Client;

    #[tokio::test]
    async fn test_mock_server_auth_flow() {
        let mock_server = MockStudioServer::new().await;
        let client = Client::new();

        // Test OIDC discovery
        let response = client
            .get(format!(
                "{}/.well-known/openid_configuration",
                mock_server.base_url
            ))
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), 200);
        let discovery: Value = response.json().await.unwrap();
        assert!(discovery.get("token_endpoint").is_some());

        // Test token exchange
        let token_response = client
            .post(format!(
                "{}/auth/realms/studio/protocol/openid-connect/token",
                mock_server.base_url
            ))
            .form(&[
                ("grant_type", "authorization_code"),
                ("code", "mock_auth_code"),
                ("client_id", "studio-cli"),
            ])
            .send()
            .await
            .unwrap();

        assert_eq!(token_response.status(), 200);
        let token_data: Value = token_response.json().await.unwrap();
        assert!(token_data.get("access_token").is_some());
        assert!(token_data.get("token_type").is_some());
    }

    #[tokio::test]
    async fn test_mock_server_mcp_resources() {
        let mock_server = MockStudioServer::new().await;
        let client = Client::new();
        let token = mock_server.get_mock_token().await;

        // Test MCP resources list
        let response = client
            .get(format!("{}/api/v1/resources", mock_server.base_url))
            .header("authorization", format!("Bearer {token}"))
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), 200);
        let resources: Value = response.json().await.unwrap();
        assert_eq!(resources["status"], "success");
        assert!(resources["data"].is_array());
    }

    #[tokio::test]
    async fn test_mock_server_vlab_operations() {
        let mock_server = MockStudioServer::new().await;
        let client = Client::new();
        let token = mock_server.get_mock_token().await;

        // Test VLAB targets list
        let response = client
            .get(format!("{}/api/vlab/targets", mock_server.base_url))
            .header("authorization", format!("Bearer {token}"))
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), 200);
        let targets: Value = response.json().await.unwrap();
        assert_eq!(targets["status"], "success");
        assert!(targets["data"].is_array());

        // Test VLAB reservation creation
        let reservation_response = client
            .post(format!("{}/api/vlab/reservations", mock_server.base_url))
            .header("authorization", format!("Bearer {token}"))
            .json(&json!({
                "target_id": "target-001",
                "duration": 8
            }))
            .send()
            .await
            .unwrap();

        assert_eq!(reservation_response.status(), 201);
        let reservation: Value = reservation_response.json().await.unwrap();
        assert_eq!(reservation["status"], "success");
        assert!(reservation["data"]["id"].is_string());
    }
}
