use crate::config::Config;
use crate::core::git::{ArchiveBranchIterator, GitService, HasTimestamp};
use crate::core::session::{SessionManager, SessionState};
use crate::utils::{ArchiveBranchParser, ParaError, Result};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct RecoveryOptions {
    pub force_overwrite: bool,
    pub preserve_original_name: bool,
}

impl Default for RecoveryOptions {
    fn default() -> Self {
        Self {
            force_overwrite: false,
            preserve_original_name: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RecoveryInfo {
    pub archived_branch: String,
    pub original_session_name: String,
    pub archived_timestamp: String,
}

impl HasTimestamp for RecoveryInfo {
    fn timestamp(&self) -> &str {
        &self.archived_timestamp
    }
}

#[derive(Debug)]
pub struct RecoveryResult {
    pub session_name: String,
    pub branch_name: String,
    pub worktree_path: PathBuf,
}

pub struct SessionRecovery<'a> {
    config: &'a Config,
    git_service: &'a GitService,
    session_manager: &'a SessionManager,
}

impl<'a> SessionRecovery<'a> {
    pub fn new(
        config: &'a Config,
        git_service: &'a GitService,
        session_manager: &'a SessionManager,
    ) -> Self {
        Self {
            config,
            git_service,
            session_manager,
        }
    }

    pub fn recover_active_session(&self, session_name: &str) -> Result<RecoveryResult> {
        let session_state = self.session_manager.load_state(session_name)?;

        // Check if worktree exists
        let worktree_exists = session_state.worktree_path.exists();
        let branch_exists = self
            .git_service
            .branch_manager()
            .branch_exists(&session_state.branch)?;

        if worktree_exists && branch_exists {
            return Ok(RecoveryResult {
                session_name: session_name.to_string(),
                branch_name: session_state.branch,
                worktree_path: session_state.worktree_path,
            });
        }

        // Need to recover missing worktree
        if !worktree_exists && branch_exists {
            // Ensure parent directory exists
            if let Some(parent) = session_state.worktree_path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            // Create worktree
            self.git_service
                .worktree_manager()
                .create_worktree(&session_state.branch, &session_state.worktree_path)?;

            return Ok(RecoveryResult {
                session_name: session_name.to_string(),
                branch_name: session_state.branch,
                worktree_path: session_state.worktree_path,
            });
        } else if !branch_exists {
            return Err(ParaError::session_not_found(format!(
                "Session '{}' has missing branch '{}' - cannot recover",
                session_name, session_state.branch
            )));
        }

        // If worktree exists but there might be other issues
        Ok(RecoveryResult {
            session_name: session_name.to_string(),
            branch_name: session_state.branch,
            worktree_path: session_state.worktree_path,
        })
    }

    pub fn is_active_session(&self, session_name: &str) -> bool {
        self.session_manager.session_exists(session_name)
    }

    pub fn recover_session_unified(
        &self,
        session_name: &str,
        options: RecoveryOptions,
    ) -> Result<RecoveryResult> {
        // First check if this is an active session that needs recovery
        if self.is_active_session(session_name) {
            return self.recover_active_session(session_name);
        }

        // If not an active session, try archived session recovery
        self.recover_session(session_name, options)
    }

    pub fn list_recoverable_sessions(&self) -> Result<Vec<RecoveryInfo>> {
        let iterator = ArchiveBranchIterator::new(self.git_service, self.config);
        iterator
            .list_archived_entries(|archived_branch| self.parse_archived_branch(archived_branch))
    }

    pub fn recover_session(
        &self,
        session_name: &str,
        options: RecoveryOptions,
    ) -> Result<RecoveryResult> {
        let recoverable_sessions = self.list_recoverable_sessions()?;

        let recovery_info = recoverable_sessions
            .iter()
            .find(|info| info.original_session_name == session_name)
            .ok_or_else(|| ParaError::session_not_found(session_name.to_string()))?;

        self.recover_from_info(recovery_info, options)
    }

    pub fn validate_recovery(&self, session_name: &str) -> Result<RecoveryValidation> {
        let _recovery_info = self
            .list_recoverable_sessions()?
            .into_iter()
            .find(|info| info.original_session_name == session_name)
            .ok_or_else(|| ParaError::session_not_found(session_name.to_string()))?;

        let mut validation = RecoveryValidation {
            can_recover: true,
            conflicts: Vec::new(),
            warnings: Vec::new(),
        };

        if self.session_manager.session_exists(session_name) {
            validation
                .conflicts
                .push(format!("Session '{}' already exists", session_name));
        }

        let target_worktree_path = self.get_target_worktree_path(session_name);
        if target_worktree_path.exists() {
            validation.conflicts.push(format!(
                "Worktree directory already exists: {}",
                target_worktree_path.display()
            ));
        }

        let branch_manager = self.git_service.branch_manager();
        if branch_manager.branch_exists(session_name)? {
            validation.warnings.push(format!(
                "Branch '{}' already exists, will create unique name",
                session_name
            ));
        }

        if !validation.conflicts.is_empty() {
            validation.can_recover = false;
        }

        Ok(validation)
    }

    fn recover_from_info(
        &self,
        recovery_info: &RecoveryInfo,
        options: RecoveryOptions,
    ) -> Result<RecoveryResult> {
        let validation = self.validate_recovery(&recovery_info.original_session_name)?;

        if !validation.can_recover && !options.force_overwrite {
            return Err(ParaError::worktree_operation(format!(
                "Cannot recover session due to conflicts: {}",
                validation.conflicts.join(", ")
            )));
        }

        let branch_manager = self.git_service.branch_manager();
        let worktree_manager = self.git_service.worktree_manager();

        let restored_branch = branch_manager.restore_from_archive(
            &recovery_info.archived_branch,
            self.config.get_branch_prefix(),
        )?;

        let final_session_name = if options.preserve_original_name {
            recovery_info.original_session_name.clone()
        } else {
            restored_branch.clone()
        };

        let worktree_path = self.get_target_worktree_path(&restored_branch);

        if worktree_path.exists() {
            if options.force_overwrite {
                std::fs::remove_dir_all(&worktree_path)?;
            } else {
                return Err(ParaError::worktree_operation(format!(
                    "Worktree directory already exists: {}",
                    worktree_path.display()
                )));
            }
        }

        if let Some(parent) = worktree_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        worktree_manager.create_worktree(&restored_branch, &worktree_path)?;

        let session_state = SessionState::new(
            final_session_name.clone(),
            restored_branch.clone(),
            worktree_path.clone(),
        );

        if self.session_manager.session_exists(&final_session_name) && options.force_overwrite {
            self.session_manager.delete_state(&final_session_name)?;
        }

        self.session_manager.save_state(&session_state)?;

        Ok(RecoveryResult {
            session_name: final_session_name,
            branch_name: restored_branch,
            worktree_path,
        })
    }

    fn parse_archived_branch(&self, archived_branch: &str) -> Result<Option<RecoveryInfo>> {
        let archive_info = ArchiveBranchParser::parse_archive_branch(
            archived_branch,
            self.config.get_branch_prefix(),
        )?;

        match archive_info {
            Some(info) => {
                // Validate that the branch actually exists by checking its commit
                let branch_manager = self.git_service.branch_manager();
                let _commit_hash = branch_manager.get_branch_commit(archived_branch)?;

                Ok(Some(RecoveryInfo {
                    archived_branch: info.full_branch_name,
                    original_session_name: info.session_name,
                    archived_timestamp: info.timestamp,
                }))
            }
            None => Ok(None),
        }
    }

    fn get_target_worktree_path(&self, session_name: &str) -> PathBuf {
        let repository_root = &self.git_service.repository().root;
        let subtrees_path = repository_root.join(&self.config.directories.subtrees_dir);
        subtrees_path
            .join(self.config.get_branch_prefix())
            .join(session_name)
    }
}

#[derive(Debug)]
pub struct RecoveryValidation {
    pub can_recover: bool,
    pub conflicts: Vec<String>,
    pub warnings: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::test_helpers::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_recovery_info_parsing() {
        let temp_dir = TempDir::new().unwrap();
        let git_temp = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, git_service) = setup_test_repo();

        let config = create_test_config_with_dir(&temp_dir);
        let session_manager = SessionManager::new(&config);
        let recovery = SessionRecovery::new(&config, &git_service, &session_manager);

        let archived_branch = "test/archived/20240301-120000/my-session";

        let branch_manager = git_service.branch_manager();
        let initial_branch = git_service.repository().get_current_branch().unwrap();
        branch_manager
            .create_branch(archived_branch, &initial_branch)
            .unwrap();

        let info = recovery
            .parse_archived_branch(archived_branch)
            .unwrap()
            .unwrap();

        assert_eq!(info.archived_branch, archived_branch);
        assert_eq!(info.original_session_name, "my-session");
        assert_eq!(info.archived_timestamp, "20240301-120000");
    }

    #[test]
    fn test_recovery_validation() {
        let temp_dir = TempDir::new().unwrap();
        let git_temp = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, git_service) = setup_test_repo();

        let config = create_test_config_with_dir(&temp_dir);
        let session_manager = SessionManager::new(&config);
        let recovery = SessionRecovery::new(&config, &git_service, &session_manager);

        let validation = recovery.validate_recovery("nonexistent-session");
        assert!(validation.is_err());
    }

