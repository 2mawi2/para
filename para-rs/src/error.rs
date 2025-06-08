use std::fmt;

pub type Result<T> = std::result::Result<T, ParaError>;

#[derive(Debug, Clone)]
pub enum ParaError {
    GitOperation(String),
    IoError(String),
    ConfigError(String),
    ValidationError(String),
    CommandError(String),
}

impl ParaError {
    pub fn git_operation(msg: String) -> Self {
        Self::GitOperation(msg)
    }

    pub fn io_error(msg: String) -> Self {
        Self::IoError(msg)
    }

    pub fn config_error(msg: String) -> Self {
        Self::ConfigError(msg)
    }

    pub fn validation_error(msg: String) -> Self {
        Self::ValidationError(msg)
    }

    pub fn command_error(msg: String) -> Self {
        Self::CommandError(msg)
    }
}

impl fmt::Display for ParaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParaError::GitOperation(msg) => write!(f, "Git operation failed: {}", msg),
            ParaError::IoError(msg) => write!(f, "IO error: {}", msg),
            ParaError::ConfigError(msg) => write!(f, "Configuration error: {}", msg),
            ParaError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
            ParaError::CommandError(msg) => write!(f, "Command error: {}", msg),
        }
    }
}

impl std::error::Error for ParaError {}

impl From<std::io::Error> for ParaError {
    fn from(err: std::io::Error) -> Self {
        ParaError::IoError(err.to_string())
    }
}