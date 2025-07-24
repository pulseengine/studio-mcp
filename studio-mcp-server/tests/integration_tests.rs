use assert_cmd::prelude::*;
use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;
use std::process::Command as StdCommand;
use tempfile::NamedTempFile;
use tokio_test;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

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

/// Integration test with mocked PLM server
#[tokio::test]
async fn test_plm_integration_mock() {
    // Start a mock server
    let mock_server = MockServer::start().await;

    // Set up mock responses for PLM API
    Mock::given(method("GET"))
        .and(path("/api/health"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "status": "healthy"
        })))
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/api/auth/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "access_token": "test-token",
            "token_type": "Bearer"
        })))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/api/plm/pipelines"))
        .and(header("authorization", "Bearer test-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
            {
                "id": "pipeline-1",
                "name": "Test Pipeline",
                "status": "active"
            }
        ])))
        .mount(&mock_server)
        .await;

    // Create config file pointing to mock server
    let temp_config = NamedTempFile::new().unwrap();
    let config_content = serde_json::json!({
        "connections": {
            "test_server": {
                "name": "Test Server",
                "url": mock_server.uri(),
                "username": "admin",
                "token": null
            }
        },
        "default_connection": "test_server",
        "cli": {
            "download_base_url": "https://example.com",
            "version": "auto",
            "install_dir": null,
            "timeout": 300,
            "timeouts": {
                "quick_operations": 5,
                "medium_operations": 15,
                "long_operations": 30,
                "network_requests": 10
            },
            "auto_update": false,
            "update_check_interval": 24
        },
        "cache": {
            "enabled": true,
            "ttl": 300,
            "max_size": 1000
        },
        "logging": {
            "level": "info",
            "format": "pretty",
            "file_logging": false,
            "log_file": null
        }
    });

    std::fs::write(temp_config.path(), config_content.to_string()).unwrap();

    // Test that server can start with this config
    let mut server_process = StdCommand::new(env!("CARGO_BIN_EXE_studio-mcp-server"))
        .arg(temp_config.path().to_str().unwrap())
        .spawn()
        .expect("Failed to start server");

    // Give it a moment to start and potentially make requests
    tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;

    // Verify server is still running (successful config parsing and startup)
    assert!(
        server_process.try_wait().unwrap().is_none(),
        "Server should still be running with valid config"
    );

    // Clean up
    server_process.kill().expect("Failed to kill server");
    server_process.wait().expect("Failed to wait for server");
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
