use super::state::{SessionConfig, SessionState, SessionStatus, SessionSummary, SessionType, StateFileFormat};
use crate::config::Config;
use crate::core::git::{GitOperations, GitService};
use crate::utils::{ParaError, Result};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct CreateSessionParams {
    pub name: String,
    pub session_type: SessionType,
    pub initial_prompt: Option<String>,
    pub base_branch: Option<String>,
}

pub struct SessionManager {
    state_dir: PathBuf,
    config: Config,
}

impl SessionManager {
    pub fn new(config: Config) -> Result<Self> {
        let state_dir = PathBuf::from(config.get_state_dir()).join("sessions");
        
        if !state_dir.exists() {
            fs::create_dir_all(&state_dir).map_err(|e| {
                ParaError::fs_error(format!(
                    "Failed to create sessions state directory {}: {}",
                    state_dir.display(),
                    e
                ))
            })?;
        }

        Ok(Self { state_dir, config })
    }

    pub fn create_session(&mut self, params: CreateSessionParams) -> Result<SessionState> {
        let git_service = GitService::discover()
            .map_err(|e| ParaError::git_error(format!("Failed to discover git repository: {}", e)))?;

        let repository_root = git_service.repository().root.clone();
        
        let base_branch = params.base_branch.unwrap_or_else(|| {
            git_service
                .repository()
                .get_main_branch()
                .unwrap_or_else(|_| "main".to_string())
        });

        let branch_name = crate::utils::generate_branch_name(self.config.get_branch_prefix());
        
        let subtrees_path = repository_root.join(&self.config.directories.subtrees_dir);
        let session_id = crate::utils::generate_session_id(&params.name);
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

        let config_snapshot = SessionConfig::from_config(&self.config);

        let session_state = match params.session_type {
            SessionType::Manual => SessionState::new_manual(
                params.name,
                branch_name,
                base_branch,
                worktree_path,
                repository_root,
                config_snapshot,
            ),
            SessionType::Dispatched => SessionState::new_dispatched(
                params.name,
                branch_name,
                base_branch,
                worktree_path,
                repository_root,
                params.initial_prompt.unwrap_or_default(),
                config_snapshot,
            ),
            SessionType::Recovered => SessionState::new_recovered(
                params.name,
                branch_name,
                base_branch,
                worktree_path,
                repository_root,
                config_snapshot,
            ),
        };

        self.save_session(&session_state)?;

        Ok(session_state)
    }

    pub fn load_session(&self, session_id: &str) -> Result<SessionState> {
        let state_file = self.state_dir.join(format!("{}.json", session_id));
        if !state_file.exists() {
            return Err(ParaError::session_not_found(session_id));
        }

        let content = fs::read_to_string(&state_file).map_err(|e| {
            ParaError::file_operation(format!(
                "Failed to read session state from {}: {}",
                state_file.display(),
                e
            ))
        })?;

        let file_format: StateFileFormat = serde_json::from_str(&content).map_err(|e| {
            ParaError::state_corruption(format!(
                "Failed to parse session state from {}: {}",
                state_file.display(),
                e
            ))
        })?;

        Ok(file_format.session)
    }

    pub fn save_session(&self, session: &SessionState) -> Result<()> {
        let state_file = self.state_dir.join(format!("{}.json", session.id));
        let file_format = session.to_file_format();
        let json = serde_json::to_string_pretty(&file_format)?;
        
        fs::write(&state_file, json).map_err(|e| {
            ParaError::file_operation(format!(
                "Failed to save session state to {}: {}",
                state_file.display(),
                e
            ))
        })?;

        self.create_backup(session)?;

        Ok(())
    }

    pub fn delete_session(&self, session_id: &str) -> Result<()> {
        let state_file = self.state_dir.join(format!("{}.json", session_id));
        if state_file.exists() {
            fs::remove_file(&state_file).map_err(|e| {
                ParaError::file_operation(format!(
                    "Failed to delete session state {}: {}",
                    state_file.display(),
                    e
                ))
            })?;
        }

        let backup_file = self.get_backup_dir()?.join(format!("{}.json.bak", session_id));
        if backup_file.exists() {
            let _ = fs::remove_file(&backup_file);
        }

        Ok(())
    }

