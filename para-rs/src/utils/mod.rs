pub mod error;
pub mod fs;
pub mod json;
pub mod names;

pub use error::{ParaError, Result};
// Re-export specific items instead of glob imports to reduce warnings
pub use names::{generate_branch_name, generate_friendly_name, generate_timestamp, generate_unique_name, validate_session_name};

pub type CommandResult = Result<()>;
pub type StringResult = Result<String>;
pub type PathResult = Result<std::path::PathBuf>;
