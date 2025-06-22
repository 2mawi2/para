use crate::utils::{gitignore::GitignoreManager, ParaError, Result};

/// Add entry to .gitignore file
pub fn add_to_gitignore(entry: &str) -> Result<bool> {
    let gitignore_manager = GitignoreManager::new(".");
    gitignore_manager
        .add_entry(entry)
        .map_err(|e| ParaError::file_operation(format!("Failed to update .gitignore: {}", e)))
}
