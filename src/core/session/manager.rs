use super::state::{SessionState, SessionStatus};
use crate::config::Config;
use crate::core::git::{GitOperations, GitService};
use crate::utils::{get_main_repository_root_from, GitignoreManager, ParaError, Result};
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

    fn resolve_state_dir(config: &Config) -> PathBuf {
        let state_dir_path = config.get_state_dir();

        if Path::new(state_dir_path).is_absolute() {
            return PathBuf::from(state_dir_path);
        }

        // Use the reliable git rev-parse method to find the main repository root
        if let Ok(main_repo_root) = get_main_repository_root_from(None) {
            main_repo_root.join(state_dir_path)
        } else {
            // Fallback to current directory if not in a git repository
            PathBuf::from(state_dir_path)
        }
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    pub fn create_session(
        &mut self,
        name: String,
        base_branch: Option<String>,
    ) -> Result<SessionState> {
        self.create_session_with_type(name, base_branch, None)
    }

    /// Create a session with specified type (worktree or container)
    pub fn create_session_with_type(
        &mut self,
        name: String,
        base_branch: Option<String>,
        session_type: Option<super::state::SessionType>,
    ) -> Result<SessionState> {
        let git_service = GitService::discover().map_err(|e| {
            ParaError::git_error(format!("Failed to discover git repository: {}", e))
        })?;

        let repository_root = git_service.repository().root.clone();

        GitignoreManager::ensure_para_ignored_in_repository(&repository_root)?;

        let _base_branch = base_branch.unwrap_or_else(|| {
            git_service
                .repository()
                .get_main_branch()
                .unwrap_or_else(|_| "main".to_string())
        });

        let final_session_name = self.resolve_session_name(name)?;
        let branch_name = crate::utils::generate_friendly_branch_name(
            self.config.get_branch_prefix(),
            &final_session_name,
        );

        let subtrees_path = repository_root.join(&self.config.directories.subtrees_dir);
        let worktree_path = subtrees_path.join(&final_session_name);

        if !subtrees_path.exists() {
            fs::create_dir_all(&subtrees_path).map_err(|e| {
                ParaError::fs_error(format!("Failed to create subtrees directory: {}", e))
            })?;

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

        let session_state = match session_type {
            Some(super::state::SessionType::Container { container_id }) => {
                SessionState::new_container(
                    final_session_name,
                    branch_name,
                    worktree_path,
                    container_id,
                )
            }
            _ => SessionState::new(final_session_name, branch_name, worktree_path),
        };

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

        let mut session: SessionState = serde_json::from_str(&content).map_err(|e| {
            ParaError::state_corruption(format!(
                "Failed to parse session state from {}: {}",
                state_file.display(),
                e
            ))
        })?;

        // Handle backward compatibility - migrate is_docker to session_type
        if let Some(is_docker) = session.is_docker {
            if is_docker {
                session.session_type = super::state::SessionType::Container { container_id: None };
            }
            session.is_docker = None;
        }

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
        // Delete the main state file
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

        // Delete the status file
        let status_file = self.state_dir.join(format!("{}.status.json", session_name));
        if status_file.exists() {
            fs::remove_file(&status_file).map_err(|e| {
                ParaError::file_operation(format!(
                    "Failed to delete session status {}: {}",
                    status_file.display(),
                    e
                ))
            })?;
        }
        Ok(())
    }

    pub fn list_sessions(&self) -> Result<Vec<SessionState>> {
        crate::utils::debug_log("Listing sessions");

        if !self.state_dir.exists() {
            crate::utils::debug_log("State directory does not exist");
            return Ok(Vec::new());
        }

        let session_files = self.collect_session_files()?;

        let sessions = session_files
            .iter()
            .filter_map(|path| self.process_session_file(path).unwrap_or(None))
            .collect::<Vec<_>>();

        let mut sessions = sessions;
        sessions.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        crate::utils::debug_log(&format!("Found {} sessions", sessions.len()));
        Ok(sessions)
    }

    fn collect_session_files(&self) -> Result<Vec<PathBuf>> {
        let entries = match fs::read_dir(&self.state_dir) {
            Ok(entries) => entries,
            Err(e) => {
                crate::utils::debug_log(&format!("Failed to read state directory: {}", e));
                return Ok(Vec::new());
            }
        };

        let mut session_files = Vec::new();

        for entry in entries {
            let entry = match entry {
                Ok(e) => e,
                Err(e) => {
                    crate::utils::debug_log(&format!("Failed to read directory entry: {}", e));
                    continue;
                }
            };

            let path = entry.path();
            if path.is_file() && path.extension().is_some_and(|ext| ext == "state") {
                session_files.push(path);
            }
        }

        Ok(session_files)
    }

    fn extract_session_name(&self, path: &Path) -> Result<Option<String>> {
        let Some(stem) = path.file_stem() else {
            return Ok(None);
        };

        let Some(session_name) = stem.to_str() else {
            return Ok(None);
        };

        Ok(Some(session_name.to_string()))
    }

    fn process_session_file(&self, path: &Path) -> Result<Option<SessionState>> {
        let Some(session_name) = self.extract_session_name(path)? else {
            return Ok(None);
        };

        crate::utils::debug_log(&format!("Loading session: {}", session_name));

        match self.load_state(&session_name) {
            Ok(state) => Ok(Some(state)),
            Err(e) => {
                crate::utils::debug_log(&format!("Failed to load session {}: {}", session_name, e));
                Ok(None)
            }
        }
    }

    pub fn find_session_by_path(&self, path: &Path) -> Result<Option<SessionState>> {
        crate::utils::debug_log(&format!("Finding session by path: {}", path.display()));
        let sessions = self.list_sessions()?;
        let normalized_path = crate::utils::safe_resolve_path(path);

        for session in sessions {
            let session_normalized = crate::utils::safe_resolve_path(&session.worktree_path);

            if normalized_path == session_normalized
                || normalized_path.starts_with(&session_normalized)
                || session_normalized.starts_with(&normalized_path)
            {
                crate::utils::debug_log(&format!("Found matching session: {}", session.name));
                return Ok(Some(session));
            }
        }

        crate::utils::debug_log("No matching session found");
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

            if let Ok(git_service) = GitService::discover() {
                let repository_root = git_service.repository().root.clone();
                GitignoreManager::ensure_para_ignored_in_repository(&repository_root)?;
            }

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
            return state_path.parent().and_then(|parent| {
                if parent.file_name()?.to_str()? == ".para" {
                    Some(parent.to_path_buf())
                } else {
                    None
                }
            });
        }
        None
    }

    /// Get the .para directory if subtrees_path is under .para structure
    fn get_para_directory_from_subtrees(&self, subtrees_path: &Path) -> Option<PathBuf> {
        // Check if subtrees_path ends with ".para/worktrees"
        if subtrees_path.file_name()?.to_str()? == "worktrees" {
            return subtrees_path.parent().and_then(|parent| {
                if parent.file_name()?.to_str()? == ".para" {
                    Some(parent.to_path_buf())
                } else {
                    None
                }
            });
        }
        None
    }

    /// Ensure .para/.gitignore exists with appropriate content
    fn ensure_para_gitignore_exists(&self, para_dir: &Path) -> Result<()> {
        GitignoreManager::create_para_internal_gitignore(para_dir)
    }

    pub fn create_docker_session(
        &mut self,
        name: String,
        docker_manager: &crate::core::docker::DockerManager,
        _initial_prompt: Option<&str>,
        docker_args: &[String],
    ) -> Result<SessionState> {
        // Create a container-type session
        let session_state = self.create_session_with_type(
            name,
            None,
            Some(super::state::SessionType::Container { container_id: None }),
        )?;

        // Create the Docker container
        docker_manager
            .create_container_session(&session_state, docker_args)
            .map_err(|e| ParaError::docker_error(format!("Failed to create container: {}", e)))?;

        // TODO: Update session with actual container ID after creation

        // Devcontainer config is already generated by DockerManager

        Ok(session_state)
    }

    pub fn cancel_session(&mut self, session_name: &str, force: bool) -> Result<()> {
        let session = self.load_state(session_name)?;

        // TODO: Connect to CLI in next phase
        // Docker cleanup will be added when CLI integration is complete
        // if session.is_container() {
        //     // Docker container cleanup
        // }

        // Remove the session state file
        let state_file = self.state_dir.join(format!("{}.state", session_name));
        if state_file.exists() {
            fs::remove_file(&state_file)
                .map_err(|e| ParaError::fs_error(format!("Failed to remove state file: {}", e)))?;
        }

        // Clean up the worktree if requested or if it's a Docker session
        if (force || session.is_container()) && session.worktree_path.exists() {
            fs::remove_dir_all(&session.worktree_path)
                .map_err(|e| ParaError::fs_error(format!("Failed to remove worktree: {}", e)))?;
        }

        Ok(())
    }
}

