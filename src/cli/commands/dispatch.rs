use crate::cli::parser::DispatchArgs;
use crate::config::Config;
use crate::core::git::{GitOperations, GitService};
use crate::core::session::{SessionManager, SessionState};
use crate::utils::{names::*, ParaError, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn execute(config: Config, args: DispatchArgs) -> Result<()> {
    args.validate()?;

    let (session_name, prompt) = args.resolve_prompt_and_session()?;

    validate_claude_code_ide(&config)?;

    let git_service = GitService::discover()
        .map_err(|e| ParaError::git_error(format!("Failed to discover git repository: {}", e)))?;
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

    let subtrees_path = repo_root.join(&config.directories.subtrees_dir);
    let session_path = subtrees_path.join(&session_id);

    if !subtrees_path.exists() {
        fs::create_dir_all(&subtrees_path).map_err(|e| {
            ParaError::fs_error(format!("Failed to create subtrees directory: {}", e))
        })?;
    }

    git_service
        .create_worktree(&branch_name, &session_path)
        .map_err(|e| ParaError::git_error(format!("Failed to create worktree: {}", e)))?;

    let mut session_state =
        SessionState::new(session_id.clone(), branch_name, session_path.clone());

    // Save task description
    session_state.task_description = Some(prompt.clone());
    session_manager.save_state(&session_state)?;

    // Also save task to a separate file for backward compatibility
    let state_dir = Path::new(&config.directories.state_dir);
    let task_file = state_dir.join(format!("{}.task", session_id));
    fs::write(&task_file, &prompt)
        .map_err(|e| ParaError::fs_error(format!("Failed to write task file: {}", e)))?;

    // Create CLAUDE.local.md with status instructions
    create_claude_local_md(&session_state.worktree_path, &session_state.name)?;

    launch_claude_code(
        &config,
        &session_state.worktree_path,
        &prompt,
        args.dangerously_skip_permissions,
    )?;

    println!("Created session '{}' with Claude Code", session_state.name);
    println!("Session path: {}", session_state.worktree_path.display());

    Ok(())
}

fn validate_claude_code_ide(config: &Config) -> Result<()> {
    // Dispatch only works with Claude Code in wrapper mode
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
) -> Result<()> {
    // Create temporary prompt file (always, for consistency)
    let temp_prompt_file = session_path.join(".claude_prompt_temp");
    if !prompt.is_empty() {
        fs::write(&temp_prompt_file, prompt)
            .map_err(|e| ParaError::fs_error(format!("Failed to write temp prompt file: {}", e)))?;
    }

    // Always launch Claude in IDE mode
    launch_claude_in_ide(config, session_path, &temp_prompt_file, skip_permissions)
}

fn launch_claude_in_ide(
    config: &Config,
    session_path: &Path,
    temp_prompt_file: &Path,
    skip_permissions: bool,
) -> Result<()> {
    // Create .vscode directory
    let vscode_dir = session_path.join(".vscode");
    fs::create_dir_all(&vscode_dir)
        .map_err(|e| ParaError::fs_error(format!("Failed to create .vscode directory: {}", e)))?;

    // Build Claude command for tasks.json
    let mut base_cmd = config.ide.command.clone();
    if skip_permissions {
        base_cmd.push_str(" --dangerously-skip-permissions");
    }

    let claude_task_cmd = if temp_prompt_file.exists() {
        format!(
            "{} \"$(cat '{}'; rm '{}')\"",
            base_cmd,
            temp_prompt_file.display(),
            temp_prompt_file.display()
        )
    } else {
        base_cmd
    };

    // Create tasks.json
    let tasks_json = create_claude_task_json(&claude_task_cmd);
    let tasks_file = vscode_dir.join("tasks.json");
    fs::write(&tasks_file, tasks_json)
        .map_err(|e| ParaError::fs_error(format!("Failed to write tasks.json: {}", e)))?;

    // Launch wrapper IDE (wrapper mode is always required for dispatch)
    let (ide_command, ide_name) = (&config.ide.wrapper.command, &config.ide.wrapper.name);

    // Save launch information for auto-close functionality
    let state_dir = std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(".para_state");

    fs::create_dir_all(&state_dir)
        .map_err(|e| ParaError::fs_error(format!("Failed to create state directory: {}", e)))?;

    let session_id = session_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("unknown");

    let launch_file = state_dir.join(format!("{}.launch", session_id));

    // Always use wrapper mode for dispatch
    let launch_content = format!(
        "LAUNCH_METHOD=wrapper\nWRAPPER_IDE={}\n",
        config.ide.wrapper.name
    );
    fs::write(&launch_file, launch_content)
        .map_err(|e| ParaError::fs_error(format!("Failed to write launch file: {}", e)))?;

    // Launch IDE
    let mut cmd = Command::new(ide_command);
    cmd.current_dir(session_path);
    cmd.arg(session_path);

    // Don't wait for the IDE process - let it run in the background
    match cmd.spawn() {
        Ok(_) => {
            // IDE launched successfully, don't wait for it
            println!("Opened {} workspace", ide_name);
        }
        Err(e) => {
            return Err(ParaError::ide_error(format!(
                "Failed to launch {}: {}. Check that '{}' is installed and accessible.",
                ide_name, e, ide_command
            )));
        }
    }

    Ok(())
}

fn create_claude_task_json(command: &str) -> String {
    format!(
        r#"{{
  "version": "2.0.0",
  "tasks": [
    {{
      "label": "Start Claude Code with Prompt",
      "type": "shell",
      "command": "{}",
      "group": {{
        "kind": "build",
        "isDefault": true
      }},
      "presentation": {{
        "echo": true,
        "reveal": "always",
        "focus": false,
        "panel": "new"
      }},
      "runOptions": {{
        "runOn": "folderOpen"
      }}
    }}
  ]
}}"#,
        command.replace('"', "\\\"")
    )
}

