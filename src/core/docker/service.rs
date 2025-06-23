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
        // CRITICAL: Validate image security if network isolation is requested
        if config.network_isolation {
            self.validate_secure_image()?;
            
            // Also check for security labels as an additional safety measure
            if !self.check_image_security_labels()? {
                println!("⚠️  Warning: Docker image lacks security labels. Consider rebuilding with: para auth setup --force");
            }
        }

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

    /// Start a container with network isolation verification
    pub fn start_container_with_verification(
        &self,
        session_name: &str,
        expected_network_isolation: bool,
    ) -> DockerResult<()> {
        // First start the container
        self.start_container(session_name)?;

        // Give the container a moment to initialize
        std::thread::sleep(std::time::Duration::from_millis(500));

        // Verify network isolation state matches expectations
        if let Err(e) = self.verify_network_isolation(session_name, expected_network_isolation) {
            // Stop the container if verification fails
            let container_name = format!("para-{}", session_name);
            let _ = Command::new("docker")
                .args(["stop", &container_name])
                .output();
            
            return Err(e);
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

    /// Validate that the Docker image supports secure network isolation
    fn validate_secure_image(&self) -> DockerResult<()> {
        // Check if image exists and has the secure entrypoint
        let output = Command::new("docker")
            .args([
                "inspect",
                "para-authenticated:latest",
                "--format",
                "{{json .Config.Entrypoint}}",
            ])
            .output()
            .map_err(|e| DockerError::DaemonNotAvailable(e.to_string()))?;

        if !output.status.success() {
            return Err(DockerError::ImageNotFound(
                "para-authenticated:latest image not found. Please build with: para auth setup"
                    .to_string(),
            ));
        }

        let entrypoint = String::from_utf8_lossy(&output.stdout);
        if !entrypoint.contains("secure-entrypoint.sh") {
            return Err(DockerError::InsecureImage(
                "SECURITY ERROR: Docker image does not support network isolation.\n\
                 The image is using an old entrypoint that doesn't enforce security policies.\n\
                 Please rebuild the image with: para auth setup --force\n\
                 This is required for network isolation to work correctly."
                    .to_string(),
            ));
        }

        Ok(())
    }

    /// Check if the Docker image has the required security labels
    fn check_image_security_labels(&self) -> DockerResult<bool> {
        let output = Command::new("docker")
            .args([
                "inspect",
                "para-authenticated:latest",
                "--format",
                "{{json .Config.Labels}}",
            ])
            .output()
            .map_err(|e| DockerError::DaemonNotAvailable(e.to_string()))?;

        if !output.status.success() {
            return Ok(false); // Image doesn't exist
        }

        let labels = String::from_utf8_lossy(&output.stdout);
        // Check for security-related labels
        Ok(labels.contains("\"para.network-isolation\":\"true\"")
            && labels.contains("\"para.secure-entrypoint\":\"true\""))
    }

    /// Verify network isolation is actually working after container start
    pub fn verify_network_isolation(
        &self,
        session_name: &str,
        expected_enabled: bool,
    ) -> DockerResult<()> {
        let container_name = format!("para-{}", session_name);

        // First check environment variable
        let env_output = Command::new("docker")
            .args(["exec", &container_name, "printenv", "PARA_NETWORK_ISOLATION"])
            .output()
            .map_err(|e| DockerError::Other(anyhow::anyhow!("Failed to check env: {}", e)))?;

        if env_output.status.success() {
            let env_value = String::from_utf8_lossy(&env_output.stdout).trim().to_string();
            let actual_enabled = env_value == "true";

            if actual_enabled != expected_enabled {
                return Err(DockerError::NetworkIsolationFailed(format!(
                    "Network isolation mismatch: expected {}, but container has {}",
                    expected_enabled, actual_enabled
                )));
            }

            // If isolation should be enabled, verify iptables rules
            if expected_enabled {
                let iptables_output = Command::new("docker")
                    .args(["exec", &container_name, "iptables", "-L", "-n"])
                    .output();

                match iptables_output {
                    Ok(result) if result.status.success() => {
                        let rules = String::from_utf8_lossy(&result.stdout);
                        if !rules.contains("PARA_ALLOWED") && !rules.contains("ipset") {
                            return Err(DockerError::NetworkIsolationFailed(
                                "SECURITY ERROR: Network isolation enabled but firewall rules not applied.\n\
                                 This indicates the security setup failed during container initialization.\n\
                                 The container will be stopped for safety.".to_string()
                            ));
                        }
                    }
                    _ => {
                        return Err(DockerError::NetworkIsolationFailed(
                            "SECURITY ERROR: Cannot verify network isolation firewall rules.\n\
                             The container may not have proper security controls.\n\
                             Stopping container for safety.".to_string()
                        ));
                    }
                }
            }
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

    #[test]
    fn test_security_validation_error_types() {
        // Test that we can create the new error types
        let _image_not_found = DockerError::ImageNotFound("test image not found".to_string());
        let _insecure_image = DockerError::InsecureImage("test insecure image".to_string());
        let _network_failed = DockerError::NetworkIsolationFailed("test network failed".to_string());
    }

    #[test]
    fn test_container_name_format() {
        let _service = DockerService;
        // Test that container names are formatted correctly
        let test_cases = vec![
            ("my-session", "para-my-session"),
            ("test123", "para-test123"),
            ("feature_branch", "para-feature_branch"),
        ];

        for (session_name, expected_container) in test_cases {
            let container_name = format!("para-{}", session_name);
            assert_eq!(container_name, expected_container);
        }
    }

    #[test]
    fn test_docker_args_with_network_isolation() {
        let _service = DockerService;
        let config = DockerConfig {
            enabled: true,
            mount_workspace: true,
            network_isolation: true,
            allowed_domains: vec!["test.com".to_string()],
        };

        // This would test the create_container method if we could mock Docker
        // For now, we just verify the configuration is correct
        assert!(config.network_isolation);
        assert!(!config.allowed_domains.is_empty());
    }
}
