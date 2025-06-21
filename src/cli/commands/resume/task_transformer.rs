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

pub struct TaskTransformer;

impl TaskTransformer {
    pub fn new() -> Self {
        Self
    }

    pub fn update_tasks_json_for_resume(&self, path: &Path) -> Result<()> {
        let tasks_file = path.join(".vscode/tasks.json");

        if !tasks_file.exists() {
            return Ok(());
        }

        let config = self.detect_task_configuration(&tasks_file)?;
        let transformation = self.determine_transformation(&config);
        self.apply_transformation(&tasks_file, transformation)
    }

    pub fn detect_task_configuration(&self, tasks_file: &Path) -> Result<TaskConfiguration> {
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

    pub fn determine_transformation(&self, config: &TaskConfiguration) -> TaskTransformation {
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

    pub fn apply_transformation(
        &self,
        tasks_file: &Path,
        transformation: TaskTransformation,
    ) -> Result<()> {
        match transformation {
            TaskTransformation::NoChange => Ok(()),
            TaskTransformation::RemovePromptFileAndAddContinue {
                has_skip_permissions,
            } => self.apply_remove_prompt_file_transformation(tasks_file, has_skip_permissions),
            TaskTransformation::AddContinueFlag {
                has_skip_permissions,
            } => self.apply_add_continue_flag_transformation(tasks_file, has_skip_permissions),
        }
    }

    fn apply_remove_prompt_file_transformation(
        &self,
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

    fn apply_add_continue_flag_transformation(
        &self,
        tasks_file: &Path,
        has_skip_permissions: bool,
    ) -> Result<()> {
        let mut json = self.load_tasks_json(tasks_file)?;

        // Navigate to tasks array and update command fields
        if let Some(tasks) = json.get_mut("tasks").and_then(|t| t.as_array_mut()) {
            for task in tasks {
                if let Some(command_value) = task.get_mut("command") {
                    // Only transform string commands, preserve arrays and other types unchanged
                    if let Some(command_str) = command_value.as_str() {
                        let updated_command =
                            self.transform_claude_command(command_str, has_skip_permissions);

                        if updated_command != command_str {
                            *command_value = Value::String(updated_command);
                        }
                    }
                    // Arrays and other non-string values are left unchanged
                }
            }
        }

        self.save_tasks_json(tasks_file, json)
    }

    fn load_tasks_json(&self, tasks_file: &Path) -> Result<Value> {
        let content = fs::read_to_string(tasks_file)
            .map_err(|e| ParaError::fs_error(format!("Failed to read tasks.json: {}", e)))?;

        serde_json::from_str(&content)
            .map_err(|e| ParaError::fs_error(format!("Failed to parse tasks.json: {}", e)))
    }

    fn save_tasks_json(&self, tasks_file: &Path, json: Value) -> Result<()> {
        let updated_content = serde_json::to_string_pretty(&json)
            .map_err(|e| ParaError::fs_error(format!("Failed to serialize tasks.json: {}", e)))?;

        fs::write(tasks_file, updated_content)
            .map_err(|e| ParaError::fs_error(format!("Failed to update tasks.json: {}", e)))
    }

    fn needs_continue_flag(&self, command: &str) -> bool {
        !command.contains("-c")
    }

    fn transform_claude_command(&self, command: &str, has_skip_permissions: bool) -> String {
        if has_skip_permissions {
            self.transform_claude_command_with_skip_permissions(command)
        } else {
            self.transform_claude_command_regular(command)
        }
    }

    fn transform_claude_command_with_skip_permissions(&self, command: &str) -> String {
        if command.contains("claude --dangerously-skip-permissions")
            && self.needs_continue_flag(command)
        {
            command.replace(
                "claude --dangerously-skip-permissions",
                "claude --dangerously-skip-permissions -c",
            )
        } else {
            command.to_string()
        }
    }

    fn transform_claude_command_regular(&self, command: &str) -> String {
        if command == "claude" {
            "claude -c".to_string()
        } else if command.starts_with("claude ") && self.needs_continue_flag(command) {
            command.replace("claude ", "claude -c ")
        } else {
            command.to_string()
        }
    }
}

impl Default for TaskTransformer {
    fn default() -> Self {
        Self::new()
    }
}