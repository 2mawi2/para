//! Docker integration module for para
//!
//! This module provides Docker container support for para sessions.

pub mod error;
pub mod ide_integration;
pub mod manager;
pub mod patch_watcher;
pub mod service;
pub mod session;

#[cfg(test)]
pub mod mock;

// Re-export main types
pub use error::{DockerError, DockerResult};
pub use ide_integration::DockerIdeIntegration;
pub use manager::DockerManager;
pub use patch_watcher::ContainerPatchWatcher;
pub use service::DockerService;
pub use session::ContainerSession;
