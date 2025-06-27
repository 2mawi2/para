use crate::config::Config;
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

    // Create tasks.json with the command
    let tasks_json = create_claude_task_json(&claude_task_cmd);
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

    #[test]
    fn test_claude_launch_options_default() {
        let options = ClaudeLaunchOptions::default();
        assert!(!options.skip_permissions);
        assert!(options.session_id.is_none());
        assert!(!options.continue_conversation);
        assert!(options.prompt_content.is_none());
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
}
