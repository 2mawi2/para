use crate::cli::commands::common::{
    create_claude_local_md, get_setup_script_path, run_worktree_setup_script,
};
use crate::cli::parser::StartArgs;
use crate::config::Config;
use crate::core::ide::IdeManager;
use crate::core::sandbox::config::SandboxResolver;
use crate::core::session::SessionManager;
use crate::utils::{generate_unique_name, validate_session_name, Result};

pub fn execute(config: Config, args: StartArgs) -> Result<()> {
    args.validate()?;

    let git_service = crate::core::git::GitService::discover().map_err(|e| {
        crate::utils::ParaError::git_error(format!("Failed to discover git repository: {e}"))
    })?;
    let repo_root = git_service.repository().root.clone();

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

        let docker_manager = crate::core::docker::DockerManager::with_options(
            config.clone(),
            network_isolation,
            allowed_domains.clone(),
            args.docker_image.clone(),
            !args.no_forward_keys,
        );
        let session = session_manager.create_docker_session_with_flags(
            session_name.clone(),
            &docker_manager,
            None,
            &args.docker_args,
            args.dangerously_skip_permissions,
        )?;

        create_claude_local_md(&session.worktree_path, &session.name)?;

        // Run setup script if specified
        if let Some(setup_script) =
            get_setup_script_path(&args.setup_script, &repo_root, &config, true)
        {
            docker_manager
                .run_setup_script(&session.name, &setup_script)
                .map_err(|e| {
                    crate::utils::ParaError::docker_error(format!(
                        "Failed to run setup script: {e}"
                    ))
                })?;
        }

        // Launch IDE connected to container
        docker_manager
            .launch_container_ide(&session, None, args.dangerously_skip_permissions)
            .map_err(|e| {
                crate::utils::ParaError::docker_error(format!("Failed to launch IDE: {e}"))
            })?;

        // Register container session with daemon for signal monitoring
        if let Err(e) = crate::core::daemon::client::register_container_session(
            &session.name,
            &session.worktree_path,
            &config,
        ) {
            eprintln!("Warning: Failed to register with daemon: {e}");
            // Continue anyway - daemon might not be running
        }

        (true, network_isolation, allowed_domains)
    } else {
        // Resolve sandbox settings using the resolver
        let resolver = SandboxResolver::new(&config);
        let sandbox_settings = resolver.resolve(
            args.sandbox_args.sandbox,
            args.sandbox_args.no_sandbox,
            args.sandbox_args.sandbox_profile.clone(),
        );

        let session = session_manager.create_session_with_all_flags(
            session_name.clone(),
            None,
            args.dangerously_skip_permissions,
            sandbox_settings.enabled,
            if sandbox_settings.enabled {
                Some(sandbox_settings.profile)
            } else {
                None
            },
        )?;

        create_claude_local_md(&session.worktree_path, &session.name)?;

        // Run setup script if specified
        if let Some(setup_script) =
            get_setup_script_path(&args.setup_script, &repo_root, &config, false)
        {
            run_worktree_setup_script(&setup_script, &session.name, &session.worktree_path)?;
        }

        let ide_manager = IdeManager::new(&config);
        let launch_options = crate::core::ide::LaunchOptions {
            skip_permissions: args.dangerously_skip_permissions,
            sandbox_override: if args.sandbox_args.no_sandbox {
                Some(false)
            } else if args.sandbox_args.sandbox {
                Some(true)
            } else {
                None
            },
            sandbox_profile: args.sandbox_args.sandbox_profile.clone(),
            ..Default::default()
        };
        ide_manager.launch_with_options(&session.worktree_path, launch_options)?;

        (false, false, vec![])
    };

    let session_state = session_manager
        .list_sessions()?
        .into_iter()
        .find(|s| s.name == session_name)
        .ok_or_else(|| crate::utils::ParaError::session_not_found(&session_name))?;

    println!("✅ Session '{session_name}' started successfully");
    if is_container {
        println!("   Container: para-{session_name}");

        // Show the actual Docker image being used
        if let Some(ref custom_image) = args.docker_image {
            println!("   Image: {custom_image} (custom)");
        } else if let Some(config_image) = config.get_docker_image() {
            println!("   Image: {config_image} (from config)");
        } else {
            println!("   Image: para-authenticated:latest (default)");
        }

        // Show network isolation warning if it's disabled
        if !network_isolation {
            println!("   ⚠️  Network isolation: OFF (use --allow-domains to enable)");
        }

        // Show API key warning if forwarding keys to custom images
        if !args.no_forward_keys && args.docker_image.is_some() {
            println!(
                "   ⚠️  API keys: Forwarding to custom image (use --no-forward-keys to disable)"
            );
            println!("      Security: Only use trusted images when forwarding API keys");
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
    use crate::cli::parser::SandboxArgs;
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
            docker: None,
            setup_script: None,
            sandbox: None,
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
            setup_script: None,
            docker_image: None,
            no_forward_keys: false,
            sandbox_args: SandboxArgs {
                sandbox: false,
                no_sandbox: false,
                sandbox_profile: None,
            },
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
            setup_script: None,
            docker_image: None,
            no_forward_keys: false,
            sandbox_args: SandboxArgs {
                sandbox: false,
                no_sandbox: false,
                sandbox_profile: None,
            },
        };

        let result = determine_session_name(&args, &session_manager).unwrap();
        assert!(!result.is_empty());
        assert!(result.contains('_'));
    }
}
