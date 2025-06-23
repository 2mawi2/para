//! Docker integration module for para
//!
//! This module provides Docker container support for para sessions.

pub mod auth;
pub mod error;
pub mod extraction;
pub mod ide_integration;
pub mod manager;
pub mod service;
pub mod session;

#[cfg(test)]
pub mod mock;

// Re-export main types
pub use auth::{get_auth_resolver, ClaudeAuthTokens};
pub use error::{DockerError, DockerResult};
pub use ide_integration::DockerIdeIntegration;
pub use manager::DockerManager;
pub use service::DockerService;
pub use session::ContainerSession;
