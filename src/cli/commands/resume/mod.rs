use crate::cli::parser::ResumeArgs;
use crate::config::Config;
use crate::core::git::{GitOperations, GitService};
use crate::core::session::SessionManager;
use crate::utils::Result;

mod execution;
mod session_detection;
mod task_transformation;
mod validation;

use execution::launch_ide_for_session;
use session_detection::{detect_and_resume_session, resume_specific_session};
use validation::validate_resume_args;

/// Main entry point for the resume command
pub fn execute(config: Config, args: ResumeArgs) -> Result<()> {
    validate_resume_args(&args)?;

    let git_service = GitService::discover()?;
    let session_manager = SessionManager::new(&config);

    match args.session {
        Some(session_name) => resume_specific_session(&config, &git_service, &session_name),
        None => detect_and_resume_session(&config, &git_service, &session_manager),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{
        Config, DirectoryConfig, GitConfig, IdeConfig, SessionConfig, WrapperConfig,
    };
    use crate::core::git::{GitService, SessionEnvironment};
    use crate::core::session::SessionManager;
    use crate::utils::ParaError;
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

    // Integration tests for session detection and workflow

    #[test]
    fn test_session_detection_from_directory() {
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
            branch_name.clone(),
            worktree_path.clone(),
        );
        session_manager.save_state(&state).unwrap();

        // Test session detection from worktree directory
        std::env::set_current_dir(&worktree_path).unwrap();
        let environment = git_service
            .validate_session_environment(&worktree_path)
            .unwrap();

        match environment {
            SessionEnvironment::Worktree { branch, .. } => {
                assert_eq!(branch, branch_name);
            }
            _ => panic!("Expected Worktree environment"),
        }
    }

    #[test]
    fn test_resume_specific_session_workflow() {
        let (_git_tmp, _state_tmp, git_service, config) = setup_test_repo();
        let session_manager = SessionManager::new(&config);

        // Create a session with existing worktree
        let session_name = "test_workflow_session".to_string();
        let branch_name = "para/workflow-branch".to_string();
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

        // Test resuming the specific session
        let args = ResumeArgs {
            session: Some(session_name.clone()),
        };
        let result = execute(config, args);
        assert!(result.is_ok());

        // Verify CLAUDE.local.md was created
        let claude_local_path = worktree_path.join("CLAUDE.local.md");
        assert!(claude_local_path.exists());
    }

    #[test]
    fn test_detect_and_resume_session_workflow() {
        let (_git_tmp, _state_tmp, git_service, config) = setup_test_repo();
        let session_manager = SessionManager::new(&config);

        // Create a session and set current directory to worktree
        let session_name = "detect_session".to_string();
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

        // Set current directory to the worktree and test detection
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(&worktree_path).unwrap();

        let args = ResumeArgs { session: None };
        let result = execute(config, args);

        // Restore original directory
        std::env::set_current_dir(original_dir).unwrap();

        assert!(result.is_ok());

        // Verify CLAUDE.local.md was created in the worktree
        let claude_local_path = worktree_path.join("CLAUDE.local.md");
        assert!(claude_local_path.exists());
    }

    #[test]
    fn test_resume_session_not_found_error() {
        let (_git_tmp, _state_tmp, _git_service, config) = setup_test_repo();

        // Try to resume a non-existent session
        let args = ResumeArgs {
            session: Some("nonexistent_session".to_string()),
        };
        let result = execute(config, args);
        assert!(result.is_err());

        match result.unwrap_err() {
            ParaError::SessionNotFound { .. } => {
                // Expected error type
            }
            _ => panic!("Expected SessionNotFound error"),
        }
    }

    #[test]
    fn test_resume_session_missing_worktree_path() {
        let (_git_tmp, _state_tmp, git_service, config) = setup_test_repo();
        let session_manager = SessionManager::new(&config);

        // Create a session state with a non-existent worktree path
        let session_name = "missing_worktree_session".to_string();
        let branch_name = "para/missing-worktree".to_string();
        let nonexistent_path = git_service.repository().root.join("nonexistent_worktree");

        let state = crate::core::session::state::SessionState::new(
            session_name.clone(),
            branch_name,
            nonexistent_path,
        );
        session_manager.save_state(&state).unwrap();

        // Try to resume - should attempt to find matching worktree but fail
        let args = ResumeArgs {
            session: Some(session_name),
        };
        let result = execute(config, args);
        assert!(result.is_err());
    }

    #[test]
    fn test_worktree_path_resolution_fallback() {
        let (_git_tmp, _state_tmp, git_service, config) = setup_test_repo();
        let session_manager = SessionManager::new(&config);

        // Create worktree first
        let session_name = "path_resolution_test".to_string();
        let branch_name = "para/resolution-branch".to_string();
        let worktree_path = git_service
            .repository()
            .root
            .join(&config.directories.subtrees_dir)
            .join(&config.git.branch_prefix)
            .join(&session_name);

        git_service
            .create_worktree(&branch_name, &worktree_path)
            .unwrap();

        // Create session state with wrong path but correct branch
        let wrong_path = git_service.repository().root.join("wrong_path");
        let state = crate::core::session::state::SessionState::new(
            session_name.clone(),
            branch_name.clone(),
            wrong_path,
        );
        session_manager.save_state(&state).unwrap();

        // Resume should find the correct worktree by branch name and update the path
        let args = ResumeArgs {
            session: Some(session_name.clone()),
        };
        let result = execute(config, args);
        assert!(result.is_ok());

        // Verify the session state was updated with correct path
        let updated_state = session_manager.load_state(&session_name).unwrap();
        assert_eq!(updated_state.worktree_path, worktree_path);
    }

    #[test]
    fn test_validate_resume_args() {
        // Test valid args
        let args = ResumeArgs {
            session: Some("valid_session".to_string()),
        };
        let args_copy = ResumeArgs {
            session: args.session.clone(),
        };
        let (_git_tmp, _state_tmp, _git_service, config) = setup_test_repo();

        // This will call validate_resume_args internally
        let result = execute(config, args_copy);
        // Should not fail due to validation (will fail due to missing session)
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ParaError::SessionNotFound { .. }));

        // Test invalid args - empty session name
        let args = ResumeArgs {
            session: Some("".to_string()),
        };
        let (_git_tmp, _state_tmp, _git_service, config) = setup_test_repo();
        let result = execute(config, args);
        assert!(result.is_err());
        // Should fail due to validation
        assert!(!matches!(result.unwrap_err(), ParaError::SessionNotFound { .. }));
    }
}