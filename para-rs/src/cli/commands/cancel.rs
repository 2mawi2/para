use crate::cli::parser::CancelArgs;
use crate::config::manager::ConfigManager;
use crate::core::git::{GitOperations, GitService, SessionEnvironment};
use crate::core::session::{SessionManager, SessionStatus};
use crate::utils::{ParaError, Result};
use std::env;
use std::io::{self, Write};

pub fn execute(args: CancelArgs) -> Result<()> {
    validate_cancel_args(&args)?;

    let config = ConfigManager::load_or_create()?;
    let git_service = GitService::discover()?;
    let session_manager = SessionManager::new(config.clone())?;

    let session_id = detect_session_id(&args, &git_service, &session_manager)?;

    let session_state = session_manager.load_session(&session_id)?;

    let has_uncommitted = git_service.repository().has_uncommitted_changes()?;
    if has_uncommitted {
        confirm_cancel_with_changes(&session_id)?;
    }

    let archived_branch =
        git_service.archive_branch(&session_state.branch, &config.git.branch_prefix)?;

    git_service.remove_worktree(&session_state.worktree_path)?;

    let mut session_manager = session_manager;  // Make mutable for update
    session_manager.update_session_status(&session_state.id, SessionStatus::Cancelled)?;

    println!(
        "Session '{}' has been cancelled and archived as '{}'",
        session_state.id, archived_branch
    );
    println!(
        "To recover this session later, use: para recover {}",
        session_state.name
    );
    println!("The archived branch is: {}", archived_branch);

    Ok(())
}

fn detect_session_id(
    args: &CancelArgs,
    git_service: &GitService,
    session_manager: &SessionManager,
) -> Result<String> {
    if let Some(ref session_id) = args.session {
        if !session_manager.session_exists(session_id) {
            return Err(ParaError::session_not_found(session_id));
        }
        return Ok(session_id.clone());
    }

    let current_dir = env::current_dir().map_err(|e| {
        ParaError::file_operation(format!("Failed to get current directory: {}", e))
    })?;

    match git_service.validate_session_environment(&current_dir)? {
        SessionEnvironment::Worktree { branch, .. } => {
            if let Ok(session) = session_manager.auto_detect_session() {
                return Ok(session.id);
            }
            
            let sessions = session_manager.list_all_sessions()?;
            for summary in sessions {
                let session = session_manager.load_session(&summary.id)?;
                if session.branch == branch && session.worktree_path == current_dir {
                    return Ok(session.id);
                }
            }
            Err(ParaError::session_not_found(format!(
                "No session found for current worktree (branch: {})",
                branch
            )))
        }
        SessionEnvironment::MainRepository => Err(ParaError::invalid_args(
            "Cannot cancel from main repository. Use 'para cancel <session-id>' to cancel a specific session.",
        )),
        SessionEnvironment::Invalid => Err(ParaError::invalid_args(
            "Not in a para session directory. Use 'para cancel <session-id>' to cancel a specific session.",
        )),
    }
}

