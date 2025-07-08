use crate::cli::parser::UnifiedStartArgs;
use crate::config::Config;
use crate::core::git::GitService;
use crate::core::session::SessionManager;
use crate::utils::{ParaError, Result};
use std::path::PathBuf;

/// Represents the user's intent when using the start command
#[derive(Debug, Clone)]
pub enum StartIntent {
    /// Create new interactive session (no prompt)
    NewInteractive { name: Option<String> },
    /// Create new session with AI agent
    NewWithAgent {
        name: Option<String>,
        prompt: String,
    },
}

/// Analyzes the provided arguments to determine user intent
pub fn determine_intent(
    args: &UnifiedStartArgs,
    session_manager: &SessionManager,
) -> Result<StartIntent> {
    // Check if session already exists
    if let Some(ref name) = args.name {
        if session_manager.session_exists(name) {
            let suggestion = if args.prompt.is_some() || args.file.is_some() {
                format!("Use 'para resume {name}' with --prompt or --file to continue with additional context.")
            } else {
                format!("Use 'para resume {name}' to continue existing session.")
            };
            return Err(ParaError::invalid_args(format!(
                "Session '{name}' already exists. {suggestion}"
            )));
        }
    }

    // Resolve prompt content from various sources
    let prompt_content = resolve_prompt_content(args)?;

    match prompt_content {
        Some(prompt) => {
            // AI-assisted session
            Ok(StartIntent::NewWithAgent {
                name: args.name.clone(),
                prompt,
            })
        }
        None => {
            // Interactive session
            Ok(StartIntent::NewInteractive {
                name: args.name.clone(),
            })
        }
    }
}

/// Resolve prompt content from various sources (inline, file, stdin)
fn resolve_prompt_content(args: &UnifiedStartArgs) -> Result<Option<String>> {
    // Priority order:
    // 1. --file flag (highest priority)
    // 2. Inline prompt argument
    // 3. Stdin (if available and no other input)

    if let Some(file_path) = &args.file {
        let content = read_prompt_file(file_path)?;
        return Ok(Some(content));
    }

    if let Some(prompt) = &args.prompt {
        return Ok(Some(prompt.clone()));
    }

    // Check for stdin input only if no other input provided
    if args.name.is_none()
        && args.prompt.is_none()
        && !std::io::IsTerminal::is_terminal(&std::io::stdin())
    {
        use std::io::Read;
        let mut buffer = String::new();
        std::io::stdin()
            .read_to_string(&mut buffer)
            .map_err(|e| ParaError::file_operation(format!("Failed to read from stdin: {e}")))?;

        if !buffer.trim().is_empty() {
            return Ok(Some(buffer));
        }
    }

    Ok(None)
}

/// Read prompt content from a file
fn read_prompt_file(path: &PathBuf) -> Result<String> {
    let absolute_path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|e| ParaError::fs_error(format!("Failed to get current directory: {e}")))?
            .join(path)
    };

    if !absolute_path.exists() {
        return Err(ParaError::file_not_found(format!(
            "Prompt file not found: {}",
            path.display()
        )));
    }

    let content = std::fs::read_to_string(&absolute_path)
        .map_err(|e| ParaError::file_operation(format!("Failed to read file: {e}")))?;

    if content.trim().is_empty() {
        return Err(ParaError::file_operation(format!(
            "Prompt file is empty: {}",
            path.display()
        )));
    }

    Ok(content)
}

/// Main entry point for unified start command
pub fn execute(config: Config, args: UnifiedStartArgs) -> Result<()> {
    args.validate()?;

    let _git_service = GitService::discover()
        .map_err(|e| ParaError::git_error(format!("Failed to discover git repository: {e}")))?;
    let session_manager = SessionManager::new(&config);

    let intent = determine_intent(&args, &session_manager)?;

    match intent {
        StartIntent::NewInteractive { name } => create_interactive_session(config, args, name),
        StartIntent::NewWithAgent { name, prompt } => {
            create_agent_session(config, args, name, prompt)
        }
    }
}

