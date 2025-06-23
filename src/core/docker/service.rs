//! Docker service implementation for health checks
//!
//! This provides core Docker functionality for the container pool system.

use super::{DockerError, DockerResult};
use std::process::Command;

/// Docker service for health checks and core operations
pub struct DockerService;

impl DockerService {
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
