use crate::core::status::{ConfidenceLevel, TestStatus};
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
    pub confidence: Option<ConfidenceLevel>,
    pub todo_percentage: Option<u8>,
    pub is_blocked: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SessionStatus {
    Active, // 🟢 Recent activity (< 5 min)
    Idle,   // 🟡 No activity (5-30 min)
    Ready,  // ✅ Finished, ready for review
    Stale,  // ⏸️  No activity (> 30 min)
}

impl SessionStatus {
    pub fn name(&self) -> &str {
        match self {
            SessionStatus::Active => "Active",
            SessionStatus::Idle => "Idle",
            SessionStatus::Ready => "Ready",
            SessionStatus::Stale => "Stale",
        }
    }

    pub fn color(&self) -> Color {
        match self {
            SessionStatus::Active => Color::Rgb(34, 197, 94), // Green
            SessionStatus::Idle => Color::Rgb(245, 158, 11),  // Amber
            SessionStatus::Ready => Color::Rgb(99, 102, 241), // Indigo
            SessionStatus::Stale => Color::Rgb(107, 114, 128), // Gray
        }
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
        assert_eq!(SessionStatus::Ready.name(), "Ready");
        assert_eq!(SessionStatus::Stale.name(), "Stale");
    }

    #[test]
    fn test_session_status_color() {
        assert_eq!(SessionStatus::Active.color(), Color::Rgb(34, 197, 94));
        assert_eq!(SessionStatus::Idle.color(), Color::Rgb(245, 158, 11));
        assert_eq!(SessionStatus::Ready.color(), Color::Rgb(99, 102, 241));
        assert_eq!(SessionStatus::Stale.color(), Color::Rgb(107, 114, 128));
    }
}
