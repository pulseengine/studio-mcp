use assert_cmd::Command;
use serde_json::Value;
use std::process::Command as StdCommand;
use tempfile::NamedTempFile;

mod mock_plm_server;
mod mock_studio_server;
use mock_plm_server::MockPlmServer;
use mock_studio_server::MockStudioServer;

/// Test the binary exists and can be executed
#[test]
fn test_binary_exists() {
    let mut cmd = Command::cargo_bin("studio-mcp-server").unwrap();
    // The binary requires a config file or --init, so test with --init and a temp path
    let temp_config = tempfile::NamedTempFile::new().unwrap();
    let config_path = temp_config.path();
    std::fs::remove_file(config_path).ok(); // Remove the empty file so --init can create it

    cmd.arg("--init").arg(config_path).assert().success();
}

/// Test configuration file initialization
#[test]
fn test_config_initialization() {
    let temp_config = NamedTempFile::new().unwrap();
    let config_path = temp_config.path();
    std::fs::remove_file(config_path).ok(); // Remove the empty file so --init can create it
    let config_path = config_path.to_str().unwrap();

    let mut cmd = Command::cargo_bin("studio-mcp-server").unwrap();
    cmd.arg("--init")
        .arg(config_path)
        .assert()
        .success()
        .stdout(predicates::str::contains("Configuration file"));

    // Verify the config file was created and contains valid JSON
    let config_content = std::fs::read_to_string(config_path).unwrap();
    let config: Value = serde_json::from_str(&config_content).unwrap();

    // Verify essential config structure
    assert!(config.get("connections").is_some());
    assert!(config.get("cli").is_some());
    assert!(config.get("cache").is_some());
    assert!(config.get("logging").is_some());

    // Verify timeout configuration is present
    let cli_config = config.get("cli").unwrap();
    assert!(cli_config.get("timeouts").is_some());

    let timeouts = cli_config.get("timeouts").unwrap();
    assert!(timeouts.get("quick_operations").is_some());
    assert!(timeouts.get("medium_operations").is_some());
    assert!(timeouts.get("long_operations").is_some());
}

/// Test invalid arguments produce proper error messages
#[test]
fn test_invalid_arguments() {
    let mut cmd = Command::cargo_bin("studio-mcp-server").unwrap();
    cmd.arg("--invalid-flag").assert().failure();
}

/// Test server startup with valid config
#[tokio::test]
async fn test_server_startup_shutdown() {
    let temp_config = NamedTempFile::new().unwrap();
    let config_path = temp_config.path();
    std::fs::remove_file(config_path).ok(); // Remove the empty file so --init can create it
    let config_path = config_path.to_str().unwrap();

    // First create a config file
    let mut init_cmd = Command::cargo_bin("studio-mcp-server").unwrap();
    init_cmd.arg("--init").arg(config_path).assert().success();

    // Test that server can start (it will wait for stdin, so we'll start and kill quickly)
    let mut server_process = StdCommand::new(env!("CARGO_BIN_EXE_studio-mcp-server"))
        .arg(config_path)
        .spawn()
        .expect("Failed to start server");

    // Give it a moment to start
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Check if process is still running (means it started successfully)
    assert!(
        server_process.try_wait().unwrap().is_none(),
        "Server should still be running"
    );

    // Kill the server
    server_process.kill().expect("Failed to kill server");
    let exit_status = server_process.wait().expect("Failed to wait for server");

    // On Unix systems, kill() results in a specific exit code
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        assert!(exit_status.signal().is_some() || exit_status.code().is_some());
    }

    #[cfg(windows)]
    {
        // On Windows, we just verify the process terminated
        assert!(exit_status.code().is_some());
    }
}

/// Test server with invalid config file
#[test]
fn test_server_invalid_config() {
    let temp_config = NamedTempFile::new().unwrap();

    // Write invalid JSON to config file
    std::fs::write(temp_config.path(), "{ invalid json }").unwrap();

    let mut cmd = Command::cargo_bin("studio-mcp-server").unwrap();
    cmd.arg(temp_config.path().to_str().unwrap())
        .assert()
        .failure();
}

/// Test server with non-existent config file
#[test]
fn test_server_missing_config() {
    let mut cmd = Command::cargo_bin("studio-mcp-server").unwrap();
    cmd.arg("/nonexistent/config.json").assert().failure();
}

