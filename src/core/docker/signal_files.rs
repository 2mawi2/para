//! Signal file protocol for container-host communication
//!
//! This module defines the structures and logic for the Signal File Protocol,
//! which enables containers to communicate with the host system for operations
//! like finish, cancel, and status updates.

use crate::utils::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Signal file for finish operation
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FinishSignal {
    pub commit_message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
}

/// Signal file for cancel operation
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CancelSignal {
    #[serde(default)]
    pub force: bool,
}

/// Status information from container
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ContainerStatus {
    pub task: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tests: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub todos: Option<String>,
    #[serde(default)]
    pub blocked: bool,
    pub timestamp: String,
}

/// Signal file paths within a worktree
pub struct SignalFilePaths {
    pub finish: PathBuf,
    pub cancel: PathBuf,
    pub status: PathBuf,
}

impl SignalFilePaths {
    /// Create signal file paths for a given worktree
    pub fn new(worktree_path: &Path) -> Self {
        let para_dir = worktree_path.join(".para");
        Self {
            finish: para_dir.join("finish_signal.json"),
            cancel: para_dir.join("cancel_signal.json"),
            status: para_dir.join("status.json"),
        }
    }
}

/// Read and parse a signal file
pub fn read_signal_file<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<Option<T>> {
    if !path.exists() {
        return Ok(None);
    }

    let content = std::fs::read_to_string(path)?;
    let signal = serde_json::from_str(&content)?;
    Ok(Some(signal))
}

/// Write a signal file
#[allow(dead_code)]
pub fn write_signal_file<T: Serialize>(path: &Path, signal: &T) -> Result<()> {
    // Ensure the parent directory exists
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let content = serde_json::to_string_pretty(signal)?;
    std::fs::write(path, content)?;
    Ok(())
}

/// Delete a signal file if it exists
pub fn delete_signal_file(path: &Path) -> Result<()> {
    if path.exists() {
        std::fs::remove_file(path)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_signal_file_paths() {
        let temp_dir = TempDir::new().unwrap();
        let worktree_path = temp_dir.path();
        let paths = SignalFilePaths::new(worktree_path);

        assert_eq!(
            paths.finish,
            worktree_path.join(".para").join("finish_signal.json")
        );
        assert_eq!(
            paths.cancel,
            worktree_path.join(".para").join("cancel_signal.json")
        );
        assert_eq!(
            paths.status,
            worktree_path.join(".para").join("status.json")
        );
    }

    #[test]
    fn test_finish_signal_serialization() {
        let signal = FinishSignal {
            commit_message: "Implement feature X".to_string(),
            branch: Some("custom-branch".to_string()),
        };

        let json = serde_json::to_string(&signal).unwrap();
        let deserialized: FinishSignal = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.commit_message, signal.commit_message);
        assert_eq!(deserialized.branch, signal.branch);
    }

    #[test]
    fn test_cancel_signal_serialization() {
        let signal = CancelSignal { force: true };

        let json = serde_json::to_string(&signal).unwrap();
        let deserialized: CancelSignal = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.force, signal.force);
    }

    #[test]
    fn test_container_status_serialization() {
        let status = ContainerStatus {
            task: "Implementing authentication".to_string(),
            tests: Some("failed".to_string()),
            todos: Some("3/5".to_string()),
            blocked: false,
            timestamp: "2024-01-20T10:30:00Z".to_string(),
        };

        let json = serde_json::to_string(&status).unwrap();
        let deserialized: ContainerStatus = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.task, status.task);
        assert_eq!(deserialized.tests, status.tests);
        assert_eq!(deserialized.todos, status.todos);
        assert_eq!(deserialized.blocked, status.blocked);
    }

    #[test]
    fn test_read_write_signal_file() {
        let temp_dir = TempDir::new().unwrap();
        let signal_path = temp_dir.path().join("test_signal.json");

        let signal = FinishSignal {
            commit_message: "Test commit".to_string(),
            branch: None,
        };

        // Write signal
        write_signal_file(&signal_path, &signal).unwrap();
        assert!(signal_path.exists());

        // Read signal
        let read_signal: Option<FinishSignal> = read_signal_file(&signal_path).unwrap();
        assert!(read_signal.is_some());
        let read_signal = read_signal.unwrap();
        assert_eq!(read_signal.commit_message, signal.commit_message);

        // Delete signal
        delete_signal_file(&signal_path).unwrap();
        assert!(!signal_path.exists());

        // Read non-existent signal
        let read_signal: Option<FinishSignal> = read_signal_file(&signal_path).unwrap();
        assert!(read_signal.is_none());
    }
}
