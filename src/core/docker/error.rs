//! Docker-specific error types

use thiserror::Error;

#[derive(Error, Debug)]
pub enum DockerError {
    #[error("Docker daemon not available: {0}")]
    DaemonNotAvailable(String),

    #[error("Container '{name}' not found")]
    ContainerNotFound { name: String },

    #[error("Failed to create container: {0}")]
    ContainerCreationFailed(String),

    #[error("Failed to start container: {0}")]
    ContainerStartFailed(String),

    #[error("Failed to stop container: {0}")]
    ContainerStopFailed(String),

    #[error("Invalid container configuration: {0}")]
    InvalidConfiguration(String),

    #[error("Volume mount error: {0}")]
    VolumeMountError(String),

    #[error("Network configuration error: {0}")]
    NetworkError(String),

    #[error("Container '{name}' is already running")]
    ContainerAlreadyRunning { name: String },

    #[error("Container '{name}' is not running")]
    ContainerNotRunning { name: String },

    #[error("Failed to execute command in container: {0}")]
    ExecFailed(String),

    #[error("Failed to communicate with container: {0}")]
    CommunicationError(String),

    #[error("Resource limit exceeded: {0}")]
    ResourceLimitExceeded(String),

    #[error("Docker image '{image}' not found")]
    ImageNotFound { image: String },

    #[error("Failed to pull Docker image: {0}")]
    ImagePullFailed(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Docker API error: {0}")]
    ApiError(String),

    #[error("General error: {0}")]
    Other(#[from] anyhow::Error),
}

pub type DockerResult<T> = Result<T, DockerError>;
