use crate::cli::commands::common::create_claude_local_md;
use crate::cli::parser::StartArgs;
use crate::config::Config;
use crate::core::ide::IdeManager;
use crate::core::session::SessionManager;
use crate::utils::{generate_unique_name, validate_session_name, Result};
use std::path::{Path, PathBuf};

/// Determine which setup script to use based on priority order
fn get_setup_script_path(
    cli_arg: &Option<PathBuf>,
    repo_root: &Path,
    config: &Config,
    is_docker: bool,
) -> Option<PathBuf> {
    // 1. CLI argument has highest priority
    if let Some(path) = cli_arg {
        if path.exists() {
            return Some(path.clone());
        } else {
            eprintln!("Warning: Setup script '{}' not found", path.display());
            return None;
        }
    }

    // 2. Check for environment-specific default scripts
    if is_docker {
        let docker_script = repo_root.join(".para/setup-docker.sh");
        if docker_script.exists() {
            return Some(docker_script);
        }
    } else {
        let worktree_script = repo_root.join(".para/setup-worktree.sh");
        if worktree_script.exists() {
            return Some(worktree_script);
        }
    }

    // 3. Check for generic default .para/setup.sh
    let default_script = repo_root.join(".para/setup.sh");
    if default_script.exists() {
        return Some(default_script);
    }

    // 4. Check config for setup script path
    // For Docker, check docker.setup_script first, then fall back to general setup_script
    if is_docker {
        if let Some(docker_config) = &config.docker {
            if let Some(script_path) = &docker_config.setup_script {
                let config_script = if Path::new(script_path).is_absolute() {
                    PathBuf::from(script_path)
                } else {
                    repo_root.join(script_path)
                };
                if config_script.exists() {
                    return Some(config_script);
                } else {
                    eprintln!(
                        "Warning: Docker config setup script '{}' not found",
                        config_script.display()
                    );
                }
            }
        }
    }

    // Check general setup_script in config
    if let Some(script_path) = &config.setup_script {
        let config_script = if Path::new(script_path).is_absolute() {
            PathBuf::from(script_path)
        } else {
            repo_root.join(script_path)
        };
        if config_script.exists() {
            return Some(config_script);
        } else {
            eprintln!(
                "Warning: Config setup script '{}' not found",
                config_script.display()
            );
        }
    }

    None
}

/// Run a setup script for a regular worktree session
fn run_worktree_setup_script(
    script_path: &Path,
    session_name: &str,
    worktree_path: &Path,
) -> Result<()> {
    use std::process::Command;

    println!("🔧 Running setup script: {}", script_path.display());

    // Security warning
    eprintln!("⚠️  Warning: Setup scripts run with your full user permissions!");
    eprintln!("   Only run scripts from trusted sources.");
    eprintln!("   Script: {}", script_path.display());

    let mut cmd = Command::new("bash");
    cmd.arg(script_path);
    cmd.current_dir(worktree_path);

    // Set environment variables
    cmd.env("PARA_WORKSPACE", worktree_path);
    cmd.env("PARA_SESSION", session_name);

    let status = cmd.status().map_err(|e| {
        crate::utils::ParaError::ide_error(format!("Failed to execute setup script: {}", e))
    })?;

    if !status.success() {
        return Err(crate::utils::ParaError::ide_error(format!(
            "Setup script failed with exit code: {}",
            status.code().unwrap_or(-1)
        )));
    }

    println!("✅ Setup script completed successfully");
    Ok(())
}

