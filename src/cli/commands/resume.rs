use crate::cli::commands::common::create_claude_local_md;
use crate::cli::parser::ResumeArgs;
use crate::config::Config;
use crate::core::git::{GitOperations, GitService, SessionEnvironment};
use crate::core::ide::IdeManager;
use crate::core::session::{SessionManager, SessionStatus};
use crate::utils::{ParaError, Result};
use dialoguer::Select;
use std::env;
use std::fs;
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
    validate_resume_args(&args)?;

    let git_service = GitService::discover()?;
    let session_manager = SessionManager::new(&config);

    match args.session {
        Some(session_name) => resume_specific_session(&config, &git_service, &session_name),
        None => detect_and_resume_session(&config, &git_service, &session_manager),
    }
}

fn resume_specific_session(
    config: &Config,
    git_service: &GitService,
    session_name: &str,
) -> Result<()> {
    let session_manager = SessionManager::new(config);

    if session_manager.session_exists(session_name) {
        let mut session_state = session_manager.load_state(session_name)?;

        if !session_state.worktree_path.exists() {
            let branch_to_match = session_state.branch.clone();
            if let Some(wt) = git_service
                .list_worktrees()?
                .into_iter()
                .find(|w| w.branch == branch_to_match)
            {
                session_state.worktree_path = wt.path.clone();
                session_manager.save_state(&session_state)?;
            } else if let Some(wt) = git_service.list_worktrees()?.into_iter().find(|w| {
                w.path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with(session_name))
                    .unwrap_or(false)
            }) {
                session_state.worktree_path = wt.path.clone();
                session_manager.save_state(&session_state)?;
            } else {
                return Err(ParaError::session_not_found(format!(
                    "Session '{}' exists but worktree path '{}' not found",
                    session_name,
                    session_state.worktree_path.display()
                )));
            }
        }

        // Ensure CLAUDE.local.md exists for the session
        create_claude_local_md(&session_state.worktree_path, &session_state.name)?;

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
            return resume_specific_session(config, git_service, &candidate.name);
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
                .map(|s| s.name)
            {
                create_claude_local_md(&current_dir, &session_name)?;
            }

            launch_ide_for_session(config, &current_dir)?;
            println!("✅ Resumed current session");
            Ok(())
        }
        SessionEnvironment::MainRepository => {
            println!("Current directory is the main repository");
            list_and_select_session(config, git_service, session_manager)
        }
        SessionEnvironment::Invalid => {
            println!("Current directory is not part of a para session");
            list_and_select_session(config, git_service, session_manager)
        }
    }
}

fn list_and_select_session(
    config: &Config,
    _git_service: &GitService,
    session_manager: &SessionManager,
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

    let new_command = if has_skip_permissions {
        "claude --dangerously-skip-permissions -c"
    } else {
        "claude -c"
    };

    let updated_content = content
        .lines()
        .map(|line| {
            if line.contains("\"command\":")
                && (line.contains("claude_prompt_temp")
                    || (line.contains("$(cat") && line.contains("rm ")))
            {
                format!("      \"command\": \"{}\",", new_command)
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");

    fs::write(tasks_file, updated_content)
        .map_err(|e| ParaError::fs_error(format!("Failed to update tasks.json: {}", e)))
}

fn apply_add_continue_flag_transformation(
    tasks_file: &Path,
    has_skip_permissions: bool,
) -> Result<()> {
    let content = fs::read_to_string(tasks_file)
        .map_err(|e| ParaError::fs_error(format!("Failed to read tasks.json: {}", e)))?;

    let updated_content = if has_skip_permissions {
        content.replace(
            "claude --dangerously-skip-permissions",
            "claude --dangerously-skip-permissions -c",
        )
    } else if content.contains("\"claude\"") {
        content.replace("\"claude\"", "\"claude -c\"")
    } else {
        content.replace("\"command\": \"claude", "\"command\": \"claude -c")
    };

    fs::write(tasks_file, updated_content)
        .map_err(|e| ParaError::fs_error(format!("Failed to update tasks.json: {}", e)))
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
        super::resume_specific_session(&config, &git_service, "test4").unwrap();
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
}
