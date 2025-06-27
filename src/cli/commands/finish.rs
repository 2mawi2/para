use crate::cli::parser::FinishArgs;
use crate::config::Config;
use crate::core::git::{
    FinishRequest, FinishResult, GitOperations, GitRepository, GitService, SessionEnvironment,
};
use crate::core::session::{SessionManager, SessionState, SessionStatus};
use crate::core::status::{Status, TestStatus};
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
    // First, update the status file to show 100% completion
    if let Some(ref session_state) = session_info {
        update_final_status(session_state, config)?;
    } else if let Ok(sessions) = session_manager.list_sessions() {
        // Fallback: find session by branch name
        for session in &sessions {
            if session.branch == feature_branch {
                update_final_status(session, config)?;
                break;
            }
        }
    }

    // Then update session status to Review
    if let Some(session_state) = session_info {
        session_manager.update_session_status(&session_state.name, SessionStatus::Review)?;
    } else if let Ok(sessions) = session_manager.list_sessions() {
        for session in sessions {
            if session.branch == feature_branch {
                let _ = session_manager.update_session_status(&session.name, SessionStatus::Review);
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

    // Destroy container if this is a container session
    if let Some(ref session) = ctx.session_info {
        if session.is_container() {
            // Use CLI-only approach - default parameters for container cleanup
            let docker_manager = crate::core::docker::DockerManager::new(
                ctx.config.clone(),
                false,  // network_isolation
                vec![], // allowed_domains
            );
            if let Err(e) = docker_manager.destroy_session_container(session) {
                eprintln!("Warning: Failed to destroy container: {}", e);
            }
        }
    }

    cleanup_session_state(
        ctx.session_manager,
        ctx.session_info.clone(),
        ctx.feature_branch,
        ctx.config,
    )?;

    if let Some(ref path) = worktree_path {
        if path != &ctx.git_service.repository().root && !ctx.config.should_preserve_on_finish() {
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

fn perform_pre_finish_operations(
    session_info: &Option<SessionState>,
    feature_branch: &str,
    config: &Config,
    git_service: &GitService,
) -> Result<()> {
    println!("Finishing session: {}", feature_branch);
    let session_id = session_info
        .as_ref()
        .map(|s| s.name.clone())
        .unwrap_or_else(|| feature_branch.to_string());

    // Check if this is a container session
    let is_container_session = session_info
        .as_ref()
        .map(|s| s.is_container())
        .unwrap_or(false);

    if !is_container_session && config.is_real_ide_environment() {
        let platform = get_platform_manager();

        let ide_to_close = if config.ide.name == "claude" && config.is_wrapper_enabled() {
            &config.ide.wrapper.name
        } else {
            &config.ide.name
        };

        if let Err(e) = platform.close_ide_window(&session_id, ide_to_close) {
            eprintln!("Warning: Failed to close IDE window: {}", e);
        }
    }

    if !is_container_session && config.should_auto_stage() {
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

fn handle_container_finish(
    session_info: &SessionState,
    _args: &FinishArgs,
    _config: &Config,
) -> Result<FinishResult> {
    // Container finish is now handled by the signal file watcher
    // This function is called when `para finish` is run from the host for a container session

    println!("Container session finish");

    // Check if there's already a finish signal file (shouldn't be if called from host)
    let signal_paths =
        crate::core::docker::signal_files::SignalFilePaths::new(&session_info.worktree_path);
    if signal_paths.finish.exists() {
        return Err(ParaError::invalid_args(
            "A finish signal is already pending. The container agent should use 'para finish' inside the container.",
        ));
    }

    // For host-initiated finish, we proceed with normal git operations
    // The watcher would have already handled container-initiated finishes
    println!(
        "⚠️  Note: For container sessions, agents should use 'para finish' inside the container."
    );
    println!("   This will create a signal file that the host processes automatically.");

    // Since the user is finishing from the host, we return an error to guide them
    Err(ParaError::invalid_args(
        "Container sessions should be finished from inside the container using 'para finish'.\n\
         The container agent should run: para finish \"commit message\"",
    ))
}

pub fn execute(config: Config, args: FinishArgs) -> Result<()> {
    let (git_service, current_dir, session_env) = initialize_finish_environment(&args)?;
    let mut session_manager = SessionManager::new(&config);

    let (session_info, is_worktree_env) =
        resolve_session_info(&args, &session_env, &mut session_manager, &current_dir)?;

    let feature_branch = determine_feature_branch(&session_info, &session_env)?;

    // Check if this is a container session
    let is_container_session = session_info
        .as_ref()
        .map(|s| s.is_container())
        .unwrap_or(false);

    // Debug logging to understand the issue
    if let Some(ref session) = session_info {
        crate::utils::debug_log(&format!(
            "Session '{}' type: {:?}, is_container: {}",
            session.name, session.session_type, is_container_session
        ));
    }

    let result = if is_container_session {
        // Handle container finish differently
        if let Some(ref session) = session_info {
            handle_container_finish(session, &args, &config)?
        } else {
            return Err(ParaError::invalid_args("Container session info not found"));
        }
    } else {
        // Traditional worktree finish
        perform_pre_finish_operations(&session_info, &feature_branch, &config, &git_service)?;

        let finish_request = FinishRequest {
            feature_branch: feature_branch.clone(),
            commit_message: args.message.clone(),
            target_branch_name: args.branch.clone(),
        };

        git_service.finish_session(finish_request)?
    };

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

fn update_final_status(session_state: &SessionState, config: &Config) -> Result<()> {
    let state_dir = if std::path::Path::new(&config.directories.state_dir).is_absolute() {
        std::path::PathBuf::from(&config.directories.state_dir)
    } else {
        // Get the main repository root for state directory
        if let Ok(root) = crate::utils::get_main_repository_root() {
            root.join(&config.directories.state_dir)
        } else {
            // Fallback to current directory
            std::env::current_dir()?.join(&config.directories.state_dir)
        }
    };

    // Load existing status or create new one
    let status = match Status::load(&state_dir, &session_state.name)
        .map_err(|e| ParaError::config_error(format!("Failed to load status: {}", e)))?
    {
        Some(mut existing_status) => {
            // Update existing status to show 100% completion
            existing_status.current_task = "Review".to_string();

            // Only update test status to Passed if it was Unknown, otherwise preserve current status
            if existing_status.test_status == TestStatus::Unknown {
                existing_status.test_status = TestStatus::Passed;
            }

            existing_status.is_blocked = false;
            existing_status.blocked_reason = None;

            // Set todos to 100% if they exist
            if let Some(total) = existing_status.todos_total {
                existing_status.todos_completed = Some(total);
            }

            existing_status
        }
        None => {
            // Create a new status with 100% completion
            Status::new(
                session_state.name.clone(),
                "Review".to_string(),
                TestStatus::Passed,
            )
        }
    };

    // Save the updated status
    status
        .save(&state_dir)
        .map_err(|e| ParaError::config_error(format!("Failed to save status: {}", e)))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::session::SessionState;
    use crate::test_utils::test_helpers::*;
    use tempfile::TempDir;

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

        let short_flag_valid_args = FinishArgs {
            message: "Test message".to_string(),
            branch: Some("custom-branch-name".to_string()),
            session: None,
        };
        assert!(short_flag_valid_args.validate().is_ok());
    }

    #[test]
    fn test_session_environment_validation() {
        let temp_dir = TempDir::new().unwrap();
        let git_temp = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, git_service) = setup_test_repo();

        let main_env = git_service
            .validate_session_environment(&git_service.repository().root)
            .expect("Failed to validate main repo");
        match main_env {
            SessionEnvironment::MainRepository => {}
            _ => panic!("Expected MainRepository environment, got: {:?}", main_env),
        }

        let worktree_path = git_temp.path().join("test-worktree");
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
        let temp_dir = TempDir::new().unwrap();
        let git_temp = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, git_service) = setup_test_repo();

        let config = create_test_config_with_dir(&temp_dir);
        let session_manager = SessionManager::new(&config);
        let repo_path = git_service.repository().root.clone();

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
        let temp_dir = TempDir::new().unwrap();
        let git_temp = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, git_service) = setup_test_repo();

        // Create config using the git repo path
        let config = create_test_config_with_dir(&temp_dir);
        let mut session_manager = SessionManager::new(&config);
        let repo_path = git_service.repository().root.clone();

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
        // Session should be in Review status (always preserved now)
        assert!(session_manager.session_exists("fallback-test-session"));
        let updated_session = session_manager
            .load_state("fallback-test-session")
            .expect("Session should exist");
        assert!(matches!(updated_session.status, SessionStatus::Review));

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
        // Session should be in Review status via fallback lookup
        assert!(session_manager.session_exists("fallback-test-session"));
        let updated_session = session_manager
            .load_state("fallback-test-session")
            .expect("Session should exist");
        assert!(matches!(updated_session.status, SessionStatus::Review));

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
        let temp_dir = TempDir::new().unwrap();
        let git_temp = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, git_service) = setup_test_repo();

        let mut config = create_test_config_with_dir(&temp_dir);
        config.session.preserve_on_finish = true; // Enable preserve mode for this test
        let mut session_manager = SessionManager::new(&config);
        let repo_path = git_service.repository().root.clone();

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

        // Session should still exist but be marked as ready for review
        assert!(session_manager.session_exists("preserve-test-session"));
        let updated_session = session_manager
            .load_state("preserve-test-session")
            .expect("Session should still exist");
        assert!(matches!(updated_session.status, SessionStatus::Review));
    }

    #[test]
    fn test_session_lifecycle_transition_to_review() {
        // Test that finish command transitions sessions to Review status in preserve mode
        let temp_dir = TempDir::new().unwrap();
        let git_temp = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, git_service) = setup_test_repo();

        let mut config = create_test_config_with_dir(&temp_dir);
        config.session.preserve_on_finish = true;
        let mut session_manager = SessionManager::new(&config);
        let repo_path = git_service.repository().root.clone();

        let session_state = SessionState::new(
            "lifecycle-test-session".to_string(),
            "test/lifecycle-branch".to_string(),
            repo_path.join("test-worktree"),
        );

        // Session should start as Active
        assert!(matches!(session_state.status, SessionStatus::Active));

        session_manager
            .save_state(&session_state)
            .expect("Failed to save session state");

        // Call cleanup_session_state with preserve mode
        let result = cleanup_session_state(
            &mut session_manager,
            Some(session_state.clone()),
            "test/lifecycle-branch",
            &config,
        );
        assert!(result.is_ok());

        // Session should exist and be in Review status
        assert!(session_manager.session_exists("lifecycle-test-session"));
        let updated_session = session_manager
            .load_state("lifecycle-test-session")
            .expect("Session should exist after finish");

        // Should transition to Review status (not Finished)
        assert!(matches!(updated_session.status, SessionStatus::Review));

        // Should preserve session metadata for review
        assert_eq!(updated_session.name, "lifecycle-test-session");
        assert_eq!(updated_session.branch, "test/lifecycle-branch");
    }

    #[test]
    fn test_session_lifecycle_worktree_cleanup_in_review() {
        // Test that when transitioning to Review status, worktree should be cleaned up
        // but session state should be preserved
        let temp_dir = TempDir::new().unwrap();
        let git_temp = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, git_service) = setup_test_repo();

        let mut config = create_test_config_with_dir(&temp_dir);
        config.session.preserve_on_finish = true;
        let mut session_manager = SessionManager::new(&config);
        let repo_path = git_service.repository().root.clone();

        let worktree_path = repo_path.join("test-worktree");

        let session_state = SessionState::new(
            "worktree-cleanup-test".to_string(),
            "test/worktree-cleanup-branch".to_string(),
            worktree_path.clone(),
        );

        session_manager
            .save_state(&session_state)
            .expect("Failed to save session state");

        // After cleanup_session_state, session should be in Review status
        let result = cleanup_session_state(
            &mut session_manager,
            Some(session_state),
            "test/worktree-cleanup-branch",
            &config,
        );
        assert!(result.is_ok());

        // Session should be preserved in Review status
        let updated_session = session_manager
            .load_state("worktree-cleanup-test")
            .expect("Session should exist");
        assert!(matches!(updated_session.status, SessionStatus::Review));

        // Session should still have worktree path info (for potential cleanup operations)
        assert_eq!(updated_session.worktree_path, worktree_path);
    }

    #[test]
    fn test_session_lifecycle_always_review_status() {
        // Test that with preserve_on_finish = false, session is deleted entirely
        let temp_dir = TempDir::new().unwrap();
        let git_temp = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, git_service) = setup_test_repo();

        let mut config = create_test_config_with_dir(&temp_dir);
        config.session.preserve_on_finish = false; // No preservation
        let mut session_manager = SessionManager::new(&config);
        let repo_path = git_service.repository().root.clone();

        let session_state = SessionState::new(
            "no-preserve-test".to_string(),
            "test/no-preserve-branch".to_string(),
            repo_path.join("test-worktree"),
        );

        session_manager
            .save_state(&session_state)
            .expect("Failed to save session state");

        // Call cleanup with no preservation
        let result = cleanup_session_state(
            &mut session_manager,
            Some(session_state),
            "test/no-preserve-branch",
            &config,
        );
        assert!(result.is_ok());

        // Session should be in Review status (always preserved now)
        assert!(session_manager.session_exists("no-preserve-test"));
        let updated_session = session_manager
            .load_state("no-preserve-test")
            .expect("Session should exist");
        assert!(matches!(updated_session.status, SessionStatus::Review));
    }

    #[test]
    fn test_finish_updates_status_to_100_percent() {
        // Test that finish command updates status to 100% completion
        let temp_dir = TempDir::new().unwrap();
        let git_temp = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, git_service) = setup_test_repo();

        let config = create_test_config_with_dir(&temp_dir);
        let mut session_manager = SessionManager::new(&config);
        let repo_path = git_service.repository().root.clone();
        let state_dir = std::path::Path::new(&config.directories.state_dir);

        let session_state = SessionState::new(
            "status-test-session".to_string(),
            "test/status-branch".to_string(),
            repo_path.join("test-worktree"),
        );

        session_manager
            .save_state(&session_state)
            .expect("Failed to save session state");

        // Create an initial status with incomplete todos
        let initial_status = Status::new(
            "status-test-session".to_string(),
            "Working on feature".to_string(),
            crate::core::status::TestStatus::Failed,
        )
        .with_todos(3, 5);
        initial_status
            .save(state_dir)
            .expect("Failed to save initial status");

        // Call cleanup_session_state (which is called during finish)
        let result = cleanup_session_state(
            &mut session_manager,
            Some(session_state),
            "test/status-branch",
            &config,
        );
        assert!(result.is_ok());

        // Load the updated status
        let updated_status = Status::load(state_dir, "status-test-session")
            .expect("Failed to load status")
            .expect("Status should exist");

        // Verify status was updated to 100%
        assert_eq!(updated_status.current_task, "Review");
        // Test status should remain Failed (not be overridden to Passed)
        assert_eq!(
            updated_status.test_status,
            crate::core::status::TestStatus::Failed
        );
        assert!(!updated_status.is_blocked);
        assert_eq!(updated_status.todos_completed, Some(5)); // Should be equal to total
        assert_eq!(updated_status.todos_total, Some(5));
    }

    #[test]
    fn test_finish_creates_status_if_missing() {
        // Test that finish creates a status file if it doesn't exist
        let temp_dir = TempDir::new().unwrap();
        let git_temp = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, git_service) = setup_test_repo();

        let config = create_test_config_with_dir(&temp_dir);
        let mut session_manager = SessionManager::new(&config);
        let repo_path = git_service.repository().root.clone();
        let state_dir = std::path::Path::new(&config.directories.state_dir);

        let session_state = SessionState::new(
            "new-status-session".to_string(),
            "test/new-status-branch".to_string(),
            repo_path.join("test-worktree"),
        );

        session_manager
            .save_state(&session_state)
            .expect("Failed to save session state");

        // Verify no status exists initially
        assert!(Status::load(state_dir, "new-status-session")
            .expect("Failed to check status")
            .is_none());

        // Call cleanup_session_state
        let result = cleanup_session_state(
            &mut session_manager,
            Some(session_state),
            "test/new-status-branch",
            &config,
        );
        assert!(result.is_ok());

        // Verify status was created
        let created_status = Status::load(state_dir, "new-status-session")
            .expect("Failed to load status")
            .expect("Status should have been created");

        assert_eq!(created_status.current_task, "Review");
        assert_eq!(
            created_status.test_status,
            crate::core::status::TestStatus::Passed
        );
        assert!(!created_status.is_blocked);
        assert_eq!(created_status.todos_completed, None); // No todos when created fresh
        assert_eq!(created_status.todos_total, None);
    }

    #[test]
    fn test_finish_preserves_failed_test_status() {
        // Test that finish preserves Failed test status but updates Unknown to Passed
        let temp_dir = TempDir::new().unwrap();
        let git_temp = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, git_service) = setup_test_repo();

        let config = create_test_config_with_dir(&temp_dir);
        let mut session_manager = SessionManager::new(&config);
        let repo_path = git_service.repository().root.clone();
        let state_dir = std::path::Path::new(&config.directories.state_dir);

        // Test 1: Failed status should be preserved
        let session_state_failed = SessionState::new(
            "failed-status-session".to_string(),
            "test/failed-branch".to_string(),
            repo_path.join("test-worktree-failed"),
        );
        session_manager
            .save_state(&session_state_failed)
            .expect("Failed to save session state");

        let failed_status = Status::new(
            "failed-status-session".to_string(),
            "Working on feature".to_string(),
            crate::core::status::TestStatus::Failed,
        );
        failed_status
            .save(state_dir)
            .expect("Failed to save failed status");

        cleanup_session_state(
            &mut session_manager,
            Some(session_state_failed),
            "test/failed-branch",
            &config,
        )
        .expect("Failed to cleanup session state");

        let updated_failed_status = Status::load(state_dir, "failed-status-session")
            .expect("Failed to load status")
            .expect("Status should exist");

        assert_eq!(updated_failed_status.current_task, "Review");
        assert_eq!(
            updated_failed_status.test_status,
            crate::core::status::TestStatus::Failed
        );

        // Test 2: Unknown status should be updated to Passed
        let session_state_unknown = SessionState::new(
            "unknown-status-session".to_string(),
            "test/unknown-branch".to_string(),
            repo_path.join("test-worktree-unknown"),
        );
        session_manager
            .save_state(&session_state_unknown)
            .expect("Failed to save session state");

        let unknown_status = Status::new(
            "unknown-status-session".to_string(),
            "Working on feature".to_string(),
            crate::core::status::TestStatus::Unknown,
        );
        unknown_status
            .save(state_dir)
            .expect("Failed to save unknown status");

        cleanup_session_state(
            &mut session_manager,
            Some(session_state_unknown),
            "test/unknown-branch",
            &config,
        )
        .expect("Failed to cleanup session state");

        let updated_unknown_status = Status::load(state_dir, "unknown-status-session")
            .expect("Failed to load status")
            .expect("Status should exist");

        assert_eq!(updated_unknown_status.current_task, "Review");
        assert_eq!(
            updated_unknown_status.test_status,
            crate::core::status::TestStatus::Passed
        );
    }
}
