use crate::core::git::{GitOperations, GitService};
use crate::utils::Result;
use std::fs;
use std::path::PathBuf;

/// Handles cleanup of orphaned state files that no longer have corresponding branches
pub struct StateFileCleaner<'a> {
    git_service: &'a GitService,
    branch_prefix: &'a str,
    state_dir: &'a str,
}

impl<'a> StateFileCleaner<'a> {
    pub fn new(git_service: &'a GitService, branch_prefix: &'a str, state_dir: &'a str) -> Self {
        Self {
            git_service,
            branch_prefix,
            state_dir,
        }
    }

    /// Find orphaned state files that no longer have corresponding branches
    pub fn find_orphaned_state_files(&self) -> Result<Vec<PathBuf>> {
        let state_dir = PathBuf::from(self.state_dir);

        if !state_dir.exists() {
            return Ok(Vec::new());
        }

        let mut orphaned_files = Vec::new();
        let state_files = self.scan_state_directory(&state_dir)?;

        for state_file in state_files {
            let session_id = self.extract_session_id(&state_file)?;

            if self.is_session_orphaned(&session_id)? {
                orphaned_files.push(state_file.clone());
                orphaned_files.extend(self.find_related_files(&state_dir, &session_id));
            }
        }

        Ok(orphaned_files)
    }

    /// Remove orphaned state files and return the count of successfully removed files
    pub fn remove_orphaned_files(&self, orphaned_files: Vec<PathBuf>) -> (usize, Vec<String>) {
        let mut removed_count = 0;
        let mut errors = Vec::new();

        for file_path in orphaned_files {
            match fs::remove_file(&file_path) {
                Ok(_) => removed_count += 1,
                Err(e) => errors.push(format!(
                    "Failed to remove file {}: {}",
                    file_path.display(),
                    e
                )),
            }
        }

        (removed_count, errors)
    }

    fn scan_state_directory(&self, state_dir: &std::path::Path) -> Result<Vec<PathBuf>> {
        let mut state_files = Vec::new();

        for entry in fs::read_dir(state_dir)? {
            let entry = entry?;
            let path = entry.path();

            if self.is_state_file(&path) {
                state_files.push(path);
            }
        }

        Ok(state_files)
    }

    fn is_state_file(&self, path: &std::path::Path) -> bool {
        path.file_name()
            .and_then(|n| n.to_str())
            .map(|name| name.ends_with(".state"))
            .unwrap_or(false)
    }

    fn extract_session_id(&self, state_file: &std::path::Path) -> Result<String> {
        let file_name = state_file
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| crate::utils::ParaError::invalid_args("Invalid state file name"))?;

        let session_id = file_name.strip_suffix(".state").ok_or_else(|| {
            crate::utils::ParaError::invalid_args("State file must end with .state")
        })?;

        Ok(session_id.to_string())
    }

    fn is_session_orphaned(&self, session_id: &str) -> Result<bool> {
        let branch_name = format!("{}/{}", self.branch_prefix, session_id);
        Ok(!self.git_service.branch_exists(&branch_name)?)
    }

    fn find_related_files(&self, state_dir: &std::path::Path, session_id: &str) -> Vec<PathBuf> {
        let mut related_files = Vec::new();

        for suffix in &[".prompt", ".launch"] {
            let related_file = state_dir.join(format!("{}{}", session_id, suffix));
            if related_file.exists() {
                related_files.push(related_file);
            }
        }

        related_files
    }
}
