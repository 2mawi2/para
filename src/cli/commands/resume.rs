use crate::cli::commands::common::create_claude_local_md;
use crate::cli::parser::ResumeArgs;
use crate::config::Config;
use crate::core::git::{GitOperations, GitService, SessionEnvironment};
use crate::core::ide::IdeManager;
use crate::core::session::state::SessionState;
use crate::core::session::{SessionManager, SessionStatus};
use crate::utils::{ParaError, Result};
use dialoguer::Select;
use serde_json::Value;
use std::env;
use std::fs;
use std::io::Write;
use std::path::Path;

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

pub fn execute(config: Config, args: ResumeArgs) -> Result<()> {
    args.validate()?;
    validate_resume_args(&args)?;

    let git_service = GitService::discover()?;
    let session_manager = SessionManager::new(&config);

    match &args.session {
        Some(session_name) => resume_specific_session(&config, &git_service, session_name, &args),
        None => detect_and_resume_session(&config, &git_service, &session_manager, &args),
    }
}

fn process_resume_context(args: &ResumeArgs) -> Result<Option<String>> {
    match (&args.prompt, &args.file) {
        (Some(prompt), None) => Ok(Some(prompt.clone())),
        (None, Some(file_path)) => {
            // Resolve path relative to current directory
            let resolved_path = if file_path.is_absolute() {
                file_path.clone()
            } else {
                env::current_dir()?.join(file_path)
            };

            // Validate file exists
            if !resolved_path.exists() {
                return Err(ParaError::fs_error(format!(
                    "File not found: {}",
                    resolved_path.display()
                )));
            }

            // Check file size (1MB limit)
            let metadata = fs::metadata(&resolved_path)?;
            if metadata.len() > 1_048_576 {
                return Err(ParaError::invalid_args(
                    "File too large. Maximum size is 1MB.",
                ));
            }

            // Read file contents
            let content = fs::read_to_string(&resolved_path)
                .map_err(|e| ParaError::fs_error(format!("Failed to read file: {}", e)))?;

            if content.trim().is_empty() {
                println!("⚠️  Warning: File is empty");
            }

            Ok(Some(content))
        }
        (None, None) => Ok(None),
        (Some(_), Some(_)) => unreachable!("Should be caught by validation"),
    }
}

fn save_resume_context(session_path: &Path, session_name: &str, context: &str) -> Result<()> {
    let para_dir = session_path.join(".para");
    let sessions_dir = para_dir.join("sessions");
    let session_dir = sessions_dir.join(session_name);

    // Create directories if they don't exist
    fs::create_dir_all(&session_dir)?;

    // Save context to file
    let context_file = session_dir.join("resume_context.md");
    let mut file = fs::File::create(&context_file)?;
    writeln!(file, "# Resume Context")?;
    writeln!(
        file,
        "This file contains additional context provided when resuming the session.\n"
    )?;
    writeln!(file, "{}", context)?;

    println!("📝 Resume context saved to: {}", context_file.display());
    Ok(())
}

fn validate_session_exists(
    session_manager: &SessionManager,
    session_name: &str,
) -> Result<Option<SessionState>> {
    if session_manager.session_exists(session_name) {
        let session_state = session_manager.load_state(session_name)?;
        Ok(Some(session_state))
    } else {
        Ok(None)
    }
}

fn repair_worktree_path(
    session_state: &mut SessionState,
    git_service: &GitService,
    session_manager: &SessionManager,
    session_name: &str,
) -> Result<()> {
    if !session_state.worktree_path.exists() {
        let branch_to_match = session_state.branch.clone();
        if let Some(wt) = git_service
            .list_worktrees()?
            .into_iter()
            .find(|w| w.branch == branch_to_match)
        {
            session_state.worktree_path = wt.path.clone();
            session_manager.save_state(session_state)?;
        } else if let Some(wt) = git_service.list_worktrees()?.into_iter().find(|w| {
            w.path
                .file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.starts_with(session_name))
                .unwrap_or(false)
        }) {
            session_state.worktree_path = wt.path.clone();
            session_manager.save_state(session_state)?;
        } else {
            return Err(ParaError::session_not_found(format!(
                "Session '{}' exists but worktree path '{}' not found",
                session_name,
                session_state.worktree_path.display()
            )));
        }
    }
    Ok(())
}

