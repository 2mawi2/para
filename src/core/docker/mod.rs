//! Docker integration module for para
//!
//! This module provides containerization support for para sessions,
//! allowing developers to work in isolated Docker environments.

#![allow(dead_code)] // TODO: Remove when Docker CLI integration is complete

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::process::Command;

pub mod auth;
pub mod config;
pub mod error;
pub mod extraction;
pub mod ide_integration;
pub mod manager;
pub mod service;
pub mod session;

#[cfg(test)]
pub mod mock;
#[cfg(test)]
mod tests;

// Re-export main types from submodules
pub use config::DockerConfig;
pub use error::DockerResult;
pub use ide_integration::DockerIdeIntegration;
pub use service::DockerService;
pub use session::{ContainerSession, ContainerStatus};

// Optional re-exports for when they're actually used
#[allow(unused_imports)]
pub use config::VolumeMapping;
#[allow(unused_imports)]
pub use error::DockerError;
#[allow(unused_imports)]
pub use session::ResourceLimits;

// TODO: Uncomment when integrating with session commands
// pub use auth::DockerAuthManager;

/// Trait defining the interface for Docker operations (from test infrastructure)
pub trait DockerServiceTrait: Send + Sync {
    /// Check if Docker is available on the system
    fn is_docker_available(&self) -> bool;

    /// Start a Docker container for a para session
    fn start_container(&self, session_name: &str, config: &DockerSessionConfig) -> Result<()>;

    /// Stop a Docker container
    fn stop_container(&self, session_name: &str) -> Result<()>;

    /// Remove a Docker container
    fn remove_container(&self, session_name: &str) -> Result<()>;

    /// Check if a container is running
    fn is_container_running(&self, session_name: &str) -> Result<bool>;

    /// Execute a command inside a running container
    fn exec_in_container(&self, session_name: &str, command: &[&str]) -> Result<String>;

    /// Get container logs
    fn get_container_logs(&self, session_name: &str, tail: Option<usize>) -> Result<String>;

    /// List all para Docker containers
    fn list_para_containers(&self) -> Result<Vec<String>>;
}

/// Configuration for a Docker-based para session (from test infrastructure)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockerSessionConfig {
    /// Docker image to use
    pub image: String,
    /// Volume mappings (host_path, container_path)
    pub volumes: Vec<(String, String)>,
    /// Environment variables
    pub env_vars: Vec<(String, String)>,
    /// Working directory inside the container
    pub workdir: Option<String>,
}

/// Simple Docker manager implementation for testing (from test infrastructure)
#[derive(Debug)]
pub struct DockerManager;

impl DockerManager {
    pub fn new() -> Self {
        Self
    }

    /// Check if Docker is available on the system
    pub fn is_docker_available(&self) -> bool {
        Command::new("docker")
            .arg("--version")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    /// Start a Docker container for a para session
    pub fn start_container(&self, session_name: &str, config: &DockerSessionConfig) -> Result<()> {
        let container_name = format!("para-{}", session_name);

        // Build docker run command
        let mut cmd = Command::new("docker");
        cmd.arg("run")
            .arg("-d") // Detached mode
            .arg("--name")
            .arg(&container_name);

        // Add volume mappings
        for (host_path, container_path) in &config.volumes {
            cmd.arg("-v")
                .arg(format!("{}:{}", host_path, container_path));
        }

        // Add environment variables
        for (key, value) in &config.env_vars {
            cmd.arg("-e").arg(format!("{}={}", key, value));
        }

        // Set working directory
        if let Some(workdir) = &config.workdir {
            cmd.arg("-w").arg(workdir);
        }

        // Add the image and command to keep container running
        cmd.arg(&config.image)
            .arg("tail")
            .arg("-f")
            .arg("/dev/null");

        let output = cmd.output().context("Failed to start Docker container")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Failed to start Docker container: {}", stderr);
        }

        Ok(())
    }

    /// Stop a Docker container
    pub fn stop_container(&self, session_name: &str) -> Result<()> {
        let container_name = format!("para-{}", session_name);

        let status = Command::new("docker")
            .args(["stop", &container_name])
            .status()
            .context("Failed to stop Docker container")?;

        if !status.success() {
            anyhow::bail!("Failed to stop Docker container {}", container_name);
        }

        Ok(())
    }

