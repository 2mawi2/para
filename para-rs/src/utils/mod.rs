pub mod error;
pub mod fs;
pub mod json;
pub mod names;

pub use error::{ParaError, Result};
// Re-export specific items instead of glob imports to reduce warnings
pub use names::{generate_branch_name, generate_unique_name, validate_session_name, generate_session_id_with_name as generate_session_id};

pub type CommandResult = Result<()>;
pub type StringResult = Result<String>;
pub type PathResult = Result<std::path::PathBuf>;
