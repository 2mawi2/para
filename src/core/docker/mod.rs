//! Docker integration module for para
//!
//! This module provides Docker container support for para sessions.

pub mod error;
pub mod extraction;
pub mod ide_integration;
pub mod manager;
pub mod pool;
pub mod service;
pub mod session;

#[cfg(test)]
pub mod mock;

// Re-export main types
pub use error::{DockerError, DockerResult};
pub use ide_integration::DockerIdeIntegration;
pub use manager::DockerManager;
pub use pool::ContainerPool;
pub use service::DockerService;
