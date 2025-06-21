use crate::cli::commands::common::create_claude_local_md;
use crate::cli::parser::ResumeArgs;
use crate::config::Config;
use crate::core::git::{GitOperations, GitService, SessionEnvironment};
use crate::core::ide::IdeManager;
use crate::core::session::{SessionManager, SessionStatus};
use crate::utils::{ParaError, Result};
use dialoguer::Select;
use serde_json::Value;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

mod task_detector;
mod task_transformer;
mod command_transformer;
mod session_validator;

use task_detector::TaskConfigurationDetector;
use task_transformer::TaskTransformer;
use command_transformer::CommandTransformer;
use session_validator::SessionValidator;

#[derive(Debug, Clone)]
struct ResumeContext<'a> {
    config: &'a Config,
    git_service: &'a GitService,
    session_manager: &'a SessionManager,
}

#[derive(Debug)]
struct TaskUpdateRequest {
    tasks_file: PathBuf,
    has_skip_permissions: bool,
}

#[derive(Debug, PartialEq)]
enum TaskConfiguration {
    HasPromptFile { has_skip_permissions: bool },
    HasContinueFlag { has_skip_permissions: bool },
    NeedsTransformation { has_skip_permissions: bool },
}

#[derive(Debug)]
enum TaskTransformation {
    RemovePromptFileAndAddContinue { has_skip_permissions: bool },
    AddContinueFlag { has_skip_permissions: bool },
    NoChange,
}

mod task_detector {
    use super::*;

    pub struct TaskConfigurationDetector;

    impl TaskConfigurationDetector {
        pub fn detect(tasks_file: &Path) -> Result<TaskConfiguration> {
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
    }
}

mod task_transformer {
    use super::*;

    pub struct TaskTransformer;

    impl TaskTransformer {
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

        pub fn apply_transformation(request: &TaskUpdateRequest, transformation: TaskTransformation) -> Result<()> {
            match transformation {
                TaskTransformation::NoChange => Ok(()),
                TaskTransformation::RemovePromptFileAndAddContinue {
                    has_skip_permissions,
                } => Self::apply_remove_prompt_file_transformation(&request.tasks_file, has_skip_permissions),
                TaskTransformation::AddContinueFlag {
                    has_skip_permissions,
                } => Self::apply_add_continue_flag_transformation(&request.tasks_file, has_skip_permissions),
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
            tasks_file: &Path,
            has_skip_permissions: bool,
        ) -> Result<()> {
            let mut json = Self::load_tasks_json(tasks_file)?;

            if let Some(tasks) = json.get_mut("tasks").and_then(|t| t.as_array_mut()) {
                for task in tasks {
                    if let Some(command_value) = task.get_mut("command") {
                        if let Some(command_str) = command_value.as_str() {
                            let updated_command =
                                CommandTransformer::transform_claude_command(command_str, has_skip_permissions);

                            if updated_command != command_str {
                                *command_value = Value::String(updated_command);
                            }
                        }
                    }
                }
            }

            Self::save_tasks_json(tasks_file, json)
        }

        pub fn load_tasks_json(tasks_file: &Path) -> Result<Value> {
            let content = fs::read_to_string(tasks_file)
                .map_err(|e| ParaError::fs_error(format!("Failed to read tasks.json: {}", e)))?;

            serde_json::from_str(&content)
                .map_err(|e| ParaError::fs_error(format!("Failed to parse tasks.json: {}", e)))
        }

        pub fn save_tasks_json(tasks_file: &Path, json: Value) -> Result<()> {
            let updated_content = serde_json::to_string_pretty(&json)
                .map_err(|e| ParaError::fs_error(format!("Failed to serialize tasks.json: {}", e)))?;

            fs::write(tasks_file, updated_content)
                .map_err(|e| ParaError::fs_error(format!("Failed to update tasks.json: {}", e)))
        }
    }
}

mod command_transformer {
    use super::*;

    pub struct CommandTransformer;

