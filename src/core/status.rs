use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

use crate::utils::ParaError;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DiffStats {
    pub additions: usize,
    pub deletions: usize,
}

impl DiffStats {
    pub fn new(additions: usize, deletions: usize) -> Self {
        Self {
            additions,
            deletions,
        }
    }
}

impl std::fmt::Display for DiffStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "+{} -{}", self.additions, self.deletions)
    }
}

/// Represents the current status of a para session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Status {
    pub session_name: String,
    pub current_task: String,
    pub test_status: TestStatus,
    pub is_blocked: bool,
    pub blocked_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub todos_completed: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub todos_total: Option<u32>,
    pub confidence: ConfidenceLevel,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diff_stats: Option<DiffStats>,
    pub last_update: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TestStatus {
    Passed,
    Failed,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ConfidenceLevel {
    High,
    Medium,
    Low,
}

impl Status {
    pub fn new(
        session_name: String,
        current_task: String,
        test_status: TestStatus,
        confidence: ConfidenceLevel,
    ) -> Self {
        Self {
            session_name,
            current_task,
            test_status,
            is_blocked: false,
            blocked_reason: None,
            todos_completed: None,
            todos_total: None,
            confidence,
            diff_stats: None,
            last_update: Utc::now(),
        }
    }

    pub fn with_blocked(mut self, blocked_reason: Option<String>) -> Self {
        self.is_blocked = blocked_reason.is_some();
        self.blocked_reason = blocked_reason;
        self
    }

    pub fn with_todos(mut self, completed: u32, total: u32) -> Self {
        self.todos_completed = Some(completed);
        self.todos_total = Some(total);
        self
    }

    pub fn with_diff_stats(mut self, diff_stats: DiffStats) -> Self {
        self.diff_stats = Some(diff_stats);
        self
    }

    pub fn status_file_path(state_dir: &Path, session_name: &str) -> PathBuf {
        state_dir.join(format!("{}.status.json", session_name))
    }

    pub fn save(&self, state_dir: &Path) -> Result<()> {
        let status_file = Self::status_file_path(state_dir, &self.session_name);

        if let Some(parent) = status_file.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                ParaError::fs_error(format!("Failed to create state directory: {}", e))
            })?;
        }

        let json = serde_json::to_string_pretty(self)
            .map_err(|e| ParaError::config_error(format!("Failed to serialize status: {}", e)))?;

        fs::write(&status_file, json)
            .map_err(|e| ParaError::fs_error(format!("Failed to write status file: {}", e)))?;

        Ok(())
    }

    pub fn load(state_dir: &Path, session_name: &str) -> Result<Option<Self>> {
        let status_file = Self::status_file_path(state_dir, session_name);

        if !status_file.exists() {
            return Ok(None);
        }

        let json = fs::read_to_string(&status_file)
            .map_err(|e| ParaError::fs_error(format!("Failed to read status file: {}", e)))?;

        let status: Status = serde_json::from_str(&json)
            .map_err(|e| ParaError::config_error(format!("Failed to parse status file: {}", e)))?;

        Ok(Some(status))
    }

    pub fn parse_test_status(s: &str) -> Result<TestStatus> {
        match s.to_lowercase().as_str() {
            "passed" => Ok(TestStatus::Passed),
            "failed" => Ok(TestStatus::Failed),
            "unknown" => Ok(TestStatus::Unknown),
            _ => Err(ParaError::invalid_args(
                "Test status must be 'passed', 'failed', or 'unknown'",
            )
            .into()),
        }
    }

    pub fn parse_confidence(s: &str) -> Result<ConfidenceLevel> {
        match s.to_lowercase().as_str() {
            "high" => Ok(ConfidenceLevel::High),
            "medium" => Ok(ConfidenceLevel::Medium),
            "low" => Ok(ConfidenceLevel::Low),
            _ => {
                Err(ParaError::invalid_args("Confidence must be 'high', 'medium', or 'low'").into())
            }
        }
    }

    pub fn parse_todos(s: &str) -> Result<(u32, u32)> {
        let parts: Vec<&str> = s.split('/').collect();
        if parts.len() != 2 {
            return Err(ParaError::invalid_args(
                "Todos must be in format 'completed/total' (e.g., '3/7')",
            )
            .into());
        }

        let completed = parts[0]
            .parse::<u32>()
            .map_err(|_| ParaError::invalid_args("Invalid completed todos number"))?;

        let total = parts[1]
            .parse::<u32>()
            .map_err(|_| ParaError::invalid_args("Invalid total todos number"))?;

        if completed > total {
            return Err(ParaError::invalid_args(
                "Completed todos cannot be greater than total todos",
            )
            .into());
        }

        Ok((completed, total))
    }

    pub fn todo_percentage(&self) -> Option<u8> {
        match (self.todos_completed, self.todos_total) {
            (Some(completed), Some(total)) if total > 0 => {
                Some(((completed as f32 / total as f32) * 100.0).round() as u8)
            }
            _ => None,
        }
    }

    pub fn format_todos(&self) -> Option<String> {
        match (self.todos_completed, self.todos_total) {
            (Some(completed), Some(total)) => {
                let percentage = self.todo_percentage().unwrap_or(0);
                Some(format!("{}% ({}/{})", percentage, completed, total))
            }
            _ => None,
        }
    }
}

