use crate::core::git::{branch::BranchManager, GitRepository};
use crate::utils::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinishRequest {
    pub feature_branch: String,
    pub commit_message: String,
    pub target_branch_name: Option<String>,
}

#[derive(Debug)]
pub enum FinishResult {
    Success { final_branch: String },
}

pub struct FinishManager<'a> {
    repo: &'a GitRepository,
}

impl<'a> FinishManager<'a> {
    pub fn new(repo: &'a GitRepository) -> Self {
        Self { repo }
    }

    pub fn finish_session(&self, request: FinishRequest) -> Result<FinishResult> {
        // Ensure we're on the feature branch
        let current_branch = self.repo.get_current_branch()?;
        if current_branch != request.feature_branch {
            self.repo.checkout_branch(&request.feature_branch)?;
        }

        // Staging changes
        if self.repo.has_uncommitted_changes()? {
            self.repo.stage_all_changes()?;
            self.repo.commit(&request.commit_message)?;
        }

        // For now, finish just creates a branch
        let final_branch = if let Some(target_name) = request.target_branch_name {
            target_name
        } else {
            request.feature_branch.clone()
        };

        // Check out the target branch if it's different
        if final_branch != current_branch {
            // Create or checkout the target branch
            let branch_manager = BranchManager::new(self.repo);
            if !branch_manager.branch_exists(&final_branch)? {
                branch_manager.create_branch(&final_branch, &current_branch)?;
            }
            self.repo.checkout_branch(&final_branch)?;
        }

        Ok(FinishResult::Success { final_branch })
    }
}