    impl CommandTransformer {
        pub fn transform_claude_command(command: &str, has_skip_permissions: bool) -> String {
            if has_skip_permissions {
                Self::transform_claude_command_with_skip_permissions(command)
            } else {
                Self::transform_claude_command_regular(command)
            }
        }

        pub fn transform_claude_command_with_skip_permissions(command: &str) -> String {
            if command.contains("claude --dangerously-skip-permissions") && Self::needs_continue_flag(command) {
                command.replace(
                    "claude --dangerously-skip-permissions",
                    "claude --dangerously-skip-permissions -c",
                )
            } else {
                command.to_string()
            }
        }

        pub fn transform_claude_command_regular(command: &str) -> String {
            if command == "claude" {
                "claude -c".to_string()
            } else if command.starts_with("claude ") && Self::needs_continue_flag(command) {
                command.replace("claude ", "claude -c ")
            } else {
                command.to_string()
            }
        }

        pub fn needs_continue_flag(command: &str) -> bool {
            !command.contains("-c")
        }
    }
}

mod session_validator {
    use super::*;

    pub struct SessionValidator;

    impl SessionValidator {
        pub fn validate_session_path(
            session_state: &mut crate::core::session::state::SessionState,
            git_service: &GitService,
            session_manager: &SessionManager,
        ) -> Result<()> {
            if session_state.worktree_path.exists() {
                return Ok(());
            }

            let branch_to_match = session_state.branch.clone();
            
            if let Some(wt) = git_service
                .list_worktrees()?
                .into_iter()
                .find(|w| w.branch == branch_to_match)
            {
                session_state.worktree_path = wt.path.clone();
                session_manager.save_state(session_state)?;
                return Ok(());
            }

            if let Some(wt) = git_service.list_worktrees()?.into_iter().find(|w| {
                w.path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with(&session_state.name))
                    .unwrap_or(false)
            }) {
                session_state.worktree_path = wt.path.clone();
                session_manager.save_state(session_state)?;
                return Ok(());
            }

            Err(ParaError::session_not_found(format!(
                "Session '{}' exists but worktree path '{}' not found",
                session_state.name,
                session_state.worktree_path.display()
            )))
        }

        pub fn find_session_by_prefix(
            session_manager: &SessionManager,
            session_name: &str,
        ) -> Result<Option<String>> {
            let sessions = session_manager.list_sessions()?;
            Ok(sessions
                .into_iter()
                .find(|s| s.name.starts_with(session_name))
                .map(|s| s.name))
        }

        pub fn find_matching_worktree(
            git_service: &GitService,
            session_name: &str,
        ) -> Result<crate::core::git::Worktree> {
            let worktrees = git_service.list_worktrees()?;
            worktrees
                .iter()
                .find(|wt| {
                    wt.branch.contains(session_name)
                        || wt
                            .path
                            .file_name()
                            .and_then(|name| name.to_str())
                            .map(|name| name.contains(session_name))
                            .unwrap_or(false)
                })
                .cloned()
                .ok_or_else(|| ParaError::session_not_found(session_name.to_string()))
        }
    }

