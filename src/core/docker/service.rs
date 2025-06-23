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
        auth_tokens: Option<&crate::core::docker::ClaudeAuthTokens>,
    ) -> DockerResult<ContainerSession> {
        let container_name = format!("para-{}", session_name);

        let mut docker_args = vec![
            "create".to_string(),
            "--name".to_string(),
            container_name.clone(),
            "-v".to_string(),
            format!("{}:/workspace", working_dir.display()),
            "-w".to_string(),
            "/workspace".to_string(),
        ];

        // Add authentication environment variables if provided
        if let Some(tokens) = auth_tokens {
            docker_args.extend([
                "-e".to_string(),
                format!("CLAUDE_CREDENTIALS_JSON={}", tokens.credentials_json),
            ]);
        }

        docker_args.extend([
            "para-claude:latest".to_string(),
            "sleep".to_string(),
            "infinity".to_string(),
        ]);

        println!(
            "🐋 Running docker command: docker {}",
            docker_args.join(" ")
        );
        let output = Command::new("docker")
            .args(&docker_args)
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
            "para-claude:latest".to_string(),
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
