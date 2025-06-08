use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ParaError {
    #[error("Git operation failed: {message}")]
    GitOperation { message: String },

    #[error("Session '{session_id}' not found")]
    SessionNotFound { session_id: String },

    #[error("Session '{session_id}' already exists")]
    SessionExists { session_id: String },

    #[error("Configuration error: {message}")]
    Config { message: String },

    #[error("IDE error: {message}")]
    Ide { message: String },

    #[error("Invalid arguments: {message}")]
    InvalidArgs { message: String },

    #[error("Repository state error: {message}")]
    RepoState { message: String },

    #[error("File operation failed: {path}")]
    FileOperation { path: String },

    #[error("File not found: {path}")]
    FileNotFound { path: String },

    #[error("Directory not found: {path}")]
    DirectoryNotFound { path: String },

    #[error("IDE not available: {ide}")]
    IdeNotAvailable { ide: String },

    #[error("Invalid session name: {name} - {reason}")]
    InvalidSessionName { name: String, reason: String },

    #[error("Invalid branch name: {name} - {reason}")]
    InvalidBranchName { name: String, reason: String },

    #[error("Permission denied: {path}")]
    PermissionDenied { path: String },

    #[error("Worktree operation failed: {message}")]
    WorktreeOperation { message: String },

    #[error("State corruption detected: {message}")]
    StateCorruption { message: String },

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Regex error: {0}")]
    Regex(#[from] regex::Error),
}

pub type Result<T> = std::result::Result<T, ParaError>;

impl ParaError {
    pub fn git_operation(message: impl Into<String>) -> Self {
        Self::GitOperation {
            message: message.into(),
        }
    }

    pub fn session_not_found(session_id: impl Into<String>) -> Self {
        Self::SessionNotFound {
            session_id: session_id.into(),
        }
    }

    pub fn session_exists(session_id: impl Into<String>) -> Self {
        Self::SessionExists {
            session_id: session_id.into(),
        }
    }

    pub fn config_error(message: impl Into<String>) -> Self {
        Self::Config {
            message: message.into(),
        }
    }

    pub fn ide_error(message: impl Into<String>) -> Self {
        Self::Ide {
            message: message.into(),
        }
    }

    pub fn invalid_args(message: impl Into<String>) -> Self {
        Self::InvalidArgs {
            message: message.into(),
        }
    }

    pub fn repo_state(message: impl Into<String>) -> Self {
        Self::RepoState {
            message: message.into(),
        }
    }

    pub fn file_operation(path: impl Into<String>) -> Self {
        Self::FileOperation { path: path.into() }
    }

    pub fn file_not_found(path: impl Into<String>) -> Self {
        Self::FileNotFound { path: path.into() }
    }

    pub fn directory_not_found(path: impl Into<String>) -> Self {
        Self::DirectoryNotFound { path: path.into() }
    }

    pub fn ide_not_available(ide: impl Into<String>) -> Self {
        Self::IdeNotAvailable { ide: ide.into() }
    }

    pub fn invalid_session_name(name: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::InvalidSessionName {
            name: name.into(),
            reason: reason.into(),
        }
    }

    pub fn invalid_branch_name(name: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::InvalidBranchName {
            name: name.into(),
            reason: reason.into(),
        }
    }

    pub fn permission_denied(path: impl Into<String>) -> Self {
        Self::PermissionDenied { path: path.into() }
    }

    pub fn worktree_operation(message: impl Into<String>) -> Self {
        Self::WorktreeOperation {
            message: message.into(),
        }
    }

    pub fn state_corruption(message: impl Into<String>) -> Self {
        Self::StateCorruption {
            message: message.into(),
        }
    }

    pub fn not_implemented(feature: impl Into<String>) -> Self {
        Self::InvalidArgs {
            message: format!("{} not implemented yet", feature.into()),
        }
    }
}

impl From<PathBuf> for ParaError {
    fn from(path: PathBuf) -> Self {
        Self::FileNotFound {
            path: path.to_string_lossy().to_string(),
        }
    }
}

impl From<&str> for ParaError {
    fn from(message: &str) -> Self {
        Self::Config {
            message: message.to_string(),
        }
    }
}

impl From<String> for ParaError {
    fn from(message: String) -> Self {
        Self::Config { message }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation_helpers() {
        let git_err = ParaError::git_operation("failed to commit");
        assert!(matches!(git_err, ParaError::GitOperation { .. }));
        assert_eq!(
            git_err.to_string(),
            "Git operation failed: failed to commit"
        );

        let session_err = ParaError::session_not_found("test-session");
        assert!(matches!(session_err, ParaError::SessionNotFound { .. }));
        assert_eq!(session_err.to_string(), "Session 'test-session' not found");

        let config_err = ParaError::config_error("invalid configuration");
        assert!(matches!(config_err, ParaError::Config { .. }));
        assert_eq!(
            config_err.to_string(),
            "Configuration error: invalid configuration"
        );
    }

    #[test]
    fn test_error_conversion() {
        let string_err: ParaError = "test error".into();
        assert!(matches!(string_err, ParaError::Config { .. }));

        let owned_string_err: ParaError = String::from("test error").into();
        assert!(matches!(owned_string_err, ParaError::Config { .. }));
    }

    #[test]
    fn test_io_error_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let para_err: ParaError = io_err.into();
        assert!(matches!(para_err, ParaError::Io(_)));
    }
}
