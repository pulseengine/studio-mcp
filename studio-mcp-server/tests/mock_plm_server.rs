//! Comprehensive PLM (Pipeline Management) mock server
//!
//! This module provides a sophisticated simulation of WindRiver Studio's Pipeline Management
//! system, including 20+ pipeline types, complete build lifecycle, realistic timing,
//! error scenarios, and resource management.

use chrono::{DateTime, Duration, Utc};
use serde_json::{Value, json};
use std::collections::HashMap;
use tokio::sync::RwLock;
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{method, path, path_regex, query_param},
};

/// Comprehensive PLM mock server
pub struct MockPlmServer {
    pub server: MockServer,
    pub base_url: String,
    /// Pipeline definitions and state
    pub pipelines: RwLock<HashMap<String, Pipeline>>,
    /// Active pipeline runs
    pub runs: RwLock<HashMap<String, PipelineRun>>,
    /// Build artifacts
    pub artifacts: RwLock<HashMap<String, BuildArtifact>>,
    /// System resources usage
    pub resources: RwLock<SystemResources>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Pipeline {
    pub id: String,
    pub name: String,
    pub pipeline_type: PipelineType,
    pub description: String,
    pub owner: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub status: PipelineStatus,
    pub tasks: Vec<PipelineTask>,
    pub parameters: HashMap<String, String>,
    pub success_rate: f64,
    pub avg_duration_seconds: u64,
    pub last_run_id: Option<String>,
    pub tags: Vec<String>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum PipelineType {
    // VxWorks Pipelines
    VxWorksKernel,
    VxWorksRtp,
    VxWorksBootloader,
    VxWorksDriver,
    VxWorksSmp,

    // Linux Pipelines
    LinuxKernel,
    LinuxUserspace,
    LinuxDriver,
    LinuxContainer,
    LinuxEmbedded,

    // Cross-compilation Pipelines
    CrossCompileArm,
    CrossCompileX86,
    CrossCompileMips,
    CrossCompilePowerPc,
    CrossCompileRiscV,

    // Application Pipelines
    CppApplication,
    CApplication,
    PythonApplication,
    JavaApplication,
    GoApplication,

    // Testing Pipelines
    UnitTest,
    IntegrationTest,
    PerformanceTest,
    RegressionTest,
    HardwareInLoop,

    // Deployment Pipelines
    ProductionDeploy,
    StagingDeploy,
    DevDeploy,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum PipelineStatus {
    Active,
    Inactive,
    Deprecated,
    UnderMaintenance,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct PipelineTask {
    pub name: String,
    pub task_type: TaskType,
    pub description: String,
    pub estimated_duration_seconds: u64,
    pub dependencies: Vec<String>,
    pub parallel_group: Option<String>,
    pub retry_count: u32,
    pub timeout_seconds: u64,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum TaskType {
    Checkout,
    Configure,
    Compile,
    Link,
    Test,
    Package,
    Deploy,
    Cleanup,
    Notification,
    Artifact,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct PipelineRun {
    pub id: String,
    pub pipeline_id: String,
    pub pipeline_name: String,
    pub run_number: u64,
    pub status: RunStatus,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub duration_seconds: Option<u64>,
    pub triggered_by: String,
    pub parameters: HashMap<String, String>,
    pub tasks: Vec<TaskRun>,
    pub artifacts_produced: Vec<String>,
    pub resource_usage: ResourceUsage,
    pub logs: Vec<LogEntry>,
    pub error_summary: Option<ErrorSummary>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum RunStatus {
    Queued,
    Running,
    Success,
    Failed,
    Cancelled,
    Timeout,
    Aborted,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct TaskRun {
    pub name: String,
    pub status: RunStatus,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub duration_seconds: Option<u64>,
    pub exit_code: Option<i32>,
    pub retry_attempt: u32,
    pub artifacts: Vec<String>,
    pub resource_usage: ResourceUsage,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ResourceUsage {
    pub cpu_usage_percent: f64,
    pub memory_usage_mb: u64,
    pub disk_usage_mb: u64,
    pub network_io_mb: u64,
    pub peak_memory_mb: u64,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct LogEntry {
    pub timestamp: DateTime<Utc>,
    pub level: LogLevel,
    pub task_name: Option<String>,
    pub message: String,
    pub raw_line: String,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum LogLevel {
    Debug,
    Info,
    Warning,
    Error,
    Fatal,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ErrorSummary {
    pub error_count: u32,
    pub warning_count: u32,
    pub failed_tasks: Vec<String>,
    pub primary_error: Option<String>,
    pub error_categories: HashMap<String, u32>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct BuildArtifact {
    pub id: String,
    pub pipeline_run_id: String,
    pub name: String,
    pub artifact_type: ArtifactType,
    pub path: String,
    pub size_bytes: u64,
    pub checksum: String,
    pub created_at: DateTime<Utc>,
    pub metadata: HashMap<String, String>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum ArtifactType {
    Binary,
    Library,
    Archive,
    Image,
    Configuration,
    Documentation,
    TestResults,
    Logs,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SystemResources {
    pub total_cpu_cores: u32,
    pub available_cpu_cores: u32,
    pub total_memory_gb: u64,
    pub available_memory_gb: u64,
    pub total_disk_gb: u64,
    pub available_disk_gb: u64,
    pub active_builds: u32,
    pub queued_builds: u32,
}

impl MockPlmServer {
    /// Create a new comprehensive PLM mock server
    pub async fn new() -> Self {
        let server = MockServer::start().await;
        let base_url = server.uri();

        let mock_server = Self {
            server,
            base_url,
            pipelines: RwLock::new(HashMap::new()),
            runs: RwLock::new(HashMap::new()),
            artifacts: RwLock::new(HashMap::new()),
            resources: RwLock::new(SystemResources::default()),
        };

        // Initialize with comprehensive pipeline data
        mock_server.initialize_pipeline_data().await;

        // Setup all PLM endpoints
        mock_server.setup_pipeline_endpoints().await;
        mock_server.setup_run_endpoints().await;
        mock_server.setup_task_endpoints().await;
        mock_server.setup_artifact_endpoints().await;
        mock_server.setup_monitoring_endpoints().await;
        mock_server.setup_integration_endpoints().await;

        mock_server
    }

    /// Initialize comprehensive pipeline data with 20+ pipeline types
    async fn initialize_pipeline_data(&self) {
        let mut pipelines = self.pipelines.write().await;
        let mut runs = self.runs.write().await;
        let mut artifacts = self.artifacts.write().await;

        // VxWorks Pipelines
        pipelines.insert(
            "vxworks-kernel-001".to_string(),
            Pipeline {
                id: "vxworks-kernel-001".to_string(),
                name: "VxWorks Kernel Build".to_string(),
                pipeline_type: PipelineType::VxWorksKernel,
                description: "Build VxWorks 7 kernel for ARM64 targets".to_string(),
                owner: "kernel-team@windriver.com".to_string(),
                created_at: Utc::now() - Duration::days(30),
                updated_at: Utc::now() - Duration::hours(2),
                status: PipelineStatus::Active,
                tasks: vec![
                    PipelineTask {
                        name: "checkout".to_string(),
                        task_type: TaskType::Checkout,
                        description: "Checkout VxWorks kernel source".to_string(),
                        estimated_duration_seconds: 120,
                        dependencies: vec![],
                        parallel_group: None,
                        retry_count: 3,
                        timeout_seconds: 300,
                    },
                    PipelineTask {
                        name: "configure".to_string(),
                        task_type: TaskType::Configure,
                        description: "Configure kernel build options".to_string(),
                        estimated_duration_seconds: 300,
                        dependencies: vec!["checkout".to_string()],
                        parallel_group: None,
                        retry_count: 2,
                        timeout_seconds: 600,
                    },
                    PipelineTask {
                        name: "compile".to_string(),
                        task_type: TaskType::Compile,
                        description: "Compile kernel modules".to_string(),
                        estimated_duration_seconds: 1800,
                        dependencies: vec!["configure".to_string()],
                        parallel_group: None,
                        retry_count: 1,
                        timeout_seconds: 3600,
                    },
                    PipelineTask {
                        name: "link".to_string(),
                        task_type: TaskType::Link,
                        description: "Link kernel image".to_string(),
                        estimated_duration_seconds: 180,
                        dependencies: vec!["compile".to_string()],
                        parallel_group: None,
                        retry_count: 1,
                        timeout_seconds: 300,
                    },
                    PipelineTask {
                        name: "test".to_string(),
                        task_type: TaskType::Test,
                        description: "Run kernel unit tests".to_string(),
                        estimated_duration_seconds: 600,
                        dependencies: vec!["link".to_string()],
                        parallel_group: Some("testing".to_string()),
                        retry_count: 2,
                        timeout_seconds: 900,
                    },
                    PipelineTask {
                        name: "package".to_string(),
                        task_type: TaskType::Package,
                        description: "Package kernel artifacts".to_string(),
                        estimated_duration_seconds: 120,
                        dependencies: vec!["test".to_string()],
                        parallel_group: None,
                        retry_count: 1,
                        timeout_seconds: 300,
                    },
                ],
                parameters: [
                    ("TARGET_ARCH".to_string(), "arm64".to_string()),
                    ("BUILD_TYPE".to_string(), "release".to_string()),
                    ("OPTIMIZATION".to_string(), "O2".to_string()),
                ]
                .iter()
                .cloned()
                .collect(),
                success_rate: 0.94,
                avg_duration_seconds: 3220,
                last_run_id: Some("run-vxk-001".to_string()),
                tags: vec![
                    "vxworks".to_string(),
                    "kernel".to_string(),
                    "arm64".to_string(),
                ],
            },
        );

        // Linux Embedded Pipeline
        pipelines.insert(
            "linux-embedded-001".to_string(),
            Pipeline {
                id: "linux-embedded-001".to_string(),
                name: "Linux Embedded System".to_string(),
                pipeline_type: PipelineType::LinuxEmbedded,
                description: "Build custom Linux for embedded ARM devices".to_string(),
                owner: "embedded-team@windriver.com".to_string(),
                created_at: Utc::now() - Duration::days(45),
                updated_at: Utc::now() - Duration::hours(6),
                status: PipelineStatus::Active,
                tasks: vec![
                    PipelineTask {
                        name: "yocto-setup".to_string(),
                        task_type: TaskType::Configure,
                        description: "Setup Yocto build environment".to_string(),
                        estimated_duration_seconds: 600,
                        dependencies: vec![],
                        parallel_group: None,
                        retry_count: 2,
                        timeout_seconds: 900,
                    },
                    PipelineTask {
                        name: "kernel-build".to_string(),
                        task_type: TaskType::Compile,
                        description: "Build Linux kernel".to_string(),
                        estimated_duration_seconds: 2400,
                        dependencies: vec!["yocto-setup".to_string()],
                        parallel_group: Some("build".to_string()),
                        retry_count: 1,
                        timeout_seconds: 3600,
                    },
                    PipelineTask {
                        name: "rootfs-build".to_string(),
                        task_type: TaskType::Compile,
                        description: "Build root filesystem".to_string(),
                        estimated_duration_seconds: 1800,
                        dependencies: vec!["yocto-setup".to_string()],
                        parallel_group: Some("build".to_string()),
                        retry_count: 1,
                        timeout_seconds: 2700,
                    },
                    PipelineTask {
                        name: "image-create".to_string(),
                        task_type: TaskType::Package,
                        description: "Create bootable image".to_string(),
                        estimated_duration_seconds: 300,
                        dependencies: vec!["kernel-build".to_string(), "rootfs-build".to_string()],
                        parallel_group: None,
                        retry_count: 1,
                        timeout_seconds: 600,
                    },
                ],
                parameters: [
                    ("MACHINE".to_string(), "raspberrypi4".to_string()),
                    ("DISTRO".to_string(), "poky".to_string()),
                    ("IMAGE_FEATURES".to_string(), "read-only-rootfs".to_string()),
                ]
                .iter()
                .cloned()
                .collect(),
                success_rate: 0.87,
                avg_duration_seconds: 5100,
                last_run_id: Some("run-linux-emb-001".to_string()),
                tags: vec![
                    "linux".to_string(),
                    "embedded".to_string(),
                    "yocto".to_string(),
                ],
            },
        );

        // Cross-compilation Pipeline
        pipelines.insert(
            "cross-compile-arm-001".to_string(),
            Pipeline {
                id: "cross-compile-arm-001".to_string(),
                name: "ARM Cross-Compilation".to_string(),
                pipeline_type: PipelineType::CrossCompileArm,
                description: "Cross-compile applications for ARM targets".to_string(),
                owner: "toolchain-team@windriver.com".to_string(),
                created_at: Utc::now() - Duration::days(20),
                updated_at: Utc::now() - Duration::hours(1),
                status: PipelineStatus::Active,
                tasks: vec![
                    PipelineTask {
                        name: "toolchain-setup".to_string(),
                        task_type: TaskType::Configure,
                        description: "Setup ARM cross-compilation toolchain".to_string(),
                        estimated_duration_seconds: 180,
                        dependencies: vec![],
                        parallel_group: None,
                        retry_count: 2,
                        timeout_seconds: 300,
                    },
                    PipelineTask {
                        name: "cross-compile".to_string(),
                        task_type: TaskType::Compile,
                        description: "Cross-compile for ARM target".to_string(),
                        estimated_duration_seconds: 900,
                        dependencies: vec!["toolchain-setup".to_string()],
                        parallel_group: None,
                        retry_count: 1,
                        timeout_seconds: 1800,
                    },
                    PipelineTask {
                        name: "strip-symbols".to_string(),
                        task_type: TaskType::Package,
                        description: "Strip debug symbols for release".to_string(),
                        estimated_duration_seconds: 60,
                        dependencies: vec!["cross-compile".to_string()],
                        parallel_group: None,
                        retry_count: 1,
                        timeout_seconds: 120,
                    },
                ],
                parameters: [
                    (
                        "TARGET_TRIPLE".to_string(),
                        "arm-linux-gnueabihf".to_string(),
                    ),
                    ("SYSROOT".to_string(), "/opt/arm-sysroot".to_string()),
                    ("STRIP_SYMBOLS".to_string(), "true".to_string()),
                ]
                .iter()
                .cloned()
                .collect(),
                success_rate: 0.91,
                avg_duration_seconds: 1140,
                last_run_id: Some("run-cross-arm-001".to_string()),
                tags: vec![
                    "cross-compile".to_string(),
                    "arm".to_string(),
                    "toolchain".to_string(),
                ],
            },
        );

        // Add sample pipeline run
        let now = Utc::now();
        runs.insert(
            "run-vxk-001".to_string(),
            PipelineRun {
                id: "run-vxk-001".to_string(),
                pipeline_id: "vxworks-kernel-001".to_string(),
                pipeline_name: "VxWorks Kernel Build".to_string(),
                run_number: 142,
                status: RunStatus::Running,
                started_at: now - Duration::minutes(15),
                completed_at: None,
                duration_seconds: None,
                triggered_by: "jenkins@windriver.com".to_string(),
                parameters: [
                    ("TARGET_ARCH".to_string(), "arm64".to_string()),
                    ("BUILD_TYPE".to_string(), "debug".to_string()),
                ]
                .iter()
                .cloned()
                .collect(),
                tasks: vec![
                    TaskRun {
                        name: "checkout".to_string(),
                        status: RunStatus::Success,
                        started_at: Some(now - Duration::minutes(15)),
                        completed_at: Some(now - Duration::minutes(13)),
                        duration_seconds: Some(120),
                        exit_code: Some(0),
                        retry_attempt: 0,
                        artifacts: vec!["source.tar.gz".to_string()],
                        resource_usage: ResourceUsage {
                            cpu_usage_percent: 25.0,
                            memory_usage_mb: 256,
                            disk_usage_mb: 1024,
                            network_io_mb: 512,
                            peak_memory_mb: 300,
                        },
                    },
                    TaskRun {
                        name: "configure".to_string(),
                        status: RunStatus::Success,
                        started_at: Some(now - Duration::minutes(13)),
                        completed_at: Some(now - Duration::minutes(8)),
                        duration_seconds: Some(300),
                        exit_code: Some(0),
                        retry_attempt: 0,
                        artifacts: vec!["config.mk".to_string(), "build.env".to_string()],
                        resource_usage: ResourceUsage {
                            cpu_usage_percent: 45.0,
                            memory_usage_mb: 512,
                            disk_usage_mb: 2048,
                            network_io_mb: 128,
                            peak_memory_mb: 600,
                        },
                    },
                    TaskRun {
                        name: "compile".to_string(),
                        status: RunStatus::Running,
                        started_at: Some(now - Duration::minutes(8)),
                        completed_at: None,
                        duration_seconds: None,
                        exit_code: None,
                        retry_attempt: 0,
                        artifacts: vec![],
                        resource_usage: ResourceUsage {
                            cpu_usage_percent: 85.0,
                            memory_usage_mb: 2048,
                            disk_usage_mb: 8192,
                            network_io_mb: 64,
                            peak_memory_mb: 2300,
                        },
                    },
                ],
                artifacts_produced: vec!["source.tar.gz".to_string(), "config.mk".to_string()],
                resource_usage: ResourceUsage {
                    cpu_usage_percent: 85.0,
                    memory_usage_mb: 2816,
                    disk_usage_mb: 11264,
                    network_io_mb: 704,
                    peak_memory_mb: 2300,
                },
                logs: vec![
                    LogEntry {
                        timestamp: now - Duration::minutes(15),
                        level: LogLevel::Info,
                        task_name: Some("checkout".to_string()),
                        message: "Starting source checkout from git repository".to_string(),
                        raw_line: "[INFO] checkout: Starting source checkout from git repository"
                            .to_string(),
                    },
                    LogEntry {
                        timestamp: now - Duration::minutes(8),
                        level: LogLevel::Info,
                        task_name: Some("compile".to_string()),
                        message: "Compiling kernel modules [progress: 45%]".to_string(),
                        raw_line: "[INFO] compile: Compiling kernel modules [progress: 45%]"
                            .to_string(),
                    },
                    LogEntry {
                        timestamp: now - Duration::minutes(5),
                        level: LogLevel::Warning,
                        task_name: Some("compile".to_string()),
                        message: "Deprecated API usage detected in network module".to_string(),
                        raw_line: "[WARN] compile: Deprecated API usage detected in network module"
                            .to_string(),
                    },
                ],
                error_summary: None,
            },
        );

        // Add sample build artifacts
        artifacts.insert(
            "artifact-001".to_string(),
            BuildArtifact {
                id: "artifact-001".to_string(),
                pipeline_run_id: "run-vxk-001".to_string(),
                name: "vxworks-kernel-arm64.bin".to_string(),
                artifact_type: ArtifactType::Binary,
                path: "/artifacts/vxworks/kernel/vxworks-kernel-arm64.bin".to_string(),
                size_bytes: 8388608, // 8MB
                checksum: "sha256:a1b2c3d4e5f6789012345678901234567890abcdef1234567890abcdef123456"
                    .to_string(),
                created_at: now - Duration::hours(2),
                metadata: [
                    ("target".to_string(), "arm64".to_string()),
                    ("build_type".to_string(), "release".to_string()),
                    ("compiler".to_string(), "gcc-11.2.0".to_string()),
                    ("optimization".to_string(), "O2".to_string()),
                ]
                .iter()
                .cloned()
                .collect(),
            },
        );

        // Initialize system resources
        let mut resources = self.resources.write().await;
        *resources = SystemResources {
            total_cpu_cores: 64,
            available_cpu_cores: 32,
            total_memory_gb: 256,
            available_memory_gb: 128,
            total_disk_gb: 10240,    // 10TB
            available_disk_gb: 5120, // 5TB
            active_builds: 8,
            queued_builds: 3,
        };
    }

    /// Setup pipeline management endpoints
    async fn setup_pipeline_endpoints(&self) {
        // List all pipelines with filtering and pagination
        Mock::given(method("GET"))
            .and(path("/api/plm/pipelines"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "data": [
                    {
                        "id": "vxworks-kernel-001",
                        "name": "VxWorks Kernel Build",
                        "type": "VxWorksKernel",
                        "description": "Build VxWorks 7 kernel for ARM64 targets",
                        "owner": "kernel-team@windriver.com",
                        "status": "Active",
                        "success_rate": 0.94,
                        "avg_duration_seconds": 3220,
                        "last_run_id": "run-vxk-001",
                        "tags": ["vxworks", "kernel", "arm64"],
                        "created_at": "2024-06-15T10:00:00Z",
                        "updated_at": "2024-07-24T22:00:00Z"
                    },
                    {
                        "id": "linux-embedded-001",
                        "name": "Linux Embedded System",
                        "type": "LinuxEmbedded",
                        "description": "Build custom Linux for embedded ARM devices",
                        "owner": "embedded-team@windriver.com",
                        "status": "Active",
                        "success_rate": 0.87,
                        "avg_duration_seconds": 5100,
                        "last_run_id": "run-linux-emb-001",
                        "tags": ["linux", "embedded", "yocto"],
                        "created_at": "2024-06-01T10:00:00Z",
                        "updated_at": "2024-07-24T18:00:00Z"
                    },
                    {
                        "id": "cross-compile-arm-001",
                        "name": "ARM Cross-Compilation",
                        "type": "CrossCompileArm",
                        "description": "Cross-compile applications for ARM targets",
                        "owner": "toolchain-team@windriver.com",
                        "status": "Active",
                        "success_rate": 0.91,
                        "avg_duration_seconds": 1140,
                        "last_run_id": "run-cross-arm-001",
                        "tags": ["cross-compile", "arm", "toolchain"],
                        "created_at": "2024-07-05T10:00:00Z",
                        "updated_at": "2024-07-24T23:00:00Z"
                    }
                ],
                "pagination": {
                    "total": 23,
                    "page": 1,
                    "per_page": 10,
                    "total_pages": 3
                },
                "status": "success"
            })))
            .mount(&self.server)
            .await;

        // Get specific pipeline details
        Mock::given(method("GET"))
            .and(path_regex(r"^/api/plm/pipelines/([^/]+)$"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "data": {
                    "id": "vxworks-kernel-001",
                    "name": "VxWorks Kernel Build",
                    "type": "VxWorksKernel",
                    "description": "Build VxWorks 7 kernel for ARM64 targets",
                    "owner": "kernel-team@windriver.com",
                    "status": "Active",
                    "tasks": [
                        {
                            "name": "checkout",
                            "type": "Checkout",
                            "description": "Checkout VxWorks kernel source",
                            "estimated_duration_seconds": 120,
                            "dependencies": [],
                            "retry_count": 3,
                            "timeout_seconds": 300
                        },
                        {
                            "name": "configure",
                            "type": "Configure",
                            "description": "Configure kernel build options",
                            "estimated_duration_seconds": 300,
                            "dependencies": ["checkout"],
                            "retry_count": 2,
                            "timeout_seconds": 600
                        },
                        {
                            "name": "compile",
                            "type": "Compile",
                            "description": "Compile kernel modules",
                            "estimated_duration_seconds": 1800,
                            "dependencies": ["configure"],
                            "retry_count": 1,
                            "timeout_seconds": 3600
                        }
                    ],
                    "parameters": {
                        "TARGET_ARCH": "arm64",
                        "BUILD_TYPE": "release",
                        "OPTIMIZATION": "O2"
                    },
                    "success_rate": 0.94,
                    "avg_duration_seconds": 3220,
                    "recent_runs": [
                        {
                            "id": "run-vxk-001",
                            "run_number": 142,
                            "status": "Running",
                            "started_at": "2024-07-25T00:45:00Z"
                        }
                    ]
                },
                "status": "success"
            })))
            .mount(&self.server)
            .await;

        // Start pipeline execution
        Mock::given(method("POST"))
            .and(path_regex(r"^/api/plm/pipelines/([^/]+)/start$"))
            .respond_with(ResponseTemplate::new(201).set_body_json(json!({
                "data": {
                    "run_id": "run-new-12345",
                    "pipeline_id": "vxworks-kernel-001",
                    "pipeline_name": "VxWorks Kernel Build",
                    "run_number": 143,
                    "status": "Queued",
                    "started_at": "2024-07-25T01:00:00Z",
                    "estimated_completion": "2024-07-25T01:53:40Z",
                    "queue_position": 2
                },
                "status": "success",
                "message": "Pipeline execution started successfully"
            })))
            .mount(&self.server)
            .await;

        // Get comprehensive pipeline types and templates (20+ types)
        Mock::given(method("GET"))
            .and(path("/api/plm/pipeline-types"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "data": [
                    {
                        "type": "VxWorksKernel",
                        "name": "VxWorks Kernel Build",
                        "description": "Build VxWorks kernel with modules",
                        "typical_duration_minutes": 45,
                        "complexity": "High",
                        "resource_requirements": {"cpu_cores": 8, "memory_gb": 16, "disk_gb": 50}
                    },
                    {
                        "type": "LinuxEmbedded",
                        "name": "Linux Embedded System",
                        "description": "Build custom Linux distribution",
                        "typical_duration_minutes": 85,
                        "complexity": "High",
                        "resource_requirements": {"cpu_cores": 12, "memory_gb": 32, "disk_gb": 100}
                    },
                    {
                        "type": "CrossCompileArm",
                        "name": "ARM Cross-Compilation",
                        "description": "Cross-compile for ARM targets",
                        "typical_duration_minutes": 19,
                        "complexity": "Medium",
                        "resource_requirements": {"cpu_cores": 4, "memory_gb": 8, "disk_gb": 20}
                    },
                    {
                        "type": "CrossCompileX86",
                        "name": "x86 Cross-Compilation",
                        "description": "Cross-compile for x86/x64 targets",
                        "typical_duration_minutes": 15,
                        "complexity": "Medium",
                        "resource_requirements": {"cpu_cores": 4, "memory_gb": 8, "disk_gb": 15}
                    },
                    {
                        "type": "CrossCompileMips",
                        "name": "MIPS Cross-Compilation",
                        "description": "Cross-compile for MIPS architecture",
                        "typical_duration_minutes": 22,
                        "complexity": "Medium",
                        "resource_requirements": {"cpu_cores": 4, "memory_gb": 8, "disk_gb": 18}
                    },
                    {
                        "type": "LinuxApplication",
                        "name": "Linux Application Build",
                        "description": "Build Linux applications and services",
                        "typical_duration_minutes": 12,
                        "complexity": "Low",
                        "resource_requirements": {"cpu_cores": 2, "memory_gb": 4, "disk_gb": 10}
                    },
                    {
                        "type": "VxWorksApplication",
                        "name": "VxWorks Application Build",
                        "description": "Build VxWorks RTP applications",
                        "typical_duration_minutes": 8,
                        "complexity": "Low",
                        "resource_requirements": {"cpu_cores": 2, "memory_gb": 4, "disk_gb": 8}
                    },
                    {
                        "type": "UnitTesting",
                        "name": "Unit Testing",
                        "description": "Run comprehensive unit test suites",
                        "typical_duration_minutes": 25,
                        "complexity": "Medium",
                        "resource_requirements": {"cpu_cores": 4, "memory_gb": 8, "disk_gb": 12}
                    },
                    {
                        "type": "IntegrationTesting",
                        "name": "Integration Testing",
                        "description": "Execute integration test scenarios",
                        "typical_duration_minutes": 65,
                        "complexity": "High",
                        "resource_requirements": {"cpu_cores": 8, "memory_gb": 16, "disk_gb": 25}
                    },
                    {
                        "type": "PerformanceTesting",
                        "name": "Performance Testing",
                        "description": "Benchmark and performance validation",
                        "typical_duration_minutes": 90,
                        "complexity": "High",
                        "resource_requirements": {"cpu_cores": 16, "memory_gb": 32, "disk_gb": 40}
                    },
                    {
                        "type": "SecurityScanning",
                        "name": "Security Scanning",
                        "description": "Static and dynamic security analysis",
                        "typical_duration_minutes": 35,
                        "complexity": "Medium",
                        "resource_requirements": {"cpu_cores": 4, "memory_gb": 8, "disk_gb": 20}
                    },
                    {
                        "type": "CodeQualityAnalysis",
                        "name": "Code Quality Analysis",
                        "description": "Code quality metrics and analysis",
                        "typical_duration_minutes": 18,
                        "complexity": "Low",
                        "resource_requirements": {"cpu_cores": 2, "memory_gb": 4, "disk_gb": 8}
                    },
                    {
                        "type": "Documentation",
                        "name": "Documentation Generation",
                        "description": "Generate API docs and user manuals",
                        "typical_duration_minutes": 12,
                        "complexity": "Low",
                        "resource_requirements": {"cpu_cores": 2, "memory_gb": 4, "disk_gb": 6}
                    },
                    {
                        "type": "ContainerBuild",
                        "name": "Container Build",
                        "description": "Build Docker/OCI containers",
                        "typical_duration_minutes": 20,
                        "complexity": "Medium",
                        "resource_requirements": {"cpu_cores": 4, "memory_gb": 8, "disk_gb": 30}
                    },
                    {
                        "type": "FirmwarePackaging",
                        "name": "Firmware Packaging",
                        "description": "Package firmware images and updates",
                        "typical_duration_minutes": 15,
                        "complexity": "Medium",
                        "resource_requirements": {"cpu_cores": 2, "memory_gb": 4, "disk_gb": 25}
                    },
                    {
                        "type": "BootloaderBuild",
                        "name": "Bootloader Build",
                        "description": "Build custom bootloaders",
                        "typical_duration_minutes": 28,
                        "complexity": "High",
                        "resource_requirements": {"cpu_cores": 4, "memory_gb": 8, "disk_gb": 15}
                    },
                    {
                        "type": "DeviceDriverBuild",
                        "name": "Device Driver Build",
                        "description": "Build hardware device drivers",
                        "typical_duration_minutes": 22,
                        "complexity": "High",
                        "resource_requirements": {"cpu_cores": 4, "memory_gb": 8, "disk_gb": 12}
                    },
                    {
                        "type": "BSPGeneration",
                        "name": "BSP Generation",
                        "description": "Generate Board Support Packages",
                        "typical_duration_minutes": 40,
                        "complexity": "High",
                        "resource_requirements": {"cpu_cores": 6, "memory_gb": 12, "disk_gb": 35}
                    },
                    {
                        "type": "ToolchainBuild",
                        "name": "Toolchain Build",
                        "description": "Build cross-compilation toolchains",
                        "typical_duration_minutes": 120,
                        "complexity": "High",
                        "resource_requirements": {"cpu_cores": 16, "memory_gb": 32, "disk_gb": 80}
                    },
                    {
                        "type": "ReleasePackaging",
                        "name": "Release Packaging",
                        "description": "Create release packages and distributions",
                        "typical_duration_minutes": 30,
                        "complexity": "Medium",
                        "resource_requirements": {"cpu_cores": 4, "memory_gb": 8, "disk_gb": 50}
                    },
                    {
                        "type": "ComplianceValidation",
                        "name": "Compliance Validation",
                        "description": "Validate regulatory and standards compliance",
                        "typical_duration_minutes": 45,
                        "complexity": "Medium",
                        "resource_requirements": {"cpu_cores": 4, "memory_gb": 8, "disk_gb": 20}
                    },
                    {
                        "type": "HardwareInTheLoop",
                        "name": "Hardware-in-the-Loop Testing",
                        "description": "Test with real hardware integration",
                        "typical_duration_minutes": 75,
                        "complexity": "High",
                        "resource_requirements": {"cpu_cores": 8, "memory_gb": 16, "disk_gb": 30}
                    },
                    {
                        "type": "CustomWorkflow",
                        "name": "Custom Workflow",
                        "description": "User-defined custom build workflows",
                        "typical_duration_minutes": 60,
                        "complexity": "Variable",
                        "resource_requirements": {"cpu_cores": 8, "memory_gb": 16, "disk_gb": 40}
                    }
                ],
                "total_types": 23,
                "status": "success"
            })))
            .mount(&self.server)
            .await;

        // Create pipeline run (new execution)
        Mock::given(method("POST"))
            .and(path_regex(r"^/api/plm/pipelines/([^/]+)/runs$"))
            .respond_with(ResponseTemplate::new(201).set_body_json(json!({
                "data": {
                    "run_id": "run-new-12345",
                    "pipeline_id": "vxworks-kernel-001",
                    "pipeline_name": "VxWorks Kernel Build",
                    "run_number": 143,
                    "status": "Queued",
                    "started_at": "2024-07-25T01:00:00Z",
                    "estimated_completion": "2024-07-25T01:53:40Z",
                    "queue_position": 2
                },
                "status": "success",
                "message": "Pipeline execution started successfully"
            })))
            .mount(&self.server)
            .await;

        // Create new pipeline
        Mock::given(method("POST"))
            .and(path("/api/plm/pipelines"))
            .respond_with(ResponseTemplate::new(201).set_body_json(json!({
                "data": {
                    "id": "pipeline-new-54321",
                    "name": "New Pipeline",
                    "type": "VxWorksKernel",
                    "status": "Created",
                    "created_at": "2024-07-25T01:00:00Z"
                },
                "status": "success",
                "message": "Pipeline created successfully"
            })))
            .mount(&self.server)
            .await;
    }

    /// Setup pipeline run management endpoints
    async fn setup_run_endpoints(&self) {
        // List pipeline runs
        Mock::given(method("GET"))
            .and(path("/api/plm/runs"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "data": [
                    {
                        "id": "run-vxk-001",
                        "pipeline_id": "vxworks-kernel-001",
                        "pipeline_name": "VxWorks Kernel Build",
                        "run_number": 142,
                        "status": "Running",
                        "started_at": "2024-07-25T00:45:00Z",
                        "duration_seconds": 900,
                        "triggered_by": "jenkins@windriver.com",
                        "progress_percent": 65,
                        "current_task": "compile",
                        "resource_usage": {
                            "cpu_usage_percent": 85.0,
                            "memory_usage_mb": 2816,
                            "peak_memory_mb": 2300
                        }
                    }
                ],
                "pagination": {
                    "total": 1,
                    "page": 1,
                    "per_page": 10
                },
                "status": "success"
            })))
            .mount(&self.server)
            .await;

        // Get specific run details (for failing runs - catch-all, must come first)
        Mock::given(method("GET"))
            .and(path_regex(r"^/api/plm/runs/([^/]+)$"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "data": {
                    "id": "run-vxk-001",
                    "pipeline_id": "vxworks-kernel-001",
                    "pipeline_name": "VxWorks Kernel Build",
                    "run_number": 142,
                    "status": "Running",
                    "started_at": "2024-07-25T00:45:00Z",
                    "duration_seconds": 900,
                    "triggered_by": "jenkins@windriver.com",
                    "parameters": {
                        "TARGET_ARCH": "arm64",
                        "BUILD_TYPE": "debug"
                    },
                    "tasks": [
                        {
                            "name": "checkout",
                            "status": "Success",
                            "started_at": "2024-07-25T00:45:00Z",
                            "completed_at": "2024-07-25T00:47:00Z",
                            "duration_seconds": 120,
                            "exit_code": 0,
                            "artifacts": ["source.tar.gz"]
                        },
                        {
                            "name": "configure",
                            "status": "Success",
                            "started_at": "2024-07-25T00:47:00Z",
                            "completed_at": "2024-07-25T00:52:00Z",
                            "duration_seconds": 300,
                            "exit_code": 0,
                            "artifacts": ["config.mk", "build.env"]
                        },
                        {
                            "name": "compile",
                            "status": "Failed",
                            "started_at": "2024-07-25T00:52:00Z",
                            "completed_at": "2024-07-25T00:55:00Z",
                            "duration_seconds": 180,
                            "exit_code": 2,
                            "error_details": {
                                "type": "compilation_error",
                                "message": "unsupported architecture: unsupported_arch"
                            }
                        }
                    ],
                    "resource_usage": {
                        "cpu_usage_percent": 85.0,
                        "memory_usage_mb": 2816,
                        "disk_usage_mb": 11264,
                        "network_io_mb": 704,
                        "peak_memory_mb": 2300
                    },
                    "artifacts_produced": ["source.tar.gz", "config.mk", "build.env"]
                },
                "status": "success"
            })))
            .mount(&self.server)
            .await;

        // Get run logs
        Mock::given(method("GET"))
            .and(path_regex(r"^/api/plm/runs/([^/]+)/logs$"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "data": {
                    "run_id": "run-vxk-001",
                    "total_lines": 1247,
                    "logs": [
                        {
                            "timestamp": "2024-07-25T00:45:00Z",
                            "level": "Info",
                            "task_name": "checkout",
                            "message": "Starting source checkout from git repository",
                            "raw_line": "[INFO] checkout: Starting source checkout from git repository"
                        },
                        {
                            "timestamp": "2024-07-25T00:52:00Z",
                            "level": "Info",
                            "task_name": "compile",
                            "message": "Compiling kernel modules [progress: 45%]",
                            "raw_line": "[INFO] compile: Compiling kernel modules [progress: 45%]"
                        },
                        {
                            "timestamp": "2024-07-25T00:55:00Z",
                            "level": "Warning",
                            "task_name": "compile",
                            "message": "Deprecated API usage detected in network module",
                            "raw_line": "[WARN] compile: Deprecated API usage detected in network module"
                        }
                    ]
                },
                "status": "success"
            })))
            .mount(&self.server)
            .await;

        // Cancel pipeline run
        Mock::given(method("POST"))
            .and(path_regex(r"^/api/plm/runs/([^/]+)/cancel$"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "data": {
                    "run_id": "run-vxk-001",
                    "status": "Cancelled",
                    "cancelled_at": "2024-07-25T01:00:00Z",
                    "cancelled_by": "user@windriver.com"
                },
                "status": "success",
                "message": "Pipeline run cancelled successfully"
            })))
            .mount(&self.server)
            .await;
    }

    /// Setup task-specific endpoints
    async fn setup_task_endpoints(&self) {
        // Get task libraries and definitions
        Mock::given(method("GET"))
            .and(path("/api/plm/tasks"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "data": [
                    {
                        "name": "vxworks-checkout",
                        "type": "Checkout",
                        "description": "Checkout VxWorks source from Git",
                        "category": "source-control",
                        "typical_duration_seconds": 120,
                        "resource_requirements": {
                            "cpu_usage_percent": 25,
                            "memory_mb": 256,
                            "disk_mb": 1024
                        },
                        "parameters": {
                            "repository_url": "https://git.windriver.com/vxworks/kernel.git",
                            "branch": "master",
                            "depth": 1
                        }
                    },
                    {
                        "name": "gcc-compile",
                        "type": "Compile",
                        "description": "Compile using GCC toolchain",
                        "category": "compilation",
                        "typical_duration_seconds": 1800,
                        "resource_requirements": {
                            "cpu_usage_percent": 85,
                            "memory_mb": 2048,
                            "disk_mb": 8192
                        },
                        "parameters": {
                            "optimization_level": "O2",
                            "debug_symbols": true,
                            "parallel_jobs": 8
                        }
                    }
                ],
                "status": "success"
            })))
            .mount(&self.server)
            .await;

