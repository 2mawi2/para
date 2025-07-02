use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

fn default_session_type() -> SessionType {
    SessionType::Worktree
}

/// Type of session - either traditional worktree or Docker container
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SessionType {
    /// Traditional git worktree session
    Worktree,
    /// Docker container session
    Container {
        /// Container ID assigned by Docker
        container_id: Option<String>,
    },
}

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

    // Session type - worktree or container
    #[serde(default = "default_session_type")]
    pub session_type: SessionType,

    // Parent branch this session was created from
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub parent_branch: Option<String>,

    // Deprecated - use session_type instead
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub is_docker: Option<bool>,

    // Whether session was created with dangerous_skip_permissions flag
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub dangerous_skip_permissions: Option<bool>,

    // Sandbox settings for the session
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub sandbox_enabled: Option<bool>,

    // Sandbox profile (permissive or restrictive)
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub sandbox_profile: Option<String>,
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
            session_type: SessionType::Worktree,
            parent_branch: None,
            is_docker: None,
            dangerous_skip_permissions: None,
            sandbox_enabled: None,
            sandbox_profile: None,
        }
    }

    #[cfg(test)]
    pub fn with_parent_branch_and_flags(
        name: String,
        branch: String,
        worktree_path: PathBuf,
        parent_branch: String,
        dangerous_skip_permissions: bool,
    ) -> Self {
        Self {
            name,
            branch,
            worktree_path,
            created_at: Utc::now(),
            status: SessionStatus::Active,
            task_description: None,
            last_activity: None,
            git_stats: None,
            session_type: SessionType::Worktree,
            parent_branch: Some(parent_branch),
            is_docker: None,
            dangerous_skip_permissions: if dangerous_skip_permissions {
                Some(true)
            } else {
                None
            },
            sandbox_enabled: None,
            sandbox_profile: None,
        }
    }

    /// Create a new container-based session with parent branch and flags
    pub fn new_container_with_parent_branch_and_flags(
        name: String,
        branch: String,
        worktree_path: PathBuf,
        container_id: Option<String>,
        parent_branch: String,
        dangerous_skip_permissions: bool,
    ) -> Self {
        Self {
            name,
            branch,
            worktree_path,
            created_at: Utc::now(),
            status: SessionStatus::Active,
            task_description: None,
            last_activity: None,
            git_stats: None,
            session_type: SessionType::Container { container_id },
            parent_branch: Some(parent_branch),
            is_docker: None,
            dangerous_skip_permissions: if dangerous_skip_permissions {
                Some(true)
            } else {
                None
            },
            sandbox_enabled: None,
            sandbox_profile: None,
        }
    }

    /// Create a new session with all flags including sandbox settings
    pub fn with_all_flags(
        name: String,
        branch: String,
        worktree_path: PathBuf,
        parent_branch: String,
        dangerous_skip_permissions: bool,
        sandbox_enabled: bool,
        sandbox_profile: Option<String>,
    ) -> Self {
        Self {
            name,
            branch,
            worktree_path,
            created_at: Utc::now(),
            status: SessionStatus::Active,
            task_description: None,
            last_activity: None,
            git_stats: None,
            session_type: SessionType::Worktree,
            parent_branch: Some(parent_branch),
            is_docker: None,
            dangerous_skip_permissions: if dangerous_skip_permissions {
                Some(true)
            } else {
                None
            },
            sandbox_enabled: if sandbox_enabled { Some(true) } else { None },
            sandbox_profile,
        }
    }

    /// Check if this is a container session
    pub fn is_container(&self) -> bool {
        matches!(self.session_type, SessionType::Container { .. })
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
        assert_eq!(state.session_type, SessionType::Worktree);
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
            session_type: SessionType::Worktree,
            parent_branch: None,
            is_docker: None,
            dangerous_skip_permissions: None,
            sandbox_enabled: None,
            sandbox_profile: None,
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

    #[test]
    fn test_container_session_state() {
        let state = SessionState::new_container_with_parent_branch_and_flags(
            "container-session".to_string(),
            "test/container-branch".to_string(),
            PathBuf::from("/path/to/worktree"),
            Some("abc123".to_string()),
            "main".to_string(),
            false,
        );

        assert_eq!(state.name, "container-session");
        assert!(state.is_container());
        // Verify container has the expected ID
        if let SessionType::Container { container_id } = &state.session_type {
            assert_eq!(container_id.as_deref(), Some("abc123"));
        } else {
            panic!("Expected container type");
        }
    }

    #[test]
    fn test_session_type_serialization() {
        // Test worktree type
        let worktree_session = SessionState::new(
            "worktree".to_string(),
            "test/worktree".to_string(),
            PathBuf::from("/test"),
        );
        let json = serde_json::to_string(&worktree_session).unwrap();
        assert!(json.contains(r#""session_type":"Worktree""#));

        // Test container type
        let container_session = SessionState::new_container_with_parent_branch_and_flags(
            "container".to_string(),
            "test/container".to_string(),
            PathBuf::from("/test"),
            Some("xyz789".to_string()),
            "main".to_string(),
            false,
        );
        let json = serde_json::to_string(&container_session).unwrap();
        assert!(json.contains(r#""session_type":{"Container""#));
        assert!(json.contains("xyz789"));
    }

    #[test]
    fn test_parent_branch_field() {
        // Test new() constructor - should have None parent_branch
        let state = SessionState::new(
            "test-session".to_string(),
            "para/test-session".to_string(),
            PathBuf::from("/test"),
        );
        assert_eq!(state.parent_branch, None);

        // Test with_parent_branch() constructor
        let state_with_parent = SessionState::with_parent_branch_and_flags(
            "feature-session".to_string(),
            "para/feature-session".to_string(),
            PathBuf::from("/test"),
            "develop".to_string(),
            false,
        );
        assert_eq!(state_with_parent.parent_branch, Some("develop".to_string()));
    }

    #[test]
    fn test_parent_branch_serialization() {
        // Test serialization with parent_branch
        let state = SessionState::with_parent_branch_and_flags(
            "test".to_string(),
            "para/test".to_string(),
            PathBuf::from("/test"),
            "main".to_string(),
            false,
        );
        let json = serde_json::to_string(&state).unwrap();
        assert!(json.contains(r#""parent_branch":"main""#));

        // Test serialization without parent_branch (should not include field)
        let state_no_parent = SessionState::new(
            "test".to_string(),
            "para/test".to_string(),
            PathBuf::from("/test"),
        );
        let json = serde_json::to_string(&state_no_parent).unwrap();
        assert!(!json.contains("parent_branch"));
    }

    #[test]
    fn test_parent_branch_deserialization() {
        // Test deserializing old sessions without parent_branch field
        let old_json = r#"{
            "name": "old-session",
            "branch": "para/old-session",
            "worktree_path": "/test",
            "created_at": "2024-01-01T00:00:00Z",
            "status": "Active",
            "session_type": "Worktree"
        }"#;
        let deserialized: SessionState = serde_json::from_str(old_json).unwrap();
        assert_eq!(deserialized.parent_branch, None);

        // Test deserializing new sessions with parent_branch field
        let new_json = r#"{
            "name": "new-session",
            "branch": "para/new-session",
            "worktree_path": "/test",
            "created_at": "2024-01-01T00:00:00Z",
            "status": "Active",
            "session_type": "Worktree",
            "parent_branch": "feature/base"
        }"#;
        let deserialized: SessionState = serde_json::from_str(new_json).unwrap();
        assert_eq!(deserialized.parent_branch, Some("feature/base".to_string()));
    }

    #[test]
    fn test_dangerous_skip_permissions_field() {
        // Test new() constructor - should have None dangerous_skip_permissions
        let state = SessionState::new(
            "test-session".to_string(),
            "para/test-session".to_string(),
            PathBuf::from("/test"),
        );
        assert_eq!(state.dangerous_skip_permissions, None);

        // Test setting dangerous_skip_permissions
        let mut state_with_flag = SessionState::new(
            "dangerous-session".to_string(),
            "para/dangerous-session".to_string(),
            PathBuf::from("/test"),
        );
        state_with_flag.dangerous_skip_permissions = Some(true);
        assert_eq!(state_with_flag.dangerous_skip_permissions, Some(true));
    }

    #[test]
    fn test_dangerous_skip_permissions_serialization() {
        // Test serialization with dangerous_skip_permissions
        let mut state = SessionState::new(
            "test".to_string(),
            "para/test".to_string(),
            PathBuf::from("/test"),
        );
        state.dangerous_skip_permissions = Some(true);
        let json = serde_json::to_string(&state).unwrap();
        assert!(json.contains(r#""dangerous_skip_permissions":true"#));

        // Test serialization without dangerous_skip_permissions (should not include field)
        let state_no_flag = SessionState::new(
            "test".to_string(),
            "para/test".to_string(),
            PathBuf::from("/test"),
        );
        let json = serde_json::to_string(&state_no_flag).unwrap();
        assert!(!json.contains("dangerous_skip_permissions"));
    }

    #[test]
    fn test_sandbox_fields() {
        // Test new() constructor - should have None sandbox fields
        let state = SessionState::new(
            "test-session".to_string(),
            "para/test-session".to_string(),
            PathBuf::from("/test"),
        );
        assert_eq!(state.sandbox_enabled, None);
        assert_eq!(state.sandbox_profile, None);

        // Test with_all_flags constructor with sandbox enabled
        let state_with_sandbox = SessionState::with_all_flags(
            "sandbox-session".to_string(),
            "para/sandbox-session".to_string(),
            PathBuf::from("/test"),
            "main".to_string(),
            false,
            true,
            Some("permissive".to_string()),
        );
        assert_eq!(state_with_sandbox.sandbox_enabled, Some(true));
        assert_eq!(
            state_with_sandbox.sandbox_profile,
            Some("permissive".to_string())
        );

        // Test with_all_flags constructor with sandbox disabled
        let state_no_sandbox = SessionState::with_all_flags(
            "no-sandbox-session".to_string(),
            "para/no-sandbox-session".to_string(),
            PathBuf::from("/test"),
            "main".to_string(),
            false,
            false,
            None,
        );
        assert_eq!(state_no_sandbox.sandbox_enabled, None);
        assert_eq!(state_no_sandbox.sandbox_profile, None);
    }

    #[test]
    fn test_sandbox_serialization() {
        // Test serialization with sandbox settings
        let state = SessionState::with_all_flags(
            "test".to_string(),
            "para/test".to_string(),
            PathBuf::from("/test"),
            "main".to_string(),
            false,
            true,
            Some("restrictive".to_string()),
        );
        let json = serde_json::to_string(&state).unwrap();
        assert!(json.contains(r#""sandbox_enabled":true"#));
        assert!(json.contains(r#""sandbox_profile":"restrictive""#));

        // Test serialization without sandbox settings (should not include fields)
        let state_no_sandbox = SessionState::new(
            "test".to_string(),
            "para/test".to_string(),
            PathBuf::from("/test"),
        );
        let json = serde_json::to_string(&state_no_sandbox).unwrap();
        assert!(!json.contains("sandbox_enabled"));
        assert!(!json.contains("sandbox_profile"));
    }

    #[test]
    fn test_sandbox_deserialization() {
        // Test deserializing old sessions without sandbox fields
        let old_json = r#"{
            "name": "old-session",
            "branch": "para/old-session",
            "worktree_path": "/test",
            "created_at": "2024-01-01T00:00:00Z",
            "status": "Active",
            "session_type": "Worktree"
        }"#;
        let deserialized: SessionState = serde_json::from_str(old_json).unwrap();
        assert_eq!(deserialized.sandbox_enabled, None);
        assert_eq!(deserialized.sandbox_profile, None);

        // Test deserializing new sessions with sandbox fields
        let new_json = r#"{
            "name": "new-session",
            "branch": "para/new-session",
            "worktree_path": "/test",
            "created_at": "2024-01-01T00:00:00Z",
            "status": "Active",
            "session_type": "Worktree",
            "sandbox_enabled": true,
            "sandbox_profile": "permissive"
        }"#;
        let deserialized: SessionState = serde_json::from_str(new_json).unwrap();
        assert_eq!(deserialized.sandbox_enabled, Some(true));
        assert_eq!(deserialized.sandbox_profile, Some("permissive".to_string()));
    }

    #[test]
    fn test_dangerous_skip_permissions_deserialization() {
        // Test deserializing old sessions without dangerous_skip_permissions field
        let old_json = r#"{
            "name": "old-session",
            "branch": "para/old-session",
            "worktree_path": "/test",
            "created_at": "2024-01-01T00:00:00Z",
            "status": "Active",
            "session_type": "Worktree"
        }"#;
        let deserialized: SessionState = serde_json::from_str(old_json).unwrap();
        assert_eq!(deserialized.dangerous_skip_permissions, None);

        // Test deserializing new sessions with dangerous_skip_permissions field
        let new_json = r#"{
            "name": "new-session",
            "branch": "para/new-session",
            "worktree_path": "/test",
            "created_at": "2024-01-01T00:00:00Z",
            "status": "Active",
            "session_type": "Worktree",
            "dangerous_skip_permissions": true
        }"#;
        let deserialized: SessionState = serde_json::from_str(new_json).unwrap();
        assert_eq!(deserialized.dangerous_skip_permissions, Some(true));
    }
}
