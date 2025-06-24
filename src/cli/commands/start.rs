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

    // Track whether we're using Docker and network isolation settings
    let (is_container, network_isolation, _allowed_domains) = if args.container {
        // Create Docker container session
        let (network_isolation, allowed_domains) = if let Some(ref domains) = args.allow_domains {
            // Enable network isolation when --allow-domains is used
            let additional_domains: Vec<String> = domains
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            (true, additional_domains)
        } else {
            (false, vec![])
        };

        let docker_manager = crate::core::docker::DockerManager::new(
            config.clone(),
            network_isolation,
            allowed_domains.clone(),
        );
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

        (true, network_isolation, allowed_domains)
    } else {
        // Create regular worktree session
        let session = session_manager.create_session(session_name.clone(), None)?;

        create_claude_local_md(&session.worktree_path, &session.name)?;

        let ide_manager = IdeManager::new(&config);
        ide_manager.launch(&session.worktree_path, args.dangerously_skip_permissions)?;

        (false, false, vec![])
    };

    let session_state = session_manager
        .list_sessions()?
        .into_iter()
        .find(|s| s.name == session_name)
        .ok_or_else(|| crate::utils::ParaError::session_not_found(&session_name))?;

    println!("✅ Session '{}' started successfully", session_name);
    if is_container {
        println!("   Container: para-{}", session_name);
        println!("   Image: para-authenticated:latest");

        // Show network isolation warning if it's disabled
        if !network_isolation {
            println!("   ⚠️  Network isolation: OFF (use --allow-domains to enable)");
        }
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
            container: false,
            allow_domains: None,
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
            allow_domains: None,
            docker_args: vec![],
        };

        let result = determine_session_name(&args, &session_manager).unwrap();
        assert!(!result.is_empty());
        assert!(result.contains('_'));
    }
}
