//! Minimal Docker service implementation for MVP
//!
//! This provides just enough functionality to:
//! - Create a Docker container with mounted workspace
//! - Start/stop containers
//! - Extract code changes

use super::{ContainerSession, DockerError, DockerResult};
use crate::config::DockerConfig;
use std::path::Path;
use std::process::Command;

/// Minimal Docker service for MVP
pub struct DockerService;

impl DockerService {
    /// Create a new Docker container for a para session
    pub fn create_container(
        &self,
        session_name: &str,
        _config: &DockerConfig,
        working_dir: &Path,
        docker_args: &[String],
    ) -> DockerResult<ContainerSession> {
        let container_name = format!("para-{}", session_name);

        let mut docker_cmd_args = vec![
            "create".to_string(),
            "--name".to_string(),
            container_name.clone(),
        ];

        // Insert user-provided Docker args before the standard args
        docker_cmd_args.extend_from_slice(docker_args);

        // Add standard args
        docker_cmd_args.extend([
            "-v".to_string(),
            format!("{}:/workspace", working_dir.display()),
            "-w".to_string(),
            "/workspace".to_string(),
        ]);

        // Mount Para state directory for session tracking
        if let Ok(para_dir) = working_dir.join(".para").canonicalize() {
            docker_cmd_args.extend([
                "-v".to_string(),
                format!("{}:/workspace/.para:rw", para_dir.display()),
            ]);
        }

        // Set environment variables for container detection
        docker_cmd_args.extend([
            "-e".to_string(),
            "PARA_CONTAINER=1".to_string(),
            "-e".to_string(),
            format!("PARA_SESSION={}", session_name),
        ]);

        // Add image and command
        docker_cmd_args.extend([
            "para-authenticated:latest".to_string(),
            "sleep".to_string(),
            "infinity".to_string(),
        ]);

        println!(
            "🐋 Running docker command: docker {}",
            docker_cmd_args.join(" ")
        );
        let output = Command::new("docker")
            .args(&docker_cmd_args)
            .output()
            .map_err(|e| DockerError::DaemonNotAvailable(e.to_string()))?;

        if !output.status.success() {
            return Err(DockerError::ContainerCreationFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        let container_id = String::from_utf8_lossy(&output.stdout).trim().to_string();

        Ok(ContainerSession::new(
            container_id,
            session_name.to_string(),
            "para-authenticated:latest".to_string(),
            working_dir.to_path_buf(),
        ))
    }

    /// Start a container
    pub fn start_container(&self, session_name: &str) -> DockerResult<()> {
        let container_name = format!("para-{}", session_name);

        let output = Command::new("docker")
            .args(["start", &container_name])
            .output()
            .map_err(|e| DockerError::DaemonNotAvailable(e.to_string()))?;

        if !output.status.success() {
            return Err(DockerError::ContainerStartFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        Ok(())
    }

    /// Check if Docker is available
    pub fn health_check(&self) -> DockerResult<()> {
        let output = Command::new("docker")
            .args(["version"])
            .output()
            .map_err(|e| DockerError::DaemonNotAvailable(format!("Docker not found: {}", e)))?;

        if !output.status.success() {
            return Err(DockerError::DaemonNotAvailable(
                "Docker daemon not running".to_string(),
            ));
        }

        Ok(())
    }
}
