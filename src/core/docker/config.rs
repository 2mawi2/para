//! Docker configuration types and schema

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

use super::session::{MountType, ResourceLimits};

/// Docker configuration for para sessions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockerConfig {
    /// Whether Docker integration is enabled
    pub enabled: bool,

    /// Default Docker image to use if not specified
    pub default_image: String,

    /// Image selection based on detected project type
    pub image_mappings: HashMap<ProjectType, String>,

    /// Default volume mappings for all containers
    pub default_volumes: Vec<VolumeMapping>,

    /// Default environment variables
    pub default_environment: HashMap<String, String>,

    /// Default resource limits
    pub default_resource_limits: ResourceLimits,

    /// Network configuration
    pub network: NetworkConfig,

    /// Build configuration for custom images
    pub build: Option<BuildConfig>,

    /// Registry configuration for private images
    pub registry: Option<RegistryConfig>,

    /// Container lifecycle hooks
    pub hooks: LifecycleHooks,

    /// Development tool configurations
    pub dev_tools: DevToolsConfig,
}

/// Volume mapping configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeMapping {
    /// Source path (can use variables like $WORKTREE, $HOME, $PROJECT_ROOT)
    pub source: String,
    /// Target path in container
    pub target: String,
    /// Whether the mount is read-only
    pub read_only: bool,
    /// Mount type
    #[serde(default = "default_mount_type")]
    pub mount_type: MountType,
}

fn default_mount_type() -> MountType {
    MountType::Bind
}

/// Project type detection for automatic image selection
#[derive(Debug, Clone, Serialize, Deserialize, Hash, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ProjectType {
    Rust,
    Node,
    Python,
    Go,
    Java,
    Ruby,
    Php,
    Dotnet,
    Custom(String),
}

/// Network configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Network mode (bridge, host, none, or custom network name)
    pub mode: String,
    /// Whether to create a custom network for para sessions
    pub create_custom_network: bool,
    /// Custom network name
    pub custom_network_name: Option<String>,
    /// DNS servers
    pub dns: Vec<String>,
    /// DNS search domains
    pub dns_search: Vec<String>,
    /// Extra hosts entries
    pub extra_hosts: Vec<String>,
}

/// Build configuration for custom Docker images
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildConfig {
    /// Path to Dockerfile relative to project root
    pub dockerfile: String,
    /// Build context path
    pub context: String,
    /// Build arguments
    pub args: HashMap<String, String>,
    /// Target stage for multi-stage builds
    pub target: Option<String>,
    /// Cache from these images
    pub cache_from: Vec<String>,
    /// Whether to rebuild on each session start
    pub rebuild_on_start: bool,
}

/// Registry configuration for private images
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryConfig {
    /// Registry URL
    pub url: String,
    /// Username for authentication
    pub username: Option<String>,
    /// Password or token (should be loaded from environment)
    pub password_env: Option<String>,
    /// Whether to always pull latest
    pub always_pull: bool,
}

/// Container lifecycle hooks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LifecycleHooks {
    /// Commands to run after container creation
    pub post_create: Vec<String>,
    /// Commands to run after container start
    pub post_start: Vec<String>,
    /// Commands to run before container stop
    pub pre_stop: Vec<String>,
    /// Health check command
    pub health_check: Option<HealthCheckCommand>,
}

/// Health check command configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckCommand {
    pub command: Vec<String>,
    pub interval_seconds: u64,
    pub timeout_seconds: u64,
    pub retries: u32,
}

/// Development tools configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DevToolsConfig {
    /// Whether to install common development tools
    pub install_basics: bool,
    /// Additional packages to install
    pub additional_packages: Vec<String>,
    /// Git configuration
    pub git_config: GitConfig,
    /// Editor configuration
    pub editor_config: EditorConfig,
}

/// Git configuration for containers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitConfig {
    /// Whether to mount git config from host
    pub mount_from_host: bool,
    /// Git user name
    pub user_name: Option<String>,
    /// Git user email  
    pub user_email: Option<String>,
}

/// Editor configuration for containers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorConfig {
    /// Default editor command
    pub default: String,
    /// Whether to mount editor configs from host
    pub mount_configs: bool,
}

impl Default for DockerConfig {
    fn default() -> Self {
        let mut image_mappings = HashMap::new();
        image_mappings.insert(ProjectType::Rust, "rust:latest".to_string());
        image_mappings.insert(ProjectType::Node, "node:lts".to_string());
        image_mappings.insert(ProjectType::Python, "python:3.11".to_string());
        image_mappings.insert(ProjectType::Go, "golang:latest".to_string());
        image_mappings.insert(ProjectType::Java, "openjdk:17".to_string());
        image_mappings.insert(ProjectType::Ruby, "ruby:latest".to_string());
        image_mappings.insert(ProjectType::Php, "php:8-cli".to_string());
        image_mappings.insert(
            ProjectType::Dotnet,
            "mcr.microsoft.com/dotnet/sdk:7.0".to_string(),
        );

        let default_volumes = vec![
            VolumeMapping {
                source: "$WORKTREE".to_string(),
                target: "/workspace".to_string(),
                read_only: false,
                mount_type: MountType::Bind,
            },
            VolumeMapping {
                source: "$HOME/.ssh".to_string(),
                target: "/root/.ssh".to_string(),
                read_only: true,
                mount_type: MountType::Bind,
            },
        ];

        Self {
            enabled: false,
            default_image: "ubuntu:latest".to_string(),
            image_mappings,
            default_volumes,
            default_environment: HashMap::new(),
            default_resource_limits: ResourceLimits::default(),
            network: NetworkConfig {
                mode: "bridge".to_string(),
                create_custom_network: true,
                custom_network_name: Some("para-network".to_string()),
                dns: vec![],
                dns_search: vec![],
                extra_hosts: vec![],
            },
            build: None,
            registry: None,
            hooks: LifecycleHooks {
                post_create: vec![],
                post_start: vec![],
                pre_stop: vec![],
                health_check: None,
            },
            dev_tools: DevToolsConfig {
                install_basics: true,
                additional_packages: vec![],
                git_config: GitConfig {
                    mount_from_host: true,
                    user_name: None,
                    user_email: None,
                },
                editor_config: EditorConfig {
                    default: "vim".to_string(),
                    mount_configs: true,
                },
            },
        }
    }
}

/// Extension to the main para Config struct
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockerConfigExtension {
    pub docker: DockerConfig,
}

/// Helper to detect project type from files in the working directory
#[allow(dead_code)]
pub fn detect_project_type(project_path: &Path) -> ProjectType {
    if project_path.join("Cargo.toml").exists() {
        ProjectType::Rust
    } else if project_path.join("package.json").exists() {
        ProjectType::Node
    } else if project_path.join("requirements.txt").exists()
        || project_path.join("setup.py").exists()
        || project_path.join("pyproject.toml").exists()
    {
        ProjectType::Python
    } else if project_path.join("go.mod").exists() {
        ProjectType::Go
    } else if project_path.join("pom.xml").exists() || project_path.join("build.gradle").exists() {
        ProjectType::Java
    } else if project_path.join("Gemfile").exists() {
        ProjectType::Ruby
    } else if project_path.join("composer.json").exists() {
        ProjectType::Php
    } else if project_path.join("*.csproj").exists() || project_path.join("*.fsproj").exists() {
        ProjectType::Dotnet
    } else {
        ProjectType::Custom("unknown".to_string())
    }
}