/// Test complete WindRiver Studio MCP protocol integration
#[tokio::test]
async fn test_windriver_studio_mcp_integration() {
    let mock_server = MockStudioServer::new().await;
    let temp_config = NamedTempFile::new().unwrap();
    let config_path = temp_config.path();
    std::fs::remove_file(config_path).ok();
    let config_path = config_path.to_str().unwrap();

    // Create config with mock server URL
    let config_content = format!(
        r#"{{
        "connections": {{
            "mock_studio": {{
                "name": "mock_studio",
                "url": "{}",
                "username": "test_user",
                "token": "test_token"
            }}
        }},
        "default_connection": "mock_studio",
        "cli": {{
            "download_base_url": "https://example.com/cli",
            "version": "auto",
            "timeout": 60,
            "timeouts": {{
                "quick_operations": 5,
                "medium_operations": 30,
                "long_operations": 300,
                "network_requests": 60
            }},
            "auto_update": false,
            "update_check_interval": 24
        }},
        "cache": {{
            "enabled": true,
            "ttl": 3600,
            "max_size": 1000
        }},
        "logging": {{
            "level": "info",
            "format": "json",
            "file_logging": false,
            "log_file": null
        }}
    }}"#,
        mock_server.base_url
    );

    std::fs::write(config_path, config_content).unwrap();

    // Test server startup with mock WindRiver Studio
    let mut server_process = StdCommand::new(env!("CARGO_BIN_EXE_studio-mcp-server"))
        .arg(config_path)
        .spawn()
        .expect("Failed to start server with WindRiver Studio config");

    // Give server time to initialize
    tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;

    // Verify server is running
    assert!(
        server_process.try_wait().unwrap().is_none(),
        "Server should be running with WindRiver Studio integration"
    );

    // Clean shutdown
    server_process.kill().expect("Failed to kill server");
    let _ = server_process.wait();
}