pub fn execute(config: Config, args: StartArgs) -> Result<()> {
    args.validate()?;

    let git_service = crate::core::git::GitService::discover().map_err(|e| {
        crate::utils::ParaError::git_error(format!("Failed to discover git repository: {}", e))
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

        // Create CLAUDE.local.md in the session directory
        create_claude_local_md(&session.worktree_path, &session.name)?;

        // Run setup script if specified
        if let Some(setup_script) =
            get_setup_script_path(&args.setup_script, &repo_root, &config, true)
        {
            docker_manager
                .run_setup_script(&session.name, &setup_script)
                .map_err(|e| {
                    crate::utils::ParaError::docker_error(format!(
                        "Failed to run setup script: {}",
                        e
                    ))
                })?;
        }

        // Launch IDE connected to container
        docker_manager
            .launch_container_ide(&session, None, args.dangerously_skip_permissions)
            .map_err(|e| {
                crate::utils::ParaError::docker_error(format!("Failed to launch IDE: {}", e))
            })?;

        // Register container session with daemon for signal monitoring
        if let Err(e) = crate::core::daemon::client::register_container_session(
            &session.name,
            &session.worktree_path,
            &config,
        ) {
            eprintln!("Warning: Failed to register with daemon: {}", e);
            // Continue anyway - daemon might not be running
        }

        (true, network_isolation, allowed_domains)
    } else {
        // Create regular worktree session
        let session = session_manager.create_session_with_flags(
            session_name.clone(),
            None,
            args.dangerously_skip_permissions,
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
            sandbox_override: if args.no_sandbox {
                Some(false)
            } else if args.sandbox {
                Some(true)
            } else {
                None
            },
            sandbox_profile: args.sandbox_profile.clone(),
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

    println!("✅ Session '{}' started successfully", session_name);
    if is_container {
        println!("   Container: para-{}", session_name);

        // Show the actual Docker image being used
        if let Some(ref custom_image) = args.docker_image {
            println!("   Image: {} (custom)", custom_image);
        } else if let Some(config_image) = config.get_docker_image() {
            println!("   Image: {} (from config)", config_image);
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
    use crate::config::{
        Config, DirectoryConfig, GitConfig, IdeConfig, SessionConfig, WrapperConfig,
    };
    use std::fs;
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
            sandbox: false,
            no_sandbox: false,
            sandbox_profile: None,
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
            sandbox: false,
            no_sandbox: false,
            sandbox_profile: None,
        };

        let result = determine_session_name(&args, &session_manager).unwrap();
        assert!(!result.is_empty());
        assert!(result.contains('_'));
    }

    #[test]
    fn test_get_setup_script_path_cli_priority() {
        let temp_dir = TempDir::new().unwrap();
        let repo_root = temp_dir.path();

        // Create scripts
        let cli_script = repo_root.join("cli-setup.sh");
        let default_script = repo_root.join(".para/setup.sh");
        fs::create_dir_all(repo_root.join(".para")).unwrap();
        fs::write(&cli_script, "#!/bin/bash\necho cli").unwrap();
        fs::write(&default_script, "#!/bin/bash\necho default").unwrap();

        let config = crate::test_utils::test_helpers::create_test_config();

        // CLI argument should take priority
        let result = get_setup_script_path(&Some(cli_script.clone()), repo_root, &config, false);
        assert_eq!(result, Some(cli_script));
    }

    #[test]
    fn test_get_setup_script_path_default_location() {
        let temp_dir = TempDir::new().unwrap();
        let repo_root = temp_dir.path();

        // Create default script
        let default_script = repo_root.join(".para/setup.sh");
        fs::create_dir_all(repo_root.join(".para")).unwrap();
        fs::write(&default_script, "#!/bin/bash\necho default").unwrap();

        let config = crate::test_utils::test_helpers::create_test_config();

        // Should find default script when no CLI argument
        let result = get_setup_script_path(&None, repo_root, &config, false);
        assert_eq!(result, Some(default_script));
    }

    #[test]
    fn test_get_setup_script_path_config_general() {
        let temp_dir = TempDir::new().unwrap();
        let repo_root = temp_dir.path();

        // Create config script
        let config_script = repo_root.join("scripts/setup.sh");
        fs::create_dir_all(repo_root.join("scripts")).unwrap();
        fs::write(&config_script, "#!/bin/bash\necho config").unwrap();

        let mut config = crate::test_utils::test_helpers::create_test_config();
        config.setup_script = Some("scripts/setup.sh".to_string());

        // Should find config script when no CLI argument or default
        let result = get_setup_script_path(&None, repo_root, &config, false);
        assert_eq!(result, Some(config_script));
    }

    #[test]
    fn test_get_setup_script_path_nonexistent() {
        let temp_dir = TempDir::new().unwrap();
        let repo_root = temp_dir.path();

        let config = crate::test_utils::test_helpers::create_test_config();

        // Should return None when no scripts exist
        let result = get_setup_script_path(&None, repo_root, &config, false);
        assert_eq!(result, None);

        // Should return None for nonexistent CLI script
        let nonexistent = repo_root.join("nonexistent.sh");
        let result = get_setup_script_path(&Some(nonexistent), repo_root, &config, false);
        assert_eq!(result, None);
    }
}
