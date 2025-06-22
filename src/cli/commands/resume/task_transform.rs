use crate::utils::{ParaError, Result};
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

/// Handle Claude task JSON transformations
pub fn transform_claude_tasks_file(path: &Path) -> Result<()> {
    let tasks_file = path.join(".vscode/tasks.json");

    if !tasks_file.exists() {
        return Ok(());
    }

    let config = detect_task_configuration(&tasks_file)?;
    let transformation = determine_transformation(&config);
    apply_transformation(&tasks_file, transformation)
}

fn detect_task_configuration(tasks_file: &Path) -> Result<TaskConfiguration> {
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

fn determine_transformation(config: &TaskConfiguration) -> TaskTransformation {
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

fn apply_transformation(tasks_file: &Path, transformation: TaskTransformation) -> Result<()> {
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

        let config = detect_task_configuration(&tasks_file).unwrap();
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

        let config = detect_task_configuration(&tasks_file).unwrap();
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

        let config = detect_task_configuration(&tasks_file).unwrap();
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

        let config = detect_task_configuration(&tasks_file).unwrap();
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

        let config = detect_task_configuration(&tasks_file).unwrap();
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

        let config = detect_task_configuration(&tasks_file).unwrap();
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
        let transformation = determine_transformation(&config);
        matches!(
            transformation,
            TaskTransformation::RemovePromptFileAndAddContinue {
                has_skip_permissions: true
            }
        );

        let config = TaskConfiguration::HasPromptFile {
            has_skip_permissions: false,
        };
        let transformation = determine_transformation(&config);
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
        let transformation = determine_transformation(&config);
        matches!(transformation, TaskTransformation::NoChange);

        // Test NeedsTransformation -> AddContinueFlag
        let config = TaskConfiguration::NeedsTransformation {
            has_skip_permissions: false,
        };
        let transformation = determine_transformation(&config);
        matches!(
            transformation,
            TaskTransformation::AddContinueFlag {
                has_skip_permissions: false
            }
        );
    }

    #[test]
    fn test_apply_transformation_no_change() {
        let temp_dir = TempDir::new().unwrap();
        let tasks_file = temp_dir.path().join("tasks.json");

        let content = r#"{"tasks":[{"command":"claude -c"}]}"#;
        fs::write(&tasks_file, content).unwrap();

        let transformation = TaskTransformation::NoChange;
        let result = apply_transformation(&tasks_file, transformation);
        assert!(result.is_ok());

        // File should remain unchanged
        let updated_content = fs::read_to_string(&tasks_file).unwrap();
        assert_eq!(updated_content, content);
    }

    #[test]
    fn test_apply_remove_prompt_file_transformation() {
        let temp_dir = TempDir::new().unwrap();
        let tasks_file = temp_dir.path().join("tasks.json");

        // Test with skip permissions
        let content = r#"{
  "version": "2.0.0",
  "tasks": [
    {
      "label": "Claude Task",
      "command": "claude --dangerously-skip-permissions \"$(cat '/tmp/.claude_prompt_temp'; rm '/tmp/.claude_prompt_temp')\""
    },
    {
      "label": "Other Task", 
      "command": "echo hello"
    }
  ]
}"#;
        fs::write(&tasks_file, content).unwrap();

        let result = apply_remove_prompt_file_transformation(&tasks_file, true);
        assert!(result.is_ok());

        let updated_content = fs::read_to_string(&tasks_file).unwrap();
        assert!(updated_content.contains("claude --dangerously-skip-permissions -c"));
        assert!(!updated_content.contains(".claude_prompt_temp"));
        assert!(!updated_content.contains("$(cat"));
        assert!(!updated_content.contains("rm '"));
        assert!(updated_content.contains("echo hello")); // Other task unchanged

        // Test without skip permissions
        let content = r#"{
  "tasks": [{
    "command": "claude \"$(cat '/tmp/prompt'; rm '/tmp/prompt')\""
  }]
}"#;
        fs::write(&tasks_file, content).unwrap();

        let result = apply_remove_prompt_file_transformation(&tasks_file, false);
        assert!(result.is_ok());

        let updated_content = fs::read_to_string(&tasks_file).unwrap();
        assert!(updated_content.contains("\"claude -c\""));
    }

    #[test]
    fn test_apply_add_continue_flag_transformation() {
        let temp_dir = TempDir::new().unwrap();
        let tasks_file = temp_dir.path().join("tasks.json");

        // Test with skip permissions
        let content = r#"{
  "tasks": [
    {
      "command": "claude --dangerously-skip-permissions"
    },
    {
      "command": "claude --dangerously-skip-permissions some args"
    },
    {
      "command": "echo hello"
    }
  ]
}"#;
        fs::write(&tasks_file, content).unwrap();

        let result = apply_add_continue_flag_transformation(&tasks_file, true);
        assert!(result.is_ok());

        let updated_content = fs::read_to_string(&tasks_file).unwrap();
        assert!(updated_content.contains("claude --dangerously-skip-permissions -c"));
        assert!(updated_content.contains("claude --dangerously-skip-permissions -c some args"));
        assert!(updated_content.contains("echo hello")); // Unchanged

        // Test without skip permissions
        let content = r#"{
  "tasks": [
    {
      "command": "claude"
    },
    {
      "command": "claude some args"
    }
  ]
}"#;
        fs::write(&tasks_file, content).unwrap();

        let result = apply_add_continue_flag_transformation(&tasks_file, false);
        assert!(result.is_ok());

        let updated_content = fs::read_to_string(&tasks_file).unwrap();
        assert!(updated_content.contains("\"claude -c\""));
        assert!(updated_content.contains("\"claude -c some args\""));
    }

    #[test]
    fn test_apply_add_continue_flag_transformation_idempotent() {
        let temp_dir = TempDir::new().unwrap();
        let tasks_file = temp_dir.path().join("tasks.json");

        // Test that already having -c flag doesn't add another one
        let content = r#"{
  "tasks": [
    {
      "command": "claude -c"
    },
    {
      "command": "claude --dangerously-skip-permissions -c"
    }
  ]
}"#;
        fs::write(&tasks_file, content).unwrap();
        let original_content = fs::read_to_string(&tasks_file).unwrap();

        let result = apply_add_continue_flag_transformation(&tasks_file, false);
        assert!(result.is_ok());

        let updated_content = fs::read_to_string(&tasks_file).unwrap();
        assert_eq!(original_content, updated_content); // Should be unchanged
    }

    #[test]
    fn test_transformation_with_malformed_json() {
        let temp_dir = TempDir::new().unwrap();
        let tasks_file = temp_dir.path().join("tasks.json");

        // Test with malformed JSON
        let content = r#"{ "tasks": [ invalid json }"#;
        fs::write(&tasks_file, content).unwrap();

        // detect_task_configuration only does string matching, not JSON parsing
        // So it should succeed but return NeedsTransformation
        let result = detect_task_configuration(&tasks_file);
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            TaskConfiguration::NeedsTransformation {
                has_skip_permissions: false
            }
        );

        let result = apply_remove_prompt_file_transformation(&tasks_file, false);
        assert!(result.is_err());

        let result = apply_add_continue_flag_transformation(&tasks_file, false);
        assert!(result.is_err());
    }

    #[test]
    fn test_transformation_with_missing_tasks() {
        let temp_dir = TempDir::new().unwrap();
        let tasks_file = temp_dir.path().join("tasks.json");

        // Test with JSON that has no tasks array
        let content = r#"{ "version": "2.0.0" }"#;
        fs::write(&tasks_file, content).unwrap();

        let config = detect_task_configuration(&tasks_file).unwrap();
        assert_eq!(
            config,
            TaskConfiguration::NeedsTransformation {
                has_skip_permissions: false
            }
        );

        // Transformations should handle missing tasks gracefully
        let result = apply_remove_prompt_file_transformation(&tasks_file, false);
        assert!(result.is_ok());

        let result = apply_add_continue_flag_transformation(&tasks_file, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_transformation_with_missing_file() {
        let temp_dir = TempDir::new().unwrap();
        let tasks_file = temp_dir.path().join("nonexistent.json");

        let result = detect_task_configuration(&tasks_file);
        assert!(result.is_err());

        let result = apply_remove_prompt_file_transformation(&tasks_file, false);
        assert!(result.is_err());

        let result = apply_add_continue_flag_transformation(&tasks_file, false);
        assert!(result.is_err());
    }

    #[test]
    fn test_apply_add_continue_flag_transformation_edge_cases() {
        let temp_dir = TempDir::new().unwrap();
        let tasks_file = temp_dir.path().join("tasks.json");

        // Test with empty command string
        let content = r#"{
  "tasks": [
    {
      "command": ""
    },
    {
      "command": "claude"
    }
  ]
}"#;
        fs::write(&tasks_file, content).unwrap();

        let result = apply_add_continue_flag_transformation(&tasks_file, false);
        assert!(result.is_ok());

        let updated_content = fs::read_to_string(&tasks_file).unwrap();
        assert!(updated_content.contains("\"command\": \"\""));
        assert!(updated_content.contains("\"claude -c\""));

        // Test with non-string command field
        let content = r#"{
  "tasks": [
    {
      "command": ["array", "command"]
    },
    {
      "command": "claude"
    }
  ]
}"#;
        fs::write(&tasks_file, content).unwrap();

        let result = apply_add_continue_flag_transformation(&tasks_file, false);
        assert!(result.is_ok());

        let updated_content = fs::read_to_string(&tasks_file).unwrap();
        assert!(updated_content.contains("\"array\"") && updated_content.contains("\"command\""));
        assert!(updated_content.contains("\"claude -c\""));

        // Test with various Claude command variations
        let content = r#"{
  "tasks": [
    {
      "command": "claude --help"
    },
    {
      "command": "claude --verbose --other-flag"
    },
    {
      "command": "claude -c --already-has-flag"
    },
    {
      "command": "other-command"
    }
  ]
}"#;
        fs::write(&tasks_file, content).unwrap();

        let result = apply_add_continue_flag_transformation(&tasks_file, false);
        assert!(result.is_ok());

        let updated_content = fs::read_to_string(&tasks_file).unwrap();
        assert!(updated_content.contains("\"claude -c --help\""));
        assert!(updated_content.contains("\"claude -c --verbose --other-flag\""));
        assert!(updated_content.contains("\"claude -c --already-has-flag\""));
        assert!(updated_content.contains("\"other-command\""));
    }

    #[test]
    fn test_apply_add_continue_flag_transformation_complex_skip_permissions() {
        let temp_dir = TempDir::new().unwrap();
        let tasks_file = temp_dir.path().join("tasks.json");

        // Test various combinations with skip permissions
        let content = r#"{
  "tasks": [
    {
      "command": "claude --dangerously-skip-permissions --verbose"
    },
    {
      "command": "claude --dangerously-skip-permissions -c already-has"
    },
    {
      "command": "claude --other-flag --dangerously-skip-permissions"
    },
    {
      "command": "claude --dangerously-skip-permissions"
    }
  ]
}"#;
        fs::write(&tasks_file, content).unwrap();

        let result = apply_add_continue_flag_transformation(&tasks_file, true);
        assert!(result.is_ok());

        let updated_content = fs::read_to_string(&tasks_file).unwrap();
        // Should add -c after --dangerously-skip-permissions (contains exact match)
        assert!(updated_content.contains("\"claude --dangerously-skip-permissions -c --verbose\""));
        // Should not change if -c already exists
        assert!(
            updated_content.contains("\"claude --dangerously-skip-permissions -c already-has\"")
        );
        // Should NOT change if exact match not found (current behavior)
        assert!(updated_content.contains("\"claude --other-flag --dangerously-skip-permissions\""));
        // Should handle exact match
        assert!(updated_content.contains("\"claude --dangerously-skip-permissions -c\""));
    }

    // Tests for helper functions

    #[test]
    fn test_needs_continue_flag() {
        assert!(needs_continue_flag("claude"));
        assert!(needs_continue_flag("claude --verbose"));
        assert!(needs_continue_flag("claude --dangerously-skip-permissions"));

        assert!(!needs_continue_flag("claude -c"));
        assert!(!needs_continue_flag(
            "claude --dangerously-skip-permissions -c"
        ));
        assert!(!needs_continue_flag("claude -c --verbose"));
    }

    #[test]
    fn test_transform_claude_command_regular() {
        // Test exact match
        assert_eq!(transform_claude_command_regular("claude"), "claude -c");

        // Test with additional flags
        assert_eq!(
            transform_claude_command_regular("claude --verbose"),
            "claude -c --verbose"
        );
        assert_eq!(
            transform_claude_command_regular("claude --help"),
            "claude -c --help"
        );

        // Test already has -c flag (no change)
        assert_eq!(transform_claude_command_regular("claude -c"), "claude -c");
        assert_eq!(
            transform_claude_command_regular("claude -c --verbose"),
            "claude -c --verbose"
        );

        // Test non-Claude commands (no change)
        assert_eq!(transform_claude_command_regular("echo hello"), "echo hello");
        assert_eq!(transform_claude_command_regular(""), "");

        // Test edge cases
        assert_eq!(transform_claude_command_regular("claudetest"), "claudetest");
    }

    #[test]
    fn test_transform_claude_command_with_skip_permissions() {
        // Test with exact match
        assert_eq!(
            transform_claude_command_with_skip_permissions("claude --dangerously-skip-permissions"),
            "claude --dangerously-skip-permissions -c"
        );

        // Test with additional flags
        assert_eq!(
            transform_claude_command_with_skip_permissions(
                "claude --dangerously-skip-permissions --verbose"
            ),
            "claude --dangerously-skip-permissions -c --verbose"
        );

        // Test already has -c flag (no change)
        assert_eq!(
            transform_claude_command_with_skip_permissions(
                "claude --dangerously-skip-permissions -c"
            ),
            "claude --dangerously-skip-permissions -c"
        );

        // Test partial match that doesn't get transformed (current behavior)
        assert_eq!(
            transform_claude_command_with_skip_permissions(
                "claude --other-flag --dangerously-skip-permissions"
            ),
            "claude --other-flag --dangerously-skip-permissions"
        );

        // Test non-matching commands (no change)
        assert_eq!(
            transform_claude_command_with_skip_permissions("claude"),
            "claude"
        );
        assert_eq!(
            transform_claude_command_with_skip_permissions("echo hello"),
            "echo hello"
        );
    }

    #[test]
    fn test_transform_claude_command() {
        // Test with skip permissions = true
        assert_eq!(
            transform_claude_command("claude --dangerously-skip-permissions", true),
            "claude --dangerously-skip-permissions -c"
        );

        // Test with skip permissions = false
        assert_eq!(transform_claude_command("claude", false), "claude -c");
        assert_eq!(
            transform_claude_command("claude --verbose", false),
            "claude -c --verbose"
        );

        // Test non-matching commands
        assert_eq!(transform_claude_command("echo hello", true), "echo hello");
        assert_eq!(transform_claude_command("echo hello", false), "echo hello");
    }

    #[test]
    fn test_load_and_save_tasks_json() {
        let temp_dir = TempDir::new().unwrap();
        let tasks_file = temp_dir.path().join("tasks.json");

        // Test with valid JSON
        let original_content = r#"{
  "version": "2.0.0",
  "tasks": [
    {
      "label": "Test Task",
      "command": "claude"
    }
  ]
}"#;
        fs::write(&tasks_file, original_content).unwrap();

        // Load JSON
        let json = load_tasks_json(&tasks_file).unwrap();
        assert!(json.get("version").is_some());
        assert!(json.get("tasks").is_some());

        // Save JSON back
        let result = save_tasks_json(&tasks_file, json);
        assert!(result.is_ok());

        // Verify it can be read again
        let content_after_save = fs::read_to_string(&tasks_file).unwrap();
        assert!(content_after_save.contains("\"version\""));
        assert!(content_after_save.contains("\"tasks\""));
        assert!(content_after_save.contains("\"claude\""));
    }

    #[test]
    fn test_load_tasks_json_with_invalid_file() {
        let temp_dir = TempDir::new().unwrap();
        let tasks_file = temp_dir.path().join("nonexistent.json");

        let result = load_tasks_json(&tasks_file);
        assert!(result.is_err());
    }

    #[test]
    fn test_load_tasks_json_with_invalid_json() {
        let temp_dir = TempDir::new().unwrap();
        let tasks_file = temp_dir.path().join("tasks.json");

        fs::write(&tasks_file, "{ invalid json }").unwrap();

        let result = load_tasks_json(&tasks_file);
        assert!(result.is_err());
    }
}
