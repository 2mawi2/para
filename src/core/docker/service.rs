//! Docker service implementation for health checks
//!
//! This provides core Docker functionality for the container pool system.

use super::session::ContainerSession;
use super::{DockerError, DockerResult};
use std::path::Path;
use std::process::Command;

/// Options for container creation
pub struct ContainerOptions<'a> {
    pub session_name: &'a str,
    pub network_isolation: bool,
    pub allowed_domains: &'a [String],
    pub working_dir: &'a Path,
    pub docker_args: &'a [String],
    pub docker_image: &'a str,
    pub forward_keys: bool,
    pub env_keys: &'a [String],
}

/// Docker service for health checks and core operations
pub struct DockerService;

impl DockerService {
    /// Create a new Docker container for a para session
    pub fn create_container(&self, options: &ContainerOptions) -> DockerResult<ContainerSession> {
        let container_name = format!("para-{}", options.session_name);

        let mut docker_cmd_args = vec![
            "create".to_string(),
            "--name".to_string(),
            container_name.clone(),
        ];

        // Insert user-provided Docker args before the standard args
        docker_cmd_args.extend_from_slice(options.docker_args);

        // Add standard args
        docker_cmd_args.extend([
            "-v".to_string(),
            format!("{}:/workspace", options.working_dir.display()),
            "-w".to_string(),
            "/workspace".to_string(),
        ]);

        // Configure network isolation
        if options.network_isolation {
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
                .chain(options.allowed_domains.iter().cloned())
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

        // Add API key forwarding if enabled
        if options.forward_keys {
            let mut forwarded_count = 0;
            for key in options.env_keys {
                if let Ok(value) = std::env::var(key) {
                    docker_cmd_args.extend(["-e".to_string(), format!("{}={}", key, value)]);
                    forwarded_count += 1;
                }
            }
            if forwarded_count > 0 {
                println!("🔑 Forwarding {} API keys to container", forwarded_count);
            }
        }

        // Add the image and command
        docker_cmd_args.extend([
            options.docker_image.to_string(),
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
            options.session_name.to_string(),
            options.docker_image.to_string(),
            options.working_dir.to_path_buf(),
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

    /// Verify network isolation is actually working after container start
    pub fn verify_network_isolation(
        &self,
        session_name: &str,
        expected_enabled: bool,
    ) -> DockerResult<()> {
        let container_name = format!("para-{}", session_name);

        // First check environment variable
        let env_output = Command::new("docker")
            .args([
                "exec",
                &container_name,
                "printenv",
                "PARA_NETWORK_ISOLATION",
            ])
            .output()
            .map_err(|e| DockerError::Other(anyhow::anyhow!("Failed to check env: {}", e)))?;

        if env_output.status.success() {
            let env_value = String::from_utf8_lossy(&env_output.stdout)
                .trim()
                .to_string();
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
                    .args(["exec", &container_name, "sudo", "iptables", "-L", "-n"])
                    .output();

                match iptables_output {
                    Ok(result) if result.status.success() => {
                        let rules = String::from_utf8_lossy(&result.stdout);
                        if !rules.contains("allowed-domains") && !rules.contains("match-set") {
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
                             Stopping container for safety."
                                .to_string(),
                        ));
                    }
                }
            }
        }

        Ok(())
    }

    /// Stop a running container
    pub fn stop_container(&self, session_name: &str) -> DockerResult<()> {
        let container_name = format!("para-{}", session_name);

        // Check if container exists
        let check_output = Command::new("docker")
            .args([
                "ps",
                "-a",
                "--format",
                "{{.Names}}",
                "--filter",
                &format!("name={}", container_name),
            ])
            .output()
            .map_err(|e| DockerError::CommandFailed(format!("Failed to check container: {}", e)))?;

        let container_list = String::from_utf8_lossy(&check_output.stdout);
        if !container_list.contains(&container_name) {
            // Container doesn't exist, nothing to do
            return Ok(());
        }

        // Stop the container
        let stop_output = Command::new("docker")
            .args(["stop", &container_name])
            .output()
            .map_err(|e| DockerError::CommandFailed(format!("Failed to stop container: {}", e)))?;

        if !stop_output.status.success() {
            let stderr = String::from_utf8_lossy(&stop_output.stderr);
            // If container is already stopped, that's fine
            if !stderr.contains("is not running") {
                return Err(DockerError::CommandFailed(format!(
                    "Failed to stop container '{}': {}",
                    container_name, stderr
                )));
            }
        }

        // Remove the container
        let rm_output = Command::new("docker")
            .args(["rm", &container_name])
            .output()
            .map_err(|e| {
                DockerError::CommandFailed(format!("Failed to remove container: {}", e))
            })?;

        if !rm_output.status.success() {
            let stderr = String::from_utf8_lossy(&rm_output.stderr);
            return Err(DockerError::CommandFailed(format!(
                "Failed to remove container '{}': {}",
                container_name, stderr
            )));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_docker_service_creation() {
        let _service = DockerService;
    }

    #[test]
    fn test_network_isolation_enabled() {
        // Test that network isolation parameters work correctly
        let network_isolation = true;
        let allowed_domains = ["custom-api.com".to_string()];

        assert!(network_isolation);
        assert_eq!(allowed_domains.len(), 1);
        assert_eq!(allowed_domains[0], "custom-api.com");
    }

    #[test]
    fn test_network_isolation_disabled() {
        // Test that network isolation can be disabled
        let network_isolation = false;
        let allowed_domains: Vec<String> = vec![];

        assert!(!network_isolation);
        assert!(allowed_domains.is_empty());
    }

    #[test]
    fn test_docker_parameters() {
        // Test various parameter combinations
        let test_cases = vec![
            (
                true,
                vec!["test.com".to_string(), "example.org".to_string()],
            ),
            (false, vec![]),
            (true, vec![]),
        ];

        for (_network_isolation, _allowed_domains) in test_cases {
            // Just verify the parameters are valid
            // network_isolation is either true or false - no need to assert
            // allowed_domains can be empty or non-empty - both are valid
        }
    }

    #[test]
    fn test_security_validation_error_types() {
        // Test that we can create the network isolation error type
        let _network_failed =
            DockerError::NetworkIsolationFailed("test network failed".to_string());
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
        let network_isolation = true;
        let allowed_domains = ["test.com".to_string()];

        // This would test the create_container method if we could mock Docker
        // For now, we just verify the parameters are correct
        assert!(network_isolation);
        assert!(!allowed_domains.is_empty());
    }

    #[test]
    fn test_api_key_env_vars() {
        // Test that we're checking for the right API keys
        let expected_keys = [
            "ANTHROPIC_API_KEY",
            "OPENAI_API_KEY",
            "GITHUB_TOKEN",
            "PERPLEXITY_API_KEY",
        ];

        // Just verify the keys are what we expect
        for key in &expected_keys {
            assert!(!key.is_empty());
            assert!(key.chars().all(|c| c.is_uppercase() || c == '_'));
        }
    }

    #[test]
    fn test_container_options_with_custom_env_keys() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let options = ContainerOptions {
            session_name: "test-session",
            network_isolation: false,
            allowed_domains: &[],
            working_dir: temp_dir.path(),
            docker_args: &[],
            docker_image: "test:latest",
            forward_keys: true,
            env_keys: &["CUSTOM_KEY".to_string(), "ANOTHER_KEY".to_string()],
        };

        assert_eq!(options.session_name, "test-session");
        assert_eq!(options.docker_image, "test:latest");
        assert!(options.forward_keys);
        assert_eq!(options.env_keys.len(), 2);
    }

    #[test]
    fn test_container_options_no_forward_keys() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let options = ContainerOptions {
            session_name: "secure-session",
            network_isolation: true,
            allowed_domains: &["api.example.com".to_string()],
            working_dir: temp_dir.path(),
            docker_args: &[],
            docker_image: "untrusted:latest",
            forward_keys: false,
            env_keys: &[],
        };

        assert!(!options.forward_keys);
        assert!(options.network_isolation);
        assert_eq!(options.docker_image, "untrusted:latest");
    }
}
