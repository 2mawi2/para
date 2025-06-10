use crate::cli::parser::StartArgs;
use crate::config::{Config, ConfigManager};
use crate::core::git::{GitOperations, GitService};
use crate::core::ide::IdeManager;
use crate::core::session::SessionManager;
use crate::utils::{
    generate_unique_name, validate_session_name, ParaError, Result,
};
use std::path::{Path, PathBuf};

pub fn execute(args: StartArgs) -> Result<()> {
    args.validate()?;

    let config = ConfigManager::load_or_create()
        .map_err(|e| ParaError::config_error(format!("Failed to load configuration: {}", e)))?;

    let mut session_manager = SessionManager::new(config.clone())?;

    let session_name = determine_session_name(&args, &session_manager)?;

    let session_state = session_manager.create_session(session_name.clone(), None)?;

    let ide_manager = IdeManager::new(&config);
    ide_manager.launch(&session_state.worktree_path, args.dangerously_skip_permissions)?;

    println!("✅ Session '{}' started successfully", session_name);
    println!("   Branch: {}", session_state.branch);
    println!("   Worktree: {}", session_state.worktree_path.display());
    println!("   IDE: {} launched", config.ide.name);

    Ok(())
}

fn determine_session_name(args: &StartArgs, session_manager: &SessionManager) -> Result<String> {
    match &args.name {
        Some(name) => {
            validate_session_name(name)?;
            // Generate ID to check for conflicts  
            let session_id = crate::utils::generate_session_id(name);
            if session_manager.session_exists(&session_id) {
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

// Removed helper functions - now handled by SessionManager

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
        let session_manager = SessionManager::new(config).unwrap();

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
        let session_manager = SessionManager::new(config).unwrap();

        let args = StartArgs {
            name: None,
            dangerously_skip_permissions: false,
        };

        let result = determine_session_name(&args, &session_manager).unwrap();
        assert!(!result.is_empty());
        assert!(result.contains('_'));
    }
}