/// Create a new interactive session (equivalent to old 'start' command)
fn create_interactive_session(
    config: Config,
    args: UnifiedStartArgs,
    name: Option<String>,
) -> Result<()> {
    // Delegate to existing start command for backward compatibility
    let start_args = args.to_start_args(name);
    crate::cli::commands::start::execute(config, start_args)
}

/// Create a new session with an AI agent (equivalent to old 'dispatch' command)
fn create_agent_session(
    config: Config,
    args: UnifiedStartArgs,
    name: Option<String>,
    prompt: String,
) -> Result<()> {
    // Validate Claude Code IDE requirement for dispatch
    validate_claude_code_ide(&config)?;

    // Delegate to existing dispatch command for agent functionality
    // When we have a file, don't pass the prompt content as it will be resolved from the file
    let dispatch_args = if args.file.is_some() {
        args.to_dispatch_args(name, None)
    } else {
        args.to_dispatch_args(name, Some(prompt))
    };
    crate::cli::commands::dispatch::execute(config, dispatch_args)
}

/// Validate that Claude Code is configured in wrapper mode (required for dispatch)
fn validate_claude_code_ide(config: &Config) -> Result<()> {
    if (config.ide.command.to_lowercase() == "claude"
        || config.ide.command.to_lowercase() == "claude-code")
        && config.is_wrapper_enabled()
    {
        return Ok(());
    }

    Err(ParaError::invalid_config(format!(
        "AI agent sessions require Claude Code in wrapper mode. Current IDE: '{}' with command: '{}', wrapper enabled: {}. Run 'para config' to configure Claude Code with wrapper mode.",
        config.ide.name, config.ide.command, config.is_wrapper_enabled()
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::parser::SandboxArgs;
    use crate::core::session::{SessionState, SessionType};
    use crate::test_utils::test_helpers::*;
    use tempfile::TempDir;

    fn create_test_args() -> UnifiedStartArgs {
        UnifiedStartArgs {
            name: None,
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
        }
    }

    #[test]
    fn test_determine_intent_new_interactive_no_args() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config_with_dir(&temp_dir);
        let session_manager = SessionManager::new(&config);

        let args = create_test_args();

        let intent = determine_intent(&args, &session_manager).unwrap();
        match intent {
            StartIntent::NewInteractive { name } => assert_eq!(name, None),
            _ => panic!("Expected NewInteractive intent"),
        }
    }

    #[test]
    fn test_determine_intent_new_interactive_with_name() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config_with_dir(&temp_dir);
        let session_manager = SessionManager::new(&config);

        let mut args = create_test_args();
        args.name = Some("new-feature".to_string());

        let intent = determine_intent(&args, &session_manager).unwrap();
        match intent {
            StartIntent::NewInteractive { name } => {
                assert_eq!(name, Some("new-feature".to_string()))
            }
            _ => panic!("Expected NewInteractive intent"),
        }
    }

    #[test]
    fn test_determine_intent_new_with_agent() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config_with_dir(&temp_dir);
        let session_manager = SessionManager::new(&config);

        let mut args = create_test_args();
        args.prompt = Some("implement feature X".to_string());

        let intent = determine_intent(&args, &session_manager).unwrap();
        match intent {
            StartIntent::NewWithAgent { name, prompt } => {
                assert_eq!(name, None);
                assert_eq!(prompt, "implement feature X");
            }
            _ => panic!("Expected NewWithAgent intent"),
        }
    }

    #[test]
    fn test_determine_intent_new_with_agent_and_name() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config_with_dir(&temp_dir);
        let session_manager = SessionManager::new(&config);

        let mut args = create_test_args();
        args.name = Some("feature-x".to_string());
        args.prompt = Some("implement feature X".to_string());

        let intent = determine_intent(&args, &session_manager).unwrap();
        match intent {
            StartIntent::NewWithAgent { name, prompt } => {
                assert_eq!(name, Some("feature-x".to_string()));
                assert_eq!(prompt, "implement feature X");
            }
            _ => panic!("Expected NewWithAgent intent"),
        }
    }

    #[test]
    fn test_resolve_prompt_from_file() {
        let temp_dir = TempDir::new().unwrap();
        let prompt_file = temp_dir.path().join("prompt.txt");
        std::fs::write(&prompt_file, "Test prompt from file").unwrap();

        let mut args = create_test_args();
        args.file = Some(prompt_file);

        let prompt = resolve_prompt_content(&args).unwrap();
        assert_eq!(prompt, Some("Test prompt from file".to_string()));
    }

    #[test]
    fn test_resolve_prompt_priority_file_over_inline() {
        let temp_dir = TempDir::new().unwrap();
        let prompt_file = temp_dir.path().join("prompt.txt");
        std::fs::write(&prompt_file, "File content").unwrap();

        let mut args = create_test_args();
        args.file = Some(prompt_file);
        args.prompt = Some("Inline content".to_string());

        let prompt = resolve_prompt_content(&args).unwrap();
        assert_eq!(prompt, Some("File content".to_string()));
    }

    #[test]
    fn test_resolve_prompt_empty_file_error() {
        let temp_dir = TempDir::new().unwrap();
        let prompt_file = temp_dir.path().join("empty.txt");
        std::fs::write(&prompt_file, "").unwrap();

        let mut args = create_test_args();
        args.file = Some(prompt_file);

        let result = resolve_prompt_content(&args);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("empty"));
    }

    #[test]
    fn test_determine_intent_existing_session_with_prompt_error() {
        let temp_dir = TempDir::new().unwrap();
        let mut config = create_test_config();
        config.directories.state_dir = temp_dir
            .path()
            .join(".para_state")
            .to_string_lossy()
            .to_string();

        // Pre-create state directory
        std::fs::create_dir_all(&config.directories.state_dir).unwrap();

        let session_manager = SessionManager::new(&config);

        // Create an existing session without git operations
        let session_state = SessionState {
            name: "existing-feature".to_string(),
            branch: "para/existing-feature".to_string(),
            worktree_path: temp_dir.path().join("existing-feature"),
            created_at: chrono::Utc::now(),
            status: crate::core::session::SessionStatus::Active,
            task_description: None,
            last_activity: None,
            git_stats: None,
            session_type: SessionType::Worktree,
            parent_branch: Some("main".to_string()),
            is_docker: None,
            dangerous_skip_permissions: None,
            sandbox_enabled: Some(false),
            sandbox_profile: None,
        };
        session_manager.save_state(&session_state).unwrap();

        let mut args = create_test_args();
        args.name = Some("existing-feature".to_string());
        args.prompt = Some("add error handling".to_string());

        let result = determine_intent(&args, &session_manager);
        assert!(result.is_err());
        let error_message = result.err().unwrap().to_string();
        assert!(error_message.contains("Session 'existing-feature' already exists"));
        assert!(error_message.contains("'para resume existing-feature' with --prompt"));
    }

    #[test]
    fn test_determine_intent_existing_session_error() {
        let temp_dir = TempDir::new().unwrap();
        let mut config = create_test_config();
        config.directories.state_dir = temp_dir
            .path()
            .join(".para_state")
            .to_string_lossy()
            .to_string();

        // Pre-create state directory
        std::fs::create_dir_all(&config.directories.state_dir).unwrap();

        let session_manager = SessionManager::new(&config);

        // Create an existing session without git operations
        let session_state = SessionState {
            name: "existing-work".to_string(),
            branch: "para/existing-work".to_string(),
            worktree_path: temp_dir.path().join("existing-work"),
            created_at: chrono::Utc::now(),
            status: crate::core::session::SessionStatus::Active,
            task_description: None,
            last_activity: None,
            git_stats: None,
            session_type: SessionType::Worktree,
            parent_branch: Some("main".to_string()),
            is_docker: None,
            dangerous_skip_permissions: None,
            sandbox_enabled: Some(false),
            sandbox_profile: None,
        };
        session_manager.save_state(&session_state).unwrap();

        let mut args = create_test_args();
        args.name = Some("existing-work".to_string());

        let result = determine_intent(&args, &session_manager);
        assert!(result.is_err());
        let error_message = result.err().unwrap().to_string();
        assert!(error_message.contains("Session 'existing-work' already exists"));
        assert!(error_message.contains("para resume existing-work"));
    }

    #[test]
    fn test_single_arg_with_spaces_should_be_treated_as_prompt() {
        let temp_dir = TempDir::new().unwrap();
        let git_temp = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let config = create_test_config();
        let _session_manager = SessionManager::new(&config);

        // This should now fail as an invalid session name
        let mut args = create_test_args();
        args.name = Some("please download golem.de".to_string());
        args.sandbox_args.sandbox_no_network = true;
        args.sandbox_args.allowed_domains = vec!["golem.de".to_string()];

        let result = args.validate();
        assert!(
            result.is_err(),
            "Should reject invalid session name with spaces"
        );
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Session name can only contain"));
    }

    #[test]
    fn test_prompt_flag_creates_agent_session() {
        let temp_dir = TempDir::new().unwrap();
        let git_temp = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let config = create_test_config();
        let session_manager = SessionManager::new(&config);

        // Using --prompt flag should create agent session
        let test_cases = vec![
            "implement the TODO items",
            "fix bug #123",
            "add feature: user auth",
            "what is 2+2?",
            "analyze website.com",
        ];

        for test_prompt in test_cases {
            let mut args = create_test_args();
            args.prompt = Some(test_prompt.to_string());

            let result = determine_intent(&args, &session_manager);
            assert!(result.is_ok(), "Failed for prompt: {test_prompt}");

            match result.unwrap() {
                StartIntent::NewWithAgent { name, prompt } => {
                    assert!(
                        name.is_none(),
                        "Should auto-generate name for: {test_prompt}"
                    );
                    assert_eq!(prompt, test_prompt);
                }
                other => panic!("Expected NewWithAgent for '{test_prompt}' but got {other:?}"),
            }
        }
    }

    #[test]
    fn test_valid_session_names_not_treated_as_prompts() {
        let temp_dir = TempDir::new().unwrap();
        let git_temp = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let config = create_test_config();
        let session_manager = SessionManager::new(&config);

        // Valid session names should NOT be treated as prompts
        let valid_names = vec![
            "feature-auth",
            "bugfix-123",
            "refactor_code",
            "test123",
            "NEW-FEATURE",
        ];

        for name in valid_names {
            let mut args = create_test_args();
            args.name = Some(name.to_string());

            let result = determine_intent(&args, &session_manager);
            assert!(result.is_ok(), "Failed for name: {name}");

            match result.unwrap() {
                StartIntent::NewInteractive { name: session_name } => {
                    assert_eq!(session_name, Some(name.to_string()));
                }
                other => panic!("Expected NewInteractive for '{name}' but got {other:?}"),
            }
        }
    }

    #[test]
    fn test_session_name_validation() {
        // Test that session names are properly validated
        let invalid_names = vec![
            "what?",       // question mark
            "test.py",     // dot
            "path\\file",  // backslash
            "bug:fix",     // colon
            "hello world", // space
            "test@email",  // at sign
            "feature#123", // hash
            "item,item",   // comma
            "code;",       // semicolon
            "'quoted'",    // quotes
            "\"quoted\"",  // quotes
            "fix/bug",     // slash
            "implement!",  // exclamation
        ];

        for name in invalid_names {
            let mut args = create_test_args();
            args.name = Some(name.to_string());

            let result = args.validate();
            assert!(result.is_err(), "'{name}' should be invalid session name");
            assert!(result
                .unwrap_err()
                .to_string()
                .contains("Session name can only contain"));
        }

        let valid_names = vec![
            "feature-auth",
            "bugfix-123",
            "refactor_code",
            "test123",
            "NEW-FEATURE",
            "fix",
            "implement",
            "2plus2",
        ];

        for name in valid_names {
            let mut args = create_test_args();
            args.name = Some(name.to_string());

            let result = args.validate();
            assert!(result.is_ok(), "'{name}' should be valid session name");
        }
    }

    #[test]
    fn test_file_takes_precedence_over_prompt() {
        let temp_dir = TempDir::new().unwrap();
        let prompt_file = temp_dir.path().join("prompt.txt");
        std::fs::write(&prompt_file, "File content takes precedence").unwrap();

        let mut args = create_test_args();
        args.file = Some(prompt_file);
        args.prompt = Some("This should be ignored".to_string());

        let prompt_content = resolve_prompt_content(&args).unwrap();
        assert_eq!(
            prompt_content,
            Some("File content takes precedence".to_string())
        );
    }

    #[test]
    fn test_deterministic_behavior_no_ambiguity() {
        let temp_dir = TempDir::new().unwrap();
        let config = create_test_config_with_dir(&temp_dir);
        let session_manager = SessionManager::new(&config);

        // Test cases that would have been ambiguous with heuristics
        let test_cases: Vec<(Option<&str>, Option<&str>, bool)> = vec![
            // These are now clearly session names (no prompt flag)
            (Some("what"), None, false),      // single word
            (Some("implement"), None, false), // could be seen as a prompt
            (Some("fix-bug"), None, false),   // valid session name
            // These are clearly prompts (with -p flag)
            (None, Some("what are you doing?"), true),
            (None, Some("implement feature"), true),
            (None, Some("fix bug"), true),
            // Named AI sessions
            (Some("my-task"), Some("what should I do?"), true),
            (Some("feature-x"), Some("implement"), true),
        ];

        for (name, prompt, should_be_ai) in test_cases {
            let mut args = create_test_args();
            args.name = name.map(String::from);
            args.prompt = prompt.map(String::from);

            let result = determine_intent(&args, &session_manager);
            assert!(
                result.is_ok(),
                "Failed for name={name:?}, prompt={prompt:?}"
            );

            match (result.unwrap(), should_be_ai) {
                (StartIntent::NewWithAgent { .. }, true) => {
                    // Expected AI session, got AI session - good
                }
                (StartIntent::NewInteractive { .. }, false) => {
                    // Expected interactive session, got interactive session - good
                }
                (intent, expected_ai) => {
                    panic!(
                        "For name={name:?}, prompt={prompt:?} expected AI={expected_ai} but got {intent:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn test_file_not_found_error() {
        let temp_dir = TempDir::new().unwrap();
        let nonexistent_file = temp_dir.path().join("does_not_exist.txt");

        let mut args = create_test_args();
        args.file = Some(nonexistent_file);

        let result = resolve_prompt_content(&args);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[test]
    fn test_both_prompt_and_file_allowed() {
        // Unlike the old validation, both prompt and file can be provided
        // File takes precedence
        let temp_dir = TempDir::new().unwrap();
        let prompt_file = temp_dir.path().join("test.txt");
        std::fs::write(&prompt_file, "File wins").unwrap();

        let mut args = create_test_args();
        args.prompt = Some("Prompt loses".to_string());
        args.file = Some(prompt_file);

        // Validation should pass
        assert!(args.validate().is_ok());

        // File should take precedence
        let content = resolve_prompt_content(&args).unwrap();
        assert_eq!(content, Some("File wins".to_string()));
    }
}
