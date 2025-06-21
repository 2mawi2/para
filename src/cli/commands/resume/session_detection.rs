// This file was moved to resume_modules/session_detection.rs
use crate::config::Config;
use crate::core::git::{GitService, SessionEnvironment};
use crate::core::session::{SessionManager, SessionStatus};
use crate::utils::{ParaError, Result};
use dialoguer::Select;
use std::env;

use super::execution::launch_ide_for_session;

/// Resumes a specific session by name
pub fn resume_specific_session(
    config: &Config,
    git_service: &GitService,
    session_name: &str,
) -> Result<()> {
    let session_manager = SessionManager::new(config);

    if session_manager.session_exists(session_name) {
        let mut session_state = session_manager.load_state(session_name)?;

        if !session_state.worktree_path.exists() {
            let branch_to_match = session_state.branch.clone();
            if let Some(wt) = git_service
                .list_worktrees()?
                .into_iter()
                .find(|w| w.branch == branch_to_match)
            {
                session_state.worktree_path = wt.path.clone();
                session_manager.save_state(&session_state)?;
            } else if let Some(wt) = git_service.list_worktrees()?.into_iter().find(|w| {
                w.path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with(session_name))
                    .unwrap_or(false)
            }) {
                session_state.worktree_path = wt.path.clone();
                session_manager.save_state(&session_state)?;
            } else {
                return Err(ParaError::session_not_found(format!(
                    "Session '{}' exists but worktree path '{}' not found",
                    session_name,
                    session_state.worktree_path.display()
                )));
            }
        }

        // Ensure CLAUDE.local.md exists for the session
        create_claude_local_md(&session_state.worktree_path, &session_state.name)?;

        launch_ide_for_session(config, &session_state.worktree_path)?;
        println!("✅ Resumed session '{}'", session_name);
    } else {
        // Fallback: maybe the state file was timestamped (e.g. test4_20250611-XYZ)
        if let Some(candidate) = session_manager
            .list_sessions()?
            .into_iter()
            .find(|s| s.name.starts_with(session_name))
        {
            // recurse with the full name
            return resume_specific_session(config, git_service, &candidate.name);
        }
        // original branch/path heuristic
        let worktrees = git_service.list_worktrees()?;
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
        if let Some(session_name) = session_manager
            .list_sessions()?
            .into_iter()
            .find(|s| {
                s.worktree_path == matching_worktree.path || s.branch == matching_worktree.branch
            })
            .map(|s| s.name)
        {
            create_claude_local_md(&matching_worktree.path, &session_name)?;
        } else {
            // Fallback: use session name from search
            create_claude_local_md(&matching_worktree.path, session_name)?;
        }

        launch_ide_for_session(config, &matching_worktree.path)?;
        println!(
            "✅ Resumed session at '{}'",
            matching_worktree.path.display()
        );
    }

    Ok(())
}

/// Detects current environment and resumes appropriate session
pub fn detect_and_resume_session(
    config: &Config,
    git_service: &GitService,
    session_manager: &SessionManager,
) -> Result<()> {
    let current_dir = env::current_dir()?;

    match git_service.validate_session_environment(&current_dir)? {
        SessionEnvironment::Worktree { branch, .. } => {
            println!("Current directory is a worktree for branch: {}", branch);

            // Try to find session name from current directory or branch
            if let Some(session_name) = session_manager
                .list_sessions()?
                .into_iter()
                .find(|s| s.worktree_path == current_dir || s.branch == branch)
                .map(|s| s.name)
            {
                create_claude_local_md(&current_dir, &session_name)?;
            }

            launch_ide_for_session(config, &current_dir)?;
            println!("✅ Resumed current session");
            Ok(())
        }
        SessionEnvironment::MainRepository => {
            println!("Current directory is the main repository");
            list_and_select_session(config, git_service, session_manager)
        }
        SessionEnvironment::Invalid => {
            println!("Current directory is not part of a para session");
            list_and_select_session(config, git_service, session_manager)
        }
    }
}