impl std::fmt::Display for TestStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TestStatus::Passed => write!(f, "Passed"),
            TestStatus::Failed => write!(f, "Failed"),
            TestStatus::Unknown => write!(f, "Unknown"),
        }
    }
}

impl std::fmt::Display for ConfidenceLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfidenceLevel::High => write!(f, "High"),
            ConfidenceLevel::Medium => write!(f, "Medium"),
            ConfidenceLevel::Low => write!(f, "Low"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_status_new() {
        let status = Status::new(
            "test-session".to_string(),
            "Working on feature".to_string(),
            TestStatus::Passed,
            ConfidenceLevel::High,
        );

        assert_eq!(status.session_name, "test-session");
        assert_eq!(status.current_task, "Working on feature");
        assert_eq!(status.test_status, TestStatus::Passed);
        assert_eq!(status.confidence, ConfidenceLevel::High);
        assert!(!status.is_blocked);
        assert!(status.blocked_reason.is_none());
        assert!(status.todos_completed.is_none());
        assert!(status.todos_total.is_none());
    }

    #[test]
    fn test_status_with_blocked() {
        let status = Status::new(
            "test-session".to_string(),
            "Fixing tests".to_string(),
            TestStatus::Failed,
            ConfidenceLevel::Low,
        )
        .with_blocked(Some("Need help with Redis mocking".to_string()));

        assert!(status.is_blocked);
        assert_eq!(
            status.blocked_reason,
            Some("Need help with Redis mocking".to_string())
        );
    }

    #[test]
    fn test_status_with_todos() {
        let status = Status::new(
            "test-session".to_string(),
            "Implementing feature".to_string(),
            TestStatus::Unknown,
            ConfidenceLevel::Medium,
        )
        .with_todos(3, 7);

        assert_eq!(status.todos_completed, Some(3));
        assert_eq!(status.todos_total, Some(7));
        assert_eq!(status.todo_percentage(), Some(43));
        assert_eq!(status.format_todos(), Some("43% (3/7)".to_string()));
    }

    #[test]
    fn test_save_and_load_status() {
        let temp_dir = TempDir::new().unwrap();
        let state_dir = temp_dir.path();

        let status = Status::new(
            "test-session".to_string(),
            "Working on tests".to_string(),
            TestStatus::Passed,
            ConfidenceLevel::High,
        )
        .with_todos(5, 10);

        // Save status
        status.save(state_dir).unwrap();

        // Load status
        let loaded = Status::load(state_dir, "test-session").unwrap().unwrap();

        assert_eq!(loaded.session_name, status.session_name);
        assert_eq!(loaded.current_task, status.current_task);
        assert_eq!(loaded.test_status, status.test_status);
        assert_eq!(loaded.confidence, status.confidence);
        assert_eq!(loaded.todos_completed, status.todos_completed);
        assert_eq!(loaded.todos_total, status.todos_total);
    }

    #[test]
    fn test_load_missing_status() {
        let temp_dir = TempDir::new().unwrap();
        let state_dir = temp_dir.path();

        let result = Status::load(state_dir, "nonexistent").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_test_status() {
        assert_eq!(
            Status::parse_test_status("passed").unwrap(),
            TestStatus::Passed
        );
        assert_eq!(
            Status::parse_test_status("FAILED").unwrap(),
            TestStatus::Failed
        );
        assert_eq!(
            Status::parse_test_status("Unknown").unwrap(),
            TestStatus::Unknown
        );
        assert!(Status::parse_test_status("invalid").is_err());
    }

    #[test]
    fn test_parse_confidence() {
        assert_eq!(
            Status::parse_confidence("high").unwrap(),
            ConfidenceLevel::High
        );
        assert_eq!(
            Status::parse_confidence("MEDIUM").unwrap(),
            ConfidenceLevel::Medium
        );
        assert_eq!(
            Status::parse_confidence("Low").unwrap(),
            ConfidenceLevel::Low
        );
        assert!(Status::parse_confidence("invalid").is_err());
    }

    #[test]
    fn test_parse_todos() {
        assert_eq!(Status::parse_todos("3/7").unwrap(), (3, 7));
        assert_eq!(Status::parse_todos("0/10").unwrap(), (0, 10));
        assert_eq!(Status::parse_todos("10/10").unwrap(), (10, 10));

        // Invalid formats
        assert!(Status::parse_todos("3").is_err());
        assert!(Status::parse_todos("3/7/10").is_err());
        assert!(Status::parse_todos("a/b").is_err());
        assert!(Status::parse_todos("5/3").is_err()); // completed > total
    }

    #[test]
    fn test_todo_percentage() {
        let status = Status::new(
            "test".to_string(),
            "task".to_string(),
            TestStatus::Unknown,
            ConfidenceLevel::Medium,
        );

        // No todos set
        assert_eq!(status.todo_percentage(), None);

        // With todos
        assert_eq!(status.clone().with_todos(0, 10).todo_percentage(), Some(0));
        assert_eq!(status.clone().with_todos(3, 10).todo_percentage(), Some(30));
        assert_eq!(status.clone().with_todos(7, 10).todo_percentage(), Some(70));
        assert_eq!(
            status.clone().with_todos(10, 10).todo_percentage(),
            Some(100)
        );
        assert_eq!(status.clone().with_todos(1, 3).todo_percentage(), Some(33));
    }

    #[test]
    fn test_format_todos() {
        let status = Status::new(
            "test".to_string(),
            "task".to_string(),
            TestStatus::Unknown,
            ConfidenceLevel::Medium,
        );

        // No todos
        assert_eq!(status.format_todos(), None);

        // With todos
        assert_eq!(
            status.clone().with_todos(3, 7).format_todos(),
            Some("43% (3/7)".to_string())
        );
        assert_eq!(
            status.clone().with_todos(0, 5).format_todos(),
            Some("0% (0/5)".to_string())
        );
        assert_eq!(
            status.clone().with_todos(10, 10).format_todos(),
            Some("100% (10/10)".to_string())
        );
    }

    #[test]
    fn test_status_json_serialization() {
        let status = Status::new(
            "my-session".to_string(),
            "Implementing auth".to_string(),
            TestStatus::Failed,
            ConfidenceLevel::Low,
        )
        .with_blocked(Some("Need Redis help".to_string()))
        .with_todos(2, 5);

        let json = serde_json::to_string_pretty(&status).unwrap();
        let parsed: Status = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.session_name, status.session_name);
        assert_eq!(parsed.current_task, status.current_task);
        assert_eq!(parsed.test_status, status.test_status);
        assert_eq!(parsed.confidence, status.confidence);
        assert_eq!(parsed.is_blocked, status.is_blocked);
        assert_eq!(parsed.blocked_reason, status.blocked_reason);
        assert_eq!(parsed.todos_completed, status.todos_completed);
        assert_eq!(parsed.todos_total, status.todos_total);
    }

    #[test]
    fn test_status_json_without_optional_fields() {
        let status = Status::new(
            "minimal".to_string(),
            "Basic task".to_string(),
            TestStatus::Passed,
            ConfidenceLevel::High,
        );

        let json = serde_json::to_string(&status).unwrap();

        // Verify optional fields are not included
        assert!(!json.contains("todos_completed"));
        assert!(!json.contains("todos_total"));
        assert!(json.contains("\"blocked_reason\":null"));
    }
}