    pub fn validate_resume_args(args: &ResumeArgs) -> Result<()> {
        if let Some(ref session) = args.session {
            if session.is_empty() {
                return Err(ParaError::invalid_args(
                    "Session identifier cannot be empty",
                ));
            }
        }
        Ok(())
    }
}

pub fn execute(config: Config, args: ResumeArgs) -> Result<()> {
    session_validator::validate_resume_args(&args)?;

    let git_service = GitService::discover()?;
    let session_manager = SessionManager::new(&config);
    let context = ResumeContext {
        config: &config,
        git_service: &git_service,
        session_manager: &session_manager,
    };

    match args.session {
        Some(session_name) => resume_specific_session(&context, &session_name),
        None => detect_and_resume_session(&context),
    }
}

fn resume_specific_session(context: &ResumeContext, session_name: &str) -> Result<()> {
    if context.session_manager.session_exists(session_name) {
        return resume_existing_session(context, session_name);
    }

    // Try to find by prefix (e.g. "test4" -> "test4_20250611-XYZ")
    if let Some(full_name) = SessionValidator::find_session_by_prefix(context.session_manager, session_name)? {
        return resume_specific_session(context, &full_name);
    }

    // Fallback to worktree heuristic
    resume_session_by_worktree_heuristic(context, session_name)
}

fn resume_existing_session(context: &ResumeContext, session_name: &str) -> Result<()> {
    let mut session_state = context.session_manager.load_state(session_name)?;
    
    SessionValidator::validate_session_path(&mut session_state, context.git_service, context.session_manager)?;
    
    create_claude_local_md(&session_state.worktree_path, &session_state.name)?;
    launch_ide_for_session(context.config, &session_state.worktree_path)?;
    println!("✅ Resumed session '{}'", session_name);
    Ok(())
}

fn resume_session_by_worktree_heuristic(context: &ResumeContext, session_name: &str) -> Result<()> {
    let matching_worktree = SessionValidator::find_matching_worktree(context.git_service, session_name)?;
    
    let actual_session_name = find_session_name_for_worktree(context, &matching_worktree)
        .unwrap_or_else(|| session_name.to_string());
        
    create_claude_local_md(&matching_worktree.path, &actual_session_name)?;
    launch_ide_for_session(context.config, &matching_worktree.path)?;
    println!("✅ Resumed session at '{}'", matching_worktree.path.display());
    Ok(())
}

fn find_session_name_for_worktree(
    context: &ResumeContext, 
    worktree: &crate::core::git::Worktree
) -> Option<String> {
    context.session_manager
        .list_sessions()
        .ok()?
        .into_iter()
        .find(|s| s.worktree_path == worktree.path || s.branch == worktree.branch)
        .map(|s| s.name)
}

fn detect_and_resume_session(context: &ResumeContext) -> Result<()> {
    let current_dir = env::current_dir()?;

    match context.git_service.validate_session_environment(&current_dir)? {
        SessionEnvironment::Worktree { branch, .. } => {
            resume_worktree_session(context, &current_dir, &branch)
        }
        SessionEnvironment::MainRepository => {
            println!("Current directory is the main repository");
            list_and_select_session(context)
        }
        SessionEnvironment::Invalid => {
            println!("Current directory is not part of a para session");
            list_and_select_session(context)
        }
    }
}

fn resume_worktree_session(context: &ResumeContext, current_dir: &Path, branch: &str) -> Result<()> {
    println!("Current directory is a worktree for branch: {}", branch);

    let session_name = find_session_name_for_current_dir(context, current_dir, branch);
    if let Some(name) = session_name {
        create_claude_local_md(current_dir, &name)?;
    }

    launch_ide_for_session(context.config, current_dir)?;
    println!("✅ Resumed current session");
    Ok(())
}

fn find_session_name_for_current_dir(
    context: &ResumeContext,
    current_dir: &Path,
    branch: &str,
) -> Option<String> {
    context.session_manager
        .list_sessions()
        .ok()?
        .into_iter()
        .find(|s| s.worktree_path == current_dir || s.branch == branch)
        .map(|s| s.name)
}

fn list_and_select_session(context: &ResumeContext) -> Result<()> {
    let active_sessions = get_active_sessions(context.session_manager)?;

    if active_sessions.is_empty() {
        println!("No active sessions found.");
        return Ok(());
    }

    display_active_sessions(&active_sessions);
    
    let selected_session = prompt_session_selection(&active_sessions)?;
    if let Some(session) = selected_session {
        resume_selected_session(context, session)?;
    }

    Ok(())
}

fn get_active_sessions(session_manager: &SessionManager) -> Result<Vec<crate::core::session::state::SessionState>> {
    let sessions = session_manager.list_sessions()?;
    Ok(sessions
        .into_iter()
        .filter(|s| matches!(s.status, SessionStatus::Active))
        .collect())
}

fn display_active_sessions(sessions: &[crate::core::session::state::SessionState]) {
    println!("Active sessions:");
    for (i, session) in sessions.iter().enumerate() {
        println!("  {}: {} ({})", i + 1, session.name, session.branch);
    }
}

fn prompt_session_selection(sessions: &[crate::core::session::state::SessionState]) -> Result<Option<&crate::core::session::state::SessionState>> {
    let selection = Select::new()
        .with_prompt("Select session to resume")
        .items(&sessions.iter().map(|s| &s.name).collect::<Vec<_>>())
        .interact();

    match selection {
        Ok(index) => Ok(Some(&sessions[index])),
        Err(_) => Ok(None),
    }
}

fn resume_selected_session(
    context: &ResumeContext,
    session: &crate::core::session::state::SessionState,
) -> Result<()> {
    if !session.worktree_path.exists() {
        return Err(ParaError::session_not_found(format!(
            "Session '{}' exists but worktree path '{}' not found",
            session.name,
            session.worktree_path.display()
        )));
    }

    create_claude_local_md(&session.worktree_path, &session.name)?;
    launch_ide_for_session(context.config, &session.worktree_path)?;
    println!("✅ Resumed session '{}'", session.name);
    Ok(())
}

fn launch_ide_for_session(config: &Config, path: &Path) -> Result<()> {
    let ide_manager = IdeManager::new(config);

    if should_use_continuation_mode(config) {
        println!("▶ resuming Claude Code session with conversation continuation...");
        update_tasks_json_for_resume(path)?;
        ide_manager.launch_with_options(path, false, true)
    } else {
        ide_manager.launch(path, false)
    }
}

fn should_use_continuation_mode(config: &Config) -> bool {
    config.ide.name == "claude" && config.ide.wrapper.enabled
}

fn update_tasks_json_for_resume(path: &Path) -> Result<()> {
    let tasks_file = path.join(".vscode/tasks.json");

    if !tasks_file.exists() {
        return Ok(());
    }

    let config = TaskConfigurationDetector::detect(&tasks_file)?;
    let transformation = TaskTransformer::determine_transformation(&config);
    let request = TaskUpdateRequest {
        tasks_file: tasks_file.clone(),
        has_skip_permissions: match &config {
            TaskConfiguration::HasPromptFile { has_skip_permissions } => *has_skip_permissions,
            TaskConfiguration::HasContinueFlag { has_skip_permissions } => *has_skip_permissions,
            TaskConfiguration::NeedsTransformation { has_skip_permissions } => *has_skip_permissions,
        },
    };
    
    TaskTransformer::apply_transformation(&request, transformation)
}


#[cfg(test)]
mod tests {
    use super::*;
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

