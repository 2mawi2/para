use crate::config::Config;
use crate::core::sandbox::config::SandboxResolver;
use crate::core::sandbox::launcher::{
    generate_network_sandbox_wrapper, is_sandbox_available, wrap_command_with_sandbox,
    SandboxOptions,
};
use crate::core::sandbox::proxy::DEFAULT_PROXY_PORT;
use crate::utils::gitignore::GitignoreManager;
use crate::utils::{ParaError, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::Duration;

/// Options for launching Claude with different continuation modes
#[derive(Debug, Default)]
pub struct ClaudeLaunchOptions {
    pub skip_permissions: bool,
    pub session_id: Option<String>,
    pub continue_conversation: bool,
    pub prompt_content: Option<String>,
    pub sandbox_override: Option<bool>,
    pub sandbox_profile: Option<String>,
    pub network_sandbox: bool,
    pub allowed_domains: Vec<String>,
}

/// Launch Claude Code with session continuation and optional prompt content
/// This is a unified approach used by both dispatch and resume commands
pub fn launch_claude_with_context(
    config: &Config,
    session_path: &Path,
    options: ClaudeLaunchOptions,
) -> Result<()> {
    let vscode_dir = session_path.join(".vscode");
    fs::create_dir_all(&vscode_dir)
        .map_err(|e| ParaError::fs_error(format!("Failed to create .vscode directory: {e}")))?;

    // Resolve sandbox settings using the resolver
    let resolver = SandboxResolver::new(config);
    let sandbox_settings = resolver.resolve_with_network(
        options.sandbox_override.unwrap_or(false),
        options.sandbox_override == Some(false),
        options.sandbox_profile.clone(),
        options.network_sandbox,
        options.allowed_domains.clone(),
    );

    // Check if sandboxing is enabled and available
    let should_sandbox = sandbox_settings.enabled && cfg!(target_os = "macos");

    if should_sandbox && !is_sandbox_available() {
        eprintln!(
            "⚠️  Warning: Sandbox is enabled but sandbox-exec is not available on this system"
        );
    }

    // Build base command
    let mut base_cmd = config.ide.command.clone();
    if options.skip_permissions {
        base_cmd.push_str(" --dangerously-skip-permissions");
    }

    // Handle prompt content via temporary file
    let temp_prompt_file = session_path.join(".claude_prompt_temp");
    if let Some(ref content) = options.prompt_content {
        if !content.is_empty() {
            fs::write(&temp_prompt_file, content).map_err(|e| {
                ParaError::fs_error(format!("Failed to write temp prompt file: {e}"))
            })?;
        }
    }

    // Build Claude command based on continuation mode
    let claude_task_cmd = if let Some(ref session_id) = options.session_id {
        if !session_id.is_empty() {
            // Resume existing session with optional prompt
            if temp_prompt_file.exists() {
                format!(
                    "{} -r \"{}\" \"$(cat '{}'; rm '{}')\"",
                    base_cmd,
                    session_id,
                    temp_prompt_file.display(),
                    temp_prompt_file.display()
                )
            } else {
                format!("{base_cmd} -r \"{session_id}\"")
            }
        } else {
            // Empty session ID, fall back to -c
            if temp_prompt_file.exists() {
                format!(
                    "{} -c \"$(cat '{}'; rm '{}')\"",
                    base_cmd,
                    temp_prompt_file.display(),
                    temp_prompt_file.display()
                )
            } else {
                format!("{base_cmd} -c")
            }
        }
    } else if options.continue_conversation {
        // Continue conversation mode
        if temp_prompt_file.exists() {
            format!(
                "{} -c \"$(cat '{}'; rm '{}')\"",
                base_cmd,
                temp_prompt_file.display(),
                temp_prompt_file.display()
            )
        } else {
            format!("{base_cmd} -c")
        }
    } else {
        // New session with optional prompt
        if temp_prompt_file.exists() {
            format!(
                "{} \"$(cat '{}'; rm '{}')\"",
                base_cmd,
                temp_prompt_file.display(),
                temp_prompt_file.display()
            )
        } else {
            base_cmd
        }
    };

    // Check if we need sandboxing
    let should_sandbox = sandbox_settings.enabled && cfg!(target_os = "macos");

    // Apply sandboxing if enabled
    let (final_command, needs_wrapper_script) = if should_sandbox && is_sandbox_available() {
        // Determine profile and proxy settings
        let (profile, proxy_address) = if sandbox_settings.network_sandbox {
            // For network sandboxing, use the proxied profile
            let proxy_addr = format!("127.0.0.1:{DEFAULT_PROXY_PORT}");
            ("standard-proxied", Some(proxy_addr))
        } else {
            (sandbox_settings.profile.as_str(), None)
        };

        let sandbox_options = SandboxOptions {
            profile: profile.to_string(),
            proxy_address: proxy_address.clone(),
            allowed_domains: sandbox_settings.allowed_domains.clone(),
        };

        match wrap_command_with_sandbox(&claude_task_cmd, session_path, &sandbox_options) {
            Ok(sandboxed_cmd) => {
                if sandbox_settings.network_sandbox {
                    println!("🔒 Network-isolated sandboxing enabled for Claude CLI");
                    if !sandbox_settings.allowed_domains.is_empty() {
                        println!(
                            "   Additional allowed domains: {}",
                            sandbox_settings.allowed_domains.join(", ")
                        );
                    }
                } else {
                    println!(
                        "🔒 Sandboxing enabled for Claude CLI (profile: {})",
                        sandbox_settings.profile
                    );
                }

                // For network sandboxing, we need to create a wrapper script
                if sandboxed_cmd.needs_wrapper_script {
                    let wrapper = generate_network_sandbox_wrapper(
                        &sandboxed_cmd.command,
                        sandboxed_cmd.proxy_port.unwrap_or(DEFAULT_PROXY_PORT),
                        &sandboxed_cmd.allowed_domains,
                    );
                    (wrapper, true)
                } else {
                    (sandboxed_cmd.command, false)
                }
            }
            Err(e) => {
                eprintln!("⚠️  Warning: Failed to apply sandbox: {e}");
                eprintln!("   Continuing without sandboxing");
                (claude_task_cmd, false)
            }
        }
    } else {
        (claude_task_cmd, false)
    };

    // Handle wrapper script or direct command
    let task_command = if needs_wrapper_script {
        // Write the wrapper script
        let script_path = vscode_dir.join("para-sandbox-launcher.sh");
        fs::write(&script_path, &final_command)
            .map_err(|e| ParaError::fs_error(format!("Failed to write launcher script: {e}")))?;

        // Make it executable on Unix systems
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&script_path)?.permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&script_path, perms)?;
        }

        println!("\n📝 Network sandboxing configured");
        println!("   VS Code will run the sandboxed Claude when you allow the task");
        println!("   The launcher script will self-delete after execution");

        // The task will run the script
        "./.vscode/para-sandbox-launcher.sh".to_string()
    } else {
        final_command
    };

    // Create tasks.json with the command
    let tasks_json = create_claude_task_json(&task_command);
    let tasks_file = vscode_dir.join("tasks.json");
    fs::write(&tasks_file, tasks_json)
        .map_err(|e| ParaError::fs_error(format!("Failed to write tasks.json: {e}")))?;

    // Add .vscode/tasks.json to gitignore to prevent it from showing in git status
    // This handles the case where VS Code might recreate the file after our cleanup
    let gitignore_manager = GitignoreManager::new(session_path.to_str().unwrap_or("."));

    // Add the entry to gitignore (the manager will handle comments appropriately)
    if let Err(e) = gitignore_manager.add_entry(".vscode/tasks.json") {
        // Log warning but don't fail - this is a best-effort operation
        eprintln!("Warning: Failed to add .vscode/tasks.json to gitignore: {e}");
    }

    // Launch IDE wrapper
    let (ide_command, ide_name) = (&config.ide.wrapper.command, &config.ide.wrapper.name);
    let mut cmd = Command::new(ide_command);
    cmd.current_dir(session_path);
    cmd.arg(session_path);

    // Detach the IDE process
    cmd.stdin(std::process::Stdio::null());
    cmd.stdout(std::process::Stdio::null());
    cmd.stderr(std::process::Stdio::null());

    match cmd.spawn() {
        Ok(_) => {
            println!(
                "✅ VS Code opened - {} will start automatically",
                config.ide.name
            );

            // Spawn a background cleanup task for the tasks.json file
            spawn_tasks_cleanup(tasks_file);
        }
        Err(e) => {
            return Err(ParaError::ide_error(format!(
                "Failed to launch {ide_name}: {e}. Check that '{ide_command}' is installed and accessible."
            )));
        }
    }

    Ok(())
}

