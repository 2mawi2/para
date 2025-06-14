use crate::cli::parser::FinishArgs;
use crate::config::Config;
use crate::core::git::{
    FinishRequest, FinishResult, GitOperations, GitRepository, GitService, SessionEnvironment,
};
use crate::core::session::{SessionManager, SessionState, SessionStatus};
use crate::platform::get_platform_manager;
use crate::utils::{ParaError, Result};
use std::env;

struct FinishContext<'a> {
    session_info: Option<SessionState>,
    is_worktree_env: bool,
    current_dir: &'a std::path::Path,
    feature_branch: &'a str,
    session_manager: &'a mut SessionManager,
    git_service: &'a GitService,
    config: &'a Config,
    args: &'a FinishArgs,
}

fn cleanup_session_state(
    session_manager: &mut SessionManager,
    session_info: Option<SessionState>,
    feature_branch: &str,
    config: &Config,
) -> Result<()> {
    if let Some(session_state) = session_info {
        if config.should_preserve_on_finish() {
            session_manager.update_session_status(&session_state.name, SessionStatus::Finished)?;
        } else {
            session_manager.delete_state(&session_state.name)?;
        }
    } else if let Ok(sessions) = session_manager.list_sessions() {
        for session in sessions {
            if session.branch == feature_branch {
                if config.should_preserve_on_finish() {
                    let _ = session_manager
                        .update_session_status(&session.name, SessionStatus::Finished);
                } else {
                    let _ = session_manager.delete_state(&session.name);
                }
                break;
            }
        }
    }
    Ok(())
}

fn handle_finish_success(final_branch: String, ctx: &mut FinishContext) -> Result<()> {
    let worktree_path = if ctx.is_worktree_env {
        Some(ctx.current_dir.to_path_buf())
    } else {
        ctx.session_info.as_ref().map(|s| s.worktree_path.clone())
    };

    cleanup_session_state(
        ctx.session_manager,
        ctx.session_info.clone(),
        ctx.feature_branch,
        ctx.config,
    )?;

    if let Some(ref path) = worktree_path {
        if path != &ctx.git_service.repository().root && !ctx.config.should_preserve_on_finish() {
            // Safety check: ensure no uncommitted changes in worktree before removing
            if let Ok(worktree_repo) = GitRepository::discover_from(path) {
                if worktree_repo.has_uncommitted_changes().unwrap_or(false) {
                    eprintln!(
                        "Warning: Preserving worktree at {} due to uncommitted changes",
                        path.display()
                    );
                    return Ok(());
                }
            }

            if let Err(e) = ctx.git_service.remove_worktree(path) {
                eprintln!(
                    "Warning: Failed to remove worktree at {}: {}",
                    path.display(),
                    e
                );
            }
        }
    }

    println!("✓ Session finished successfully");
    println!("  Feature branch: {}", final_branch);
    println!("  Commit message: {}", ctx.args.message);

    Ok(())
}

/// Initialize and validate the environment for finish operation
fn initialize_finish_environment(
    args: &FinishArgs,
) -> Result<(GitService, std::path::PathBuf, SessionEnvironment)> {
    args.validate()?;

    let git_service = GitService::discover()
        .map_err(|e| ParaError::git_error(format!("Failed to discover git repository: {}", e)))?;

    let current_dir = env::current_dir()
        .map_err(|e| ParaError::fs_error(format!("Failed to get current directory: {}", e)))?;

    let session_env = git_service.validate_session_environment(&current_dir)?;

    Ok((git_service, current_dir, session_env))
}

/// Resolve session information from args and environment
fn resolve_session_info(
    args: &FinishArgs,
    session_env: &SessionEnvironment,
    session_manager: &mut SessionManager,
    current_dir: &std::path::Path,
) -> Result<(Option<SessionState>, bool)> {
    let (session_info, is_worktree_env) = match &args.session {
        Some(session_id) => {
            let session_state = session_manager.load_state(session_id)?;
            (Some(session_state), false)
        }
        None => match session_env {
            SessionEnvironment::Worktree { branch: _, .. } => {
                // Try to auto-detect session by current path
                if let Ok(Some(session)) = session_manager.find_session_by_path(current_dir) {
                    (Some(session), true)
                } else {
                    (None, true)
                }
            }
            SessionEnvironment::MainRepository => {
                return Err(ParaError::invalid_args(
                    "Cannot finish from main repository. Use --session to specify a session or run from within a session worktree.",
                ));
            }
            SessionEnvironment::Invalid => {
                return Err(ParaError::invalid_args(
                    "Cannot finish from this location. Use --session to specify a session or run from within a session worktree.",
                ));
            }
        },
    };

    Ok((session_info, is_worktree_env))
}

