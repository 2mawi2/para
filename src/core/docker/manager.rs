//! Docker manager for para sessions
//!
//! Coordinates Docker operations with para session management

use super::{DockerError, DockerIdeIntegration, DockerResult, DockerService};
use crate::config::Config;
use crate::core::docker::session::ContainerSession;
use crate::core::session::SessionState;
use std::process::Command;

/// Docker manager that integrates with para sessions
pub struct DockerManager {
    service: DockerService,
    config: Config,
    network_isolation: bool,
    allowed_domains: Vec<String>,
}

impl DockerManager {
    /// Create a new Docker manager
    pub fn new(config: Config, network_isolation: bool, allowed_domains: Vec<String>) -> Self {
        Self {
            service: DockerService,
            config,
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

    /// Create and start a container for a session
    pub fn create_container_session(
        &self,
        session: &SessionState,
        docker_args: &[String],
    ) -> DockerResult<()> {
        println!("🐳 Creating Docker container for session: {}", session.name);

        // Check Docker is available
        self.service.health_check()?;

        // Create the container (authentication is now baked into the image)
        println!("🏗️  Creating container with authenticated image");
        let _container = self.service.create_container(
            &session.name,
            self.network_isolation,
            &self.allowed_domains,
            &session.worktree_path,
            docker_args,
        )?;

        // Start it with verification
        println!("▶️  Starting container: para-{}", session.name);
        self.service
            .start_container_with_verification(&session.name, self.network_isolation)?;

        Ok(())
    }

    /// Launch IDE connected to container
    pub fn launch_container_ide(
        &self,
        session: &SessionState,
        initial_prompt: Option<&str>,
        dangerously_skip_permissions: bool,
    ) -> DockerResult<()> {
        let image_name = self.get_docker_image()?;
        let container_session = ContainerSession::new(
            format!("para-{}", session.name),
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

    /// Stop and remove a container for a session
    pub fn stop_container(&self, session_name: &str) -> DockerResult<()> {
        self.service.stop_container(session_name)
    }
}