        // now resume with base name
        let session_manager = SessionManager::new(&config);
        let context = ResumeContext {
            config: &config,
            git_service: &git_service,
            session_manager: &session_manager,
        };
        super::resume_specific_session(&context, "test4").unwrap();
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

        // Update the tasks.json
        super::update_tasks_json_for_resume(temp_dir.path()).unwrap();

        // Check it was updated
        let updated_content = fs::read_to_string(&tasks_file).unwrap();
        assert!(updated_content.contains("claude --dangerously-skip-permissions -c"));
        assert!(!updated_content.contains("claude --dangerously-skip-permissions \""));

        // Test idempotency - running again shouldn't change it
        super::update_tasks_json_for_resume(temp_dir.path()).unwrap();
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

        // Update the tasks.json
        super::update_tasks_json_for_resume(temp_dir.path()).unwrap();

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

        let config = TaskConfigurationDetector::detect(&tasks_file).unwrap();
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

        let config = TaskConfigurationDetector::detect(&tasks_file).unwrap();
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

        let config = TaskConfigurationDetector::detect(&tasks_file).unwrap();
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

        let config = TaskConfigurationDetector::detect(&tasks_file).unwrap();
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

        let config = TaskConfigurationDetector::detect(&tasks_file).unwrap();
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

        let config = TaskConfigurationDetector::detect(&tasks_file).unwrap();
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
        let transformation = TaskTransformer::determine_transformation(&config);
        assert!(matches!(
            transformation,
            TaskTransformation::RemovePromptFileAndAddContinue {
                has_skip_permissions: true
            }
        ));

