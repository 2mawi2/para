//! Docker manager for coordinating container operations with para sessions

use anyhow::Result;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::config::Config;
use crate::core::session::{SessionManager, SessionState};

use super::{
    config::{detect_project_type, DockerConfig, VolumeMapping},
    error::{DockerError, DockerResult},
    extraction::{ContainerExtractor, ExtractionOptions, ExtractionResult},
    service::DockerService,
    session::{ContainerSession, ContainerStatus},
};

/// Manages Docker containers for para sessions
pub struct DockerManager {
    /// Docker service implementation
    docker_service: Arc<dyn DockerService>,
    /// Para configuration
    config: Config,
    /// Docker-specific configuration
    docker_config: DockerConfig,
    /// Path to para state directory
    state_dir: PathBuf,
}

impl DockerManager {
    /// Create a new Docker manager
    pub fn new(
        docker_service: Arc<dyn DockerService>,
        config: Config,
        docker_config: DockerConfig,
    ) -> Self {
        let state_dir = PathBuf::from(&config.directories.state_dir);
        Self {
            docker_service,
            config,
            docker_config,
            state_dir,
        }
    }

    /// Create a Docker container for a para session
    pub fn create_container_for_session(
        &self,
        session_state: &SessionState,
    ) -> DockerResult<ContainerSession> {
        // Determine the image to use
        let project_type = detect_project_type(&session_state.worktree_path);
        let image = self
            .docker_config
            .image_mappings
            .get(&project_type)
            .cloned()
            .unwrap_or_else(|| self.docker_config.default_image.clone());

        // Ensure the image is available
        self.docker_service.ensure_image(&image)?;

        // Create container with session-specific configuration
        let mut container_config = self.docker_config.clone();
        container_config.default_image = image;

        // Add session-specific volume for the worktree
        let worktree_volume = VolumeMapping {
            source: session_state.worktree_path.to_string_lossy().to_string(),
            target: "/workspace".to_string(),
            read_only: false,
            mount_type: super::session::MountType::Bind,
        };
        container_config.default_volumes.insert(0, worktree_volume);

        // Create the container
        let container = self.docker_service.create_session(
            &session_state.name,
            &container_config,
            Path::new("/workspace"),
        )?;

        // Save container metadata
        self.save_container_metadata(&session_state.name, &container)?;

        Ok(container)
    }

    /// Start a container for an existing session
    pub fn start_container(&self, session_name: &str) -> DockerResult<()> {
        // Check if container exists
        let status = self.docker_service.get_container_status(session_name)?;

        match status {
            ContainerStatus::Running => {
                return Err(DockerError::ContainerAlreadyRunning {
                    name: session_name.to_string(),
                });
            }
            ContainerStatus::Unknown => {
                return Err(DockerError::ContainerNotFound {
                    name: session_name.to_string(),
                });
            }
            _ => {}
        }

        // Start the container
        self.docker_service.start_session(session_name)?;

        // Wait for container to be ready
        self.docker_service
            .wait_for_status(session_name, ContainerStatus::Running, 30)?;

        // Run post-start hooks
        for command in &self.docker_config.hooks.post_start {
            self.docker_service.exec_in_container(
                session_name,
                "sh",
                &["-c".to_string(), command.clone()],
                None,
            )?;
        }

        Ok(())
    }

    /// Stop a container for a session
    pub fn stop_container(&self, session_name: &str) -> DockerResult<()> {
        // Check if container is running
        let status = self.docker_service.get_container_status(session_name)?;

        if !matches!(status, ContainerStatus::Running) {
            return Err(DockerError::ContainerNotRunning {
                name: session_name.to_string(),
            });
        }

        // Run pre-stop hooks
        for command in &self.docker_config.hooks.pre_stop {
            // Ignore errors in pre-stop hooks
            let _ = self.docker_service.exec_in_container(
                session_name,
                "sh",
                &["-c".to_string(), command.clone()],
                None,
            );
        }

        // Stop the container
        self.docker_service.stop_session(session_name)
    }

    /// Finish a session and optionally remove its container
    pub fn finish_container(&self, session_name: &str, remove: bool) -> DockerResult<()> {
        // Stop the container if running
        let status = self.docker_service.get_container_status(session_name)?;
        if matches!(status, ContainerStatus::Running) {
            self.stop_container(session_name)?;
        }

        if remove {
            self.docker_service.finish_session(session_name, true)?;
            self.remove_container_metadata(session_name)?;
        }

        Ok(())
    }

