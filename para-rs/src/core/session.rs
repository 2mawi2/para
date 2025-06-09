use crate::config::Config;
use crate::utils::{ParaError, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

pub mod archive;
pub mod recovery;

pub use archive::{ArchiveEntry, ArchiveManager, ArchiveStats};
pub use recovery::{RecoveryInfo, RecoveryOptions, RecoveryResult, SessionRecovery};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionState {
    pub name: String,
    pub branch: String,
    pub worktree_path: PathBuf,
    pub created_at: String,
    pub updated_at: String,
    pub status: SessionStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SessionStatus {
    Active,
    Finished,
    Cancelled,
}

pub struct SessionManager {
    state_dir: PathBuf,
}

impl SessionManager {
    pub fn new(config: &Config) -> Self {
        let state_dir = PathBuf::from(config.get_state_dir());
        Self { state_dir }
    }

    pub fn save_state(&self, state: &SessionState) -> Result<()> {
        if let Some(parent) = self.state_dir.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::create_dir_all(&self.state_dir)?;

        let state_file = self.state_dir.join(format!("{}.state", state.name));
        let json = serde_json::to_string_pretty(state)?;
        fs::write(&state_file, json).map_err(|e| {
            ParaError::file_operation(format!(
                "Failed to save session state to {}: {}",
                state_file.display(),
                e
            ))
        })?;

        Ok(())
    }

    pub fn load_state(&self, session_name: &str) -> Result<SessionState> {
        let state_file = self.state_dir.join(format!("{}.state", session_name));
        if !state_file.exists() {
            return Err(ParaError::session_not_found(session_name));
        }

        let content = fs::read_to_string(&state_file).map_err(|e| {
            ParaError::file_operation(format!(
                "Failed to read session state from {}: {}",
                state_file.display(),
                e
            ))
        })?;

        let state: SessionState = serde_json::from_str(&content).map_err(|e| {
            ParaError::state_corruption(format!(
                "Failed to parse session state from {}: {}",
                state_file.display(),
                e
            ))
        })?;

        Ok(state)
    }

    pub fn list_sessions(&self) -> Result<Vec<SessionState>> {
        if !self.state_dir.exists() {
            return Ok(Vec::new());
        }

        let entries = fs::read_dir(&self.state_dir)?;
        let mut sessions = Vec::new();

        for entry in entries {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() && path.extension().is_some_and(|ext| ext == "state") {
                if let Some(stem) = path.file_stem() {
                    if let Some(session_name) = stem.to_str() {
                        match self.load_state(session_name) {
                            Ok(state) => sessions.push(state),
                            Err(_) => continue, // Skip corrupted state files
                        }
                    }
                }
            }
        }

        sessions.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(sessions)
    }

    pub fn delete_state(&self, session_name: &str) -> Result<()> {
        let state_file = self.state_dir.join(format!("{}.state", session_name));
        if state_file.exists() {
            fs::remove_file(&state_file).map_err(|e| {
                ParaError::file_operation(format!(
                    "Failed to delete session state {}: {}",
                    state_file.display(),
                    e
                ))
            })?;
        }
        Ok(())
    }

    pub fn session_exists(&self, session_name: &str) -> bool {
        let state_file = self.state_dir.join(format!("{}.state", session_name));
        state_file.exists()
    }
}

impl SessionState {
    pub fn new(name: String, branch: String, worktree_path: PathBuf) -> Self {
        let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
        Self {
            name,
            branch,
            worktree_path,
            created_at: now.clone(),
            updated_at: now,
            status: SessionStatus::Active,
        }
    }

    pub fn update_status(&mut self, status: SessionStatus) {
        self.status = status;
        self.updated_at = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use tempfile::TempDir;

    fn create_test_config(temp_dir: &Path) -> Config {
        Config {
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
                subtrees_dir: "subtrees".to_string(),
                state_dir: temp_dir.join(".para_state").to_string_lossy().to_string(),
            },
            git: crate::config::GitConfig {
                branch_prefix: "test".to_string(),
                auto_stage: true,
                auto_commit: false,
            },
            session: crate::config::SessionConfig {
                default_name_format: "%Y%m%d-%H%M%S".to_string(),
                preserve_on_finish: false,
                auto_cleanup_days: Some(7),
            },
        }
    }

    #[test]
    fn test_session_state_creation() {
        let state = SessionState::new(
            "test-session".to_string(),
            "test-branch".to_string(),
            PathBuf::from("/test/path"),
        );

        assert_eq!(state.name, "test-session");
        assert_eq!(state.branch, "test-branch");
        assert_eq!(state.worktree_path, PathBuf::from("/test/path"));
        assert!(matches!(state.status, SessionStatus::Active));
        assert!(!state.created_at.is_empty());
        assert!(!state.updated_at.is_empty());
    }

    #[test]
    fn test_session_manager_save_and_load() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config(temp_dir.path());
        let manager = SessionManager::new(&config);

        let state = SessionState::new(
            "test-session".to_string(),
            "test-branch".to_string(),
            PathBuf::from("/test/path"),
        );

        manager.save_state(&state).unwrap();
        assert!(manager.session_exists("test-session"));

        let loaded_state = manager.load_state("test-session").unwrap();
        assert_eq!(loaded_state.name, state.name);
        assert_eq!(loaded_state.branch, state.branch);
        assert_eq!(loaded_state.worktree_path, state.worktree_path);
    }

    #[test]
    fn test_session_manager_list() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config(temp_dir.path());
        let manager = SessionManager::new(&config);

        let state1 = SessionState::new(
            "session1".to_string(),
            "branch1".to_string(),
            PathBuf::from("/path1"),
        );

        let state2 = SessionState::new(
            "session2".to_string(),
            "branch2".to_string(),
            PathBuf::from("/path2"),
        );

        manager.save_state(&state1).unwrap();
        manager.save_state(&state2).unwrap();

        let sessions = manager.list_sessions().unwrap();
        assert_eq!(sessions.len(), 2);

        let names: Vec<&String> = sessions.iter().map(|s| &s.name).collect();
        assert!(names.contains(&&"session1".to_string()));
        assert!(names.contains(&&"session2".to_string()));
    }

    #[test]
    fn test_session_manager_delete() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config(temp_dir.path());
        let manager = SessionManager::new(&config);

        let state = SessionState::new(
            "test-session".to_string(),
            "test-branch".to_string(),
            PathBuf::from("/test/path"),
        );

        manager.save_state(&state).unwrap();
        assert!(manager.session_exists("test-session"));

        manager.delete_state("test-session").unwrap();
        assert!(!manager.session_exists("test-session"));
    }
}
