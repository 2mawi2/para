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

    let session_state = if args.container {
        // Create Docker container session
        if !config.docker.enabled {
            return Err(crate::utils::ParaError::invalid_config(
                "Docker is not enabled in configuration. Run 'para config' to enable Docker support."
            ));
        }

        let docker_manager = crate::core::docker::DockerManager::new(config.clone());
        let session = session_manager.create_docker_session(
            session_name.clone(),
            &docker_manager,
            None,
            &args.docker_args,
        )?;

        // Create CLAUDE.local.md in the session directory
        create_claude_local_md(&session.worktree_path, &session.name)?;

        // Launch IDE connected to container
        docker_manager
            .launch_container_ide(&session, None, args.dangerously_skip_permissions)
            .map_err(|e| {
                crate::utils::ParaError::docker_error(format!("Failed to launch IDE: {}", e))
            })?;

        session
    } else {
        // Create regular worktree session
        let session = session_manager.create_session(session_name.clone(), None)?;

        create_claude_local_md(&session.worktree_path, &session.name)?;

        let ide_manager = IdeManager::new(&config);
        ide_manager.launch(&session.worktree_path, args.dangerously_skip_permissions)?;

        session
    };

    println!("✅ Session '{}' started successfully", session_name);
    if args.container {
        println!("   Container: para-{}", session_name);
        println!("   Image: para-authenticated:latest");
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{
        Config, DirectoryConfig, DockerConfig, GitConfig, IdeConfig, SessionConfig, WrapperConfig,
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
            docker: DockerConfig {
                enabled: false,
                mount_workspace: true,
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
            container: false,
            docker_args: vec![],
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
            container: false,
            docker_args: vec![],
        };

        let result = determine_session_name(&args, &session_manager).unwrap();
        assert!(!result.is_empty());
        assert!(result.contains('_'));
    }
}