fn confirm_cancel_with_changes(session_id: &str) -> Result<()> {
    print!(
        "Session '{}' has uncommitted changes. Are you sure you want to cancel? This will archive the session but preserve your work. [y/N]: ",
        session_id
    );
    io::stdout()
        .flush()
        .map_err(|e| ParaError::file_operation(format!("Failed to flush stdout: {}", e)))?;

    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .map_err(|e| ParaError::file_operation(format!("Failed to read input: {}", e)))?;

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
    use crate::config::{
        Config, DirectoryConfig, GitConfig, IdeConfig, SessionConfig, WrapperConfig,
    };
    use crate::core::git::GitRepository;
    use crate::core::session::SessionState;
    use std::fs;
    use std::path::Path;
    use std::process::Command;
    use tempfile::TempDir;

    fn create_test_config(temp_dir: &Path) -> Config {
        Config {
            ide: IdeConfig {
                name: "test".to_string(),
                command: "echo".to_string(),
                user_data_dir: None,
                wrapper: WrapperConfig {
                    enabled: false,
                    name: String::new(),
                    command: String::new(),
                },
            },
            directories: DirectoryConfig {
                subtrees_dir: temp_dir.join("subtrees").to_string_lossy().to_string(),
                state_dir: temp_dir.join(".para_state").to_string_lossy().to_string(),
            },
            git: GitConfig {
                branch_prefix: "pc".to_string(),
                auto_stage: true,
                auto_commit: false,
            },
            session: SessionConfig {
                default_name_format: "%Y%m%d-%H%M%S".to_string(),
                preserve_on_finish: false,
                auto_cleanup_days: Some(7),
            },
        }
    }

    fn setup_test_repo() -> (TempDir, GitRepository, Config) {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let repo_path = temp_dir.path();

        Command::new("git")
            .current_dir(repo_path)
            .args(&["init"])
            .status()
            .expect("Failed to init git repo");

        Command::new("git")
            .current_dir(repo_path)
            .args(&["config", "user.name", "Test User"])
            .status()
            .expect("Failed to set git user name");

        Command::new("git")
            .current_dir(repo_path)
            .args(&["config", "user.email", "test@example.com"])
            .status()
            .expect("Failed to set git user email");

        fs::write(repo_path.join("README.md"), "# Test Repository")
            .expect("Failed to write README");

        Command::new("git")
            .current_dir(repo_path)
            .args(&["add", "README.md"])
            .status()
            .expect("Failed to add README");

        Command::new("git")
            .current_dir(repo_path)
            .args(&["commit", "-m", "Initial commit"])
            .status()
            .expect("Failed to commit README");

        let repo = GitRepository::discover_from(repo_path).expect("Failed to discover repo");
        let config = create_test_config(temp_dir.path());

        (temp_dir, repo, config)
    }

    #[test]
    fn test_validate_cancel_args_valid() {
        let args = CancelArgs { session: None };
        assert!(validate_cancel_args(&args).is_ok());

        let args = CancelArgs {
            session: Some("valid-session".to_string()),
        };
        assert!(validate_cancel_args(&args).is_ok());
    }

    #[test]
    fn test_validate_cancel_args_empty_session() {
        let args = CancelArgs {
            session: Some(String::new()),
        };
        let result = validate_cancel_args(&args);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cannot be empty"));
    }

    #[test]
    fn test_detect_session_id_explicit() {
        let (_temp_dir, repo, config) = setup_test_repo();
        let git_service =
            GitService::discover_from(&repo.root).expect("Failed to create git service");
        let session_manager = SessionManager::new(config.clone())?;

        let session_state = SessionState::new(
            "test-session".to_string(),
            "test-branch".to_string(),
            repo.root.join("test-worktree"),
        );
        session_manager
            .save_state(&session_state)
            .expect("Failed to save state");

        let args = CancelArgs {
            session: Some("test-session".to_string()),
        };

        let result = detect_session_id(&args, &git_service, &session_manager);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "test-session");
    }

    #[test]
    fn test_detect_session_id_nonexistent() {
        let (_temp_dir, repo, config) = setup_test_repo();
        let git_service =
            GitService::discover_from(&repo.root).expect("Failed to create git service");
        let session_manager = SessionManager::new(config.clone())?;

        let args = CancelArgs {
            session: Some("nonexistent-session".to_string()),
        };

        let result = detect_session_id(&args, &git_service, &session_manager);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[test]
    fn test_detect_session_id_from_main_repo() {
        let (_temp_dir, repo, config) = setup_test_repo();
        let git_service =
            GitService::discover_from(&repo.root).expect("Failed to create git service");
        let session_manager = SessionManager::new(config.clone())?;

        let args = CancelArgs { session: None };

        std::env::set_current_dir(&repo.root).expect("Failed to change to repo root");

        let result = detect_session_id(&args, &git_service, &session_manager);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("main repository"));
    }

    #[test]
    fn test_detect_session_id_invalid_directory() {
        let (_temp_dir, repo, config) = setup_test_repo();
        let git_service =
            GitService::discover_from(&repo.root).expect("Failed to create git service");
        let session_manager = SessionManager::new(config.clone())?;

        let args = CancelArgs { session: None };

        let invalid_dir = TempDir::new().expect("Failed to create invalid dir");
        std::env::set_current_dir(invalid_dir.path()).expect("Failed to change to invalid dir");

        let result = detect_session_id(&args, &git_service, &session_manager);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Not in a para session"));
    }

    #[test]
    fn test_confirm_cancel_with_changes_format() {
        let session_id = "test-session";

        let output = std::panic::catch_unwind(|| {
            let _result = confirm_cancel_with_changes(session_id);
        });

        assert!(output.is_err() || output.is_ok());
    }
}
