use super::state::{SessionState, SessionStatus};
use crate::config::Config;
use crate::core::git::{GitOperations, GitService};
use crate::utils::{ParaError, Result};
use std::fs;
use std::path::{Path, PathBuf};

pub struct SessionManager {
    state_dir: PathBuf,
    config: Config,
}

impl SessionManager {
    pub fn new(config: &Config) -> Self {
        let state_dir = PathBuf::from(config.get_state_dir());
        Self { 
            state_dir, 
            config: config.clone() 
        }
    }

    pub fn create_session(&mut self, name: String, base_branch: Option<String>) -> Result<SessionState> {
        let git_service = GitService::discover()
            .map_err(|e| ParaError::git_error(format!("Failed to discover git repository: {}", e)))?;

        let repository_root = git_service.repository().root.clone();
        
        let _base_branch = base_branch.unwrap_or_else(|| {
            git_service
                .repository()
                .get_main_branch()
                .unwrap_or_else(|_| "main".to_string())
        });

        let branch_name = crate::utils::generate_branch_name(self.config.get_branch_prefix());
        
        let subtrees_path = repository_root.join(&self.config.directories.subtrees_dir);
        let worktree_path = subtrees_path
            .join(self.config.get_branch_prefix())
            .join(&name);

        if !subtrees_path.exists() {
            fs::create_dir_all(&subtrees_path).map_err(|e| {
                ParaError::fs_error(format!("Failed to create subtrees directory: {}", e))
            })?;
        }

        if self.session_exists(&name) {
            return Err(ParaError::session_exists(&name));
        }

        if worktree_path.exists() {
            return Err(ParaError::file_operation(format!(
                "Worktree path already exists: {}",
                worktree_path.display()
            )));
        }

        git_service.create_worktree(&branch_name, &worktree_path)?;

        let session_state = SessionState::new(name, branch_name, worktree_path);

        self.save_state(&session_state)?;

        Ok(session_state)
    }

    pub fn load_state(&self, session_name: &str) -> Result<SessionState> {
        self.ensure_state_dir_exists()?;
        
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

        let session: SessionState = serde_json::from_str(&content).map_err(|e| {
            ParaError::state_corruption(format!(
                "Failed to parse session state from {}: {}",
                state_file.display(),
                e
            ))
        })?;

        Ok(session)
    }