        // Get task execution details
        Mock::given(method("GET"))
            .and(path_regex(r"^/api/plm/runs/([^/]+)/tasks/([^/]+)$"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "data": {
                    "run_id": "run-vxk-001",
                    "task_name": "compile",
                    "status": "Running",
                    "started_at": "2024-07-25T00:52:00Z",
                    "progress_percent": 45,
                    "estimated_completion": "2024-07-25T01:22:00Z",
                    "resource_usage": {
                        "cpu_usage_percent": 85.0,
                        "memory_usage_mb": 2048,
                        "disk_usage_mb": 8192,
                        "peak_memory_mb": 2300
                    },
                    "logs": [
                        {
                            "timestamp": "2024-07-25T00:52:00Z",
                            "level": "Info",
                            "message": "Starting compilation with 8 parallel jobs"
                        },
                        {
                            "timestamp": "2024-07-25T00:55:00Z",
                            "level": "Info",
                            "message": "Compiled 145/320 source files"
                        }
                    ],
                    "artifacts": [],
                    "error_details": null
                },
                "status": "success"
            })))
            .mount(&self.server)
            .await;
    }

    /// Setup artifact management endpoints
    async fn setup_artifact_endpoints(&self) {
        // List build artifacts
        Mock::given(method("GET"))
            .and(path("/api/plm/artifacts"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "data": [
                    {
                        "id": "artifact-001",
                        "pipeline_run_id": "run-vxk-001",
                        "name": "vxworks-kernel-arm64.bin",
                        "type": "Binary",
                        "path": "/artifacts/vxworks/kernel/vxworks-kernel-arm64.bin",
                        "size_bytes": 8388608,
                        "checksum": "sha256:a1b2c3d4e5f6789012345678901234567890abcdef1234567890abcdef123456",
                        "created_at": "2024-07-24T22:00:00Z",
                        "metadata": {
                            "target": "arm64",
                            "build_type": "release",
                            "compiler": "gcc-11.2.0",
                            "optimization": "O2"
                        }
                    }
                ],
                "pagination": {
                    "total": 1,
                    "page": 1,
                    "per_page": 10
                },
                "status": "success"
            })))
            .mount(&self.server)
            .await;

        // Get specific artifact details
        Mock::given(method("GET"))
            .and(path_regex(r"^/api/plm/artifacts/([^/]+)$"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "data": {
                    "id": "artifact-001",
                    "pipeline_run_id": "run-vxk-001",
                    "name": "vxworks-kernel-arm64.bin",
                    "type": "Binary",
                    "path": "/artifacts/vxworks/kernel/vxworks-kernel-arm64.bin",
                    "size_bytes": 8388608,
                    "checksum": "sha256:a1b2c3d4e5f6789012345678901234567890abcdef1234567890abcdef123456",
                    "created_at": "2024-07-24T22:00:00Z",
                    "download_url": "https://artifacts.windriver.com/download/artifact-001",
                    "metadata": {
                        "target": "arm64",
                        "build_type": "release",
                        "compiler": "gcc-11.2.0",
                        "optimization": "O2",
                        "debug_symbols": false,
                        "strip_level": "all"
                    },
                    "quality_metrics": {
                        "code_coverage": 0.85,
                        "static_analysis_score": 0.92,
                        "security_score": 0.98
                    }
                },
                "status": "success"
            })))
            .mount(&self.server)
            .await;
    }

    /// Setup monitoring and resource management endpoints
    async fn setup_monitoring_endpoints(&self) {
        // Resource exhaustion scenario (must be first to match before general endpoint)
        Mock::given(method("GET"))
            .and(path("/api/plm/resources"))
            .and(query_param("scenario", "resource_exhaustion"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "data": {
                    "cpu_usage": 96.8,
                    "memory_usage": 97.2,
                    "disk_usage": 91.5,
                    "build_slots": {
                        "total": 16,
                        "active": 16,
                        "available": 0
                    }
                },
                "status": "success"
            })))
            .mount(&self.server)
            .await;

        // Resource management endpoint (for test compatibility)
        Mock::given(method("GET"))
            .and(path("/api/plm/resources"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "data": {
                    "cpu_usage": 45.2,
                    "memory_usage": 62.8,
                    "disk_usage": 38.1,
                    "build_slots": {
                        "total": 16,
                        "active": 8,
                        "available": 8
                    }
                },
                "status": "success"
            })))
            .mount(&self.server)
            .await;

        // Artifacts endpoint
        Mock::given(method("GET"))
            .and(path("/api/plm/artifacts"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "data": [
                    {
                        "id": "artifact-001",
                        "name": "vxworks-kernel.bin",
                        "type": "kernel_image",
                        "size_bytes": 8388608,
                        "created_at": "2024-07-25T00:30:00Z",
                        "pipeline_id": "vxworks-kernel-001",
                        "run_id": "run-vxk-001"
                    },
                    {
                        "id": "artifact-002",
                        "name": "debug-symbols.tar.gz",
                        "type": "debug_info",
                        "size_bytes": 2097152,
                        "created_at": "2024-07-25T00:35:00Z",
                        "pipeline_id": "vxworks-kernel-001",
                        "run_id": "run-vxk-001"
                    }
                ],
                "status": "success"
            })))
            .mount(&self.server)
            .await;

        // PLM metrics endpoint
        Mock::given(method("GET"))
            .and(path("/api/plm/metrics"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "data": {
                    "total_pipelines": 23,
                    "active_runs": 8,
                    "success_rate": 0.91,
                    "avg_build_time": 1845
                },
                "status": "success"
            })))
            .mount(&self.server)
            .await;
        // System resource status
        Mock::given(method("GET"))
            .and(path("/api/plm/system/resources"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "data": {
                    "cpu": {
                        "total_cores": 64,
                        "available_cores": 32,
                        "usage_percent": 50.0,
                        "load_average": [2.1, 2.3, 2.0]
                    },
                    "memory": {
                        "total_gb": 256,
                        "available_gb": 128,
                        "usage_percent": 50.0,
                        "cached_gb": 64,
                        "buffers_gb": 16
                    },
                    "disk": {
                        "total_gb": 10240,
                        "available_gb": 5120,
                        "usage_percent": 50.0,
                        "io_read_mbps": 150.5,
                        "io_write_mbps": 89.2
                    },
                    "network": {
                        "interfaces": ["eth0", "eth1"],
                        "total_bandwidth_gbps": 20.0,
                        "current_usage_mbps": 234.7
                    },
                    "builds": {
                        "active_builds": 8,
                        "queued_builds": 3,
                        "max_concurrent_builds": 16,
                        "total_builds_today": 47
                    }
                },
                "status": "success"
            })))
            .mount(&self.server)
            .await;

        // Build queue status
        Mock::given(method("GET"))
            .and(path("/api/plm/queue"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "data": {
                    "queue_length": 3,
                    "estimated_wait_minutes": 12,
                    "queued_builds": [
                        {
                            "run_id": "run-queued-001",
                            "pipeline_name": "Linux Container Build",
                            "priority": "High",
                            "queued_at": "2024-07-25T00:58:00Z",
                            "estimated_start": "2024-07-25T01:05:00Z",
                            "resource_requirements": {
                                "cpu_cores": 4,
                                "memory_gb": 8,
                                "estimated_duration_minutes": 25
                            }
                        }
                    ]
                },
                "status": "success"
            })))
            .mount(&self.server)
            .await;

        // Performance metrics
        Mock::given(method("GET"))
            .and(path("/api/plm/metrics"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "data": {
                    "build_success_rate": {
                        "last_24h": 0.94,
                        "last_7d": 0.91,
                        "last_30d": 0.89
                    },
                    "average_build_times": {
                        "VxWorksKernel": 3220,
                        "LinuxEmbedded": 5100,
                        "CrossCompileArm": 1140
                    },
                    "resource_efficiency": {
                        "cpu_utilization": 0.76,
                        "memory_utilization": 0.68,
                        "disk_utilization": 0.45
                    },
                    "error_categories": {
                        "compilation_errors": 12,
                        "test_failures": 8,
                        "timeout_errors": 3,
                        "resource_errors": 2
                    },
                    "throughput": {
                        "builds_per_hour": 4.2,
                        "peak_builds_per_hour": 7.8,
                        "total_builds_today": 47
                    }
                },
                "status": "success"
            })))
            .mount(&self.server)
            .await;
    }

    /// Setup integration endpoints (VLAB, SCM, etc.)
    async fn setup_integration_endpoints(&self) {
        // VLAB targets integration (direct path for tests)
        Mock::given(method("GET"))
            .and(path("/api/plm/vlab/targets"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "data": [
                    {
                        "id": "vlab-target-001",
                        "name": "vxworks-sim-x86",
                        "architecture": "x86_64",
                        "target_type": "simulator",
                        "status": "available",
                        "capabilities": ["debug", "profiling", "network"]
                    },
                    {
                        "id": "vlab-target-002",
                        "name": "linux-qemu-arm",
                        "architecture": "aarch64",
                        "target_type": "emulator",
                        "status": "busy",
                        "capabilities": ["debug", "graphics"]
                    }
                ],
                "status": "success"
            })))
            .mount(&self.server)
            .await;

        // SCM repositories integration (direct path for tests)
        Mock::given(method("GET"))
            .and(path("/api/plm/scm/repositories"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "data": [
                    {
                        "id": "repo-001",
                        "name": "vxworks-kernel",
                        "url": "https://git.windriver.com/vxworks/kernel.git",
                        "default_branch": "main",
                        "type": "git",
                        "status": "active"
                    },
                    {
                        "id": "repo-002",
                        "name": "linux-yocto",
                        "url": "https://git.yoctoproject.org/linux-yocto",
                        "default_branch": "master",
                        "type": "git",
                        "status": "active"
                    }
                ],
                "status": "success"
            })))
            .mount(&self.server)
            .await;

        // Jenkins jobs integration (direct path for tests)
        Mock::given(method("GET"))
            .and(path("/api/plm/jenkins/jobs"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "data": [
                    {
                        "id": "jenkins-job-001",
                        "name": "VxWorks-Nightly-Build",
                        "url": "https://jenkins.windriver.com/job/VxWorks-Nightly-Build/",
                        "status": "enabled",
                        "last_build": {
                            "number": 142,
                            "status": "success",
                            "timestamp": "2024-07-25T02:00:00Z",
                            "duration_seconds": 3240
                        }
                    },
                    {
                        "id": "jenkins-job-002",
                        "name": "Linux-Embedded-CI",
                        "url": "https://jenkins.windriver.com/job/Linux-Embedded-CI/",
                        "status": "enabled",
                        "last_build": {
                            "number": 89,
                            "status": "running",
                            "timestamp": "2024-07-25T01:30:00Z"
                        }
                    }
                ],
                "status": "success"
            })))
            .mount(&self.server)
            .await;
        // VLAB integration - available targets
        Mock::given(method("GET"))
            .and(path("/api/plm/integrations/vlab/targets"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "data": [
                    {
                        "id": "vlab-arm64-001",
                        "name": "ARM64 Development Board",
                        "type": "physical",
                        "architecture": "aarch64",
                        "status": "available",
                        "capabilities": ["debug", "profiling", "deployment"],
                        "pipeline_compatibility": ["VxWorksKernel", "CrossCompileArm"],
                        "location": "Lab-A-Rack-3"
                    },
                    {
                        "id": "vlab-x86-sim-001",
                        "name": "x86_64 QEMU Simulator",
                        "type": "virtual",
                        "architecture": "x86_64",
                        "status": "busy",
                        "capabilities": ["debug", "automated-testing"],
                        "pipeline_compatibility": ["LinuxEmbedded", "UnitTest"],
                        "current_user": "jenkins@windriver.com"
                    }
                ],
                "status": "success"
            })))
            .mount(&self.server)
            .await;

        // SCM integration - repository status
        Mock::given(method("GET"))
            .and(path("/api/plm/integrations/scm/repositories"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "data": [
                    {
                        "name": "vxworks-kernel",
                        "url": "https://git.windriver.com/vxworks/kernel.git",
                        "branch": "master",
                        "last_commit": "a1b2c3d4",
                        "last_commit_time": "2024-07-24T20:15:00Z",
                        "author": "kernel-dev@windriver.com",
                        "status": "healthy",
                        "pipelines_using": ["vxworks-kernel-001"]
                    },
                    {
                        "name": "linux-distro",
                        "url": "https://git.windriver.com/linux/distro.git",
                        "branch": "main",
                        "last_commit": "e5f6g7h8",
                        "last_commit_time": "2024-07-24T18:30:00Z",
                        "author": "linux-team@windriver.com",
                        "status": "healthy",
                        "pipelines_using": ["linux-embedded-001"]
                    }
                ],
                "status": "success"
            })))
            .mount(&self.server)
            .await;

        // Jenkins integration status
        Mock::given(method("GET"))
            .and(path("/api/plm/integrations/jenkins/status"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "data": {
                    "status": "connected",
                    "version": "2.401.3",
                    "url": "https://jenkins.windriver.com",
                    "active_jobs": 8,
                    "queue_length": 3,
                    "last_sync": "2024-07-25T00:59:30Z",
                    "plugin_versions": {
                        "pipeline": "2.6",
                        "git": "4.8.3",
                        "build-timeout": "1.24"
                    }
                },
                "status": "success"
            })))
            .mount(&self.server)
            .await;
    }

    /// Get mock authentication token
    pub async fn get_mock_token(&self) -> String {
        "eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9.mock_plm_token".to_string()
    }

    /// Generate realistic error scenarios
    #[allow(dead_code)]
    pub async fn setup_error_scenarios(&self) {
        // Compilation error scenario
        Mock::given(method("GET"))
            .and(path("/api/plm/runs/run-error-compile"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "data": {
                    "id": "run-error-compile",
                    "pipeline_id": "vxworks-kernel-001",
                    "status": "Failed",
                    "error_summary": {
                        "error_count": 15,
                        "warning_count": 3,
                        "failed_tasks": ["compile"],
                        "primary_error": "undefined reference to `network_init'",
                        "error_categories": {
                            "linker_errors": 12,
                            "syntax_errors": 3
                        }
                    },
                    "tasks": [
                        {
                            "name": "compile",
                            "status": "Failed",
                            "exit_code": 2,
                            "error_details": {
                                "type": "compilation_error",
                                "file": "src/network/network_core.c",
                                "line": 247,
                                "column": 15,
                                "message": "undefined reference to `network_init'"
                            }
                        }
                    ]
                },
                "status": "success"
            })))
            .mount(&self.server)
            .await;

        // Resource exhaustion scenario
        Mock::given(method("GET"))
            .and(path("/api/plm/system/resources"))
            .and(query_param("scenario", "resource_exhaustion"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "data": {
                    "cpu": {
                        "total_cores": 64,
                        "available_cores": 2,
                        "usage_percent": 96.8,
                        "status": "critical"
                    },
                    "memory": {
                        "total_gb": 256,
                        "available_gb": 4,
                        "usage_percent": 98.4,
                        "status": "critical"
                    },
                    "builds": {
                        "active_builds": 16,
                        "queued_builds": 12,
                        "max_concurrent_builds": 16,
                        "status": "at_capacity"
                    }
                },
                "status": "warning",
                "message": "System resources are critically low"
            })))
            .mount(&self.server)
            .await;
    }
}

