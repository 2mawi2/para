//! Docker manager for para sessions
//!
//! Coordinates Docker operations with para session management

use super::{get_auth_resolver, DockerError, DockerIdeIntegration, DockerResult, DockerService};
use crate::config::Config;
use crate::core::docker::session::ContainerSession;
use crate::core::session::SessionState;

/// Docker manager that integrates with para sessions
pub struct DockerManager {
    service: DockerService,
    config: Config,
}

impl DockerManager {
    /// Create a new Docker manager
    pub fn new(config: Config) -> Self {
        Self {
            service: DockerService,
            config,
        }
    }

    /// Create and start a container for a session
    pub fn create_container_session(&self, session: &SessionState) -> DockerResult<()> {
        println!("🐳 Creating Docker container for session: {}", session.name);

        // Check Docker is available
        self.service.health_check()?;

        // Get the auth resolver and retrieve Claude credentials
        println!("🔐 Retrieving Claude credentials from keychain...");
        let auth_resolver = get_auth_resolver();
        let auth_tokens = auth_resolver.get_claude_credentials()?;
        println!("✅ Successfully retrieved Claude credentials");

        // Create the container with auth tokens
        println!("🏗️  Creating container with image: para-claude:latest");
        let _container = self.service.create_container(
            &session.name,
            &self.config.docker,
            &session.worktree_path,
            Some(&auth_tokens),
        )?;

        // Start it
        println!("▶️  Starting container: para-{}", session.name);
        self.service.start_container(&session.name)?;

        Ok(())
    }

    /// Launch IDE connected to container
    pub fn launch_container_ide(
        &self,
        session: &SessionState,
        initial_prompt: Option<&str>,
    ) -> DockerResult<()> {
        let container_session = ContainerSession::new(
            format!("para-{}", session.name),
            session.name.clone(),
            "para-claude:latest".to_string(),
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
}
