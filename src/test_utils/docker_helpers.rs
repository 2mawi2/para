use crate::core::docker::{DockerService, DockerSessionConfig};
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

/// Helper struct for Docker test utilities
pub struct DockerTestHelper;

#[allow(dead_code)]
impl DockerTestHelper {
    /// Check if Docker is available on the system
    pub fn is_docker_available() -> bool {
        Command::new("docker")
            .arg("--version")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    /// Pull a Docker image if not already present
    pub fn ensure_image(image: &str) -> Result<()> {
        println!("Ensuring Docker image {} is available...", image);

        let output = Command::new("docker")
            .args(["images", "-q", image])
            .output()
            .context("Failed to check for Docker image")?;

        if output.stdout.is_empty() {
            println!("Pulling Docker image {}...", image);
            let status = Command::new("docker")
                .args(["pull", image])
                .status()
                .context("Failed to pull Docker image")?;

            if !status.success() {
                anyhow::bail!("Failed to pull Docker image {}", image);
            }
        }

        Ok(())
    }

    /// Create a test container with a unique name
    pub fn create_test_container(base_name: &str, image: &str) -> Result<String> {
        let timestamp = chrono::Utc::now().timestamp_millis();
        let container_name = format!("para-test-{}-{}", base_name, timestamp);

        let status = Command::new("docker")
            .args([
                "create",
                "--name",
                &container_name,
                image,
                "sleep",
                "infinity", // Keep container running
            ])
            .status()
            .context("Failed to create test container")?;

        if !status.success() {
            anyhow::bail!("Failed to create test container");
        }

        Ok(container_name)
    }

    /// Start a container and wait for it to be ready
    pub fn start_container_and_wait(container_name: &str, timeout_secs: u64) -> Result<()> {
        // Start the container
        let status = Command::new("docker")
            .args(["start", container_name])
            .status()
            .context("Failed to start container")?;

        if !status.success() {
            anyhow::bail!("Failed to start container {}", container_name);
        }

        // Wait for container to be running
        let start_time = std::time::Instant::now();
        while start_time.elapsed().as_secs() < timeout_secs {
            let output = Command::new("docker")
                .args(["inspect", "-f", "{{.State.Running}}", container_name])
                .output()
                .context("Failed to inspect container")?;

            let running = String::from_utf8_lossy(&output.stdout).trim() == "true";
            if running {
                return Ok(());
            }

            thread::sleep(Duration::from_millis(100));
        }

        anyhow::bail!(
            "Container {} did not start within {} seconds",
            container_name,
            timeout_secs
        )
    }

    /// Execute a command in a running container
    pub fn exec_in_container(container_name: &str, command: &[&str]) -> Result<String> {
        let output = Command::new("docker")
            .arg("exec")
            .arg(container_name)
            .args(command)
            .output()
            .context("Failed to execute command in container")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Command failed in container: {}", stderr);
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    /// Copy files to a container
    pub fn copy_to_container(container_name: &str, src: &str, dest: &str) -> Result<()> {
        let status = Command::new("docker")
            .args(["cp", src, &format!("{}:{}", container_name, dest)])
            .status()
            .context("Failed to copy files to container")?;

        if !status.success() {
            anyhow::bail!("Failed to copy files to container");
        }

        Ok(())
    }

    /// Clean up a test container
    pub fn cleanup_container(container_name: &str) -> Result<()> {
        // Stop the container
        let _ = Command::new("docker")
            .args(["stop", container_name])
            .output();

        // Remove the container
        let status = Command::new("docker")
            .args(["rm", "-f", container_name])
            .status()
            .context("Failed to remove container")?;

        if !status.success() {
            anyhow::bail!("Failed to remove container {}", container_name);
        }

        Ok(())
    }

    /// Create a Docker volume for testing
    pub fn create_test_volume(volume_name: &str) -> Result<()> {
        let status = Command::new("docker")
            .args(["volume", "create", volume_name])
            .status()
            .context("Failed to create Docker volume")?;

        if !status.success() {
            anyhow::bail!("Failed to create Docker volume {}", volume_name);
        }

        Ok(())
    }

    /// Remove a Docker volume
    pub fn cleanup_volume(volume_name: &str) -> Result<()> {
        let status = Command::new("docker")
            .args(["volume", "rm", "-f", volume_name])
            .status()
            .context("Failed to remove Docker volume")?;

        if !status.success() {
            anyhow::bail!("Failed to remove Docker volume {}", volume_name);
        }

        Ok(())
    }

    /// Get container logs
    pub fn get_container_logs(container_name: &str) -> Result<String> {
        let output = Command::new("docker")
            .args(["logs", container_name])
            .output()
            .context("Failed to get container logs")?;

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }
}

/// Test guard that automatically cleans up Docker resources
#[derive(Default)]
pub struct DockerTestGuard {
    containers: Vec<String>,
    volumes: Vec<String>,
}

impl DockerTestGuard {
    pub fn add_container(&mut self, container_name: String) {
        self.containers.push(container_name);
    }

    pub fn add_volume(&mut self, volume_name: String) {
        self.volumes.push(volume_name);
    }
}

impl Drop for DockerTestGuard {
    fn drop(&mut self) {
        // Clean up containers
        for container in &self.containers {
            let _ = DockerTestHelper::cleanup_container(container);
        }

        // Clean up volumes
        for volume in &self.volumes {
            let _ = DockerTestHelper::cleanup_volume(volume);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_docker_availability_check() {
        // This test just verifies the function runs without panicking
        let _available = DockerTestHelper::is_docker_available();
    }

    #[test]
    fn test_docker_test_guard() {
        let mut guard = DockerTestGuard::default();
        guard.add_container("test-container-1".to_string());
        guard.add_container("test-container-2".to_string());
        guard.add_volume("test-volume-1".to_string());

        assert_eq!(guard.containers.len(), 2);
        assert_eq!(guard.volumes.len(), 1);

        // Guard will clean up when dropped
    }
}

/// Mock implementation of DockerService for testing
pub struct MockDockerService {
    containers: Arc<Mutex<HashMap<String, MockContainerState>>>,
    docker_available: bool,
}

#[derive(Clone, Debug)]
struct MockContainerState {
    running: bool,
    #[allow(dead_code)]
    config: DockerSessionConfig,
    logs: Vec<String>,
}

#[allow(dead_code)]
impl MockDockerService {
    pub fn new(docker_available: bool) -> Self {
        Self {
            containers: Arc::new(Mutex::new(HashMap::new())),
            docker_available,
        }
    }

    pub fn with_container(self, session_name: &str, running: bool) -> Self {
        let mut containers = self.containers.lock().unwrap();
        containers.insert(
            session_name.to_string(),
            MockContainerState {
                running,
                config: DockerSessionConfig {
                    image: "mock:latest".to_string(),
                    volumes: vec![],
                    env_vars: vec![],
                    workdir: None,
                },
                logs: vec![],
            },
        );
        drop(containers);
        self
    }

    pub fn add_log(&self, session_name: &str, log: &str) {
        let mut containers = self.containers.lock().unwrap();
        if let Some(state) = containers.get_mut(session_name) {
            state.logs.push(log.to_string());
        }
    }
}

impl DockerService for MockDockerService {
    fn is_docker_available(&self) -> bool {
        self.docker_available
    }

    fn start_container(&self, session_name: &str, config: &DockerSessionConfig) -> Result<()> {
        let mut containers = self.containers.lock().unwrap();
        if containers.contains_key(session_name) {
            anyhow::bail!("Container already exists");
        }

        containers.insert(
            session_name.to_string(),
            MockContainerState {
                running: true,
                config: config.clone(),
                logs: vec!["Container started".to_string()],
            },
        );
        Ok(())
    }

    fn stop_container(&self, session_name: &str) -> Result<()> {
        let mut containers = self.containers.lock().unwrap();
        match containers.get_mut(session_name) {
            Some(state) => {
                state.running = false;
                state.logs.push("Container stopped".to_string());
                Ok(())
            }
            None => anyhow::bail!("Container not found"),
        }
    }

    fn remove_container(&self, session_name: &str) -> Result<()> {
        let mut containers = self.containers.lock().unwrap();
        containers
            .remove(session_name)
            .ok_or_else(|| anyhow::anyhow!("Container not found"))?;
        Ok(())
    }

    fn is_container_running(&self, session_name: &str) -> Result<bool> {
        let containers = self.containers.lock().unwrap();
        match containers.get(session_name) {
            Some(state) => Ok(state.running),
            None => Ok(false),
        }
    }

    fn exec_in_container(&self, session_name: &str, command: &[&str]) -> Result<String> {
        let containers = self.containers.lock().unwrap();
        match containers.get(session_name) {
            Some(state) if state.running => Ok(format!("Executed: {}", command.join(" "))),
            Some(_) => anyhow::bail!("Container is not running"),
            None => anyhow::bail!("Container not found"),
        }
    }

    fn get_container_logs(&self, session_name: &str, tail: Option<usize>) -> Result<String> {
        let containers = self.containers.lock().unwrap();
        match containers.get(session_name) {
            Some(state) => {
                let logs = if let Some(n) = tail {
                    state
                        .logs
                        .iter()
                        .rev()
                        .take(n)
                        .rev()
                        .cloned()
                        .collect::<Vec<_>>()
                } else {
                    state.logs.clone()
                };
                Ok(logs.join("\n"))
            }
            None => anyhow::bail!("Container not found"),
        }
    }

    fn list_para_containers(&self) -> Result<Vec<String>> {
        let containers = self.containers.lock().unwrap();
        Ok(containers
            .keys()
            .map(|name| format!("para-{}", name))
            .collect())
    }
}

/// Helper functions for creating test configurations and environments
#[allow(dead_code)]
pub fn create_test_docker_config() -> DockerSessionConfig {
    DockerSessionConfig {
        image: "alpine:latest".to_string(),
        volumes: vec![("/test/host".to_string(), "/test/container".to_string())],
        env_vars: vec![("TEST_ENV".to_string(), "test_value".to_string())],
        workdir: Some("/workspace".to_string()),
    }
}

#[allow(dead_code)]
pub fn mock_docker_service() -> MockDockerService {
    MockDockerService::new(true)
}

#[allow(dead_code)]
pub fn setup_test_container_environment() -> Result<(DockerTestGuard, String)> {
    let mut guard = DockerTestGuard::default();
    let container_name = format!("para-test-env-{}", chrono::Utc::now().timestamp_millis());
    guard.add_container(container_name.clone());
    Ok((guard, container_name))
}

#[allow(dead_code)]
pub fn verify_container_cleanup(session_id: &str) -> Result<bool> {
    let container_name = format!("para-{}", session_id);
    let output = Command::new("docker")
        .args([
            "ps",
            "-a",
            "--filter",
            &format!("name={}", container_name),
            "--format",
            "{{.Names}}",
        ])
        .output()
        .context("Failed to check for container")?;

    let exists = !String::from_utf8_lossy(&output.stdout).trim().is_empty();
    Ok(!exists) // Return true if container does NOT exist (was cleaned up)
}
