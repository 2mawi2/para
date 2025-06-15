use crate::config::Config;
use crate::core::git::{GitOperations, GitService};
use crate::core::session::SessionManager;
use crate::ui::monitor::SessionInfo;
use crate::utils::Result;
use std::process::Command;

/// Business logic actions for the monitor UI
pub struct MonitorActions {
    config: Config,
}

impl MonitorActions {
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    pub fn resume_session(&self, session: &SessionInfo) -> Result<()> {
        // Verify worktree path exists
        if !session.worktree_path.exists() {
            return Err(crate::utils::ParaError::file_operation(format!(
                "Worktree path does not exist: {}",
                session.worktree_path.display()
            )));
        }

        // Use internal APIs to open IDE
        let ide_manager = crate::core::ide::IdeManager::new(&self.config);
        ide_manager
            .launch(&session.worktree_path, false)
            .map_err(|e| crate::utils::ParaError::ide_error(format!("Failed to launch IDE: {}", e)))
    }

    pub fn finish_session(&self, session: &SessionInfo, message: String) -> Result<()> {
        // Run in a separate thread to prevent git output from appearing in UI
        let worktree_path = session.worktree_path.clone();
        let branch = session.branch.clone();

        std::thread::spawn(move || {
            if let Ok(git_service) = GitService::discover_from(&worktree_path) {
                let finish_request = crate::core::git::FinishRequest {
                    feature_branch: branch,
                    commit_message: message,
                    target_branch_name: None,
                };
                let _ = git_service.finish_session(finish_request);
            }
        });

        Ok(())
    }

    pub fn cancel_session(&self, session: &SessionInfo) -> Result<()> {
        let session_manager = SessionManager::new(&self.config);

        // Load session state and archive the branch
        if let Ok(session_state) = session_manager.load_state(&session.name) {
            // Run git operations in a separate thread to prevent output in UI
            let worktree_path = session.worktree_path.clone();
            let branch = session_state.branch.clone();
            let name = session_state.name.clone();
            let prefix = self.config.git.branch_prefix.clone();
            let worktree_to_remove = session_state.worktree_path.clone();

            std::thread::spawn(move || {
                if let Ok(git_service) = GitService::discover_from(&worktree_path) {
                    // Archive the branch
                    let _ = git_service.archive_branch_with_session_name(&branch, &name, &prefix);

                    // Remove worktree
                    let _ = git_service.remove_worktree(&worktree_to_remove);
                }
            });

            // Delete session state
            session_manager.delete_state(&session_state.name)?;
        }

        Ok(())
    }

    pub fn integrate_session(&self, session: &SessionInfo) -> Result<()> {
        use crate::ui::monitor::SessionStatus;

        if matches!(session.status, SessionStatus::Ready) {
            let _ = Command::new("para")
                .args(["integrate", &session.name])
                .current_dir(&session.worktree_path)
                .spawn();
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::monitor::{SessionInfo, SessionStatus};
    use chrono::Utc;
    use std::path::PathBuf;

    fn create_test_session() -> SessionInfo {
        SessionInfo {
            name: "test-session".to_string(),
            branch: "test-branch".to_string(),
            status: SessionStatus::Active,
            last_activity: Utc::now(),
            task: "Test task".to_string(),
            worktree_path: PathBuf::from("/tmp/test-session"),
        }
    }

    fn create_test_config() -> Config {
        // Create a minimal test config without using test_utils
        Config {
            ide: crate::config::IdeConfig {
                name: "test".to_string(),
                command: "echo".to_string(),
                user_data_dir: None,
                wrapper: crate::config::WrapperConfig {
                    enabled: false,
                    name: String::new(),
                    command: String::new(),
                },
            },
            directories: crate::config::DirectoryConfig {
                subtrees_dir: "/tmp/subtrees".to_string(),
                state_dir: "/tmp/.para_state".to_string(),
            },
            git: crate::config::GitConfig {
                branch_prefix: "para".to_string(),
                auto_stage: true,
                auto_commit: false,
            },
            session: crate::config::SessionConfig {
                default_name_format: "%Y%m%d-%H%M%S".to_string(),
                preserve_on_finish: false,
                auto_cleanup_days: Some(7),
            },
        }
    }

    #[test]
    fn test_actions_creation() {
        let config = create_test_config();
        let actions = MonitorActions::new(config);

        // Just test that we can create the actions instance
        assert_eq!(actions.config.git.branch_prefix, "para");
    }

    #[test]
    fn test_resume_session_nonexistent_path() {
        let config = create_test_config();
        let actions = MonitorActions::new(config);
        let session = create_test_session();

        // Should fail because the path doesn't exist
        let result = actions.resume_session(&session);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("does not exist"));
    }

    #[test]
    fn test_integrate_session_logic() {
        let config = create_test_config();
        let actions = MonitorActions::new(config);

        let mut session = create_test_session();
        session.status = SessionStatus::Ready;

        // Should succeed (even if command doesn't work in test)
        let result = actions.integrate_session(&session);
        assert!(result.is_ok());

        // Test with non-ready status
        session.status = SessionStatus::Idle;
        let result = actions.integrate_session(&session);
        assert!(result.is_ok()); // Should still succeed but do nothing
    }
}
