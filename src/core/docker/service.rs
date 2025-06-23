//! Docker service trait defining the interface for container operations

use async_trait::async_trait;
use std::collections::HashMap;
use std::path::Path;

use super::{ContainerSession, ContainerStatus, DockerConfig, DockerResult};

/// Trait defining the Docker service interface for para
///
/// This trait abstracts Docker operations to allow for different implementations
/// (e.g., Docker Engine API, Podman, mock implementations for testing)
#[async_trait]
pub trait DockerService: Send + Sync {
    /// Create a new Docker container session
    ///
    /// # Arguments
    /// * `session_name` - Name of the para session
    /// * `config` - Docker configuration including image, volumes, etc.
    /// * `working_dir` - Working directory inside the container
    async fn create_session(
        &self,
        session_name: &str,
        config: &DockerConfig,
        working_dir: &Path,
    ) -> DockerResult<ContainerSession>;

    /// Start an existing container session
    async fn start_session(&self, session_name: &str) -> DockerResult<()>;

    /// Stop a running container session
    async fn stop_session(&self, session_name: &str) -> DockerResult<()>;

    /// Finish a container session (stop and optionally remove)
    ///
    /// # Arguments
    /// * `session_name` - Name of the para session
    /// * `remove` - Whether to remove the container after stopping
    async fn finish_session(&self, session_name: &str, remove: bool) -> DockerResult<()>;

    /// Cancel a container session (stop and remove)
    async fn cancel_session(&self, session_name: &str) -> DockerResult<()>;

    /// Get the current status of a container
    async fn get_container_status(&self, session_name: &str) -> DockerResult<ContainerStatus>;

    /// Execute a command inside a running container
    ///
    /// # Arguments
    /// * `session_name` - Name of the para session
    /// * `command` - Command to execute
    /// * `args` - Command arguments
    /// * `env` - Additional environment variables
    async fn exec_in_container(
        &self,
        session_name: &str,
        command: &str,
        args: &[String],
        env: Option<HashMap<String, String>>,
    ) -> DockerResult<String>;

    /// Attach to a running container's shell
    async fn attach_to_container(&self, session_name: &str) -> DockerResult<()>;

    /// List all para container sessions
    async fn list_sessions(&self) -> DockerResult<Vec<ContainerSession>>;

    /// Check if Docker daemon is available and responding
    async fn health_check(&self) -> DockerResult<()>;

    /// Pull a Docker image if not already available
    async fn ensure_image(&self, image: &str) -> DockerResult<()>;

    /// Get container logs
    ///
    /// # Arguments
    /// * `session_name` - Name of the para session
    /// * `follow` - Whether to follow log output
    /// * `tail` - Number of lines to show from the end
    async fn get_logs(
        &self,
        session_name: &str,
        follow: bool,
        tail: Option<usize>,
    ) -> DockerResult<String>;

    /// Copy files to a container
    ///
    /// # Arguments
    /// * `session_name` - Name of the para session
    /// * `src` - Source path on host
    /// * `dest` - Destination path in container
    async fn copy_to_container(
        &self,
        session_name: &str,
        src: &Path,
        dest: &Path,
    ) -> DockerResult<()>;

    /// Copy files from a container
    ///
    /// # Arguments
    /// * `session_name` - Name of the para session
    /// * `src` - Source path in container
    /// * `dest` - Destination path on host
    async fn copy_from_container(
        &self,
        session_name: &str,
        src: &Path,
        dest: &Path,
    ) -> DockerResult<()>;

    /// Update container resource limits
    async fn update_resources(
        &self,
        session_name: &str,
        cpu_limit: Option<f64>,
        memory_limit: Option<u64>,
    ) -> DockerResult<()>;

    /// Get container resource usage statistics
    async fn get_stats(&self, session_name: &str) -> DockerResult<ContainerStats>;

    /// Wait for a container to reach a specific status
    async fn wait_for_status(
        &self,
        session_name: &str,
        status: ContainerStatus,
        timeout_seconds: u64,
    ) -> DockerResult<()>;
}

/// Container resource usage statistics
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ContainerStats {
    pub cpu_usage_percent: f64,
    pub memory_usage_bytes: u64,
    pub memory_limit_bytes: u64,
    pub network_rx_bytes: u64,
    pub network_tx_bytes: u64,
    pub block_read_bytes: u64,
    pub block_write_bytes: u64,
}