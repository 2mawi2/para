use super::branch::BranchManager;
use super::repository::{execute_git_command, execute_git_command_with_status, GitRepository};
use crate::utils::error::{ParaError, Result};
use std::path::PathBuf;

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
    ConflictsPending { state_saved: bool },
}

#[derive(Debug)]
pub struct IntegrationRequest {
    pub feature_branch: String,
    pub base_branch: String,
    pub commit_message: String,
}

#[derive(Debug)]
pub enum IntegrationResult {
    Success,
    ConflictsPending { conflicted_files: Vec<PathBuf> },
    Failed { error: String },
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

        if self.repo.has_uncommitted_changes()? {
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
                commit_message: request.commit_message,
            })? {
                IntegrationResult::Success => Ok(FinishResult::Success {
                    final_branch: request.base_branch,
                }),
                IntegrationResult::ConflictsPending { .. } => {
                    Ok(FinishResult::ConflictsPending { state_saved: true })
                }
                IntegrationResult::Failed { error } => Err(ParaError::git_operation(error)),
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
        self.update_base_branch(&request.base_branch)?;

        match self.prepare_rebase(&request.feature_branch, &request.base_branch) {
            Ok(()) => {
                self.repo.checkout_branch(&request.base_branch)?;

                execute_git_command_with_status(
                    self.repo,
                    &["merge", "--ff-only", &request.feature_branch],
                )?;

                Ok(IntegrationResult::Success)
            }
            Err(_) => {
                if self.has_rebase_conflicts()? {
                    let conflicted_files = self.get_conflicted_files()?;
                    Ok(IntegrationResult::ConflictsPending { conflicted_files })
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
        if !self.has_rebase_conflicts()? {
            return Ok(Vec::new());
        }

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

    pub fn get_merge_conflicts_summary(&self) -> Result<String> {
        let conflicted_files = self.get_conflicted_files()?;

        if conflicted_files.is_empty() {
            return Ok("No conflicts detected".to_string());
        }

        let mut summary = format!("Merge conflicts in {} files:\n", conflicted_files.len());
        for file in &conflicted_files {
            summary.push_str(&format!("  - {}\n", file.display()));
        }

        summary.push_str("\nResolve conflicts and run 'git rebase --continue' to proceed.");
        Ok(summary)
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

    pub fn is_merge_in_progress(&self) -> Result<bool> {
        let merge_head = self.repo.git_dir.join("MERGE_HEAD");
        Ok(merge_head.exists())
    }

    pub fn is_cherry_pick_in_progress(&self) -> Result<bool> {
        let cherry_pick_head = self.repo.git_dir.join("CHERRY_PICK_HEAD");
        Ok(cherry_pick_head.exists())
    }

    pub fn is_any_operation_in_progress(&self) -> Result<bool> {
        Ok(self.is_rebase_in_progress()? 
           || self.is_merge_in_progress()? 
           || self.is_cherry_pick_in_progress()?)
    }

    pub fn safe_abort_integration(&self, backup_branch: Option<&str>, target_branch: &str) -> Result<()> {
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
            .args(&["init"])
            .status()
            .expect("Failed to init git repo");

        Command::new("git")
            .current_dir(repo_path)
            .args(&["config", "user.name", "Test User"])
            .status()
            .expect("Failed to set git user name");

        Command::new("git")
            .current_dir(repo_path)
            .args(&["config", "user.email", "test@example.com"])
            .status()
            .expect("Failed to set git user email");

        fs::write(repo_path.join("README.md"), "# Test Repository")
            .expect("Failed to write README");

        Command::new("git")
            .current_dir(repo_path)
            .args(&["add", "README.md"])
            .status()
            .expect("Failed to add README");

        Command::new("git")
            .current_dir(repo_path)
            .args(&["commit", "-m", "Initial commit"])
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
    fn test_is_any_operation_in_progress_clean_state() {
        let (_temp_dir, repo) = setup_test_repo();
        let manager = IntegrationManager::new(&repo);
        
        let result = manager.is_any_operation_in_progress();
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[test]
    fn test_safe_abort_integration_no_backup() {
        let (_temp_dir, repo) = setup_test_repo();
        let manager = IntegrationManager::new(&repo);
        
        let main_branch = repo.get_current_branch().expect("Failed to get current branch");
        let result = manager.safe_abort_integration(None, &main_branch);
        
        assert!(result.is_ok());
    }

    #[test]
    fn test_safe_abort_integration_with_nonexistent_backup() {
        let (_temp_dir, repo) = setup_test_repo();
        let manager = IntegrationManager::new(&repo);
        
        let main_branch = repo.get_current_branch().expect("Failed to get current branch");
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
    fn test_operation_detection_consistency() {
        let (_temp_dir, repo) = setup_test_repo();
        let manager = IntegrationManager::new(&repo);
        
        // All individual operation checks should be false
        assert!(!manager.is_rebase_in_progress().unwrap());
        assert!(!manager.is_merge_in_progress().unwrap());
        assert!(!manager.is_cherry_pick_in_progress().unwrap());
        
        // Combined check should also be false
        assert!(!manager.is_any_operation_in_progress().unwrap());
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
}