    /// Remove a Docker container
    pub fn remove_container(&self, session_name: &str) -> Result<()> {
        let container_name = format!("para-{}", session_name);

        let status = Command::new("docker")
            .args(["rm", "-f", &container_name])
            .status()
            .context("Failed to remove Docker container")?;

        if !status.success() {
            anyhow::bail!("Failed to remove Docker container {}", container_name);
        }

        Ok(())
    }

    /// Check if a container is running
    pub fn is_container_running(&self, session_name: &str) -> Result<bool> {
        let container_name = format!("para-{}", session_name);

        let output = Command::new("docker")
            .args(["inspect", "-f", "{{.State.Running}}", &container_name])
            .output()
            .context("Failed to inspect Docker container")?;

        if !output.status.success() {
            // Container doesn't exist
            return Ok(false);
        }

        let running = String::from_utf8_lossy(&output.stdout).trim() == "true";
        Ok(running)
    }

    /// Execute a command inside a running container
    pub fn exec_in_container(&self, session_name: &str, command: &[&str]) -> Result<String> {
        let container_name = format!("para-{}", session_name);

        let output = Command::new("docker")
            .arg("exec")
            .arg(&container_name)
            .args(command)
            .output()
            .context("Failed to execute command in Docker container")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Command failed in Docker container: {}", stderr);
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    /// Get container logs
    pub fn get_container_logs(&self, session_name: &str, tail: Option<usize>) -> Result<String> {
        let container_name = format!("para-{}", session_name);

        let mut cmd = Command::new("docker");
        cmd.arg("logs");

        if let Some(lines) = tail {
            cmd.arg("--tail").arg(lines.to_string());
        }

        cmd.arg(&container_name);

        let output = cmd
            .output()
            .context("Failed to get Docker container logs")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Failed to get Docker container logs: {}", stderr);
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    /// List all para Docker containers
    pub fn list_para_containers(&self) -> Result<Vec<String>> {
        let output = Command::new("docker")
            .args([
                "ps",
                "-a",
                "--filter",
                "name=para-",
                "--format",
                "{{.Names}}",
            ])
            .output()
            .context("Failed to list Docker containers")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Failed to list Docker containers: {}", stderr);
        }

        let containers = String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter(|line| !line.is_empty())
            .map(|s| s.to_string())
            .collect();

        Ok(containers)
    }
}

impl Default for DockerManager {
    fn default() -> Self {
        Self
    }
}

impl DockerServiceTrait for DockerManager {
    fn is_docker_available(&self) -> bool {
        self.is_docker_available()
    }

    fn start_container(&self, session_name: &str, config: &DockerSessionConfig) -> Result<()> {
        self.start_container(session_name, config)
    }

    fn stop_container(&self, session_name: &str) -> Result<()> {
        self.stop_container(session_name)
    }

    fn remove_container(&self, session_name: &str) -> Result<()> {
        self.remove_container(session_name)
    }

    fn is_container_running(&self, session_name: &str) -> Result<bool> {
        self.is_container_running(session_name)
    }

    fn exec_in_container(&self, session_name: &str, command: &[&str]) -> Result<String> {
        self.exec_in_container(session_name, command)
    }

    fn get_container_logs(&self, session_name: &str, tail: Option<usize>) -> Result<String> {
        self.get_container_logs(session_name, tail)
    }

    fn list_para_containers(&self) -> Result<Vec<String>> {
        self.list_para_containers()
    }
}

#[cfg(test)]
mod test_integration {
    use super::*;

    #[test]
    fn test_docker_session_config_serialization() {
        let config = DockerSessionConfig {
            image: "rust:latest".to_string(),
            volumes: vec![("/host/path".to_string(), "/container/path".to_string())],
            env_vars: vec![("KEY".to_string(), "value".to_string())],
            workdir: Some("/workspace".to_string()),
        };

        // Test serialization
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("rust:latest"));
        assert!(json.contains("/host/path"));
        assert!(json.contains("/container/path"));

        // Test deserialization
        let deserialized: DockerSessionConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.image, config.image);
        assert_eq!(deserialized.volumes.len(), 1);
        assert_eq!(deserialized.env_vars.len(), 1);
        assert_eq!(deserialized.workdir, Some("/workspace".to_string()));
    }

    #[test]
    fn test_docker_manager_creation() {
        let _manager = DockerManager::new();
        // Just verify it can be created
        let _default_manager: DockerManager = Default::default();
    }
}