/// Test MCP resource provider operations with mock WindRiver Studio
#[tokio::test]
async fn test_mcp_resource_operations() {
    let mock_server = MockStudioServer::new().await;
    let client = reqwest::Client::new();
    let token = mock_server.get_mock_token().await;

    // Test resource listing
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

    // Test license operations
    let license_response = client
        .post(format!("{}/license/assign", mock_server.base_url))
        .header("authorization", format!("Bearer {token}"))
        .json(&serde_json::json!({
            "license_id": "lic-001",
            "user_id": "user-001"
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(license_response.status(), 200);
    let license_result: Value = license_response.json().await.unwrap();
    assert_eq!(license_result["status"], "success");
}

/// Test VLAB virtual lab integration
#[tokio::test]
async fn test_vlab_integration() {
    let mock_server = MockStudioServer::new().await;
    let client = reqwest::Client::new();
    let token = mock_server.get_mock_token().await;

    // Test VLAB targets
    let targets_response = client
        .get(format!("{}/api/vlab/targets", mock_server.base_url))
        .header("authorization", format!("Bearer {token}"))
        .send()
        .await
        .unwrap();

    assert_eq!(targets_response.status(), 200);
    let targets: Value = targets_response.json().await.unwrap();
    assert_eq!(targets["status"], "success");
    assert!(targets["data"].is_array());

    // Verify target structure matches WindRiver Studio format
    let first_target = &targets["data"][0];
    assert!(first_target["id"].is_string());
    assert!(first_target["name"].is_string());
    assert!(first_target["type"].is_string());
    assert!(first_target["architecture"].is_string());
    assert!(first_target["capabilities"].is_array());

    // Test reservation creation
    let reservation_response = client
        .post(format!("{}/api/vlab/reservations", mock_server.base_url))
        .header("authorization", format!("Bearer {token}"))
        .json(&serde_json::json!({
            "target_id": "target-001",
            "duration": 8
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(reservation_response.status(), 201);
    let reservation: Value = reservation_response.json().await.unwrap();
    assert_eq!(reservation["status"], "success");
    assert!(reservation["data"]["reservation_url"].is_string());
}

/// Test artifacts management system
#[tokio::test]
async fn test_artifacts_management() {
    let mock_server = MockStudioServer::new().await;
    let client = reqwest::Client::new();
    let token = mock_server.get_mock_token().await;

    // Test artifact listing
    let artifacts_response = client
        .get(format!("{}/api/artifacts", mock_server.base_url))
        .header("authorization", format!("Bearer {token}"))
        .send()
        .await
        .unwrap();

    assert_eq!(artifacts_response.status(), 200);
    let artifacts: Value = artifacts_response.json().await.unwrap();
    assert_eq!(artifacts["status"], "success");
    assert!(artifacts["data"].is_array());

    // Verify artifact structure
    let first_artifact = &artifacts["data"][0];
    assert!(first_artifact["id"].is_string());
    assert!(first_artifact["name"].is_string());
    assert!(first_artifact["path"].is_string());
    assert!(first_artifact["size"].is_number());
    assert!(first_artifact["type"].is_string());

    // Test artifact upload initiation
    let upload_response = client
        .post(format!("{}/api/artifacts", mock_server.base_url))
        .header("authorization", format!("Bearer {token}"))
        .json(&serde_json::json!({
            "name": "test-artifact.so",
            "type": "library",
            "size": 2048000
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(upload_response.status(), 201);
    let upload_result: Value = upload_response.json().await.unwrap();
    assert_eq!(upload_result["status"], "success");
    assert!(upload_result["data"]["upload_url"].is_string());
}

/// Test scheduled job management
#[tokio::test]
async fn test_schedule_management() {
    let mock_server = MockStudioServer::new().await;
    let client = reqwest::Client::new();
    let token = mock_server.get_mock_token().await;

    // Test job listing
    let jobs_response = client
        .get(format!("{}/schedule/jobs", mock_server.base_url))
        .header("authorization", format!("Bearer {token}"))
        .send()
        .await
        .unwrap();

    assert_eq!(jobs_response.status(), 200);
    let jobs: Value = jobs_response.json().await.unwrap();
    assert_eq!(jobs["status"], "success");
    assert!(jobs["data"].is_array());

    // Verify job structure matches ApiCallJob from reverse engineering
    let first_job = &jobs["data"][0];
    assert!(first_job["id"].is_string());
    assert!(first_job["name"].is_string());
    assert!(first_job["owner"].is_string());
    assert!(first_job["type"].is_number());
    assert!(first_job["cron"].is_string());
    assert!(first_job["scheduleOptions"].is_object());

    let schedule_options = &first_job["scheduleOptions"];
    assert!(schedule_options["endpoint"].is_string());
    assert!(schedule_options["httpMethod"].is_string());
    assert!(schedule_options["httpPayload"].is_string());

    // Test job creation
    let create_response = client
        .post(format!("{}/schedule/jobs", mock_server.base_url))
        .header("authorization", format!("Bearer {token}"))
        .json(&serde_json::json!({
            "name": "Test Build Job",
            "owner": "test-user",
            "type": 1,
            "description": "Test build automation",
            "cron": "0 3 * * *",
            "scheduleOptions": {
                "endpoint": "/api/v3/builds/test",
                "httpMethod": "POST",
                "httpPayload": "{\"config\":\"debug\"}"
            }
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(create_response.status(), 201);
    let create_result: Value = create_response.json().await.unwrap();
    assert_eq!(create_result["status"], "success");
}

/// Test OAuth 2.0/OIDC authentication flow
#[tokio::test]
async fn test_oauth_authentication_flow() {
    let mock_server = MockStudioServer::new().await;
    let client = reqwest::Client::new();

    // Test OIDC discovery
    let discovery_response = client
        .get(format!(
            "{}/.well-known/openid_configuration",
            mock_server.base_url
        ))
        .send()
        .await
        .unwrap();

    assert_eq!(discovery_response.status(), 200);
    let discovery: Value = discovery_response.json().await.unwrap();
    assert!(discovery["issuer"].is_string());
    assert!(discovery["authorization_endpoint"].is_string());
    assert!(discovery["token_endpoint"].is_string());
    assert!(discovery["userinfo_endpoint"].is_string());

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
            ("redirect_uri", "http://localhost:8250/oidc/callback"),
        ])
        .send()
        .await
        .unwrap();

    assert_eq!(token_response.status(), 200);
    let token_data: Value = token_response.json().await.unwrap();
    assert!(token_data["access_token"].is_string());
    assert!(token_data["refresh_token"].is_string());
    assert!(token_data["id_token"].is_string());
    assert_eq!(token_data["token_type"], "Bearer");
    assert_eq!(token_data["expires_in"], 3600);

    // Test userinfo endpoint
    let userinfo_response = client
        .get(format!(
            "{}/auth/realms/studio/protocol/openid-connect/userinfo",
            mock_server.base_url
        ))
        .header(
            "authorization",
            format!("Bearer {}", token_data["access_token"].as_str().unwrap()),
        )
        .send()
        .await
        .unwrap();

    assert_eq!(userinfo_response.status(), 200);
    let userinfo: Value = userinfo_response.json().await.unwrap();
    assert!(userinfo["sub"].is_string());
    assert!(userinfo["username"].is_string());
    assert!(userinfo["email"].is_string());
    assert!(userinfo["realm_access"]["roles"].is_array());
}

/// Test error handling and edge cases
#[tokio::test]
async fn test_error_handling() {
    let mock_server = MockStudioServer::new().await;
    let client = reqwest::Client::new();

    // Test unauthorized access
    let unauth_response = client
        .get(format!("{}/api/v1/resources", mock_server.base_url))
        .header("authorization", "Bearer invalid_token")
        .send()
        .await
        .unwrap();

    assert_eq!(unauth_response.status(), 401);
    let error: Value = unauth_response.json().await.unwrap();
    assert_eq!(error["error"], "invalid_token");
    assert_eq!(error["status"], "error");

    // Test rate limiting
    let rate_limit_response = client
        .post(format!("{}/api/artifacts", mock_server.base_url))
        .header("x-test-scenario", "rate_limit")
        .send()
        .await
        .unwrap();

    assert_eq!(rate_limit_response.status(), 429);
    let rate_limit_error: Value = rate_limit_response.json().await.unwrap();
    assert_eq!(rate_limit_error["error"], "rate_limit_exceeded");

    // Test server errors
    let server_error_response = client
        .post(format!("{}/schedule/jobs", mock_server.base_url))
        .header("x-test-scenario", "server_error")
        .send()
        .await
        .unwrap();

    assert_eq!(server_error_response.status(), 500);
    let server_error: Value = server_error_response.json().await.unwrap();
    assert_eq!(server_error["error"], "internal_server_error");
}

/// Test configuration validation
#[test]
fn test_config_validation() {
    let temp_config = NamedTempFile::new().unwrap();

    // Test with missing required fields
    let invalid_config = serde_json::json!({
        "connections": {}
        // Missing other required fields
    });

    std::fs::write(temp_config.path(), invalid_config.to_string()).unwrap();

    let mut cmd = Command::cargo_bin("studio-mcp-server").unwrap();
    cmd.arg(temp_config.path().to_str().unwrap())
        .assert()
        .failure();
}

/// Test timeout configuration values
#[test]
fn test_timeout_config_values() {
    let temp_config = NamedTempFile::new().unwrap();
    let config_path = temp_config.path();
    std::fs::remove_file(config_path).ok(); // Remove the empty file so --init can create it
    let config_path = config_path.to_str().unwrap();

    // Create config with custom timeout values
    let mut cmd = Command::cargo_bin("studio-mcp-server").unwrap();
    cmd.arg("--init").arg(config_path).assert().success();

    // Read and verify timeout values are reasonable
    let config_content = std::fs::read_to_string(config_path).unwrap();
    let config: Value = serde_json::from_str(&config_content).unwrap();

    let timeouts = config["cli"]["timeouts"].as_object().unwrap();

    // Verify timeout values are positive numbers
    assert!(timeouts["quick_operations"].as_u64().unwrap() > 0);
    assert!(timeouts["medium_operations"].as_u64().unwrap() > 0);
    assert!(timeouts["long_operations"].as_u64().unwrap() > 0);
    assert!(timeouts["network_requests"].as_u64().unwrap() > 0);

    // Verify logical ordering (quick < medium < long)
    let quick = timeouts["quick_operations"].as_u64().unwrap();
    let medium = timeouts["medium_operations"].as_u64().unwrap();
    let long = timeouts["long_operations"].as_u64().unwrap();

    assert!(quick <= medium);
    assert!(medium <= long);
}

/// Test comprehensive PLM (Pipeline Management) integration
#[tokio::test]
async fn test_plm_comprehensive_integration() {
    let plm_server = MockPlmServer::new().await;
    let client = reqwest::Client::new();

    // Test pipeline types listing
    let types_response = client
        .get(format!("{}/api/plm/pipeline-types", plm_server.base_url))
        .send()
        .await
        .unwrap();

    assert_eq!(types_response.status(), 200);
    let types: Value = types_response.json().await.unwrap();
    assert_eq!(types["status"], "success");
    assert!(types["data"].is_array());

    // Verify comprehensive pipeline types
    let pipeline_types = types["data"].as_array().unwrap();
    assert!(pipeline_types.len() >= 20); // Should have 20+ pipeline types

    // Check for key pipeline types from PLM analysis
    let type_names: Vec<String> = pipeline_types
        .iter()
        .map(|t| t["name"].as_str().unwrap().to_string())
        .collect();

    assert!(type_names.contains(&"VxWorks Kernel Build".to_string()));
    assert!(type_names.contains(&"Linux Application Build".to_string()));
    assert!(type_names.contains(&"ARM Cross-Compilation".to_string()));
    assert!(type_names.contains(&"Unit Testing".to_string()));
    assert!(type_names.contains(&"Security Scanning".to_string()));

    // Test pipeline creation
    let create_response = client
        .post(format!("{}/api/plm/pipelines", plm_server.base_url))
        .json(&serde_json::json!({
            "name": "Test VxWorks Build",
            "type": "vxworks_kernel",
            "config": {
                "target_arch": "x86_64",
                "build_type": "release",
                "features": ["smp", "networking"]
            }
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(create_response.status(), 201);
    let created_pipeline: Value = create_response.json().await.unwrap();
    assert_eq!(created_pipeline["status"], "success");
    assert!(created_pipeline["data"]["id"].is_string());

    let pipeline_id = created_pipeline["data"]["id"].as_str().unwrap();

    // Test pipeline execution
    let run_response = client
        .post(format!(
            "{}/api/plm/pipelines/{}/runs",
            plm_server.base_url, pipeline_id
        ))
        .json(&serde_json::json!({
            "trigger": "manual",
            "parameters": {
                "branch": "main",
                "clean_build": true
            }
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(run_response.status(), 201);
    let run_result: Value = run_response.json().await.unwrap();
    assert_eq!(run_result["status"], "success");
    assert!(run_result["data"]["run_id"].is_string());

    let run_id = run_result["data"]["run_id"].as_str().unwrap();

    // Give the mock pipeline time to progress through stages
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Test run status monitoring
    let status_response = client
        .get(format!("{}/api/plm/runs/{}", plm_server.base_url, run_id))
        .send()
        .await
        .unwrap();

    assert_eq!(status_response.status(), 200);
    let run_status: Value = status_response.json().await.unwrap();
    assert_eq!(run_status["status"], "success");
    assert!(run_status["data"]["tasks"].is_array());

    // Verify build lifecycle tasks
    let tasks = run_status["data"]["tasks"].as_array().unwrap();
    let task_names: Vec<String> = tasks
        .iter()
        .map(|t| t["name"].as_str().unwrap().to_string())
        .collect();

    assert!(task_names.contains(&"checkout".to_string()));
    assert!(task_names.contains(&"configure".to_string()));
    assert!(task_names.contains(&"compile".to_string()));
}

/// Test PLM resource management and monitoring
#[tokio::test]
async fn test_plm_resource_management() {
    let plm_server = MockPlmServer::new().await;
    let client = reqwest::Client::new();

    // Test system resources monitoring
    let resources_response = client
        .get(format!("{}/api/plm/resources", plm_server.base_url))
        .send()
        .await
        .unwrap();

    assert_eq!(resources_response.status(), 200);
    let resources: Value = resources_response.json().await.unwrap();
    assert_eq!(resources["status"], "success");
    assert!(resources["data"]["cpu_usage"].is_number());
    assert!(resources["data"]["memory_usage"].is_number());
    assert!(resources["data"]["disk_usage"].is_number());
    assert!(resources["data"]["build_slots"].is_object());

    // Test build artifacts management
    let artifacts_response = client
        .get(format!("{}/api/plm/artifacts", plm_server.base_url))
        .send()
        .await
        .unwrap();

    assert_eq!(artifacts_response.status(), 200);
    let artifacts: Value = artifacts_response.json().await.unwrap();
    assert_eq!(artifacts["status"], "success");
    assert!(artifacts["data"].is_array());

    // Test pipeline metrics
    let metrics_response = client
        .get(format!("{}/api/plm/metrics", plm_server.base_url))
        .send()
        .await
        .unwrap();

    assert_eq!(metrics_response.status(), 200);
    let metrics: Value = metrics_response.json().await.unwrap();
    assert_eq!(metrics["status"], "success");
    assert!(metrics["data"]["total_pipelines"].is_number());
    assert!(metrics["data"]["active_runs"].is_number());
    assert!(metrics["data"]["success_rate"].is_number());
    assert!(metrics["data"]["avg_build_time"].is_number());
}

/// Test PLM error scenarios and recovery
#[tokio::test]
async fn test_plm_error_scenarios() {
    let plm_server = MockPlmServer::new().await;
    let client = reqwest::Client::new();

    // Test build failure scenario
    let failure_response = client
        .post(format!("{}/api/plm/pipelines", plm_server.base_url))
        .json(&serde_json::json!({
            "name": "Failing Build Test",
            "type": "vxworks_kernel",
            "config": {
                "target_arch": "unsupported_arch", // This should trigger failure
                "build_type": "release"
            }
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(failure_response.status(), 201);
    let pipeline: Value = failure_response.json().await.unwrap();
    let pipeline_id = pipeline["data"]["id"].as_str().unwrap();

    // Run the failing pipeline
    let run_response = client
        .post(format!(
            "{}/api/plm/pipelines/{}/runs",
            plm_server.base_url, pipeline_id
        ))
        .json(&serde_json::json!({
            "trigger": "manual"
        }))
        .send()
        .await
        .unwrap();

    let run_result: Value = run_response.json().await.unwrap();
    let run_id = run_result["data"]["run_id"].as_str().unwrap();

    // Wait for failure to occur
    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

    // Check run status should show failure
    let status_response = client
        .get(format!("{}/api/plm/runs/{}", plm_server.base_url, run_id))
        .send()
        .await
        .unwrap();

    let run_status: Value = status_response.json().await.unwrap();

    // Should show failure in one of the tasks
    let tasks = run_status["data"]["tasks"].as_array().unwrap();
    let has_failure = tasks
        .iter()
        .any(|task| task["status"].as_str().unwrap_or("") == "Failed");
    assert!(has_failure, "Expected at least one task to fail");

    // Test resource exhaustion scenario
    let resource_limit_response = client
        .get(format!(
            "{}/api/plm/resources?scenario=resource_exhaustion",
            plm_server.base_url
        ))
        .send()
        .await
        .unwrap();

    assert_eq!(resource_limit_response.status(), 200);
    let exhausted_resources: Value = resource_limit_response.json().await.unwrap();

    // Should show high resource usage
    let cpu_usage = exhausted_resources["data"]["cpu_usage"].as_f64().unwrap();
    let memory_usage = exhausted_resources["data"]["memory_usage"]
        .as_f64()
        .unwrap();

    assert!(
        cpu_usage > 90.0,
        "CPU usage should be high in exhaustion scenario"
    );
    assert!(
        memory_usage > 90.0,
        "Memory usage should be high in exhaustion scenario"
    );
}

/// Test PLM integration with external services
#[tokio::test]
async fn test_plm_external_integrations() {
    let plm_server = MockPlmServer::new().await;
    let client = reqwest::Client::new();

    // Test VLAB integration
    let vlab_targets_response = client
        .get(format!("{}/api/plm/vlab/targets", plm_server.base_url))
        .send()
        .await
        .unwrap();

    assert_eq!(vlab_targets_response.status(), 200);
    let vlab_targets: Value = vlab_targets_response.json().await.unwrap();
    assert_eq!(vlab_targets["status"], "success");
    assert!(vlab_targets["data"].is_array());

    // Test SCM integration
    let scm_repos_response = client
        .get(format!("{}/api/plm/scm/repositories", plm_server.base_url))
        .send()
        .await
        .unwrap();

    assert_eq!(scm_repos_response.status(), 200);
    let scm_repos: Value = scm_repos_response.json().await.unwrap();
    assert_eq!(scm_repos["status"], "success");
    assert!(scm_repos["data"].is_array());

    // Test Jenkins integration
    let jenkins_jobs_response = client
        .get(format!("{}/api/plm/jenkins/jobs", plm_server.base_url))
        .send()
        .await
        .unwrap();

    assert_eq!(jenkins_jobs_response.status(), 200);
    let jenkins_jobs: Value = jenkins_jobs_response.json().await.unwrap();
    assert_eq!(jenkins_jobs["status"], "success");
    assert!(jenkins_jobs["data"].is_array());

    // Verify integration data structure
    let first_target = &vlab_targets["data"][0];
    assert!(first_target["name"].is_string());
    assert!(first_target["architecture"].is_string());
    assert!(first_target["status"].is_string());

    let first_repo = &scm_repos["data"][0];
    assert!(first_repo["name"].is_string());
    assert!(first_repo["url"].is_string());
    assert!(first_repo["default_branch"].is_string());

    let first_job = &jenkins_jobs["data"][0];
    assert!(first_job["name"].is_string());
    assert!(first_job["url"].is_string());
    assert!(first_job["last_build"].is_object());
}
