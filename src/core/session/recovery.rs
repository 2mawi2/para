use crate::config::Config;
use crate::core::git::GitService;
use crate::core::session::{SessionManager, SessionState};
use crate::utils::{ParaError, Result};
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

    pub fn list_recoverable_sessions(&self) -> Result<Vec<RecoveryInfo>> {
        let archived_branches = self
            .git_service
            .branch_manager()
            .list_archived_branches(self.config.get_branch_prefix())?;

        let mut recovery_infos = Vec::new();

        for archived_branch in archived_branches {
            if let Some(info) = self.parse_archived_branch(&archived_branch)? {
                recovery_infos.push(info);
            }
        }

        recovery_infos.sort_by(|a, b| b.archived_timestamp.cmp(&a.archived_timestamp));
        Ok(recovery_infos)
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
        let archive_prefix = format!("{}/archived/", self.config.get_branch_prefix());

        if !archived_branch.starts_with(&archive_prefix) {
            return Ok(None);
        }

        let suffix = archived_branch.strip_prefix(&archive_prefix).unwrap();
        let parts: Vec<&str> = suffix.split('/').collect();

        if parts.len() != 2 {
            return Ok(None);
        }

        let timestamp = parts[0];
        let session_name = parts[1];

        let branch_manager = self.git_service.branch_manager();
        let _commit_hash = branch_manager.get_branch_commit(archived_branch)?;

        Ok(Some(RecoveryInfo {
            archived_branch: archived_branch.to_string(),
            original_session_name: session_name.to_string(),
            archived_timestamp: timestamp.to_string(),
        }))
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
    use crate::config::{DirectoryConfig, GitConfig, IdeConfig, SessionConfig, WrapperConfig};
    use std::fs;
    use std::path::Path;
    use std::process::Command;
    use tempfile::TempDir;

    fn create_test_config(temp_dir: &Path) -> Config {
        Config {
            ide: IdeConfig {
                name: "test".to_string(),
                command: "echo".to_string(),
                user_data_dir: None,
                wrapper: WrapperConfig {
                    enabled: false,
                    name: String::new(),
                    command: String::new(),
                },
            },
            directories: DirectoryConfig {
                subtrees_dir: temp_dir.join("subtrees").to_string_lossy().to_string(),
                state_dir: temp_dir.join(".para_state").to_string_lossy().to_string(),
            },
            git: GitConfig {
                branch_prefix: "test".to_string(),
                auto_stage: true,
                auto_commit: false,
            },
            session: SessionConfig {
                default_name_format: "%Y%m%d-%H%M%S".to_string(),
                preserve_on_finish: false,
                auto_cleanup_days: Some(7),
            },
        }
    }

    fn setup_test_repo() -> (TempDir, GitService, Config, SessionManager) {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let repo_path = temp_dir.path().join("repo");
        fs::create_dir_all(&repo_path).expect("Failed to create repo dir");

        Command::new("git")
            .current_dir(&repo_path)
            .args(["init", "--initial-branch=main"])
            .status()
            .expect("Failed to init git repo");

        Command::new("git")
            .current_dir(&repo_path)
            .args(["config", "user.name", "Test User"])
            .status()
            .expect("Failed to set git user name");

        Command::new("git")
            .current_dir(&repo_path)
            .args(["config", "user.email", "test@example.com"])
            .status()
            .expect("Failed to set git user email");

        fs::write(repo_path.join("README.md"), "# Test Repository")
            .expect("Failed to write README");

        Command::new("git")
            .current_dir(&repo_path)
            .args(["add", "README.md"])
            .status()
            .expect("Failed to add README");

        Command::new("git")
            .current_dir(&repo_path)
            .args(["commit", "-m", "Initial commit"])
            .status()
            .expect("Failed to commit README");

        let git_service = GitService::discover_from(&repo_path).expect("Failed to discover repo");
        let config = create_test_config(temp_dir.path());
        let session_manager = SessionManager::new(&config);

        (temp_dir, git_service, config, session_manager)
    }

    #[test]
    fn test_recovery_info_parsing() {
        let (_temp_dir, git_service, config, session_manager) = setup_test_repo();
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
        let (_temp_dir, git_service, config, session_manager) = setup_test_repo();
        let recovery = SessionRecovery::new(&config, &git_service, &session_manager);

        let validation = recovery.validate_recovery("nonexistent-session");
        assert!(validation.is_err());
    }

    #[test]
    fn test_target_worktree_path() {
        let (_temp_dir, git_service, config, session_manager) = setup_test_repo();
        let recovery = SessionRecovery::new(&config, &git_service, &session_manager);

        let path = recovery.get_target_worktree_path("my-session");
        assert!(path.to_string_lossy().contains("subtrees/test/my-session"));
    }

    #[test]
    fn test_full_recovery_workflow() {
        let (_temp_dir, git_service, config, session_manager) = setup_test_repo();
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
        Command::new("git")
            .current_dir(repo_path)
            .args(["add", "test-file.txt"])
            .status()
            .unwrap();
        Command::new("git")
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
        let (_temp_dir, git_service, config, session_manager) = setup_test_repo();
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
            _temp_dir.path().join("test-worktree"),
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
}
