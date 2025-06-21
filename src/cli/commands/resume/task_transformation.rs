// This file was moved to resume_modules/task_transformation.rs
use serde_json::Value;
use std::fs;
use std::path::Path;

#[derive(Debug, PartialEq)]
pub enum TaskConfiguration {
    HasPromptFile { has_skip_permissions: bool },
    HasContinueFlag { has_skip_permissions: bool },
    NeedsTransformation { has_skip_permissions: bool },
}

#[derive(Debug)]
pub enum TaskTransformation {
    RemovePromptFileAndAddContinue { has_skip_permissions: bool },
    AddContinueFlag { has_skip_permissions: bool },
    NoChange,
}

/// Updates tasks.json for resume workflow - adds continue flag and removes prompt file logic
pub fn update_tasks_json_for_resume(path: &Path) -> Result<()> {
    let tasks_file = path.join(".vscode/tasks.json");

    if !tasks_file.exists() {
        return Ok(());
    }

    let config = detect_task_configuration(&tasks_file)?;
    let transformation = determine_transformation(&config);
    apply_transformation(&tasks_file, transformation)
}

/// Analyzes a tasks.json file to determine its current configuration
pub fn detect_task_configuration(tasks_file: &Path) -> Result<TaskConfiguration> {
    let content = fs::read_to_string(tasks_file)
        .map_err(|e| ParaError::fs_error(format!("Failed to read tasks.json: {}", e)))?;

    let has_prompt_file = content.contains(".claude_prompt_temp")
        || (content.contains("$(cat") && content.contains("rm "));
    let has_continue_flag = content.contains(" -c");
    let has_skip_permissions = content.contains("--dangerously-skip-permissions");

    if has_prompt_file {
        Ok(TaskConfiguration::HasPromptFile {
            has_skip_permissions,
        })
    } else if has_continue_flag {
        Ok(TaskConfiguration::HasContinueFlag {
            has_skip_permissions,
        })
    } else {
        Ok(TaskConfiguration::NeedsTransformation {
            has_skip_permissions,
        })
    }
}

/// Determines what transformation is needed based on the task configuration
pub fn determine_transformation(config: &TaskConfiguration) -> TaskTransformation {
    match config {
        TaskConfiguration::HasPromptFile {
            has_skip_permissions,
        } => TaskTransformation::RemovePromptFileAndAddContinue {
            has_skip_permissions: *has_skip_permissions,
        },
        TaskConfiguration::HasContinueFlag { .. } => TaskTransformation::NoChange,
        TaskConfiguration::NeedsTransformation {
            has_skip_permissions,
        } => TaskTransformation::AddContinueFlag {
            has_skip_permissions: *has_skip_permissions,
        },
    }
}

/// Applies the specified transformation to the tasks.json file
pub fn apply_transformation(tasks_file: &Path, transformation: TaskTransformation) -> Result<()> {
    match transformation {
        TaskTransformation::NoChange => Ok(()),
        TaskTransformation::RemovePromptFileAndAddContinue {
            has_skip_permissions,
        } => apply_remove_prompt_file_transformation(tasks_file, has_skip_permissions),
        TaskTransformation::AddContinueFlag {
            has_skip_permissions,
        } => apply_add_continue_flag_transformation(tasks_file, has_skip_permissions),
    }
}

