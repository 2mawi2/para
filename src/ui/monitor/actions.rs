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
        if !session.worktree_path.exists() {
            return Err(crate::utils::ParaError::file_operation(format!(
                "Worktree path does not exist: {}",
                session.worktree_path.display()
            )));
        }

        let session_name = session.name.clone();

        // Check if the session was created with dangerous flag
        let session_manager = SessionManager::new(&self.config);
        let use_dangerous_flag =
            if let Ok(session_state) = session_manager.load_state(&session.name) {
                session_state.dangerous_skip_permissions.unwrap_or(false)
            } else {
                false
            };

        std::thread::spawn(move || {
            use std::process::{Command, Stdio};

            let mut cmd = Command::new("para");
            cmd.arg("resume").arg(&session_name);

            // Add dangerous flag if the session was originally created with it
            if use_dangerous_flag {
                cmd.arg("--dangerously-skip-permissions");
            }

            let _ = cmd
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn();
        });

        Ok(())
    }

    pub fn finish_session(&self, session: &SessionInfo, message: String) -> Result<()> {
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

        if let Ok(session_state) = session_manager.load_state(&session.name) {
            let worktree_path = session.worktree_path.clone();
            let branch = session_state.branch.clone();
            let name = session_state.name.clone();
            let prefix = self.config.git.branch_prefix.clone();
            let worktree_to_remove = session_state.worktree_path.clone();

            std::thread::spawn(move || {
                if let Ok(git_service) = GitService::discover_from(&worktree_path) {
                    let _ = git_service.archive_branch_with_session_name(&branch, &name, &prefix);
                    let _ = git_service
                        .worktree_manager()
                        .force_remove_worktree(&worktree_to_remove);
                }
            });

            // Delete session state
            session_manager.delete_state(&session_state.name)?;
        }

        Ok(())
    }

    pub fn integrate_session(&self, session: &SessionInfo) -> Result<()> {
        use crate::ui::monitor::SessionStatus;
        use std::process::Stdio;

        if matches!(session.status, SessionStatus::Ready) {
            let _ = Command::new("para")
                .args(["integrate", &session.name])
                .current_dir(&session.worktree_path)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
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
            test_status: None,
            diff_stats: None,
            todo_percentage: None,
            is_blocked: false,
        }
    }

    fn create_test_config() -> Config {
        crate::test_utils::test_helpers::create_test_config()
    }

    #[test]
    fn test_actions_creation() {
        let config = create_test_config();
        let actions = MonitorActions::new(config);

        assert_eq!(actions.config.git.branch_prefix, "para");
    }

    #[test]
    fn test_resume_session_nonexistent_path() {
        let config = create_test_config();
        let actions = MonitorActions::new(config);
        let session = create_test_session();

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

        let result = actions.integrate_session(&session);
        assert!(result.is_ok());

        // Test with non-ready status
        session.status = SessionStatus::Idle;
        let result = actions.integrate_session(&session);
        assert!(result.is_ok());
    }

    #[test]
    fn test_resume_session_dangerous_flag_preservation() {
        use crate::core::session::state::SessionState;
        use crate::core::session::SessionManager;
        use tempfile::TempDir;

        // Create a temporary directory for state
        let temp_dir = TempDir::new().unwrap();
        let state_dir = temp_dir.path().join(".para_state");
        std::fs::create_dir_all(&state_dir).unwrap();

        let mut config = create_test_config();
        config.directories.state_dir = state_dir.to_string_lossy().to_string();

        let session_manager = SessionManager::new(&config);
        let actions = MonitorActions::new(config.clone());

        // Create a session with dangerous flag
        let worktree_path = temp_dir.path().join("test-dangerous-worktree");
        std::fs::create_dir_all(&worktree_path).unwrap();

        let session_state = SessionState::with_parent_branch_and_flags(
            "test-dangerous".to_string(),
            "para/test-dangerous".to_string(),
            worktree_path.clone(),
            "main".to_string(),
            true, // dangerous_skip_permissions = true
        );

        session_manager.save_state(&session_state).unwrap();

        // Create SessionInfo for the monitor
        let session_info = SessionInfo {
            name: "test-dangerous".to_string(),
            branch: "para/test-dangerous".to_string(),
            status: SessionStatus::Active,
            last_activity: chrono::Utc::now(),
            task: "Test task".to_string(),
            worktree_path: worktree_path.clone(),
            test_status: None,
            diff_stats: None,
            todo_percentage: None,
            is_blocked: false,
        };

        // The resume_session function should check the session state
        // and include the dangerous flag when spawning the command
        let result = actions.resume_session(&session_info);
        assert!(result.is_ok());

        // Verify that the session state has the dangerous flag
        let loaded_state = session_manager.load_state("test-dangerous").unwrap();
        assert_eq!(loaded_state.dangerous_skip_permissions, Some(true));
    }
}
