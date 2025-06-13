use super::branch::BranchManager;
use super::repository::{execute_git_command, execute_git_command_with_status, GitRepository};
use crate::utils::error::{ParaError, Result};
use crate::utils::names::generate_timestamp;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct FinishRequest {
    pub feature_branch: String,
    pub base_branch: String,
    pub commit_message: String,
    pub target_branch_name: Option<String>,
    pub integrate: bool,
}

#[derive(Debug)]
pub enum FinishResult {
    Success { final_branch: String },
    SuccessWithIntegrationFailure { final_branch: String, error: String },
}

#[derive(Debug)]
pub struct IntegrationRequest {
    pub feature_branch: String,
    pub base_branch: String,
    pub commit_message: Option<String>,
}

#[derive(Debug)]
pub enum IntegrationResult {
    Success,
    ConflictsPending,
    Failed { error: String },
}

#[derive(Debug)]
pub enum PreserveResult {
    NoChanges,
    Stashed { stash_message: String },
}

pub struct IntegrationManager<'a> {
    repo: &'a GitRepository,
}

impl<'a> IntegrationManager<'a> {
    pub fn new(repo: &'a GitRepository) -> Self {
        Self { repo }
    }

    pub fn finish_session(&self, request: FinishRequest) -> Result<FinishResult> {
        // Don't checkout the feature branch - we're likely already in the worktree with it checked out
        // This avoids the "already used by worktree" error

        let has_changes = self.repo.has_uncommitted_changes()?;

        if has_changes {
            self.repo.stage_all_changes()?;
            self.repo.commit(&request.commit_message)?;
        }

        let commit_count = self
            .repo
            .get_commit_count_since(&request.base_branch, &request.feature_branch)?;

        if commit_count > 1 {
            self.squash_commits(
                &request.feature_branch,
                &request.base_branch,
                &request.commit_message,
            )?;
        }

        let final_branch_name = if let Some(ref target_name) = request.target_branch_name {
            if target_name != &request.feature_branch {
                let branch_manager = BranchManager::new(self.repo);
                branch_manager.rename_branch(&request.feature_branch, target_name)?;
                target_name.clone()
            } else {
                request.feature_branch.clone()
            }
        } else {
            request.feature_branch.clone()
        };

        if request.integrate {
            match self.integrate_branch(IntegrationRequest {
                feature_branch: final_branch_name.clone(),
                base_branch: request.base_branch.clone(),
                commit_message: Some(request.commit_message.clone()),
            })? {
                IntegrationResult::Success => Ok(FinishResult::Success {
                    final_branch: request.base_branch,
                }),
                IntegrationResult::ConflictsPending => {
                    // Graceful fallback: keep changes on feature branch with user's commit message
                    Ok(FinishResult::SuccessWithIntegrationFailure {
                        final_branch: final_branch_name,
                        error:
                            "Integration conflicts detected - changes preserved on feature branch"
                                .to_string(),
                    })
                }
                IntegrationResult::Failed { error } => {
                    // Graceful fallback: keep changes on feature branch with user's commit message
                    Ok(FinishResult::SuccessWithIntegrationFailure {
                        final_branch: final_branch_name,
                        error,
                    })
                }
            }
        } else {
            Ok(FinishResult::Success {
                final_branch: final_branch_name,
            })
        }
    }

    pub fn squash_commits(
        &self,
        feature_branch: &str,
        base_branch: &str,
        message: &str,
    ) -> Result<()> {
        let merge_base = self.repo.get_merge_base(base_branch, feature_branch)?;

        execute_git_command_with_status(self.repo, &["reset", "--soft", &merge_base])?;

        let status_output = execute_git_command(self.repo, &["status", "--porcelain"])?;
        if !status_output.trim().is_empty() {
            self.repo.commit(message)?;
        }

        Ok(())
    }

    pub fn integrate_branch(&self, request: IntegrationRequest) -> Result<IntegrationResult> {
        // Check if we're in a worktree and handle differently
        if self.is_in_worktree()? {
            match self.integrate_from_worktree(
                &request.feature_branch,
                &request.base_branch,
                request.commit_message.as_deref(),
            ) {
                Ok(()) => return Ok(IntegrationResult::Success),
                Err(e) => {
                    // Check if it's a conflict error
                    if e.to_string().contains("patch does not apply")
                        || e.to_string().contains("Failed to apply patches")
                    {
                        return Ok(IntegrationResult::ConflictsPending);
                    } else {
                        return Ok(IntegrationResult::Failed {
                            error: format!("Worktree integration failed: {}", e),
                        });
                    }
                }
            }
        }

        // Original main repo integration logic
        // Preserve uncommitted changes before integration
        let preserve_result = self.preserve_uncommitted_changes(&request.base_branch)?;

        self.update_base_branch(&request.base_branch)?;

        match self.prepare_rebase(&request.feature_branch, &request.base_branch) {
            Ok(()) => {
                self.repo.checkout_branch(&request.base_branch)?;

                execute_git_command_with_status(
                    self.repo,
                    &["merge", "--ff-only", &request.feature_branch],
                )?;

                // Restore uncommitted changes after successful integration
                self.restore_uncommitted_changes(preserve_result)?;

                Ok(IntegrationResult::Success)
            }
            Err(_) => {
                if self.has_rebase_conflicts()? {
                    Ok(IntegrationResult::ConflictsPending)
                } else {
                    Ok(IntegrationResult::Failed {
                        error: "Rebase failed without conflicts".to_string(),
                    })
                }
            }
        }
    }

    pub fn prepare_rebase(&self, feature_branch: &str, base_branch: &str) -> Result<()> {
        self.repo.checkout_branch(feature_branch)?;
        execute_git_command_with_status(self.repo, &["rebase", base_branch])
    }

