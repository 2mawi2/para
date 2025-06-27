use crate::cli::commands::common::create_claude_local_md;
use crate::cli::parser::ResumeArgs;
use crate::config::Config;
use crate::core::git::{GitOperations, GitService, SessionEnvironment};
use crate::core::ide::IdeManager;
use crate::core::session::state::SessionState;
use crate::core::session::{SessionManager, SessionStatus};
use crate::utils::{ParaError, Result};
use dialoguer::Select;
use std::env;
use std::path::Path;

use super::context::{process_resume_context, save_resume_context};
use super::repair::repair_worktree_path;
use super::task_transform::transform_claude_tasks_file;

/// Session-specific resume operations
pub fn resume_specific_session(
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
        prepare_session_files(&session_state.worktree_path, &session_state.name)?;

        // Handle resume context
        handle_resume_context(&session_state.worktree_path, &session_state.name, args)?;

        // Launch IDE
        launch_ide_for_session(
            config,
            &session_state.worktree_path,
            session_state.dangerous_skip_permissions.unwrap_or(false),
        )?;
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

        // Try to find session from matching worktree
        let session_opt = session_manager.list_sessions()?.into_iter().find(|s| {
            s.worktree_path == matching_worktree.path || s.branch == matching_worktree.branch
        });

        let session_name_for_files = session_opt
            .as_ref()
            .map(|s| s.name.clone())
            .unwrap_or_else(|| session_name.to_string());

        let skip_permissions = session_opt
            .as_ref()
            .and_then(|s| s.dangerous_skip_permissions)
            .unwrap_or(false);

        // Prepare session files using extracted function
        prepare_session_files(&matching_worktree.path, &session_name_for_files)?;

        // Handle resume context using extracted function
        handle_resume_context(&matching_worktree.path, &session_name_for_files, args)?;

        launch_ide_for_session(config, &matching_worktree.path, skip_permissions)?;
        println!(
            "✅ Resumed session at '{}'",
            matching_worktree.path.display()
        );
    }

    Ok(())
}

/// Detect and resume session from current directory
pub fn detect_and_resume_session(
    config: &Config,
    git_service: &GitService,
    session_manager: &SessionManager,
    args: &ResumeArgs,
) -> Result<()> {
    let current_dir = env::current_dir()?;

    match git_service.validate_session_environment(&current_dir)? {
        SessionEnvironment::Worktree { branch, .. } => {
            println!("Current directory is a worktree for branch: {}", branch);

            // Try to find session from current directory or branch
            let session_opt = session_manager
                .list_sessions()?
                .into_iter()
                .find(|s| s.worktree_path == current_dir || s.branch == branch);

            if let Some(ref session) = session_opt {
                create_claude_local_md(&current_dir, &session.name)?;

                // Process and save resume context if provided
                if let Some(context) = process_resume_context(args)? {
                    save_resume_context(&current_dir, &session.name, &context)?;
                }
            }

            let skip_permissions = session_opt
                .as_ref()
                .and_then(|s| s.dangerous_skip_permissions)
                .unwrap_or(false);

            launch_ide_for_session(config, &current_dir, skip_permissions)?;
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

/// List and select session interactively
pub fn list_and_select_session(
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

        launch_ide_for_session(
            config,
            &session.worktree_path,
            session.dangerous_skip_permissions.unwrap_or(false),
        )?;
        println!("✅ Resumed session '{}'", session.name);
    }

    Ok(())
}

/// Validate session exists and return state if found
pub fn validate_session_exists(
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

fn prepare_session_files(worktree_path: &Path, session_name: &str) -> Result<()> {
    // Ensure CLAUDE.local.md exists for the session
    create_claude_local_md(worktree_path, session_name)?;
    Ok(())
}

fn handle_resume_context(
    worktree_path: &Path,
    session_name: &str,
    args: &ResumeArgs,
) -> Result<()> {
    // Process and save resume context if provided
    if let Some(context) = process_resume_context(args)? {
        save_resume_context(worktree_path, session_name, &context)?;
    }
    Ok(())
}

fn launch_ide_for_session(config: &Config, path: &Path, skip_permissions: bool) -> Result<()> {
    let ide_manager = IdeManager::new(config);

    // For Claude Code in wrapper mode, always use continuation flag when resuming
    if config.ide.name == "claude" && config.ide.wrapper.enabled {
        println!("▶ resuming Claude Code session with conversation continuation...");
        // Update existing tasks.json to include -c flag
        transform_claude_tasks_file(path)?;
        ide_manager.launch_with_options(path, skip_permissions, true)
    } else {
        ide_manager.launch(path, skip_permissions)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{
        Config, DirectoryConfig, GitConfig, IdeConfig, SessionConfig, WrapperConfig,
    };
    use crate::core::session::state::SessionState;
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
            docker: None,
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

        let state = SessionState::new(session_full.clone(), branch_name, worktree_path.clone());
        session_manager.save_state(&state).unwrap();

        // now resume with base name
        let args = ResumeArgs {
            session: Some("test4".to_string()),
            prompt: None,
            file: None,
        };
        resume_specific_session(&config, &git_service, "test4", &args).unwrap();
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

        let state = SessionState::new(session_name.clone(), branch_name, worktree_path.clone());
        session_manager.save_state(&state).unwrap();

        // Resume with prompt
        let args = ResumeArgs {
            session: Some(session_name.clone()),
            prompt: Some("Continue implementing the feature".to_string()),
            file: None,
        };

        // Execute resume (with echo IDE it won't actually launch anything)
        resume_specific_session(&config, &git_service, &session_name, &args).unwrap();

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

        let state = SessionState::new(session_name.clone(), branch_name, worktree_path.clone());
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
        resume_specific_session(&config, &git_service, &session_name, &args).unwrap();

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

        let state = SessionState::new(session_name.clone(), branch_name, worktree_path.clone());
        session_manager.save_state(&state).unwrap();

        // Resume without any additional context (old behavior)
        let args = ResumeArgs {
            session: Some(session_name.clone()),
            prompt: None,
            file: None,
        };

        // Execute resume - should work exactly as before
        let result = resume_specific_session(&config, &git_service, &session_name, &args);
        assert!(result.is_ok());

        // Verify no context file was created
        let context_file = worktree_path
            .join(".para/sessions")
            .join(&session_name)
            .join("resume_context.md");
        assert!(!context_file.exists());
    }
}
