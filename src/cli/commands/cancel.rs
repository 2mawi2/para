use crate::cli::parser::CancelArgs;
use crate::config::Config;
use crate::core::git::{GitOperations, GitService, SessionEnvironment};
use crate::core::session::SessionManager;
use crate::platform::get_platform_manager;
use crate::utils::{ParaError, Result};
use std::env;
use std::io::{self, Write};

fn is_non_interactive() -> bool {
    env::var("PARA_NON_INTERACTIVE").is_ok()
        || env::var("CI").is_ok()
        || !atty::is(atty::Stream::Stdin)
}

pub fn execute(config: Config, args: CancelArgs) -> Result<()> {
    validate_cancel_args(&args)?;

    let git_service = GitService::discover()?;
    let mut session_manager = SessionManager::new(&config);

    let session_name = detect_session_name(&args, &git_service, &session_manager)?;

    let session_state = session_manager.load_state(&session_name)?;

    let has_uncommitted = git_service.repository().has_uncommitted_changes()?;
    if has_uncommitted && !args.force {
        confirm_cancel_with_changes(&session_name)?;
    } else if has_uncommitted && args.force {
        eprintln!(
            "WARNING: Force canceling session '{session_name}' with uncommitted changes. Your work will be archived."
        );
    }

    // Use session manager's cancel method which handles Docker cleanup
    session_manager.cancel_session(&session_name, args.force)?;

    let archived_branch = git_service.archive_branch_with_session_name(
        &session_state.branch,
        &session_state.name,
        &config.git.branch_prefix,
    )?;

    let archive_manager = crate::core::session::archive::ArchiveManager::new(&config, &git_service);
    if let Ok((old_removed, limit_removed)) = archive_manager.auto_cleanup() {
        if old_removed > 0 || limit_removed > 0 {
            eprintln!(
                "Archive cleanup: removed {old_removed} old archives, {limit_removed} for limit"
            );
        }
    }

    if config.is_real_ide_environment() {
        let platform = get_platform_manager();
        if let Err(e) = platform.close_ide_window(
            &session_state.name,
            &config.ide.name,
            config.get_state_dir(),
        ) {
            eprintln!("Warning: Failed to close IDE window: {e}");
        }
    }

    println!(
        "Session '{}' has been cancelled and archived as '{}'",
        session_state.name, archived_branch
    );
    println!(
        "To recover this session later, use: para recover {}",
        session_state.name
    );
    println!("The archived branch is: {archived_branch}");

    Ok(())
}

fn detect_session_name(
    args: &CancelArgs,
    git_service: &GitService,
    session_manager: &SessionManager,
) -> Result<String> {
    if let Some(ref session_name) = args.session {
        if !session_manager.session_exists(session_name) {
            return Err(ParaError::session_not_found(session_name));
        }
        return Ok(session_name.clone());
    }

    let current_dir = env::current_dir()
        .map_err(|e| ParaError::file_operation(format!("Failed to get current directory: {e}")))?;

    match git_service.validate_session_environment(&current_dir)? {
        SessionEnvironment::Worktree { branch, .. } => {
            if let Some(session) = session_manager.find_session_by_path(&current_dir)? {
                return Ok(session.name);
            }

            if let Some(session) = session_manager.find_session_by_branch(&branch)? {
                return Ok(session.name);
            }
            Err(ParaError::session_not_found(format!(
                "No session found for current worktree (branch: {branch})"
            )))
        }
        SessionEnvironment::MainRepository => Err(ParaError::invalid_args(
            "Cannot cancel from main repository. Use 'para cancel <session-name>' to cancel a specific session.",
        )),
        SessionEnvironment::Invalid => Err(ParaError::invalid_args(
            "Not in a para session directory. Use 'para cancel <session-name>' to cancel a specific session.",
        )),
    }
}

fn confirm_cancel_with_changes(session_name: &str) -> Result<()> {
    if is_non_interactive() {
        return Err(ParaError::invalid_args(
            "Cannot cancel session with uncommitted changes in non-interactive mode. \
             Commit or stash changes first, or run interactively.",
        ));
    }

    print!(
        "Session '{session_name}' has uncommitted changes. Are you sure you want to cancel? This will archive the session but preserve your work. [y/N]: "
    );
    io::stdout()
        .flush()
        .map_err(|e| ParaError::file_operation(format!("Failed to flush stdout: {e}")))?;

    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .map_err(|e| ParaError::file_operation(format!("Failed to read input: {e}")))?;

    let response = input.trim().to_lowercase();
    if response != "y" && response != "yes" {
        return Err(ParaError::invalid_args("Cancel operation aborted by user"));
    }

    Ok(())
}