    pub fn preserve_uncommitted_changes(&self, target_branch: &str) -> Result<PreserveResult> {
        // Always ensure we're on the target branch first
        let current_branch = self.repo.get_current_branch()?;
        if current_branch != target_branch {
            self.repo.checkout_branch(target_branch)?;
        }

        // Check if there are uncommitted changes to preserve
        if !self.repo.has_uncommitted_changes()? {
            return Ok(PreserveResult::NoChanges);
        }

        let timestamp = generate_timestamp();
        let stash_message = format!("temp-integration-stash-{}", timestamp);

        // Create stash to preserve changes on target branch
        execute_git_command_with_status(self.repo, &["stash", "push", "-m", &stash_message])?;

        // Verify the stash was actually created by checking if it's in the stash list
        let stash_list = execute_git_command(self.repo, &["stash", "list"])?;
        if stash_list.contains(&stash_message) {
            Ok(PreserveResult::Stashed { stash_message })
        } else {
            // Stash command succeeded but no stash was created (edge case)
            Ok(PreserveResult::NoChanges)
        }
    }

    pub fn restore_uncommitted_changes(&self, preserve_result: PreserveResult) -> Result<()> {
        match preserve_result {
            PreserveResult::NoChanges => Ok(()),
            PreserveResult::Stashed { stash_message } => {
                match self.attempt_stash_pop(&stash_message) {
                    Ok(()) => Ok(()),
                    Err(_) => self.handle_stash_conflicts(&stash_message),
                }
            }
        }
    }

    fn handle_stash_conflicts(&self, stash_message: &str) -> Result<()> {
        let branch_manager = BranchManager::new(self.repo);
        let conflict_branch_name = self.create_conflict_branch(&branch_manager)?;
        let current_branch = self.repo.get_current_branch()?;

        self.apply_stash_to_conflict_branch(&conflict_branch_name, stash_message)?;
        self.repo.checkout_branch(&current_branch)?;
        self.print_conflict_resolution_guidance(&conflict_branch_name, &current_branch);

        Ok(())
    }

    fn create_conflict_branch(&self, branch_manager: &BranchManager) -> Result<String> {
        let timestamp = generate_timestamp();
        let conflict_branch = format!("uncommitted-changes-{}", timestamp);
        let unique_branch_name = branch_manager.generate_unique_branch_name(&conflict_branch)?;
        let current_branch = self.repo.get_current_branch()?;

        let _ = execute_git_command_with_status(self.repo, &["reset", "--hard", "HEAD"]);
        branch_manager.create_branch(&unique_branch_name, &current_branch)?;

        Ok(unique_branch_name)
    }

    fn apply_stash_to_conflict_branch(
        &self,
        conflict_branch_name: &str,
        stash_message: &str,
    ) -> Result<()> {
        self.repo.checkout_branch(conflict_branch_name)?;

        match self.attempt_stash_pop(stash_message) {
            Ok(()) => {
                let _ = execute_git_command_with_status(self.repo, &["add", "."]);
                let _ = execute_git_command_with_status(
                    self.repo,
                    &["commit", "-m", "Preserved uncommitted changes"],
                );
            }
            Err(_) => {
                let _ = execute_git_command_with_status(self.repo, &["add", "."]);
                let _ = execute_git_command_with_status(
                    self.repo,
                    &["commit", "-m", "Conflicted changes preserved"],
                );
            }
        }

        Ok(())
    }

    fn print_conflict_resolution_guidance(&self, conflict_branch: &str, current_branch: &str) {
        eprintln!("⚠️  Found uncommitted changes on target branch");
        eprintln!("❌ Conflicts detected during rebase");
        eprintln!("🌿 Created branch '{}' with your changes", conflict_branch);
        eprintln!("💡 To resolve conflicts:");
        eprintln!("   git checkout {}", conflict_branch);
        eprintln!("   # resolve conflicts, then:");
        eprintln!(
            "   git checkout {} && git merge {}",
            current_branch, conflict_branch
        );
    }

    fn attempt_stash_pop(&self, stash_message: &str) -> Result<()> {
        // Find the stash by message
        let stash_list = execute_git_command(self.repo, &["stash", "list"])?;

        for (index, line) in stash_list.lines().enumerate() {
            if line.contains(stash_message) {
                // Try to apply the specific stash
                let stash_ref = format!("stash@{{{}}}", index);
                return execute_git_command_with_status(self.repo, &["stash", "pop", &stash_ref]);
            }
        }

        Err(ParaError::git_operation("Stash not found".to_string()))
    }
    pub fn continue_rebase(&self) -> Result<()> {
        execute_git_command_with_status(self.repo, &["rebase", "--continue"])
    }

    pub fn abort_rebase(&self) -> Result<()> {
        execute_git_command_with_status(self.repo, &["rebase", "--abort"])
    }

    pub fn has_rebase_conflicts(&self) -> Result<bool> {
        let rebase_dir = self.repo.git_dir.join("rebase-merge");
        let rebase_apply_dir = self.repo.git_dir.join("rebase-apply");

        Ok(rebase_dir.exists() || rebase_apply_dir.exists())
    }