fn create_claude_local_md(session_path: &Path, session_name: &str) -> Result<()> {
    let claude_local_path = session_path.join("CLAUDE.local.md");

    let content = format!(
        r#"<!-- Para Agent Instructions - DO NOT COMMIT -->
# Para Session Status Commands

You are working in para session: {}

Use these commands to communicate your progress:

**Required status updates:**
```bash
# Every status update MUST include current task, test status, and confidence
para status "Starting user authentication" --tests unknown --confidence medium
para status "Implementing JWT tokens" --tests passed --confidence high --todos 2/5
para status "Fixing auth middleware" --tests failed --confidence low --todos 3/5
para status "Need help with Redis mocking" --tests failed --confidence low --blocked

# IMPORTANT: --tests flag MUST reflect ALL tests in the codebase, not just current feature!
# Run full test suite before updating status
```

**Test Status Guidelines:**
- `--tests passed`: ALL tests in the entire codebase are passing
- `--tests failed`: One or more tests are failing anywhere in the codebase
- `--tests unknown`: Haven't run tests yet or tests are currently running

NEVER report partial test results. Always run the complete test suite.

**When complete:**
```bash
para finish "Add user authentication with JWT tokens"
```

Remember: 
- EVERY status update must include: task description, --tests flag, and --confidence flag
- Run ALL tests before updating status (not just tests for current feature)
- After using TodoWrite tool, include --todos flag with completed/total
- Update status when:
  - Starting new work
  - After running tests
  - Confidence level changes
  - Getting blocked
  - Making significant progress
"#,
        session_name
    );

    // Write the file (overwrite if exists)
    fs::write(&claude_local_path, content)
        .map_err(|e| ParaError::fs_error(format!("Failed to write CLAUDE.local.md: {}", e)))?;

    Ok(())
}