/// Removes prompt file logic and adds continue flag
fn apply_remove_prompt_file_transformation(
    tasks_file: &Path,
    has_skip_permissions: bool,
) -> Result<()> {
    let content = fs::read_to_string(tasks_file)
        .map_err(|e| ParaError::fs_error(format!("Failed to read tasks.json: {}", e)))?;

    let mut json: Value = serde_json::from_str(&content)
        .map_err(|e| ParaError::fs_error(format!("Failed to parse tasks.json: {}", e)))?;

    let new_command = if has_skip_permissions {
        "claude --dangerously-skip-permissions -c"
    } else {
        "claude -c"
    };

    // Navigate to tasks array and update command fields
    if let Some(tasks) = json.get_mut("tasks").and_then(|t| t.as_array_mut()) {
        for task in tasks {
            if let Some(command) = task.get_mut("command").and_then(|c| c.as_str()) {
                if command.contains(".claude_prompt_temp")
                    || (command.contains("$(cat") && command.contains("rm "))
                {
                    task["command"] = Value::String(new_command.to_string());
                }
            }
        }
    }

    let updated_content = serde_json::to_string_pretty(&json)
        .map_err(|e| ParaError::fs_error(format!("Failed to serialize tasks.json: {}", e)))?;

    fs::write(tasks_file, updated_content)
        .map_err(|e| ParaError::fs_error(format!("Failed to update tasks.json: {}", e)))
}

/// Loads and parses a tasks.json file
fn load_tasks_json(tasks_file: &Path) -> Result<Value> {
    let content = fs::read_to_string(tasks_file)
        .map_err(|e| ParaError::fs_error(format!("Failed to read tasks.json: {}", e)))?;

    serde_json::from_str(&content)
        .map_err(|e| ParaError::fs_error(format!("Failed to parse tasks.json: {}", e)))
}

/// Saves a JSON value to a tasks.json file with pretty formatting
fn save_tasks_json(tasks_file: &Path, json: Value) -> Result<()> {
    let updated_content = serde_json::to_string_pretty(&json)
        .map_err(|e| ParaError::fs_error(format!("Failed to serialize tasks.json: {}", e)))?;

    fs::write(tasks_file, updated_content)
        .map_err(|e| ParaError::fs_error(format!("Failed to update tasks.json: {}", e)))
}

/// Checks if a command needs the continue flag added
fn needs_continue_flag(command: &str) -> bool {
    !command.contains("-c")
}

/// Transforms a Claude command to include the continue flag
fn transform_claude_command(command: &str, has_skip_permissions: bool) -> String {
    if has_skip_permissions {
        transform_claude_command_with_skip_permissions(command)
    } else {
        transform_claude_command_regular(command)
    }
}

/// Transforms Claude commands with --dangerously-skip-permissions flag
fn transform_claude_command_with_skip_permissions(command: &str) -> String {
    if command.contains("claude --dangerously-skip-permissions") && needs_continue_flag(command) {
        command.replace(
            "claude --dangerously-skip-permissions",
            "claude --dangerously-skip-permissions -c",
        )
    } else {
        command.to_string()
    }
}

/// Transforms regular Claude commands (without --dangerously-skip-permissions)
fn transform_claude_command_regular(command: &str) -> String {
    if command == "claude" {
        "claude -c".to_string()
    } else if command.starts_with("claude ") && needs_continue_flag(command) {
        command.replace("claude ", "claude -c ")
    } else {
        command.to_string()
    }
}

