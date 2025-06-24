//! Docker container pool implementation
//!
//! Manages container lifecycle with resource limits to prevent system overload.
//! Each session gets its own isolated container (no sharing for security).

use super::{DockerError, DockerResult};
use std::process::Command;
use std::sync::{Arc, Mutex};

/// Pool that manages Docker container lifecycle with resource limits
///
/// This pool enforces limits on concurrent containers but does NOT reuse
/// containers across sessions for security isolation. Each session gets
/// its own dedicated container that is created and destroyed as needed.
pub struct ContainerPool {
    active_sessions: Arc<Mutex<Vec<String>>>, // Track active session containers
    max_size: usize,
}

impl ContainerPool {
    /// Create a new container pool with the specified maximum size
    pub fn new(max_size: usize) -> Self {
        Self {
            active_sessions: Arc::new(Mutex::new(Vec::new())),
            max_size,
        }
    }

    /// Check if pool has capacity for a new container
    pub fn check_capacity(&self) -> DockerResult<()> {
        let active = self.active_sessions.lock().unwrap();
        if active.len() >= self.max_size {
            return Err(DockerError::Other(anyhow::anyhow!(
                "Docker container pool exhausted (max: {}). Finish some sessions or increase max_containers.",
                self.max_size
            )));
        }
        Ok(())
    }

    /// Add a container to pool tracking
    pub fn add_container(&self, session_name: &str, container_id: &str) -> DockerResult<()> {
        let mut active = self.active_sessions.lock().unwrap();

        // Double-check capacity hasn't changed
        if active.len() >= self.max_size {
            return Err(DockerError::Other(anyhow::anyhow!(
                "Docker container pool exhausted during container creation (max: {})",
                self.max_size
            )));
        }

        active.push(container_id.to_string());
        println!(
            "🐳 Added container to pool tracking for session {}: {}",
            session_name, container_id
        );
        Ok(())
    }

    /// Destroy a session's container and remove from pool tracking
    pub fn destroy_session_container(&self, container_id: &str) -> DockerResult<()> {
        self.validate_container_id_for_destruction(container_id)?;

        let mut active = self.active_sessions.lock().unwrap();

        if !active.iter().any(|id| id == container_id) {
            return Err(DockerError::Other(anyhow::anyhow!(
                "Refusing to destroy untracked container: {}",
                container_id
            )));
        }

        active.retain(|id| id != container_id);
        drop(active); // Release lock before Docker operations

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

            // Re-add to tracking if removal failed (recovery)
            let mut active = self.active_sessions.lock().unwrap();
            if !active.iter().any(|id| id == container_id) {
                active.push(container_id.to_string());
            }

            return Err(DockerError::Other(anyhow::anyhow!(
                "Failed to remove container {}: {}",
                container_id,
                error_msg
            )));
        }

        println!("🗑️  Safely destroyed para container: {}", container_id);
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
        self.active_sessions.lock().unwrap().len()
    }

    /// Get the maximum pool size
    #[allow(dead_code)] // Used for pool statistics and monitoring
    pub fn max_size(&self) -> usize {
        self.max_size
    }

    /// Clean up all containers in the pool
    pub fn cleanup(&self) -> DockerResult<()> {
        let active = self.active_sessions.lock().unwrap();

        // Stop and remove all active containers
        for container_id in active.iter() {
            let _ = Command::new("docker").args(["stop", container_id]).output();
            let _ = Command::new("docker").args(["rm", container_id]).output();
        }

        Ok(())
    }
}