impl Default for SystemResources {
    fn default() -> Self {
        Self {
            total_cpu_cores: 64,
            available_cpu_cores: 32,
            total_memory_gb: 256,
            available_memory_gb: 128,
            total_disk_gb: 10240,
            available_disk_gb: 5120,
            active_builds: 8,
            queued_builds: 3,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::Client;

    #[tokio::test]
    async fn test_plm_pipeline_management() {
        let mock_server = MockPlmServer::new().await;
        let client = Client::new();
        let token = mock_server.get_mock_token().await;

        // Test pipeline listing
        let response = client
            .get(format!("{}/api/plm/pipelines", mock_server.base_url))
            .header("authorization", format!("Bearer {token}"))
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), 200);
        let pipelines: Value = response.json().await.unwrap();
        assert_eq!(pipelines["status"], "success");
        assert!(pipelines["data"].is_array());
        assert_eq!(pipelines["data"].as_array().unwrap().len(), 3);

        // Verify pipeline types are diverse
        let first_pipeline = &pipelines["data"][0];
        assert_eq!(first_pipeline["type"], "VxWorksKernel");
        assert!(first_pipeline["success_rate"].as_f64().unwrap() > 0.9);
    }

    #[tokio::test]
    async fn test_plm_build_execution() {
        let mock_server = MockPlmServer::new().await;
        let client = Client::new();
        let token = mock_server.get_mock_token().await;

        // Test pipeline start
        let response = client
            .post(format!(
                "{}/api/plm/pipelines/vxworks-kernel-001/start",
                mock_server.base_url
            ))
            .header("authorization", format!("Bearer {token}"))
            .json(&json!({
                "parameters": {
                    "TARGET_ARCH": "arm64",
                    "BUILD_TYPE": "debug"
                }
            }))
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), 201);
        let result: Value = response.json().await.unwrap();
        assert_eq!(result["status"], "success");
        assert!(result["data"]["run_id"].is_string());
        assert_eq!(result["data"]["status"], "Queued");
    }

    #[tokio::test]
    async fn test_plm_resource_monitoring() {
        let mock_server = MockPlmServer::new().await;
        let client = Client::new();
        let token = mock_server.get_mock_token().await;

        // Test system resources
        let response = client
            .get(format!("{}/api/plm/system/resources", mock_server.base_url))
            .header("authorization", format!("Bearer {token}"))
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), 200);
        let resources: Value = response.json().await.unwrap();
        assert_eq!(resources["status"], "success");
        assert!(resources["data"]["cpu"]["total_cores"].as_u64().unwrap() > 0);
        assert!(resources["data"]["memory"]["total_gb"].as_u64().unwrap() > 0);
        // active_builds is u64, so it's always >= 0 - just verify it exists
        assert!(
            resources["data"]["builds"]["active_builds"]
                .as_u64()
                .is_some()
        );
    }

    #[tokio::test]
    async fn test_plm_integration_endpoints() {
        let mock_server = MockPlmServer::new().await;
        let client = Client::new();
        let token = mock_server.get_mock_token().await;

        // Test VLAB integration
        let response = client
            .get(format!(
                "{}/api/plm/integrations/vlab/targets",
                mock_server.base_url
            ))
            .header("authorization", format!("Bearer {token}"))
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), 200);
        let targets: Value = response.json().await.unwrap();
        assert_eq!(targets["status"], "success");
        assert!(targets["data"].is_array());

        // Verify target diversity
        let targets_array = targets["data"].as_array().unwrap();
        assert!(targets_array.len() >= 2);
        assert!(targets_array.iter().any(|t| t["type"] == "physical"));
        assert!(targets_array.iter().any(|t| t["type"] == "virtual"));
    }
}
