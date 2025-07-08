use crate::cli::commands::common::create_claude_local_md;
use crate::cli::parser::DispatchArgs;
use crate::config::Config;
use crate::core::git::{GitOperations, GitService};
use crate::core::sandbox::config::SandboxResolver;
use crate::core::session::{SessionManager, SessionState};
use crate::utils::{names::*, ParaError, Result};
use std::fs;
use std::io::{self, IsTerminal, Read};
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

    let status = cmd
        .status()
        .map_err(|e| ParaError::ide_error(format!("Failed to execute setup script: {e}")))?;

    if !status.success() {
        return Err(ParaError::ide_error(format!(
            "Setup script failed with exit code: {}",
            status.code().unwrap_or(-1)
        )));
    }

    println!("✅ Setup script completed successfully");
    Ok(())
}

pub fn execute(config: Config, args: DispatchArgs) -> Result<()> {
    args.validate()?;

    let (session_name, prompt) = args.resolve_prompt_and_session()?;

    validate_claude_code_ide(&config)?;

    let git_service = GitService::discover()
        .map_err(|e| ParaError::git_error(format!("Failed to discover git repository: {e}")))?;
    let repo_root = git_service.repository().root.clone();

    let session_manager = SessionManager::new(&config);
    let session_name = match session_name {
        Some(name) => {
            validate_session_name(&name)?;
            if session_manager.session_exists(&name) {
                return Err(ParaError::session_exists(&name));
            }
            name
        }
        None => {
            let existing_sessions = session_manager
                .list_sessions()?
                .into_iter()
                .map(|s| s.name)
                .collect::<Vec<String>>();
            generate_unique_name(&existing_sessions)
        }
    };

    let branch_name = generate_friendly_branch_name(config.get_branch_prefix(), &session_name);
    let session_id = session_name.clone();

    let mut session_manager = SessionManager::new(&config);

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
            session_id.clone(),
            &docker_manager,
            Some(&prompt),
            &args.docker_args,
            args.dangerously_skip_permissions,
        )?;

        // Write task file
        let state_dir = session_manager.state_dir();
        let task_file = state_dir.join(format!("{session_id}.task"));
        fs::write(&task_file, &prompt)
            .map_err(|e| ParaError::fs_error(format!("Failed to write task file: {e}")))?;

        // Create CLAUDE.local.md in the session directory
        create_claude_local_md(&session.worktree_path, &session.name)?;

        // Run setup script if specified
        if let Some(setup_script) =
            get_setup_script_path(&args.setup_script, &repo_root, &config, true)
        {
            docker_manager
                .run_setup_script(&session.name, &setup_script)
                .map_err(|e| ParaError::docker_error(format!("Failed to run setup script: {e}")))?;
        }

        // Launch IDE connected to container with initial prompt
        docker_manager
            .launch_container_ide(&session, Some(&prompt), args.dangerously_skip_permissions)
            .map_err(|e| ParaError::docker_error(format!("Failed to launch IDE: {e}")))?;

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
        // Create regular worktree session
        let subtrees_path = repo_root.join(&config.directories.subtrees_dir);
        let session_path = subtrees_path.join(&session_id);

        if !subtrees_path.exists() {
            fs::create_dir_all(&subtrees_path).map_err(|e| {
                ParaError::fs_error(format!("Failed to create subtrees directory: {e}"))
            })?;
        }

        // Get the current branch as the parent branch
        let parent_branch = git_service
            .repository()
            .get_current_branch()
            .unwrap_or_else(|_| "main".to_string());

        git_service
            .create_worktree(&branch_name, &session_path)
            .map_err(|e| ParaError::git_error(format!("Failed to create worktree: {e}")))?;

        // Resolve sandbox settings using the resolver
        let resolver = SandboxResolver::new(&config);
        let sandbox_settings = resolver.resolve_with_network(
            args.sandbox_args.sandbox,
            args.sandbox_args.no_sandbox,
            args.sandbox_args.sandbox_profile.clone(),
            args.sandbox_args.sandbox_no_network,
        );

        let mut session_state = SessionState::with_all_flags(
            session_id.clone(),
            branch_name,
            session_path.clone(),
            parent_branch,
            args.dangerously_skip_permissions,
            sandbox_settings.enabled,
            if sandbox_settings.enabled {
                Some(sandbox_settings.profile)
            } else {
                None
            },
        );

        session_state.task_description = Some(prompt.clone());
        session_manager.save_state(&session_state)?;

        // Write task file
        let state_dir = session_manager.state_dir();
        let task_file = state_dir.join(format!("{session_id}.task"));
        fs::write(&task_file, &prompt)
            .map_err(|e| ParaError::fs_error(format!("Failed to write task file: {e}")))?;

        create_claude_local_md(&session_state.worktree_path, &session_state.name)?;

        // Run setup script if specified
        if let Some(setup_script) =
            get_setup_script_path(&args.setup_script, &repo_root, &config, false)
        {
            run_worktree_setup_script(
                &setup_script,
                &session_state.name,
                &session_state.worktree_path,
            )?;
        }

        create_launch_metadata(&config, &session_state.worktree_path)?;
        launch_claude_code(
            &config,
            &session_state.worktree_path,
            &prompt,
            args.dangerously_skip_permissions,
            &args,
        )?;

        (false, false, vec![])
    };

    // Get session state for display
    let session_state = session_manager
        .list_sessions()?
        .into_iter()
        .find(|s| s.name == session_id)
        .ok_or_else(|| ParaError::session_not_found(&session_id))?;

    println!(
        "✅ Created session '{}' with Claude Code",
        session_state.name
    );
    if is_container {
        println!("   Container: para-{}", session_state.name);

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

    Ok(())
}