#[cfg(test)]
#[path = "manager/manager_mixed_tests.rs"]
mod manager_mixed_tests;

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

        manager.ensure_state_dir_exists().unwrap();

        let para_dir = temp_dir.path().join(".para");
        assert!(para_dir.exists());
        assert!(para_dir.join("state").exists());

        let gitignore_path = para_dir.join(".gitignore");
        assert!(gitignore_path.exists());

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

        let session_name = "test-session".to_string();

        let expected_worktree_path = temp_dir.path().join(".para/worktrees").join(&session_name);

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

        let session = SessionState::new(
            "test-session-save".to_string(),
            "para/test-branch".to_string(),
            temp_dir.path().join("test-worktree"),
        );

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

    #[test]
    fn test_friendly_naming_no_conflicts() {
        let temp_dir = TempDir::new().unwrap();

        let mut config = default_config();
        config.directories.state_dir = temp_dir
            .path()
            .join(".para_state")
            .to_string_lossy()
            .to_string();
        config.directories.subtrees_dir = temp_dir
            .path()
            .join(".para_worktrees")
            .to_string_lossy()
            .to_string();

        let manager = SessionManager::new(&config);

        // Test session name resolution without conflict
        let result = manager
            .resolve_session_name("my-awesome-feature".to_string())
            .unwrap();
        assert_eq!(result, "my-awesome-feature");

        // Test branch name generation
        let branch_name = crate::utils::generate_friendly_branch_name("test", &result);
        assert_eq!(branch_name, "test/my-awesome-feature");

        // Verify no timestamp in the name since there was no conflict
        assert!(!result.contains("20"));
        assert!(!branch_name.contains("20"));
    }

    #[test]
    fn test_session_name_conflict_resolution() {
        let temp_dir = TempDir::new().unwrap();

        let mut config = default_config();
        config.directories.state_dir = temp_dir
            .path()
            .join(".para_state")
            .to_string_lossy()
            .to_string();
        config.directories.subtrees_dir = temp_dir
            .path()
            .join(".para_worktrees")
            .to_string_lossy()
            .to_string();

        let manager = SessionManager::new(&config);

        // Create first session manually to simulate existing session
        let existing_session = SessionState::new(
            "duplicate-test".to_string(),
            "test/duplicate-test".to_string(),
            temp_dir.path().join(".para_worktrees/duplicate-test"),
        );
        manager.save_state(&existing_session).unwrap();

        // Test: Try to resolve session with same name - should get timestamped version
        let result = manager
            .resolve_session_name("duplicate-test".to_string())
            .unwrap();

        // Verify session name got timestamp suffix due to conflict
        assert_ne!(result, "duplicate-test");
        assert!(result.starts_with("duplicate-test_"));
        assert!(result.contains("20")); // Should contain timestamp

        // Test branch name generation with timestamped session name
        let branch_name = crate::utils::generate_friendly_branch_name("test", &result);
        assert!(branch_name.starts_with("test/duplicate-test_"));
        assert!(branch_name.contains("20")); // Should contain timestamp

        // Verify both session and branch use the same timestamp suffix
        let session_suffix = result.strip_prefix("duplicate-test_").unwrap();
        let branch_suffix = branch_name.strip_prefix("test/duplicate-test_").unwrap();
        assert_eq!(session_suffix, branch_suffix);
    }

    #[test]
    fn test_session_and_branch_consistency() {
        // Test that session names and branch names are consistent
        let session_name = "new-feature";
        let expected_branch = crate::utils::generate_friendly_branch_name("test", session_name);
        assert_eq!(expected_branch, "test/new-feature");

        // Test with timestamped name
        let timestamped_name = "feature-x_20250613-123456";
        let expected_timestamped_branch =
            crate::utils::generate_friendly_branch_name("test", timestamped_name);
        assert_eq!(
            expected_timestamped_branch,
            "test/feature-x_20250613-123456"
        );
    }

    #[test]
    fn test_find_session_by_path() {
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

        // Create test sessions
        let session1_worktree = temp_dir.path().join(".para/worktrees/feature1");
        std::fs::create_dir_all(&session1_worktree).unwrap();
        let session1 = SessionState::new(
            "feature1".to_string(),
            "test/feature1".to_string(),
            session1_worktree.clone(),
        );

        let session2_worktree = temp_dir.path().join(".para/worktrees/feature2");
        std::fs::create_dir_all(&session2_worktree).unwrap();
        let session2 = SessionState::new(
            "feature2".to_string(),
            "test/feature2".to_string(),
            session2_worktree.clone(),
        );

        manager.save_state(&session1).unwrap();
        manager.save_state(&session2).unwrap();

        // Test exact match
        let found = manager
            .find_session_by_path(&session1_worktree)
            .unwrap()
            .unwrap();
        assert_eq!(found.name, "feature1");

        // Test path inside worktree
        let nested_path = session1_worktree.join("src/main.rs");
        std::fs::create_dir_all(nested_path.parent().unwrap()).unwrap();
        std::fs::write(&nested_path, "test content").unwrap();

        let found = manager.find_session_by_path(&nested_path).unwrap().unwrap();
        assert_eq!(found.name, "feature1");

        // Test non-existent path
        let non_existent = temp_dir.path().join("random/path");
        let result = manager.find_session_by_path(&non_existent).unwrap();
        assert!(result.is_none());

        // Test path outside any worktree
        let outside_path = temp_dir.path().join("outside");
        std::fs::create_dir_all(&outside_path).unwrap();
        let result = manager.find_session_by_path(&outside_path).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_find_session_by_branch() {
        let temp_dir = TempDir::new().unwrap();

        let mut config = default_config();
        config.directories.state_dir = temp_dir
            .path()
            .join(".para/state")
            .to_string_lossy()
            .to_string();
        let manager = SessionManager::new(&config);

        // Create test sessions with different statuses
        let active_session = SessionState::new(
            "active-feature".to_string(),
            "test/active-branch".to_string(),
            temp_dir.path().join("active"),
        );

        let mut finished_session = SessionState::new(
            "finished-feature".to_string(),
            "test/finished-branch".to_string(),
            temp_dir.path().join("finished"),
        );
        finished_session.update_status(SessionStatus::Finished);

        let mut cancelled_session = SessionState::new(
            "cancelled-feature".to_string(),
            "test/cancelled-branch".to_string(),
            temp_dir.path().join("cancelled"),
        );
        cancelled_session.update_status(SessionStatus::Cancelled);

        manager.save_state(&active_session).unwrap();
        manager.save_state(&finished_session).unwrap();
        manager.save_state(&cancelled_session).unwrap();

        // Test finding active session by branch
        let found = manager
            .find_session_by_branch("test/active-branch")
            .unwrap()
            .unwrap();
        assert_eq!(found.name, "active-feature");
        assert!(matches!(found.status, SessionStatus::Active));

        // Test that finished sessions are not found by branch search
        let result = manager
            .find_session_by_branch("test/finished-branch")
            .unwrap();
        assert!(result.is_none());

        // Test that cancelled sessions are not found by branch search
        let result = manager
            .find_session_by_branch("test/cancelled-branch")
            .unwrap();
        assert!(result.is_none());

        // Test non-existent branch
        let result = manager.find_session_by_branch("test/non-existent").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_find_session_with_symlinks() {
        let temp_dir = TempDir::new().unwrap();

        let mut config = default_config();
        config.directories.state_dir = temp_dir
            .path()
            .join(".para/state")
            .to_string_lossy()
            .to_string();
        let manager = SessionManager::new(&config);

        // Create a session
        let worktree_path = temp_dir.path().join("worktree");
        std::fs::create_dir_all(&worktree_path).unwrap();
        let session = SessionState::new(
            "symlink-test".to_string(),
            "test/symlink-test".to_string(),
            worktree_path.clone(),
        );
        manager.save_state(&session).unwrap();

        // Create a symlink to the worktree
        let symlink_path = temp_dir.path().join("symlink");
        #[cfg(unix)]
        std::os::unix::fs::symlink(&worktree_path, &symlink_path).unwrap();
        #[cfg(windows)]
        std::os::windows::fs::symlink_dir(&worktree_path, &symlink_path).unwrap();

        // Test finding session through symlink
        let found = manager
            .find_session_by_path(&symlink_path)
            .unwrap()
            .unwrap();
        assert_eq!(found.name, "symlink-test");
    }
}
