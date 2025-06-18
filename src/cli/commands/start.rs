use crate::cli::commands::common::create_claude_local_md;
use crate::cli::parser::StartArgs;
use crate::config::Config;
use crate::core::ide::IdeManager;
use crate::core::session::SessionManager;
use crate::utils::{generate_unique_name, validate_session_name, Result};

pub fn execute(config: Config, args: StartArgs) -> Result<()> {
    args.validate()?;

    let mut session_manager = SessionManager::new(&config);

    let session_name = determine_session_name(&args, &session_manager)?;

    let session_state = session_manager.create_session(session_name.clone(), None)?;

    create_claude_local_md(&session_state.worktree_path, &session_state.name)?;

    let ide_manager = IdeManager::new(&config);
    ide_manager.launch(
        &session_state.worktree_path,
        args.dangerously_skip_permissions,
    )?;

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
    use crate::config::{
        Config, DirectoryConfig, GitConfig, IdeConfig, SessionConfig, WrapperConfig,
    };
    use tempfile::TempDir;

    fn create_simple_test_config(temp_dir: &TempDir) -> Config {
        Config {
            ide: IdeConfig {
                name: "test-ide".to_string(),
                command: "echo".to_string(),
                user_data_dir: None,
                wrapper: WrapperConfig {
                    enabled: true,
                    name: "test-wrapper".to_string(),
                    command: "echo".to_string(),
                },
            },
            directories: DirectoryConfig {
                subtrees_dir: temp_dir
                    .path()
                    .join("subtrees")
                    .to_string_lossy()
                    .to_string(),
                state_dir: temp_dir
                    .path()
                    .join(".para_state")
                    .to_string_lossy()
                    .to_string(),
            },
            git: GitConfig {
                branch_prefix: "test".to_string(),
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

    #[test]
    fn test_determine_session_name_with_provided_name() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_simple_test_config(&temp_dir);
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
        let temp_dir = TempDir::new().unwrap();
        let config = create_simple_test_config(&temp_dir);
        let session_manager = SessionManager::new(&config);

        let args = StartArgs {
            name: None,
            dangerously_skip_permissions: false,
        };

        let result = determine_session_name(&args, &session_manager).unwrap();
        assert!(!result.is_empty());
        assert!(result.contains('_'));
    }
}