    /// Execute a command in a container
    pub fn exec_in_container(
        &self,
        session_name: &str,
        command: &str,
        args: &[String],
    ) -> DockerResult<String> {
        // Ensure container is running
        let status = self.docker_service.get_container_status(session_name)?;
        if !matches!(status, ContainerStatus::Running) {
            self.start_container(session_name)?;
        }

        self.docker_service
            .exec_in_container(session_name, command, args, None)
    }

    /// Open an interactive shell in a container
    pub fn attach_to_container(&self, session_name: &str) -> DockerResult<()> {
        // Ensure container is running
        let status = self.docker_service.get_container_status(session_name)?;
        if !matches!(status, ContainerStatus::Running) {
            self.start_container(session_name)?;
        }

        self.docker_service.attach_to_container(session_name)
    }

    /// List all Docker containers managed by para
    pub fn list_containers(&self) -> DockerResult<Vec<ContainerSession>> {
        self.docker_service.list_sessions()
    }

    /// Get container logs
    pub fn get_logs(
        &self,
        session_name: &str,
        follow: bool,
        tail: Option<usize>,
    ) -> DockerResult<String> {
        self.docker_service.get_logs(session_name, follow, tail)
    }

    /// Check Docker daemon health
    pub fn health_check(&self) -> DockerResult<()> {
        self.docker_service.health_check()
    }

    /// Save container metadata to state directory
    fn save_container_metadata(
        &self,
        session_name: &str,
        container: &ContainerSession,
    ) -> Result<()> {
        let metadata_path = self.state_dir.join(format!("{}.docker", session_name));
        let metadata_json = serde_json::to_string_pretty(container)?;
        std::fs::write(metadata_path, metadata_json)?;
        Ok(())
    }

    /// Load container metadata from state directory
    pub fn load_container_metadata(&self, session_name: &str) -> Result<Option<ContainerSession>> {
        let metadata_path = self.state_dir.join(format!("{}.docker", session_name));
        if !metadata_path.exists() {
            return Ok(None);
        }

        let metadata_json = std::fs::read_to_string(metadata_path)?;
        let container: ContainerSession = serde_json::from_str(&metadata_json)?;
        Ok(Some(container))
    }

    /// Remove container metadata
    fn remove_container_metadata(&self, session_name: &str) -> Result<()> {
        let metadata_path = self.state_dir.join(format!("{}.docker", session_name));
        if metadata_path.exists() {
            std::fs::remove_file(metadata_path)?;
        }
        Ok(())
    }

    /// Sync container status with Docker daemon
    pub fn sync_container_status(&self, session_name: &str) -> DockerResult<ContainerStatus> {
        let status = self.docker_service.get_container_status(session_name)?;

        // Update metadata if container exists
        if let Ok(Some(mut metadata)) = self.load_container_metadata(session_name) {
            metadata.status = status.clone();
            let _ = self.save_container_metadata(session_name, &metadata);
        }

        Ok(status)
    }

    /// Clean up orphaned containers (containers without corresponding para sessions)
    pub fn cleanup_orphaned_containers(&self) -> DockerResult<Vec<String>> {
        let containers = self.docker_service.list_sessions()?;
        let session_manager = SessionManager::new(&self.config);
        let sessions = session_manager
            .list_sessions()
            .map_err(|e| DockerError::Other(anyhow::anyhow!("Session manager error: {}", e)))?;
        let session_names: Vec<String> = sessions.into_iter().map(|s| s.name).collect();

        let mut removed = Vec::new();

        for container in containers {
            if !session_names.contains(&container.session_name) {
                // Container exists but session doesn't - clean it up
                self.docker_service
                    .cancel_session(&container.session_name)?;
                self.remove_container_metadata(&container.session_name)?;
                removed.push(container.session_name);
            }
        }

        Ok(removed)
    }

    /// Extract code changes from a container and apply them to a new Git branch
    pub fn extract_container_changes(
        &self,
        options: ExtractionOptions,
    ) -> DockerResult<ExtractionResult> {
        // Get the Git repository
        let git_repo = crate::core::git::GitRepository::discover().map_err(|e| {
            DockerError::Other(anyhow::anyhow!("Failed to discover Git repository: {}", e))
        })?;

        // Create the extractor
        let extractor = ContainerExtractor::new(self.docker_service.as_ref(), git_repo);

        // Extract the changes
        extractor.extract_container_changes(options)
    }
}
