//! Docker integration module for para
//!
//! This module provides containerization support for para sessions,
//! allowing developers to work in isolated Docker environments.

pub mod auth;
pub mod config;
pub mod error;
pub mod manager;
pub mod service;
pub mod session;

#[cfg(test)]
pub mod mock;
#[cfg(test)]
mod tests;

// Re-export main types from submodules
pub use config::{DockerConfig, ResourceLimits, VolumeMapping};
pub use error::{DockerError, DockerResult};
pub use manager::DockerManager;
pub use service::DockerService;
pub use session::{ContainerSession, ContainerStatus};

// TODO: Uncomment when integrating with session commands
// pub use auth::DockerAuthManager;

// Re-export simple types for test infrastructure compatibility
use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Simple configuration for Docker-based para sessions (for test compatibility)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockerSessionConfig {
    /// Docker image to use
    pub image: String,
    /// Volume mappings (host_path, container_path)
    pub volumes: Vec<(String, String)>,
    /// Environment variables
    pub env_vars: Vec<(String, String)>,
    /// Working directory inside the container
    pub workdir: Option<String>,
}
