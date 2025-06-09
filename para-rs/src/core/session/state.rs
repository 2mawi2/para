use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionState {
    pub id: String,
    pub name: String,
    pub session_type: SessionType,
    
    pub branch: String,
    pub base_branch: String,
    pub worktree_path: PathBuf,
    pub repository_root: PathBuf,
    
    pub created_at: DateTime<Utc>,
    pub last_modified: DateTime<Utc>,
    pub status: SessionStatus,
    
    pub initial_prompt: Option<String>,
    pub commit_count: u32,
    pub last_commit_hash: Option<String>,
    
    pub config_snapshot: SessionConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SessionType {
    Manual,
    Dispatched,
    Recovered,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SessionStatus {
    Active,
    Finishing,
    Integrating,
    Cancelled,
    Completed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfig {
    pub branch_prefix: String,
    pub subtrees_dir: String,
    pub ide_name: String,
    pub auto_stage: bool,
    pub auto_commit: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
    pub id: String,
    pub name: String,
    pub session_type: SessionType,
    pub status: SessionStatus,
    pub created_at: DateTime<Utc>,
    pub last_modified: DateTime<Utc>,
    pub branch: String,
    pub commit_count: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StateFileFormat {
    pub version: String,
    pub session: SessionState,
}

impl SessionState {
    pub fn new_manual(
        name: String,
        branch: String,
        base_branch: String,
        worktree_path: PathBuf,
        repository_root: PathBuf,
        config_snapshot: SessionConfig,
    ) -> Self {
        let now = Utc::now();
        let id = generate_session_id(&name);
        
        Self {
            id,
            name,
            session_type: SessionType::Manual,
            branch,
            base_branch,
            worktree_path,
            repository_root,
            created_at: now,
            last_modified: now,
            status: SessionStatus::Active,
            initial_prompt: None,
            commit_count: 0,
            last_commit_hash: None,
            config_snapshot,
        }
    }

    pub fn new_dispatched(
        name: String,
        branch: String,
        base_branch: String,
        worktree_path: PathBuf,
        repository_root: PathBuf,
        initial_prompt: String,
        config_snapshot: SessionConfig,
    ) -> Self {
        let now = Utc::now();
        let id = generate_session_id(&name);
        
        Self {
            id,
            name,
            session_type: SessionType::Dispatched,
            branch,
            base_branch,
            worktree_path,
            repository_root,
            created_at: now,
            last_modified: now,
            status: SessionStatus::Active,
            initial_prompt: Some(initial_prompt),
            commit_count: 0,
            last_commit_hash: None,
            config_snapshot,
        }
    }

    pub fn new_recovered(
        name: String,
        branch: String,
        base_branch: String,
        worktree_path: PathBuf,
        repository_root: PathBuf,
        config_snapshot: SessionConfig,
    ) -> Self {
        let now = Utc::now();
        let id = generate_session_id(&name);
        
        Self {
            id,
            name,
            session_type: SessionType::Recovered,
            branch,
            base_branch,
            worktree_path,
            repository_root,
            created_at: now,
            last_modified: now,
            status: SessionStatus::Active,
            initial_prompt: None,
            commit_count: 0,
            last_commit_hash: None,
            config_snapshot,
        }
    }

    pub fn update_status(&mut self, status: SessionStatus) {
        self.status = status;
        self.last_modified = Utc::now();
    }

    pub fn update_commit_info(&mut self, commit_hash: String) {
        self.commit_count += 1;
        self.last_commit_hash = Some(commit_hash);
        self.last_modified = Utc::now();
    }

    pub fn to_summary(&self) -> SessionSummary {
        SessionSummary {
            id: self.id.clone(),
            name: self.name.clone(),
            session_type: self.session_type.clone(),
            status: self.status.clone(),
            created_at: self.created_at,
            last_modified: self.last_modified,
            branch: self.branch.clone(),
            commit_count: self.commit_count,
        }
    }

    pub fn to_file_format(&self) -> StateFileFormat {
        StateFileFormat {
            version: "1.0".to_string(),
            session: self.clone(),
        }
    }
}

impl StateFileFormat {
    pub fn from_session(session: SessionState) -> Self {
        Self {
            version: "1.0".to_string(),
            session,
        }
    }
}

impl SessionConfig {
    pub fn from_config(config: &crate::config::Config) -> Self {
        Self {
            branch_prefix: config.git.branch_prefix.clone(),
            subtrees_dir: config.directories.subtrees_dir.clone(),
            ide_name: config.ide.name.clone(),
            auto_stage: config.git.auto_stage,
            auto_commit: config.git.auto_commit,
        }
    }
}

fn generate_session_id(name: &str) -> String {
    let timestamp = Utc::now().format("%Y%m%d-%H%M%S").to_string();
    format!("{}_{}", name, timestamp)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn create_test_config_snapshot() -> SessionConfig {
        SessionConfig {
            branch_prefix: "test".to_string(),
            subtrees_dir: "subtrees/test".to_string(),
            ide_name: "claude".to_string(),
            auto_stage: true,
            auto_commit: false,
        }
    }

    #[test]
    fn test_session_state_new_manual() {
        let config = create_test_config_snapshot();
        let state = SessionState::new_manual(
            "test-session".to_string(),
            "test/branch".to_string(),
            "main".to_string(),
            PathBuf::from("/path/to/worktree"),
            PathBuf::from("/path/to/repo"),
            config.clone(),
        );

        assert_eq!(state.name, "test-session");
        assert!(state.id.starts_with("test-session_"));
        assert!(matches!(state.session_type, SessionType::Manual));
        assert!(matches!(state.status, SessionStatus::Active));
        assert_eq!(state.base_branch, "main");
        assert_eq!(state.commit_count, 0);
        assert!(state.initial_prompt.is_none());
        assert_eq!(state.config_snapshot.ide_name, config.ide_name);
    }

    #[test]
    fn test_session_state_new_dispatched() {
        let config = create_test_config_snapshot();
        let state = SessionState::new_dispatched(
            "dispatch-session".to_string(),
            "test/dispatch-branch".to_string(),
            "main".to_string(),
            PathBuf::from("/path/to/worktree"),
            PathBuf::from("/path/to/repo"),
            "Implement authentication".to_string(),
            config,
        );

        assert_eq!(state.name, "dispatch-session");
        assert!(matches!(state.session_type, SessionType::Dispatched));
        assert_eq!(state.initial_prompt, Some("Implement authentication".to_string()));
    }

    #[test]
    fn test_session_state_update_status() {
        let config = create_test_config_snapshot();
        let mut state = SessionState::new_manual(
            "test".to_string(),
            "test/branch".to_string(),
            "main".to_string(),
            PathBuf::from("/test"),
            PathBuf::from("/repo"),
            config,
        );

        let initial_modified = state.last_modified;
        
        std::thread::sleep(std::time::Duration::from_millis(1));
        state.update_status(SessionStatus::Finishing);

        assert!(matches!(state.status, SessionStatus::Finishing));
        assert!(state.last_modified > initial_modified);
    }

    #[test]
    fn test_session_state_update_commit_info() {
        let config = create_test_config_snapshot();
        let mut state = SessionState::new_manual(
            "test".to_string(),
            "test/branch".to_string(),
            "main".to_string(),
            PathBuf::from("/test"),
            PathBuf::from("/repo"),
            config,
        );

        state.update_commit_info("abc123def456".to_string());

        assert_eq!(state.commit_count, 1);
        assert_eq!(state.last_commit_hash, Some("abc123def456".to_string()));
    }

    #[test]
    fn test_session_state_to_summary() {
        let config = create_test_config_snapshot();
        let state = SessionState::new_manual(
            "test".to_string(),
            "test/branch".to_string(),
            "main".to_string(),
            PathBuf::from("/test"),
            PathBuf::from("/repo"),
            config,
        );

        let summary = state.to_summary();
        
        assert_eq!(summary.id, state.id);
        assert_eq!(summary.name, state.name);
        assert!(matches!(summary.session_type, SessionType::Manual));
        assert!(matches!(summary.status, SessionStatus::Active));
        assert_eq!(summary.branch, state.branch);
        assert_eq!(summary.commit_count, 0);
    }

    #[test]
    fn test_state_file_format() {
        let config = create_test_config_snapshot();
        let state = SessionState::new_manual(
            "test".to_string(),
            "test/branch".to_string(),
            "main".to_string(),
            PathBuf::from("/test"),
            PathBuf::from("/repo"),
            config,
        );

        let file_format = state.to_file_format();
        
        assert_eq!(file_format.version, "1.0");
        assert_eq!(file_format.session.id, state.id);
    }

    #[test]
    fn test_generate_session_id() {
        let id1 = generate_session_id("test");
        let id2 = generate_session_id("test");
        
        assert!(id1.starts_with("test_"));
        assert!(id2.starts_with("test_"));
        assert_ne!(id1, id2);
        
        let timestamp_part = &id1[5..];
        assert!(timestamp_part.len() >= 15);
        assert!(timestamp_part.contains('-'));
    }
}