        let config = TaskConfiguration::HasPromptFile {
            has_skip_permissions: false,
        };
        let transformation = TaskTransformer::determine_transformation(&config);
        assert!(matches!(
            transformation,
            TaskTransformation::RemovePromptFileAndAddContinue {
                has_skip_permissions: false
            }
        ));

        // Test HasContinueFlag -> NoChange
        let config = TaskConfiguration::HasContinueFlag {
            has_skip_permissions: true,
        };
        let transformation = TaskTransformer::determine_transformation(&config);
        assert!(matches!(transformation, TaskTransformation::NoChange));

        // Test NeedsTransformation -> AddContinueFlag
        let config = TaskConfiguration::NeedsTransformation {
            has_skip_permissions: false,
        };
        let transformation = TaskTransformer::determine_transformation(&config);
        assert!(matches!(
            transformation,
            TaskTransformation::AddContinueFlag {
                has_skip_permissions: false
            }
        ));
    }

    #[test]
    fn test_apply_transformation_no_change() {
        let temp_dir = TempDir::new().unwrap();
        let tasks_file = temp_dir.path().join("tasks.json");

        let content = r#"{"tasks":[{"command":"claude -c"}]}"#;
        fs::write(&tasks_file, content).unwrap();

        let transformation = TaskTransformation::NoChange;
        let request = TaskUpdateRequest {
            tasks_file: tasks_file.clone(),
            has_skip_permissions: match &transformation {
                TaskTransformation::RemovePromptFileAndAddContinue { has_skip_permissions } => *has_skip_permissions,
                TaskTransformation::AddContinueFlag { has_skip_permissions } => *has_skip_permissions,
                TaskTransformation::NoChange => false,
            },
        };
        let result = TaskTransformer::apply_transformation(&request, transformation);
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

        let request = TaskUpdateRequest {
            tasks_file: tasks_file.clone(),
            has_skip_permissions: true,
        };
        let transformation = TaskTransformation::RemovePromptFileAndAddContinue { has_skip_permissions: true };
        let result = TaskTransformer::apply_transformation(&request, transformation);
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

        let request = TaskUpdateRequest {
            tasks_file: tasks_file.clone(),
            has_skip_permissions: false,
        };
        let transformation = TaskTransformation::RemovePromptFileAndAddContinue { has_skip_permissions: false };
        let result = TaskTransformer::apply_transformation(&request, transformation);
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

        let request = TaskUpdateRequest {
            tasks_file: tasks_file.clone(),
            has_skip_permissions: true,
        };
        let transformation = TaskTransformation::AddContinueFlag { has_skip_permissions: true };
        let result = TaskTransformer::apply_transformation(&request, transformation);
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

        let request = TaskUpdateRequest {
            tasks_file: tasks_file.clone(),
            has_skip_permissions: false,
        };
        let transformation = TaskTransformation::AddContinueFlag { has_skip_permissions: false };
        let result = TaskTransformer::apply_transformation(&request, transformation);
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

        let request = TaskUpdateRequest {
            tasks_file: tasks_file.clone(),
            has_skip_permissions: false,
        };
        let transformation = TaskTransformation::AddContinueFlag { has_skip_permissions: false };
        let result = TaskTransformer::apply_transformation(&request, transformation);
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
        let result = TaskConfigurationDetector::detect(&tasks_file);
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            TaskConfiguration::NeedsTransformation {
                has_skip_permissions: false
            }
        );

        let request = TaskUpdateRequest {
            tasks_file: tasks_file.clone(),
            has_skip_permissions: false,
        };
        let transformation = TaskTransformation::RemovePromptFileAndAddContinue { has_skip_permissions: false };
        let result = TaskTransformer::apply_transformation(&request, transformation);
        assert!(result.is_err());

