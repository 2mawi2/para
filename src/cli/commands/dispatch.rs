use crate::cli::commands::common::create_claude_local_md;
use crate::cli::parser::DispatchArgs;
use crate::config::Config;
use crate::core::git::{GitOperations, GitService};
use crate::core::session::{SessionManager, SessionState};
use crate::utils::{names::*, ParaError, Result};
use std::fs;
use std::io::{self, IsTerminal, Read};
use std::path::{Path, PathBuf};
use std::process::Command;

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

    let subtrees_path = repo_root.join(&config.directories.subtrees_dir);
    let session_path = subtrees_path.join(&session_id);

    if !subtrees_path.exists() {
        fs::create_dir_all(&subtrees_path).map_err(|e| {
            ParaError::fs_error(format!("Failed to create subtrees directory: {e}"))
        })?;
    }

    git_service
        .create_worktree(&branch_name, &session_path)
        .map_err(|e| ParaError::git_error(format!("Failed to create worktree: {e}")))?;

    let mut session_state =
        SessionState::new(session_id.clone(), branch_name, session_path.clone());

    session_state.task_description = Some(prompt.clone());
    session_manager.save_state(&session_state)?;

    let state_dir = Path::new(&config.directories.state_dir);
    let task_file = state_dir.join(format!("{session_id}.task"));
    fs::write(&task_file, &prompt)
        .map_err(|e| ParaError::fs_error(format!("Failed to write task file: {e}")))?;

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
    if is_claude_code_command(&config.ide.command) && config.is_wrapper_enabled() {
        return Ok(());
    }

    Err(ParaError::invalid_config(format!(
        "Dispatch command requires Claude Code in wrapper mode. Current IDE: '{}' with command: '{}', wrapper enabled: {}. Run 'para config' to configure Claude Code with wrapper mode.",
        config.ide.name, config.ide.command, config.is_wrapper_enabled()
    )))
}

fn is_claude_code_command(command: &str) -> bool {
    let command_lower = command.to_lowercase();
    command_lower == "claude" || command_lower == "claude-code"
}

fn launch_claude_code(
    config: &Config,
    session_path: &Path,
    prompt: &str,
    skip_permissions: bool,
) -> Result<()> {
    let temp_prompt_file = session_path.join(".claude_prompt_temp");
    if !prompt.is_empty() {
        fs::write(&temp_prompt_file, prompt)
            .map_err(|e| ParaError::fs_error(format!("Failed to write temp prompt file: {e}")))?;
    }

    launch_claude_in_ide(config, session_path, &temp_prompt_file, skip_permissions)
}

