use crate::cli::parser::{FinishArgs, IntegrationStrategy};
use crate::config::ConfigManager;
use crate::core::git::{
    FinishRequest, FinishResult, GitOperations, GitService, SessionEnvironment,
};
use crate::core::session::{SessionManager, SessionStatus};
use crate::platform::get_platform_manager;
use crate::utils::{ParaError, Result};
use std::env;

pub fn execute(args: FinishArgs) -> Result<()> {
    args.validate()?;

    let config = ConfigManager::load_or_create()
        .map_err(|e| ParaError::config_error(format!("Failed to load config: {}", e)))?;

    let git_service = GitService::discover()
        .map_err(|e| ParaError::git_error(format!("Failed to discover git repository: {}", e)))?;

    let current_dir = env::current_dir()
        .map_err(|e| ParaError::fs_error(format!("Failed to get current directory: {}", e)))?;

    let session_env = git_service.validate_session_environment(&current_dir)?;

    let mut session_manager = SessionManager::new(&config);

    let (session_info, current_branch, is_worktree_env) = match &args.session {
        Some(session_id) => {
            let session_state = session_manager.load_state(session_id)?;
            (Some(session_state), None, false)
        }
        None => match &session_env {
            SessionEnvironment::Worktree { branch, .. } => {
                // Try to auto-detect session by current path
                if let Ok(Some(session)) = session_manager.find_session_by_path(&current_dir) {
                    (Some(session), None, true)
                } else {
                    (None, Some(branch.clone()), true)
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

    let feature_branch = session_info
        .as_ref()
        .map(|s| s.branch.clone())
        .or(current_branch)
        .ok_or_else(|| ParaError::invalid_args("Unable to determine feature branch"))?;

    let base_branch = git_service
        .repository()
        .get_main_branch()
        .unwrap_or_else(|_| "main".to_string());

    let finish_request = FinishRequest {
        feature_branch: feature_branch.clone(),
        base_branch: base_branch.clone(),
        commit_message: args.message.clone(),
        target_branch_name: args.branch.clone(),
        integrate: args.integrate,
    };

    println!("Finishing session: {}", feature_branch);

    // Close IDE window before Git operations (in case Git operations fail)
    let session_id = session_info
        .as_ref()
        .map(|s| s.name.clone())
        .unwrap_or_else(|| feature_branch.clone());

    let platform = get_platform_manager();
    if let Err(e) = platform.close_ide_window(&session_id, &config.ide.name) {
        eprintln!("Warning: Failed to close IDE window: {}", e);
    }

    if config.should_auto_stage() {
        git_service.stage_all_changes()?;
    }

    let result = git_service.finish_session(finish_request)?;

    match result {
        FinishResult::Success { final_branch } => {
            let worktree_path = if is_worktree_env {
                Some(current_dir.clone())
            } else {
                session_info.as_ref().map(|s| s.worktree_path.clone())
            };

            if let Some(session_state) = session_info {
                if config.should_preserve_on_finish() {
                    session_manager
                        .update_session_status(&session_state.name, SessionStatus::Finished)?;
                } else {
                    session_manager.delete_state(&session_state.name)?;
                }
            }

            git_service.repository().checkout_branch(&base_branch)?;

            if let Some(ref path) = worktree_path {
                if path != &git_service.repository().root && !config.should_preserve_on_finish() {
                    if let Err(e) = git_service.remove_worktree(path) {
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
            if args.integrate {
                println!("  Integrated into: {}", base_branch);
            }
            println!("  Commit message: {}", args.message);
        }
        FinishResult::ConflictsPending { .. } => {
            println!("⚠ Conflicts detected during integration");
            println!("Resolve conflicts manually and run 'para continue'");
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
                default_integration_strategy: IntegrationStrategy::Squash,
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
            .args(&["init"])
            .status()
            .expect("Failed to init git repo");

        Command::new("git")
            .current_dir(&repo_path)
            .args(&["config", "user.name", "Test User"])
            .status()
            .expect("Failed to set git user name");

        Command::new("git")
            .current_dir(&repo_path)
            .args(&["config", "user.email", "test@example.com"])
            .status()
            .expect("Failed to set git user email");

        fs::write(repo_path.join("README.md"), "# Test Repository")
            .expect("Failed to write README");

        Command::new("git")
            .current_dir(&repo_path)
            .args(&["add", "README.md"])
            .status()
            .expect("Failed to add README");

        Command::new("git")
            .current_dir(&repo_path)
            .args(&["commit", "-m", "Initial commit"])
            .status()
            .expect("Failed to commit README");

        (temp_dir, repo_path)
    }

    #[test]
    fn test_finish_args_validation() {
        let valid_args = FinishArgs {
            message: "Test commit message".to_string(),
            branch: None,
            integrate: false,
            session: None,
        };
        assert!(valid_args.validate().is_ok());

        let empty_message_args = FinishArgs {
            message: "".to_string(),
            branch: None,
            integrate: false,
            session: None,
        };
        assert!(empty_message_args.validate().is_err());

        let whitespace_message_args = FinishArgs {
            message: "   ".to_string(),
            branch: None,
            integrate: false,
            session: None,
        };
        assert!(whitespace_message_args.validate().is_err());

        let invalid_branch_args = FinishArgs {
            message: "Test message".to_string(),
            branch: Some("-invalid-branch".to_string()),
            integrate: false,
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
}
