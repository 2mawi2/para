use crate::cli::parser::IntegrationStrategy;
use crate::core::git::integration::IntegrationManager;
use crate::core::git::repository::GitRepository;
use crate::utils::error::{ParaError, Result};
use std::path::PathBuf;

#[derive(Debug)]
pub struct StrategyRequest {
    pub feature_branch: String,
    pub target_branch: String,
    pub strategy: IntegrationStrategy,
    pub dry_run: bool,
    pub commit_message: Option<String>,
}

#[derive(Debug)]
pub enum StrategyResult {
    Success { final_branch: String },
    ConflictsPending { conflicted_files: Vec<PathBuf> },
    DryRun { preview: String },
    Failed { error: String },
}

pub struct StrategyManager<'a> {
    repo: &'a GitRepository,
    integration: IntegrationManager<'a>,
}

impl<'a> StrategyManager<'a> {
    pub fn new(repo: &'a GitRepository) -> Self {
        Self {
            repo,
            integration: IntegrationManager::new(repo),
        }
    }

    pub fn execute_strategy(&self, request: StrategyRequest) -> Result<StrategyResult> {
        self.integration
            .validate_integration_preconditions(&request.feature_branch, &request.target_branch)?;

        if request.dry_run {
            return self.preview_strategy(&request);
        }

        // Handle worktree integration differently to avoid checkout conflicts
        if self.integration.is_in_worktree()? {
            return self.execute_worktree_strategy(&request);
        }

        // Original main repo integration logic
        let backup_name = self.integration.create_backup_branch(
            &request.target_branch,
            &format!("pre-integration-{}", chrono::Utc::now().timestamp()),
        )?;

        match self.execute_integration_strategy(&request) {
            Ok(result) => Ok(result),
            Err(e) => {
                self.integration
                    .restore_from_backup(&backup_name, &request.target_branch)?;
                Err(e)
            }
        }
    }

    fn preview_strategy(&self, request: &StrategyRequest) -> Result<StrategyResult> {
        let preview = match request.strategy {
            IntegrationStrategy::Merge => self.preview_merge(request),
            IntegrationStrategy::Squash => self.preview_squash(request),
            IntegrationStrategy::Rebase => self.preview_rebase(request),
        }?;

        Ok(StrategyResult::DryRun { preview })
    }

    fn execute_integration_strategy(&self, request: &StrategyRequest) -> Result<StrategyResult> {
        match request.strategy {
            IntegrationStrategy::Merge => self.execute_merge(request),
            IntegrationStrategy::Squash => self.execute_squash(request),
            IntegrationStrategy::Rebase => self.execute_rebase(request),
        }
    }

    fn execute_merge(&self, request: &StrategyRequest) -> Result<StrategyResult> {
        self.integration
            .update_base_branch(&request.target_branch)?;

        self.repo.checkout_branch(&request.target_branch)?;

        let commit_message =
            self.generate_merge_commit_message(&request.feature_branch, &request.target_branch)?;

        match self.integration.create_merge_commit(
            &request.feature_branch,
            &request.target_branch,
            &commit_message,
        ) {
            Ok(()) => Ok(StrategyResult::Success {
                final_branch: request.target_branch.clone(),
            }),
            Err(_) => {
                let conflicted_files = self.integration.get_conflicted_files()?;
                if !conflicted_files.is_empty() {
                    Ok(StrategyResult::ConflictsPending { conflicted_files })
                } else {
                    Ok(StrategyResult::Failed {
                        error: "Merge failed without conflicts".to_string(),
                    })
                }
            }
        }
    }

    fn execute_squash(&self, request: &StrategyRequest) -> Result<StrategyResult> {
        self.integration
            .update_base_branch(&request.target_branch)?;

        let commits = self
            .integration
            .get_commit_range(&request.target_branch, &request.feature_branch)?;

        if commits.is_empty() {
            return Ok(StrategyResult::Success {
                final_branch: request.target_branch.clone(),
            });
        }

        self.repo.checkout_branch(&request.target_branch)?;

        let squash_message =
            self.generate_squash_commit_message(&request.feature_branch, &request.target_branch)?;

        match self.integration.cherry_pick_commits(&commits) {
            Ok(()) => {
                let merge_base = self
                    .repo
                    .get_merge_base(&request.target_branch, &request.feature_branch)?;
                self.integration.squash_commits(
                    &request.feature_branch,
                    &merge_base,
                    &squash_message,
                )?;

                Ok(StrategyResult::Success {
                    final_branch: request.target_branch.clone(),
                })
            }
            Err(_) => {
                let conflicted_files = self.integration.get_conflicted_files()?;
                if !conflicted_files.is_empty() {
                    Ok(StrategyResult::ConflictsPending { conflicted_files })
                } else {
                    Ok(StrategyResult::Failed {
                        error: "Squash failed without conflicts".to_string(),
                    })
                }
            }
        }
    }