/// Determine the feature branch name from session info or environment
fn determine_feature_branch(
    session_info: &Option<SessionState>,
    session_env: &SessionEnvironment,
) -> Result<String> {
    if let Some(session) = session_info {
        return Ok(session.branch.clone());
    }

    match session_env {
        SessionEnvironment::Worktree { branch, .. } => Ok(branch.clone()),
        _ => Err(ParaError::invalid_args(
            "Unable to determine feature branch",
        )),
    }
}

/// Perform pre-finish operations (IDE closing, staging)
fn perform_pre_finish_operations(
    session_info: &Option<SessionState>,
    feature_branch: &str,
    config: &Config,
    git_service: &GitService,
) -> Result<()> {
    println!("Finishing session: {}", feature_branch);

    // Close IDE window before Git operations (in case Git operations fail)
    let session_id = session_info
        .as_ref()
        .map(|s| s.name.clone())
        .unwrap_or_else(|| feature_branch.to_string());

    if config.is_real_ide_environment() {
        let platform = get_platform_manager();

        // For Claude, we need to close the wrapper IDE, not Claude itself
        let ide_to_close = if config.ide.name == "claude" && config.is_wrapper_enabled() {
            &config.ide.wrapper.name
        } else {
            &config.ide.name
        };

        if let Err(e) = platform.close_ide_window(&session_id, ide_to_close) {
            eprintln!("Warning: Failed to close IDE window: {}", e);
        }
    }

    if config.should_auto_stage() {
        if let Err(e) = git_service.stage_all_changes() {
            eprintln!(
                "Warning: Auto-staging failed: {}. Please stage changes manually.",
                e
            );
            return Err(e);
        }
    }

    Ok(())
}

