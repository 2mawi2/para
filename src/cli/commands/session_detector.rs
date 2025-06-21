use crate::cli::commands::common::create_claude_local_md;
use crate::config::Config;
use crate::core::git::{GitService, SessionEnvironment};
use crate::core::session::{SessionManager, SessionStatus};
use crate::utils::{ParaError, Result};
use dialoguer::Select;
use std::env;
use std::path::Path;

/// Detects the current session environment and either resumes the current session
/// or provides interactive session selection
pub fn detect_and_resume_session(
    config: &Config,
    git_service: &GitService,
    session_manager: &SessionManager,
) -> Result<()> {
    let current_dir = env::current_dir()?;

    match git_service.validate_session_environment(&current_dir)? {
        SessionEnvironment::Worktree { branch, .. } => {
            handle_worktree_environment(config, session_manager, &current_dir, &branch)
        }
        SessionEnvironment::MainRepository => {
            handle_main_repository_environment(config, git_service, session_manager)
        }
        SessionEnvironment::Invalid => {
            handle_invalid_environment(config, git_service, session_manager)
        }
    }
}

/// Handles resuming a session when already in a worktree environment
fn handle_worktree_environment(
    config: &Config,
    session_manager: &SessionManager,
    current_dir: &Path,
    branch: &str,
) -> Result<()> {
    println!("Current directory is a worktree for branch: {}", branch);

    // Try to find session name from current directory or branch
    if let Some(session_name) = find_session_name_by_path_or_branch(
        session_manager,
        current_dir,
        branch,
    )? {
        create_claude_local_md(current_dir, &session_name)?;
    }

    launch_ide_for_current_session(config, current_dir)?;
    println!("✅ Resumed current session");
    Ok(())
}

/// Handles session selection when in the main repository
fn handle_main_repository_environment(
    config: &Config,
    git_service: &GitService,
    session_manager: &SessionManager,
) -> Result<()> {
    println!("Current directory is the main repository");
    list_and_select_session(config, git_service, session_manager)
}

/// Handles session selection when in an invalid environment
fn handle_invalid_environment(
    config: &Config,
    git_service: &GitService,
    session_manager: &SessionManager,
) -> Result<()> {
    println!("Current directory is not part of a para session");
    list_and_select_session(config, git_service, session_manager)
}

/// Lists available sessions and provides interactive selection
pub fn list_and_select_session(
    config: &Config,
    _git_service: &GitService,
    session_manager: &SessionManager,
) -> Result<()> {
    let sessions = session_manager.list_sessions()?;
    let active_sessions: Vec<_> = sessions
        .into_iter()
        .filter(|s| matches!(s.status, SessionStatus::Active))
        .collect();

    if active_sessions.is_empty() {
        println!("No active sessions found.");
        return Ok(());
    }

    display_session_list(&active_sessions);
    
    let selected_session = select_session_interactively(&active_sessions)?;
    if let Some(session) = selected_session {
        resume_selected_session(config, session)?;
    }

    Ok(())
}

/// Displays a formatted list of active sessions
fn display_session_list(sessions: &[crate::core::session::state::SessionState]) {
    println!("Active sessions:");
    for (i, session) in sessions.iter().enumerate() {
        println!("  {}: {} ({})", i + 1, session.name, session.branch);
    }
}

/// Provides interactive session selection and returns the selected session
fn select_session_interactively(
    sessions: &[crate::core::session::state::SessionState],
) -> Result<Option<&crate::core::session::state::SessionState>> {
    let selection = Select::new()
        .with_prompt("Select session to resume")
        .items(&sessions.iter().map(|s| &s.name).collect::<Vec<_>>())
        .interact();

    match selection {
        Ok(index) => Ok(Some(&sessions[index])),
        Err(_) => Ok(None), // User canceled selection
    }
}

/// Resumes the selected session by validating and launching it
fn resume_selected_session(
    config: &Config,
    session: &crate::core::session::state::SessionState,
) -> Result<()> {
    validate_session_worktree_exists(session)?;
    create_claude_local_md(&session.worktree_path, &session.name)?;
    launch_ide_for_current_session(config, &session.worktree_path)?;
    println!("✅ Resumed session '{}'", session.name);
    Ok(())
}

/// Validates that a session's worktree still exists
fn validate_session_worktree_exists(
    session: &crate::core::session::state::SessionState,
) -> Result<()> {
    if !session.worktree_path.exists() {
        return Err(ParaError::session_not_found(format!(
            "Session '{}' exists but worktree path '{}' not found",
            session.name,
            session.worktree_path.display()
        )));
    }
    Ok(())
}

/// Finds session name by matching against directory path or branch
fn find_session_name_by_path_or_branch(
    session_manager: &SessionManager,
    current_dir: &Path,
    branch: &str,
) -> Result<Option<String>> {
    let sessions = session_manager.list_sessions()?;
    Ok(sessions
        .into_iter()
        .find(|s| s.worktree_path == current_dir || s.branch == branch)
        .map(|s| s.name))
}

