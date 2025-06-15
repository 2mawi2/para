use crate::config::Config;
use crate::core::session::{SessionManager, SessionStatus as CoreSessionStatus};
use crate::ui::monitor::activity::detect_last_activity;
use crate::ui::monitor::{SessionInfo, SessionStatus};
use crate::utils::Result;
use chrono::{DateTime, Utc};
use std::path::{Path, PathBuf};

/// Service for managing session data in the monitor UI
pub struct SessionService {
    config: Config,
}

impl SessionService {
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    pub fn load_sessions(&self, show_stale: bool) -> Result<Vec<SessionInfo>> {
        let session_manager = SessionManager::new(&self.config);
        let sessions = session_manager.list_sessions()?;

        let mut session_infos = Vec::new();

        // Check if current directory is a session worktree
        let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let current_session = session_manager
            .find_session_by_path(&current_dir)
            .unwrap_or(None);

        for session in sessions {
            // Skip cancelled sessions
            if matches!(session.status, CoreSessionStatus::Cancelled) {
                continue;
            }

            // Get last activity from git/filesystem or fall back to session data
            let last_activity = detect_last_activity(&session.worktree_path)
                .or(session.last_activity)
                .unwrap_or(session.created_at);

            // Detect status based on activity
            let mut status = detect_session_status(&session, &last_activity);

            // If this is the current session (where monitor was run from), mark it as active
            if let Some(ref current) = current_session {
                if current.name == session.name {
                    status = SessionStatus::Active;
                }
            }

            // Load task description
            let task = session.task_description.unwrap_or_else(|| {
                // Try to load from task file for backward compatibility
                let state_dir = Path::new(&self.config.directories.state_dir);
                let task_file = state_dir.join(format!("{}.task", session.name));
                std::fs::read_to_string(task_file).unwrap_or_else(|_| {
                    // Show full session name if no task description
                    format!("Session: {}", &session.name)
                })
            });

            let session_info = SessionInfo {
                name: session.name.clone(),
                branch: session.branch.clone(),
                status,
                last_activity,
                task,
                worktree_path: session.worktree_path.clone(),
            };

            // Filter out stale sessions if not showing them
            if !show_stale && matches!(session_info.status, SessionStatus::Stale) {
                continue;
            }

            session_infos.push(session_info);
        }

        // Sort by last activity (most recent first), but put current session first if found
        session_infos.sort_by(|a, b| {
            if let Some(ref current) = current_session {
                if a.name == current.name {
                    return std::cmp::Ordering::Less; // Current session goes first
                }
                if b.name == current.name {
                    return std::cmp::Ordering::Greater; // Current session goes first
                }
            }
            b.last_activity.cmp(&a.last_activity)
        });

        Ok(session_infos)
    }
}

fn detect_session_status(
    session: &crate::core::session::SessionState,
    last_activity: &DateTime<Utc>,
) -> SessionStatus {
    // Check if session is marked as finished
    if matches!(session.status, CoreSessionStatus::Finished) {
        return SessionStatus::Ready;
    }

    // Check activity time
    let now = Utc::now();
    let elapsed = now - *last_activity;

    match elapsed.num_minutes() {
        0..=5 => SessionStatus::Active,
        6..=30 => SessionStatus::Idle,
        _ => SessionStatus::Stale,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::session::SessionState;

    #[test]
    fn test_detect_session_status() {
        let session = SessionState::new(
            "test-session".to_string(),
            "test-branch".to_string(),
            std::path::PathBuf::from("/test"),
        );

        // Test active status (< 5 minutes)
        let now = chrono::Utc::now();
        let status = detect_session_status(&session, &now);
        assert!(matches!(status, SessionStatus::Active));

        // Test idle status (10 minutes ago)
        let ten_minutes_ago = now - chrono::Duration::minutes(10);
        let status = detect_session_status(&session, &ten_minutes_ago);
        assert!(matches!(status, SessionStatus::Idle));

        // Test stale status (> 30 minutes)
        let hour_ago = now - chrono::Duration::hours(1);
        let status = detect_session_status(&session, &hour_ago);
        assert!(matches!(status, SessionStatus::Stale));
    }

    #[test]
    fn test_detect_session_status_ready() {
        let mut session = SessionState::new(
            "test-session".to_string(),
            "test-branch".to_string(),
            std::path::PathBuf::from("/test"),
        );
        session.update_status(CoreSessionStatus::Finished);

        let now = chrono::Utc::now();
        let status = detect_session_status(&session, &now);
        assert!(matches!(status, SessionStatus::Ready));
    }

    #[test]
    fn test_load_sessions() {
        use crate::config::Config;

        let config = Config {
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
                state_dir: "/tmp/.para_state_test".to_string(),
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
        };

        let service = SessionService::new(config);

        // Test loading sessions (should handle missing directory gracefully)
        let result = service.load_sessions(true);
        assert!(result.is_ok());

        // Test without stale sessions
        let result = service.load_sessions(false);
        assert!(result.is_ok());
    }
}
