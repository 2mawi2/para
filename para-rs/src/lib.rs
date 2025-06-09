pub mod cli;
pub mod config;
pub mod core;
pub mod platform;
pub mod utils;

pub use config::Config;
pub use core::git::GitService;
pub use core::session::{SessionManager, SessionState, SessionStatus};
pub use platform::{get_platform_manager, IdeConfig, PlatformManager, WindowInfo};
pub use utils::{ParaError, Result};