fn validate_claude_code_ide(config: &Config) -> Result<()> {
    if (config.ide.command.to_lowercase() == "claude"
        || config.ide.command.to_lowercase() == "claude-code")
        && config.is_wrapper_enabled()
    {
        return Ok(());
    }

    Err(ParaError::invalid_config(format!(
        "Dispatch command requires Claude Code in wrapper mode. Current IDE: '{}' with command: '{}', wrapper enabled: {}. Run 'para config' to configure Claude Code with wrapper mode.",
        config.ide.name, config.ide.command, config.is_wrapper_enabled()
    )))
}

fn launch_claude_code(
    config: &Config,
    session_path: &Path,
    prompt: &str,
    skip_permissions: bool,
    args: &DispatchArgs,
) -> Result<()> {
    let options = crate::core::claude_launcher::ClaudeLaunchOptions {
        skip_permissions,
        session_id: None,
        continue_conversation: false,
        prompt_content: if prompt.is_empty() {
            None
        } else {
            Some(prompt.to_string())
        },
        sandbox_override: if args.sandbox_args.no_sandbox {
            Some(false)
        } else if args.sandbox_args.sandbox || args.sandbox_args.sandbox_no_network {
            Some(true)
        } else {
            None
        },
        sandbox_profile: args.sandbox_args.sandbox_profile.clone(),
        network_sandbox: args.sandbox_args.sandbox_no_network,
        allowed_domains: args.sandbox_args.allowed_domains.clone(),
    };

    crate::core::claude_launcher::launch_claude_with_context(config, session_path, options)
}

fn create_launch_metadata(config: &Config, session_path: &Path) -> Result<()> {
    let state_dir = std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(".para_state");

    fs::create_dir_all(&state_dir)
        .map_err(|e| ParaError::fs_error(format!("Failed to create state directory: {e}")))?;

    let session_id = session_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("unknown");

    let launch_file = state_dir.join(format!("{session_id}.launch"));

    let launch_content = format!(
        "LAUNCH_METHOD=wrapper\nWRAPPER_IDE={}\n",
        config.ide.wrapper.name
    );
    fs::write(&launch_file, launch_content)
        .map_err(|e| ParaError::fs_error(format!("Failed to write launch file: {e}")))?;

    Ok(())
}

impl DispatchArgs {
    pub fn resolve_prompt_and_session(&self) -> Result<(Option<String>, String)> {
        // Priority order:
        // 1. File flag (highest priority)
        // 2. Explicit arguments
        // 3. Stdin input (lowest priority)

        // If we have a --file argument, use it directly without checking stdin
        // This prevents blocking in non-terminal environments like MCP
        if self.file.is_some() {
            return self.resolve_prompt_and_session_no_stdin();
        }

        // If we have explicit arguments, use them instead of checking stdin
        // This fixes the MCP issue where stdin is not a terminal but we have valid args
        if self.name_or_prompt.is_some() || self.prompt.is_some() {
            return self.resolve_prompt_and_session_no_stdin();
        }

        // Only check stdin if we don't have file flag or explicit arguments
        if !io::stdin().is_terminal() {
            let mut buffer = String::new();
            io::stdin().read_to_string(&mut buffer).map_err(|e| {
                ParaError::file_operation(format!("Failed to read from stdin: {e}"))
            })?;

            if buffer.trim().is_empty() {
                return Err(ParaError::invalid_args("Piped input is empty"));
            }

            // When using stdin, the first positional argument (if any) is the session name
            return Ok((self.name_or_prompt.clone(), buffer));
        }

        // No file, no explicit args, no stdin input - fall back to no_stdin method
        // which will return appropriate error
        self.resolve_prompt_and_session_no_stdin()
    }

