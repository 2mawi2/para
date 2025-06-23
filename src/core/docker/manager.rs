//! Docker manager for para sessions
//!
//! Coordinates Docker operations with para session management

use super::{ContainerPool, DockerError, DockerIdeIntegration, DockerResult, DockerService};
use crate::config::Config;
use crate::core::docker::session::ContainerSession;
use crate::core::session::{SessionState, SessionType};
use std::process::Command;
use std::sync::Arc;

/// Docker manager that integrates with para sessions
pub struct DockerManager {
    service: DockerService,
    config: Config,
    pool: Arc<ContainerPool>,
}

impl DockerManager {
    /// Create a new Docker manager
    pub fn new(config: Config) -> Self {
        let pool_size = config.docker.max_containers;
        let pool = Arc::new(ContainerPool::new(pool_size));

        Self {
            service: DockerService,
            config,
            pool,
        }
    }

    /// Get the appropriate Docker image name
    fn get_docker_image(&self) -> DockerResult<&'static str> {
        // Check if authenticated image exists
        let output = Command::new("docker")
            .args(["images", "-q", "para-authenticated:latest"])
            .output()
            .map_err(|e| DockerError::DaemonNotAvailable(e.to_string()))?;

        if output.status.success() && !output.stdout.is_empty() {
            Ok("para-authenticated:latest")
        } else {
            Err(DockerError::Other(anyhow::anyhow!(
                "The 'para-authenticated:latest' image is not available. Please build it first with authentication credentials baked in."
            )))
        }
    }

    /// Create and start a container for a session using the pool
    pub fn create_container_session(&self, session: &mut SessionState) -> DockerResult<()> {
        println!(
            "🐳 Setting up Docker container for session: {}",
            session.name
        );

        // Check Docker is available
        self.service.health_check()?;

        // Acquire container from pool
        let container_id = self.pool.acquire()?;
        println!("🔄 Acquired container from pool: {}", container_id);

        // Setup workspace in container
        self.setup_container_workspace(&container_id, session)?;

        // Update session to track container
        session.session_type = SessionType::Container {
            container_id: Some(container_id.clone()),
        };

        println!("✅ Container ready: {}", container_id);
        Ok(())
    }

    /// Launch IDE connected to container
    pub fn launch_container_ide(
        &self,
        session: &SessionState,
        initial_prompt: Option<&str>,
    ) -> DockerResult<()> {
        // Get container ID from session
        let container_id = match &session.session_type {
            SessionType::Container {
                container_id: Some(id),
            } => id.clone(),
            _ => {
                return Err(DockerError::Other(anyhow::anyhow!(
                    "Session {} is not a container session or has no container ID",
                    session.name
                )))
            }
        };

        let image_name = self.get_docker_image()?;
        let container_session = ContainerSession::new(
            container_id,
            session.name.clone(),
            image_name.to_string(),
            session.worktree_path.clone(),
        );

        // Launch IDE with automatic container connection
        DockerIdeIntegration::launch_container_ide(
            &self.config,
            &session.worktree_path,
            &container_session,
            initial_prompt,
        )
        .map_err(|e| DockerError::Other(e.into()))?;

        Ok(())
    }

    /// Release a container back to the pool
    pub fn release_container(&self, session: &SessionState) -> DockerResult<()> {
        match &session.session_type {
            SessionType::Container {
                container_id: Some(id),
            } => {
                println!("🔄 Returning container to pool: {}", id);
                self.pool.release(id.clone())?;
                println!("✅ Container returned to pool");
                Ok(())
            }
            _ => {
                // Not a container session, nothing to release
                Ok(())
            }
        }
    }

    /// Setup workspace in a container for a session
    fn setup_container_workspace(
        &self,
        container_id: &str,
        session: &SessionState,
    ) -> DockerResult<()> {
        // Create workspace directory in container
        let workspace_path = format!("/workspace/{}", session.name);
        Command::new("docker")
            .args(["exec", container_id, "mkdir", "-p", &workspace_path])
            .output()
            .map_err(|e| {
                DockerError::Other(anyhow::anyhow!("Failed to create workspace: {}", e))
            })?;

        // Copy worktree files to container
        let host_path = session.worktree_path.display().to_string();
        let copy_cmd = format!("{}/.git", host_path);
        Command::new("docker")
            .args([
                "cp",
                &copy_cmd,
                &format!("{}:{}", container_id, workspace_path),
            ])
            .output()
            .map_err(|e| DockerError::Other(anyhow::anyhow!("Failed to copy files: {}", e)))?;

        // Copy source files too
        Command::new("docker")
            .args([
                "exec",
                container_id,
                "sh",
                "-c",
                &format!(
                    "cd '{}' && cp -r '{}'/.* '{}' 2>/dev/null || true",
                    workspace_path, host_path, workspace_path
                ),
            ])
            .output()
            .map_err(|e| {
                DockerError::Other(anyhow::anyhow!("Failed to copy source files: {}", e))
            })?;

        Ok(())
    }

    /// Get pool statistics
    #[allow(dead_code)] // Used for debugging and future monitoring features
    pub fn pool_stats(&self) -> (usize, usize, usize) {
        (
            self.pool.containers_in_use(),
            self.pool.containers_available(),
            self.pool.max_size(),
        )
    }
}
