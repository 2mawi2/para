//! Docker-specific error types

use thiserror::Error;

#[derive(Error, Debug)]
pub enum DockerError {
    #[error("Docker daemon not available: {0}")]
    DaemonNotAvailable(String),

    #[error("Failed to create container: {0}")]
    ContainerCreationFailed(String),

    #[error("Failed to start container: {0}")]
    ContainerStartFailed(String),

    #[error("Network isolation verification failed: {0}")]
    NetworkIsolationFailed(String),

    #[error("Docker command failed: {0}")]
    CommandFailed(String),

    #[error("General error: {0}")]
    Other(#[from] anyhow::Error),
}

pub type DockerResult<T> = Result<T, DockerError>;
