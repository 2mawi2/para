//! Docker container pool implementation
//!
//! Manages container lifecycle with resource limits to prevent system overload.
//! Each session gets its own isolated container (no sharing for security).

use super::{DockerError, DockerResult};
use std::process::Command;

/// Pool that manages Docker container lifecycle with resource limits
///
/// This pool enforces limits on concurrent containers by querying
/// Docker directly for the current state. Docker is the single source
/// of truth for container existence.
pub struct ContainerPool {
    max_size: usize,
}

impl ContainerPool {
    /// Create a new container pool with the specified maximum size
    pub fn new(max_size: usize) -> Self {
        Self { max_size }
    }

    /// Get ALL para containers from Docker (including stopped/exited)
    fn get_all_para_containers(&self) -> DockerResult<Vec<String>> {
        // Query Docker for ALL containers with names starting with "para-"
        let output = Command::new("docker")
            .args([
                "ps",
                "-a", // -a flag includes stopped containers
                "--format",
                "{{.ID}}",
                "--filter",
                "name=^para-",
            ])
            .output()
            .map_err(|e| {
                DockerError::Other(anyhow::anyhow!("Failed to query Docker containers: {}", e))
            })?;

        if !output.status.success() {
            return Err(DockerError::Other(anyhow::anyhow!(
                "Failed to list containers: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        let container_ids: Vec<String> = String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter(|line| !line.is_empty())
            .map(|line| line.to_string())
            .collect();

        Ok(container_ids)
    }

    /// Get list of active (running) para containers from Docker
    fn get_active_containers(&self) -> DockerResult<Vec<String>> {
        // Query Docker for all containers with names starting with "para-"
        let output = Command::new("docker")
            .args([
                "ps",
                "--format",
                "{{.ID}}",
                "--filter",
                "name=^para-",
                "--filter",
                "status=running",
            ])
            .output()
            .map_err(|e| {
                DockerError::Other(anyhow::anyhow!("Failed to query Docker containers: {}", e))
            })?;

        if !output.status.success() {
            return Err(DockerError::Other(anyhow::anyhow!(
                "Failed to list containers: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        let container_ids: Vec<String> = String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter(|line| !line.is_empty())
            .map(|s| s.to_string())
            .collect();

        Ok(container_ids)
    }

    /// Get names of active para sessions
    #[allow(dead_code)]
    fn get_active_session_names(&self) -> DockerResult<Vec<String>> {
        let output = Command::new("docker")
            .args([
                "ps",
                "--format",
                "{{.Names}}",
                "--filter",
                "name=^para-",
                "--filter",
                "status=running",
            ])
            .output()
            .map_err(|e| {
                DockerError::Other(anyhow::anyhow!("Failed to query session names: {}", e))
            })?;

        if output.status.success() {
            let names: Vec<String> = String::from_utf8_lossy(&output.stdout)
                .lines()
                .filter(|line| !line.is_empty())
                .map(|name| name.strip_prefix("para-").unwrap_or(name).to_string())
                .collect();
            Ok(names)
        } else {
            Ok(vec![])
        }
    }

    /// Check if pool has capacity for a new container
    pub fn check_capacity(&self) -> DockerResult<()> {
        // Query Docker for actual container count (including non-running)
        let active_containers = self.get_all_para_containers()?;
        let active_count = active_containers.len();

        println!(
            "🔍 Container pool status: {}/{} para containers in Docker",
            active_count, self.max_size
        );

        if active_count >= self.max_size {
            return Err(DockerError::Other(anyhow::anyhow!(
                "Docker container pool exhausted ({}/{}). \
                Finish some sessions with 'para finish' or increase max_containers.",
                active_count,
                self.max_size
            )));
        }
        Ok(())
    }

    /// Add a container to pool tracking
    pub fn add_container(&self, session_name: &str, container_id: &str) -> DockerResult<()> {
        // Verify the container actually exists in Docker
        let output = Command::new("docker")
            .args(["inspect", "--format", "{{.State.Running}}", container_id])
            .output()
            .map_err(|e| {
                DockerError::Other(anyhow::anyhow!("Failed to verify container: {}", e))
            })?;

        if !output.status.success() {
            return Err(DockerError::Other(anyhow::anyhow!(
                "Container {} was not created successfully",
                container_id
            )));
        }

        // Log current pool state for visibility
        let active_count = self.get_active_containers()?.len();
        println!(
            "✅ Container pool updated for session '{}': {}/{} active containers",
            session_name, active_count, self.max_size
        );

        Ok(())
    }

    /// Destroy a session's container
    pub fn destroy_session_container(&self, container_id: &str) -> DockerResult<()> {
        self.validate_container_id_for_destruction(container_id)?;
        self.verify_para_container(container_id)?;

        let _stop_output = Command::new("docker")
            .args(["stop", "--time", "10", container_id])
            .output(); // Ignore errors - container might already be stopped

        let remove_output = Command::new("docker")
            .args(["rm", "--force", container_id]) // Force removal to handle edge cases
            .output()
            .map_err(|e| {
                DockerError::Other(anyhow::anyhow!("Failed to remove container: {}", e))
            })?;

        if !remove_output.status.success() {
            let error_msg = String::from_utf8_lossy(&remove_output.stderr);

            return Err(DockerError::Other(anyhow::anyhow!(
                "Failed to remove container {}: {}",
                container_id,
                error_msg
            )));
        }

        println!("🗑️  Safely destroyed para container: {container_id}");
        Ok(())
    }

    fn validate_container_id_for_destruction(&self, container_id: &str) -> DockerResult<()> {
        if container_id.is_empty() || container_id.len() > 100 {
            return Err(DockerError::Other(anyhow::anyhow!(
                "Invalid container ID length: {}",
                container_id
            )));
        }

        if !container_id
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
        {
            return Err(DockerError::Other(anyhow::anyhow!(
                "Container ID contains unsafe characters: {}",
                container_id
            )));
        }

        let forbidden_patterns = [
            "docker",
            "registry",
            "nginx",
            "postgres",
            "mysql",
            "redis",
            "elasticsearch",
            "mongo",
            "ubuntu",
            "alpine",
            "debian",
            "centos",
        ];

        let lower_id = container_id.to_lowercase();
        for pattern in &forbidden_patterns {
            if lower_id.contains(pattern) && !lower_id.starts_with("para-") {
                return Err(DockerError::Other(anyhow::anyhow!(
                    "Refusing to destroy system-like container: {}",
                    container_id
                )));
            }
        }

        Ok(())
    }

    fn verify_para_container(&self, container_id: &str) -> DockerResult<()> {
        let inspect_output = Command::new("docker")
            .args(["inspect", "--format", "{{.Name}}", container_id])
            .output()
            .map_err(|e| {
                DockerError::Other(anyhow::anyhow!("Failed to inspect container: {}", e))
            })?;

        if !inspect_output.status.success() {
            return Err(DockerError::Other(anyhow::anyhow!(
                "Container {} does not exist or is not accessible",
                container_id
            )));
        }

        let container_name = String::from_utf8_lossy(&inspect_output.stdout);
        let clean_name = container_name.trim().trim_start_matches('/');

        if !clean_name.starts_with("para-") {
            return Err(DockerError::Other(anyhow::anyhow!(
                "Container {} is not a para container (name: {})",
                container_id,
                clean_name
            )));
        }

        Ok(())
    }

    /// Get the current number of active containers
    #[allow(dead_code)] // Used for pool statistics and monitoring
    pub fn active_containers(&self) -> usize {
        self.get_active_containers().unwrap_or_default().len()
    }

    /// Get the maximum pool size
    #[allow(dead_code)] // Used for pool statistics and monitoring
    pub fn max_size(&self) -> usize {
        self.max_size
    }

    /// Clean up all containers in the pool
    #[allow(dead_code)] // Used for explicit cleanup and testing
    pub fn cleanup(&self) -> DockerResult<()> {
        let active_containers = self.get_active_containers().unwrap_or_default();

        // Stop and remove all active containers
        for container_id in active_containers.iter() {
            let _ = Command::new("docker").args(["stop", container_id]).output();
            let _ = Command::new("docker").args(["rm", container_id]).output();
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_container_pool_creation() {
        let pool = ContainerPool::new(5);
        assert_eq!(pool.max_size(), 5);
        // active_containers() now queries Docker, so the count depends on actual Docker state
        let active = pool.active_containers();
        println!("Pool created with max_size: 5, current active containers: {active}");
    }

    #[test]
    fn test_pool_queries_docker_state() {
        // This test verifies the pool queries actual Docker state
        let pool = ContainerPool::new(3);

        // First check if Docker is available
        let docker_check = Command::new("docker").args(["info"]).output();

        if docker_check.is_err() || !docker_check.unwrap().status.success() {
            println!("Skipping test - Docker not available");
            return;
        }

        // Get actual container count from Docker
        let active_count = pool.active_containers();
        println!("Current active para containers: {active_count}");

        // Check capacity - should succeed unless we have 3+ real containers running
        match pool.check_capacity() {
            Ok(_) => {
                assert!(
                    active_count < 3,
                    "Pool should have capacity when under limit"
                );
            }
            Err(e) => {
                assert!(active_count >= 3, "Pool should be full when at/over limit");
                let error_msg = e.to_string();
                assert!(error_msg.contains("Docker container pool exhausted"));
            }
        }
    }

    #[test]
    fn test_pool_container_verification() {
        let pool = ContainerPool::new(2);

        // Test adding a non-existent container should fail
        let result = pool.add_container("test-session", "non-existent-container-id");
        assert!(result.is_err(), "Should fail to add non-existent container");

        // Test container ID validation
        assert!(pool.validate_container_id_for_destruction("").is_err());
        assert!(pool
            .validate_container_id_for_destruction("../etc/passwd")
            .is_err());
        assert!(pool
            .validate_container_id_for_destruction("valid-container-123")
            .is_ok());
    }

    #[test]
    fn test_pool_respects_configured_limit() {
        // Test that pool correctly reports its configured limit
        let test_cases = vec![1, 5, 10];

        for max_size in test_cases {
            let pool = ContainerPool::new(max_size);
            assert_eq!(
                pool.max_size(),
                max_size,
                "Pool should report correct max size"
            );

            // The actual enforcement depends on real Docker state
            // We can't simulate it in unit tests without creating real containers
        }
    }

    #[test]
    fn test_get_active_containers_format() {
        let pool = ContainerPool::new(2);

        // Test that get_active_containers returns valid format
        match pool.get_active_containers() {
            Ok(containers) => {
                // Each container ID should be non-empty if present
                for id in containers {
                    assert!(!id.is_empty(), "Container ID should not be empty");
                }
            }
            Err(e) => {
                // If Docker is not available, that's ok for unit tests
                println!("Docker not available for test: {e}");
            }
        }
    }

    #[test]
    fn test_pool_cleanup() {
        let pool = ContainerPool::new(2);

        // Cleanup should not fail even if no containers exist
        assert!(pool.cleanup().is_ok());
    }

    #[test]
    fn test_docker_integration() {
        use std::process::Command;

        println!("\n=== Testing Docker Integration ===");

        // Check if Docker is available
        let docker_check = Command::new("docker").args(["version"]).output();

        match docker_check {
            Ok(output) if output.status.success() => {
                println!("✓ Docker is available");

                // Create pool and check actual state
                let pool = ContainerPool::new(3);
                let active = pool.active_containers();
                println!("Active para containers: {}/{}", active, pool.max_size());

                // Test capacity check with real state
                match pool.check_capacity() {
                    Ok(_) => println!("Pool has capacity for new containers"),
                    Err(e) => println!("Pool is full: {e}"),
                }
            }
            _ => {
                println!("⚠️  Docker not available, skipping integration test");
            }
        }
    }
}