fn prepare_session_files(session_state: &SessionState) -> Result<()> {
    // Ensure CLAUDE.local.md exists for the session
    create_claude_local_md(&session_state.worktree_path, &session_state.name)?;
    Ok(())
}

fn handle_resume_context(session_state: &SessionState, args: &ResumeArgs) -> Result<()> {
    // Process and save resume context if provided
    if let Some(context) = process_resume_context(args)? {
        save_resume_context(&session_state.worktree_path, &session_state.name, &context)?;
    }
    Ok(())
}

fn resume_specific_session(
    config: &Config,
    git_service: &GitService,
    session_name: &str,
    args: &ResumeArgs,
) -> Result<()> {
    let session_manager = SessionManager::new(config);

    // Try to validate and load existing session
    if let Some(mut session_state) = validate_session_exists(&session_manager, session_name)? {
        // Repair worktree path if needed
        repair_worktree_path(
            &mut session_state,
            git_service,
            &session_manager,
            session_name,
        )?;

        // Prepare session files
        prepare_session_files(&session_state)?;

        // Handle resume context
        handle_resume_context(&session_state, args)?;

        // Launch IDE
        launch_ide_for_session(config, &session_state.worktree_path)?;
        println!("✅ Resumed session '{}'", session_name);
    } else {
        // Fallback: maybe the state file was timestamped (e.g. test4_20250611-XYZ)
        if let Some(candidate) = session_manager
            .list_sessions()?
            .into_iter()
            .find(|s| s.name.starts_with(session_name))
        {
            // recurse with the full name
            return resume_specific_session(config, git_service, &candidate.name, args);
        }
        // original branch/path heuristic
        let worktrees = git_service.list_worktrees()?;
        let matching_worktree = worktrees
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
            .ok_or_else(|| ParaError::session_not_found(session_name.to_string()))?;

        // Try to find session name from matching worktree
        if let Some(session_name) = session_manager
            .list_sessions()?
            .into_iter()
            .find(|s| {
                s.worktree_path == matching_worktree.path || s.branch == matching_worktree.branch
            })
            .map(|s| s.name)
        {
            create_claude_local_md(&matching_worktree.path, &session_name)?;
        } else {
            // Fallback: use session name from search
            create_claude_local_md(&matching_worktree.path, session_name)?;
        }

        // Process and save resume context if provided
        if let Some(context) = process_resume_context(args)? {
            // Try to determine session name for context saving
            let session_name_for_context = session_manager
                .list_sessions()?
                .into_iter()
                .find(|s| {
                    s.worktree_path == matching_worktree.path
                        || s.branch == matching_worktree.branch
                })
                .map(|s| s.name)
                .unwrap_or_else(|| session_name.to_string());

            save_resume_context(&matching_worktree.path, &session_name_for_context, &context)?;
        }

        launch_ide_for_session(config, &matching_worktree.path)?;
        println!(
            "✅ Resumed session at '{}'",
            matching_worktree.path.display()
        );
    }

    Ok(())
}

fn detect_and_resume_session(
    config: &Config,
    git_service: &GitService,
    session_manager: &SessionManager,
    args: &ResumeArgs,
) -> Result<()> {
    let current_dir = env::current_dir()?;

    match git_service.validate_session_environment(&current_dir)? {
        SessionEnvironment::Worktree { branch, .. } => {
            println!("Current directory is a worktree for branch: {}", branch);

            // Try to find session name from current directory or branch
            if let Some(session_name) = session_manager
                .list_sessions()?
                .into_iter()
                .find(|s| s.worktree_path == current_dir || s.branch == branch)
                .map(|s| s.name.clone())
            {
                create_claude_local_md(&current_dir, &session_name)?;

                // Process and save resume context if provided
                if let Some(context) = process_resume_context(args)? {
                    save_resume_context(&current_dir, &session_name, &context)?;
                }
            }

            launch_ide_for_session(config, &current_dir)?;
            println!("✅ Resumed current session");
            Ok(())
        }
        SessionEnvironment::MainRepository => {
            println!("Current directory is the main repository");
            list_and_select_session(config, git_service, session_manager, args)
        }
        SessionEnvironment::Invalid => {
            println!("Current directory is not part of a para session");
            list_and_select_session(config, git_service, session_manager, args)
        }
    }
}

