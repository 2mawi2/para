use crate::core::session::SessionManager;
use crate::utils::{ParaError, Result};
use std::path::PathBuf;

pub struct SessionDetector<'a> {
    session_manager: &'a SessionManager,
}

impl<'a> SessionDetector<'a> {
    pub fn new(session_manager: &'a SessionManager) -> Self {
        Self { session_manager }
    }

    pub fn detect_or_use_provided(&self, provided: Option<String>) -> Result<String> {
        match provided {
            Some(name) => Ok(name),
            None => self.find_current_session(),
        }
    }

    pub fn find_current_session(&self) -> Result<String> {
        // Try to detect session from current directory
        let current_dir = std::env::current_dir().map_err(|e| {
            ParaError::fs_error(format!("Failed to get current directory: {}", e))
        })?;

        match self.session_manager.find_session_by_path(&current_dir)? {
            Some(session) => Ok(session.name),
            None => Err(ParaError::invalid_args(
                "Not in a para session directory. Use --session to specify session name.",
            )),
        }
    }

    pub fn get_state_directory(&self, config_state_dir: &str) -> Result<PathBuf> {
        use crate::utils::get_main_repository_root;
        use std::path::Path;

        if Path::new(config_state_dir).is_absolute() {
            // If state_dir is already absolute (e.g., in tests), use it directly
            Ok(PathBuf::from(config_state_dir))
        } else {
            // Otherwise, resolve it relative to the main repo root
            let repo_root = get_main_repository_root()
                .map_err(|e| ParaError::git_error(format!("Not in a para repository: {}", e)))?;
            Ok(repo_root.join(config_state_dir))
        }
    }
}