    fn execute_rebase(&self, request: &StrategyRequest) -> Result<StrategyResult> {
        self.integration
            .update_base_branch(&request.target_branch)?;

        match self
            .integration
            .prepare_rebase(&request.feature_branch, &request.target_branch)
        {
            Ok(()) => {
                self.repo.checkout_branch(&request.target_branch)?;

                match self.repo.merge_fast_forward(&request.feature_branch) {
                    Ok(()) => Ok(StrategyResult::Success {
                        final_branch: request.target_branch.clone(),
                    }),
                    Err(e) => Ok(StrategyResult::Failed {
                        error: format!("Fast-forward merge failed: {}", e),
                    }),
                }
            }
            Err(_) => {
                let conflicted_files = self.integration.get_conflicted_files()?;
                if !conflicted_files.is_empty() {
                    Ok(StrategyResult::ConflictsPending { conflicted_files })
                } else {
                    Ok(StrategyResult::Failed {
                        error: "Rebase failed without conflicts".to_string(),
                    })
                }
            }
        }
    }

    fn preview_merge(&self, request: &StrategyRequest) -> Result<String> {
        let commits = self
            .integration
            .get_commit_range(&request.target_branch, &request.feature_branch)?;

        let commit_message =
            self.generate_merge_commit_message(&request.feature_branch, &request.target_branch)?;

        Ok(format!(
            "Merge Strategy Preview:\n\
            • Target branch: {}\n\
            • Feature branch: {}\n\
            • Commits to merge: {}\n\
            • Merge commit message: {}\n\
            • Result: All commits preserved in feature branch history\n\
            • Conflicts: Run with --strategy merge to check for conflicts",
            request.target_branch,
            request.feature_branch,
            commits.len(),
            commit_message
        ))
    }

    fn preview_squash(&self, request: &StrategyRequest) -> Result<String> {
        let commits = self
            .integration
            .get_commit_range(&request.target_branch, &request.feature_branch)?;

        let squash_message =
            self.generate_squash_commit_message(&request.feature_branch, &request.target_branch)?;

        Ok(format!(
            "Squash Strategy Preview:\n\
            • Target branch: {}\n\
            • Feature branch: {}\n\
            • Commits to squash: {}\n\
            • Squash commit message: {}\n\
            • Result: Single commit combining all changes\n\
            • Conflicts: Run with --strategy squash to check for conflicts",
            request.target_branch,
            request.feature_branch,
            commits.len(),
            squash_message
        ))
    }

    fn preview_rebase(&self, request: &StrategyRequest) -> Result<String> {
        let commits = self
            .integration
            .get_commit_range(&request.target_branch, &request.feature_branch)?;

        Ok(format!(
            "Rebase Strategy Preview:\n\
            • Target branch: {}\n\
            • Feature branch: {}\n\
            • Commits to rebase: {}\n\
            • Result: Individual commits replayed on target branch\n\
            • History: Linear history maintained\n\
            • Conflicts: Run with --strategy rebase to check for conflicts",
            request.target_branch,
            request.feature_branch,
            commits.len()
        ))
    }

    fn generate_merge_commit_message(
        &self,
        feature_branch: &str,
        target_branch: &str,
    ) -> Result<String> {
        let commits = self
            .integration
            .get_commit_range(target_branch, feature_branch)?;

        if commits.is_empty() {
            return Ok(format!(
                "Merge branch '{}' into {}",
                feature_branch, target_branch
            ));
        }

        let first_commit_msg = self.repo.get_commit_message(&commits[0])?;
        let summary = if commits.len() == 1 {
            first_commit_msg.lines().next().unwrap_or("").to_string()
        } else {
            format!(
                "{} (+{} more commits)",
                first_commit_msg.lines().next().unwrap_or(""),
                commits.len() - 1
            )
        };

        Ok(format!("Merge branch '{}': {}", feature_branch, summary))
    }

    fn generate_squash_commit_message(
        &self,
        feature_branch: &str,
        target_branch: &str,
    ) -> Result<String> {
        let commits = self
            .integration
            .get_commit_range(target_branch, feature_branch)?;

        if commits.is_empty() {
            return Ok(format!("Squash merge from {}", feature_branch));
        }

        if commits.len() == 1 {
            return self.repo.get_commit_message(&commits[0]);
        }

        let mut messages = Vec::new();
        for commit in &commits {
            let msg = self.repo.get_commit_message(commit)?;
            if let Some(first_line) = msg.lines().next() {
                messages.push(first_line.to_string());
            }
        }

        let summary = messages.join("; ");
        Ok(format!("Squash merge from {}: {}", feature_branch, summary))
    }

    pub fn continue_integration(&self) -> Result<StrategyResult> {
        if !self.integration.is_rebase_in_progress()? {
            return Err(ParaError::git_operation(
                "No integration in progress".to_string(),
            ));
        }

        let conflicted_files = self.integration.get_conflicted_files()?;
        if !conflicted_files.is_empty() {
            return Err(ParaError::git_operation(format!(
                "Conflicts still exist in {} files. Resolve them first.",
                conflicted_files.len()
            )));
        }

        self.integration.stage_resolved_files()?;

        match self.integration.continue_rebase() {
            Ok(()) => Ok(StrategyResult::Success {
                final_branch: self.repo.get_current_branch()?,
            }),
            Err(e) => Ok(StrategyResult::Failed {
                error: format!("Failed to continue integration: {}", e),
            }),
        }
    }

