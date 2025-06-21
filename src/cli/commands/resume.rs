use crate::cli::parser::ResumeArgs;
use crate::config::Config;
use crate::core::git::GitService;
use crate::core::session::SessionManager;
use crate::utils::Result;

mod orchestrator;
mod session_detector;
mod task_transformer;

use orchestrator::ResumeOrchestrator;

pub fn execute(config: Config, args: ResumeArgs) -> Result<()> {
    let git_service = GitService::discover()?;
    let session_manager = SessionManager::new(&config);
    
    let orchestrator = ResumeOrchestrator::new(&config, &git_service, &session_manager);
    orchestrator.execute(&args)
}


#[cfg(test)]
mod tests {
    use super::*;
    use super::task_transformer::{TaskConfiguration, TaskTransformation, TaskTransformer};
    use crate::config::{
        Config, DirectoryConfig, GitConfig, IdeConfig, SessionConfig, WrapperConfig,
    };
    use std::fs;
    use std::process::Command;
    use tempfile::TempDir;

    fn setup_test_repo() -> (TempDir, TempDir, GitService, Config) {
        let git_dir = TempDir::new().expect("tmp git");
        let state_dir = TempDir::new().expect("tmp state");
        let repo_path = git_dir.path();
        Command::new("git")
            .current_dir(repo_path)
            .args(["init", "--initial-branch=main"])
            .status()
            .unwrap();
        Command::new("git")
            .current_dir(repo_path)
            .args(["config", "user.name", "Test"])
            .status()
            .unwrap();
        Command::new("git")
            .current_dir(repo_path)
            .args(["config", "user.email", "test@example.com"])
            .status()
            .unwrap();
        fs::write(repo_path.join("README.md"), "# Test").unwrap();
        Command::new("git")
            .current_dir(repo_path)
            .args(["add", "README.md"])
            .status()
            .unwrap();
        Command::new("git")
            .current_dir(repo_path)
            .args(["commit", "-m", "init"])
            .status()
            .unwrap();

        let config = Config {
            ide: IdeConfig {
                name: "test".into(),
                command: "echo".into(),
                user_data_dir: None,
                wrapper: WrapperConfig {
                    enabled: true,
                    name: "cursor".into(),
                    command: "echo".into(),
                },
            },
            directories: DirectoryConfig {
                subtrees_dir: "subtrees/para".into(),
                state_dir: state_dir
                    .path()
                    .join(".para_state")
                    .to_string_lossy()
                    .to_string(),
            },
            git: GitConfig {
                branch_prefix: "para".into(),
                auto_stage: true,
                auto_commit: false,
            },
            session: SessionConfig {
                default_name_format: "%Y%m%d-%H%M%S".into(),
                preserve_on_finish: false,
                auto_cleanup_days: None,
            },
        };
        let service = GitService::discover_from(repo_path).unwrap();
        (git_dir, state_dir, service, config)
    }

    #[test]
    fn test_resume_base_name_fallback() {
        let (_git_tmp, _state_tmp, git_service, config) = setup_test_repo();
        let session_manager = SessionManager::new(&config);

        // create timestamped session state only
        let session_full = "test4_20250611-131147".to_string();
        let branch_name = "para/test-branch".to_string();
        let worktree_path = git_service
            .repository()
            .root
            .join(&config.directories.subtrees_dir)
            .join(&config.git.branch_prefix)
            .join(&session_full);

        git_service
            .create_worktree(&branch_name, &worktree_path)
            .unwrap();

        let state = crate::core::session::state::SessionState::new(
            session_full.clone(),
            branch_name,
            worktree_path.clone(),
        );
        session_manager.save_state(&state).unwrap();

        // now resume with base name using the new orchestrator
        let orchestrator = ResumeOrchestrator::new(&config, &git_service, &session_manager);
        let args = crate::cli::parser::ResumeArgs {
            session: Some("test4".to_string()),
        };
        orchestrator.execute(&args).unwrap();
    }

    #[test]
    fn test_update_tasks_json_for_resume() {
        let temp_dir = TempDir::new().unwrap();
        let vscode_dir = temp_dir.path().join(".vscode");
        fs::create_dir_all(&vscode_dir).unwrap();

        // Test with dangerously-skip-permissions flag
        let tasks_file = vscode_dir.join("tasks.json");
        let original_content = r#"{
  "version": "2.0.0",
  "tasks": [
    {
      "label": "Start Claude Code with Prompt",
      "type": "shell",
      "command": "claude --dangerously-skip-permissions \"$(cat '/path/to/prompt'; rm '/path/to/prompt')\"",
      "runOptions": {
        "runOn": "folderOpen"
      }
    }
  ]
}"#;
        fs::write(&tasks_file, original_content).unwrap();

