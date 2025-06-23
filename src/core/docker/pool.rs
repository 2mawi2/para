//! Docker container pool implementation
//!
//! Manages a pool of Docker containers to prevent system overload when running
//! multiple Para agents in parallel. Provides container reuse and resource limiting.

use super::{DockerError, DockerResult};
use std::collections::VecDeque;
use std::process::Command;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

/// Pool of Docker containers that can be reused across sessions
pub struct ContainerPool {
    available: Arc<Mutex<VecDeque<String>>>,
    in_use: Arc<Mutex<Vec<String>>>,
    max_size: usize,
}

impl ContainerPool {
    /// Create a new container pool with the specified maximum size
    pub fn new(max_size: usize) -> Self {
        Self {
            available: Arc::new(Mutex::new(VecDeque::new())),
            in_use: Arc::new(Mutex::new(Vec::new())),
            max_size,
        }
    }

    /// Acquire a container from the pool
    ///
    /// Returns either an existing available container or creates a new one
    /// if the pool has capacity. Returns an error if the pool is exhausted.
    pub fn acquire(&self) -> DockerResult<String> {
        let mut available = self.available.lock().unwrap();
        let mut in_use = self.in_use.lock().unwrap();

        // Try to get an available container first
        if let Some(container_id) = available.pop_front() {
            in_use.push(container_id.clone());
            return Ok(container_id);
        }

        // Check if we can create a new container
        if in_use.len() < self.max_size {
            let container_id = self.create_new_container()?;
            in_use.push(container_id.clone());
            return Ok(container_id);
        }

        // Pool is exhausted
        Err(DockerError::Other(anyhow::anyhow!(
            "Docker container pool exhausted (max: {}). Finish some sessions or increase max_containers in config.",
            self.max_size
        )))
    }

    /// Release a container back to the pool
    ///
    /// The container is reset and made available for reuse
    pub fn release(&self, container_id: String) -> DockerResult<()> {
        let mut available = self.available.lock().unwrap();
        let mut in_use = self.in_use.lock().unwrap();

        // Remove from in_use
        in_use.retain(|id| id != &container_id);

        // Reset container for reuse
        self.reset_container(&container_id)?;

        // Add to available pool
        available.push_back(container_id);
        Ok(())
    }

    /// Get the current number of containers in use
    #[allow(dead_code)] // Used for pool statistics and monitoring
    pub fn containers_in_use(&self) -> usize {
        self.in_use.lock().unwrap().len()
    }

    /// Get the current number of available containers
    #[allow(dead_code)] // Used for pool statistics and monitoring
    pub fn containers_available(&self) -> usize {
        self.available.lock().unwrap().len()
    }

    /// Get the maximum pool size
    #[allow(dead_code)] // Used for pool statistics and monitoring
    pub fn max_size(&self) -> usize {
        self.max_size
    }

    /// Clean up all containers in the pool
    pub fn cleanup(&self) -> DockerResult<()> {
        let available = self.available.lock().unwrap();
        let in_use = self.in_use.lock().unwrap();

        // Stop and remove all pool containers
        for container_id in available.iter().chain(in_use.iter()) {
            let _ = Command::new("docker").args(["stop", container_id]).output();
            let _ = Command::new("docker").args(["rm", container_id]).output();
        }

        Ok(())
    }

    /// Create a new container for the pool
    fn create_new_container(&self) -> DockerResult<String> {
        let container_name = format!("para-pool-{}", Uuid::new_v4());

        let output = Command::new("docker")
            .args([
                "run",
                "-dt",
                "--name",
                &container_name,
                "para-authenticated:latest",
                "sleep",
                "infinity",
            ])
            .output()
            .map_err(|e| DockerError::DaemonNotAvailable(e.to_string()))?;

        if !output.status.success() {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            return Err(DockerError::ContainerCreationFailed(format!(
                "Failed to create pool container: {}",
                error_msg
            )));
        }

        Ok(container_name)
    }

    /// Reset a container for reuse
    fn reset_container(&self, container_id: &str) -> DockerResult<()> {
        // Clean up container workspace
        let _output = Command::new("docker")
            .args(["exec", container_id, "rm", "-rf", "/workspace/*"])
            .output()
            .map_err(|e| DockerError::Other(anyhow::anyhow!("Failed to clean workspace: {}", e)))?;

        // Reset git config in case it was modified
        let _output = Command::new("docker")
            .args([
                "exec",
                container_id,
                "git",
                "config",
                "--global",
                "--unset-all",
                "include.path",
            ])
            .output(); // Ignore errors - config might not be set

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
        assert_eq!(pool.containers_in_use(), 0);
        assert_eq!(pool.containers_available(), 0);
    }