    /// Handle integration from worktree using change extraction
    fn execute_worktree_strategy(&self, request: &StrategyRequest) -> Result<StrategyResult> {
        println!("🔄 Integrating from worktree using change extraction...");

        match self.integration.integrate_from_worktree(
            &request.feature_branch,
            &request.target_branch,
            request.commit_message.as_deref(),
        ) {
            Ok(()) => {
                println!("✅ Successfully integrated changes to main repository");
                Ok(StrategyResult::Success {
                    final_branch: request.target_branch.clone(),
                })
            }
            Err(e) => {
                // Check if it's a conflict error from git am
                if e.to_string().contains("patch does not apply")
                    || e.to_string().contains("Failed to apply patches")
                {
                    println!("⚠️  Integration conflicts detected");
                    Ok(StrategyResult::ConflictsPending {
                        conflicted_files: vec![], // TODO: Extract actual conflicted files from git am output
                    })
                } else {
                    Ok(StrategyResult::Failed {
                        error: format!("Worktree integration failed: {}", e),
                    })
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::git::branch::BranchManager;
    use std::fs;
    use std::process::Command;
    use tempfile::TempDir;

    fn setup_test_repo() -> (TempDir, GitRepository) {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let repo_path = temp_dir.path();

        Command::new("git")
            .current_dir(repo_path)
            .args(["init", "--initial-branch=main"])
            .status()
            .expect("Failed to init git repo");

        Command::new("git")
            .current_dir(repo_path)
            .args(["config", "user.name", "Test User"])
            .status()
            .expect("Failed to set git user name");

        Command::new("git")
            .current_dir(repo_path)
            .args(["config", "user.email", "test@example.com"])
            .status()
            .expect("Failed to set git user email");

        fs::write(repo_path.join("README.md"), "# Test Repository")
            .expect("Failed to write README");

        Command::new("git")
            .current_dir(repo_path)
            .args(["add", "README.md"])
            .status()
            .expect("Failed to add README");

        Command::new("git")
            .current_dir(repo_path)
            .args(["commit", "-m", "Initial commit"])
            .status()
            .expect("Failed to commit README");

        let repo = GitRepository::discover_from(repo_path).expect("Failed to discover repo");
        (temp_dir, repo)
    }

    #[test]
    fn test_preview_merge_strategy() {
        let (temp_dir, repo) = setup_test_repo();
        let strategy_manager = StrategyManager::new(&repo);
        let branch_manager = BranchManager::new(&repo);

        let main_branch = repo
            .get_current_branch()
            .expect("Failed to get current branch");

        branch_manager
            .create_branch("feature", &main_branch)
            .expect("Failed to create feature branch");

        fs::write(temp_dir.path().join("feature.txt"), "New feature")
            .expect("Failed to write feature file");
        repo.stage_all_changes().expect("Failed to stage changes");
        repo.commit("Add feature").expect("Failed to commit");

        let request = StrategyRequest {
            feature_branch: "feature".to_string(),
            target_branch: main_branch.clone(),
            strategy: IntegrationStrategy::Merge,
            dry_run: true,
            commit_message: None,
        };

        let result = strategy_manager
            .execute_strategy(request)
            .expect("Failed to preview merge");

        match result {
            StrategyResult::DryRun { preview } => {
                assert!(preview.contains("Merge Strategy Preview"));
                assert!(preview.contains("Commits to merge: 1"));
            }
            _ => panic!("Expected dry run result"),
        }
    }

    #[test]
    fn test_preview_squash_strategy() {
        let (temp_dir, repo) = setup_test_repo();
        let strategy_manager = StrategyManager::new(&repo);
        let branch_manager = BranchManager::new(&repo);

        let main_branch = repo
            .get_current_branch()
            .expect("Failed to get current branch");

        branch_manager
            .create_branch("feature", &main_branch)
            .expect("Failed to create feature branch");

        fs::write(temp_dir.path().join("file1.txt"), "First change")
            .expect("Failed to write file1");
        repo.stage_all_changes().expect("Failed to stage changes");
        repo.commit("First commit").expect("Failed to commit");

        fs::write(temp_dir.path().join("file2.txt"), "Second change")
            .expect("Failed to write file2");
        repo.stage_all_changes().expect("Failed to stage changes");
        repo.commit("Second commit").expect("Failed to commit");

        let request = StrategyRequest {
            feature_branch: "feature".to_string(),
            target_branch: main_branch.clone(),
            strategy: IntegrationStrategy::Squash,
            dry_run: true,
            commit_message: None,
        };

        let result = strategy_manager
            .execute_strategy(request)
            .expect("Failed to preview squash");

        match result {
            StrategyResult::DryRun { preview } => {
                assert!(preview.contains("Squash Strategy Preview"));
                assert!(preview.contains("Commits to squash: 2"));
            }
            _ => panic!("Expected dry run result"),
        }
    }
}
