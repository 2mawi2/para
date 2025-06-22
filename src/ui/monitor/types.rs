use crate::core::status::{ConfidenceLevel, DiffStats, TestStatus};
use chrono::{DateTime, Utc};
use ratatui::style::Color;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct SessionInfo {
    pub name: String,
    pub branch: String,
    pub status: SessionStatus,
    pub last_activity: DateTime<Utc>,
    pub task: String,
    pub worktree_path: PathBuf,
    // Agent status fields
    pub test_status: Option<TestStatus>,
    pub confidence: Option<ConfidenceLevel>, // Keep for backwards compatibility
    pub diff_stats: Option<DiffStats>,
    pub todo_percentage: Option<u8>,
    pub is_blocked: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SessionStatus {
    Active, // 🟢 Recent activity (< 5 min)
    Idle,   // 🟡 No activity (5-30 min)
    Review, // 👀 Finished, ready for review
    Ready,  // ✅ Finished, ready for review (legacy)
    Stale,  // ⏸️  No activity (> 30 min)
}

impl SessionStatus {
    pub fn name(&self) -> &str {
        match self {
            SessionStatus::Active => "Active",
            SessionStatus::Idle => "Idle",
            SessionStatus::Review => "Review",
            SessionStatus::Ready => "Ready",
            SessionStatus::Stale => "Stale",
        }
    }

    pub fn color(&self) -> Color {
        match self {
            SessionStatus::Active => Color::Rgb(34, 197, 94), // Green
            SessionStatus::Idle => Color::Rgb(245, 158, 11),  // Amber
            SessionStatus::Review => Color::Rgb(147, 51, 234), // Purple
            SessionStatus::Ready => Color::Rgb(99, 102, 241), // Indigo
            SessionStatus::Stale => Color::Rgb(107, 114, 128), // Gray
        }
    }

    /// Returns true if this session status should be rendered with dimmed/transparent appearance
    pub fn should_dim(&self) -> bool {
        matches!(self, SessionStatus::Stale)
    }

    /// Returns a dimmed version of the standard text color for stale sessions
    pub fn dimmed_text_color() -> Color {
        Color::Rgb(75, 85, 99) // Darker gray for stale session text
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AppMode {
    Normal,
    FinishPrompt,
    CancelConfirm,
    ErrorDialog,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_status_name() {
        assert_eq!(SessionStatus::Active.name(), "Active");
        assert_eq!(SessionStatus::Idle.name(), "Idle");
        assert_eq!(SessionStatus::Review.name(), "Review");
        assert_eq!(SessionStatus::Ready.name(), "Ready");
        assert_eq!(SessionStatus::Stale.name(), "Stale");
    }

    #[test]
    fn test_session_status_color() {
        assert_eq!(SessionStatus::Active.color(), Color::Rgb(34, 197, 94));
        assert_eq!(SessionStatus::Idle.color(), Color::Rgb(245, 158, 11));
        assert_eq!(SessionStatus::Review.color(), Color::Rgb(147, 51, 234));
        assert_eq!(SessionStatus::Ready.color(), Color::Rgb(99, 102, 241));
        assert_eq!(SessionStatus::Stale.color(), Color::Rgb(107, 114, 128));
    }

    #[test]
    fn test_session_status_should_dim() {
        assert!(!SessionStatus::Active.should_dim());
        assert!(!SessionStatus::Idle.should_dim());
        assert!(!SessionStatus::Review.should_dim());
        assert!(!SessionStatus::Ready.should_dim());
        assert!(SessionStatus::Stale.should_dim());
    }

    #[test]
    fn test_dimmed_text_color() {
        assert_eq!(SessionStatus::dimmed_text_color(), Color::Rgb(75, 85, 99));
    }
}