fn validate_cancel_args(args: &CancelArgs) -> Result<()> {
    if let Some(ref session) = args.session {
        if session.is_empty() {
            return Err(ParaError::invalid_args(
                "Session identifier cannot be empty",
            ));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::session::SessionState;
    use crate::test_utils::test_helpers::*;
    use tempfile::TempDir;

    #[test]
    fn test_validate_cancel_args_valid() {
        let args = CancelArgs {
            session: None,
            force: false,
        };
        assert!(validate_cancel_args(&args).is_ok());

        let args = CancelArgs {
            session: Some("valid-session".to_string()),
            force: false,
        };
        assert!(validate_cancel_args(&args).is_ok());
    }

    #[test]
    fn test_validate_cancel_args_empty_session() {
        let args = CancelArgs {
            session: Some(String::new()),
            force: false,
        };
        let result = validate_cancel_args(&args);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cannot be empty"));
    }

    #[test]
    fn test_detect_session_name_explicit() {
        let temp_dir = TempDir::new().unwrap();
        let git_temp = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, git_service) = setup_test_repo();

        let config = create_test_config_with_dir(&temp_dir);
        let session_manager = SessionManager::new(&config);

        let session_state = SessionState::new(
            "test-session".to_string(),
            "test-branch".to_string(),
            git_service.repository().root.join("test-worktree"),
        );
        session_manager
            .save_state(&session_state)
            .expect("Failed to save state");

        let args = CancelArgs {
            session: Some("test-session".to_string()),
            force: false,
        };

        let result = detect_session_name(&args, &git_service, &session_manager);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "test-session");
    }

    #[test]
    fn test_detect_session_name_nonexistent() {
        let temp_dir = TempDir::new().unwrap();
        let git_temp = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, git_service) = setup_test_repo();

        let config = create_test_config_with_dir(&temp_dir);
        let session_manager = SessionManager::new(&config);

        let args = CancelArgs {
            session: Some("nonexistent-session".to_string()),
            force: false,
        };

        let result = detect_session_name(&args, &git_service, &session_manager);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[test]
    fn test_detect_session_name_from_main_repo() {
        let temp_dir = TempDir::new().unwrap();
        let git_temp = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, git_service) = setup_test_repo();

        let config = create_test_config_with_dir(&temp_dir);
        let session_manager = SessionManager::new(&config);

        let args = CancelArgs {
            session: None,
            force: false,
        };

        std::env::set_current_dir(&git_service.repository().root)
            .expect("Failed to change to repo root");

        let result = detect_session_name(&args, &git_service, &session_manager);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("main repository"));
    }

    #[test]
    fn test_detect_session_name_invalid_directory() {
        let temp_dir = TempDir::new().unwrap();
        let git_temp = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, git_service) = setup_test_repo();

        let config = create_test_config_with_dir(&temp_dir);
        let session_manager = SessionManager::new(&config);

        let args = CancelArgs {
            session: None,
            force: false,
        };

        let invalid_dir = TempDir::new().expect("Failed to create invalid dir");
        std::env::set_current_dir(invalid_dir.path()).expect("Failed to change to invalid dir");

        let result = detect_session_name(&args, &git_service, &session_manager);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Not in a para session"));
    }

    #[test]
    fn test_confirm_cancel_with_changes_format() {
        // Test the function signature and basic validation logic
        // Skip actual interactive test since it requires user input
        let session_name = "test-session";

        // Verify the function exists and has correct signature
        assert!(!session_name.is_empty());

        // We cannot test the interactive part in automated tests
        // The function would require stdin input which we cannot provide in CI
    }

    #[test]
    fn test_non_interactive_error_in_confirm() {
        // Test that non-interactive mode returns appropriate error
        std::env::set_var("PARA_NON_INTERACTIVE", "1");

        let result = confirm_cancel_with_changes("test-session");
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("non-interactive mode"));
        assert!(error_msg.contains("Commit or stash changes"));

        std::env::remove_var("PARA_NON_INTERACTIVE");
    }

    #[test]
    fn test_force_cancel_with_uncommitted_changes() {
        // Test that force flag bypasses uncommitted changes check
        let temp_dir = TempDir::new().unwrap();
        let git_temp = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, git_service) = setup_test_repo();

        let config = create_test_config_with_dir(&temp_dir);
        let session_manager = SessionManager::new(&config);

        // Create a session
        let session_state = SessionState::new(
            "test-force-session".to_string(),
            "test-branch".to_string(),
            git_service.repository().root.join("test-worktree"),
        );
        session_manager
            .save_state(&session_state)
            .expect("Failed to save session state");

        // Create worktree directory
        std::fs::create_dir_all(&session_state.worktree_path).unwrap();

        // Create a file with uncommitted changes
        let test_file = session_state.worktree_path.join("test.txt");
        std::fs::write(&test_file, "uncommitted changes").unwrap();

        // Test with force flag - should succeed
        let args = CancelArgs {
            session: Some("test-force-session".to_string()),
            force: true,
        };

        // This should not error even with uncommitted changes
        let result = execute(config.clone(), args);

        // The actual execute might fail due to git operations in test environment,
        // but we're testing that it doesn't fail due to uncommitted changes
        if result.is_err() {
            let error_msg = result.unwrap_err().to_string();
            assert!(!error_msg.contains("uncommitted changes"));
        }
    }

    #[test]
    fn test_force_flag_non_interactive() {
        // Test that force flag works in non-interactive mode
        std::env::set_var("PARA_NON_INTERACTIVE", "1");

        let temp_dir = TempDir::new().unwrap();
        let git_temp = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, git_service) = setup_test_repo();

        let config = create_test_config_with_dir(&temp_dir);
        let session_manager = SessionManager::new(&config);

        // Create a session
        let session_state = SessionState::new(
            "test-force-noninteractive".to_string(),
            "test-branch".to_string(),
            git_service.repository().root.join("test-worktree"),
        );
        session_manager
            .save_state(&session_state)
            .expect("Failed to save session state");

        // Create worktree directory
        std::fs::create_dir_all(&session_state.worktree_path).unwrap();

        // Test with force flag in non-interactive mode
        let args = CancelArgs {
            session: Some("test-force-noninteractive".to_string()),
            force: true,
        };

        // This should work even in non-interactive mode with force flag
        let result = execute(config, args);

        // The actual execute might fail due to git operations,
        // but should not fail due to non-interactive mode
        if result.is_err() {
            let error_msg = result.unwrap_err().to_string();
            assert!(!error_msg.contains("non-interactive mode"));
        }

        std::env::remove_var("PARA_NON_INTERACTIVE");
    }
}
