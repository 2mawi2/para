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

    // New fields for monitor UI
    pub task_description: Option<String>,
    pub last_activity: Option<DateTime<Utc>>,
    pub git_stats: Option<GitStats>,

    // Docker support
    pub is_docker: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitStats {
    pub files_changed: u32,
    pub lines_added: u32,
    pub lines_removed: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SessionStatus {
    Active,
    Review,
    Finished, // Deprecated - use Review instead
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
            task_description: None,
            last_activity: None,
            git_stats: None,
            is_docker: false,
        }
    }

    pub fn update_status(&mut self, status: SessionStatus) {
        self.status = status;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn test_session_lifecycle_active_to_review() {
        let mut state = SessionState::new(
            "feature-session".to_string(),
            "para/feature-session".to_string(),
            PathBuf::from("/repo/.para/worktrees/feature-session"),
        );

        // Should start as Active
        assert!(matches!(state.status, SessionStatus::Active));

        // When work is finished, should transition to Review
        state.update_status(SessionStatus::Review);
        assert!(matches!(state.status, SessionStatus::Review));
    }

    #[test]
    fn test_session_lifecycle_review_status_properties() {
        let mut state = SessionState::new(
            "review-session".to_string(),
            "para/review-session".to_string(),
            PathBuf::from("/repo/.para/worktrees/review-session"),
        );

        // Transition to Review status
        state.update_status(SessionStatus::Review);

        // Review sessions should:
        // 1. Still have branch information (for merging)
        assert_eq!(state.branch, "para/review-session");

        // 2. Still have session name (for identification)
        assert_eq!(state.name, "review-session");

        // 3. Be in Review status
        assert!(matches!(state.status, SessionStatus::Review));
    }

    #[test]
    fn test_session_status_display_behavior() {
        // Test that Review status can be properly serialized/deserialized
        let state = SessionState {
            name: "test".to_string(),
            branch: "para/test".to_string(),
            worktree_path: PathBuf::from("/test"),
            created_at: Utc::now(),
            status: SessionStatus::Review,
            task_description: Some("Completed feature implementation".to_string()),
            last_activity: None,
            git_stats: None,
            is_docker: false,
        };

        // Should be able to serialize and deserialize Review status
        let json = serde_json::to_string(&state).unwrap();
        let deserialized: SessionState = serde_json::from_str(&json).unwrap();
        assert!(matches!(deserialized.status, SessionStatus::Review));
        assert_eq!(deserialized.name, "test");
    }

    #[test]
    fn test_session_lifecycle_all_valid_transitions() {
        let mut state = SessionState::new(
            "transition-test".to_string(),
            "para/transition-test".to_string(),
            PathBuf::from("/test"),
        );

        // Active -> Review (normal completion)
        assert!(matches!(state.status, SessionStatus::Active));
        state.update_status(SessionStatus::Review);
        assert!(matches!(state.status, SessionStatus::Review));

        // Reset for next test
        state.update_status(SessionStatus::Active);

        // Active -> Cancelled (user cancellation)
        state.update_status(SessionStatus::Cancelled);
        assert!(matches!(state.status, SessionStatus::Cancelled));
    }
}
