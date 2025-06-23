//! Container session types and state management

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Represents a Docker container session associated with a para session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerSession {
    /// Unique container ID assigned by Docker
    pub container_id: String,
    
    /// Para session name (matches the worktree name)
    pub session_name: String,
    
    /// Current status of the container
    pub status: ContainerStatus,
    
    /// Docker image used for this container
    pub image: String,
    
    /// Volume mount mappings (host_path -> container_path)
    pub volumes: Vec<VolumeMount>,
    
    /// Port mappings (host_port -> container_port)
    pub ports: Vec<PortMapping>,
    
    /// Environment variables set in the container
    pub environment: HashMap<String, String>,
    
    /// Working directory inside the container
    pub working_dir: PathBuf,
    
    /// Network mode (bridge, host, none, or custom network name)
    pub network_mode: String,
    
    /// Container hostname
    pub hostname: String,
    
    /// When the container was created
    pub created_at: DateTime<Utc>,
    
    /// When the container was last started
    pub started_at: Option<DateTime<Utc>>,
    
    /// When the container was stopped
    pub stopped_at: Option<DateTime<Utc>>,
    
    /// Resource limits applied to the container
    pub resource_limits: ResourceLimits,
    
    /// Custom labels for container metadata
    pub labels: HashMap<String, String>,
    
    /// Health check configuration
    pub health_check: Option<HealthCheckConfig>,
}

/// Container status enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ContainerStatus {
    /// Container has been created but not started
    Created,
    /// Container is currently running
    Running,
    /// Container is paused
    Paused,
    /// Container is restarting
    Restarting,
    /// Container has been stopped
    Stopped,
    /// Container is being removed
    Removing,
    /// Container has exited with an error
    Exited(i32),
    /// Container is dead (unrecoverable error)
    Dead,
    /// Status unknown or container not found
    Unknown,
}

/// Volume mount configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeMount {
    /// Path on the host system
    pub host_path: PathBuf,
    /// Path inside the container
    pub container_path: PathBuf,
    /// Whether the mount is read-only
    pub read_only: bool,
    /// Mount type (bind, volume, tmpfs)
    pub mount_type: MountType,
}

/// Mount type for volumes
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MountType {
    Bind,
    Volume,
    Tmpfs,
}

/// Port mapping configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortMapping {
    /// Port on the host system
    pub host_port: u16,
    /// Port inside the container
    pub container_port: u16,
    /// Protocol (tcp or udp)
    pub protocol: PortProtocol,
}

/// Network protocol for port mappings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PortProtocol {
    Tcp,
    Udp,
}

/// Resource limits for containers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    /// CPU limit (number of CPUs, can be fractional)
    pub cpu_limit: Option<f64>,
    /// Memory limit in bytes
    pub memory_limit: Option<u64>,
    /// Memory + swap limit in bytes
    pub memory_swap_limit: Option<u64>,
    /// CPU shares (relative weight)
    pub cpu_shares: Option<u64>,
    /// Disk I/O weight (10-1000)
    pub blkio_weight: Option<u16>,
    /// Maximum number of PIDs
    pub pids_limit: Option<i64>,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            cpu_limit: None,
            memory_limit: None,
            memory_swap_limit: None,
            cpu_shares: None,
            blkio_weight: None,
            pids_limit: None,
        }
    }
}

/// Health check configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckConfig {
    /// Command to run for health check
    pub test: Vec<String>,
    /// Time between health checks
    pub interval_seconds: u64,
    /// Time to wait for health check to complete
    pub timeout_seconds: u64,
    /// Number of consecutive failures before marking unhealthy
    pub retries: u32,
    /// Time to wait before starting health checks
    pub start_period_seconds: u64,
}

impl ContainerSession {
    /// Create a new container session
    pub fn new(
        container_id: String,
        session_name: String,
        image: String,
        working_dir: PathBuf,
    ) -> Self {
        Self {
            container_id,
            session_name: session_name.clone(),
            status: ContainerStatus::Created,
            image,
            volumes: Vec::new(),
            ports: Vec::new(),
            environment: HashMap::new(),
            working_dir,
            network_mode: "bridge".to_string(),
            hostname: session_name,
            created_at: Utc::now(),
            started_at: None,
            stopped_at: None,
            resource_limits: ResourceLimits::default(),
            labels: HashMap::new(),
            health_check: None,
        }
    }

    /// Check if the container is in a running state
    pub fn is_running(&self) -> bool {
        matches!(self.status, ContainerStatus::Running)
    }

    /// Check if the container can be started
    pub fn can_start(&self) -> bool {
        matches!(
            self.status,
            ContainerStatus::Created | ContainerStatus::Stopped | ContainerStatus::Exited(_)
        )
    }

    /// Check if the container exists and is not removed
    pub fn exists(&self) -> bool {
        !matches!(self.status, ContainerStatus::Unknown)
    }

    /// Get a unique container name for para sessions
    pub fn get_container_name(&self) -> String {
        format!("para-{}", self.session_name)
    }

    /// Add default para labels to identify containers
    pub fn add_para_labels(&mut self) {
        self.labels.insert("para.session".to_string(), self.session_name.clone());
        self.labels.insert("para.managed".to_string(), "true".to_string());
        self.labels.insert("para.created_at".to_string(), self.created_at.to_rfc3339());
    }
}