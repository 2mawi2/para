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
        config: &DockerConfig,
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

        // Configure network isolation
        if config.network_isolation {
            // Add capabilities required for iptables/ipset
            docker_cmd_args.extend([
                "--cap-add".to_string(),
                "NET_ADMIN".to_string(),
                "--cap-add".to_string(),
                "NET_RAW".to_string(),
            ]);

            // Set network isolation environment variable
            docker_cmd_args.extend(["-e".to_string(), "PARA_NETWORK_ISOLATION=true".to_string()]);

            // Combine default and user-specified allowed domains
            let default_domains = vec![
                "api.anthropic.com".to_string(),
                "github.com".to_string(),
                "api.github.com".to_string(),
                "registry.npmjs.org".to_string(),
            ];

            let all_domains: Vec<String> = default_domains
                .into_iter()
                .chain(config.allowed_domains.iter().cloned())
                .collect();

            if !all_domains.is_empty() {
                docker_cmd_args.extend([
                    "-e".to_string(),
                    format!("PARA_ALLOWED_DOMAINS={}", all_domains.join(",")),
                ]);
            }
        } else {
            // Legacy behavior: use host network for backward compatibility
            docker_cmd_args.extend(["--network".to_string(), "host".to_string()]);

            // Disable network isolation
            docker_cmd_args.extend(["-e".to_string(), "PARA_NETWORK_ISOLATION=false".to_string()]);
        }

        // Add the image and command
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::DockerConfig;

    #[test]
    fn test_docker_service_creation() {
        let _service = DockerService;
    }

    #[test]
    fn test_network_isolation_enabled_config() {
        let config = DockerConfig {
            enabled: true,
            mount_workspace: true,
            network_isolation: true,
            allowed_domains: vec!["custom-api.com".to_string()],
        };

        // Test the config structure
        assert!(config.network_isolation);
        assert_eq!(config.allowed_domains.len(), 1);
        assert_eq!(config.allowed_domains[0], "custom-api.com");
    }

    #[test]
    fn test_network_isolation_disabled_config() {
        let config = DockerConfig {
            enabled: true,
            mount_workspace: true,
            network_isolation: false,
            allowed_domains: vec![],
        };

        // Test the config structure
        assert!(!config.network_isolation);
        assert!(config.allowed_domains.is_empty());
    }

    #[test]
    fn test_docker_config_serialization() {
        let config = DockerConfig {
            enabled: true,
            mount_workspace: true,
            network_isolation: true,
            allowed_domains: vec!["test.com".to_string(), "example.org".to_string()],
        };

        // Test that the config can be serialized and deserialized
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: DockerConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(config.enabled, deserialized.enabled);
        assert_eq!(config.mount_workspace, deserialized.mount_workspace);
        assert_eq!(config.network_isolation, deserialized.network_isolation);
        assert_eq!(config.allowed_domains, deserialized.allowed_domains);
    }
}
