pub mod error;
pub mod fs;
pub mod names;
pub mod json;

pub use error::{ParaError, Result};
pub use fs::*;
pub use names::*;
pub use json::*;

pub type CommandResult = Result<()>;
pub type StringResult = Result<String>;
pub type PathResult = Result<std::path::PathBuf>;