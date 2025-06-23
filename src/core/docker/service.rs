//! Docker service trait defining the interface for container operations

use std::collections::HashMap;
use std::path::Path;

use super::{ContainerSession, ContainerStatus, DockerConfig, DockerResult};

/// Trait defining the Docker service interface for para
///
/// This trait abstracts Docker operations to allow for different implementations
/// (e.g., Docker Engine API, Podman, mock implementations for testing)
pub trait DockerService: Send + Sync {
    /// Create a new Docker container session
    fn create_session(
        &self,
        session_name: &str,
        config: &DockerConfig,
        working_dir: &Path,
    ) -> DockerResult<ContainerSession>;

    /// Start an existing container session
    fn start_session(&self, session_name: &str) -> DockerResult<()>;

    /// Stop a running container session
    fn stop_session(&self, session_name: &str) -> DockerResult<()>;

    /// Finish a container session (stop and optionally remove)
    fn finish_session(&self, session_name: &str, remove: bool) -> DockerResult<()>;

    /// Cancel a container session (stop and remove)
    fn cancel_session(&self, session_name: &str) -> DockerResult<()>;

    /// Get the current status of a container
    fn get_container_status(&self, session_name: &str) -> DockerResult<ContainerStatus>;

    /// Execute a command inside a running container
    fn exec_in_container(
        &self,
        session_name: &str,
        command: &str,
        args: &[String],
        env: Option<HashMap<String, String>>,
    ) -> DockerResult<String>;

    /// Attach to a running container's shell
    fn attach_to_container(&self, session_name: &str) -> DockerResult<()>;

    /// List all para container sessions
    fn list_sessions(&self) -> DockerResult<Vec<ContainerSession>>;

    /// Check if Docker daemon is available and responding
    fn health_check(&self) -> DockerResult<()>;

    /// Pull a Docker image if not already available
    fn ensure_image(&self, image: &str) -> DockerResult<()>;

    /// Get container logs
    fn get_logs(
        &self,
        session_name: &str,
        follow: bool,
        tail: Option<usize>,
    ) -> DockerResult<String>;

    /// Copy files to a container
    fn copy_to_container(&self, session_name: &str, src: &Path, dest: &Path) -> DockerResult<()>;

    /// Copy files from a container
    fn copy_from_container(&self, session_name: &str, src: &Path, dest: &Path) -> DockerResult<()>;

    /// Update container resource limits
    fn update_resources(
        &self,
        session_name: &str,
        cpu_limit: Option<f64>,
        memory_limit: Option<u64>,
    ) -> DockerResult<()>;

    /// Get container resource usage statistics
    fn get_stats(&self, session_name: &str) -> DockerResult<ContainerStats>;

    /// Wait for a container to reach a specific status
    fn wait_for_status(
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

/// Mock Docker service for testing
#[derive(Debug)]
pub struct MockDockerService;

impl DockerService for MockDockerService {
    fn create_session(
        &self,
        session_name: &str,
        config: &DockerConfig,
        working_dir: &Path,
    ) -> DockerResult<ContainerSession> {
        let session = ContainerSession::new(
            format!("mock-{}", session_name),
            session_name.to_string(),
            config.default_image.clone(),
            working_dir.to_path_buf(),
        );
        Ok(session)
    }

    fn start_session(&self, _session_name: &str) -> DockerResult<()> {
        Ok(())
    }

    fn stop_session(&self, _session_name: &str) -> DockerResult<()> {
        Ok(())
    }

    fn finish_session(&self, _session_name: &str, _remove: bool) -> DockerResult<()> {
        Ok(())
    }

    fn cancel_session(&self, _session_name: &str) -> DockerResult<()> {
        Ok(())
    }

    fn get_container_status(&self, _session_name: &str) -> DockerResult<ContainerStatus> {
        Ok(ContainerStatus::Running)
    }

    fn exec_in_container(
        &self,
        _session_name: &str,
        command: &str,
        args: &[String],
        _env: Option<HashMap<String, String>>,
    ) -> DockerResult<String> {
        Ok(format!("Mock exec: {} {}", command, args.join(" ")))
    }

    fn attach_to_container(&self, _session_name: &str) -> DockerResult<()> {
        Ok(())
    }

    fn list_sessions(&self) -> DockerResult<Vec<ContainerSession>> {
        Ok(vec![])
    }

    fn health_check(&self) -> DockerResult<()> {
        Ok(())
    }

    fn ensure_image(&self, _image: &str) -> DockerResult<()> {
        Ok(())
    }

    fn get_logs(
        &self,
        _session_name: &str,
        _follow: bool,
        _tail: Option<usize>,
    ) -> DockerResult<String> {
        Ok("Mock logs".to_string())
    }

    fn copy_to_container(
        &self,
        _session_name: &str,
        _src: &Path,
        _dest: &Path,
    ) -> DockerResult<()> {
        Ok(())
    }

    fn copy_from_container(
        &self,
        _session_name: &str,
        _src: &Path,
        _dest: &Path,
    ) -> DockerResult<()> {
        Ok(())
    }

    fn update_resources(
        &self,
        _session_name: &str,
        _cpu_limit: Option<f64>,
        _memory_limit: Option<u64>,
    ) -> DockerResult<()> {
        Ok(())
    }

    fn get_stats(&self, _session_name: &str) -> DockerResult<ContainerStats> {
        Ok(ContainerStats {
            cpu_usage_percent: 0.0,
            memory_usage_bytes: 0,
            memory_limit_bytes: 0,
            network_rx_bytes: 0,
            network_tx_bytes: 0,
            block_read_bytes: 0,
            block_write_bytes: 0,
        })
    }

    fn wait_for_status(
        &self,
        _session_name: &str,
        _status: ContainerStatus,
        _timeout_seconds: u64,
    ) -> DockerResult<()> {
        Ok(())
    }
}