/// Launches IDE for the current session
fn launch_ide_for_current_session(config: &Config, path: &Path) -> Result<()> {
    use crate::core::ide::IdeManager;
    
    let ide_manager = IdeManager::new(config);

    // For Claude Code in wrapper mode, always use continuation flag when resuming
    if config.ide.name == "claude" && config.ide.wrapper.enabled {
        println!("▶ resuming Claude Code session with conversation continuation...");
        // Update existing tasks.json to include -c flag
        crate::cli::commands::task_transform::update_tasks_json_for_resume(path)?;
        ide_manager.launch_with_options(path, false, true)
    } else {
        ide_manager.launch(path, false)
    }
}

/// Resolves a session using different strategies based on what's available
pub struct SessionResolver<'a> {
    config: &'a Config,
    git_service: &'a GitService,
    session_manager: &'a SessionManager,
}

impl<'a> SessionResolver<'a> {
    pub fn new(
        config: &'a Config,
        git_service: &'a GitService,
        session_manager: &'a SessionManager,
    ) -> Self {
        Self {
            config,
            git_service,
            session_manager,
        }
    }

    /// Resolves a specific session by name using multiple fallback strategies
    pub fn resolve_session_by_name(&self, session_name: &str) -> Result<()> {
        // Strategy 1: Try exact session name match
        if let Ok(()) = self.try_exact_session_match(session_name) {
            return Ok(());
        }

        // Strategy 2: Try partial session name match (e.g., "test4" matches "test4_20250611-131147")
        if let Ok(()) = self.try_partial_session_match(session_name) {
            return Ok(());
        }

        // Strategy 3: Try worktree heuristic matching
        self.try_worktree_heuristic_match(session_name)
    }

    /// Strategy 1: Try to find and resume an exact session name match
    fn try_exact_session_match(&self, session_name: &str) -> Result<()> {
        if !self.session_manager.session_exists(session_name) {
            return Err(ParaError::session_not_found("Session not found".to_string()));
        }

        let mut session_state = self.session_manager.load_state(session_name)?;
        
        // Update worktree path if needed
        self.update_session_worktree_path(&mut session_state)?;
        
        // Resume the session
        self.resume_session_state(&session_state)
    }

    /// Strategy 2: Try to find a session with a partial name match
    fn try_partial_session_match(&self, session_name: &str) -> Result<()> {
        let sessions = self.session_manager.list_sessions()?;
        
        if let Some(candidate) = sessions
            .into_iter()
            .find(|s| s.name.starts_with(session_name))
        {
            // Recursively resolve with the full name
            return self.resolve_session_by_name(&candidate.name);
        }
        
        Err(ParaError::session_not_found("No partial match found".to_string()))
    }

    /// Strategy 3: Try to match by worktree branch or path patterns
    fn try_worktree_heuristic_match(&self, session_name: &str) -> Result<()> {
        let worktrees = self.git_service.list_worktrees()?;
        let matching_worktree = worktrees
            .iter()
            .find(|wt| {
                wt.branch.contains(session_name)
                    || wt
                        .path
                        .file_name()
                        .and_then(|name| name.to_str())
                        .map(|name| name.contains(session_name))
                        .unwrap_or(false)
            })
            .ok_or_else(|| ParaError::session_not_found(session_name.to_string()))?;

        // Try to find session name from matching worktree
        let display_name = if let Some(session_name) = self.find_session_name_for_worktree(matching_worktree)? {
            create_claude_local_md(&matching_worktree.path, &session_name)?;
            session_name
        } else {
            // Fallback: use session name from search
            create_claude_local_md(&matching_worktree.path, session_name)?;
            session_name.to_string()
        };

        launch_ide_for_current_session(self.config, &matching_worktree.path)?;
        println!("✅ Resumed session at '{}'", matching_worktree.path.display());
        Ok(())
    }

    /// Updates the session worktree path if the original path no longer exists
    fn update_session_worktree_path(
        &self,
        session_state: &mut crate::core::session::state::SessionState,
    ) -> Result<()> {
        if session_state.worktree_path.exists() {
            return Ok(());
        }

        let branch_to_match = session_state.branch.clone();
        let worktrees = self.git_service.list_worktrees()?;
        
        // Try to match by branch first
        if let Some(wt) = worktrees
            .iter()
            .find(|w| w.branch == branch_to_match)
        {
            session_state.worktree_path = wt.path.clone();
            self.session_manager.save_state(session_state)?;
            return Ok(());
        }
        
        // Try to match by session name pattern
        if let Some(wt) = worktrees.iter().find(|w| {
            w.path
                .file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.starts_with(&session_state.name))
                .unwrap_or(false)
        }) {
            session_state.worktree_path = wt.path.clone();
            self.session_manager.save_state(session_state)?;
            return Ok(());
        }