    fn resolve_prompt_and_session_no_stdin(&self) -> Result<(Option<String>, String)> {
        match (&self.name_or_prompt, &self.prompt, &self.file) {
            (_, _, Some(file_path)) => {
                let prompt = read_file_content(file_path)?;
                if prompt.trim().is_empty() {
                    return Err(ParaError::file_not_found(format!(
                        "file is empty: {}",
                        file_path.display()
                    )));
                }
                Ok((self.name_or_prompt.clone(), prompt))
            }

            (Some(arg), None, None) => {
                if is_likely_file_path(arg) {
                    let prompt = read_file_content(Path::new(arg))?;
                    if prompt.trim().is_empty() {
                        return Err(ParaError::file_not_found(format!("file is empty: {arg}")));
                    }
                    Ok((None, prompt))
                } else {
                    Ok((None, arg.clone()))
                }
            }

            (Some(session), Some(prompt_or_file), None) => {
                if is_likely_file_path(prompt_or_file) {
                    let prompt = read_file_content(Path::new(prompt_or_file))?;
                    if prompt.trim().is_empty() {
                        return Err(ParaError::file_not_found(format!(
                            "file is empty: {prompt_or_file}"
                        )));
                    }
                    Ok((Some(session.clone()), prompt))
                } else {
                    Ok((Some(session.clone()), prompt_or_file.clone()))
                }
            }

            (None, None, None) => Err(ParaError::invalid_args(
                "dispatch requires a prompt text or file path",
            )),

            _ => Err(ParaError::invalid_args(
                "Invalid argument combination for dispatch",
            )),
        }
    }
}

fn is_likely_file_path(input: &str) -> bool {
    if input.is_empty() {
        return false;
    }

    if Path::new(input).is_file() {
        return true;
    }

    if input.starts_with("http://")
        || input.starts_with("https://")
        || input.starts_with("ftp://")
        || input.starts_with("ftps://")
        || input.starts_with("ssh://")
        || input.starts_with("git://")
        || input.starts_with("file://")
    {
        return false;
    }

    if input.contains('/') {
        if (input.contains(" http://")
            || input.contains(" https://")
            || input.contains(" ftp://")
            || input.contains(" ssh://"))
            && input.contains(' ')
        {
            return false;
        }
        return true;
    }

    input.ends_with(".txt")
        || input.ends_with(".md")
        || input.ends_with(".rst")
        || input.ends_with(".org")
        || input.ends_with(".prompt")
        || input.ends_with(".tmpl")
        || input.ends_with(".template")
}

