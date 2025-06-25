//! Docker integration module for para
//!
//! This module provides Docker container support for para sessions.

pub mod error;
pub mod extraction;
pub mod ide_integration;
pub mod manager;
pub mod service;
pub mod session;
pub mod signal_files;
pub mod watcher;

#[cfg(test)]
pub mod mock;

#[cfg(test)]
pub mod network_isolation_tests;

// Re-export main types
pub use error::{DockerError, DockerResult};
pub use ide_integration::DockerIdeIntegration;
pub use manager::DockerManager;
pub use service::DockerService;
pub use session::ContainerSession;