    pub fn save_state(&self, session: &SessionState) -> Result<()> {
        self.ensure_state_dir_exists()?;

        let state_file = self.state_dir.join(format!("{}.state", session.name));
        let json = serde_json::to_string_pretty(session)?;
        fs::write(&state_file, json).map_err(|e| {
            ParaError::file_operation(format!(
                "Failed to save session state to {}: {}",
                state_file.display(),
                e
            ))
        })?;

        Ok(())
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

    pub fn find_session_by_path(&self, path: &Path) -> Result<Option<SessionState>> {
        let sessions = self.list_sessions()?;
        
        for session in sessions {
            if session.worktree_path == path || path.starts_with(&session.worktree_path) {
                return Ok(Some(session));
            }
        }
        
        Ok(None)
    }

    pub fn find_session_by_branch(&self, branch: &str) -> Result<Option<SessionState>> {
        let sessions = self.list_sessions()?;
        
        for session in sessions {
            if session.branch == branch && matches!(session.status, SessionStatus::Active) {
                return Ok(Some(session));
            }
        }
        
        Ok(None)
    }

    pub fn update_session_status(&mut self, session_name: &str, status: SessionStatus) -> Result<()> {
        let mut session = self.load_state(session_name)?;
        session.update_status(status);
        self.save_state(&session)
    }

    pub fn session_exists(&self, session_name: &str) -> bool {
        let state_file = self.state_dir.join(format!("{}.state", session_name));
        state_file.exists()
    }

    fn ensure_state_dir_exists(&self) -> Result<()> {
        if !self.state_dir.exists() {
            fs::create_dir_all(&self.state_dir).map_err(|e| {
                ParaError::fs_error(format!(
                    "Failed to create state directory {}: {}",
                    self.state_dir.display(),
                    e
                ))
            })?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::defaults::default_config;
    use tempfile::TempDir;

    #[test]
    fn test_session_manager_creation() {
        let temp_dir = TempDir::new().unwrap();
        
        let mut config = default_config();
        config.directories.state_dir = temp_dir.path().join(".para_state").to_string_lossy().to_string();
        config.directories.subtrees_dir = "subtrees".to_string();

        let manager = SessionManager::new(&config);
        // State directory is created on demand
        assert!(!manager.state_dir.exists());
    }

    #[test]
    fn test_session_manager_save_and_load() {
        let temp_dir = TempDir::new().unwrap();
        
        let mut config = default_config();
        config.directories.state_dir = temp_dir.path().join(".para_state").to_string_lossy().to_string();
        config.directories.subtrees_dir = "subtrees".to_string();
        let manager = SessionManager::new(&config);

        // Create session manually without git operations
        let session = SessionState::new(
            "test-session-save".to_string(),
            "pc/test-branch".to_string(),
            temp_dir.path().join("test-worktree"),
        );
        
        // Test save and load functionality
        manager.save_state(&session).unwrap();
        assert!(manager.session_exists(&session.name));

        let loaded_session = manager.load_state(&session.name).unwrap();
        assert_eq!(loaded_session.name, session.name);
        assert_eq!(loaded_session.branch, session.branch);
        assert_eq!(loaded_session.worktree_path, session.worktree_path);
    }

    #[test]
    fn test_session_manager_list_sessions() {
        let temp_dir = TempDir::new().unwrap();
        
        let mut config = default_config();
        config.directories.state_dir = temp_dir.path().join(".para_state").to_string_lossy().to_string();
        config.directories.subtrees_dir = "subtrees".to_string();
        let manager = SessionManager::new(&config);

        // Create sessions without using git operations to avoid directory issues
        let session1 = SessionState::new(
            "session1".to_string(),
            "test/branch1".to_string(),
            temp_dir.path().join("session1"),
        );
        let session2 = SessionState::new(
            "session2".to_string(),
            "test/branch2".to_string(),
            temp_dir.path().join("session2"),
        );

        manager.save_state(&session1).unwrap();
        manager.save_state(&session2).unwrap();

        let sessions = manager.list_sessions().unwrap();
        assert_eq!(sessions.len(), 2);
    }

    #[test]
    fn test_session_manager_delete() {
        let temp_dir = TempDir::new().unwrap();
        
        let mut config = default_config();
        config.directories.state_dir = temp_dir.path().join(".para_state").to_string_lossy().to_string();
        config.directories.subtrees_dir = "subtrees".to_string();
        let manager = SessionManager::new(&config);

        // Create session manually without git operations
        let session = SessionState::new(
            "test-session-delete".to_string(),
            "pc/test-branch".to_string(),
            temp_dir.path().join("test-worktree"),
        );
        
        manager.save_state(&session).unwrap();
        assert!(manager.session_exists(&session.name));

        manager.delete_state(&session.name).unwrap();
        assert!(!manager.session_exists(&session.name));
    }

    #[test]
    fn test_session_manager_update_status() {
        let temp_dir = TempDir::new().unwrap();
        
        let mut config = default_config();
        config.directories.state_dir = temp_dir.path().join(".para_state").to_string_lossy().to_string();
        config.directories.subtrees_dir = "subtrees".to_string();
        let mut manager = SessionManager::new(&config);

        // Create session manually without git operations
        let session = SessionState::new(
            "test-session-update".to_string(),
            "pc/test-branch".to_string(),
            temp_dir.path().join("test-worktree"),
        );
        
        manager.save_state(&session).unwrap();
        
        manager.update_session_status(&session.name, SessionStatus::Finished).unwrap();
        
        let updated_session = manager.load_state(&session.name).unwrap();
        assert!(matches!(updated_session.status, SessionStatus::Finished));
    }
}