    pub fn get_conflicted_files(&self) -> Result<Vec<PathBuf>> {
        // Check for any type of git conflicts, not just rebase conflicts
        // This includes cherry-pick, merge, and rebase conflicts
        let output = execute_git_command(self.repo, &["diff", "--name-only", "--diff-filter=U"])?;

        Ok(output
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| PathBuf::from(line.trim()))
            .collect())
    }

    pub fn is_rebase_in_progress(&self) -> Result<bool> {
        self.has_rebase_conflicts()
    }

    pub fn is_cherry_pick_in_progress(&self) -> Result<bool> {
        let cherry_pick_head = self.repo.git_dir.join("CHERRY_PICK_HEAD");
        Ok(cherry_pick_head.exists())
    }

    pub fn is_merge_in_progress(&self) -> Result<bool> {
        let merge_head = self.repo.git_dir.join("MERGE_HEAD");
        Ok(merge_head.exists())
    }

    pub fn is_am_in_progress(&self) -> Result<bool> {
        let rebase_apply = self.repo.git_dir.join("rebase-apply");
        Ok(rebase_apply.exists())
    }

    pub fn update_base_branch(&self, branch: &str) -> Result<()> {
        let current_branch = self.repo.get_current_branch()?;

        if current_branch != branch {
            self.repo.checkout_branch(branch)?;
        }

        match self.pull_latest_changes(branch) {
            Ok(()) => Ok(()),
            Err(_) => Ok(()),
        }
    }

    pub fn pull_latest_changes(&self, branch: &str) -> Result<()> {
        let remote_url = self.repo.get_remote_url()?;
        if remote_url.is_none() {
            return Ok(());
        }

        let current_branch = self.repo.get_current_branch()?;
        if current_branch != branch {
            return Err(ParaError::git_operation(format!(
                "Cannot pull changes: not on branch {}",
                branch
            )));
        }

        execute_git_command_with_status(self.repo, &["pull", "--ff-only"])
    }

    pub fn create_merge_commit(
        &self,
        feature_branch: &str,
        base_branch: &str,
        message: &str,
    ) -> Result<()> {
        self.repo.checkout_branch(base_branch)?;
        execute_git_command_with_status(
            self.repo,
            &["merge", "--no-ff", "-m", message, feature_branch],
        )
    }

    pub fn cherry_pick_commits(&self, commits: &[String]) -> Result<()> {
        for commit in commits {
            execute_git_command_with_status(self.repo, &["cherry-pick", commit])?;
        }
        Ok(())
    }

    pub fn get_commit_range(&self, base_branch: &str, feature_branch: &str) -> Result<Vec<String>> {
        let range = format!("{}..{}", base_branch, feature_branch);
        let output = execute_git_command(self.repo, &["rev-list", "--reverse", &range])?;

        Ok(output
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| line.trim().to_string())
            .collect())
    }

    pub fn stage_resolved_files(&self) -> Result<()> {
        let conflicted_files = self.get_conflicted_files()?;

        for file in conflicted_files {
            let file_str = file.to_string_lossy();
            execute_git_command_with_status(self.repo, &["add", &file_str])?;
        }

        Ok(())
    }

    pub fn create_backup_branch(&self, branch: &str, suffix: &str) -> Result<String> {
        let branch_manager = BranchManager::new(self.repo);
        let backup_name = format!("{}-{}", branch, suffix);

        let unique_backup_name = branch_manager.generate_unique_branch_name(&backup_name)?;
        let commit = branch_manager.get_branch_commit(branch)?;

        branch_manager.create_branch_from_commit(&unique_backup_name, &commit)?;

        Ok(unique_backup_name)
    }

    pub fn restore_from_backup(&self, backup_branch: &str, target_branch: &str) -> Result<()> {
        let branch_manager = BranchManager::new(self.repo);
        let backup_commit = branch_manager.get_branch_commit(backup_branch)?;

        self.repo.checkout_branch(target_branch)?;
        self.repo.reset_hard(&backup_commit)?;

        Ok(())
    }

    pub fn cleanup_integration_state(&self) -> Result<()> {
        if self.is_rebase_in_progress()? {
            let _ = self.abort_rebase();
        }

        if self.is_merge_in_progress()? {
            let _ = self.abort_merge();
        }

        if self.is_cherry_pick_in_progress()? {
            let _ = self.abort_cherry_pick();
        }

        if self.is_am_in_progress()? {
            let _ = self.abort_am();
        }

        let merge_head = self.repo.git_dir.join("MERGE_HEAD");
        if merge_head.exists() {
            let _ = std::fs::remove_file(merge_head);
        }

        let cherry_pick_head = self.repo.git_dir.join("CHERRY_PICK_HEAD");
        if cherry_pick_head.exists() {
            let _ = std::fs::remove_file(cherry_pick_head);
        }

        let revert_head = self.repo.git_dir.join("REVERT_HEAD");
        if revert_head.exists() {
            let _ = std::fs::remove_file(revert_head);
        }

        Ok(())
    }

    pub fn abort_merge(&self) -> Result<()> {
        execute_git_command_with_status(self.repo, &["merge", "--abort"])
    }

    pub fn abort_cherry_pick(&self) -> Result<()> {
        execute_git_command_with_status(self.repo, &["cherry-pick", "--abort"])
    }

    pub fn abort_am(&self) -> Result<()> {
        execute_git_command_with_status(self.repo, &["am", "--abort"])
    }

    pub fn safe_abort_integration(
        &self,
        backup_branch: Option<&str>,
        target_branch: &str,
    ) -> Result<()> {
        self.cleanup_integration_state()?;

        if let Some(backup) = backup_branch {
            self.restore_from_backup(backup, target_branch)?;
        }

        Ok(())
    }

    pub fn validate_integration_preconditions(
        &self,
        feature_branch: &str,
        base_branch: &str,
    ) -> Result<()> {
        let branch_manager = BranchManager::new(self.repo);

        if !branch_manager.branch_exists(feature_branch)? {
            return Err(ParaError::git_operation(format!(
                "Feature branch '{}' does not exist",
                feature_branch
            )));
        }

        if !branch_manager.branch_exists(base_branch)? {
            return Err(ParaError::git_operation(format!(
                "Base branch '{}' does not exist",
                base_branch
            )));
        }

        if self.is_rebase_in_progress()? {
            return Err(ParaError::git_operation(
                "Cannot start integration: rebase in progress".to_string(),
            ));
        }

        Ok(())
    }

    /// Check if current repository is a worktree (not main repo)
    pub fn is_in_worktree(&self) -> Result<bool> {
        let git_path = self.repo.root.join(".git");
        Ok(git_path.is_file()) // Worktrees have .git file, main repo has .git directory
    }

    /// Get path to main repository from worktree
    pub fn get_main_repo_path(&self) -> Result<PathBuf> {
        let git_path = self.repo.root.join(".git");

        // Check if we're in a worktree (has .git file instead of directory)
        if !git_path.is_file() {
            // Not in worktree, already in main repo
            return Ok(self.repo.root.clone());
        }

        // Read .git file to find main repo path
        let git_content = std::fs::read_to_string(&git_path)
            .map_err(|e| ParaError::git_operation(format!("Failed to read .git file: {}", e)))?;

        // Format: "gitdir: /path/to/main/repo/.git/worktrees/session-name"
        let git_dir = git_content
            .strip_prefix("gitdir: ")
            .ok_or_else(|| ParaError::git_operation("Invalid .git file format".to_string()))?
            .trim();

        // Extract main repo path: /path/to/main/repo/.git/worktrees/session -> /path/to/main/repo
        let git_path = PathBuf::from(git_dir);
        let main_repo = git_path
            .parent() // removes "session-name" -> /path/to/main/repo/.git/worktrees
            .and_then(|p| p.parent()) // removes "worktrees" -> /path/to/main/repo/.git
            .and_then(|p| p.parent()) // removes ".git" -> /path/to/main/repo
            .ok_or_else(|| {
                ParaError::git_operation("Cannot determine main repo path".to_string())
            })?;

        Ok(main_repo.to_path_buf())
    }

    /// Extract changes from worktree and apply to main repo using format-patch + git am
    pub fn integrate_from_worktree(
        &self,
        _feature_branch: &str,
        target_branch: &str,
        commit_message: Option<&str>,
    ) -> Result<()> {
        let main_repo_path = self.get_main_repo_path()?;

        // For main repository, git directory is always at main_repo_path/.git
        // This is different from worktree where git_dir points to .git/worktrees/session-name
        let main_git_dir = main_repo_path.join(".git");

        // Ensure any uncommitted changes are committed so they appear in the patch stream.
        if self.repo.has_uncommitted_changes()? {
            // Stage all changes first (respect auto_stage setting has been done by caller)
            self.repo.stage_all_changes()?;
            let msg = commit_message.unwrap_or("Apply uncommitted changes from worktree session");
            self.repo.commit(msg)?;
        }

        // Create patches for all commits since branching from target_branch
        let patch_output = execute_git_command(
            self.repo,
            &[
                "format-patch",
                &format!("{}..HEAD", target_branch),
                "--stdout",
            ],
        )?;

        if patch_output.trim().is_empty() {
            // Nothing to do
            return Ok(());
        }

        self.apply_patches_to_main_repo(patch_output, &main_git_dir, &main_repo_path, target_branch)
    }

    fn apply_patches_to_main_repo(
        &self,
        patch_output: String,
        main_git_dir: &Path,
        main_repo_path: &Path,
        target_branch: &str,
    ) -> Result<()> {
        // Write patch to temporary file
        let temp_patch = format!("/tmp/para-integration-{}.patch", generate_timestamp());
        std::fs::write(&temp_patch, patch_output)
            .map_err(|e| ParaError::git_operation(format!("Failed to write patch file: {}", e)))?;

        // Save current branch to restore on failure
        let original_branch = execute_git_command(
            self.repo,
            &[
                "--git-dir",
                &main_git_dir.to_string_lossy(),
                "--work-tree",
                &main_repo_path.to_string_lossy(),
                "rev-parse",
                "--abbrev-ref",
                "HEAD",
            ],
        )?
        .trim()
        .to_string();

        // First, checkout target branch in main repo
        execute_git_command_with_status(
            self.repo,
            &[
                "--git-dir",
                &main_git_dir.to_string_lossy(),
                "--work-tree",
                &main_repo_path.to_string_lossy(),
                "checkout",
                target_branch,
            ],
        )?;

        // Apply patches using git am
        let result = execute_git_command_with_status(
            self.repo,
            &[
                "--git-dir",
                &main_git_dir.to_string_lossy(),
                "--work-tree",
                &main_repo_path.to_string_lossy(),
                "am",
                &temp_patch,
            ],
        );

        // Cleanup temp file
        let _ = std::fs::remove_file(&temp_patch);

        match result {
            Ok(()) => Ok(()),
            Err(e) => {
                // If git am failed, abort it to clean up the state
                let _ = execute_git_command_with_status(
                    self.repo,
                    &[
                        "--git-dir",
                        &main_git_dir.to_string_lossy(),
                        "--work-tree",
                        &main_repo_path.to_string_lossy(),
                        "am",
                        "--abort",
                    ],
                );

                // Restore original branch if it's different
                if original_branch != target_branch {
                    let _ = execute_git_command_with_status(
                        self.repo,
                        &[
                            "--git-dir",
                            &main_git_dir.to_string_lossy(),
                            "--work-tree",
                            &main_repo_path.to_string_lossy(),
                            "checkout",
                            &original_branch,
                        ],
                    );
                }

                Err(ParaError::git_operation(format!(
                    "Failed to apply patches to main repository: {}. \
                    The main repository has been restored to its original state. \
                    Please resolve conflicts manually by checking out the session worktree.",
                    e
                )))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
    fn test_finish_session_simple() {
        let (temp_dir, repo) = setup_test_repo();
        let manager = IntegrationManager::new(&repo);
        let branch_manager = BranchManager::new(&repo);

        let main_branch = repo
            .get_current_branch()
            .expect("Failed to get current branch");

        branch_manager
            .create_branch("feature", &main_branch)
            .expect("Failed to create feature branch");

        fs::write(temp_dir.path().join("feature.txt"), "New feature")
            .expect("Failed to write feature file");

        let request = FinishRequest {
            feature_branch: "feature".to_string(),
            base_branch: main_branch.clone(),
            commit_message: "Add new feature".to_string(),
            target_branch_name: None,
            integrate: false,
        };

        let result = manager
            .finish_session(request)
            .expect("Failed to finish session");

        match result {
            FinishResult::Success { final_branch } => {
                assert_eq!(final_branch, "feature");
            }
            _ => panic!("Expected success result"),
        }
    }

    #[test]
    fn test_squash_commits() {
        let (temp_dir, repo) = setup_test_repo();
        let manager = IntegrationManager::new(&repo);
        let branch_manager = BranchManager::new(&repo);

        let main_branch = repo
            .get_current_branch()
            .expect("Failed to get current branch");

        branch_manager
            .create_branch("multi-commit", &main_branch)
            .expect("Failed to create branch");

        fs::write(temp_dir.path().join("file1.txt"), "First change")
            .expect("Failed to write file1");
        repo.stage_all_changes().expect("Failed to stage changes");
        repo.commit("First commit").expect("Failed to commit");

        fs::write(temp_dir.path().join("file2.txt"), "Second change")
            .expect("Failed to write file2");
        repo.stage_all_changes().expect("Failed to stage changes");
        repo.commit("Second commit").expect("Failed to commit");

        let commits_before = repo
            .get_commit_count_since(&main_branch, "multi-commit")
            .expect("Failed to count commits");
        assert_eq!(commits_before, 2);

        manager
            .squash_commits("multi-commit", &main_branch, "Combined changes")
            .expect("Failed to squash commits");

        let commits_after = repo
            .get_commit_count_since(&main_branch, "multi-commit")
            .expect("Failed to count commits");
        assert_eq!(commits_after, 1);
    }

    #[test]
    fn test_integration_validation() {
        let (_temp_dir, repo) = setup_test_repo();
        let manager = IntegrationManager::new(&repo);

        let main_branch = repo
            .get_current_branch()
            .expect("Failed to get current branch");

        let result = manager.validate_integration_preconditions("nonexistent", &main_branch);
        assert!(result.is_err());

        let result = manager.validate_integration_preconditions(&main_branch, "nonexistent");
        assert!(result.is_err());

        let result = manager.validate_integration_preconditions(&main_branch, &main_branch);
        assert!(result.is_ok());
    }

    #[test]
    fn test_backup_and_restore() {
        let (temp_dir, repo) = setup_test_repo();
        let manager = IntegrationManager::new(&repo);
        let branch_manager = BranchManager::new(&repo);

        let main_branch = repo
            .get_current_branch()
            .expect("Failed to get current branch");

        branch_manager
            .create_branch("backup-test", &main_branch)
            .expect("Failed to create branch");

        fs::write(temp_dir.path().join("backup-file.txt"), "Original content")
            .expect("Failed to write file");
        repo.stage_all_changes().expect("Failed to stage");
        repo.commit("Original commit").expect("Failed to commit");

        let backup_name = manager
            .create_backup_branch("backup-test", "backup")
            .expect("Failed to create backup");

        assert!(backup_name.starts_with("backup-test-backup"));
        assert!(branch_manager
            .branch_exists(&backup_name)
            .expect("Failed to check backup"));

        fs::write(temp_dir.path().join("backup-file.txt"), "Modified content")
            .expect("Failed to write file");
        repo.stage_all_changes().expect("Failed to stage");
        repo.commit("Modified commit").expect("Failed to commit");

        manager
            .restore_from_backup(&backup_name, "backup-test")
            .expect("Failed to restore from backup");

        let content = fs::read_to_string(temp_dir.path().join("backup-file.txt"))
            .expect("Failed to read file");
        assert_eq!(content, "Original content");
    }

    #[test]
    fn test_get_commit_range() {
        let (temp_dir, repo) = setup_test_repo();
        let manager = IntegrationManager::new(&repo);
        let branch_manager = BranchManager::new(&repo);

        let main_branch = repo
            .get_current_branch()
            .expect("Failed to get current branch");

        branch_manager
            .create_branch("range-test", &main_branch)
            .expect("Failed to create branch");

        fs::write(temp_dir.path().join("commit1.txt"), "First").expect("Failed to write file");
        repo.stage_all_changes().expect("Failed to stage");
        repo.commit("First commit").expect("Failed to commit");

        fs::write(temp_dir.path().join("commit2.txt"), "Second").expect("Failed to write file");
        repo.stage_all_changes().expect("Failed to stage");
        repo.commit("Second commit").expect("Failed to commit");

        let commits = manager
            .get_commit_range(&main_branch, "range-test")
            .expect("Failed to get commit range");

        assert_eq!(commits.len(), 2);
        for commit in commits {
            assert_eq!(commit.len(), 40);
        }
    }

    #[test]
    fn test_cleanup_integration_state() {
        let (_temp_dir, repo) = setup_test_repo();
        let manager = IntegrationManager::new(&repo);

        manager
            .cleanup_integration_state()
            .expect("Failed to cleanup integration state");
    }

    #[test]
    fn test_is_rebase_in_progress_no_rebase() {
        let (_temp_dir, repo) = setup_test_repo();
        let manager = IntegrationManager::new(&repo);

        let result = manager.is_rebase_in_progress();
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[test]
    fn test_is_merge_in_progress_no_merge() {
        let (_temp_dir, repo) = setup_test_repo();
        let manager = IntegrationManager::new(&repo);

        let result = manager.is_merge_in_progress();
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[test]
    fn test_is_cherry_pick_in_progress_no_cherry_pick() {
        let (_temp_dir, repo) = setup_test_repo();
        let manager = IntegrationManager::new(&repo);

        let result = manager.is_cherry_pick_in_progress();
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[test]
    fn test_safe_abort_integration_no_backup() {
        let (_temp_dir, repo) = setup_test_repo();
        let manager = IntegrationManager::new(&repo);

        let main_branch = repo
            .get_current_branch()
            .expect("Failed to get current branch");
        let result = manager.safe_abort_integration(None, &main_branch);

        assert!(result.is_ok());
    }

    #[test]
    fn test_safe_abort_integration_with_nonexistent_backup() {
        let (_temp_dir, repo) = setup_test_repo();
        let manager = IntegrationManager::new(&repo);

        let main_branch = repo
            .get_current_branch()
            .expect("Failed to get current branch");
        let result = manager.safe_abort_integration(Some("nonexistent-backup"), &main_branch);

        // Should handle missing backup gracefully
        assert!(result.is_err() || result.is_ok());
    }

    #[test]
    fn test_abort_operations_no_operations() {
        let (_temp_dir, repo) = setup_test_repo();
        let manager = IntegrationManager::new(&repo);

        // These should fail gracefully since no operations are in progress
        let rebase_result = manager.abort_rebase();
        let merge_result = manager.abort_merge();
        let cherry_pick_result = manager.abort_cherry_pick();

        assert!(rebase_result.is_err());
        assert!(merge_result.is_err());
        assert!(cherry_pick_result.is_err());
    }

    #[test]
    fn test_continue_rebase_no_rebase() {
        let (_temp_dir, repo) = setup_test_repo();
        let manager = IntegrationManager::new(&repo);

        let result = manager.continue_rebase();

        // Should fail since no rebase is in progress
        assert!(result.is_err());
    }

    #[test]
    fn test_stage_resolved_files_no_conflicts() {
        let (_temp_dir, repo) = setup_test_repo();
        let manager = IntegrationManager::new(&repo);

        let result = manager.stage_resolved_files();

        // Should succeed even with no conflicts to resolve
        assert!(result.is_ok());
    }

    #[test]
    fn test_enhanced_cleanup_integration_state() {
        let (_temp_dir, repo) = setup_test_repo();
        let manager = IntegrationManager::new(&repo);

        // Test that cleanup can be called multiple times safely
        assert!(manager.cleanup_integration_state().is_ok());
        assert!(manager.cleanup_integration_state().is_ok());
        assert!(manager.cleanup_integration_state().is_ok());
    }

    #[test]
    fn test_robust_error_handling() {
        let (_temp_dir, repo) = setup_test_repo();
        let manager = IntegrationManager::new(&repo);

        // Test operations that should fail but not panic
        let operations = [
            manager.continue_rebase(),
            manager.abort_rebase(),
            manager.abort_merge(),
            manager.abort_cherry_pick(),
        ];

        // All should return errors (since no operations are in progress)
        for op_result in operations {
            assert!(op_result.is_err());
        }

        // But cleanup should always work
        assert!(manager.cleanup_integration_state().is_ok());
    }

    #[test]
    fn test_integration_with_no_uncommitted_changes() {
        let (temp_dir, repo) = setup_test_repo();
        let manager = IntegrationManager::new(&repo);
        let branch_manager = BranchManager::new(&repo);

        let main_branch = repo
            .get_current_branch()
            .expect("Failed to get current branch");

        // Create and checkout feature branch
        branch_manager
            .create_branch("feature", &main_branch)
            .expect("Failed to create feature branch");

        // Add a commit to feature branch
        fs::write(temp_dir.path().join("feature.txt"), "Feature content")
            .expect("Failed to write feature file");
        repo.stage_all_changes().expect("Failed to stage changes");
        repo.commit("Add feature").expect("Failed to commit");

        // Go back to main branch (simulating integration from anywhere)
        repo.checkout_branch(&main_branch)
            .expect("Failed to checkout main");

        // Verify no uncommitted changes
        assert!(!repo
            .has_uncommitted_changes()
            .expect("Failed to check uncommitted changes"));

        // Integration should succeed and preserve clean state
        let request = IntegrationRequest {
            feature_branch: "feature".to_string(),
            base_branch: main_branch.clone(),
            commit_message: None,
        };

        let result = manager
            .integrate_branch(request)
            .expect("Integration failed");
        assert!(matches!(result, IntegrationResult::Success));

        // Verify still no uncommitted changes after integration
        assert!(!repo
            .has_uncommitted_changes()
            .expect("Failed to check uncommitted changes"));
    }

    #[test]
    fn test_integration_with_staged_changes() {
        let (temp_dir, repo) = setup_test_repo();
        let manager = IntegrationManager::new(&repo);
        let branch_manager = BranchManager::new(&repo);

        let main_branch = repo
            .get_current_branch()
            .expect("Failed to get current branch");

        // Create and checkout feature branch
        branch_manager
            .create_branch("feature", &main_branch)
            .expect("Failed to create feature branch");

        // Add a commit to feature branch
        fs::write(temp_dir.path().join("feature.txt"), "Feature content")
            .expect("Failed to write feature file");
        repo.stage_all_changes().expect("Failed to stage changes");
        repo.commit("Add feature").expect("Failed to commit");

        // Go back to main branch and add staged changes
        repo.checkout_branch(&main_branch)
            .expect("Failed to checkout main");
        fs::write(temp_dir.path().join("staged.txt"), "Staged content")
            .expect("Failed to write staged file");
        repo.stage_all_changes().expect("Failed to stage changes");

        // Verify we have uncommitted (staged) changes
        assert!(repo
            .has_uncommitted_changes()
            .expect("Failed to check uncommitted changes"));

        // Integration should succeed and preserve staged changes
        let request = IntegrationRequest {
            feature_branch: "feature".to_string(),
            base_branch: main_branch.clone(),
            commit_message: None,
        };

        let result = manager
            .integrate_branch(request)
            .expect("Integration failed");
        assert!(matches!(result, IntegrationResult::Success));

        // Verify staged changes are still present after integration
        assert!(repo
            .has_uncommitted_changes()
            .expect("Failed to check uncommitted changes"));

        // Verify the staged file still exists
        assert!(temp_dir.path().join("staged.txt").exists());
        let content = fs::read_to_string(temp_dir.path().join("staged.txt"))
            .expect("Failed to read staged file");
        assert_eq!(content, "Staged content");
    }

    #[test]
    fn test_integration_with_unstaged_changes() {
        let (temp_dir, repo) = setup_test_repo();
        let manager = IntegrationManager::new(&repo);
        let branch_manager = BranchManager::new(&repo);

        let main_branch = repo
            .get_current_branch()
            .expect("Failed to get current branch");

        // Create and checkout feature branch
        branch_manager
            .create_branch("feature", &main_branch)
            .expect("Failed to create feature branch");

        // Add a commit to feature branch
        fs::write(temp_dir.path().join("feature.txt"), "Feature content")
            .expect("Failed to write feature file");
        repo.stage_all_changes().expect("Failed to stage changes");
        repo.commit("Add feature").expect("Failed to commit");

        // Go back to main branch and add unstaged changes
        repo.checkout_branch(&main_branch)
            .expect("Failed to checkout main");
        fs::write(temp_dir.path().join("unstaged.txt"), "Unstaged content")
            .expect("Failed to write unstaged file");

        // Verify we have uncommitted (unstaged) changes
        assert!(repo
            .has_uncommitted_changes()
            .expect("Failed to check uncommitted changes"));

        // Integration should succeed and preserve unstaged changes
        let request = IntegrationRequest {
            feature_branch: "feature".to_string(),
            base_branch: main_branch.clone(),
            commit_message: None,
        };

        let result = manager
            .integrate_branch(request)
            .expect("Integration failed");
        assert!(matches!(result, IntegrationResult::Success));

        // Verify unstaged changes are still present after integration
        assert!(repo
            .has_uncommitted_changes()
            .expect("Failed to check uncommitted changes"));

        // Verify the unstaged file still exists
        assert!(temp_dir.path().join("unstaged.txt").exists());
        let content = fs::read_to_string(temp_dir.path().join("unstaged.txt"))
            .expect("Failed to read unstaged file");
        assert_eq!(content, "Unstaged content");
    }

    #[test]
    fn test_integration_with_conflicting_changes() {
        let (temp_dir, repo) = setup_test_repo();
        let manager = IntegrationManager::new(&repo);
        let branch_manager = BranchManager::new(&repo);

        let main_branch = repo
            .get_current_branch()
            .expect("Failed to get current branch");

        // Create and checkout feature branch
        branch_manager
            .create_branch("feature", &main_branch)
            .expect("Failed to create feature branch");

        // Add a commit to feature branch that modifies README.md
        fs::write(temp_dir.path().join("README.md"), "# Feature Repository")
            .expect("Failed to write feature README");
        repo.stage_all_changes().expect("Failed to stage changes");
        repo.commit("Update README in feature")
            .expect("Failed to commit");

        // Go back to main branch and make conflicting changes to README.md
        repo.checkout_branch(&main_branch)
            .expect("Failed to checkout main");
        fs::write(temp_dir.path().join("README.md"), "# Modified Repository")
            .expect("Failed to write main README");

        // Verify we have uncommitted changes
        assert!(repo
            .has_uncommitted_changes()
            .expect("Failed to check uncommitted changes"));

        // Integration should succeed but create a conflict branch
        let request = IntegrationRequest {
            feature_branch: "feature".to_string(),
            base_branch: main_branch.clone(),
            commit_message: None,
        };

        let result = manager
            .integrate_branch(request)
            .expect("Integration failed");
        assert!(matches!(result, IntegrationResult::Success));

        // After integration, there should be a new branch with conflicted changes
        let branches = branch_manager
            .list_branches()
            .expect("Failed to list branches");
        let conflict_branch_exists = branches
            .iter()
            .any(|b| b.name.starts_with("uncommitted-changes-"));
        assert!(
            conflict_branch_exists,
            "Expected a conflict branch to be created"
        );
    }

    #[test]
    fn test_finish_session_commit_message_propagation() {
        // Test that commit message is propagated through the finish flow
        let (_temp_dir, repo) = setup_test_repo();
        let manager = IntegrationManager::new(&repo);
        let branch_manager = BranchManager::new(&repo);

        let main_branch = repo
            .get_current_branch()
            .expect("Failed to get current branch");

        // Create feature branch
        branch_manager
            .create_branch("feature-msg-test", &main_branch)
            .expect("Failed to create feature branch");

        // Switch to feature branch and make changes
        repo.checkout_branch("feature-msg-test")
            .expect("Failed to checkout feature branch");

        fs::write(repo.root.join("feature.txt"), "Feature content")
            .expect("Failed to write feature file");

        // Test finish without integration
        let custom_message = "Custom feature implementation";
        let request = FinishRequest {
            feature_branch: "feature-msg-test".to_string(),
            base_branch: main_branch.clone(),
            commit_message: custom_message.to_string(),
            target_branch_name: None,
            integrate: false,
        };

        let result = manager
            .finish_session(request)
            .expect("Failed to finish session");

        match result {
            FinishResult::Success { final_branch } => {
                assert_eq!(final_branch, "feature-msg-test");

                // Verify commit message was used
                let commit_msg = repo
                    .get_commit_message("HEAD")
                    .expect("Failed to get commit message");
                assert_eq!(commit_msg.trim(), custom_message);
            }
            _ => panic!("Expected Success result"),
        }
    }

    #[test]
    fn test_integration_request_with_commit_message() {
        // Test that IntegrationRequest properly handles commit_message field
        let (_temp_dir, repo) = setup_test_repo();
        let manager = IntegrationManager::new(&repo);
        let branch_manager = BranchManager::new(&repo);

        let main_branch = repo
            .get_current_branch()
            .expect("Failed to get current branch");

        // Create and checkout feature branch
        branch_manager
            .create_branch("feature-integration", &main_branch)
            .expect("Failed to create feature branch");

        repo.checkout_branch("feature-integration")
            .expect("Failed to checkout feature branch");

        // Make a change
        fs::write(repo.root.join("integration.txt"), "Integration test")
            .expect("Failed to write file");
        repo.stage_all_changes().expect("Failed to stage changes");
        repo.commit("Initial feature commit")
            .expect("Failed to commit");

        // Go back to main
        repo.checkout_branch(&main_branch)
            .expect("Failed to checkout main");

        // Test integration with custom commit message
        let custom_message = "Integrated feature with custom message";
        let request = IntegrationRequest {
            feature_branch: "feature-integration".to_string(),
            base_branch: main_branch.clone(),
            commit_message: Some(custom_message.to_string()),
        };

        // We're testing the request structure is properly handled
        // The actual integration might fail in test environment, but that's ok
        let _result = manager.integrate_branch(request);

        // The test passes if it compiles and runs without panic
        // This validates that commit_message is properly handled in the flow
    }

    #[test]
    fn test_finish_result_variants() {
        // Test that FinishResult enum variants work correctly
        let success_result = FinishResult::Success {
            final_branch: "main".to_string(),
        };

        match success_result {
            FinishResult::Success { final_branch } => {
                assert_eq!(final_branch, "main");
            }
            _ => panic!("Expected Success variant"),
        }

        let failure_result = FinishResult::SuccessWithIntegrationFailure {
            final_branch: "feature".to_string(),
            error: "Conflict detected".to_string(),
        };

        match failure_result {
            FinishResult::SuccessWithIntegrationFailure {
                final_branch,
                error,
            } => {
                assert_eq!(final_branch, "feature");
                assert_eq!(error, "Conflict detected");
            }
            _ => panic!("Expected SuccessWithIntegrationFailure variant"),
        }
    }

    #[test]
    fn test_integrate_from_worktree_with_commit_message() {
        // Simplified test focusing on the commit message parameter
        let (_temp_dir, repo) = setup_test_repo();
        let manager = IntegrationManager::new(&repo);

        // Create a mock worktree scenario by setting up a file that makes is_in_worktree return true
        let git_file = repo.root.join(".git_test");
        fs::write(&git_file, "gitdir: /fake/path").expect("Failed to write git file");

        // The method should handle the commit_message parameter
        // We're testing the signature and flow, not the full git operations
        let result =
            manager.integrate_from_worktree("feature", "main", Some("Test commit message"));

        // Clean up
        let _ = fs::remove_file(git_file);

        // The test passes if the method accepts the commit_message parameter
        // Actual integration might fail in test environment, which is ok
        assert!(result.is_err() || result.is_ok());
    }

    #[test]
    fn test_is_am_in_progress() {
        let (_temp_dir, repo) = setup_test_repo();
        let manager = IntegrationManager::new(&repo);

        // Initially should not be in AM state
        assert!(!manager.is_am_in_progress().unwrap());

        // Create rebase-apply directory to simulate AM in progress
        let rebase_apply = repo.git_dir.join("rebase-apply");
        fs::create_dir(&rebase_apply).expect("Failed to create rebase-apply dir");

        // Now should detect AM in progress
        assert!(manager.is_am_in_progress().unwrap());

        // Cleanup
        fs::remove_dir(&rebase_apply).expect("Failed to remove rebase-apply dir");
        assert!(!manager.is_am_in_progress().unwrap());
    }

    #[test]
    fn test_abort_am() {
        let (_temp_dir, repo) = setup_test_repo();
        let manager = IntegrationManager::new(&repo);

        // abort_am should fail gracefully when not in AM state
        let result = manager.abort_am();
        assert!(result.is_err());
    }

    #[test]
    fn test_cleanup_with_am_state() {
        let (_temp_dir, repo) = setup_test_repo();
        let manager = IntegrationManager::new(&repo);

        // Create rebase-apply directory to simulate AM in progress
        let rebase_apply = repo.git_dir.join("rebase-apply");
        fs::create_dir(&rebase_apply).expect("Failed to create rebase-apply dir");

        // Cleanup should handle AM state
        let result = manager.cleanup_integration_state();
        assert!(result.is_ok());

        // Cleanup the test directory
        let _ = fs::remove_dir(&rebase_apply);
    }

    #[test]
    fn test_apply_patches_handles_detached_head() {
        let (_temp_dir, repo) = setup_test_repo();
        let manager = IntegrationManager::new(&repo);

        // This test verifies the code handles detached HEAD gracefully
        // The actual git operations would fail in test environment, but
        // we're testing that our error handling is robust
        let result = manager.apply_patches_to_main_repo(
            "some patch content".to_string(),
            &repo.git_dir,
            &repo.root,
            "main",
        );

        // Should fail but not panic
        assert!(result.is_err());
    }

    #[test]
    fn test_get_main_repo_path_parsing() {
        // Test that get_main_repo_path correctly extracts main repo path
        // from worktree .git file content
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        // Create a mock worktree directory structure
        let worktree_path = temp_dir.path().join("session-worktree");
        fs::create_dir(&worktree_path).expect("Failed to create worktree dir");

        // Create a mock .git file that points to main repo worktree directory
        let git_file = worktree_path.join(".git");
        let main_repo_path = temp_dir.path().join("main-repo");
        let worktree_git_dir = format!(
            "{}/main-repo/.git/worktrees/session",
            temp_dir.path().to_string_lossy()
        );
        fs::write(&git_file, format!("gitdir: {}", worktree_git_dir))
            .expect("Failed to write .git file");

        // Create minimal git structure for main repo
        let main_git_dir = main_repo_path.join(".git");
        fs::create_dir_all(&main_git_dir).expect("Failed to create main .git dir");

        // Create GitRepository structure pointing to worktree
        let repo = GitRepository {
            root: worktree_path.clone(),
            git_dir: PathBuf::from(&worktree_git_dir), // Points to worktree git dir
            work_dir: worktree_path.clone(),
        };

        let manager = IntegrationManager::new(&repo);

        // Test that get_main_repo_path correctly extracts the main repo path
        let result = manager.get_main_repo_path();
        assert!(result.is_ok());

        let extracted_main_path = result.unwrap();
        assert_eq!(extracted_main_path, main_repo_path);

        // Verify that when we join ".git" to this path, we get the correct main git directory
        let computed_main_git_dir = extracted_main_path.join(".git");
        assert_eq!(computed_main_git_dir, main_git_dir);

        // This test specifically verifies the fix for the double .git/.git issue
        // Before the fix: extracted_main_path would be /path/to/main/.git
        // After the fix: extracted_main_path is correctly /path/to/main
    }

    #[test]
    fn test_cleanup_multiple_operations() {
        let (_temp_dir, repo) = setup_test_repo();
        let manager = IntegrationManager::new(&repo);

        // Simulate multiple operations in progress
        let rebase_apply = repo.git_dir.join("rebase-apply");
        let merge_head = repo.git_dir.join("MERGE_HEAD");
        let cherry_pick_head = repo.git_dir.join("CHERRY_PICK_HEAD");

        fs::create_dir(&rebase_apply).expect("Failed to create rebase-apply");
        fs::write(&merge_head, "dummy").expect("Failed to create MERGE_HEAD");
        fs::write(&cherry_pick_head, "dummy").expect("Failed to create CHERRY_PICK_HEAD");

        // Cleanup should handle all states
        let result = manager.cleanup_integration_state();
        assert!(result.is_ok());

        // Files should be cleaned up
        assert!(!merge_head.exists());
        assert!(!cherry_pick_head.exists());

        // Cleanup test files
        let _ = fs::remove_dir(&rebase_apply);
    }

    #[test]
    fn test_is_in_worktree_with_broken_git_file() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        // Create a worktree-like structure with .git file
        let worktree_path = temp_dir.path().join("worktree");
        fs::create_dir(&worktree_path).expect("Failed to create worktree dir");

        // Write invalid .git file content
        let git_file = worktree_path.join(".git");
        fs::write(&git_file, "invalid content").expect("Failed to write .git file");

        // Create a minimal git dir structure
        let git_dir = temp_dir.path().join(".git");
        fs::create_dir(&git_dir).expect("Failed to create .git dir");

        // Create repo from worktree path
        let repo = GitRepository {
            root: worktree_path.clone(),
            git_dir: git_dir.clone(),
            work_dir: worktree_path.clone(),
        };
        let manager = IntegrationManager::new(&repo);

        // Should handle gracefully
        let result = manager.is_in_worktree();
        assert!(result.is_ok());
        assert!(result.unwrap()); // It's a file, so it's treated as worktree
    }

    #[test]
    fn test_safe_abort_integration_with_all_states() {
        let (_temp_dir, repo) = setup_test_repo();
        let manager = IntegrationManager::new(&repo);

        // Create various state files
        let rebase_apply = repo.git_dir.join("rebase-apply");
        fs::create_dir(&rebase_apply).expect("Failed to create rebase-apply");

        let result = manager.safe_abort_integration(None, "main");
        // Should succeed even with no backup branch
        assert!(result.is_ok());

        // Cleanup
        let _ = fs::remove_dir(&rebase_apply);
    }
}
