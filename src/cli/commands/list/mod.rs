use crate::cli::parser::ListArgs;
use crate::config::Config;
use crate::core::git::GitService;
use crate::core::session::SessionManager;
use crate::utils::Result;

pub mod analyzer;
pub mod formatters;
pub mod test_utils;

pub use analyzer::*;
pub use formatters::*;

pub fn execute(config: Config, args: ListArgs) -> Result<()> {
    let session_manager = SessionManager::new(&config);

    let git_service = GitService::discover()?;
    let sessions = if args.archived {
        list_archived_sessions(&session_manager, &git_service)?
    } else {
        list_active_sessions(&session_manager, &git_service)?
    };

    if sessions.is_empty() {
        if !args.quiet {
            if args.archived {
                println!("No archived sessions found.");
            } else {
                println!("No active sessions found.");
            }
        }
        return Ok(());
    }

    display_sessions(&sessions, &args)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::parser::ListArgs;
    use crate::test_utils::test_helpers::*;
    use tempfile::TempDir;

    #[test]
    fn test_execute_no_sessions() -> Result<()> {
        let git_temp = TempDir::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, git_service) = setup_test_repo();

        // Create config that points to our test state directory
        let config = create_test_config_with_dir(&temp_dir);
        let session_manager = SessionManager::new(&config);

        // Test the internal functions directly with proper git context
        let sessions = list_active_sessions(&session_manager, &git_service)?;
        assert!(sessions.is_empty());

        // Test that empty sessions are handled correctly
        let args = ListArgs {
            verbose: false,
            archived: false,
            quiet: false,
        };

        let result = display_sessions(&sessions, &args);
        assert!(result.is_ok());

        Ok(())
    }

    #[test]
    fn test_execute_not_in_git_repo() {
        use crate::core::git::GitService;
        // Test that GitService::discover fails when not in a git repo
        // This test validates the error handling without changing directories
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        // We can't easily test the execute function without changing directories
        // So we'll test that GitService::discover_from fails for non-git directories
        let result = GitService::discover_from(temp_dir.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_list_active_sessions_with_new_test_utils() -> Result<()> {
        let git_temp = TempDir::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, git_service) = setup_test_repo();

        // Create config that points to our test state directory
        let config = create_test_config_with_dir(&temp_dir);
        let session_manager = SessionManager::new(&config);
        let state_dir = std::path::PathBuf::from(&config.directories.state_dir);

        // Create a simple directory for the worktree path - we just need to test listing
        let worktree_path = temp_dir.path().join("test-worktree");
        std::fs::create_dir_all(&worktree_path)?;

        // Use the new improved test utilities
        let params = test_utils::test_helpers::SessionParams::new(
            "test-session",
            "para/test-branch",
            &worktree_path,
        );

        test_utils::test_helpers::create_test_session_state(&state_dir, params)?;

        let sessions = list_active_sessions(&session_manager, &git_service)?;
        assert_eq!(sessions.len(), 1);

        let session = &sessions[0];
        assert_eq!(session.session_id, "test-session");
        assert_eq!(session.branch, "para/test-branch");
        assert_eq!(session.base_branch, "main");
        assert_eq!(session.merge_mode, "squash");
        assert_eq!(session.status, SessionStatus::Missing); // Directory exists but not a proper worktree

        Ok(())
    }
}