fn list_and_select_session(
    config: &Config,
    _git_service: &GitService,
    session_manager: &SessionManager,
    args: &ResumeArgs,
) -> Result<()> {
    let sessions = session_manager.list_sessions()?;
    let active_sessions: Vec<_> = sessions
        .into_iter()
        .filter(|s| matches!(s.status, SessionStatus::Active))
        .collect();

    if active_sessions.is_empty() {
        println!("No active sessions found.");
        return Ok(());
    }

    println!("Active sessions:");
    for (i, session) in active_sessions.iter().enumerate() {
        println!("  {}: {} ({})", i + 1, session.name, session.branch);
    }

    let selection = Select::new()
        .with_prompt("Select session to resume")
        .items(&active_sessions.iter().map(|s| &s.name).collect::<Vec<_>>())
        .interact();

    if let Ok(index) = selection {
        let session = &active_sessions[index];

        if !session.worktree_path.exists() {
            return Err(ParaError::session_not_found(format!(
                "Session '{}' exists but worktree path '{}' not found",
                session.name,
                session.worktree_path.display()
            )));
        }

        // Ensure CLAUDE.local.md exists for the session
        create_claude_local_md(&session.worktree_path, &session.name)?;

        // Process and save resume context if provided
        if let Some(context) = process_resume_context(args)? {
            save_resume_context(&session.worktree_path, &session.name, &context)?;
        }

        launch_ide_for_session(config, &session.worktree_path)?;
        println!("✅ Resumed session '{}'", session.name);
    }

    Ok(())
}

fn launch_ide_for_session(config: &Config, path: &Path) -> Result<()> {
    let ide_manager = IdeManager::new(config);

    // For Claude Code in wrapper mode, always use continuation flag when resuming
    if config.ide.name == "claude" && config.ide.wrapper.enabled {
        println!("▶ resuming Claude Code session with conversation continuation...");
        // Update existing tasks.json to include -c flag
        update_tasks_json_for_resume(path)?;
        ide_manager.launch_with_options(path, false, true)
    } else {
        ide_manager.launch(path, false)
    }
}

