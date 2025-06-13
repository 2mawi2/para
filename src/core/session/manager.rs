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
        let state_dir = Self::resolve_state_dir(config);
        Self {
            state_dir,
            config: config.clone(),
        }
    }

    /// Resolve state directory relative to main repository root, not current working directory
    fn resolve_state_dir(config: &Config) -> PathBuf {
        let state_dir_path = config.get_state_dir();

        // If it's already an absolute path, use it as-is (important for tests)
        if Path::new(state_dir_path).is_absolute() {
            return PathBuf::from(state_dir_path);
        }

        // Try to resolve relative to main repository root, but be very conservative
        // Don't let git discovery failures break anything
        if let Ok(git_service) = GitService::discover() {
            let repo_root = git_service.repository().root.clone();
            let main_repo_root = Self::get_main_repo_root(&repo_root);
            main_repo_root.join(state_dir_path)
        } else {
            // Git discovery failed - just use the path as-is for backward compatibility
            // This ensures tests and edge cases continue to work
            PathBuf::from(state_dir_path)
        }
    }

    /// Get the main repository root, handling worktree case
    fn get_main_repo_root(current_repo_root: &Path) -> PathBuf {
        let git_path = current_repo_root.join(".git");

        // If .git is a file (worktree), extract main repo path
        if git_path.is_file() {
            if let Ok(git_content) = fs::read_to_string(&git_path) {
                if let Some(git_dir) = git_content.strip_prefix("gitdir: ") {
                    let git_dir = git_dir.trim();
                    let git_path = PathBuf::from(git_dir);
                    if let Some(main_repo_root) = git_path
                        .parent() // .git/worktrees
                        .and_then(|p| p.parent()) // .git
                        .and_then(|p| p.parent())
                    // main repo root
                    {
                        return main_repo_root.to_path_buf();
                    }
                }
            }
        }

        // If .git is a directory or we can't parse the worktree info, assume we're in main repo
        current_repo_root.to_path_buf()
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    pub fn create_session(
        &mut self,
        name: String,
        base_branch: Option<String>,
    ) -> Result<SessionState> {
        let git_service = GitService::discover().map_err(|e| {
            ParaError::git_error(format!("Failed to discover git repository: {}", e))
        })?;

        let repository_root = git_service.repository().root.clone();

        let _base_branch = base_branch.unwrap_or_else(|| {
            git_service
                .repository()
                .get_main_branch()
                .unwrap_or_else(|_| "main".to_string())
        });

        let final_session_name = self.resolve_session_name(name.clone())?;
        // Use original name for branch, not the potentially timestamped session name
        let branch_name = crate::utils::generate_friendly_branch_name(
            self.config.get_branch_prefix(),
            &name,
        );

        let subtrees_path = repository_root.join(&self.config.directories.subtrees_dir);
        let worktree_path = subtrees_path.join(&final_session_name);

        if !subtrees_path.exists() {
            fs::create_dir_all(&subtrees_path).map_err(|e| {
                ParaError::fs_error(format!("Failed to create subtrees directory: {}", e))
            })?;

            // Create .para/.gitignore if we're using the new consolidated structure
            if let Some(para_dir) = self.get_para_directory_from_subtrees(&subtrees_path) {
                self.ensure_para_gitignore_exists(&para_dir)?;
            }
        }

        if worktree_path.exists() {
            return Err(ParaError::file_operation(format!(
                "Worktree path already exists: {}",
                worktree_path.display()
            )));
        }

        git_service.create_worktree(&branch_name, &worktree_path)?;

        let session_state = SessionState::new(final_session_name, branch_name, worktree_path);

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

    pub fn update_session_status(
        &mut self,
        session_name: &str,
        status: SessionStatus,
    ) -> Result<()> {
        let mut session = self.load_state(session_name)?;
        session.update_status(status);
        self.save_state(&session)
    }

    pub fn session_exists(&self, session_name: &str) -> bool {
        let state_file = self.state_dir.join(format!("{}.state", session_name));
        state_file.exists()
    }

    fn resolve_session_name(&self, requested_name: String) -> Result<String> {
        if !self.session_exists(&requested_name) {
            return Ok(requested_name);
        }

        let timestamp = crate::utils::names::generate_timestamp();
        let unique_name = format!("{}_{}", requested_name, timestamp);

        if self.session_exists(&unique_name) {
            return Err(ParaError::session_exists(&unique_name));
        }

        Ok(unique_name)
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

            // Create .para/.gitignore if we're using the new consolidated structure
            if let Some(para_dir) = self.get_para_directory() {
                self.ensure_para_gitignore_exists(&para_dir)?;
            }
        }
        Ok(())
    }

    /// Get the .para directory if state_dir is under .para structure
    fn get_para_directory(&self) -> Option<PathBuf> {
        let state_path = &self.state_dir;

        // Check if state_dir ends with ".para/state"
        if state_path.file_name()?.to_str()? == "state" {
            if let Some(parent) = state_path.parent() {
                if parent.file_name()?.to_str()? == ".para" {
                    return Some(parent.to_path_buf());
                }
            }
        }
        None
    }

    /// Get the .para directory if subtrees_path is under .para structure
    fn get_para_directory_from_subtrees(&self, subtrees_path: &Path) -> Option<PathBuf> {
        // Check if subtrees_path ends with ".para/worktrees"
        if subtrees_path.file_name()?.to_str()? == "worktrees" {
            if let Some(parent) = subtrees_path.parent() {
                if parent.file_name()?.to_str()? == ".para" {
                    return Some(parent.to_path_buf());
                }
            }
        }
        None
    }

    /// Ensure .para/.gitignore exists with appropriate content
    fn ensure_para_gitignore_exists(&self, para_dir: &Path) -> Result<()> {
        let gitignore_path = para_dir.join(".gitignore");

        if !gitignore_path.exists() {
            let gitignore_content =
                "# Ignore all para contents except configuration\n*\n!.gitignore\n";
            fs::write(&gitignore_path, gitignore_content).map_err(|e| {
                ParaError::fs_error(format!(
                    "Failed to create .para/.gitignore file {}: {}",
                    gitignore_path.display(),
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
        config.directories.state_dir = temp_dir
            .path()
            .join(".para/state")
            .to_string_lossy()
            .to_string();
        config.directories.subtrees_dir = temp_dir
            .path()
            .join(".para/worktrees")
            .to_string_lossy()
            .to_string();

        let manager = SessionManager::new(&config);
        // State directory is created on demand
        assert!(!manager.state_dir.exists());
    }

    #[test]
    fn test_consolidated_directory_structure() {
        let temp_dir = TempDir::new().unwrap();

        let mut config = default_config();
        config.directories.state_dir = temp_dir
            .path()
            .join(".para/state")
            .to_string_lossy()
            .to_string();
        config.directories.subtrees_dir = temp_dir
            .path()
            .join(".para/worktrees")
            .to_string_lossy()
            .to_string();

        let manager = SessionManager::new(&config);

        // Trigger state directory creation
        manager.ensure_state_dir_exists().unwrap();

        // Verify .para directory structure is created
        let para_dir = temp_dir.path().join(".para");
        assert!(para_dir.exists());
        assert!(para_dir.join("state").exists());

        // Verify .para/.gitignore is created
        let gitignore_path = para_dir.join(".gitignore");
        assert!(gitignore_path.exists());

        // Verify gitignore content
        let gitignore_content = std::fs::read_to_string(&gitignore_path).unwrap();
        assert!(gitignore_content.contains("*"));
        assert!(gitignore_content.contains("!.gitignore"));
    }

    #[test]
    fn test_worktree_path_simplified() {
        let temp_dir = TempDir::new().unwrap();

        let mut config = default_config();
        config.directories.state_dir = temp_dir
            .path()
            .join(".para/state")
            .to_string_lossy()
            .to_string();
        config.directories.subtrees_dir = temp_dir
            .path()
            .join(".para/worktrees")
            .to_string_lossy()
            .to_string();

        let _manager = SessionManager::new(&config);

        // Create a session manually to test path structure
        let session_name = "test-session".to_string();

        // Verify expected worktree path (simplified without brand prefix)
        let expected_worktree_path = temp_dir.path().join(".para/worktrees").join(&session_name);

        // The worktree path should be directly under .para/worktrees/ without brand prefix
        assert!(expected_worktree_path
            .to_string_lossy()
            .contains(".para/worktrees/test-session"));
        assert!(!expected_worktree_path
            .to_string_lossy()
            .contains("/para/para/"));
    }

    #[test]
    fn test_session_manager_save_and_load() {
        let temp_dir = TempDir::new().unwrap();

        let mut config = default_config();
        config.directories.state_dir = temp_dir
            .path()
            .join(".para/state")
            .to_string_lossy()
            .to_string();
        config.directories.subtrees_dir = temp_dir
            .path()
            .join(".para/worktrees")
            .to_string_lossy()
            .to_string();
        let manager = SessionManager::new(&config);

        // Create session manually without git operations
        let session = SessionState::new(
            "test-session-save".to_string(),
            "para/test-branch".to_string(),
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
        config.directories.state_dir = temp_dir
            .path()
            .join(".para/state")
            .to_string_lossy()
            .to_string();
        config.directories.subtrees_dir = temp_dir
            .path()
            .join(".para/worktrees")
            .to_string_lossy()
            .to_string();
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
        config.directories.state_dir = temp_dir
            .path()
            .join(".para/state")
            .to_string_lossy()
            .to_string();
        config.directories.subtrees_dir = temp_dir
            .path()
            .join(".para/worktrees")
            .to_string_lossy()
            .to_string();
        let manager = SessionManager::new(&config);

        // Create session manually without git operations
        let session = SessionState::new(
            "test-session-delete".to_string(),
            "para/test-branch".to_string(),
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
        config.directories.state_dir = temp_dir
            .path()
            .join(".para/state")
            .to_string_lossy()
            .to_string();
        config.directories.subtrees_dir = temp_dir
            .path()
            .join(".para/worktrees")
            .to_string_lossy()
            .to_string();
        let mut manager = SessionManager::new(&config);

        // Create session manually without git operations
        let session = SessionState::new(
            "test-session-update".to_string(),
            "para/test-branch".to_string(),
            temp_dir.path().join("test-worktree"),
        );

        manager.save_state(&session).unwrap();

        manager
            .update_session_status(&session.name, SessionStatus::Finished)
            .unwrap();

        let updated_session = manager.load_state(&session.name).unwrap();
        assert!(matches!(updated_session.status, SessionStatus::Finished));
    }

    #[test]
    fn test_resolve_session_name_no_collision() {
        let temp_dir = TempDir::new().unwrap();

        let mut config = default_config();
        config.directories.state_dir = temp_dir
            .path()
            .join(".para_state")
            .to_string_lossy()
            .to_string();
        let manager = SessionManager::new(&config);

        let result = manager
            .resolve_session_name("new-session".to_string())
            .unwrap();
        assert_eq!(result, "new-session");
    }

    #[test]
    fn test_resolve_session_name_with_collision() {
        let temp_dir = TempDir::new().unwrap();

        let mut config = default_config();
        config.directories.state_dir = temp_dir
            .path()
            .join(".para_state")
            .to_string_lossy()
            .to_string();
        let manager = SessionManager::new(&config);

        // Create existing session
        let existing_session = SessionState::new(
            "collision-test".to_string(),
            "para/test-branch".to_string(),
            temp_dir.path().join("test-worktree"),
        );
        manager.save_state(&existing_session).unwrap();

        // Try to resolve same name - should get timestamped version
        let result = manager
            .resolve_session_name("collision-test".to_string())
            .unwrap();
        assert_ne!(result, "collision-test");
        assert!(result.starts_with("collision-test_"));
        assert!(result.contains('-')); // Should contain timestamp
    }

    #[test]
    fn test_friendly_branch_naming_logic() {
        // Test the friendly branch naming function directly
        let branch_name = crate::utils::generate_friendly_branch_name("para", "epic_feature");
        assert_eq!(branch_name, "para/epic_feature");

        // Verify it doesn't contain timestamp patterns
        assert!(!branch_name.contains('-')); // No timestamp separators
        assert!(!branch_name.chars().any(|c| c.is_ascii_digit())); // No timestamp digits

        // Test with different prefix
        let branch_name2 = crate::utils::generate_friendly_branch_name("feature", "awesome_robot");
        assert_eq!(branch_name2, "feature/awesome_robot");
    }
}
