use std::fmt;

pub type Result<T> = std::result::Result<T, ParaError>;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum ParaError {
    InvalidArgs(String),
    NotImplemented(String),
    IoError(String),
    GitError(String),
    ConfigError(String),
}

impl ParaError {
    pub fn invalid_args(msg: impl Into<String>) -> Self {
        ParaError::InvalidArgs(msg.into())
    }
    
    pub fn not_implemented(msg: impl Into<String>) -> Self {
        ParaError::NotImplemented(msg.into())
    }
    
    #[allow(dead_code)]
    pub fn io_error(msg: impl Into<String>) -> Self {
        ParaError::IoError(msg.into())
    }
    
    #[allow(dead_code)]
    pub fn git_error(msg: impl Into<String>) -> Self {
        ParaError::GitError(msg.into())
    }
    
    #[allow(dead_code)]
    pub fn config_error(msg: impl Into<String>) -> Self {
        ParaError::ConfigError(msg.into())
    }
}

impl fmt::Display for ParaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParaError::InvalidArgs(msg) => write!(f, "Invalid arguments: {}", msg),
            ParaError::NotImplemented(msg) => write!(f, "{} not implemented yet", msg),
            ParaError::IoError(msg) => write!(f, "IO error: {}", msg),
            ParaError::GitError(msg) => write!(f, "Git error: {}", msg),
            ParaError::ConfigError(msg) => write!(f, "Configuration error: {}", msg),
        }
    }
}

impl std::error::Error for ParaError {}