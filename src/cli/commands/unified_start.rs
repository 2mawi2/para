use crate::cli::parser::UnifiedStartArgs;
use crate::config::Config;
use crate::core::git::GitService;
use crate::core::session::SessionManager;
use crate::utils::{validate_session_name, ParaError, Result};
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
    // Resolve prompt content from various sources
    let prompt_content = resolve_prompt_content(args)?;

    match (&args.name_or_session, prompt_content) {
        // No arguments: new interactive session
        (None, None) => Ok(StartIntent::NewInteractive { name: None }),

        // Name only: need to determine if it's a session name or prompt
        (Some(text), None) => {
            // Check if session already exists
            if session_manager.session_exists(text) {
                return Err(ParaError::invalid_args(format!(
                    "Session '{text}' already exists. Use 'para resume {text}' to continue existing session."
                )));
            }

            // Try to determine if this is a prompt or session name
            if looks_like_prompt(text) {
                // Treat as prompt with auto-generated name
                Ok(StartIntent::NewWithAgent {
                    name: None,
                    prompt: text.clone(),
                })
            } else {
                // Treat as session name
                Ok(StartIntent::NewInteractive {
                    name: Some(text.clone()),
                })
            }
        }

        // With prompt: create new session (error if name exists)
        (name_opt, Some(prompt)) => match name_opt {
            Some(name) if session_manager.session_exists(name) => {
                Err(ParaError::invalid_args(format!(
                    "Session '{name}' already exists. Use 'para resume {name} --prompt \"{prompt}\"' to continue with additional context."
                )))
            }
            Some(name) => Ok(StartIntent::NewWithAgent {
                name: Some(name.clone()),
                prompt,
            }),
            None => Ok(StartIntent::NewWithAgent { name: None, prompt }),
        },
    }
}

/// Heuristic to determine if a string looks more like a prompt than a session name
fn looks_like_prompt(text: &str) -> bool {
    // If it's a valid session name, it's probably meant as one
    if validate_session_name(text).is_ok() {
        return false;
    }

    // Otherwise, treat as prompt if it contains:
    // - Spaces (session names can't have spaces)
    // - Special characters that aren't allowed in session names
    // - Common prompt patterns
    text.contains(' ')
        || text.contains('#')
        || text.contains(':')
        || text.contains('?')
        || text.contains('.')
        || text.contains('+')
        || text.contains('!')
        || text.contains(',')
        || text.contains(';')
        || text.contains('\'')
        || text.contains('"')
        || text.contains('/')
        || text.contains('\\')
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
    if args.name_or_session.is_none() && !std::io::IsTerminal::is_terminal(&std::io::stdin()) {
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
            name_or_session: None,
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
        args.name_or_session = Some("new-feature".to_string());

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
        args.name_or_session = Some("feature-x".to_string());
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
        args.name_or_session = Some("existing-feature".to_string());
        args.prompt = Some("add error handling".to_string());

        let result = determine_intent(&args, &session_manager);
        assert!(result.is_err());
        let error_message = result.err().unwrap().to_string();
        assert!(error_message.contains("Session 'existing-feature' already exists"));
        assert!(error_message.contains("para resume existing-feature --prompt"));
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
        args.name_or_session = Some("existing-work".to_string());

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
        let session_manager = SessionManager::new(&config);

        // This should be treated as a prompt, not a session name
        let mut args = create_test_args();
        args.name_or_session = Some("please download golem.de".to_string());
        args.sandbox_args.sandbox_no_network = true;
        args.sandbox_args.allowed_domains = vec!["golem.de".to_string()];

        let result = determine_intent(&args, &session_manager);

        // With the looks_like_prompt heuristic, this should be treated as a prompt
        assert!(result.is_ok());
        match result.unwrap() {
            StartIntent::NewWithAgent { name, prompt } => {
                // It should treat the text as a prompt with auto-generated name
                assert!(name.is_none(), "Should auto-generate session name");
                assert_eq!(prompt, "please download golem.de");
            }
            other => panic!("Expected NewWithAgent but got {other:?}"),
        }
    }

    #[test]
    fn test_prompt_detection_with_special_characters() {
        let temp_dir = TempDir::new().unwrap();
        let git_temp = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let config = create_test_config();
        let session_manager = SessionManager::new(&config);

        // Text with special chars should be treated as prompt
        let test_cases = vec![
            "implement the TODO items",
            "fix bug #123",
            "add feature: user auth",
            "what is 2+2?",
            "analyze website.com",
        ];

        for test_prompt in test_cases {
            let mut args = create_test_args();
            args.name_or_session = Some(test_prompt.to_string());

            let result = determine_intent(&args, &session_manager);

            // With the looks_like_prompt heuristic, these should be treated as prompts
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
            args.name_or_session = Some(name.to_string());

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
    fn test_edge_cases_for_prompt_detection() {
        let temp_dir = TempDir::new().unwrap();
        let git_temp = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let config = create_test_config();
        let session_manager = SessionManager::new(&config);

        // Edge cases - some are valid session names, some are prompts
        let edge_cases = vec![
            // Invalid session names -> treated as prompts
            ("what?", true),      // question mark (invalid char)
            ("test.py", true),    // dot (invalid char)
            ("path\\file", true), // backslash (invalid char)
            ("bug:fix", true),    // colon (invalid char)
            // Valid session names -> NOT treated as prompts
            ("implement!", false),  // exclamation mark is allowed
            ("2+2", false),         // plus sign is allowed
            ("fix/bug", false),     // slash is allowed
            ("item,item", false),   // comma is allowed
            ("code;", false),       // semicolon is allowed
            ("'quoted'", false),    // single quotes are allowed
            ("\"quoted\"", false),  // double quotes are allowed
            ("feature#123", false), // hash is allowed
        ];

        for (text, should_be_prompt) in edge_cases {
            let mut args = create_test_args();
            args.name_or_session = Some(text.to_string());

            let result = determine_intent(&args, &session_manager);
            assert!(result.is_ok(), "Failed for text: {text}");

            match (result.unwrap(), should_be_prompt) {
                (StartIntent::NewWithAgent { .. }, true) => {
                    // Expected prompt, got prompt - good
                }
                (StartIntent::NewInteractive { .. }, false) => {
                    // Expected session name, got session name - good
                }
                (intent, expected_prompt) => {
                    panic!("For '{text}' expected prompt={expected_prompt} but got {intent:?}");
                }
            }
        }
    }
}