/// Adds continue flag to Claude commands that need it
fn apply_add_continue_flag_transformation(
    tasks_file: &Path,
    has_skip_permissions: bool,
) -> Result<()> {
    let mut json = load_tasks_json(tasks_file)?;

    // Navigate to tasks array and update command fields
    if let Some(tasks) = json.get_mut("tasks").and_then(|t| t.as_array_mut()) {
        for task in tasks {
            if let Some(command_value) = task.get_mut("command") {
                // Only transform string commands, preserve arrays and other types unchanged
                if let Some(command_str) = command_value.as_str() {
                    let updated_command =
                        transform_claude_command(command_str, has_skip_permissions);

                    if updated_command != command_str {
                        *command_value = Value::String(updated_command);
                    }
                }
                // Arrays and other non-string values are left unchanged
            }
        }
    }

    save_tasks_json(tasks_file, json)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_detect_task_configuration_has_prompt_file() {
        let temp_dir = TempDir::new().unwrap();
        let tasks_file = temp_dir.path().join("tasks.json");

        // Test prompt file detection with skip permissions
        let content = r#"{
  "tasks": [{
    "command": "claude --dangerously-skip-permissions \"$(cat '/path/.claude_prompt_temp'; rm '/path/.claude_prompt_temp')\""
  }]
}"#;
        fs::write(&tasks_file, content).unwrap();

        let config = super::detect_task_configuration(&tasks_file).unwrap();
        assert_eq!(
            config,
            TaskConfiguration::HasPromptFile {
                has_skip_permissions: true
            }
        );

        // Test prompt file detection without skip permissions
        let content = r#"{
  "tasks": [{
    "command": "claude \"$(cat '/path/to/prompt'; rm '/path/to/prompt')\""
  }]
}"#;
        fs::write(&tasks_file, content).unwrap();

        let config = super::detect_task_configuration(&tasks_file).unwrap();
        assert_eq!(
            config,
            TaskConfiguration::HasPromptFile {
                has_skip_permissions: false
            }
        );
    }

    #[test]
    fn test_detect_task_configuration_has_continue_flag() {
        let temp_dir = TempDir::new().unwrap();
        let tasks_file = temp_dir.path().join("tasks.json");

        // Test continue flag detection with skip permissions
        let content = r#"{
  "tasks": [{
    "command": "claude --dangerously-skip-permissions -c"
  }]
}"#;
        fs::write(&tasks_file, content).unwrap();

        let config = super::detect_task_configuration(&tasks_file).unwrap();
        assert_eq!(
            config,
            TaskConfiguration::HasContinueFlag {
                has_skip_permissions: true
            }
        );

        // Test continue flag detection without skip permissions
        let content = r#"{
  "tasks": [{
    "command": "claude -c"
  }]
}"#;
        fs::write(&tasks_file, content).unwrap();

        let config = super::detect_task_configuration(&tasks_file).unwrap();
        assert_eq!(
            config,
            TaskConfiguration::HasContinueFlag {
                has_skip_permissions: false
            }
        );
    }

    #[test]
    fn test_detect_task_configuration_needs_transformation() {
        let temp_dir = TempDir::new().unwrap();
        let tasks_file = temp_dir.path().join("tasks.json");

        // Test needs transformation with skip permissions
        let content = r#"{
  "tasks": [{
    "command": "claude --dangerously-skip-permissions"
  }]
}"#;
        fs::write(&tasks_file, content).unwrap();

        let config = super::detect_task_configuration(&tasks_file).unwrap();
        assert_eq!(
            config,
            TaskConfiguration::NeedsTransformation {
                has_skip_permissions: true
            }
        );

        // Test needs transformation without skip permissions
        let content = r#"{
  "tasks": [{
    "command": "claude"
  }]
}"#;
        fs::write(&tasks_file, content).unwrap();

        let config = super::detect_task_configuration(&tasks_file).unwrap();
        assert_eq!(
            config,
            TaskConfiguration::NeedsTransformation {
                has_skip_permissions: false
            }
        );
    }

    #[test]
    fn test_determine_transformation() {
        // Test HasPromptFile -> RemovePromptFileAndAddContinue
        let config = TaskConfiguration::HasPromptFile {
            has_skip_permissions: true,
        };
        let transformation = super::determine_transformation(&config);
        matches!(
            transformation,
            TaskTransformation::RemovePromptFileAndAddContinue {
                has_skip_permissions: true
            }
        );

        let config = TaskConfiguration::HasPromptFile {
            has_skip_permissions: false,
        };
        let transformation = super::determine_transformation(&config);
        matches!(
            transformation,
            TaskTransformation::RemovePromptFileAndAddContinue {
                has_skip_permissions: false
            }
        );

        // Test HasContinueFlag -> NoChange
        let config = TaskConfiguration::HasContinueFlag {
            has_skip_permissions: true,
        };
        let transformation = super::determine_transformation(&config);
        matches!(transformation, TaskTransformation::NoChange);

        // Test NeedsTransformation -> AddContinueFlag
        let config = TaskConfiguration::NeedsTransformation {
            has_skip_permissions: false,
        };
        let transformation = super::determine_transformation(&config);
        matches!(
            transformation,
            TaskTransformation::AddContinueFlag {
                has_skip_permissions: false
            }
        );
    }

    #[test]
    fn test_needs_continue_flag() {
        assert!(super::needs_continue_flag("claude"));
        assert!(super::needs_continue_flag("claude --verbose"));
        assert!(super::needs_continue_flag(
            "claude --dangerously-skip-permissions"
        ));

        assert!(!super::needs_continue_flag("claude -c"));
        assert!(!super::needs_continue_flag(
            "claude --dangerously-skip-permissions -c"
        ));
        assert!(!super::needs_continue_flag("claude -c --verbose"));
    }

    #[test]
    fn test_transform_claude_command_regular() {
        // Test exact match
        assert_eq!(
            super::transform_claude_command_regular("claude"),
            "claude -c"
        );

        // Test with additional flags
        assert_eq!(
            super::transform_claude_command_regular("claude --verbose"),
            "claude -c --verbose"
        );
        assert_eq!(
            super::transform_claude_command_regular("claude --help"),
            "claude -c --help"
        );

        // Test already has -c flag (no change)
        assert_eq!(
            super::transform_claude_command_regular("claude -c"),
            "claude -c"
        );
        assert_eq!(
            super::transform_claude_command_regular("claude -c --verbose"),
            "claude -c --verbose"
        );

        // Test non-Claude commands (no change)
        assert_eq!(
            super::transform_claude_command_regular("echo hello"),
            "echo hello"
        );
        assert_eq!(super::transform_claude_command_regular(""), "");

        // Test edge cases
        assert_eq!(
            super::transform_claude_command_regular("claudetest"),
            "claudetest"
        );
    }

    #[test]
    fn test_transform_claude_command_with_skip_permissions() {
        // Test with exact match
        assert_eq!(
            super::transform_claude_command_with_skip_permissions(
                "claude --dangerously-skip-permissions"
            ),
            "claude --dangerously-skip-permissions -c"
        );

        // Test with additional flags
        assert_eq!(
            super::transform_claude_command_with_skip_permissions(
                "claude --dangerously-skip-permissions --verbose"
            ),
            "claude --dangerously-skip-permissions -c --verbose"
        );

        // Test already has -c flag (no change)
        assert_eq!(
            super::transform_claude_command_with_skip_permissions(
                "claude --dangerously-skip-permissions -c"
            ),
            "claude --dangerously-skip-permissions -c"
        );

        // Test partial match that doesn't get transformed (current behavior)
        assert_eq!(
            super::transform_claude_command_with_skip_permissions(
                "claude --other-flag --dangerously-skip-permissions"
            ),
            "claude --other-flag --dangerously-skip-permissions"
        );

        // Test non-matching commands (no change)
        assert_eq!(
            super::transform_claude_command_with_skip_permissions("claude"),
            "claude"
        );
        assert_eq!(
            super::transform_claude_command_with_skip_permissions("echo hello"),
            "echo hello"
        );
    }

    #[test]
    fn test_transform_claude_command() {
        // Test with skip permissions = true
        assert_eq!(
            super::transform_claude_command("claude --dangerously-skip-permissions", true),
            "claude --dangerously-skip-permissions -c"
        );

        // Test with skip permissions = false
        assert_eq!(
            super::transform_claude_command("claude", false),
            "claude -c"
        );
        assert_eq!(
            super::transform_claude_command("claude --verbose", false),
            "claude -c --verbose"
        );

        // Test non-matching commands
        assert_eq!(
            super::transform_claude_command("echo hello", true),
            "echo hello"
        );
        assert_eq!(
            super::transform_claude_command("echo hello", false),
            "echo hello"
        );
    }
}