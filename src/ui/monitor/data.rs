/// Pure data models for UI monitoring system
/// This module contains business data without any presentation logic
use crate::core::status::{ConfidenceLevel, TestStatus};
use chrono::{DateTime, Utc};
use std::path::PathBuf;

/// Pure data representation of a session
/// Contains only business logic data, no presentation concerns
#[derive(Debug, Clone, PartialEq)]
pub struct SessionData {
    pub name: String,
    pub branch: String,
    pub status: SessionDataStatus,
    pub last_activity: DateTime<Utc>,
    pub task: String,
    pub worktree_path: PathBuf,
    // Agent status fields
    pub test_status: Option<TestStatus>,
    pub confidence: Option<ConfidenceLevel>,
    pub todo_percentage: Option<u8>,
    pub is_blocked: bool,
}

/// Pure data representation of session status
/// Contains only the business logic state, no presentation concerns
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SessionDataStatus {
    Active, // Recent activity (< 5 min)
    Idle,   // No activity (5-30 min)
    Review, // Finished, ready for review
    Ready,  // Finished, ready for review (legacy)
    Stale,  // No activity (> 30 min)
}

#[allow(dead_code)]
impl SessionDataStatus {
    /// Returns the string representation of the status
    pub fn as_str(&self) -> &'static str {
        match self {
            SessionDataStatus::Active => "Active",
            SessionDataStatus::Idle => "Idle",
            SessionDataStatus::Review => "Review",
            SessionDataStatus::Ready => "Ready",
            SessionDataStatus::Stale => "Stale",
        }
    }

    /// Returns true if this status represents a session that should be considered inactive
    pub fn is_inactive(&self) -> bool {
        matches!(self, SessionDataStatus::Stale)
    }

    /// Returns true if this status represents a session that needs attention
    pub fn needs_attention(&self) -> bool {
        matches!(self, SessionDataStatus::Review | SessionDataStatus::Ready)
    }

    /// Returns true if this status represents an actively worked session
    pub fn is_active(&self) -> bool {
        matches!(self, SessionDataStatus::Active)
    }
}

#[allow(dead_code)]
impl SessionData {
    /// Create a new SessionData instance
    pub fn new(
        name: String,
        branch: String,
        status: SessionDataStatus,
        last_activity: DateTime<Utc>,
        task: String,
        worktree_path: PathBuf,
    ) -> Self {
        Self {
            name,
            branch,
            status,
            last_activity,
            task,
            worktree_path,
            test_status: None,
            confidence: None,
            todo_percentage: None,
            is_blocked: false,
        }
    }

    /// Returns true if this session has test failures
    pub fn has_test_failures(&self) -> bool {
        matches!(self.test_status, Some(TestStatus::Failed))
    }

    /// Returns true if this session has low confidence
    pub fn has_low_confidence(&self) -> bool {
        matches!(self.confidence, Some(ConfidenceLevel::Low))
    }

    /// Returns the completion percentage, defaulting to 0 if not set
    pub fn completion_percentage(&self) -> u8 {
        self.todo_percentage.unwrap_or(0)
    }

    /// Returns true if this session is considered complete (100% progress)
    pub fn is_complete(&self) -> bool {
        self.todo_percentage == Some(100)
    }

    /// Returns true if this session needs immediate attention (blocked or failed tests)
    pub fn needs_immediate_attention(&self) -> bool {
        self.is_blocked || self.has_test_failures()
    }
}

/// Conversion trait to migrate from existing SessionInfo to SessionData
impl From<crate::ui::monitor::SessionInfo> for SessionData {
    fn from(session_info: crate::ui::monitor::SessionInfo) -> Self {
        Self {
            name: session_info.name,
            branch: session_info.branch,
            status: session_info.status.into(),
            last_activity: session_info.last_activity,
            task: session_info.task,
            worktree_path: session_info.worktree_path,
            test_status: session_info.test_status,
            confidence: session_info.confidence,
            todo_percentage: session_info.todo_percentage,
            is_blocked: session_info.is_blocked,
        }
    }
}

/// Conversion from old SessionStatus to new SessionDataStatus
impl From<crate::ui::monitor::SessionStatus> for SessionDataStatus {
    fn from(status: crate::ui::monitor::SessionStatus) -> Self {
        match status {
            crate::ui::monitor::SessionStatus::Active => SessionDataStatus::Active,
            crate::ui::monitor::SessionStatus::Idle => SessionDataStatus::Idle,
            crate::ui::monitor::SessionStatus::Review => SessionDataStatus::Review,
            crate::ui::monitor::SessionStatus::Ready => SessionDataStatus::Ready,
            crate::ui::monitor::SessionStatus::Stale => SessionDataStatus::Stale,
        }
    }
}