impl Drop for ContainerPool {
    /// Automatically cleanup containers when the pool is dropped
    fn drop(&mut self) {
        let _ = self.cleanup();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_container_pool_creation() {
        let pool = ContainerPool::new(5);
        assert_eq!(pool.max_size(), 5);
        assert_eq!(pool.active_containers(), 0);
    }

    #[test]
    fn test_pool_enforces_maximum_limit() {
        // Create a pool with max size of 3
        let pool = ContainerPool::new(3);

        // Simulate 3 active sessions (at capacity)
        {
            let mut active = pool.active_sessions.lock().unwrap();
            active.push("container-1".to_string());
            active.push("container-2".to_string());
            active.push("container-3".to_string());
        }

        // Verify pool is at capacity
        assert_eq!(pool.active_containers(), 3);

        // Try to check capacity - should fail
        let result = pool.check_capacity();

        // Verify it returns the expected error
        assert!(result.is_err());
        if let Err(DockerError::Other(err)) = result {
            let error_msg = err.to_string();
            assert!(error_msg.contains("Docker container pool exhausted"));
            assert!(error_msg.contains("max: 3"));
            assert!(error_msg.contains("Finish some sessions or increase max_containers"));
        } else {
            panic!("Expected DockerError::Other with pool exhaustion message");
        }
    }

    #[test]
    fn test_pool_tracks_container_lifecycle() {
        let pool = ContainerPool::new(2);

        // Initially empty
        assert_eq!(pool.active_containers(), 0);

        // Simulate container creation
        {
            let mut active = pool.active_sessions.lock().unwrap();
            active.push("test-container-1".to_string());
        }

        assert_eq!(pool.active_containers(), 1);

        // Simulate container destruction
        {
            let mut active = pool.active_sessions.lock().unwrap();
            active.retain(|id| id != "test-container-1");
        }

        assert_eq!(pool.active_containers(), 0);
    }

    #[test]
    fn test_pool_respects_configured_limit() {
        // Test with different pool sizes
        let test_cases = vec![(1, "Pool size 1"), (5, "Pool size 5"), (10, "Pool size 10")];

        for (max_size, description) in test_cases {
            let pool = ContainerPool::new(max_size);

            // Fill the pool to capacity
            {
                let mut active = pool.active_sessions.lock().unwrap();
                for i in 0..max_size {
                    active.push(format!("container-{}", i));
                }
            }

            // Verify pool is at configured capacity
            assert_eq!(pool.active_containers(), max_size, "{}", description);
            assert_eq!(pool.max_size(), max_size, "{}", description);

            // Try to exceed limit
            let result = pool.check_capacity();
            assert!(result.is_err(), "{}: Should not exceed limit", description);

            // Verify error message contains the correct limit
            if let Err(DockerError::Other(err)) = result {
                let error_msg = err.to_string();
                assert!(
                    error_msg.contains(&format!("max: {}", max_size)),
                    "{}: Error should mention limit {}",
                    description,
                    max_size
                );
            }
        }
    }

    #[test]
    fn test_pool_capacity_checking() {
        let pool = ContainerPool::new(2);

        // Should have capacity initially
        assert!(pool.check_capacity().is_ok());

        // Add containers to capacity
        pool.add_container("test1", "container1").unwrap();
        assert_eq!(pool.active_containers(), 1);
        assert!(pool.check_capacity().is_ok());

        pool.add_container("test2", "container2").unwrap();
        assert_eq!(pool.active_containers(), 2);

        // At capacity, check_capacity should fail (can't add more)
        let capacity_check = pool.check_capacity();
        assert!(capacity_check.is_err());
        assert!(capacity_check
            .unwrap_err()
            .to_string()
            .contains("exhausted"));

        // Try to exceed capacity - add_container should also fail
        let result = pool.add_container("test3", "container3");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("exhausted"));

        // Pool count should remain unchanged after failed add
        assert_eq!(pool.active_containers(), 2);
    }

    #[test]
    fn test_pool_cleanup() {
        let pool = ContainerPool::new(2);

        // Add some test containers to state
        {
            let mut active = pool.active_sessions.lock().unwrap();
            active.push("test-container-1".to_string());
            active.push("test-container-2".to_string());
        }

        // Cleanup should not fail (even though containers don't exist)
        assert!(pool.cleanup().is_ok());
    }
}
