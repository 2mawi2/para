//! Docker manager for para sessions
//!
//! Coordinates Docker operations with para session management

use super::{ContainerPool, DockerError, DockerIdeIntegration, DockerResult, DockerService};
use crate::config::Config;
use crate::core::docker::service::ContainerOptions;
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
    docker_image: Option<String>,
    forward_keys: bool,
}

impl DockerManager {
    /// Create a new Docker manager
    pub fn new(config: Config, network_isolation: bool, allowed_domains: Vec<String>) -> Self {
        Self::with_image(config, network_isolation, allowed_domains, None)
    }

    /// Create a new Docker manager with a custom Docker image
    pub fn with_image(
        config: Config,
        network_isolation: bool,
        allowed_domains: Vec<String>,
        docker_image: Option<String>,
    ) -> Self {
        Self::with_options(
            config,
            network_isolation,
            allowed_domains,
            docker_image,
            true,
        )
    }

    /// Create a new Docker manager with all options
    pub fn with_options(
        config: Config,
        network_isolation: bool,
        allowed_domains: Vec<String>,
        docker_image: Option<String>,
        forward_keys: bool,
    ) -> Self {
        // Use CLI-only approach: pool size is passed as runtime parameter, not from config
        let pool_size = 5; // Default pool size, can be made configurable via CLI flag later
        let pool = Arc::new(ContainerPool::new(pool_size));

        Self {
            service: DockerService,
            config,
            pool,
            network_isolation,
            allowed_domains,
            docker_image,
            forward_keys,
        }
    }

    /// Get the appropriate Docker image name based on priority
    fn get_docker_image(&self) -> DockerResult<String> {
        // Priority order:
        // 1. CLI flag (docker_image from manager)
        // 2. Config docker.default_image
        // 3. Default: "para-authenticated:latest"

        let image = self
            .docker_image
            .clone()
            .or_else(|| self.config.get_docker_image().map(|s| s.to_string()))
            .unwrap_or_else(|| "para-authenticated:latest".to_string());

        // Check if the image exists locally
        let output = Command::new("docker")
            .args(["images", "-q", &image])
            .output()
            .map_err(|e| DockerError::DaemonNotAvailable(e.to_string()))?;

        if !output.status.success() || output.stdout.is_empty() {
            // Image doesn't exist locally
            if image == "para-authenticated:latest" {
                return Err(DockerError::Other(anyhow::anyhow!(
                    "The 'para-authenticated:latest' image is not available. Please build it first with authentication credentials baked in.\n\
                     Alternatively, you can specify a custom image using --docker-image flag."
                )));
            } else {
                // Try to pull the custom image
                println!("🐳 Image '{image}' not found locally. Attempting to pull...");

                let pull_output = Command::new("docker")
                    .args(["pull", &image])
                    .output()
                    .map_err(|e| {
                        DockerError::Other(anyhow::anyhow!("Failed to execute docker pull: {}", e))
                    })?;

                if !pull_output.status.success() {
                    let stderr = String::from_utf8_lossy(&pull_output.stderr);
                    return Err(DockerError::Other(anyhow::anyhow!(
                        "Failed to pull Docker image '{}':\n{}\n\n\
                         Please ensure:\n\
                         1. The image name is correct\n\
                         2. You have access to the image repository\n\
                         3. You are logged in to the registry if required (docker login)",
                        image,
                        stderr
                    )));
                }

                println!("✅ Successfully pulled image: {image}");
            }
        }

        Ok(image)
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

        // Get the Docker image to use
        let docker_image = self.get_docker_image()?;

        // Create the container with CLI parameters (authentication is now baked into the image)
        println!("🏗️  Creating container with image: {docker_image}");

        // Get the configured API keys to forward
        let env_keys = self.config.get_forward_env_keys();

        let options = ContainerOptions {
            session_name: &session.name,
            network_isolation: self.network_isolation,
            allowed_domains: &self.allowed_domains,
            working_dir: &session.worktree_path,
            docker_args,
            docker_image: &docker_image,
            forward_keys: self.forward_keys,
            env_keys: &env_keys,
        };
        let container_session = self.service.create_container(&options)?;

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

        println!("✅ Container ready: {container_id}");
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
            image_name,
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

    /// Run setup script in container
    pub fn run_setup_script(
        &self,
        session_name: &str,
        script_path: &std::path::Path,
    ) -> DockerResult<()> {
        self.service.run_setup_script(session_name, script_path)
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

    /// Stop and remove a container for a session
    pub fn stop_container(&self, session_name: &str) -> DockerResult<()> {
        self.service.stop_container(session_name)
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

        let git_source = format!("{host_path}/.git");
        let copy_result = Command::new("docker")
            .args([
                "cp",
                &git_source,
                &format!("{container_id}:{workspace_path}"),
            ])
            .output()
            .map_err(|e| DockerError::Other(anyhow::anyhow!("Failed to copy .git: {}", e)))?;

        if !copy_result.status.success() {
            let stderr = String::from_utf8_lossy(&copy_result.stderr);
            eprintln!("Warning: .git copy failed (non-fatal): {stderr}");
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
            eprintln!("Warning: Source file copy had issues (non-fatal): {stderr}");
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

        let safe_path = format!("/workspace/{session_name}");

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
            if host_path == *dangerous || host_path.starts_with(&format!("{dangerous}/")) {
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

        // Validate user inputs first
        let forbidden_patterns = [
            "rm", "del", "unlink", "truncate", ";", "&&", "||", "|", "`", "$",
        ];
        for forbidden in &forbidden_patterns {
            if workspace_path.contains(forbidden) || host_path.contains(forbidden) {
                return Err(DockerError::Other(anyhow::anyhow!(
                    "Path contains forbidden pattern: {}",
                    forbidden
                )));
            }
        }

        let safe_cmd = format!(
            "set -euo pipefail; cd '{workspace_path}' && find '{host_path}' -maxdepth 3 -type f -name '*.rs' -o -name '*.toml' -o -name '*.md' -o -name '*.json' -o -name '*.txt' -o -name '*.yml' -o -name '*.yaml' | head -1000 | xargs -I {{}} cp '{{}}' '{workspace_path}/' 2>/dev/null || true"
        );

        Ok(safe_cmd)
    }
}

#[cfg(test)]
#[path = "manager_test.rs"]
mod manager_test;