        // Update the tasks.json using TaskTransformer
        let task_transformer = TaskTransformer::new();
        task_transformer.update_tasks_json_for_resume(temp_dir.path()).unwrap();

        // Check it was updated
        let updated_content = fs::read_to_string(&tasks_file).unwrap();
        assert!(updated_content.contains("claude --dangerously-skip-permissions -c"));
        assert!(!updated_content.contains("claude --dangerously-skip-permissions \""));

        // Test idempotency - running again shouldn't change it
        task_transformer.update_tasks_json_for_resume(temp_dir.path()).unwrap();
        let content_after_second_update = fs::read_to_string(&tasks_file).unwrap();
        assert_eq!(updated_content, content_after_second_update);
    }

    #[test]
    fn test_update_tasks_json_removes_prompt_file() {
        let temp_dir = TempDir::new().unwrap();
        let vscode_dir = temp_dir.path().join(".vscode");
        fs::create_dir_all(&vscode_dir).unwrap();

        // Test with prompt file command from dispatch
        let tasks_file = vscode_dir.join("tasks.json");
        let original_content = r#"{
  "version": "2.0.0",
  "tasks": [
    {
      "label": "Start Claude Code with Prompt",
      "type": "shell",
      "command": "claude --dangerously-skip-permissions \"$(cat '/path/.claude_prompt_temp'; rm '/path/.claude_prompt_temp')\"",
      "runOptions": {
        "runOn": "folderOpen"
      }
    }
  ]
}"#;
        fs::write(&tasks_file, original_content).unwrap();

        // Update the tasks.json using TaskTransformer
        let task_transformer = TaskTransformer::new();
        task_transformer.update_tasks_json_for_resume(temp_dir.path()).unwrap();

        // Check prompt file logic was removed and -c flag added
        let updated_content = fs::read_to_string(&tasks_file).unwrap();
        assert!(!updated_content.contains(".claude_prompt_temp"));
        assert!(!updated_content.contains("$(cat"));
        assert!(!updated_content.contains("rm '"));
        assert!(
            updated_content.contains("\"command\": \"claude --dangerously-skip-permissions -c\",")
        );
    }

    // Unit tests for new refactored functions

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

        let task_transformer = TaskTransformer::new();
        let config = task_transformer.detect_task_configuration(&tasks_file).unwrap();
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

        let config = task_transformer.detect_task_configuration(&tasks_file).unwrap();
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

        let task_transformer = TaskTransformer::new();
        let config = task_transformer.detect_task_configuration(&tasks_file).unwrap();
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

        let config = task_transformer.detect_task_configuration(&tasks_file).unwrap();
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

        let task_transformer = TaskTransformer::new();
        let config = task_transformer.detect_task_configuration(&tasks_file).unwrap();
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

        let config = task_transformer.detect_task_configuration(&tasks_file).unwrap();
        assert_eq!(
            config,
            TaskConfiguration::NeedsTransformation {
                has_skip_permissions: false
            }
        );
    }

    #[test]
    fn test_determine_transformation() {
        let task_transformer = TaskTransformer::new();
        
        // Test HasPromptFile -> RemovePromptFileAndAddContinue
        let config = TaskConfiguration::HasPromptFile {
            has_skip_permissions: true,
        };
        let transformation = task_transformer.determine_transformation(&config);
        matches!(
            transformation,
            TaskTransformation::RemovePromptFileAndAddContinue {
                has_skip_permissions: true
            }
        );

        let config = TaskConfiguration::HasPromptFile {
            has_skip_permissions: false,
        };
        let transformation = task_transformer.determine_transformation(&config);
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
        let transformation = task_transformer.determine_transformation(&config);
        matches!(transformation, TaskTransformation::NoChange);

        // Test NeedsTransformation -> AddContinueFlag
        let config = TaskConfiguration::NeedsTransformation {
            has_skip_permissions: false,
        };
        let transformation = task_transformer.determine_transformation(&config);
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

        let task_transformer = TaskTransformer::new();
        let transformation = TaskTransformation::NoChange;
        let result = task_transformer.apply_transformation(&tasks_file, transformation);
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

        let task_transformer = TaskTransformer::new();
        let transformation = TaskTransformation::RemovePromptFileAndAddContinue { has_skip_permissions: true };
        let result = task_transformer.apply_transformation(&tasks_file, transformation);
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

        let transformation = TaskTransformation::RemovePromptFileAndAddContinue { has_skip_permissions: false };
        let result = task_transformer.apply_transformation(&tasks_file, transformation);
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

        let task_transformer = TaskTransformer::new();
        let transformation = TaskTransformation::AddContinueFlag { has_skip_permissions: true };
        let result = task_transformer.apply_transformation(&tasks_file, transformation);
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

        let transformation = TaskTransformation::AddContinueFlag { has_skip_permissions: false };
        let result = task_transformer.apply_transformation(&tasks_file, transformation);
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

        let task_transformer = TaskTransformer::new();
        let transformation = TaskTransformation::AddContinueFlag { has_skip_permissions: false };
        let result = task_transformer.apply_transformation(&tasks_file, transformation);
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
        let task_transformer = TaskTransformer::new();
        let result = task_transformer.detect_task_configuration(&tasks_file);
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            TaskConfiguration::NeedsTransformation {
                has_skip_permissions: false
            }
        );

        let transformation = TaskTransformation::RemovePromptFileAndAddContinue { has_skip_permissions: false };
        let result = task_transformer.apply_transformation(&tasks_file, transformation);
        assert!(result.is_err());

        let transformation = TaskTransformation::AddContinueFlag { has_skip_permissions: false };
        let result = task_transformer.apply_transformation(&tasks_file, transformation);
        assert!(result.is_err());
    }

    #[test]
    fn test_transformation_with_missing_tasks() {
        let temp_dir = TempDir::new().unwrap();
        let tasks_file = temp_dir.path().join("tasks.json");

        // Test with JSON that has no tasks array
        let content = r#"{ "version": "2.0.0" }"#;
        fs::write(&tasks_file, content).unwrap();

        let task_transformer = TaskTransformer::new();
        let config = task_transformer.detect_task_configuration(&tasks_file).unwrap();
        assert_eq!(
            config,
            TaskConfiguration::NeedsTransformation {
                has_skip_permissions: false
            }
        );

        // Transformations should handle missing tasks gracefully
        let transformation = TaskTransformation::RemovePromptFileAndAddContinue { has_skip_permissions: false };
        let result = task_transformer.apply_transformation(&tasks_file, transformation);
        assert!(result.is_ok());

        let transformation = TaskTransformation::AddContinueFlag { has_skip_permissions: false };
        let result = task_transformer.apply_transformation(&tasks_file, transformation);
        assert!(result.is_ok());
    }

    #[test]
    fn test_transformation_with_missing_file() {
        let temp_dir = TempDir::new().unwrap();
        let tasks_file = temp_dir.path().join("nonexistent.json");

        let task_transformer = TaskTransformer::new();
        let result = task_transformer.detect_task_configuration(&tasks_file);
        assert!(result.is_err());

        let transformation = TaskTransformation::RemovePromptFileAndAddContinue { has_skip_permissions: false };
        let result = task_transformer.apply_transformation(&tasks_file, transformation);
        assert!(result.is_err());

        let transformation = TaskTransformation::AddContinueFlag { has_skip_permissions: false };
        let result = task_transformer.apply_transformation(&tasks_file, transformation);
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

        let task_transformer = TaskTransformer::new();
        let transformation = TaskTransformation::AddContinueFlag { has_skip_permissions: false };
        let result = task_transformer.apply_transformation(&tasks_file, transformation);
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

        let transformation = TaskTransformation::AddContinueFlag { has_skip_permissions: false };
        let result = task_transformer.apply_transformation(&tasks_file, transformation);
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

        let result = super::apply_add_continue_flag_transformation(&tasks_file, false);
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

        let task_transformer = TaskTransformer::new();
        let transformation = TaskTransformation::AddContinueFlag { has_skip_permissions: true };
        let result = task_transformer.apply_transformation(&tasks_file, transformation);
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
        let json = super::load_tasks_json(&tasks_file).unwrap();
        assert!(json.get("version").is_some());
        assert!(json.get("tasks").is_some());

        // Save JSON back
        let result = super::save_tasks_json(&tasks_file, json);
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

        let result = super::load_tasks_json(&tasks_file);
        assert!(result.is_err());
    }

    #[test]
    fn test_load_tasks_json_with_invalid_json() {
        let temp_dir = TempDir::new().unwrap();
        let tasks_file = temp_dir.path().join("tasks.json");

        fs::write(&tasks_file, "{ invalid json }").unwrap();

        let result = super::load_tasks_json(&tasks_file);
        assert!(result.is_err());
    }
}