fn update_tasks_json_for_resume(path: &Path) -> Result<()> {
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

fn validate_resume_args(args: &ResumeArgs) -> Result<()> {
    if let Some(ref session) = args.session {
        if session.is_empty() {
            return Err(ParaError::invalid_args(
                "Session identifier cannot be empty",
            ));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{
        Config, DirectoryConfig, GitConfig, IdeConfig, SessionConfig, WrapperConfig,
    };
    use std::fs;
    use std::path::PathBuf;
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
        let args = ResumeArgs {
            session: Some("test4".to_string()),
            prompt: None,
            file: None,
        };
        super::resume_specific_session(&config, &git_service, "test4", &args).unwrap();
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
    fn test_apply_transformation_no_change() {
        let temp_dir = TempDir::new().unwrap();
        let tasks_file = temp_dir.path().join("tasks.json");

        let content = r#"{"tasks":[{"command":"claude -c"}]}"#;
        fs::write(&tasks_file, content).unwrap();

        let transformation = TaskTransformation::NoChange;
        let result = super::apply_transformation(&tasks_file, transformation);
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

        let result = super::apply_remove_prompt_file_transformation(&tasks_file, true);
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

        let result = super::apply_remove_prompt_file_transformation(&tasks_file, false);
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

        let result = super::apply_add_continue_flag_transformation(&tasks_file, true);
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

        let result = super::apply_add_continue_flag_transformation(&tasks_file, false);
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

        let result = super::apply_add_continue_flag_transformation(&tasks_file, false);
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
        let result = super::detect_task_configuration(&tasks_file);
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            TaskConfiguration::NeedsTransformation {
                has_skip_permissions: false
            }
        );

        let result = super::apply_remove_prompt_file_transformation(&tasks_file, false);
        assert!(result.is_err());

        let result = super::apply_add_continue_flag_transformation(&tasks_file, false);
        assert!(result.is_err());
    }

    #[test]
    fn test_transformation_with_missing_tasks() {
        let temp_dir = TempDir::new().unwrap();
        let tasks_file = temp_dir.path().join("tasks.json");

        // Test with JSON that has no tasks array
        let content = r#"{ "version": "2.0.0" }"#;
        fs::write(&tasks_file, content).unwrap();

        let config = super::detect_task_configuration(&tasks_file).unwrap();
        assert_eq!(
            config,
            TaskConfiguration::NeedsTransformation {
                has_skip_permissions: false
            }
        );

        // Transformations should handle missing tasks gracefully
        let result = super::apply_remove_prompt_file_transformation(&tasks_file, false);
        assert!(result.is_ok());

        let result = super::apply_add_continue_flag_transformation(&tasks_file, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_transformation_with_missing_file() {
        let temp_dir = TempDir::new().unwrap();
        let tasks_file = temp_dir.path().join("nonexistent.json");

        let result = super::detect_task_configuration(&tasks_file);
        assert!(result.is_err());

        let result = super::apply_remove_prompt_file_transformation(&tasks_file, false);
        assert!(result.is_err());

        let result = super::apply_add_continue_flag_transformation(&tasks_file, false);
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

        let result = super::apply_add_continue_flag_transformation(&tasks_file, false);
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

        let result = super::apply_add_continue_flag_transformation(&tasks_file, false);
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

        let result = super::apply_add_continue_flag_transformation(&tasks_file, true);
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

    #[test]
    fn test_process_resume_context_with_prompt() {
        let args = ResumeArgs {
            session: None,
            prompt: Some("Continue working on the authentication system".to_string()),
            file: None,
        };

        let result = super::process_resume_context(&args).unwrap();
        assert_eq!(
            result,
            Some("Continue working on the authentication system".to_string())
        );
    }

    #[test]
    fn test_process_resume_context_with_file() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("context.md");
        fs::write(&test_file, "# New Requirements\n\nAdd OAuth support").unwrap();

        let args = ResumeArgs {
            session: None,
            prompt: None,
            file: Some(test_file.clone()),
        };

        let result = super::process_resume_context(&args).unwrap();
        assert_eq!(
            result,
            Some("# New Requirements\n\nAdd OAuth support".to_string())
        );
    }

    #[test]
    fn test_process_resume_context_no_input() {
        let args = ResumeArgs {
            session: None,
            prompt: None,
            file: None,
        };

        let result = super::process_resume_context(&args).unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_process_resume_context_file_not_found() {
        let args = ResumeArgs {
            session: None,
            prompt: None,
            file: Some(PathBuf::from("/nonexistent/file.txt")),
        };

        let result = super::process_resume_context(&args);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("File not found"));
    }

    #[test]
    fn test_process_resume_context_file_too_large() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("large.txt");

        // Create a file larger than 1MB
        let large_content = "x".repeat(1_048_577);
        fs::write(&test_file, large_content).unwrap();

        let args = ResumeArgs {
            session: None,
            prompt: None,
            file: Some(test_file),
        };

        let result = super::process_resume_context(&args);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("File too large"));
    }

    #[test]
    fn test_save_resume_context() {
        let temp_dir = TempDir::new().unwrap();
        let session_path = temp_dir.path();
        let session_name = "test-session";
        let context = "This is test context\nWith multiple lines";

        super::save_resume_context(session_path, session_name, context).unwrap();

        let expected_file = session_path.join(".para/sessions/test-session/resume_context.md");
        assert!(expected_file.exists());

        let saved_content = fs::read_to_string(&expected_file).unwrap();
        assert!(saved_content.contains("# Resume Context"));
        assert!(saved_content.contains(context));
    }

    #[test]
    fn test_resume_args_validate() {
        // Test valid cases
        let args = ResumeArgs {
            session: None,
            prompt: Some("test".to_string()),
            file: None,
        };
        assert!(args.validate().is_ok());

        let args = ResumeArgs {
            session: None,
            prompt: None,
            file: Some(PathBuf::from("test.md")),
        };
        assert!(args.validate().is_ok());

        // Test invalid case - both prompt and file
        let args = ResumeArgs {
            session: None,
            prompt: Some("test".to_string()),
            file: Some(PathBuf::from("test.md")),
        };
        assert!(args.validate().is_err());
        assert!(args
            .validate()
            .unwrap_err()
            .to_string()
            .contains("Cannot specify both"));
    }

    // Integration tests for new resume functionality
    #[test]
    fn test_resume_with_prompt_integration() {
        let (_git_tmp, _state_tmp, git_service, config) = setup_test_repo();
        let session_manager = SessionManager::new(&config);

        // Create a test session
        let session_name = "test-prompt-session".to_string();
        let branch_name = "para/test-prompt-branch".to_string();
        let worktree_path = git_service
            .repository()
            .root
            .join(&config.directories.subtrees_dir)
            .join(&config.git.branch_prefix)
            .join(&session_name);

        git_service
            .create_worktree(&branch_name, &worktree_path)
            .unwrap();

        let state = crate::core::session::state::SessionState::new(
            session_name.clone(),
            branch_name,
            worktree_path.clone(),
        );
        session_manager.save_state(&state).unwrap();

        // Resume with prompt
        let args = ResumeArgs {
            session: Some(session_name.clone()),
            prompt: Some("Continue implementing the feature".to_string()),
            file: None,
        };

        // Execute resume (with echo IDE it won't actually launch anything)
        super::resume_specific_session(&config, &git_service, &session_name, &args).unwrap();

        // Verify context was saved
        let context_file = worktree_path
            .join(".para/sessions")
            .join(&session_name)
            .join("resume_context.md");
        assert!(context_file.exists());
        let saved_content = fs::read_to_string(&context_file).unwrap();
        assert!(saved_content.contains("Continue implementing the feature"));
    }

    #[test]
    fn test_resume_with_file_integration() {
        let (_git_tmp, _state_tmp, git_service, config) = setup_test_repo();
        let session_manager = SessionManager::new(&config);

        // Create a test session
        let session_name = "test-file-session".to_string();
        let branch_name = "para/test-file-branch".to_string();
        let worktree_path = git_service
            .repository()
            .root
            .join(&config.directories.subtrees_dir)
            .join(&config.git.branch_prefix)
            .join(&session_name);

        git_service
            .create_worktree(&branch_name, &worktree_path)
            .unwrap();

        let state = crate::core::session::state::SessionState::new(
            session_name.clone(),
            branch_name,
            worktree_path.clone(),
        );
        session_manager.save_state(&state).unwrap();

        // Create a test file with context
        let temp_dir = TempDir::new().unwrap();
        let context_file = temp_dir.path().join("new_requirements.md");
        fs::write(
            &context_file,
            "# Updated Requirements\n\nImplement OAuth2 authentication",
        )
        .unwrap();

        // Resume with file
        let args = ResumeArgs {
            session: Some(session_name.clone()),
            prompt: None,
            file: Some(context_file),
        };

        // Execute resume
        super::resume_specific_session(&config, &git_service, &session_name, &args).unwrap();

        // Verify context was saved
        let saved_context_file = worktree_path
            .join(".para/sessions")
            .join(&session_name)
            .join("resume_context.md");
        assert!(saved_context_file.exists());
        let saved_content = fs::read_to_string(&saved_context_file).unwrap();
        assert!(saved_content.contains("Updated Requirements"));
        assert!(saved_content.contains("Implement OAuth2 authentication"));
    }

    #[test]
    fn test_resume_backwards_compatibility() {
        let (_git_tmp, _state_tmp, git_service, config) = setup_test_repo();
        let session_manager = SessionManager::new(&config);

        // Create a test session
        let session_name = "test-compat-session".to_string();
        let branch_name = "para/test-compat-branch".to_string();
        let worktree_path = git_service
            .repository()
            .root
            .join(&config.directories.subtrees_dir)
            .join(&config.git.branch_prefix)
            .join(&session_name);

        git_service
            .create_worktree(&branch_name, &worktree_path)
            .unwrap();

        let state = crate::core::session::state::SessionState::new(
            session_name.clone(),
            branch_name,
            worktree_path.clone(),
        );
        session_manager.save_state(&state).unwrap();

        // Resume without any additional context (old behavior)
        let args = ResumeArgs {
            session: Some(session_name.clone()),
            prompt: None,
            file: None,
        };

        // Execute resume - should work exactly as before
        let result = super::resume_specific_session(&config, &git_service, &session_name, &args);
        assert!(result.is_ok());

        // Verify no context file was created
        let context_file = worktree_path
            .join(".para/sessions")
            .join(&session_name)
            .join("resume_context.md");
        assert!(!context_file.exists());
    }

    #[test]
    fn test_resume_empty_file_warning() {
        let temp_dir = TempDir::new().unwrap();
        let empty_file = temp_dir.path().join("empty.txt");
        fs::write(&empty_file, "").unwrap();

        let args = ResumeArgs {
            session: None,
            prompt: None,
            file: Some(empty_file),
        };

        // Process should succeed but with empty content
        let result = super::process_resume_context(&args).unwrap();
        assert_eq!(result, Some("".to_string()));
    }
}
