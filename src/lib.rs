pub mod cli;
pub mod config;
pub mod core;
pub mod platform;
pub mod ui;
pub mod utils;

#[cfg(test)]
pub mod test_utils;

pub use config::Config;
pub use core::git::GitService;
pub use core::session::{SessionManager, SessionState, SessionStatus};
pub use platform::{get_platform_manager, PlatformManager};
pub use utils::{ParaError, Result};
