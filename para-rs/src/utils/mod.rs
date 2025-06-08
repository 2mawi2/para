pub mod error;
pub mod fs;
pub mod json;
pub mod names;

pub use error::{ParaError, Result};
pub use fs::*;
pub use json::*;
pub use names::*;

pub type CommandResult = Result<()>;
pub type StringResult = Result<String>;
pub type PathResult = Result<std::path::PathBuf>;
