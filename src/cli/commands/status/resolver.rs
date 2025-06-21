use crate::config::Config;
use crate::utils::{get_main_repository_root, ParaError, Result};
use std::path::{Path, PathBuf};

pub struct StatePathResolver;

impl StatePathResolver {
    pub fn resolve_state_dir(config: &Config) -> Result<PathBuf> {
        if Path::new(&config.directories.state_dir).is_absolute() {
            // If state_dir is already absolute (e.g., in tests), use it directly
            Ok(PathBuf::from(&config.directories.state_dir))
        } else {
            // Otherwise, resolve it relative to the main repo root
            let repo_root = get_main_repository_root()
                .map_err(|e| ParaError::git_error(format!("Not in a para repository: {}", e)))?;
            Ok(repo_root.join(&config.directories.state_dir))
        }
    }
}