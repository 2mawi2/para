use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionState {
    pub name: String,
    pub branch: String,
    pub worktree_path: PathBuf,
    pub created_at: DateTime<Utc>,
    pub status: SessionStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SessionStatus {
    Active,
    Finished,
    Cancelled,
}

impl SessionState {
    pub fn new(name: String, branch: String, worktree_path: PathBuf) -> Self {
        Self {
            name,
            branch,
            worktree_path,
            created_at: Utc::now(),
            status: SessionStatus::Active,
        }
    }

    pub fn update_status(&mut self, status: SessionStatus) {
        self.status = status;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_session_state_new() {
        let state = SessionState::new(
            "test-session".to_string(),
            "test/branch".to_string(),
            PathBuf::from("/path/to/worktree"),
        );

        assert_eq!(state.name, "test-session");
        assert_eq!(state.branch, "test/branch");
        assert_eq!(state.worktree_path, PathBuf::from("/path/to/worktree"));
        assert!(matches!(state.status, SessionStatus::Active));
    }

    #[test]
    fn test_session_state_update_status() {
        let mut state = SessionState::new(
            "test".to_string(),
            "test/branch".to_string(),
            PathBuf::from("/test"),
        );

        state.update_status(SessionStatus::Finished);
        assert!(matches!(state.status, SessionStatus::Finished));

        state.update_status(SessionStatus::Cancelled);
        assert!(matches!(state.status, SessionStatus::Cancelled));
    }
}