    pub fn list_active_sessions(&self) -> Result<Vec<SessionSummary>> {
        self.list_sessions_with_status(Some(SessionStatus::Active))
    }

    pub fn list_all_sessions(&self) -> Result<Vec<SessionSummary>> {
        self.list_sessions_with_status(None)
    }

    fn list_sessions_with_status(&self, status_filter: Option<SessionStatus>) -> Result<Vec<SessionSummary>> {
        if !self.state_dir.exists() {
            return Ok(Vec::new());
        }

        let entries = fs::read_dir(&self.state_dir)?;
        let mut sessions = Vec::new();

        for entry in entries {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() && path.extension().is_some_and(|ext| ext == "json") {
                if let Some(stem) = path.file_stem() {
                    if let Some(session_id) = stem.to_str() {
                        match self.load_session(session_id) {
                            Ok(session) => {
                                if let Some(ref filter_status) = status_filter {
                                    if std::mem::discriminant(&session.status) == std::mem::discriminant(filter_status) {
                                        sessions.push(session.to_summary());
                                    }
                                } else {
                                    sessions.push(session.to_summary());
                                }
                            }
                            Err(_) => continue,
                        }
                    }
                }
            }
        }

        sessions.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(sessions)
    }

    pub fn find_session_by_path(&self, path: &Path) -> Result<Option<SessionState>> {
        let sessions = self.list_all_sessions()?;
        
        for summary in sessions {
            if let Ok(session) = self.load_session(&summary.id) {
                if session.worktree_path == path || path.starts_with(&session.worktree_path) {
                    return Ok(Some(session));
                }
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

        let sessions = self.list_active_sessions()?;
        for summary in sessions {
            if let Ok(session) = self.load_session(&summary.id) {
                if session.branch == current_branch {
                    return Ok(session);
                }
            }
        }

        Err(ParaError::session_not_found("auto-detected"))
    }

    pub fn update_session_status(&mut self, session_id: &str, status: SessionStatus) -> Result<()> {
        let mut session = self.load_session(session_id)?;
        session.update_status(status);
        self.save_session(&session)
    }

    pub fn update_session_commit(&mut self, session_id: &str, commit_hash: String) -> Result<()> {
        let mut session = self.load_session(session_id)?;
        session.update_commit_info(commit_hash);
        self.save_session(&session)
    }

    pub fn session_exists(&self, session_id: &str) -> bool {
        let state_file = self.state_dir.join(format!("{}.json", session_id));
        state_file.exists()
    }

    pub fn cleanup_orphaned_sessions(&mut self) -> Result<Vec<String>> {
        let git_service = GitService::discover()
            .map_err(|e| ParaError::git_error(format!("Failed to discover git repository: {}", e)))?;

        let sessions = self.list_all_sessions()?;
        let mut cleaned_up = Vec::new();

        for summary in sessions {
            if let Ok(session) = self.load_session(&summary.id) {
                let worktree_exists = session.worktree_path.exists();
                let branch_exists = git_service.repository()
                    .get_branches()
                    .map(|branches| branches.contains(&session.branch))
                    .unwrap_or(false);

                if !worktree_exists && !branch_exists {
                    self.delete_session(&session.id)?;
                    cleaned_up.push(session.id);
                }
            }
        }

        Ok(cleaned_up)
    }

    fn create_backup(&self, session: &SessionState) -> Result<()> {
        let backup_dir = self.get_backup_dir()?;
        let backup_file = backup_dir.join(format!("{}.json.bak", session.id));
        let file_format = session.to_file_format();
        let json = serde_json::to_string_pretty(&file_format)?;
        
        fs::write(&backup_file, json).map_err(|e| {
            ParaError::file_operation(format!(
                "Failed to create backup at {}: {}",
                backup_file.display(),
                e
            ))
        })?;

        Ok(())
    }

    fn get_backup_dir(&self) -> Result<PathBuf> {
        let backup_dir = self.state_dir.parent()
            .ok_or_else(|| ParaError::fs_error("Invalid state directory path".to_string()))?
            .join("backups");
        
        if !backup_dir.exists() {
            fs::create_dir_all(&backup_dir).map_err(|e| {
                ParaError::fs_error(format!(
                    "Failed to create backup directory {}: {}",
                    backup_dir.display(),
                    e
                ))
            })?;
        }

        Ok(backup_dir)
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

        let manager = SessionManager::new(config).unwrap();
        assert!(manager.state_dir.exists());
    }

    #[test]
    fn test_session_manager_save_and_load() {
        let (_temp_dir, config) = setup_test_repo();
        let manager = SessionManager::new(config).unwrap();

        let params = CreateSessionParams {
            name: "test-session".to_string(),
            session_type: SessionType::Manual,
            initial_prompt: None,
            base_branch: Some("main".to_string()),
        };

        let session = manager.create_session(params).unwrap();
        assert!(manager.session_exists(&session.id));

        let loaded_session = manager.load_session(&session.id).unwrap();
        assert_eq!(loaded_session.name, session.name);
        assert_eq!(loaded_session.branch, session.branch);
        assert_eq!(loaded_session.worktree_path, session.worktree_path);
    }

    #[test]
    fn test_session_manager_list_sessions() {
        let (_temp_dir, config) = setup_test_repo();
        let mut manager = SessionManager::new(config).unwrap();

        let params1 = CreateSessionParams {
            name: "session1".to_string(),
            session_type: SessionType::Manual,
            initial_prompt: None,
            base_branch: Some("main".to_string()),
        };

        let params2 = CreateSessionParams {
            name: "session2".to_string(),
            session_type: SessionType::Dispatched,
            initial_prompt: Some("Test prompt".to_string()),
            base_branch: Some("main".to_string()),
        };

        let session1 = manager.create_session(params1).unwrap();
        let session2 = manager.create_session(params2).unwrap();

        let sessions = manager.list_all_sessions().unwrap();
        assert_eq!(sessions.len(), 2);

        let active_sessions = manager.list_active_sessions().unwrap();
        assert_eq!(active_sessions.len(), 2);

        manager.update_session_status(&session1.id, SessionStatus::Completed).unwrap();
        let active_sessions = manager.list_active_sessions().unwrap();
        assert_eq!(active_sessions.len(), 1);
        assert_eq!(active_sessions[0].id, session2.id);
    }

    #[test]
    fn test_session_manager_delete() {
        let (_temp_dir, config) = setup_test_repo();
        let manager = SessionManager::new(config).unwrap();

        let params = CreateSessionParams {
            name: "test-session".to_string(),
            session_type: SessionType::Manual,
            initial_prompt: None,
            base_branch: Some("main".to_string()),
        };

        let session = manager.create_session(params).unwrap();
        assert!(manager.session_exists(&session.id));

        manager.delete_session(&session.id).unwrap();
        assert!(!manager.session_exists(&session.id));
    }

    #[test]
    fn test_session_manager_update_status() {
        let (_temp_dir, config) = setup_test_repo();
        let mut manager = SessionManager::new(config).unwrap();

        let params = CreateSessionParams {
            name: "test-session".to_string(),
            session_type: SessionType::Manual,
            initial_prompt: None,
            base_branch: Some("main".to_string()),
        };

        let session = manager.create_session(params).unwrap();
        
        manager.update_session_status(&session.id, SessionStatus::Finishing).unwrap();
        
        let updated_session = manager.load_session(&session.id).unwrap();
        assert!(matches!(updated_session.status, SessionStatus::Finishing));
    }

    #[test]
    fn test_session_manager_update_commit() {
        let (_temp_dir, config) = setup_test_repo();
        let mut manager = SessionManager::new(config).unwrap();

        let params = CreateSessionParams {
            name: "test-session".to_string(),
            session_type: SessionType::Manual,
            initial_prompt: None,
            base_branch: Some("main".to_string()),
        };

        let session = manager.create_session(params).unwrap();
        
        manager.update_session_commit(&session.id, "abc123def456".to_string()).unwrap();
        
        let updated_session = manager.load_session(&session.id).unwrap();
        assert_eq!(updated_session.commit_count, 1);
        assert_eq!(updated_session.last_commit_hash, Some("abc123def456".to_string()));
    }
}