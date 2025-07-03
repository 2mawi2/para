use crate::core::git::{GitOperations, GitService};
use crate::utils::Result;
use std::path::PathBuf;

/// Handles cleanup of stale branches that no longer have corresponding state files
pub struct BranchCleaner<'a> {
    git_service: &'a GitService,
    branch_prefix: &'a str,
    state_dir: &'a str,
}

impl<'a> BranchCleaner<'a> {
    pub fn new(git_service: &'a GitService, branch_prefix: &'a str, state_dir: &'a str) -> Self {
        Self {
            git_service,
            branch_prefix,
            state_dir,
        }
    }

    /// Find stale branches that no longer have corresponding state files
    pub fn find_stale_branches(&self) -> Result<Vec<String>> {
        let mut stale_branches = Vec::new();
        let prefix = format!("{}/", self.branch_prefix);
        let state_dir = PathBuf::from(self.state_dir);

        let all_branches = self.git_service.branch_manager().list_branches()?;

        for branch_info in all_branches {
            if branch_info.name.starts_with(&prefix) && !branch_info.name.contains("/archived/") {
                let session_id = branch_info.name.strip_prefix(&prefix).unwrap_or("");
                let state_file = state_dir.join(format!("{}.state", session_id));

                if !state_file.exists() {
                    stale_branches.push(branch_info.name);
                }
            }
        }

        Ok(stale_branches)
    }

    /// Remove stale branches and return the count of successfully removed branches
    pub fn remove_stale_branches(&self, stale_branches: Vec<String>) -> (usize, Vec<String>) {
        let mut removed_count = 0;
        let mut errors = Vec::new();

        for branch in stale_branches {
            match self.git_service.delete_branch(&branch, true) {
                Ok(_) => removed_count += 1,
                Err(e) => errors.push(format!("Failed to remove branch {}: {}", branch, e)),
            }
        }

        (removed_count, errors)
    }
}