pub fn execute(config: Config, args: FinishArgs) -> Result<()> {
    // Initialize environment and validate
    let (git_service, current_dir, session_env) = initialize_finish_environment(&args)?;
    let mut session_manager = SessionManager::new(&config);

    // Resolve session information
    let (session_info, is_worktree_env) =
        resolve_session_info(&args, &session_env, &mut session_manager, &current_dir)?;

    // Determine feature and base branches
    let feature_branch = determine_feature_branch(&session_info, &session_env)?;

    // Perform pre-finish operations
    perform_pre_finish_operations(&session_info, &feature_branch, &config, &git_service)?;

    // Execute the finish operation
    let finish_request = FinishRequest {
        feature_branch: feature_branch.clone(),
        commit_message: args.message.clone(),
        target_branch_name: args.branch.clone(),
    };

    let result = git_service.finish_session(finish_request)?;

    // Handle the result
    let mut ctx = FinishContext {
        session_info,
        is_worktree_env,
        current_dir: &current_dir,
        feature_branch: &feature_branch,
        session_manager: &mut session_manager,
        git_service: &git_service,
        config: &config,
        args: &args,
    };

    match result {
        FinishResult::Success { final_branch } => {
            handle_finish_success(final_branch, &mut ctx)?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::core::session::SessionState;
    use std::fs;
    use std::path::{Path, PathBuf};
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

    fn setup_test_repo() -> (TempDir, PathBuf) {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let repo_path = temp_dir.path().to_path_buf();

        Command::new("git")
            .current_dir(&repo_path)
            .args(["init", "--initial-branch=main"])
            .status()
            .expect("Failed to init git repo");

        Command::new("git")
            .current_dir(&repo_path)
            .args(["config", "user.name", "Test User"])
            .status()
            .expect("Failed to set git user name");

        Command::new("git")
            .current_dir(&repo_path)
            .args(["config", "user.email", "test@example.com"])
            .status()
            .expect("Failed to set git user email");

        fs::write(repo_path.join("README.md"), "# Test Repository")
            .expect("Failed to write README");

        Command::new("git")
            .current_dir(&repo_path)
            .args(["add", "README.md"])
            .status()
            .expect("Failed to add README");

        Command::new("git")
            .current_dir(&repo_path)
            .args(["commit", "-m", "Initial commit"])
            .status()
            .expect("Failed to commit README");

        (temp_dir, repo_path)
    }

    #[test]
    fn test_finish_args_validation() {
        let valid_args = FinishArgs {
            message: "Test commit message".to_string(),
            branch: None,
            session: None,
        };
        assert!(valid_args.validate().is_ok());

        let empty_message_args = FinishArgs {
            message: "".to_string(),
            branch: None,
            session: None,
        };
        assert!(empty_message_args.validate().is_err());

        let whitespace_message_args = FinishArgs {
            message: "   ".to_string(),
            branch: None,
            session: None,
        };
        assert!(whitespace_message_args.validate().is_err());

        let invalid_branch_args = FinishArgs {
            message: "Test message".to_string(),
            branch: Some("-invalid-branch".to_string()),
            session: None,
        };
        assert!(invalid_branch_args.validate().is_err());
    }

    #[test]
    fn test_session_environment_validation() {
        let (temp_dir, repo_path) = setup_test_repo();

        let git_service = GitService::discover_from(&repo_path).expect("Failed to discover repo");

        let main_env = git_service
            .validate_session_environment(&git_service.repository().root)
            .expect("Failed to validate main repo");
        match main_env {
            SessionEnvironment::MainRepository => {}
            _ => panic!("Expected MainRepository environment, got: {:?}", main_env),
        }

        let worktree_path = temp_dir.path().join("test-worktree");
        git_service
            .create_worktree("test-branch", &worktree_path)
            .expect("Failed to create worktree");

        let worktree_env = git_service
            .validate_session_environment(&worktree_path)
            .expect("Failed to validate worktree");
        match worktree_env {
            SessionEnvironment::Worktree { branch, .. } => {
                assert_eq!(branch, "test-branch");
            }
            _ => panic!("Expected Worktree environment, got: {:?}", worktree_env),
        }
    }

    #[test]
    fn test_finish_integration_validation() {
        let (temp_dir, repo_path) = setup_test_repo();
        let config = create_test_config(temp_dir.path());
        let session_manager = SessionManager::new(&config);

        let session_state = SessionState::new(
            "test-session".to_string(),
            "test-branch".to_string(),
            repo_path.join("worktree"),
        );

        session_manager
            .save_state(&session_state)
            .expect("Failed to save state");

        let loaded_state = session_manager
            .load_state("test-session")
            .expect("Failed to load state");
        assert_eq!(loaded_state.name, "test-session");
        assert_eq!(loaded_state.branch, "test-branch");
    }

    #[test]
    fn test_cleanup_session_state_fallback() {
        // Use setup_test_repo to create a proper git repository
        let (_temp_dir, repo_path) = setup_test_repo();

        // Create config using the git repo path
        let config = create_test_config(&repo_path);
        let mut session_manager = SessionManager::new(&config);

        // Create a session state that would normally be found by path
        let session_state = SessionState::new(
            "fallback-test-session".to_string(),
            "test/fallback-branch".to_string(),
            repo_path.join("some-worktree"),
        );

        session_manager
            .save_state(&session_state)
            .expect("Failed to save session state");

        // Verify session exists before cleanup
        assert!(session_manager.session_exists("fallback-test-session"));

        // Test Case 1: Primary path with session_info present
        let result = cleanup_session_state(
            &mut session_manager,
            Some(session_state.clone()),
            "test/fallback-branch",
            &config,
        );
        assert!(result.is_ok());
        // Session should be deleted (preserve_on_finish = false in test config)
        assert!(!session_manager.session_exists("fallback-test-session"));

        // Test Case 2: Fallback path - session_info is None but session exists with matching branch
        // Re-create the session for fallback test
        session_manager
            .save_state(&session_state)
            .expect("Failed to save session state");
        assert!(session_manager.session_exists("fallback-test-session"));

        // Call cleanup with session_info = None (simulating session detection failure)
        let result = cleanup_session_state(
            &mut session_manager,
            None,                   // No session_info - triggers fallback
            "test/fallback-branch", // This should match the branch name
            &config,
        );
        assert!(result.is_ok());
        // Session should be deleted via fallback lookup
        assert!(!session_manager.session_exists("fallback-test-session"));

        // Test Case 3: Fallback with non-matching branch name
        session_manager
            .save_state(&session_state)
            .expect("Failed to save session state");
        assert!(session_manager.session_exists("fallback-test-session"));

        let result = cleanup_session_state(
            &mut session_manager,
            None,                    // No session_info - triggers fallback
            "different/branch-name", // This won't match
            &config,
        );
        assert!(result.is_ok());
        // Session should still exist because branch name didn't match
        assert!(session_manager.session_exists("fallback-test-session"));
    }

    #[test]
    fn test_cleanup_session_state_preserve_mode() {
        // Use setup_test_repo to create a proper git repository
        let (_temp_dir, repo_path) = setup_test_repo();

        let mut config = create_test_config(&repo_path);
        config.session.preserve_on_finish = true; // Enable preserve mode for this test
        let mut session_manager = SessionManager::new(&config);

        let session_state = SessionState::new(
            "preserve-test-session".to_string(),
            "test/preserve-branch".to_string(),
            repo_path.join("some-worktree"),
        );

        session_manager
            .save_state(&session_state)
            .expect("Failed to save session state");

        // Test fallback in preserve mode
        let result = cleanup_session_state(
            &mut session_manager,
            None, // Triggers fallback
            "test/preserve-branch",
            &config,
        );
        assert!(result.is_ok());

        // Session should still exist but be marked as finished
        assert!(session_manager.session_exists("preserve-test-session"));
        let updated_session = session_manager
            .load_state("preserve-test-session")
            .expect("Session should still exist");
        assert!(matches!(updated_session.status, SessionStatus::Finished));
    }
}
