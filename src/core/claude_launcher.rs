use crate::config::Config;
use crate::core::sandbox::config::SandboxResolver;
use crate::core::sandbox::launcher::{is_sandbox_available, wrap_with_sandbox};
use crate::utils::{ParaError, Result};
use std::fs;
use std::path::Path;
use std::process::Command;

/// Options for launching Claude with different continuation modes
#[derive(Debug, Default)]
pub struct ClaudeLaunchOptions {
    pub skip_permissions: bool,
    pub session_id: Option<String>,
    pub continue_conversation: bool,
    pub prompt_content: Option<String>,
    pub sandbox_override: Option<bool>,
    pub sandbox_profile: Option<String>,
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
        .map_err(|e| ParaError::fs_error(format!("Failed to create .vscode directory: {}", e)))?;

    // Resolve sandbox settings using the resolver
    let resolver = SandboxResolver::new(config);
    let sandbox_settings = resolver.resolve(
        options.sandbox_override.unwrap_or(false),
        options.sandbox_override == Some(false),
        options.sandbox_profile.clone(),
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
                ParaError::fs_error(format!("Failed to write temp prompt file: {}", e))
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
                format!("{} -r \"{}\"", base_cmd, session_id)
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
                format!("{} -c", base_cmd)
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
            format!("{} -c", base_cmd)
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

    // Apply sandboxing if enabled
    let final_command = if should_sandbox && is_sandbox_available() {
        match wrap_with_sandbox(&claude_task_cmd, session_path, &sandbox_settings.profile) {
            Ok(sandboxed_cmd) => {
                println!(
                    "🔒 Sandboxing enabled for Claude CLI (profile: {})",
                    sandbox_settings.profile
                );
                sandboxed_cmd
            }
            Err(e) => {
                eprintln!("⚠️  Warning: Failed to apply sandbox: {}", e);
                eprintln!("   Continuing without sandboxing");
                claude_task_cmd
            }
        }
    } else {
        claude_task_cmd
    };

    // Create tasks.json with the command
    let tasks_json = create_claude_task_json(&final_command);
    let tasks_file = vscode_dir.join("tasks.json");
    fs::write(&tasks_file, tasks_json)
        .map_err(|e| ParaError::fs_error(format!("Failed to write tasks.json: {}", e)))?;

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
        });

        let options = ClaudeLaunchOptions {
            skip_permissions: true,
            session_id: Some("test-123".to_string()),
            continue_conversation: false,
            prompt_content: Some("Test prompt".to_string()),
            sandbox_override: None,
            sandbox_profile: None,
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
}