        Err(ParaError::session_not_found(format!(
            "Session '{}' exists but worktree path '{}' not found",
            session_state.name,
            session_state.worktree_path.display()
        )))
    }

    /// Resumes a session from its state
    fn resume_session_state(&self, session_state: &crate::core::session::state::SessionState) -> Result<()> {
        create_claude_local_md(&session_state.worktree_path, &session_state.name)?;
        launch_ide_for_current_session(self.config, &session_state.worktree_path)?;
        println!("✅ Resumed session '{}'", session_state.name);
        Ok(())
    }

    /// Finds the session name that corresponds to a given worktree
    fn find_session_name_for_worktree(
        &self,
        worktree: &crate::core::git::WorktreeInfo,
    ) -> Result<Option<String>> {
        let sessions = self.session_manager.list_sessions()?;
        Ok(sessions
            .into_iter()
            .find(|s| s.worktree_path == worktree.path || s.branch == worktree.branch)
            .map(|s| s.name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, DirectoryConfig, GitConfig, IdeConfig, SessionConfig, WrapperConfig};
    use crate::core::session::state::SessionState;
    use std::process::Command;
    use tempfile::TempDir;

    fn setup_test_environment() -> (TempDir, TempDir, GitService, Config) {
        let git_dir = TempDir::new().expect("tmp git");
        let state_dir = TempDir::new().expect("tmp state");
        let repo_path = git_dir.path();
        
        // Initialize git repo
        Command::new("git")
            .current_dir(repo_path)
            .args(["init", "--initial-branch=main"])
            .status()
            .unwrap();
        Command::new("git")
            .current_dir(repo_path)
            .args(["config", "user.name", "Test"])
            .status()
            .unwrap();
        Command::new("git")
            .current_dir(repo_path)
            .args(["config", "user.email", "test@example.com"])
            .status()
            .unwrap();
        
        std::fs::write(repo_path.join("README.md"), "# Test").unwrap();
        Command::new("git")
            .current_dir(repo_path)
            .args(["add", "README.md"])
            .status()
            .unwrap();
        Command::new("git")
            .current_dir(repo_path)
            .args(["commit", "-m", "init"])
            .status()
            .unwrap();

        let config = Config {
            ide: IdeConfig {
                name: "test".into(),
                command: "echo".into(),
                user_data_dir: None,
                wrapper: WrapperConfig {
                    enabled: false,
                    name: "test".into(),
                    command: "echo".into(),
                },
            },
            directories: DirectoryConfig {
                subtrees_dir: "subtrees/para".into(),
                state_dir: state_dir
                    .path()
                    .join(".para_state")
                    .to_string_lossy()
                    .to_string(),
            },
            git: GitConfig {
                branch_prefix: "para".into(),
                auto_stage: true,
                auto_commit: false,
            },
            session: SessionConfig {
                default_name_format: "%Y%m%d-%H%M%S".into(),
                preserve_on_finish: false,
                auto_cleanup_days: None,
            },
        };
        
        let service = GitService::discover_from(repo_path).unwrap();
        (git_dir, state_dir, service, config)
    }

    #[test]
    fn test_find_session_name_by_path_or_branch() {
        let (_git_tmp, _state_tmp, git_service, config) = setup_test_environment();
        let session_manager = SessionManager::new(&config);
        
        // Create a test session
        let session_name = "test_session".to_string();
        let branch_name = "para/test-branch".to_string();
        let worktree_path = git_service
            .repository()
            .root
            .join("test_worktree");

        let state = SessionState::new(session_name.clone(), branch_name.clone(), worktree_path.clone());
        session_manager.save_state(&state).unwrap();

        // Test finding by path
        let result = find_session_name_by_path_or_branch(
            &session_manager,
            &worktree_path,
            "other_branch",
        ).unwrap();
        assert_eq!(result, Some(session_name.clone()));

        // Test finding by branch
        let result = find_session_name_by_path_or_branch(
            &session_manager,
            &std::path::PathBuf::from("/other/path"),
            &branch_name,
        ).unwrap();
        assert_eq!(result, Some(session_name));

        // Test not found
        let result = find_session_name_by_path_or_branch(
            &session_manager,
            &std::path::PathBuf::from("/other/path"),
            "other_branch",
        ).unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_validate_session_worktree_exists() {
        let temp_dir = TempDir::new().unwrap();
        let existing_path = temp_dir.path().to_path_buf();
        let non_existing_path = temp_dir.path().join("non_existing");

        // Test with existing path
        let session = SessionState::new(
            "test".to_string(),
            "branch".to_string(),
            existing_path,
        );
        assert!(validate_session_worktree_exists(&session).is_ok());

        // Test with non-existing path
        let session = SessionState::new(
            "test".to_string(),
            "branch".to_string(),
            non_existing_path,
        );
        assert!(validate_session_worktree_exists(&session).is_err());
    }

    #[test]
    fn test_session_resolver_creation() {
        let (_git_tmp, _state_tmp, git_service, config) = setup_test_environment();
        let session_manager = SessionManager::new(&config);
        
        let resolver = SessionResolver::new(&config, &git_service, &session_manager);
        assert_eq!(resolver.config.ide.name, "test");
    }
}