    #[test]
    fn test_pool_size_limits() {
        let pool = ContainerPool::new(2);

        // Should be able to track pool state
        assert_eq!(pool.containers_in_use(), 0);
        assert_eq!(pool.containers_available(), 0);

        // Pool state management
        {
            let mut in_use = pool.in_use.lock().unwrap();
            in_use.push("test-container-1".to_string());
            in_use.push("test-container-2".to_string());
        }

        assert_eq!(pool.containers_in_use(), 2);

        // Should be at capacity
        {
            let in_use = pool.in_use.lock().unwrap();
            assert_eq!(in_use.len(), pool.max_size());
        }
    }

    #[test]
    fn test_pool_state_tracking() {
        let pool = ContainerPool::new(3);

        // Test available pool
        {
            let mut available = pool.available.lock().unwrap();
            available.push_back("available-1".to_string());
            available.push_back("available-2".to_string());
        }

        assert_eq!(pool.containers_available(), 2);

        // Test in_use tracking
        {
            let mut in_use = pool.in_use.lock().unwrap();
            in_use.push("in-use-1".to_string());
        }

        assert_eq!(pool.containers_in_use(), 1);
        assert_eq!(pool.containers_available(), 2);
    }

    #[test]
    fn test_pool_cleanup() {
        let pool = ContainerPool::new(2);

        // Add some test containers to state
        {
            let mut available = pool.available.lock().unwrap();
            let mut in_use = pool.in_use.lock().unwrap();
            available.push_back("test-available".to_string());
            in_use.push("test-in-use".to_string());
        }

        // Cleanup should not fail (even though containers don't exist)
        assert!(pool.cleanup().is_ok());
    }

    #[test]
    fn test_pool_enforces_maximum_limit() {
        // Create a pool with max size of 3
        let pool = ContainerPool::new(3);
        
        // Simulate acquiring containers up to the limit
        {
            let mut in_use = pool.in_use.lock().unwrap();
            in_use.push("container-1".to_string());
            in_use.push("container-2".to_string());
            in_use.push("container-3".to_string());
        }
        
        // Verify pool is at capacity
        assert_eq!(pool.containers_in_use(), 3);
        assert_eq!(pool.containers_available(), 0);
        
        // Try to acquire another container - should fail
        let result = pool.acquire();
        
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
    fn test_pool_reuse_after_release() {
        // Create a pool with max size of 2
        let pool = ContainerPool::new(2);
        
        // Simulate 2 containers in use (at capacity)
        {
            let mut in_use = pool.in_use.lock().unwrap();
            in_use.push("container-1".to_string());
            in_use.push("container-2".to_string());
        }
        
        assert_eq!(pool.containers_in_use(), 2);
        
        // Try to acquire - should fail (pool exhausted)
        let result = pool.acquire();
        assert!(result.is_err());
        
        // Release one container
        {
            let mut in_use = pool.in_use.lock().unwrap();
            let mut available = pool.available.lock().unwrap();
            
            // Simulate release
            in_use.retain(|id| id != "container-1");
            available.push_back("container-1".to_string());
        }
        
        assert_eq!(pool.containers_in_use(), 1);
        assert_eq!(pool.containers_available(), 1);
        
        // Now acquire should succeed (reusing the released container)
        let result = pool.acquire();
        assert!(result.is_ok());
        
        if let Ok(container_id) = result {
            assert_eq!(container_id, "container-1");
        }
        
        // Pool should be at capacity again
        assert_eq!(pool.containers_in_use(), 2);
        assert_eq!(pool.containers_available(), 0);
    }

    #[test]
    fn test_pool_respects_configured_limit() {
        // Test with different pool sizes
        let test_cases = vec![
            (1, "Pool size 1"),
            (5, "Pool size 5"),
            (10, "Pool size 10"),
        ];
        
        for (max_size, description) in test_cases {
            let pool = ContainerPool::new(max_size);
            
            // Fill the pool to capacity
            {
                let mut in_use = pool.in_use.lock().unwrap();
                for i in 0..max_size {
                    in_use.push(format!("container-{}", i));
                }
            }
            
            // Verify pool is at configured capacity
            assert_eq!(pool.containers_in_use(), max_size, "{}", description);
            assert_eq!(pool.max_size(), max_size, "{}", description);
            
            // Try to exceed limit
            let result = pool.acquire();
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
}