fn launch_claude_in_ide(
    config: &Config,
    session_path: &Path,
    temp_prompt_file: &Path,
    skip_permissions: bool,
) -> Result<()> {
    let vscode_dir = session_path.join(".vscode");
    fs::create_dir_all(&vscode_dir)
        .map_err(|e| ParaError::fs_error(format!("Failed to create .vscode directory: {e}")))?;

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

    let tasks_json = create_claude_task_json(&claude_task_cmd);
    let tasks_file = vscode_dir.join("tasks.json");
    fs::write(&tasks_file, tasks_json)
        .map_err(|e| ParaError::fs_error(format!("Failed to write tasks.json: {e}")))?;

    let (ide_command, ide_name) = (&config.ide.wrapper.command, &config.ide.wrapper.name);

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

    let mut cmd = Command::new(ide_command);
    cmd.current_dir(session_path);
    cmd.arg(session_path);

    // Detach the IDE process from the parent's stdio to prevent blocking
    cmd.stdin(std::process::Stdio::null());
    cmd.stdout(std::process::Stdio::null());
    cmd.stderr(std::process::Stdio::null());

    match cmd.spawn() {
        Ok(_) => {
            println!("Opened {ide_name} workspace");
        }
        Err(e) => {
            return Err(ParaError::ide_error(format!(
                "Failed to launch {ide_name}: {e}. Check that '{ide_command}' is installed and accessible."
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
      "options": {{
        "env": {{
          "FORCE_COLOR": "1",
          "TERM": "xterm-256color"
        }}
      }},
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

impl DispatchArgs {
    pub fn resolve_prompt_and_session(&self) -> Result<(Option<String>, String)> {
        // Priority order:
        // 1. File flag (highest priority)
        // 2. Explicit arguments
        // 3. Stdin input (lowest priority)

        // Priority 1: File flag - use directly without checking stdin
        if self.file.is_some() {
            return self.resolve_prompt_and_session_no_stdin();
        }

        // Priority 2: Explicit arguments - use instead of checking stdin
        if self.has_explicit_arguments() {
            return self.resolve_prompt_and_session_no_stdin();
        }

        // Priority 3: Stdin input - only if no file flag or explicit arguments
        if self.should_read_from_stdin() {
            return self.resolve_from_stdin();
        }

        // No valid input source - return error
        self.resolve_prompt_and_session_no_stdin()
    }

    fn has_explicit_arguments(&self) -> bool {
        self.name_or_prompt.is_some() || self.prompt.is_some()
    }

    fn should_read_from_stdin(&self) -> bool {
        !io::stdin().is_terminal()
    }

    fn resolve_from_stdin(&self) -> Result<(Option<String>, String)> {
        let mut buffer = String::new();
        io::stdin()
            .read_to_string(&mut buffer)
            .map_err(|e| ParaError::file_operation(format!("Failed to read from stdin: {e}")))?;

        if buffer.trim().is_empty() {
            return Err(ParaError::invalid_args("Piped input is empty"));
        }

        // When using stdin, the first positional argument (if any) is the session name
        Ok((self.name_or_prompt.clone(), buffer))
    }

    fn resolve_prompt_and_session_no_stdin(&self) -> Result<(Option<String>, String)> {
        // Priority 1: File flag - highest priority
        if let Some(file_path) = &self.file {
            return self.resolve_from_file(file_path);
        }

        // Priority 2: Explicit arguments
        match (&self.name_or_prompt, &self.prompt) {
            (Some(arg), None) => self.resolve_single_argument(arg),
            (Some(session), Some(prompt_or_file)) => {
                self.resolve_session_and_prompt(session, prompt_or_file)
            }
            (None, None) => Err(ParaError::invalid_args(
                "dispatch requires a prompt text or file path",
            )),
            _ => Err(ParaError::invalid_args(
                "Invalid argument combination for dispatch",
            )),
        }
    }

    fn resolve_from_file(&self, file_path: &Path) -> Result<(Option<String>, String)> {
        let prompt = read_file_content(file_path)?;
        validate_non_empty_content(&prompt, &file_path.display().to_string())?;
        Ok((self.name_or_prompt.clone(), prompt))
    }

    fn resolve_single_argument(&self, arg: &str) -> Result<(Option<String>, String)> {
        if is_likely_file_path(arg) {
            let prompt = read_file_content(Path::new(arg))?;
            validate_non_empty_content(&prompt, arg)?;
            Ok((None, prompt))
        } else {
            Ok((None, arg.to_string()))
        }
    }

    fn resolve_session_and_prompt(
        &self,
        session: &str,
        prompt_or_file: &str,
    ) -> Result<(Option<String>, String)> {
        if is_likely_file_path(prompt_or_file) {
            let prompt = read_file_content(Path::new(prompt_or_file))?;
            validate_non_empty_content(&prompt, prompt_or_file)?;
            Ok((Some(session.to_string()), prompt))
        } else {
            Ok((Some(session.to_string()), prompt_or_file.to_string()))
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

    if is_url_pattern(input) {
        return false;
    }

    if input.contains('/') {
        if contains_url_in_text(input) {
            return false;
        }
        return true;
    }

    has_supported_file_extension(input)
}

fn is_url_pattern(input: &str) -> bool {
    let url_prefixes = [
        "http://", "https://", "ftp://", "ftps://", "ssh://", "git://", "file://",
    ];

    url_prefixes.iter().any(|prefix| input.starts_with(prefix))
}

fn contains_url_in_text(input: &str) -> bool {
    let url_patterns = [" http://", " https://", " ftp://", " ssh://"];

    input.contains(' ') && url_patterns.iter().any(|pattern| input.contains(pattern))
}

fn has_supported_file_extension(input: &str) -> bool {
    let supported_extensions = [
        ".txt",
        ".md",
        ".rst",
        ".org",
        ".prompt",
        ".tmpl",
        ".template",
    ];

    supported_extensions.iter().any(|ext| input.ends_with(ext))
}

fn validate_non_empty_content(content: &str, source: &str) -> Result<()> {
    if content.trim().is_empty() {
        return Err(ParaError::file_not_found(format!(
            "file is empty: {source}"
        )));
    }
    Ok(())
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
