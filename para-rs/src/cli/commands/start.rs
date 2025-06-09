use crate::cli::parser::StartArgs;
use crate::config::{Config, ConfigManager};
use crate::core::git::{GitOperations, GitService};
use crate::core::ide::IdeManager;
use crate::core::session::{SessionManager, SessionState};
use crate::platform::{get_platform_manager, IdeConfig};
use crate::utils::{
    generate_branch_name, generate_unique_name, validate_session_name, ParaError, Result,
};
use std::path::{Path, PathBuf};

pub fn execute(args: StartArgs) -> Result<()> {
    args.validate()?;

    let config = ConfigManager::load_or_create()
        .map_err(|e| ParaError::config_error(format!("Failed to load configuration: {}", e)))?;

    let git_service = GitService::discover()
        .map_err(|e| ParaError::git_operation(format!("Not in a git repository: {}", e)))?;

    let session_manager = SessionManager::new(&config);

    let session_name = determine_session_name(&args, &session_manager)?;

    let branch_name = generate_branch_name(config.get_branch_prefix());

    let worktree_path = create_worktree_path(&config, &session_name)?;

    check_session_conflicts(&session_manager, &session_name, &worktree_path)?;

    git_service.create_worktree(&branch_name, &worktree_path)?;

    let session_state = SessionState::new(
        session_name.clone(),
        branch_name.clone(),
        worktree_path.clone(),
    );

    session_manager.save_state(&session_state)?;

    let platform = get_platform_manager();
    let ide_config = IdeConfig {
        name: config.ide.name.clone(),
        command: config.ide.command.clone(),
        wrapper_enabled: config.ide.wrapper.enabled,
        wrapper_name: config.ide.wrapper.name.clone(),
        wrapper_command: config.ide.wrapper.command.clone(),
    };
    
    if let Err(e) = platform.launch_ide_with_wrapper(&ide_config, &worktree_path, None) {
        eprintln!("Warning: Failed to launch IDE using platform manager, falling back to legacy IDE manager: {}", e);
        let ide_manager = IdeManager::new(&config);
        ide_manager.launch(&worktree_path, args.dangerously_skip_permissions)?;
    }

    println!("✅ Session '{}' started successfully", session_name);
    println!("   Branch: {}", branch_name);
    println!("   Worktree: {}", worktree_path.display());
    println!("   IDE: {} launched", config.ide.name);

    Ok(())
}

fn determine_session_name(args: &StartArgs, session_manager: &SessionManager) -> Result<String> {
    match &args.name {
        Some(name) => {
            validate_session_name(name)?;
            if session_manager.session_exists(name) {
                return Err(ParaError::session_exists(name));
            }
            Ok(name.clone())
        }
        None => {
            let existing_sessions = session_manager
                .list_sessions()?
                .into_iter()
                .map(|s| s.name)
                .collect::<Vec<String>>();
            Ok(generate_unique_name(&existing_sessions))
        }
    }
}

fn create_worktree_path(config: &Config, session_name: &str) -> Result<PathBuf> {
    let subtrees_dir = PathBuf::from(config.get_subtrees_dir());
    let worktree_path = subtrees_dir.join(session_name);

    if worktree_path.exists() {
        return Err(ParaError::file_operation(format!(
            "Worktree directory already exists: {}",
            worktree_path.display()
        )));
    }

    Ok(worktree_path)
}

fn check_session_conflicts(
    session_manager: &SessionManager,
    session_name: &str,
    worktree_path: &Path,
) -> Result<()> {
    if session_manager.session_exists(session_name) {
        return Err(ParaError::session_exists(session_name));
    }

    if worktree_path.exists() {
        return Err(ParaError::file_operation(format!(
            "Worktree path already exists: {}",
            worktree_path.display()
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use std::fs;
    use std::process::Command;
    use tempfile::TempDir;

    fn setup_test_repo() -> (TempDir, Config) {
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

        let config = Config {
            ide: crate::config::IdeConfig {
                name: "echo".to_string(),
                command: "echo".to_string(),
                user_data_dir: None,
                wrapper: crate::config::WrapperConfig {
                    enabled: false,
                    name: String::new(),
                    command: String::new(),
                },
            },
            directories: crate::config::DirectoryConfig {
                subtrees_dir: repo_path.join("subtrees").to_string_lossy().to_string(),
                state_dir: repo_path.join(".para_state").to_string_lossy().to_string(),
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
        };

        (temp_dir, config)
    }

    #[test]
    fn test_determine_session_name_with_provided_name() {
        let (_temp_dir, config) = setup_test_repo();
        let session_manager = SessionManager::new(&config);

        let args = StartArgs {
            name: Some("test-session".to_string()),
            dangerously_skip_permissions: false,
        };

        let result = determine_session_name(&args, &session_manager).unwrap();
        assert_eq!(result, "test-session");
    }

    #[test]
    fn test_determine_session_name_auto_generate() {
        let (_temp_dir, config) = setup_test_repo();
        let session_manager = SessionManager::new(&config);

        let args = StartArgs {
            name: None,
            dangerously_skip_permissions: false,
        };

        let result = determine_session_name(&args, &session_manager).unwrap();
        assert!(!result.is_empty());
        assert!(result.contains('_'));
    }

    #[test]
    fn test_create_worktree_path() {
        let (_temp_dir, config) = setup_test_repo();

        let path = create_worktree_path(&config, "test-session").unwrap();
        assert!(path.to_string_lossy().contains("subtrees"));
        assert!(path.to_string_lossy().contains("test-session"));
    }

    #[test]
    fn test_check_session_conflicts() {
        let (_temp_dir, config) = setup_test_repo();
        let session_manager = SessionManager::new(&config);
        let worktree_path = PathBuf::from("/tmp/nonexistent-test-path");

        let result = check_session_conflicts(&session_manager, "new-session", &worktree_path);
        assert!(result.is_ok());
    }
}