/// Lists active sessions and allows user to select one
fn list_and_select_session(
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

    println!("Active sessions:");
    for (i, session) in active_sessions.iter().enumerate() {
        println!("  {}: {} ({})", i + 1, session.name, session.branch);
    }

    let selection = Select::new()
        .with_prompt("Select session to resume")
        .items(&active_sessions.iter().map(|s| &s.name).collect::<Vec<_>>())
        .interact();

    if let Ok(index) = selection {
        let session = &active_sessions[index];

        if !session.worktree_path.exists() {
            return Err(ParaError::session_not_found(format!(
                "Session '{}' exists but worktree path '{}' not found",
                session.name,
                session.worktree_path.display()
            )));
        }

        // Ensure CLAUDE.local.md exists for the session
        create_claude_local_md(&session.worktree_path, &session.name)?;

        launch_ide_for_session(config, &session.worktree_path)?;
        println!("✅ Resumed session '{}'", session.name);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{
        Config, DirectoryConfig, GitConfig, IdeConfig, SessionConfig, WrapperConfig,
    };
    use std::fs;
    use std::process::Command;
    use tempfile::TempDir;

    fn setup_test_repo() -> (TempDir, TempDir, GitService, Config) {
        let git_dir = TempDir::new().expect("tmp git");
        let state_dir = TempDir::new().expect("tmp state");
        let repo_path = git_dir.path();
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
        fs::write(repo_path.join("README.md"), "# Test").unwrap();
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
                name: "echo".into(),
                command: "echo".into(),
                user_data_dir: None,
                wrapper: WrapperConfig {
                    enabled: true,
                    name: "cursor".into(),
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
    fn test_resume_specific_session_existing() {
        let (_git_tmp, _state_tmp, git_service, config) = setup_test_repo();
        let session_manager = SessionManager::new(&config);

        // Create a session and worktree
        let session_name = "test_session".to_string();
        let branch_name = "para/test-branch".to_string();
        let worktree_path = git_service
            .repository()
            .root
            .join(&config.directories.subtrees_dir)
            .join(&config.git.branch_prefix)
            .join(&session_name);

        git_service
            .create_worktree(&branch_name, &worktree_path)
            .unwrap();

        let state = crate::core::session::state::SessionState::new(
            session_name.clone(),
            branch_name,
            worktree_path.clone(),
        );
        session_manager.save_state(&state).unwrap();

        let result = resume_specific_session(&config, &git_service, &session_name);
        assert!(result.is_ok());

        // Verify CLAUDE.local.md was created
        let claude_local_path = worktree_path.join("CLAUDE.local.md");
        assert!(claude_local_path.exists());
    }

    #[test]
    fn test_resume_specific_session_nonexistent() {
        let (_git_tmp, _state_tmp, git_service, config) = setup_test_repo();

        let result = resume_specific_session(&config, &git_service, "nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_detect_and_resume_session_from_worktree() {
        let (_git_tmp, _state_tmp, git_service, config) = setup_test_repo();
        let session_manager = SessionManager::new(&config);

        // Create a session and worktree
        let session_name = "detect_test".to_string();
        let branch_name = "para/detect-branch".to_string();
        let worktree_path = git_service
            .repository()
            .root
            .join(&config.directories.subtrees_dir)
            .join(&config.git.branch_prefix)
            .join(&session_name);

        git_service
            .create_worktree(&branch_name, &worktree_path)
            .unwrap();

        let state = crate::core::session::state::SessionState::new(
            session_name.clone(),
            branch_name,
            worktree_path.clone(),
        );
        session_manager.save_state(&state).unwrap();

        // Change to worktree directory
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(&worktree_path).unwrap();

        let result = detect_and_resume_session(&config, &git_service, &session_manager);

        // Restore directory
        std::env::set_current_dir(original_dir).unwrap();

        assert!(result.is_ok());

        // Verify CLAUDE.local.md was created
        let claude_local_path = worktree_path.join("CLAUDE.local.md");
        assert!(claude_local_path.exists());
    }
}