/// Create tasks.json for Claude with proper escaping
fn create_claude_task_json(command: &str) -> String {
    format!(
        r#"{{
    "version": "2.0.0",
    "tasks": [
        {{
            "label": "Start claude",
            "type": "shell",
            "command": "{}",
            "group": "build",
            "options": {{
                "env": {{
                    "FORCE_COLOR": "1",
                    "COLORTERM": "truecolor",
                    "TERM": "xterm-256color"
                }}
            }},
            "presentation": {{
                "echo": true,
                "reveal": "always",
                "focus": true,
                "panel": "new",
                "showReuseMessage": false,
                "clear": false
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

/// Spawn a background thread to clean up the tasks.json file after VS Code has had time to read it
fn spawn_tasks_cleanup(tasks_file: PathBuf) {
    thread::spawn(move || {
        // Wait for VS Code to read and execute the task
        // Try multiple times with increasing delays to handle VS Code recreating the file
        let delays = [5, 10, 20]; // seconds

        for (attempt, delay) in delays.iter().enumerate() {
            thread::sleep(Duration::from_secs(*delay));

            // Check if file still exists before trying to remove it
            if !tasks_file.exists() {
                // File already removed, we're done
                return;
            }

            // Attempt to remove the tasks.json file
            match fs::remove_file(&tasks_file) {
                Ok(_) => {
                    // Successfully removed, check once more after a short delay
                    thread::sleep(Duration::from_secs(2));
                    if !tasks_file.exists() {
                        // File is gone and stayed gone, success
                        return;
                    }
                    // File was recreated, continue trying
                }
                Err(e) => {
                    // Only log on the last attempt
                    if attempt == delays.len() - 1 {
                        eprintln!(
                            "Warning: Failed to clean up tasks.json after multiple attempts: {e}"
                        );
                    }
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::sandbox::launcher::is_sandbox_available;
    use crate::test_utils::test_helpers::*;
    use tempfile::TempDir;

    #[test]
    fn test_claude_launch_options_default() {
        let options = ClaudeLaunchOptions::default();
        assert!(!options.skip_permissions);
        assert!(options.session_id.is_none());
        assert!(!options.continue_conversation);
        assert!(options.prompt_content.is_none());
    }

    #[test]
    fn test_claude_launch_options_with_all_fields() {
        let options = ClaudeLaunchOptions {
            skip_permissions: true,
            session_id: Some("test-session-123".to_string()),
            continue_conversation: true,
            prompt_content: Some("Test prompt content".to_string()),
            sandbox_override: Some(true),
            sandbox_profile: Some("restrictive-closed".to_string()),
            network_sandbox: false,
            allowed_domains: vec![],
        };

        assert!(options.skip_permissions);
        assert_eq!(options.session_id, Some("test-session-123".to_string()));
        assert!(options.continue_conversation);
        assert_eq!(
            options.prompt_content,
            Some("Test prompt content".to_string())
        );
        assert_eq!(options.sandbox_override, Some(true));
        assert_eq!(
            options.sandbox_profile,
            Some("restrictive-closed".to_string())
        );
    }

    #[test]
    fn test_create_claude_task_json_basic() {
        let command = "claude --version";
        let json = create_claude_task_json(command);

        // Should be valid JSON structure
        assert!(json.contains(r#""version": "2.0.0""#));
        assert!(json.contains(r#""label": "Start claude""#));
        assert!(json.contains(r#""type": "shell""#));
        assert!(json.contains(r#""command": "claude --version""#));
        assert!(json.contains(r#""runOn": "folderOpen""#));

        // Should contain environment variables for color support
        assert!(json.contains(r#""options""#));
        assert!(json.contains(r#""env""#));
        assert!(json.contains(r#""FORCE_COLOR": "1""#));
        assert!(json.contains(r#""COLORTERM": "truecolor""#));
        assert!(json.contains(r#""TERM": "xterm-256color""#));
    }

    #[test]
    fn test_create_claude_task_json_escaping() {
        let command = r#"claude -r "session-id" "prompt with \"quotes\"""#;
        let json = create_claude_task_json(command);

        // Should escape inner quotes correctly
        let expected_in_json = command.replace('"', "\\\"");
        assert!(json.contains(&expected_in_json));

        // Should be valid JSON structure
        assert!(json.contains(r#""version": "2.0.0""#));
        assert!(json.contains(r#""label": "Start claude""#));
    }

    #[test]
    fn test_create_claude_task_json_complex_escaping() {
        let command = r#"claude -c "test with 'single' and \"double\" quotes""#;
        let json = create_claude_task_json(command);

        // The command.replace('"', "\\\"") will convert:
        // claude -c "test with 'single' and \"double\" quotes"
        // to:
        // claude -c \"test with 'single' and \\\"double\\\" quotes\"
        let expected_escaped = command.replace('"', "\\\"");
        assert!(json.contains(&expected_escaped));

        // Verify JSON structure is intact
        assert!(json.starts_with("{\n"));
        assert!(json.ends_with("}"));
        assert!(json.contains(r#""tasks": ["#));

        // Check for single task
        assert_eq!(json.matches(r#""label":"#).count(), 1);
    }

    #[test]
    fn test_launch_claude_with_context_basic() {
        let temp_dir = TempDir::new().unwrap();
        let session_path = temp_dir.path().join("test-session");
        fs::create_dir_all(&session_path).unwrap();

        let config = create_test_config();
        let options = ClaudeLaunchOptions::default();

        let result = launch_claude_with_context(&config, &session_path, options);
        assert!(result.is_ok());

        // Verify .vscode directory was created
        assert!(session_path.join(".vscode").exists());

        // Verify tasks.json was created
        let tasks_file = session_path.join(".vscode/tasks.json");
        assert!(tasks_file.exists());

        // Verify tasks.json content
        let tasks_content = fs::read_to_string(tasks_file).unwrap();
        assert!(tasks_content.contains("Start claude"));
        assert!(tasks_content.contains("echo")); // Test config uses echo

        // Verify environment variables are present
        assert!(tasks_content.contains(r#""FORCE_COLOR": "1""#));
        assert!(tasks_content.contains(r#""COLORTERM": "truecolor""#));
        assert!(tasks_content.contains(r#""TERM": "xterm-256color""#));

        // Verify gitignore was created with .vscode/tasks.json entry
        let gitignore_path = session_path.join(".gitignore");
        assert!(gitignore_path.exists(), "Gitignore should be created");
        let gitignore_content = fs::read_to_string(gitignore_path).unwrap();
        assert!(
            gitignore_content.contains(".vscode/tasks.json"),
            "Gitignore should contain .vscode/tasks.json entry"
        );
    }

    #[test]
    fn test_launch_claude_with_skip_permissions() {
        let temp_dir = TempDir::new().unwrap();
        let session_path = temp_dir.path().join("test-session");
        fs::create_dir_all(&session_path).unwrap();

        let config = create_test_config();
        let options = ClaudeLaunchOptions {
            skip_permissions: true,
            ..Default::default()
        };

        let result = launch_claude_with_context(&config, &session_path, options);
        assert!(result.is_ok());

        // Check tasks.json contains skip permissions flag
        let tasks_content = fs::read_to_string(session_path.join(".vscode/tasks.json")).unwrap();
        assert!(tasks_content.contains("--dangerously-skip-permissions"));
    }

    #[test]
    fn test_launch_claude_with_session_id() {
        let temp_dir = TempDir::new().unwrap();
        let session_path = temp_dir.path().join("test-session");
        fs::create_dir_all(&session_path).unwrap();

        let config = create_test_config();
        let options = ClaudeLaunchOptions {
            session_id: Some("test-session-456".to_string()),
            ..Default::default()
        };

        let result = launch_claude_with_context(&config, &session_path, options);
        assert!(result.is_ok());

        // Check tasks.json contains resume flag with session ID
        let tasks_content = fs::read_to_string(session_path.join(".vscode/tasks.json")).unwrap();
        assert!(tasks_content.contains("-r"));
        assert!(tasks_content.contains("test-session-456"));
    }

    #[test]
    fn test_launch_claude_with_empty_session_id() {
        let temp_dir = TempDir::new().unwrap();
        let session_path = temp_dir.path().join("test-session");
        fs::create_dir_all(&session_path).unwrap();

        let config = create_test_config();
        let options = ClaudeLaunchOptions {
            session_id: Some("".to_string()),
            ..Default::default()
        };

        let result = launch_claude_with_context(&config, &session_path, options);
        assert!(result.is_ok());

        // Should fall back to -c flag for empty session ID
        let tasks_content = fs::read_to_string(session_path.join(".vscode/tasks.json")).unwrap();
        assert!(tasks_content.contains("-c"));
        assert!(!tasks_content.contains("-r"));
    }

    #[test]
    fn test_launch_claude_with_continue_conversation() {
        let temp_dir = TempDir::new().unwrap();
        let session_path = temp_dir.path().join("test-session");
        fs::create_dir_all(&session_path).unwrap();

        let config = create_test_config();
        let options = ClaudeLaunchOptions {
            continue_conversation: true,
            ..Default::default()
        };

        let result = launch_claude_with_context(&config, &session_path, options);
        assert!(result.is_ok());

        // Check tasks.json contains continue flag
        let tasks_content = fs::read_to_string(session_path.join(".vscode/tasks.json")).unwrap();
        assert!(tasks_content.contains("-c"));
    }

    #[test]
    fn test_launch_claude_with_prompt_content() {
        let temp_dir = TempDir::new().unwrap();
        let session_path = temp_dir.path().join("test-session");
        fs::create_dir_all(&session_path).unwrap();

        let config = create_test_config();
        let options = ClaudeLaunchOptions {
            prompt_content: Some("Test prompt for Claude".to_string()),
            ..Default::default()
        };

        let result = launch_claude_with_context(&config, &session_path, options);
        assert!(result.is_ok());

        // Check that prompt file was created temporarily
        // Note: The file is deleted as part of the command, so we check tasks.json
        let tasks_content = fs::read_to_string(session_path.join(".vscode/tasks.json")).unwrap();
        assert!(tasks_content.contains(".claude_prompt_temp"));
        assert!(tasks_content.contains("$(cat"));
        assert!(tasks_content.contains("rm"));
    }

    #[test]
    fn test_launch_claude_with_prompt_and_session_id() {
        let temp_dir = TempDir::new().unwrap();
        let session_path = temp_dir.path().join("test-session");
        fs::create_dir_all(&session_path).unwrap();

        let config = create_test_config();
        let options = ClaudeLaunchOptions {
            session_id: Some("resume-123".to_string()),
            prompt_content: Some("Resume with this prompt".to_string()),
            ..Default::default()
        };

        let result = launch_claude_with_context(&config, &session_path, options);
        assert!(result.is_ok());

        // Check tasks.json contains both resume flag and prompt handling
        let tasks_content = fs::read_to_string(session_path.join(".vscode/tasks.json")).unwrap();
        assert!(tasks_content.contains("-r"));
        assert!(tasks_content.contains("resume-123"));
        assert!(tasks_content.contains(".claude_prompt_temp"));
    }

    #[test]
    fn test_launch_claude_with_prompt_and_continue() {
        let temp_dir = TempDir::new().unwrap();
        let session_path = temp_dir.path().join("test-session");
        fs::create_dir_all(&session_path).unwrap();

        let config = create_test_config();
        let options = ClaudeLaunchOptions {
            continue_conversation: true,
            prompt_content: Some("Continue with this prompt".to_string()),
            ..Default::default()
        };

        let result = launch_claude_with_context(&config, &session_path, options);
        assert!(result.is_ok());

        // Check tasks.json contains continue flag and prompt handling
        let tasks_content = fs::read_to_string(session_path.join(".vscode/tasks.json")).unwrap();
        assert!(tasks_content.contains("-c"));
        assert!(tasks_content.contains(".claude_prompt_temp"));
    }

    #[test]
    fn test_launch_claude_with_all_options() {
        let temp_dir = TempDir::new().unwrap();
        let session_path = temp_dir.path().join("test-session");
        fs::create_dir_all(&session_path).unwrap();

        let config = create_test_config();
        let options = ClaudeLaunchOptions {
            skip_permissions: true,
            session_id: Some("complex-session".to_string()),
            continue_conversation: false, // Should be ignored when session_id is present
            prompt_content: Some("Complex prompt".to_string()),
            sandbox_override: None,
            sandbox_profile: None,
            network_sandbox: false,
            allowed_domains: vec![],
        };

        let result = launch_claude_with_context(&config, &session_path, options);
        assert!(result.is_ok());

        // Check all options are reflected in tasks.json
        let tasks_content = fs::read_to_string(session_path.join(".vscode/tasks.json")).unwrap();
        assert!(tasks_content.contains("--dangerously-skip-permissions"));
        assert!(tasks_content.contains("-r"));
        assert!(tasks_content.contains("complex-session"));
        assert!(tasks_content.contains(".claude_prompt_temp"));
    }

    #[test]
    fn test_launch_claude_empty_prompt_content() {
        let temp_dir = TempDir::new().unwrap();
        let session_path = temp_dir.path().join("test-session");
        fs::create_dir_all(&session_path).unwrap();

        let config = create_test_config();
        let options = ClaudeLaunchOptions {
            prompt_content: Some("".to_string()), // Empty prompt
            ..Default::default()
        };

        let result = launch_claude_with_context(&config, &session_path, options);
        assert!(result.is_ok());

        // Empty prompt should not create temp file
        let tasks_content = fs::read_to_string(session_path.join(".vscode/tasks.json")).unwrap();
        assert!(!tasks_content.contains(".claude_prompt_temp"));
        assert!(tasks_content.contains("echo")); // Just the base command
    }

    #[test]
    fn test_launch_claude_creates_vscode_directory() {
        let temp_dir = TempDir::new().unwrap();
        let session_path = temp_dir.path().join("deeply/nested/session");

        // Don't pre-create any directories
        let config = create_test_config();
        let options = ClaudeLaunchOptions::default();

        let result = launch_claude_with_context(&config, &session_path, options);
        assert!(result.is_ok());

        // Should create all necessary directories
        assert!(session_path.join(".vscode").exists());
        assert!(session_path.join(".vscode/tasks.json").exists());
    }

    #[test]
    fn test_launch_claude_overwrites_existing_tasks() {
        let temp_dir = TempDir::new().unwrap();
        let session_path = temp_dir.path().join("test-session");
        let vscode_dir = session_path.join(".vscode");
        fs::create_dir_all(&vscode_dir).unwrap();

        // Create existing tasks.json with different content
        let old_tasks = r#"{"version": "1.0.0", "tasks": []}"#;
        fs::write(vscode_dir.join("tasks.json"), old_tasks).unwrap();

        let config = create_test_config();
        let options = ClaudeLaunchOptions::default();

        let result = launch_claude_with_context(&config, &session_path, options);
        assert!(result.is_ok());

        // Should overwrite with new content
        let tasks_content = fs::read_to_string(vscode_dir.join("tasks.json")).unwrap();
        assert!(tasks_content.contains("2.0.0"));
        assert!(tasks_content.contains("Start claude"));
        assert!(!tasks_content.contains("1.0.0"));
    }

    #[test]
    fn test_launch_claude_special_characters_in_prompt() {
        let temp_dir = TempDir::new().unwrap();
        let session_path = temp_dir.path().join("test-session");
        fs::create_dir_all(&session_path).unwrap();

        let config = create_test_config();
        let options = ClaudeLaunchOptions {
            prompt_content: Some("Prompt with\nnewlines\tand\ttabs".to_string()),
            ..Default::default()
        };

        let result = launch_claude_with_context(&config, &session_path, options);
        assert!(result.is_ok());

        // Should handle special characters in prompt file
        // File gets created and deleted by the command, so check tasks.json
        let tasks_content = fs::read_to_string(session_path.join(".vscode/tasks.json")).unwrap();
        assert!(tasks_content.contains(".claude_prompt_temp"));
    }

    #[test]
    fn test_create_claude_task_json_formatting() {
        let command = "test-command";
        let json = create_claude_task_json(command);

        // Check proper formatting and indentation
        assert!(json.starts_with("{\n"));
        assert!(json.contains("    \"version\": \"2.0.0\",\n"));
        assert!(json.contains("    \"tasks\": [\n"));
        assert!(json.contains("        {\n"));
        assert!(json.contains("            \"label\": \"Start claude\",\n"));
        assert!(json.ends_with("}"));

        // Check all required fields are present
        assert!(json.contains("\"type\": \"shell\""));
        assert!(json.contains("\"group\": \"build\""));
        assert!(json.contains("\"presentation\""));
        assert!(json.contains("\"runOptions\""));
        assert!(json.contains("\"runOn\": \"folderOpen\""));
        // Simple task doesn't have problemMatcher or dependsOrder
    }

    #[test]
    fn test_launch_claude_with_sandbox_enabled() {
        // Skip test on non-macOS platforms
        if !cfg!(target_os = "macos") {
            return;
        }

        let temp_dir = TempDir::new().unwrap();
        let session_path = temp_dir.path().join("test-session");
        fs::create_dir_all(&session_path).unwrap();

        let mut config = create_test_config();
        config.sandbox = Some(crate::core::sandbox::SandboxConfig {
            enabled: true,
            profile: "permissive-open".to_string(),
            allowed_domains: Vec::new(),
        });

        let options = ClaudeLaunchOptions::default();

        let result = launch_claude_with_context(&config, &session_path, options);
        assert!(result.is_ok());

        // Check tasks.json was created
        let tasks_content = fs::read_to_string(session_path.join(".vscode/tasks.json")).unwrap();

        // If sandbox-exec is available, the command should be wrapped
        if is_sandbox_available() {
            assert!(tasks_content.contains("sandbox-exec"));
            assert!(tasks_content.contains("-D 'TARGET_DIR="));
            assert!(tasks_content.contains("-D 'TMP_DIR="));
            assert!(tasks_content.contains("-D 'HOME_DIR="));
            assert!(tasks_content.contains("-D 'CACHE_DIR="));
        } else {
            // Without sandbox-exec, it should fall back to regular command
            assert!(!tasks_content.contains("sandbox-exec"));
        }
    }

    #[test]
    fn test_launch_claude_with_sandbox_disabled() {
        let temp_dir = TempDir::new().unwrap();
        let session_path = temp_dir.path().join("test-session");
        fs::create_dir_all(&session_path).unwrap();

        let mut config = create_test_config();
        config.sandbox = Some(crate::core::sandbox::SandboxConfig {
            enabled: false,
            profile: "permissive-open".to_string(),
            allowed_domains: Vec::new(),
        });

        let options = ClaudeLaunchOptions::default();

        let result = launch_claude_with_context(&config, &session_path, options);
        assert!(result.is_ok());

        // Check tasks.json doesn't contain sandbox commands
        let tasks_content = fs::read_to_string(session_path.join(".vscode/tasks.json")).unwrap();
        assert!(!tasks_content.contains("sandbox-exec"));
    }

    #[test]
    fn test_launch_claude_no_sandbox_config() {
        let temp_dir = TempDir::new().unwrap();
        let session_path = temp_dir.path().join("test-session");
        fs::create_dir_all(&session_path).unwrap();

        let mut config = create_test_config();
        config.sandbox = None;

        let options = ClaudeLaunchOptions::default();

        let result = launch_claude_with_context(&config, &session_path, options);
        assert!(result.is_ok());

        // Check tasks.json doesn't contain sandbox commands
        let tasks_content = fs::read_to_string(session_path.join(".vscode/tasks.json")).unwrap();
        assert!(!tasks_content.contains("sandbox-exec"));
    }

    #[test]
    fn test_launch_claude_sandbox_with_complex_options() {
        // Skip test on non-macOS platforms
        if !cfg!(target_os = "macos") {
            return;
        }

        let temp_dir = TempDir::new().unwrap();
        let session_path = temp_dir.path().join("test-session");
        fs::create_dir_all(&session_path).unwrap();

        let mut config = create_test_config();
        config.sandbox = Some(crate::core::sandbox::SandboxConfig {
            enabled: true,
            profile: "permissive-open".to_string(),
            allowed_domains: Vec::new(),
        });

        let options = ClaudeLaunchOptions {
            skip_permissions: true,
            session_id: Some("test-123".to_string()),
            continue_conversation: false,
            prompt_content: Some("Test prompt".to_string()),
            sandbox_override: None,
            sandbox_profile: None,
            network_sandbox: false,
            allowed_domains: vec![],
        };

        let result = launch_claude_with_context(&config, &session_path, options);
        assert!(result.is_ok());

        // Check tasks.json contains all expected elements
        let tasks_content = fs::read_to_string(session_path.join(".vscode/tasks.json")).unwrap();

        if is_sandbox_available() {
            assert!(tasks_content.contains("sandbox-exec"));
            // The entire Claude command should be wrapped in sh -c
            assert!(tasks_content.contains("sh -c"));
            // Should still contain the Claude-specific flags
            assert!(tasks_content.contains("--dangerously-skip-permissions"));
            assert!(tasks_content.contains("-r"));
            assert!(tasks_content.contains("test-123"));
        }
    }

    #[test]
    fn test_tasks_cleanup_removes_file() {
        let temp_dir = TempDir::new().unwrap();
        let vscode_dir = temp_dir.path().join(".vscode");
        fs::create_dir_all(&vscode_dir).unwrap();

        let tasks_file = vscode_dir.join("tasks.json");
        fs::write(&tasks_file, "test content").unwrap();

        // Verify file exists
        assert!(tasks_file.exists());

        // Spawn cleanup
        spawn_tasks_cleanup(tasks_file.clone());

        // Wait for cleanup to complete (slightly longer than the cleanup delay)
        thread::sleep(Duration::from_secs(6));

        // Verify file was removed
        assert!(!tasks_file.exists());
    }

    #[test]
    fn test_tasks_cleanup_handles_missing_file() {
        let temp_dir = TempDir::new().unwrap();
        let tasks_file = temp_dir.path().join(".vscode/tasks.json");

        // File doesn't exist
        assert!(!tasks_file.exists());

        // Spawn cleanup - should not panic
        spawn_tasks_cleanup(tasks_file.clone());

        // Wait for cleanup to complete
        thread::sleep(Duration::from_secs(6));

        // Should complete without errors
        assert!(!tasks_file.exists());
    }

    #[test]
    fn test_launch_claude_adds_gitignore_entry() {
        let temp_dir = TempDir::new().unwrap();
        let session_path = temp_dir.path().join("test-session");
        fs::create_dir_all(&session_path).unwrap();

        // Create existing gitignore with some content
        let gitignore_path = session_path.join(".gitignore");
        fs::write(&gitignore_path, "*.log\ntarget/\n").unwrap();

        let config = create_test_config();
        let options = ClaudeLaunchOptions::default();

        let result = launch_claude_with_context(&config, &session_path, options);
        assert!(result.is_ok());

        // Verify gitignore was updated
        let gitignore_content = fs::read_to_string(&gitignore_path).unwrap();
        assert!(
            gitignore_content.contains("*.log"),
            "Original content should be preserved"
        );
        assert!(
            gitignore_content.contains("target/"),
            "Original content should be preserved"
        );
        assert!(
            gitignore_content.contains(".vscode/tasks.json"),
            "Should add .vscode/tasks.json entry"
        );
    }

    #[test]
    fn test_launch_claude_handles_duplicate_gitignore_entry() {
        let temp_dir = TempDir::new().unwrap();
        let session_path = temp_dir.path().join("test-session");
        fs::create_dir_all(&session_path).unwrap();

        // Create existing gitignore that already has the entry
        let gitignore_path = session_path.join(".gitignore");
        fs::write(&gitignore_path, "*.log\n.vscode/tasks.json\n").unwrap();

        let config = create_test_config();
        let options = ClaudeLaunchOptions::default();

        let result = launch_claude_with_context(&config, &session_path, options);
        assert!(result.is_ok());

        // Verify gitignore wasn't duplicated
        let gitignore_content = fs::read_to_string(&gitignore_path).unwrap();
        // Count occurrences of .vscode/tasks.json
        let count = gitignore_content.matches(".vscode/tasks.json").count();
        assert_eq!(count, 1, "Should not duplicate .vscode/tasks.json entry");
    }

    #[test]
    fn test_resume_claude_double_resolution_bug() {
        let temp_dir = TempDir::new().unwrap();
        let session_path = temp_dir.path().join("test-session");
        fs::create_dir_all(&session_path).unwrap();

        // Create config with specific sandbox settings
        let mut config = create_test_config();
        config.sandbox = Some(crate::core::sandbox::SandboxConfig {
            enabled: true,
            profile: "standard".to_string(),
            allowed_domains: vec!["example.com".to_string()],
        });

        // This simulates what the resume command does INCORRECTLY:
        // It resolves sandbox settings first, then passes them to claude launcher
        // which resolves them AGAIN, causing double resolution
        let resolver = crate::core::sandbox::config::SandboxResolver::new(&config);
        let sandbox_settings = resolver.resolve_with_network(
            false,      // sandbox CLI flag not set
            false,      // no_sandbox CLI flag not set
            None,       // no CLI profile override
            false,      // no network sandbox CLI flag
            Vec::new(), // no CLI allowed domains
        );

        // This is what resume function creates (the bug):
        // It passes the RESOLVED settings as if they were CLI overrides
        let options = ClaudeLaunchOptions {
            session_id: Some("resume-session-123".to_string()),
            sandbox_override: Some(sandbox_settings.enabled), // BUG: This is already resolved!
            sandbox_profile: Some(sandbox_settings.profile),  // BUG: This is already resolved!
            network_sandbox: sandbox_settings.network_sandbox, // BUG: This is already resolved!
            allowed_domains: sandbox_settings.allowed_domains, // BUG: This is already resolved!
            ..Default::default()
        };

        let result = launch_claude_with_context(&config, &session_path, options);
        assert!(result.is_ok());

        // The tasks.json should be created properly despite the double resolution
        let tasks_content = fs::read_to_string(session_path.join(".vscode/tasks.json")).unwrap();
        assert!(tasks_content.contains("-r"));
        assert!(tasks_content.contains("resume-session-123"));

        // This test demonstrates the issue exists but doesn't fail because
        // the double resolution happens to work in most cases.
        // The real issue is architectural - resume shouldn't pre-resolve settings
        // before passing them to claude_launcher which resolves them again.
    }
}
