//! Container session types for MVP

use std::path::PathBuf;

/// Minimal container session info for MVP
#[allow(dead_code)]
pub struct ContainerSession {
    /// Container ID (or name)
    pub container_id: String,
    /// Para session name
    pub session_name: String,
    /// Docker image
    pub image: String,
    /// Working directory (mounted path)
    pub working_dir: PathBuf,
}

impl ContainerSession {
    /// Create a new container session for MVP
    pub fn new(
        container_id: String,
        session_name: String,
        image: String,
        working_dir: PathBuf,
    ) -> Self {
        Self {
            container_id,
            session_name,
            image,
            working_dir,
        }
    }
}
