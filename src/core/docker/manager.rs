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
    network_isolation: bool,
    allowed_domains: Vec<String>,
}

impl DockerManager {
    /// Create a new Docker manager
    pub fn new(config: Config, network_isolation: bool, allowed_domains: Vec<String>) -> Self {
        // Use CLI-only approach: pool size is passed as runtime parameter, not from config
        let pool_size = 1; // TEMPORARY: Set to 1 for testing, should be 5 in production
        let pool = Arc::new(ContainerPool::new(pool_size));

        Self {
            service: DockerService,
            config,
            pool,
            network_isolation,
            allowed_domains,
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
    pub fn create_container_session(
        &self,
        session: &mut SessionState,
        docker_args: &[String],
    ) -> DockerResult<()> {
        println!(
            "🐳 Setting up Docker container for session: {}",
            session.name
        );

        // Check Docker is available
        self.service.health_check()?;

        // Check pool capacity BEFORE creating container
        println!("🔍 Checking pool capacity before creating container...");
        self.pool.check_capacity()?;

        // Create the container with CLI parameters (authentication is now baked into the image)
        println!("🏗️  Creating container with authenticated image");
        let container_session = self.service.create_container(
            &session.name,
            self.network_isolation,
            &self.allowed_domains,
            &session.worktree_path,
            docker_args,
        )?;

        // Add the successfully created container to pool tracking immediately
        let container_id = container_session.container_id.clone();
        self.pool.add_container(&session.name, &container_id)?;

        // Start it with verification
        println!("▶️  Starting container: para-{}", session.name);
        self.service
            .start_container_with_verification(&session.name, self.network_isolation)?;

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
        dangerously_skip_permissions: bool,
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
            dangerously_skip_permissions,
        )
        .map_err(|e| DockerError::Other(e.into()))?;

        Ok(())
    }

    /// Destroy a session's container
    pub fn destroy_session_container(&self, session: &SessionState) -> DockerResult<()> {
        match &session.session_type {
            SessionType::Container {
                container_id: Some(id),
            } => {
                println!("🗑️  Destroying container for session: {}", session.name);
                self.pool.destroy_session_container(id)?;
                Ok(())
            }
            _ => {
                // Not a container session, nothing to destroy
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
        self.validate_container_setup_inputs(container_id, session)?;

        let workspace_path = self.get_safe_workspace_path(&session.name)?;
        let mkdir_result = Command::new("docker")
            .args(["exec", container_id, "mkdir", "-p", &workspace_path])
            .output()
            .map_err(|e| {
                DockerError::Other(anyhow::anyhow!("Failed to create workspace: {}", e))
            })?;

        if !mkdir_result.status.success() {
            let stderr = String::from_utf8_lossy(&mkdir_result.stderr);
            return Err(DockerError::Other(anyhow::anyhow!(
                "Workspace creation failed: {}",
                stderr
            )));
        }

        let host_path = self.validate_host_path(&session.worktree_path)?;

        let git_source = format!("{}/.git", host_path);
        let copy_result = Command::new("docker")
            .args([
                "cp",
                &git_source,
                &format!("{}:{}", container_id, workspace_path),
            ])
            .output()
            .map_err(|e| DockerError::Other(anyhow::anyhow!("Failed to copy .git: {}", e)))?;

        if !copy_result.status.success() {
            let stderr = String::from_utf8_lossy(&copy_result.stderr);
            eprintln!("Warning: .git copy failed (non-fatal): {}", stderr);
        }

        let safe_copy_cmd = self.build_safe_copy_command(&workspace_path, &host_path)?;
        let source_copy_result = Command::new("docker")
            .args(["exec", container_id, "sh", "-c", &safe_copy_cmd])
            .output()
            .map_err(|e| {
                DockerError::Other(anyhow::anyhow!("Failed to copy source files: {}", e))
            })?;

        if !source_copy_result.status.success() {
            let stderr = String::from_utf8_lossy(&source_copy_result.stderr);
            eprintln!(
                "Warning: Source file copy had issues (non-fatal): {}",
                stderr
            );
        }

        Ok(())
    }

    fn validate_container_setup_inputs(
        &self,
        container_id: &str,
        session: &SessionState,
    ) -> DockerResult<()> {
        if !container_id
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
        {
            return Err(DockerError::Other(anyhow::anyhow!(
                "Invalid container ID format: {}",
                container_id
            )));
        }

        if session.name.contains("..") || session.name.contains('/') || session.name.contains('\\')
        {
            return Err(DockerError::Other(anyhow::anyhow!(
                "Session name contains unsafe characters: {}",
                session.name
            )));
        }

        if session.name.is_empty() || session.name.len() > 100 {
            return Err(DockerError::Other(anyhow::anyhow!(
                "Session name is empty or too long: {}",
                session.name
            )));
        }

        Ok(())
    }

    fn get_safe_workspace_path(&self, session_name: &str) -> DockerResult<String> {
        if session_name.contains("..") || session_name.contains('/') {
            return Err(DockerError::Other(anyhow::anyhow!(
                "Session name contains path traversal attempts: {}",
                session_name
            )));
        }

        let safe_path = format!("/workspace/{}", session_name);

        if !safe_path.starts_with("/workspace/") {
            return Err(DockerError::Other(anyhow::anyhow!(
                "Generated unsafe workspace path: {}",
                safe_path
            )));
        }

        Ok(safe_path)
    }

    fn validate_host_path(&self, worktree_path: &std::path::Path) -> DockerResult<String> {
        let host_path = worktree_path.display().to_string();

        let dangerous_paths = ["/", "/bin", "/usr", "/etc", "/var", "/tmp", "/home"];
        for dangerous in &dangerous_paths {
            if host_path == *dangerous || host_path.starts_with(&format!("{}/", dangerous)) {
                return Err(DockerError::Other(anyhow::anyhow!(
                    "Refusing to operate on system directory: {}",
                    host_path
                )));
            }
        }

        if !worktree_path.exists() {
            return Err(DockerError::Other(anyhow::anyhow!(
                "Source path does not exist: {}",
                host_path
            )));
        }

        if !worktree_path.is_dir() {
            return Err(DockerError::Other(anyhow::anyhow!(
                "Source path is not a directory: {}",
                host_path
            )));
        }

        Ok(host_path)
    }

    fn build_safe_copy_command(
        &self,
        workspace_path: &str,
        host_path: &str,
    ) -> DockerResult<String> {
        if !workspace_path.starts_with("/workspace/") {
            return Err(DockerError::Other(anyhow::anyhow!(
                "Workspace path validation failed: {}",
                workspace_path
            )));
        }

        let safe_cmd = format!(
            "set -euo pipefail; cd '{}' && find '{}' -maxdepth 3 -type f -name '*.rs' -o -name '*.toml' -o -name '*.md' -o -name '*.json' -o -name '*.txt' -o -name '*.yml' -o -name '*.yaml' | head -1000 | xargs -I {{}} cp '{{}}' '{}/' 2>/dev/null || true",
            workspace_path,
            host_path,
            workspace_path
        );

        let forbidden_commands = ["rm", "del", "unlink", "truncate", ">", ">>"];
        for forbidden in &forbidden_commands {
            if safe_cmd.contains(forbidden) {
                return Err(DockerError::Other(anyhow::anyhow!(
                    "Generated command contains forbidden operation: {}",
                    forbidden
                )));
            }
        }

        Ok(safe_cmd)
    }

    /// Get pool statistics
    #[allow(dead_code)] // Used for debugging and future monitoring features
    pub fn pool_stats(&self) -> (usize, usize) {
        (self.pool.active_containers(), self.pool.max_size())
    }
}
