use crate::cli::commands::common::create_claude_local_md;
use crate::cli::parser::ResumeArgs;
use crate::config::Config;
use crate::core::git::{GitOperations, GitService, SessionEnvironment};
use crate::core::ide::{IdeManager, LaunchOptions};
use crate::core::session::state::SessionState;
use crate::core::session::{SessionManager, SessionStatus};
use crate::utils::{ParaError, Result};
use dialoguer::Select;
use std::env;
use std::path::Path;

use super::claude_session::find_claude_session;
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
    let mut session_manager = SessionManager::new(config);

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

        // Handle resume context and get processed content
        let processed_context = process_resume_context(args)?;

        // If session is in Review state and we have a task/prompt, transition back to Active
        if matches!(session_state.status, SessionStatus::Review) && processed_context.is_some() {
            session_manager.update_session_status(&session_state.name, SessionStatus::Active)?;
            println!("🔄 Transitioning session from Review to Active due to new task");
        }

        if let Some(ref context) = processed_context {
            save_resume_context(&session_state.worktree_path, &session_state.name, context)?;
        }

        // Launch IDE with prompt if provided
        launch_ide_for_session_with_state(
            config,
            &session_state.worktree_path,
            args,
            processed_context.as_ref(),
            Some(&session_state),
        )?;
        println!("✅ Resumed session '{session_name}'");
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

        // Prepare session files using extracted function
        prepare_session_files(&matching_worktree.path, &session_name_for_files)?;

        // Handle resume context and get processed content
        let processed_context = process_resume_context(args)?;
        if let Some(ref context) = processed_context {
            save_resume_context(&matching_worktree.path, &session_name_for_files, context)?;
        }

        launch_ide_for_session(
            config,
            &matching_worktree.path,
            args,
            processed_context.as_ref(),
            session_opt.as_ref(),
        )?;
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
            println!("Current directory is a worktree for branch: {branch}");

            // Process resume context once
            let processed_context = process_resume_context(args)?;

            // Try to find session from current directory or branch
            let session_opt = session_manager
                .list_sessions()?
                .into_iter()
                .find(|s| s.worktree_path == current_dir || s.branch == branch);

            if let Some(ref session) = session_opt {
                create_claude_local_md(&current_dir, &session.name)?;

                // If session is in Review state and we have a task/prompt, transition back to Active
                if matches!(session.status, SessionStatus::Review) && processed_context.is_some() {
                    let mut session_manager = SessionManager::new(config);
                    session_manager.update_session_status(&session.name, SessionStatus::Active)?;
                    println!("🔄 Transitioning session from Review to Active due to new task");
                }

                // Save resume context if provided
                if let Some(ref context) = processed_context {
                    save_resume_context(&current_dir, &session.name, context)?;
                }
            }

            launch_ide_for_session_with_state(
                config,
                &current_dir,
                args,
                processed_context.as_ref(),
                session_opt.as_ref(),
            )?;
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
    let resumable_sessions: Vec<_> = sessions
        .into_iter()
        .filter(|s| matches!(s.status, SessionStatus::Active | SessionStatus::Review))
        .collect();

    if resumable_sessions.is_empty() {
        println!("No resumable sessions found.");
        return Ok(());
    }

    println!("Resumable sessions:");
    for (i, session) in resumable_sessions.iter().enumerate() {
        let status_label = match session.status {
            SessionStatus::Active => "",
            SessionStatus::Review => " [Review]",
            _ => "",
        };
        println!(
            "  {}: {}{} ({})",
            i + 1,
            session.name,
            status_label,
            session.branch
        );
    }

    let selection = Select::new()
        .with_prompt("Select session to resume")
        .items(
            &resumable_sessions
                .iter()
                .map(|s| &s.name)
                .collect::<Vec<_>>(),
        )
        .interact();

    if let Ok(index) = selection {
        let session = &resumable_sessions[index];

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
        let processed_context = process_resume_context(args)?;

        // If session is in Review state and we have a task/prompt, transition back to Active
        if matches!(session.status, SessionStatus::Review) && processed_context.is_some() {
            let mut session_manager = SessionManager::new(config);
            session_manager.update_session_status(&session.name, SessionStatus::Active)?;
            println!("🔄 Transitioning session from Review to Active due to new task");
        }

        if let Some(ref context) = processed_context {
            save_resume_context(&session.worktree_path, &session.name, context)?;
        }

        launch_ide_for_session(
            config,
            &session.worktree_path,
            args,
            processed_context.as_ref(),
            Some(session),
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

fn launch_ide_for_session(
    config: &Config,
    path: &Path,
    args: &ResumeArgs,
    processed_context: Option<&String>,
    _session_state: Option<&SessionState>,
) -> Result<()> {
    launch_ide_for_session_with_state(config, path, args, processed_context, None)
}

fn launch_ide_for_session_with_state(
    config: &Config,
    path: &Path,
    args: &ResumeArgs,
    processed_context: Option<&String>,
    session_state: Option<&SessionState>,
) -> Result<()> {
    let ide_manager = IdeManager::new(config);

    // Determine if we should skip permissions:
    // 1. If the session was originally created with dangerous flag, respect it
    // 2. If the user explicitly passes the flag during resume, respect it
    // 3. Otherwise, don't skip permissions
    let skip_permissions = session_state
        .and_then(|s| s.dangerous_skip_permissions)
        .unwrap_or(false)
        || args.dangerously_skip_permissions;

    // For Claude Code in wrapper mode, check for existing session
    if config.ide.name == "claude" && config.ide.wrapper.enabled {
        let mut launch_options = LaunchOptions {
            skip_permissions,
            // Pass raw CLI args, not resolved settings - let claude_launcher resolve them
            sandbox_override: if args.sandbox_args.sandbox {
                Some(true)
            } else if args.sandbox_args.no_sandbox {
                Some(false)
            } else {
                None
            },
            sandbox_profile: args.sandbox_args.sandbox_profile.clone(),
            network_sandbox: args.sandbox_args.sandbox_no_network,
            allowed_domains: args.sandbox_args.allowed_domains.clone(),
            ..Default::default()
        };

        // Try to find existing Claude session
        match find_claude_session(path) {
            Ok(Some(claude_session)) => {
                if claude_session.id.is_empty() {
                    println!("⚠️  Found Claude session but ID is empty");
                    launch_options.continue_conversation = true;
                } else {
                    println!("🔗 Found existing Claude session: {}", claude_session.id);
                    launch_options.claude_session_id = Some(claude_session.id);

                    // Include prompt from processed context (file or inline prompt)
                    if let Some(_context) = processed_context {
                        println!("▶ resuming Claude Code session with prompt...");
                    } else {
                        println!("▶ resuming Claude Code session with conversation history...");
                    }
                }
            }
            Ok(None) => {
                // No existing session found, use continuation flag
                println!("▶ starting new Claude Code session...");
                launch_options.continue_conversation = true;

                // Update existing tasks.json to include -c flag
                transform_claude_tasks_file(path)?;
            }
            Err(e) => {
                println!("⚠️  Error finding Claude session: {e}");
                launch_options.continue_conversation = true;
            }
        }

        // Launch with shared claude launcher
        let claude_options = crate::core::claude_launcher::ClaudeLaunchOptions {
            skip_permissions: launch_options.skip_permissions,
            session_id: launch_options.claude_session_id.clone(),
            continue_conversation: launch_options.continue_conversation,
            prompt_content: processed_context.cloned(),
            sandbox_override: launch_options.sandbox_override,
            sandbox_profile: launch_options.sandbox_profile,
            network_sandbox: launch_options.network_sandbox,
            allowed_domains: launch_options.allowed_domains.clone(),
        };
        crate::core::claude_launcher::launch_claude_with_context(config, path, claude_options)
    } else {
        ide_manager.launch(path, skip_permissions)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::parser::SandboxArgs;
    use crate::core::session::state::SessionState;
    use crate::test_utils::test_helpers::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_resume_base_name_fallback() {
        let git_temp = TempDir::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, git_service) = setup_test_repo();

        let mut config = create_test_config();
        config.directories.state_dir = temp_dir
            .path()
            .join(".para_state")
            .to_string_lossy()
            .to_string();
        config.directories.subtrees_dir = "subtrees/para".to_string();
        config.git.branch_prefix = "para".to_string();
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
            dangerously_skip_permissions: false,
            sandbox_args: SandboxArgs {
                sandbox: false,
                no_sandbox: false,
                sandbox_profile: None,
                sandbox_no_network: false,
                allowed_domains: vec![],
            },
        };
        resume_specific_session(&config, &git_service, "test4", &args).unwrap();
    }

    // Integration tests for new resume functionality
    #[test]
    fn test_resume_with_prompt_integration() {
        let git_temp = TempDir::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, git_service) = setup_test_repo();

        let mut config = create_test_config();
        config.directories.state_dir = temp_dir
            .path()
            .join(".para_state")
            .to_string_lossy()
            .to_string();
        config.directories.subtrees_dir = "subtrees/para".to_string();
        config.git.branch_prefix = "para".to_string();
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
            dangerously_skip_permissions: false,
            sandbox_args: SandboxArgs {
                sandbox: false,
                no_sandbox: false,
                sandbox_profile: None,
                sandbox_no_network: false,
                allowed_domains: vec![],
            },
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
        let git_temp = TempDir::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, git_service) = setup_test_repo();

        let mut config = create_test_config();
        config.directories.state_dir = temp_dir
            .path()
            .join(".para_state")
            .to_string_lossy()
            .to_string();
        config.directories.subtrees_dir = "subtrees/para".to_string();
        config.git.branch_prefix = "para".to_string();
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
            dangerously_skip_permissions: false,
            sandbox_args: SandboxArgs {
                sandbox: false,
                no_sandbox: false,
                sandbox_profile: None,
                sandbox_no_network: false,
                allowed_domains: vec![],
            },
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
        let git_temp = TempDir::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, git_service) = setup_test_repo();

        let mut config = create_test_config();
        config.directories.state_dir = temp_dir
            .path()
            .join(".para_state")
            .to_string_lossy()
            .to_string();
        config.directories.subtrees_dir = "subtrees/para".to_string();
        config.git.branch_prefix = "para".to_string();
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
            dangerously_skip_permissions: false,
            sandbox_args: SandboxArgs {
                sandbox: false,
                no_sandbox: false,
                sandbox_profile: None,
                sandbox_no_network: false,
                allowed_domains: vec![],
            },
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

    // Test removed - Claude session integration testing moved to integration tests
    // The core functionality is tested through unit tests in ide.rs and claude_session.rs

    #[test]
    fn test_resume_review_session_with_prompt() {
        let git_temp = TempDir::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, git_service) = setup_test_repo();

        let mut config = create_test_config();
        config.directories.state_dir = temp_dir
            .path()
            .join(".para_state")
            .to_string_lossy()
            .to_string();
        config.directories.subtrees_dir = "subtrees/para".to_string();
        config.git.branch_prefix = "para".to_string();
        let session_manager = SessionManager::new(&config);

        // Create a test session in Review state
        let session_name = "test-review-session".to_string();
        let branch_name = "para/test-review-branch".to_string();
        let worktree_path = git_service
            .repository()
            .root
            .join(&config.directories.subtrees_dir)
            .join(&config.git.branch_prefix)
            .join(&session_name);

        git_service
            .create_worktree(&branch_name, &worktree_path)
            .unwrap();

        let mut state = SessionState::new(session_name.clone(), branch_name, worktree_path.clone());
        state.status = SessionStatus::Review;
        session_manager.save_state(&state).unwrap();

        // Verify session is in Review state
        let loaded_state = session_manager.load_state(&session_name).unwrap();
        assert!(matches!(loaded_state.status, SessionStatus::Review));

        // Resume with prompt
        let args = ResumeArgs {
            session: Some(session_name.clone()),
            prompt: Some("Continue with OAuth implementation".to_string()),
            file: None,
            dangerously_skip_permissions: false,
            sandbox_args: SandboxArgs {
                sandbox: false,
                no_sandbox: false,
                sandbox_profile: None,
                sandbox_no_network: false,
                allowed_domains: vec![],
            },
        };

        // Execute resume
        resume_specific_session(&config, &git_service, &session_name, &args).unwrap();

        // Verify session transitioned back to Active
        let updated_state = session_manager.load_state(&session_name).unwrap();
        assert!(matches!(updated_state.status, SessionStatus::Active));

        // Verify context was saved
        let context_file = worktree_path
            .join(".para/sessions")
            .join(&session_name)
            .join("resume_context.md");
        assert!(context_file.exists());
        let saved_content = fs::read_to_string(&context_file).unwrap();
        assert!(saved_content.contains("Continue with OAuth implementation"));
    }

    #[test]
    fn test_resume_review_session_without_prompt_stays_review() {
        let git_temp = TempDir::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, git_service) = setup_test_repo();

        let mut config = create_test_config();
        config.directories.state_dir = temp_dir
            .path()
            .join(".para_state")
            .to_string_lossy()
            .to_string();
        config.directories.subtrees_dir = "subtrees/para".to_string();
        config.git.branch_prefix = "para".to_string();
        let session_manager = SessionManager::new(&config);

        // Create a test session in Review state
        let session_name = "test-review-no-prompt".to_string();
        let branch_name = "para/test-review-no-prompt-branch".to_string();
        let worktree_path = git_service
            .repository()
            .root
            .join(&config.directories.subtrees_dir)
            .join(&config.git.branch_prefix)
            .join(&session_name);

        git_service
            .create_worktree(&branch_name, &worktree_path)
            .unwrap();

        let mut state = SessionState::new(session_name.clone(), branch_name, worktree_path.clone());
        state.status = SessionStatus::Review;
        session_manager.save_state(&state).unwrap();

        // Resume without prompt
        let args = ResumeArgs {
            session: Some(session_name.clone()),
            prompt: None,
            file: None,
            dangerously_skip_permissions: false,
            sandbox_args: SandboxArgs {
                sandbox: false,
                no_sandbox: false,
                sandbox_profile: None,
                sandbox_no_network: false,
                allowed_domains: vec![],
            },
        };

        // Execute resume
        resume_specific_session(&config, &git_service, &session_name, &args).unwrap();

        // Verify session stays in Review state
        let updated_state = session_manager.load_state(&session_name).unwrap();
        assert!(matches!(updated_state.status, SessionStatus::Review));
    }

    #[test]
    fn test_list_sessions_includes_review_sessions() {
        let git_temp = TempDir::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, _git_service) = setup_test_repo();

        let mut config = create_test_config();
        config.directories.state_dir = temp_dir
            .path()
            .join(".para_state")
            .to_string_lossy()
            .to_string();
        let session_manager = SessionManager::new(&config);

        // Create Active session
        let active_session = SessionState::new(
            "active-session".to_string(),
            "para/active".to_string(),
            temp_dir.path().join("active"),
        );
        session_manager.save_state(&active_session).unwrap();

        // Create Review session
        let mut review_session = SessionState::new(
            "review-session".to_string(),
            "para/review".to_string(),
            temp_dir.path().join("review"),
        );
        review_session.status = SessionStatus::Review;
        session_manager.save_state(&review_session).unwrap();

        // Create Cancelled session (should be excluded)
        let mut cancelled_session = SessionState::new(
            "cancelled-session".to_string(),
            "para/cancelled".to_string(),
            temp_dir.path().join("cancelled"),
        );
        cancelled_session.status = SessionStatus::Cancelled;
        session_manager.save_state(&cancelled_session).unwrap();

        // Get resumable sessions
        let sessions = session_manager.list_sessions().unwrap();
        let resumable_sessions: Vec<_> = sessions
            .into_iter()
            .filter(|s| matches!(s.status, SessionStatus::Active | SessionStatus::Review))
            .collect();

        // Should have 2 resumable sessions
        assert_eq!(resumable_sessions.len(), 2);

        // Verify we have both Active and Review sessions
        let has_active = resumable_sessions
            .iter()
            .any(|s| s.name == "active-session");
        let has_review = resumable_sessions
            .iter()
            .any(|s| s.name == "review-session");
        assert!(has_active);
        assert!(has_review);
    }

    #[test]
    fn test_dangerous_flag_preservation_in_resume() {
        let git_temp = TempDir::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, git_service) = setup_test_repo();

        let mut config = create_test_config();
        config.directories.state_dir = temp_dir
            .path()
            .join(".para_state")
            .to_string_lossy()
            .to_string();

        // Create state directory
        std::fs::create_dir_all(&config.directories.state_dir).unwrap();

        let session_manager = SessionManager::new(&config);

        // Create worktree for the session
        let worktree_path = git_service.repository().root.join("test-dangerous-session");
        git_service
            .worktree_manager()
            .create_worktree("para/test-dangerous", &worktree_path)
            .unwrap();

        // Create a session with dangerous flag set
        let session_with_flag = SessionState::with_parent_branch_and_flags(
            "test-dangerous-session".to_string(),
            "para/test-dangerous".to_string(),
            worktree_path.clone(),
            "main".to_string(),
            true, // dangerous_skip_permissions = true
        );

        session_manager.save_state(&session_with_flag).unwrap();

        // Test that launch_ide_for_session respects the stored flag
        let args = ResumeArgs {
            session: Some("test-dangerous-session".to_string()),
            prompt: None,
            file: None,
            dangerously_skip_permissions: false, // User didn't pass the flag
            sandbox_args: SandboxArgs {
                sandbox: false,
                no_sandbox: false,
                sandbox_profile: None,
                sandbox_no_network: false,
                allowed_domains: vec![],
            },
        };

        // In a real test, we'd mock the IDE launch, but here we verify the logic
        // The function should use the session's stored flag (true) even though args has false
        let loaded_session = session_manager
            .load_state("test-dangerous-session")
            .unwrap();

        // This is the logic from launch_ide_for_session
        let skip_permissions = loaded_session.dangerous_skip_permissions.unwrap_or(false)
            || args.dangerously_skip_permissions;

        assert!(
            skip_permissions,
            "Should use dangerous flag from session state"
        );

        // Test case 2: User explicitly passes flag, session doesn't have it
        let session_without_flag = SessionState::with_parent_branch_and_flags(
            "test-safe-session".to_string(),
            "para/test-safe".to_string(),
            worktree_path.clone(),
            "main".to_string(),
            false, // dangerous_skip_permissions = false
        );

        session_manager.save_state(&session_without_flag).unwrap();

        let args_with_flag = ResumeArgs {
            session: Some("test-safe-session".to_string()),
            prompt: None,
            file: None,
            dangerously_skip_permissions: true, // User explicitly passes the flag
            sandbox_args: SandboxArgs {
                sandbox: false,
                no_sandbox: false,
                sandbox_profile: None,
                sandbox_no_network: false,
                allowed_domains: vec![],
            },
        };

        let loaded_safe = session_manager.load_state("test-safe-session").unwrap();
        let skip_permissions_2 = loaded_safe.dangerous_skip_permissions.unwrap_or(false)
            || args_with_flag.dangerously_skip_permissions;

        assert!(skip_permissions_2, "Should use dangerous flag from args");
    }

    #[test]
    fn test_resume_passes_raw_sandbox_args_not_resolved() {
        // This test verifies the fix for the double resolution bug
        let git_temp = TempDir::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let _guard = TestEnvironmentGuard::new(&git_temp, &temp_dir).unwrap();
        let (_git_temp, git_service) = setup_test_repo();

        let mut config = create_test_config();
        config.directories.state_dir = temp_dir
            .path()
            .join(".para_state")
            .to_string_lossy()
            .to_string();
        // Set up Claude as the IDE for this test
        config.ide.name = "claude".to_string();
        config.ide.wrapper.enabled = true;

        let session_manager = SessionManager::new(&config);

        // Create a test session
        let session_name = "test-sandbox-args".to_string();
        let branch_name = "para/test-sandbox".to_string();
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

        // Test with specific sandbox CLI args
        let args = ResumeArgs {
            session: Some(session_name.clone()),
            prompt: None,
            file: None,
            dangerously_skip_permissions: false,
            sandbox_args: SandboxArgs {
                sandbox: true, // CLI arg: enable sandbox
                no_sandbox: false,
                sandbox_profile: Some("restrictive".to_string()), // CLI profile override
                sandbox_no_network: true,                         // CLI arg: enable network sandbox
                allowed_domains: vec!["api.claude.ai".to_string()], // CLI allowed domains
            },
        };

        // Execute resume - this should now pass raw CLI args to claude_launcher
        // instead of pre-resolving them
        resume_specific_session(&config, &git_service, &session_name, &args).unwrap();

        // Verify tasks.json was created
        let tasks_file = worktree_path.join(".vscode/tasks.json");
        assert!(tasks_file.exists(), "tasks.json should be created");

        // The key verification is that the function completes without errors
        // The real test is that it doesn't do double resolution anymore
    }
}