/// Conversion from new SessionDataStatus to old SessionStatus (for backward compatibility)
impl From<SessionDataStatus> for crate::ui::monitor::SessionStatus {
    fn from(status: SessionDataStatus) -> Self {
        match status {
            SessionDataStatus::Active => crate::ui::monitor::SessionStatus::Active,
            SessionDataStatus::Idle => crate::ui::monitor::SessionStatus::Idle,
            SessionDataStatus::Review => crate::ui::monitor::SessionStatus::Review,
            SessionDataStatus::Ready => crate::ui::monitor::SessionStatus::Ready,
            SessionDataStatus::Stale => crate::ui::monitor::SessionStatus::Stale,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn create_test_session_data() -> SessionData {
        SessionData::new(
            "test-session".to_string(),
            "test-branch".to_string(),
            SessionDataStatus::Active,
            Utc::now(),
            "Test task".to_string(),
            PathBuf::from("/test/path"),
        )
    }

    #[test]
    fn test_session_data_creation() {
        let session = create_test_session_data();
        assert_eq!(session.name, "test-session");
        assert_eq!(session.branch, "test-branch");
        assert_eq!(session.status, SessionDataStatus::Active);
        assert_eq!(session.task, "Test task");
    }

    #[test]
    fn test_session_data_status_string_representation() {
        assert_eq!(SessionDataStatus::Active.as_str(), "Active");
        assert_eq!(SessionDataStatus::Idle.as_str(), "Idle");
        assert_eq!(SessionDataStatus::Review.as_str(), "Review");
        assert_eq!(SessionDataStatus::Ready.as_str(), "Ready");
        assert_eq!(SessionDataStatus::Stale.as_str(), "Stale");
    }

    #[test]
    fn test_session_data_status_predicates() {
        assert!(SessionDataStatus::Stale.is_inactive());
        assert!(!SessionDataStatus::Active.is_inactive());

        assert!(SessionDataStatus::Review.needs_attention());
        assert!(SessionDataStatus::Ready.needs_attention());
        assert!(!SessionDataStatus::Active.needs_attention());

        assert!(SessionDataStatus::Active.is_active());
        assert!(!SessionDataStatus::Idle.is_active());
    }

    #[test]
    fn test_session_data_business_logic() {
        let mut session = create_test_session_data();

        // Test test failure detection
        session.test_status = Some(TestStatus::Failed);
        assert!(session.has_test_failures());

        session.test_status = Some(TestStatus::Passed);
        assert!(!session.has_test_failures());

        // Test confidence detection
        session.confidence = Some(ConfidenceLevel::Low);
        assert!(session.has_low_confidence());

        session.confidence = Some(ConfidenceLevel::High);
        assert!(!session.has_low_confidence());

        // Test completion percentage
        assert_eq!(session.completion_percentage(), 0); // Default
        session.todo_percentage = Some(85);
        assert_eq!(session.completion_percentage(), 85);

        session.todo_percentage = Some(100);
        assert!(session.is_complete());

        // Test immediate attention needs
        session.is_blocked = true;
        assert!(session.needs_immediate_attention());

        session.is_blocked = false;
        session.test_status = Some(TestStatus::Failed);
        assert!(session.needs_immediate_attention());
    }

    #[test]
    fn test_conversion_from_session_info() {
        use crate::ui::monitor::{SessionInfo, SessionStatus};

        let session_info = SessionInfo {
            name: "convert-test".to_string(),
            branch: "convert-branch".to_string(),
            status: SessionStatus::Review,
            last_activity: Utc::now(),
            task: "Convert task".to_string(),
            worktree_path: PathBuf::from("/convert/path"),
            test_status: Some(TestStatus::Passed),
            confidence: Some(ConfidenceLevel::High),
            todo_percentage: Some(75),
            is_blocked: false,
        };

        let session_data: SessionData = session_info.into();
        assert_eq!(session_data.name, "convert-test");
        assert_eq!(session_data.status, SessionDataStatus::Review);
        assert_eq!(session_data.todo_percentage, Some(75));
    }

    #[test]
    fn test_status_conversion_roundtrip() {
        let original_status = SessionDataStatus::Review;
        let old_status: crate::ui::monitor::SessionStatus = original_status.into();
        let back_to_new: SessionDataStatus = old_status.into();
        assert_eq!(original_status, back_to_new);
    }
}