fn read_file_content(path: &Path) -> Result<String> {
    let absolute_path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|e| ParaError::fs_error(format!("Failed to get current directory: {e}")))?
            .join(path)
    };

    if !absolute_path.exists() {
        return Err(ParaError::file_not_found(format!(
            "file not found: {}",
            path.display()
        )));
    }

    if !absolute_path.is_file() {
        return Err(ParaError::file_operation(format!(
            "path is not a file: {}",
            path.display()
        )));
    }

    match fs::metadata(&absolute_path) {
        Ok(metadata) => {
            if metadata.permissions().readonly() && metadata.len() == 0 {
                return Err(ParaError::file_not_found(format!(
                    "file not readable: {}",
                    path.display()
                )));
            }
        }
        Err(_) => {
            return Err(ParaError::file_not_found(format!(
                "file not readable: {}",
                path.display()
            )));
        }
    }

    fs::read_to_string(&absolute_path).map_err(|e| {
        ParaError::file_operation(format!("failed to read file: {} ({})", path.display(), e))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::parser::SandboxArgs;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_file(dir: &TempDir, name: &str, content: &str) -> PathBuf {
        let file_path = dir.path().join(name);
        fs::write(&file_path, content).unwrap();
        file_path
    }

    #[test]
    fn test_is_likely_file_path() {
        // File paths with separators
        assert!(is_likely_file_path("path/to/file"));
        assert!(is_likely_file_path("./file.txt"));
        assert!(is_likely_file_path("../file.md"));

        // Common file extensions
        assert!(is_likely_file_path("prompt.txt"));
        assert!(is_likely_file_path("requirements.md"));
        assert!(is_likely_file_path("task.prompt"));
        assert!(is_likely_file_path("template.tmpl"));

        // URLs should not be file paths
        assert!(!is_likely_file_path("http://example.com"));
        assert!(!is_likely_file_path("https://github.com/user/repo"));
        assert!(!is_likely_file_path("ftp://server.com"));

        // Text with URLs should not be file paths
        assert!(!is_likely_file_path(
            "Check out https://example.com for more info"
        ));
        assert!(!is_likely_file_path("Visit http://test.com or see docs"));

        // Regular prompts should not be file paths
        assert!(!is_likely_file_path("implement user authentication"));
        assert!(!is_likely_file_path("add login form"));
        assert!(!is_likely_file_path(""));
    }

    #[test]
    fn test_resolve_prompt_and_session_inline_prompt() {
        let args = DispatchArgs {
            name_or_prompt: Some("implement user auth".to_string()),
            prompt: None,
            file: None,
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
                sandbox_no_network: false,
                allowed_domains: vec![],
            },
        };

        let result = args.resolve_prompt_and_session_no_stdin().unwrap();
        assert_eq!(result.0, None); // No session name
        assert_eq!(result.1, "implement user auth"); // Prompt content
    }

    #[test]
    fn test_resolve_prompt_and_session_with_session_name() {
        let args = DispatchArgs {
            name_or_prompt: Some("auth-feature".to_string()),
            prompt: Some("implement user authentication".to_string()),
            file: None,
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
                sandbox_no_network: false,
                allowed_domains: vec![],
            },
        };

        let result = args.resolve_prompt_and_session_no_stdin().unwrap();
        assert_eq!(result.0, Some("auth-feature".to_string())); // Session name
        assert_eq!(result.1, "implement user authentication"); // Prompt content
    }

    #[test]
    fn test_resolve_prompt_and_session_file_flag() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = create_test_file(&temp_dir, "prompt.txt", "implement user auth from file");

        let args = DispatchArgs {
            name_or_prompt: Some("my-session".to_string()),
            prompt: None,
            file: Some(file_path),
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
                sandbox_no_network: false,
                allowed_domains: vec![],
            },
        };

        let result = args.resolve_prompt_and_session_no_stdin().unwrap();
        assert_eq!(result.0, Some("my-session".to_string())); // Session name
        assert_eq!(result.1, "implement user auth from file"); // File content
    }

    #[test]
    fn test_resolve_prompt_and_session_auto_detect_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = create_test_file(&temp_dir, "task.md", "auto-detected file content");
        let file_path_str = file_path.to_string_lossy().to_string();

        let args = DispatchArgs {
            name_or_prompt: Some(file_path_str),
            prompt: None,
            file: None,
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
                sandbox_no_network: false,
                allowed_domains: vec![],
            },
        };

        let result = args.resolve_prompt_and_session_no_stdin().unwrap();
        assert_eq!(result.0, None); // No session name
        assert_eq!(result.1, "auto-detected file content"); // File content
    }

    #[test]
    fn test_resolve_prompt_and_session_session_with_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = create_test_file(&temp_dir, "spec.txt", "session with file content");
        let file_path_str = file_path.to_string_lossy().to_string();

        let args = DispatchArgs {
            name_or_prompt: Some("feature-branch".to_string()),
            prompt: Some(file_path_str),
            file: None,
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
                sandbox_no_network: false,
                allowed_domains: vec![],
            },
        };

        let result = args.resolve_prompt_and_session_no_stdin().unwrap();
        assert_eq!(result.0, Some("feature-branch".to_string())); // Session name
        assert_eq!(result.1, "session with file content"); // File content
    }

    #[test]
    fn test_resolve_prompt_and_session_empty_file_error() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = create_test_file(&temp_dir, "empty.txt", "");

        let args = DispatchArgs {
            name_or_prompt: None,
            prompt: None,
            file: Some(file_path),
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
                sandbox_no_network: false,
                allowed_domains: vec![],
            },
        };

        let result = args.resolve_prompt_and_session_no_stdin();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("file is empty"));
    }

    #[test]
    fn test_resolve_prompt_and_session_no_args_error() {
        let args = DispatchArgs {
            name_or_prompt: None,
            prompt: None,
            file: None,
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
                sandbox_no_network: false,
                allowed_domains: vec![],
            },
        };

        let result = args.resolve_prompt_and_session_no_stdin();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("dispatch requires a prompt text or file path"));
    }

    #[test]
    fn test_read_file_content_missing_file() {
        let result = read_file_content(Path::new("nonexistent.txt"));
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("not found")
                || err_msg.contains("No such file")
                || err_msg.contains("does not exist")
        );
    }

    #[test]
    fn test_resolve_prompt_with_file_should_ignore_stdin() {
        // This test simulates the MCP environment issue where stdin is not a terminal
        // but we're using --file, which should bypass stdin detection entirely
        let temp_dir = TempDir::new().unwrap();
        let file_path = create_test_file(&temp_dir, "task.md", "task from file");

        let args = DispatchArgs {
            name_or_prompt: Some("test-session".to_string()),
            prompt: None,
            file: Some(file_path),
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
                sandbox_no_network: false,
                allowed_domains: vec![],
            },
        };

        // The resolve_prompt_and_session method checks stdin, but when --file is provided
        // it should skip stdin detection and use the file directly
        // Currently this would fail in MCP environment with "Piped input is empty"
        let result = args.resolve_prompt_and_session();
        assert!(
            result.is_ok(),
            "Should succeed even when stdin is not a terminal"
        );
        let (session, prompt) = result.unwrap();
        assert_eq!(session, Some("test-session".to_string()));
        assert_eq!(prompt, "task from file");
    }

    #[test]
    fn test_resolve_prompt_with_inline_text_no_stdin() {
        // Test that inline text works correctly using the no_stdin method directly
        let args = DispatchArgs {
            name_or_prompt: Some("implement feature".to_string()),
            prompt: None,
            file: None,
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
                sandbox_no_network: false,
                allowed_domains: vec![],
            },
        };

        // Test the no_stdin method directly to avoid stdin detection issues in tests
        let result = args.resolve_prompt_and_session_no_stdin();
        assert!(result.is_ok());
        let (session, prompt) = result.unwrap();
        assert_eq!(session, None);
        assert_eq!(prompt, "implement feature");
    }

    #[test]
    fn test_resolve_prompt_should_prioritize_explicit_args_over_stdin() {
        // This test demonstrates the stdin detection logic issue:
        // When we have explicit arguments (like name_or_prompt), we should use them
        // instead of trying to read from stdin, even if stdin is not a terminal.
        //
        // Current problematic logic:
        // 1. Check if file flag -> use file (CORRECT)
        // 2. Check if stdin is not terminal -> try to read stdin (PROBLEM)
        // 3. Fall back to explicit args (TOO LATE)
        //
        // Better logic would be:
        // 1. Check if file flag -> use file
        // 2. Check if we have explicit args -> use them
        // 3. Check if stdin has content -> use stdin
        // 4. Error: no input provided

        let args = DispatchArgs {
            name_or_prompt: Some("implement authentication".to_string()),
            prompt: None,
            file: None,
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
                sandbox_no_network: false,
                allowed_domains: vec![],
            },
        };

        // This should work with explicit args regardless of stdin status
        let result_no_stdin = args.resolve_prompt_and_session_no_stdin();
        assert!(result_no_stdin.is_ok());
        let (session, prompt) = result_no_stdin.unwrap();
        assert_eq!(session, None);
        assert_eq!(prompt, "implement authentication");

        // The issue: resolve_prompt_and_session() might fail in non-terminal environments
        // even when we have valid explicit arguments, because it checks stdin first
    }

    #[test]
    fn test_dispatch_logic_priority_order() {
        // Test that dispatch resolves arguments in the correct priority order:
        // 1. File flag (highest priority)
        // 2. Explicit arguments
        // 3. Stdin input (lowest priority)

        let temp_dir = TempDir::new().unwrap();
        let file_path = create_test_file(&temp_dir, "priority.txt", "file content");

        // Test 1: File flag should override everything
        let args_with_file = DispatchArgs {
            name_or_prompt: Some("session-name".to_string()),
            prompt: Some("explicit prompt".to_string()),
            file: Some(file_path), // Should take priority
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
                sandbox_no_network: false,
                allowed_domains: vec![],
            },
        };

        let result = args_with_file
            .resolve_prompt_and_session_no_stdin()
            .unwrap();
        assert_eq!(result.0, Some("session-name".to_string()));
        assert_eq!(result.1, "file content"); // File content wins

        // Test 2: Explicit args should work when no file
        let args_explicit = DispatchArgs {
            name_or_prompt: Some("explicit prompt text".to_string()),
            prompt: None,
            file: None,
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
                sandbox_no_network: false,
                allowed_domains: vec![],
            },
        };

        let result = args_explicit.resolve_prompt_and_session_no_stdin().unwrap();
        assert_eq!(result.0, None);
        assert_eq!(result.1, "explicit prompt text"); // Explicit args work
    }

    #[test]
    fn test_explicit_args_should_take_priority_over_stdin() {
        // Test that explicit arguments are used even when stdin might be available
        // This is the correct behavior - explicit args should have higher priority

        let args = DispatchArgs {
            name_or_prompt: Some("explicit prompt".to_string()),
            prompt: None,
            file: None,
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
                sandbox_no_network: false,
                allowed_domains: vec![],
            },
        };

        // The current implementation has a logical flaw:
        // It checks stdin before checking explicit arguments
        // This test verifies the fix where explicit args take priority

        let result = args.resolve_prompt_and_session();
        assert!(
            result.is_ok(),
            "Should succeed with explicit args regardless of stdin"
        );

        let (session, prompt) = result.unwrap();
        assert_eq!(session, None);
        assert_eq!(prompt, "explicit prompt");
    }

    #[test]
    fn test_read_file_content_success() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = create_test_file(&temp_dir, "test.txt", "test content");

        let result = read_file_content(&file_path).unwrap();
        assert_eq!(result, "test content");
    }

    #[test]
    fn test_file_extension_detection() {
        // Test all supported extensions
        assert!(is_likely_file_path("file.txt"));
        assert!(is_likely_file_path("file.md"));
        assert!(is_likely_file_path("file.rst"));
        assert!(is_likely_file_path("file.org"));
        assert!(is_likely_file_path("file.prompt"));
        assert!(is_likely_file_path("file.tmpl"));
        assert!(is_likely_file_path("file.template"));

        // Test unsupported extensions
        assert!(!is_likely_file_path("file.jpg"));
        assert!(!is_likely_file_path("file.pdf"));
        assert!(!is_likely_file_path("file.exe"));
    }

    #[test]
    fn test_validate_claude_code_ide_rejects_claude_without_wrapper() {
        let config = crate::config::Config {
            ide: crate::config::IdeConfig {
                name: "claude".to_string(),
                command: "claude".to_string(),
                user_data_dir: None,
                wrapper: crate::config::WrapperConfig {
                    enabled: false,
                    name: "".to_string(),
                    command: "".to_string(),
                },
            },
            directories: crate::config::defaults::default_directory_config(),
            git: crate::config::defaults::default_git_config(),
            session: crate::config::defaults::default_session_config(),
            docker: None,
            setup_script: None,
            sandbox: None,
        };

        let result = validate_claude_code_ide(&config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("wrapper mode"));
    }

    #[test]
    fn test_validate_claude_code_ide_rejects_claude_code_without_wrapper() {
        let config = crate::config::Config {
            ide: crate::config::IdeConfig {
                name: "claude-code".to_string(),
                command: "claude-code".to_string(),
                user_data_dir: None,
                wrapper: crate::config::WrapperConfig {
                    enabled: false,
                    name: "".to_string(),
                    command: "".to_string(),
                },
            },
            directories: crate::config::defaults::default_directory_config(),
            git: crate::config::defaults::default_git_config(),
            session: crate::config::defaults::default_session_config(),
            docker: None,
            setup_script: None,
            sandbox: None,
        };

        let result = validate_claude_code_ide(&config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("wrapper mode"));
    }

    #[test]
    fn test_validate_claude_code_ide_accepts_wrapper_mode() {
        let config = crate::config::Config {
            ide: crate::config::IdeConfig {
                name: "cursor".to_string(),
                command: "claude".to_string(), // Using claude command in wrapper mode
                user_data_dir: None,
                wrapper: crate::config::WrapperConfig {
                    enabled: true,
                    name: "cursor".to_string(),
                    command: "cursor".to_string(),
                },
            },
            directories: crate::config::defaults::default_directory_config(),
            git: crate::config::defaults::default_git_config(),
            session: crate::config::defaults::default_session_config(),
            docker: None,
            setup_script: None,
            sandbox: None,
        };

        let result = validate_claude_code_ide(&config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_claude_code_ide_rejects_cursor() {
        let config = crate::config::Config {
            ide: crate::config::IdeConfig {
                name: "cursor".to_string(),
                command: "cursor".to_string(),
                user_data_dir: None,
                wrapper: crate::config::WrapperConfig {
                    enabled: false,
                    name: "".to_string(),
                    command: "".to_string(),
                },
            },
            directories: crate::config::defaults::default_directory_config(),
            git: crate::config::defaults::default_git_config(),
            session: crate::config::defaults::default_session_config(),
            docker: None,
            setup_script: None,
            sandbox: None,
        };

        let result = validate_claude_code_ide(&config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("wrapper mode"));
    }

    #[test]
    fn test_validate_claude_code_ide_rejects_vscode() {
        let config = crate::config::Config {
            ide: crate::config::IdeConfig {
                name: "code".to_string(),
                command: "code".to_string(),
                user_data_dir: None,
                wrapper: crate::config::WrapperConfig {
                    enabled: false,
                    name: "".to_string(),
                    command: "".to_string(),
                },
            },
            directories: crate::config::defaults::default_directory_config(),
            git: crate::config::defaults::default_git_config(),
            session: crate::config::defaults::default_session_config(),
            docker: None,
            setup_script: None,
            sandbox: None,
        };

        let result = validate_claude_code_ide(&config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("wrapper mode"));
    }

    #[test]
    fn test_create_claude_local_md() {
        let temp_dir = TempDir::new().unwrap();
        let session_path = temp_dir.path().join("session-worktree");
        std::fs::create_dir_all(&session_path).unwrap();

        let session_name = "test-auth-session";
        let result = create_claude_local_md(&session_path, session_name);
        assert!(result.is_ok());

        // Verify file was created
        let claude_local_path = session_path.join("CLAUDE.local.md");
        assert!(claude_local_path.exists());

        // Verify content
        let content = std::fs::read_to_string(&claude_local_path).unwrap();

        // Check session name is included
        assert!(content.contains(session_name));

        // Check required content sections
        assert!(content.contains("Para Session Status Commands"));
        assert!(content.contains("Required status updates:"));
        assert!(content.contains("--tests"));
        assert!(content.contains("Test Status Guidelines:"));
        assert!(content.contains("para finish"));

        // Check specific command examples
        assert!(content.contains("para status \"Starting"));
        assert!(content.contains("--tests"));
        assert!(content.contains("--blocked"));
        assert!(content.contains("--todos"));

        // Check guidelines
        assert!(content.contains("ALL tests in the entire codebase"));
        assert!(content.contains("NEVER report partial test results"));
        assert!(content.contains("Run full test suite"));

        // Check it contains the DO NOT COMMIT warning
        assert!(content.contains("DO NOT COMMIT"));
    }

    #[test]
    fn test_create_claude_local_md_overwrites_existing() {
        let temp_dir = TempDir::new().unwrap();
        let session_path = temp_dir.path().join("session-worktree");
        std::fs::create_dir_all(&session_path).unwrap();

        let claude_local_path = session_path.join("CLAUDE.local.md");

        // Create existing file with different content
        std::fs::write(&claude_local_path, "old content").unwrap();
        assert_eq!(
            std::fs::read_to_string(&claude_local_path).unwrap(),
            "old content"
        );

        // Create new CLAUDE.local.md
        let session_name = "overwrite-test";
        let result = create_claude_local_md(&session_path, session_name);
        assert!(result.is_ok());

        // Verify content was overwritten
        let content = std::fs::read_to_string(&claude_local_path).unwrap();
        assert!(content.contains(session_name));
        assert!(content.contains("Para Session Status Commands"));
        assert!(!content.contains("old content"));
    }

    #[test]
    fn test_create_claude_local_md_creates_directory() {
        let temp_dir = TempDir::new().unwrap();
        let session_path = temp_dir.path().join("deep").join("nested").join("session");
        // Don't create directory - let function handle it

        let session_name = "nested-session";
        let result = create_claude_local_md(&session_path, session_name);

        // Should fail because parent directory doesn't exist and we don't create it
        assert!(result.is_err());

        // Now create the directory and try again
        std::fs::create_dir_all(&session_path).unwrap();
        let result = create_claude_local_md(&session_path, session_name);
        assert!(result.is_ok());

        let claude_local_path = session_path.join("CLAUDE.local.md");
        assert!(claude_local_path.exists());
    }

    #[test]
    fn test_create_claude_local_md_special_characters_in_session_name() {
        let temp_dir = TempDir::new().unwrap();
        let session_path = temp_dir.path().join("session-worktree");
        std::fs::create_dir_all(&session_path).unwrap();

        // Test with session names containing special characters
        let session_names = vec![
            "session-with-dashes",
            "session_with_underscores",
            "session with spaces",
            "session.with.dots",
            "session/with/slashes",
            "session@with@symbols",
        ];

        for session_name in session_names {
            let result = create_claude_local_md(&session_path, session_name);
            assert!(result.is_ok(), "Failed for session name: {session_name}");

            let content = std::fs::read_to_string(session_path.join("CLAUDE.local.md")).unwrap();
            assert!(
                content.contains(session_name),
                "Session name not found in content for: {session_name}"
            );
        }
    }

    #[test]
    fn test_setup_script_priority_cli_arg() {
        let temp_dir = TempDir::new().unwrap();
        let repo_root = temp_dir.path();

        // Create script files
        let cli_script = repo_root.join("cli-script.sh");
        let default_script = repo_root.join(".para/setup.sh");
        fs::write(&cli_script, "#!/bin/bash\necho 'cli'").unwrap();
        fs::create_dir_all(repo_root.join(".para")).unwrap();
        fs::write(&default_script, "#!/bin/bash\necho 'default'").unwrap();

        let mut config = crate::config::defaults::default_config();
        config.docker = Some(crate::config::DockerConfig {
            setup_script: Some("config-script.sh".to_string()),
            default_image: None,
            forward_env_keys: None,
        });

        // CLI arg should take priority
        let result = get_setup_script_path(&Some(cli_script.clone()), repo_root, &config, false);
        assert_eq!(result, Some(cli_script));
    }

    #[test]
    fn test_setup_script_priority_default() {
        let temp_dir = TempDir::new().unwrap();
        let repo_root = temp_dir.path();

        // Create default script
        let default_script = repo_root.join(".para/setup.sh");
        fs::create_dir_all(repo_root.join(".para")).unwrap();
        fs::write(&default_script, "#!/bin/bash\necho 'default'").unwrap();

        let mut config = crate::config::defaults::default_config();
        config.docker = Some(crate::config::DockerConfig {
            setup_script: Some("config-script.sh".to_string()),
            default_image: None,
            forward_env_keys: None,
        });

        // Default script should be found when no CLI arg
        let result = get_setup_script_path(&None, repo_root, &config, false);
        assert_eq!(result, Some(default_script));
    }

    #[test]
    fn test_setup_script_priority_config() {
        let temp_dir = TempDir::new().unwrap();
        let repo_root = temp_dir.path();

        // Create config script
        let config_script = repo_root.join("scripts/config-script.sh");
        fs::create_dir_all(repo_root.join("scripts")).unwrap();
        fs::write(&config_script, "#!/bin/bash\necho 'config'").unwrap();

        let mut config = crate::config::defaults::default_config();
        config.docker = Some(crate::config::DockerConfig {
            setup_script: Some("scripts/config-script.sh".to_string()),
            default_image: None,
            forward_env_keys: None,
        });

        // Config script should be found when no CLI arg or default
        let result = get_setup_script_path(&None, repo_root, &config, true);
        assert_eq!(result, Some(config_script));
    }

    #[test]
    fn test_setup_script_absolute_path_in_config() {
        let temp_dir = TempDir::new().unwrap();
        let repo_root = temp_dir.path();

        // Create script with absolute path
        let abs_script = temp_dir.path().join("absolute-script.sh");
        fs::write(&abs_script, "#!/bin/bash\necho 'absolute'").unwrap();

        let mut config = crate::config::defaults::default_config();
        config.docker = Some(crate::config::DockerConfig {
            setup_script: Some(abs_script.to_string_lossy().to_string()),
            default_image: None,
            forward_env_keys: None,
        });

        // Absolute path in config should work
        let result = get_setup_script_path(&None, repo_root, &config, true);
        assert_eq!(result, Some(abs_script));
    }

    #[test]
    fn test_setup_script_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let repo_root = temp_dir.path();

        let config = crate::config::defaults::default_config();

        // No script should be found
        let result = get_setup_script_path(&None, repo_root, &config, false);
        assert_eq!(result, None);
    }

    #[test]
    fn test_setup_script_cli_arg_not_exists() {
        let temp_dir = TempDir::new().unwrap();
        let repo_root = temp_dir.path();

        let non_existent = PathBuf::from("/non/existent/script.sh");
        let config = crate::config::defaults::default_config();

        // Should return None and print warning
        let result = get_setup_script_path(&Some(non_existent), repo_root, &config, false);
        assert_eq!(result, None);
    }
}
