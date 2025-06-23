//! Docker manager for para sessions
//!
//! Coordinates Docker operations with para session management

use super::{DockerError, DockerResult, DockerService};
use crate::config::Config;
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
        // Check Docker is available
        self.service.health_check()?;

        // Create the container
        let _container = self.service.create_container(
            &session.name,
            &self.config.docker,
            &session.worktree_path,
        )?;

        // Start it
        self.service.start_container(&session.name)?;

        // Generate devcontainer config
        let container_session = crate::core::docker::session::ContainerSession::new(
            format!("para-{}", session.name),
            session.name.clone(),
            self.config.docker.default_image.clone(),
            session.worktree_path.clone(),
        );

        crate::core::docker::DockerIdeIntegration::generate_devcontainer_config(
            &session.worktree_path,
            &container_session,
            None,
        )
        .map_err(|e| DockerError::Other(e.into()))?;

        Ok(())
    }
}