    #[test]
    fn test_target_worktree_path() {
        let temp_dir = TempDir::new().unwrap();
        let git_temp = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, git_service) = setup_test_repo();

        let config = create_test_config_with_dir(&temp_dir);
        let session_manager = SessionManager::new(&config);
        let recovery = SessionRecovery::new(&config, &git_service, &session_manager);

        let path = recovery.get_target_worktree_path("my-session");
        assert!(path.to_string_lossy().contains("subtrees/test/my-session"));
    }

    #[test]
    fn test_full_recovery_workflow() {
        let temp_dir = TempDir::new().unwrap();
        let git_temp = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, git_service) = setup_test_repo();

        let config = create_test_config_with_dir(&temp_dir);
        let session_manager = SessionManager::new(&config);
        let recovery = SessionRecovery::new(&config, &git_service, &session_manager);
        let branch_manager = git_service.branch_manager();

        let initial_branch = git_service.repository().get_current_branch().unwrap();
        branch_manager
            .create_branch("test-session", &initial_branch)
            .unwrap();

        fs::write(
            git_service.repository().root.join("test-file.txt"),
            "test content",
        )
        .unwrap();

        let repo_path = &git_service.repository().root;
        std::process::Command::new("git")
            .current_dir(repo_path)
            .args(["add", "test-file.txt"])
            .status()
            .unwrap();
        std::process::Command::new("git")
            .current_dir(repo_path)
            .args(["commit", "-m", "Add test file"])
            .status()
            .unwrap();

        git_service
            .repository()
            .checkout_branch(&initial_branch)
            .unwrap();

        // Archive the session using the new session-name-based method
        branch_manager
            .move_to_archive_with_session_name(
                "test-session",
                "test-session",
                config.get_branch_prefix(),
            )
            .unwrap();

        let options = RecoveryOptions {
            force_overwrite: false,
            preserve_original_name: true,
        };

        let result = recovery.recover_session("test-session", options).unwrap();

        assert_eq!(result.session_name, "test-session");
        assert!(result.worktree_path.exists());
        assert!(result.worktree_path.join("test-file.txt").exists());

        let content = fs::read_to_string(result.worktree_path.join("test-file.txt")).unwrap();
        assert_eq!(content, "test content");

        assert!(session_manager.session_exists(&result.session_name));
    }

    #[test]
    fn test_recovery_validation_with_conflicts() {
        let temp_dir = TempDir::new().unwrap();
        let git_temp = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, git_service) = setup_test_repo();

        let config = create_test_config_with_dir(&temp_dir);
        let session_manager = SessionManager::new(&config);
        let recovery = SessionRecovery::new(&config, &git_service, &session_manager);
        let branch_manager = git_service.branch_manager();

        let initial_branch = git_service.repository().get_current_branch().unwrap();
        branch_manager
            .create_branch("conflict-session", &initial_branch)
            .unwrap();
        git_service
            .repository()
            .checkout_branch(&initial_branch)
            .unwrap();
        let _archived_branch = branch_manager
            .move_to_archive("conflict-session", config.get_branch_prefix())
            .unwrap();

        branch_manager
            .create_branch("conflict-session", &initial_branch)
            .unwrap();
        git_service
            .repository()
            .checkout_branch(&initial_branch)
            .unwrap();

        let session_state = crate::core::session::SessionState::new(
            "conflict-session".to_string(),
            "conflict-session".to_string(),
            temp_dir.path().join("test-worktree"),
        );
        session_manager.save_state(&session_state).unwrap();

        let validation = recovery.validate_recovery("conflict-session").unwrap();
        assert!(!validation.can_recover);
        assert!(!validation.conflicts.is_empty());
        assert!(validation
            .conflicts
            .iter()
            .any(|c| c.contains("already exists")));
    }

    #[test]
    fn test_unified_recovery_active_session() {
        let temp_dir = TempDir::new().unwrap();
        let git_temp = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, git_service) = setup_test_repo();

        let config = create_test_config_with_dir(&temp_dir);
        let session_manager = SessionManager::new(&config);
        let recovery = SessionRecovery::new(&config, &git_service, &session_manager);

        // Create an active session
        let session_name = "test-unified-active";
        let worktree_path = temp_dir.path().join("worktree");
        std::fs::create_dir_all(&worktree_path).unwrap();

        let session_state = SessionState::new(
            session_name.to_string(),
            "test-branch".to_string(),
            worktree_path.clone(),
        );

        session_manager.save_state(&session_state).unwrap();

        // Create the branch
        let initial_branch = git_service.repository().get_current_branch().unwrap();
        git_service
            .branch_manager()
            .create_branch("test-branch", &initial_branch)
            .unwrap();

        // Test unified recovery - should use active session recovery
        let options = RecoveryOptions {
            force_overwrite: false,
            preserve_original_name: true,
        };

        let result = recovery
            .recover_session_unified(session_name, options)
            .unwrap();
        assert_eq!(result.session_name, session_name);
        assert_eq!(result.branch_name, "test-branch");
    }
}