        let request = TaskUpdateRequest {
            tasks_file: tasks_file.clone(),
            has_skip_permissions: false,
        };
        let transformation = TaskTransformation::AddContinueFlag { has_skip_permissions: false };
        let result = TaskTransformer::apply_transformation(&request, transformation);
        assert!(result.is_err());
    }

    #[test]
    fn test_transformation_with_missing_tasks() {
        let temp_dir = TempDir::new().unwrap();
        let tasks_file = temp_dir.path().join("tasks.json");

        // Test with JSON that has no tasks array
        let content = r#"{ "version": "2.0.0" }"#;
        fs::write(&tasks_file, content).unwrap();

        let config = TaskConfigurationDetector::detect(&tasks_file).unwrap();
        assert_eq!(
            config,
            TaskConfiguration::NeedsTransformation {
                has_skip_permissions: false
            }
        );

        // Transformations should handle missing tasks gracefully
        let request = TaskUpdateRequest {
            tasks_file: tasks_file.clone(),
            has_skip_permissions: false,
        };
        let transformation = TaskTransformation::RemovePromptFileAndAddContinue { has_skip_permissions: false };
        let result = TaskTransformer::apply_transformation(&request, transformation);
        assert!(result.is_ok());

        let request = TaskUpdateRequest {
            tasks_file: tasks_file.clone(),
            has_skip_permissions: false,
        };
        let transformation = TaskTransformation::AddContinueFlag { has_skip_permissions: false };
        let result = TaskTransformer::apply_transformation(&request, transformation);
        assert!(result.is_ok());
    }

    #[test]
    fn test_transformation_with_missing_file() {
        let temp_dir = TempDir::new().unwrap();
        let tasks_file = temp_dir.path().join("nonexistent.json");

        let result = TaskConfigurationDetector::detect(&tasks_file);
        assert!(result.is_err());

        let request = TaskUpdateRequest {
            tasks_file: tasks_file.clone(),
            has_skip_permissions: false,
        };
        let transformation = TaskTransformation::RemovePromptFileAndAddContinue { has_skip_permissions: false };
        let result = TaskTransformer::apply_transformation(&request, transformation);
        assert!(result.is_err());

        let request = TaskUpdateRequest {
            tasks_file: tasks_file.clone(),
            has_skip_permissions: false,
        };
        let transformation = TaskTransformation::AddContinueFlag { has_skip_permissions: false };
        let result = TaskTransformer::apply_transformation(&request, transformation);
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

        let request = TaskUpdateRequest {
            tasks_file: tasks_file.clone(),
            has_skip_permissions: false,
        };
        let transformation = TaskTransformation::AddContinueFlag { has_skip_permissions: false };
        let result = TaskTransformer::apply_transformation(&request, transformation);
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

        let request = TaskUpdateRequest {
            tasks_file: tasks_file.clone(),
            has_skip_permissions: false,
        };
        let transformation = TaskTransformation::AddContinueFlag { has_skip_permissions: false };
        let result = TaskTransformer::apply_transformation(&request, transformation);
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

        let request = TaskUpdateRequest {
            tasks_file: tasks_file.clone(),
            has_skip_permissions: false,
        };
        let transformation = TaskTransformation::AddContinueFlag { has_skip_permissions: false };
        let result = TaskTransformer::apply_transformation(&request, transformation);
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

        let request = TaskUpdateRequest {
            tasks_file: tasks_file.clone(),
            has_skip_permissions: true,
        };
        let transformation = TaskTransformation::AddContinueFlag { has_skip_permissions: true };
        let result = TaskTransformer::apply_transformation(&request, transformation);
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
        assert!(CommandTransformer::needs_continue_flag("claude"));
        assert!(CommandTransformer::needs_continue_flag("claude --verbose"));
        assert!(CommandTransformer::needs_continue_flag(
            "claude --dangerously-skip-permissions"
        ));

        assert!(!CommandTransformer::needs_continue_flag("claude -c"));
        assert!(!CommandTransformer::needs_continue_flag(
            "claude --dangerously-skip-permissions -c"
        ));
        assert!(!CommandTransformer::needs_continue_flag("claude -c --verbose"));
    }

    #[test]
    fn test_transform_claude_command_regular() {
        // Test exact match
        assert_eq!(
            CommandTransformer::transform_claude_command_regular("claude"),
            "claude -c"
        );

        // Test with additional flags
        assert_eq!(
            CommandTransformer::transform_claude_command_regular("claude --verbose"),
            "claude -c --verbose"
        );
        assert_eq!(
            CommandTransformer::transform_claude_command_regular("claude --help"),
            "claude -c --help"
        );

        // Test already has -c flag (no change)
        assert_eq!(
            CommandTransformer::transform_claude_command_regular("claude -c"),
            "claude -c"
        );
        assert_eq!(
            CommandTransformer::transform_claude_command_regular("claude -c --verbose"),
            "claude -c --verbose"
        );

        // Test non-Claude commands (no change)
        assert_eq!(
            CommandTransformer::transform_claude_command_regular("echo hello"),
            "echo hello"
        );
        assert_eq!(CommandTransformer::transform_claude_command_regular(""), "");

        // Test edge cases
        assert_eq!(
            CommandTransformer::transform_claude_command_regular("claudetest"),
            "claudetest"
        );
    }

    #[test]
    fn test_transform_claude_command_with_skip_permissions() {
        // Test with exact match
        assert_eq!(
            CommandTransformer::transform_claude_command_with_skip_permissions(
                "claude --dangerously-skip-permissions"
            ),
            "claude --dangerously-skip-permissions -c"
        );

        // Test with additional flags
        assert_eq!(
            CommandTransformer::transform_claude_command_with_skip_permissions(
                "claude --dangerously-skip-permissions --verbose"
            ),
            "claude --dangerously-skip-permissions -c --verbose"
        );

        // Test already has -c flag (no change)
        assert_eq!(
            CommandTransformer::transform_claude_command_with_skip_permissions(
                "claude --dangerously-skip-permissions -c"
            ),
            "claude --dangerously-skip-permissions -c"
        );

        // Test partial match that doesn't get transformed (current behavior)
        assert_eq!(
            CommandTransformer::transform_claude_command_with_skip_permissions(
                "claude --other-flag --dangerously-skip-permissions"
            ),
            "claude --other-flag --dangerously-skip-permissions"
        );

        // Test non-matching commands (no change)
        assert_eq!(
            CommandTransformer::transform_claude_command_with_skip_permissions("claude"),
            "claude"
        );
        assert_eq!(
            CommandTransformer::transform_claude_command_with_skip_permissions("echo hello"),
            "echo hello"
        );
    }

    #[test]
    fn test_transform_claude_command() {
        // Test with skip permissions = true
        assert_eq!(
            CommandTransformer::transform_claude_command("claude --dangerously-skip-permissions", true),
            "claude --dangerously-skip-permissions -c"
        );

        // Test with skip permissions = false
        assert_eq!(
            CommandTransformer::transform_claude_command("claude", false),
            "claude -c"
        );
        assert_eq!(
            CommandTransformer::transform_claude_command("claude --verbose", false),
            "claude -c --verbose"
        );

        // Test non-matching commands
        assert_eq!(
            CommandTransformer::transform_claude_command("echo hello", true),
            "echo hello"
        );
        assert_eq!(
            CommandTransformer::transform_claude_command("echo hello", false),
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
        let json = TaskTransformer::load_tasks_json(&tasks_file).unwrap();
        assert!(json.get("version").is_some());
        assert!(json.get("tasks").is_some());

        // Save JSON back
        let result = TaskTransformer::save_tasks_json(&tasks_file, json);
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

        let result = TaskTransformer::load_tasks_json(&tasks_file);
        assert!(result.is_err());
    }

    #[test]
    fn test_load_tasks_json_with_invalid_json() {
        let temp_dir = TempDir::new().unwrap();
        let tasks_file = temp_dir.path().join("tasks.json");

        fs::write(&tasks_file, "{ invalid json }").unwrap();

        let result = TaskTransformer::load_tasks_json(&tasks_file);
        assert!(result.is_err());
    }
}
