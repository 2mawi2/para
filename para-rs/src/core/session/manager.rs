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
        
        let base_branch = base_branch.unwrap_or_else(|| {
            git_service
                .repository()
                .get_main_branch()
                .unwrap_or_else(|_| "main".to_string())
        });

        let branch_name = crate::utils::generate_branch_name(self.config.get_branch_prefix());
        
        let subtrees_path = repository_root.join(&self.config.directories.subtrees_dir);
        let session_id = crate::utils::generate_session_id(&name);
        let worktree_path = subtrees_path
            .join(self.config.get_branch_prefix())
            .join(&session_id);

        if !subtrees_path.exists() {
            fs::create_dir_all(&subtrees_path).map_err(|e| {
                ParaError::fs_error(format!("Failed to create subtrees directory: {}", e))
            })?;
        }

        if self.session_exists(&session_id) {
            return Err(ParaError::session_exists(&session_id));
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
        if let Some(parent) = self.state_dir.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::create_dir_all(&self.state_dir)?;

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

    pub fn auto_detect_session(&self) -> Result<SessionState> {
        let current_dir = std::env::current_dir()
            .map_err(|e| ParaError::fs_error(format!("Failed to get current directory: {}", e)))?;

        if let Some(session) = self.find_session_by_path(&current_dir)? {
            return Ok(session);
        }

        let git_service = GitService::discover()
            .map_err(|e| ParaError::git_error(format!("Failed to discover git repository: {}", e)))?;

        let current_branch = git_service.repository().get_current_branch()
            .map_err(|e| ParaError::git_error(format!("Failed to get current branch: {}", e)))?;

        let sessions = self.list_sessions()?;
        for session in sessions {
            if session.branch == current_branch && matches!(session.status, SessionStatus::Active) {
                return Ok(session);
            }
        }

        Err(ParaError::session_not_found("auto-detected"))
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

    pub fn generate_session_id(&self, name: &str) -> String {
        let timestamp = chrono::Utc::now().format("%Y%m%d-%H%M%S").to_string();
        format!("{}_{}", name, timestamp)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;
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
                default_integration_strategy: crate::cli::parser::IntegrationStrategy::Squash,
            },
            session: crate::config::SessionConfig {
                default_name_format: "%Y%m%d-%H%M%S".to_string(),
                preserve_on_finish: false,
                auto_cleanup_days: Some(7),
            },
        }
    }

    fn setup_test_repo() -> (TempDir, Config) {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let repo_path = temp_dir.path();

        Command::new("git")
            .current_dir(repo_path)
            .args(&["init"])
            .status()
            .expect("Failed to init git repo");

        Command::new("git")
            .current_dir(repo_path)
            .args(&["config", "user.name", "Test User"])
            .status()
            .expect("Failed to set git user name");

        Command::new("git")
            .current_dir(repo_path)
            .args(&["config", "user.email", "test@example.com"])
            .status()
            .expect("Failed to set git user email");

        std::fs::write(repo_path.join("README.md"), "# Test Repository")
            .expect("Failed to write README");

        Command::new("git")
            .current_dir(repo_path)
            .args(&["add", "README.md"])
            .status()
            .expect("Failed to add README");

        Command::new("git")
            .current_dir(repo_path)
            .args(&["commit", "-m", "Initial commit"])
            .status()
            .expect("Failed to commit README");

        let config = create_test_config(temp_dir.path());

        std::env::set_current_dir(repo_path).expect("Failed to change directory");

        (temp_dir, config)
    }

    #[test]
    fn test_session_manager_creation() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config(temp_dir.path());

        let manager = SessionManager::new(&config);
        assert!(manager.state_dir.exists());
    }

    #[test]
    fn test_session_manager_save_and_load() {
        let (_temp_dir, config) = setup_test_repo();
        let mut manager = SessionManager::new(&config);

        let session = manager.create_session("test-session".to_string(), Some("main".to_string())).unwrap();
        assert!(manager.session_exists(&session.name));

        let loaded_session = manager.load_state(&session.name).unwrap();
        assert_eq!(loaded_session.name, session.name);
        assert_eq!(loaded_session.branch, session.branch);
        assert_eq!(loaded_session.worktree_path, session.worktree_path);
    }

    #[test]
    fn test_session_manager_list_sessions() {
        let (_temp_dir, config) = setup_test_repo();
        let mut manager = SessionManager::new(&config);

        let _session1 = manager.create_session("session1".to_string(), Some("main".to_string())).unwrap();
        let _session2 = manager.create_session("session2".to_string(), Some("main".to_string())).unwrap();

        let sessions = manager.list_sessions().unwrap();
        assert_eq!(sessions.len(), 2);

        let session1_id = manager.generate_session_id(&session1.name);
        manager.update_session_status(&session1_id, SessionStatus::Finished).unwrap();
        
        let sessions = manager.list_sessions().unwrap();
        assert_eq!(sessions.len(), 2);
    }

    #[test]
    fn test_session_manager_delete() {
        let (_temp_dir, config) = setup_test_repo();
        let mut manager = SessionManager::new(&config);

        let session = manager.create_session("test-session".to_string(), Some("main".to_string())).unwrap();
        assert!(manager.session_exists(&session.name));

        manager.delete_state(&session.name).unwrap();
        assert!(!manager.session_exists(&session.name));
    }

    #[test]
    fn test_session_manager_update_status() {
        let (_temp_dir, config) = setup_test_repo();
        let mut manager = SessionManager::new(&config);

        let session = manager.create_session("test-session".to_string(), Some("main".to_string())).unwrap();
        
        manager.update_session_status(&session.name, SessionStatus::Finished).unwrap();
        
        let updated_session = manager.load_state(&session.name).unwrap();
        assert!(matches!(updated_session.status, SessionStatus::Finished));
    }
}