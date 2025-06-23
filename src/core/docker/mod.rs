//! Docker integration module for para
//!
//! This module provides containerization support for para sessions,
//! allowing developers to work in isolated Docker environments.

pub mod config;
pub mod error;
pub mod manager;
pub mod service;
pub mod session;

#[cfg(test)]
mod tests;

pub use config::{DockerConfig, ResourceLimits, VolumeMapping};
pub use error::{DockerError, DockerResult};
pub use manager::DockerManager;
pub use service::DockerService;
pub use session::{ContainerSession, ContainerStatus};