impl DispatchArgs {
    pub fn resolve_prompt_and_session(&self) -> Result<(Option<String>, String)> {
        match (&self.name_or_prompt, &self.prompt, &self.file) {
            // File flag provided - highest priority
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

            // Single argument - could be session+prompt, prompt, or file
            (Some(arg), None, None) => {
                if is_likely_file_path(arg) {
                    // Auto-detect file path
                    let prompt = read_file_content(Path::new(arg))?;
                    if prompt.trim().is_empty() {
                        return Err(ParaError::file_not_found(format!("file is empty: {}", arg)));
                    }
                    Ok((None, prompt))
                } else {
                    // Treat as inline prompt
                    Ok((None, arg.clone()))
                }
            }

            // Two arguments - session name + (prompt or file)
            (Some(session), Some(prompt_or_file), None) => {
                if is_likely_file_path(prompt_or_file) {
                    let prompt = read_file_content(Path::new(prompt_or_file))?;
                    if prompt.trim().is_empty() {
                        return Err(ParaError::file_not_found(format!(
                            "file is empty: {}",
                            prompt_or_file
                        )));
                    }
                    Ok((Some(session.clone()), prompt))
                } else {
                    Ok((Some(session.clone()), prompt_or_file.clone()))
                }
            }

            // Error cases
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
    // Return false if empty
    if input.is_empty() {
        return false;
    }

    // Check if it's an existing file first
    if Path::new(input).is_file() {
        return true;
    }

    // Check if it looks like a URL scheme - if so, it's NOT a file path
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

    // Check if it looks like a file path (contains / or ends with common extensions)
    if input.contains('/') {
        // Contains path separator, but make sure it's not just text with URLs
        // If it contains spaces and URLs, it's likely a prompt, not a file path
        if (input.contains(" http://")
            || input.contains(" https://")
            || input.contains(" ftp://")
            || input.contains(" ssh://"))
            && input.contains(' ')
        {
            return false; // Contains spaces and URLs - likely a prompt
        }
        return true; // Looks like a real file path
    }

    // Check for common text file extensions
    input.ends_with(".txt")
        || input.ends_with(".md")
        || input.ends_with(".rst")
        || input.ends_with(".org")
        || input.ends_with(".prompt")
        || input.ends_with(".tmpl")
        || input.ends_with(".template")
}

fn read_file_content(path: &Path) -> Result<String> {
    // Convert relative path to absolute if needed for better error messages
    let absolute_path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|e| ParaError::fs_error(format!("Failed to get current directory: {}", e)))?
            .join(path)
    };

    // Check if file exists and is readable
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

    // Check if file is readable
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

    // Read file content
    fs::read_to_string(&absolute_path).map_err(|e| {
        ParaError::file_operation(format!("failed to read file: {} ({})", path.display(), e))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
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
        };

        let result = args.resolve_prompt_and_session().unwrap();
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
        };

        let result = args.resolve_prompt_and_session().unwrap();
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
        };

        let result = args.resolve_prompt_and_session().unwrap();
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
        };

        let result = args.resolve_prompt_and_session().unwrap();
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
        };

        let result = args.resolve_prompt_and_session().unwrap();
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
        };

        let result = args.resolve_prompt_and_session();
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
        };

        let result = args.resolve_prompt_and_session();
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
        assert!(content.contains("--confidence"));
        assert!(content.contains("Test Status Guidelines:"));
        assert!(content.contains("para finish"));

        // Check specific command examples
        assert!(content.contains("para status \"Starting"));
        assert!(content.contains("--tests unknown --confidence medium"));
        assert!(content.contains("--tests passed --confidence high"));
        assert!(content.contains("--tests failed --confidence low"));
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
            assert!(result.is_ok(), "Failed for session name: {}", session_name);

            let content = std::fs::read_to_string(session_path.join("CLAUDE.local.md")).unwrap();
            assert!(
                content.contains(session_name),
                "Session name not found in content for: {}",
                session_name
            );
        }
    }
}
