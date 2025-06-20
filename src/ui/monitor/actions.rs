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

        std::thread::spawn(move || {
            use std::process::{Command, Stdio};

            let _ = Command::new("para")
                .args(["resume", &session_name])
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
            confidence: None,
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
}
