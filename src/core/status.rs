use anyhow::Result;
use chrono::{DateTime, Utc};
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
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

/// Aggregated status information for monitor display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusSummary {
    pub total_sessions: usize,
    pub active_sessions: usize,
    pub blocked_sessions: usize,
    pub stale_sessions: usize,
    pub test_summary: TestSummary,
    pub overall_progress: Option<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestSummary {
    pub passed: usize,
    pub failed: usize,
    pub unknown: usize,
}

impl Status {
    pub fn new(session_name: String, current_task: String, test_status: TestStatus) -> Self {
        Self {
            session_name,
            current_task,
            test_status,
            is_blocked: false,
            blocked_reason: None,
            todos_completed: None,
            todos_total: None,
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
        state_dir.join(format!("{session_name}.status.json"))
    }

    pub fn save(&self, state_dir: &Path) -> Result<()> {
        let status_file = Self::status_file_path(state_dir, &self.session_name);

        if let Some(parent) = status_file.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                ParaError::fs_error(format!("Failed to create state directory: {e}"))
            })?;
        }

        let json = serde_json::to_string_pretty(self)
            .map_err(|e| ParaError::config_error(format!("Failed to serialize status: {e}")))?;

        // Write to a temporary file first, then rename atomically
        // Use a unique temp file name to avoid conflicts in concurrent writes
        use rand::Rng;
        let random_id: u32 = rand::thread_rng().gen();
        let temp_file = status_file.with_extension(format!("tmp.{random_id}"));

        // Use file locking to prevent race conditions
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&temp_file)
            .map_err(|e| ParaError::fs_error(format!("Failed to open temp status file: {e}")))?;

        // Lock the file exclusively for writing
        file.lock_exclusive()
            .map_err(|e| ParaError::fs_error(format!("Failed to lock status file: {e}")))?;

        // Write the content
        use std::io::Write;
        file.write_all(json.as_bytes())
            .map_err(|e| ParaError::fs_error(format!("Failed to write status file: {e}")))?;

        // Sync to disk
        file.sync_all()
            .map_err(|e| ParaError::fs_error(format!("Failed to sync status file: {e}")))?;

        // Explicitly drop to release lock before rename
        drop(file);

        // Rename atomically
        fs::rename(temp_file, status_file)
            .map_err(|e| ParaError::fs_error(format!("Failed to rename status file: {e}")))?;

        Ok(())
    }

    pub fn load(state_dir: &Path, session_name: &str) -> Result<Option<Self>> {
        let status_file = Self::status_file_path(state_dir, session_name);

        if !status_file.exists() {
            return Ok(None);
        }

        // Skip temporary files that might be in the process of being written
        if status_file.extension().is_some_and(|ext| ext == "tmp") {
            return Ok(None);
        }

        // Open file with shared lock for reading
        match OpenOptions::new().read(true).open(&status_file) {
            Ok(mut file) => {
                // Lock the file for shared reading
                FileExt::lock_shared(&file).map_err(|e| {
                    ParaError::fs_error(format!("Failed to lock status file for reading: {e}"))
                })?;

                // Read the content
                use std::io::Read;
                let mut json = String::new();
                match file.read_to_string(&mut json) {
                    Ok(_) => {
                        // Try to parse the content
                        match serde_json::from_str(&json) {
                            Ok(status) => Ok(Some(status)),
                            Err(e) => {
                                // If parsing fails and the file is empty or contains partial data,
                                // treat it as if the status doesn't exist yet
                                if json.trim().is_empty() || e.is_eof() {
                                    Ok(None)
                                } else {
                                    Err(ParaError::config_error(format!(
                                        "Failed to parse status file: {e}"
                                    ))
                                    .into())
                                }
                            }
                        }
                    }
                    Err(e) => {
                        Err(ParaError::fs_error(format!("Failed to read status file: {e}")).into())
                    }
                }
            }
            Err(e) => {
                // If the file was deleted between the exists check and open,
                // treat it as if it doesn't exist
                if e.kind() == std::io::ErrorKind::NotFound {
                    Ok(None)
                } else {
                    Err(ParaError::fs_error(format!("Failed to open status file: {e}")).into())
                }
            }
        }
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

    /// Calculate diff stats for a session
    pub fn calculate_diff_stats_for_session(
        session_state: &crate::core::session::SessionState,
    ) -> Result<DiffStats> {
        use crate::core::git::calculate_diff_stats;

        // Use the parent branch from session state, or default to "main"
        let parent_branch = session_state.parent_branch.as_deref().unwrap_or("main");

        // Calculate diff stats
        calculate_diff_stats(&session_state.worktree_path, parent_branch)
            .map_err(|e| anyhow::anyhow!("Failed to calculate diff stats: {}", e))
    }

    pub fn todo_percentage(&self) -> Option<u8> {
        match (self.todos_completed, self.todos_total) {
            (Some(completed), Some(total)) if total > 0 => {
                Some(((completed as f32 / total as f32) * 100.0).round() as u8)
            }
            _ => None,
        }
    }

    /// Calculate progress including the finish task
    /// Formula: todos_finished / (todos_total + 1)
    /// Returns 100% only if session is finished
    pub fn calculate_progress_with_finish(&self, is_finished: bool) -> Option<u8> {
        match (self.todos_completed, self.todos_total) {
            // Case: Has todos
            (Some(completed), Some(total)) => {
                // Defensive: cap completed to total before calculation
                let safe_completed = completed.min(total);
                let effective_total = total + 1; // +1 for the finish task
                let effective_completed = if is_finished {
                    safe_completed + 1
                } else {
                    safe_completed
                };

                Some(((effective_completed as f32 / effective_total as f32) * 100.0).round() as u8)
            }
            // Case: No todos but finished
            (None, None) if is_finished => Some(100),
            // Case: No todos and not finished
            _ => Some(0),
        }
    }

    pub fn format_todos(&self) -> Option<String> {
        match (self.todos_completed, self.todos_total) {
            (Some(completed), Some(total)) => {
                let percentage = self.todo_percentage().unwrap_or(0);
                Some(format!("{percentage}% ({completed}/{total})"))
            }
            _ => None,
        }
    }

    /// Check if the status is stale based on the last update time
    pub fn is_stale(&self, stale_threshold_hours: u32) -> bool {
        let now = Utc::now();
        let duration = now.signed_duration_since(self.last_update);
        duration.num_hours() >= stale_threshold_hours as i64
    }

    /// Load all status files from the state directory
    pub fn load_all(state_dir: &Path) -> Result<Vec<Status>> {
        let mut statuses = Vec::new();

        if !state_dir.exists() {
            return Ok(statuses);
        }

        for entry in fs::read_dir(state_dir)
            .map_err(|e| ParaError::fs_error(format!("Failed to read state directory: {e}")))?
        {
            let entry = entry
                .map_err(|e| ParaError::fs_error(format!("Failed to read directory entry: {e}")))?;

            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Some(filename) = path.file_stem().and_then(|s| s.to_str()) {
                    if filename.ends_with(".status") {
                        let session_name = filename.trim_end_matches(".status");
                        if let Some(status) = Self::load(state_dir, session_name)? {
                            statuses.push(status);
                        }
                    }
                }
            }
        }

        Ok(statuses)
    }

    /// Clean up stale status files
    pub fn cleanup_stale(state_dir: &Path, stale_threshold_hours: u32) -> Result<Vec<String>> {
        let mut cleaned = Vec::new();

        for status in Self::load_all(state_dir)? {
            if status.is_stale(stale_threshold_hours) {
                let status_file = Self::status_file_path(state_dir, &status.session_name);
                if status_file.exists() {
                    fs::remove_file(&status_file).map_err(|e| {
                        ParaError::fs_error(format!("Failed to remove stale status file: {e}"))
                    })?;
                    cleaned.push(status.session_name);
                }
            }
        }

        Ok(cleaned)
    }

    /// Generate a summary of all status files
    pub fn generate_summary(state_dir: &Path, stale_threshold_hours: u32) -> Result<StatusSummary> {
        let statuses = Self::load_all(state_dir)?;

        let mut test_summary = TestSummary {
            passed: 0,
            failed: 0,
            unknown: 0,
        };

        let mut blocked_sessions = 0;
        let mut stale_sessions = 0;
        let mut active_sessions = 0;
        let mut total_completed = 0;
        let mut total_todos = 0;

        for status in &statuses {
            // Count test statuses
            match status.test_status {
                TestStatus::Passed => test_summary.passed += 1,
                TestStatus::Failed => test_summary.failed += 1,
                TestStatus::Unknown => test_summary.unknown += 1,
            }

            // Count blocked and stale sessions
            if status.is_blocked {
                blocked_sessions += 1;
            }

            if status.is_stale(stale_threshold_hours) {
                stale_sessions += 1;
            } else {
                active_sessions += 1;
            }

            // Accumulate todos for overall progress
            if let (Some(completed), Some(total)) = (status.todos_completed, status.todos_total) {
                total_completed += completed;
                total_todos += total;
            }
        }

        let overall_progress = if total_todos > 0 {
            Some(((total_completed as f32 / total_todos as f32) * 100.0).round() as u8)
        } else {
            None
        };

        Ok(StatusSummary {
            total_sessions: statuses.len(),
            active_sessions,
            blocked_sessions,
            stale_sessions,
            test_summary,
            overall_progress,
        })
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;
    use tempfile::TempDir;

    #[test]
    fn test_status_new() {
        let status = Status::new(
            "test-session".to_string(),
            "Working on feature".to_string(),
            TestStatus::Passed,
        );

        assert_eq!(status.session_name, "test-session");
        assert_eq!(status.current_task, "Working on feature");
        assert_eq!(status.test_status, TestStatus::Passed);
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
        )
        .with_todos(5, 10);

        // Save status
        status.save(state_dir).unwrap();

        // Load status
        let loaded = Status::load(state_dir, "test-session").unwrap().unwrap();

        assert_eq!(loaded.session_name, status.session_name);
        assert_eq!(loaded.current_task, status.current_task);
        assert_eq!(loaded.test_status, status.test_status);
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
        let status = Status::new("test".to_string(), "task".to_string(), TestStatus::Unknown);

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
        let status = Status::new("test".to_string(), "task".to_string(), TestStatus::Unknown);

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
        )
        .with_blocked(Some("Need Redis help".to_string()))
        .with_todos(2, 5);

        let json = serde_json::to_string_pretty(&status).unwrap();
        let parsed: Status = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.session_name, status.session_name);
        assert_eq!(parsed.current_task, status.current_task);
        assert_eq!(parsed.test_status, status.test_status);
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
        );

        let json = serde_json::to_string(&status).unwrap();

        // Verify optional fields are not included
        assert!(!json.contains("todos_completed"));
        assert!(!json.contains("todos_total"));
        assert!(json.contains("\"blocked_reason\":null"));
    }

    #[test]
    fn test_file_locking() {
        let temp_dir = TempDir::new().unwrap();
        let state_dir = temp_dir.path();

        let status = Status::new(
            "lock-test".to_string(),
            "Testing file locks".to_string(),
            TestStatus::Passed,
        );

        // Test concurrent writes
        let state_dir_clone = state_dir.to_path_buf();
        let status_clone = status.clone();

        let handle = thread::spawn(move || {
            for i in 0..5 {
                let mut s = status_clone.clone();
                s.current_task = format!("Task from thread {i}");
                s.save(&state_dir_clone).unwrap();
                thread::sleep(Duration::from_millis(10));
            }
        });

        // Main thread writes
        for i in 0..5 {
            let mut s = status.clone();
            s.current_task = format!("Task from main {i}");
            s.save(state_dir).unwrap();
            thread::sleep(Duration::from_millis(10));
        }

        handle.join().unwrap();

        // Verify file can be read after concurrent writes
        let loaded = Status::load(state_dir, "lock-test").unwrap().unwrap();
        assert!(loaded.current_task.starts_with("Task from"));
    }

    #[test]
    fn test_is_stale() {
        let mut status = Status::new(
            "stale-test".to_string(),
            "Testing staleness".to_string(),
            TestStatus::Unknown,
        );

        // Fresh status should not be stale
        assert!(!status.is_stale(24));
        assert!(!status.is_stale(1));

        // Set last_update to 25 hours ago
        status.last_update = Utc::now() - chrono::Duration::hours(25);
        assert!(status.is_stale(24));
        assert!(!status.is_stale(48));
    }

    #[test]
    fn test_load_all() {
        let temp_dir = TempDir::new().unwrap();
        let state_dir = temp_dir.path();

        // Create multiple status files
        let statuses = vec![
            Status::new(
                "session1".to_string(),
                "Task 1".to_string(),
                TestStatus::Passed,
            ),
            Status::new(
                "session2".to_string(),
                "Task 2".to_string(),
                TestStatus::Failed,
            ),
            Status::new(
                "session3".to_string(),
                "Task 3".to_string(),
                TestStatus::Unknown,
            ),
        ];

        for status in &statuses {
            status.save(state_dir).unwrap();
        }

        // Also create a non-status JSON file that should be ignored
        std::fs::write(state_dir.join("other.json"), r#"{"not": "a status file"}"#).unwrap();

        // Load all status files
        let loaded = Status::load_all(state_dir).unwrap();
        assert_eq!(loaded.len(), 3);

        // Verify all sessions are loaded
        let session_names: Vec<String> = loaded.iter().map(|s| s.session_name.clone()).collect();
        assert!(session_names.contains(&"session1".to_string()));
        assert!(session_names.contains(&"session2".to_string()));
        assert!(session_names.contains(&"session3".to_string()));
    }

    #[test]
    fn test_cleanup_stale() {
        let temp_dir = TempDir::new().unwrap();
        let state_dir = temp_dir.path();

        // Create fresh status
        let fresh_status = Status::new(
            "fresh".to_string(),
            "Fresh task".to_string(),
            TestStatus::Passed,
        );
        fresh_status.save(state_dir).unwrap();

        // Create stale status
        let mut stale_status = Status::new(
            "stale".to_string(),
            "Stale task".to_string(),
            TestStatus::Failed,
        );
        stale_status.last_update = Utc::now() - chrono::Duration::hours(48);
        stale_status.save(state_dir).unwrap();

        // Verify both files exist
        assert!(Status::status_file_path(state_dir, "fresh").exists());
        assert!(Status::status_file_path(state_dir, "stale").exists());

        // Clean up stale files (threshold: 24 hours)
        let cleaned = Status::cleanup_stale(state_dir, 24).unwrap();
        assert_eq!(cleaned.len(), 1);
        assert_eq!(cleaned[0], "stale");

        // Verify only fresh file remains
        assert!(Status::status_file_path(state_dir, "fresh").exists());
        assert!(!Status::status_file_path(state_dir, "stale").exists());
    }

    #[test]
    fn test_generate_summary() {
        let temp_dir = TempDir::new().unwrap();
        let state_dir = temp_dir.path();

        // Create various status files
        let statuses = vec![
            Status::new("s1".to_string(), "Task 1".to_string(), TestStatus::Passed)
                .with_todos(5, 10),
            Status::new("s2".to_string(), "Task 2".to_string(), TestStatus::Failed)
                .with_blocked(Some("Need help".to_string()))
                .with_todos(2, 8),
            Status::new("s3".to_string(), "Task 3".to_string(), TestStatus::Unknown)
                .with_todos(3, 5),
            // Stale status
            {
                let mut s = Status::new(
                    "s4".to_string(),
                    "Stale task".to_string(),
                    TestStatus::Passed,
                );
                s.last_update = Utc::now() - chrono::Duration::hours(48);
                s
            },
        ];

        for status in &statuses {
            status.save(state_dir).unwrap();
        }

        // Generate summary
        let summary = Status::generate_summary(state_dir, 24).unwrap();

        assert_eq!(summary.total_sessions, 4);
        assert_eq!(summary.active_sessions, 3);
        assert_eq!(summary.stale_sessions, 1);
        assert_eq!(summary.blocked_sessions, 1);

        // Test summary
        assert_eq!(summary.test_summary.passed, 2);
        assert_eq!(summary.test_summary.failed, 1);
        assert_eq!(summary.test_summary.unknown, 1);

        // Overall progress: (5 + 2 + 3) / (10 + 8 + 5) = 10/23 ≈ 43%
        assert_eq!(summary.overall_progress, Some(43));
    }

    #[test]
    fn test_session_manager_cleanup() {
        use crate::core::session::manager::SessionManager;
        use crate::test_utils::test_helpers::create_test_config;

        let temp_dir = TempDir::new().unwrap();
        let mut config = create_test_config();
        config.directories.state_dir = temp_dir.path().to_string_lossy().to_string();

        let manager = SessionManager::new(&config);
        let state_dir = Path::new(&config.directories.state_dir);

        // Create a status file
        let status = Status::new(
            "test-session".to_string(),
            "Test task".to_string(),
            TestStatus::Passed,
        );
        status.save(state_dir).unwrap();

        // Verify status file exists
        assert!(Status::status_file_path(state_dir, "test-session").exists());

        // Delete state (should also delete status file)
        manager.delete_state("test-session").unwrap();

        // Verify status file is deleted
        assert!(!Status::status_file_path(state_dir, "test-session").exists());
    }

    #[test]
    fn test_calculate_progress_with_finish() {
        let status = Status::new("test".to_string(), "task".to_string(), TestStatus::Unknown);

        // Test 1: No todos and not finished = 0%
        assert_eq!(status.calculate_progress_with_finish(false), Some(0));

        // Test 2: No todos but finished = 100%
        assert_eq!(status.calculate_progress_with_finish(true), Some(100));

        // Test 3: With todos, not finished
        // 3/10 todos done, not finished = 3/11 = 27%
        let status_with_todos = status.clone().with_todos(3, 10);
        assert_eq!(
            status_with_todos.calculate_progress_with_finish(false),
            Some(27)
        );

        // Test 4: With todos, finished
        // 3/10 todos done, finished = 4/11 = 36%
        assert_eq!(
            status_with_todos.calculate_progress_with_finish(true),
            Some(36)
        );

        // Test 5: All todos done, not finished
        // 10/10 todos done, not finished = 10/11 = 91%
        let all_done = status.clone().with_todos(10, 10);
        assert_eq!(all_done.calculate_progress_with_finish(false), Some(91));

        // Test 6: All todos done and finished = 100%
        assert_eq!(all_done.calculate_progress_with_finish(true), Some(100));

        // Test 7: Edge case - 0 todos total but finished
        let zero_todos = status.clone().with_todos(0, 0);
        assert_eq!(zero_todos.calculate_progress_with_finish(false), Some(0));
        assert_eq!(zero_todos.calculate_progress_with_finish(true), Some(100));
    }

    #[test]
    fn test_progress_edge_cases() {
        let status = Status::new(
            "edge-test".to_string(),
            "edge task".to_string(),
            TestStatus::Unknown,
        );

        // Edge case 1: Completed > Total (defensive check)
        // Should cap at effective total
        let invalid_status = Status {
            session_name: "invalid".to_string(),
            current_task: "task".to_string(),
            test_status: TestStatus::Unknown,
            is_blocked: false,
            blocked_reason: None,
            todos_completed: Some(15), // More than total!
            todos_total: Some(10),
            diff_stats: None,
            last_update: Utc::now(),
        };

        // 15 is capped to 10, so 10/11 = 91%
        assert_eq!(
            invalid_status.calculate_progress_with_finish(false),
            Some(91)
        );
        // With finish: 11 is capped to 11, so 11/11 = 100%
        assert_eq!(
            invalid_status.calculate_progress_with_finish(true),
            Some(100)
        );

        // Edge case 2: Very large numbers
        let large_status = status.clone().with_todos(999, 1000);
        // 999/1001 ≈ 99.8% rounds to 100%
        assert_eq!(
            large_status.calculate_progress_with_finish(false),
            Some(100)
        );
        assert_eq!(large_status.calculate_progress_with_finish(true), Some(100));

        // Edge case 3: Single todo
        let single_todo = status.clone().with_todos(0, 1);
        assert_eq!(single_todo.calculate_progress_with_finish(false), Some(0)); // 0/2 = 0%
        assert_eq!(single_todo.calculate_progress_with_finish(true), Some(50)); // 1/2 = 50%

        let single_todo_done = status.clone().with_todos(1, 1);
        assert_eq!(
            single_todo_done.calculate_progress_with_finish(false),
            Some(50)
        ); // 1/2 = 50%
        assert_eq!(
            single_todo_done.calculate_progress_with_finish(true),
            Some(100)
        ); // 2/2 = 100%
    }

    #[test]
    fn test_progress_calculation_consistency() {
        // Ensure old todo_percentage still works for backward compatibility
        let status = Status::new(
            "compat".to_string(),
            "task".to_string(),
            TestStatus::Unknown,
        )
        .with_todos(7, 10);

        // Old method should still return 70%
        assert_eq!(status.todo_percentage(), Some(70));

        // New method should return different value due to +1
        // 7/11 ≈ 63.6% rounds to 64%
        assert_eq!(status.calculate_progress_with_finish(false), Some(64));
    }
}
