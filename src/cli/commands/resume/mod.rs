use crate::cli::parser::ResumeArgs;
use crate::cli::terminal_mode::{
    self, ResumeResponse, SessionInfo as TerminalSessionInfo, TerminalConfig,
};
use crate::config::Config;
use crate::core::git::GitService;
use crate::core::session::SessionManager;
use crate::utils::{ParaError, Result};
use std::collections::HashMap;

mod claude_session;
mod context;
mod repair;
mod session;
mod task_transform;

// Public API is exposed through the execute function only

/// Main entry point - orchestrates resume logic
pub fn execute(config: Config, args: ResumeArgs) -> Result<()> {
    args.validate()?;
    validate_resume_args(&args)?;

    // Handle terminal mode
    if args.terminal {
        return execute_terminal_mode(config, args);
    }

    let git_service = GitService::discover()?;
    let session_manager = SessionManager::new(&config);

    match &args.session {
        Some(session_name) => {
            session::resume_specific_session(&config, &git_service, session_name, &args)
        }
        None => session::detect_and_resume_session(&config, &git_service, &session_manager, &args),
    }
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

/// Execute resume command in terminal mode
fn execute_terminal_mode(config: Config, args: ResumeArgs) -> Result<()> {
    // Validate terminal mode restrictions (no sandboxing in terminal mode)
    terminal_mode::validate_terminal_mode_args(
        args.terminal,
        false, // container not supported in resume
        args.sandbox_args.sandbox,
        args.sandbox_args.sandbox_no_network,
    )
    .map_err(ParaError::invalid_args)?;

    let git_service = GitService::discover()?;
    let session_manager = SessionManager::new(&config);

    // Determine session to resume
    let session_name = match &args.session {
        Some(name) => name.clone(),
        None => {
            // Auto-detect from current directory
            detect_session_from_cwd(&git_service, &session_manager)?
        }
    };

    // Get session info
    let session_info = session_manager
        .list_sessions()?
        .into_iter()
        .find(|s| s.name == session_name)
        .ok_or_else(|| ParaError::session_not_found(&session_name))?;

    // Get worktree path
    let worktree_path = session_info.worktree_path.clone();

    // Prepare resume prompt if provided
    let resume_prompt = context::process_resume_context(&args)?;

    // Build terminal configuration
    let mut env_vars = HashMap::new();
    env_vars.insert("PARA_SESSION".to_string(), session_name.clone());
    env_vars.insert(
        "PARA_WORKTREE".to_string(),
        worktree_path.display().to_string(),
    );
    env_vars.insert("PARA_RESUME".to_string(), "true".to_string());

    // Try to detect Claude session ID if available
    let claude_session_id =
        claude_session::find_claude_session(&worktree_path)?.map(|session| session.id);

    // Build AI command and arguments
    let ai_command = config.ide.command.clone();
    let mut ai_args = Vec::new();

    // Add Claude-specific flags if applicable
    if config.ide.name.to_lowercase() == "claude" || config.ide.name.to_lowercase() == "claude code"
    {
        if args.dangerously_skip_permissions {
            ai_args.push("--dangerously-skip-permissions".to_string());
        }

        // Add resume flag if Claude session ID is available
        if let Some(ref session_id) = claude_session_id {
            ai_args.push("--resume".to_string());
            ai_args.push(session_id.clone());
        }

        // Add resume prompt if provided
        if let Some(ref prompt) = resume_prompt {
            ai_args.push(prompt.clone());
        }
    }

    let terminal_config = TerminalConfig {
        working_directory: worktree_path.clone(),
        ai_command: ai_command.clone(),
        ai_args,
        initial_prompt: None,
        resume_prompt: resume_prompt.clone(),
        claude_session_id: claude_session_id.clone(),
        environment: env_vars,
        note: if resume_prompt.is_some() {
            Some("Resuming with additional context".to_string())
        } else if claude_session_id.is_some() {
            Some("Resuming existing Claude session".to_string())
        } else {
            Some("Resuming existing session".to_string())
        },
    };

    // Check for uncommitted changes (this checks the current repository)
    // Note: In terminal mode we're just providing info, not switching to the worktree
    let has_uncommitted_changes = false; // Default to false since we're not in the worktree

    let response = ResumeResponse {
        success: true,
        action: if resume_prompt.is_some() {
            "resume_with_context".to_string()
        } else if claude_session_id.is_some() {
            "resume_claude_session".to_string()
        } else {
            "resume_interactive".to_string()
        },
        session: TerminalSessionInfo {
            name: session_name.clone(),
            id: session_name.clone(),
            worktree_path: worktree_path.clone(),
            branch: session_info.branch.clone(),
            base_branch: "main".to_string(), // Default to main, will be determined by session
            has_uncommitted_changes: Some(has_uncommitted_changes),
        },
        terminal_config,
        metadata: terminal_mode::build_metadata("resume"),
    };

    // Output JSON response
    println!("{}", serde_json::to_string_pretty(&response).unwrap());
    Ok(())
}

/// Helper function to detect session from current working directory
fn detect_session_from_cwd(
    git_service: &GitService,
    session_manager: &SessionManager,
) -> Result<String> {
    let current_dir = std::env::current_dir()?;

    match git_service.validate_session_environment(&current_dir)? {
        crate::core::git::SessionEnvironment::Worktree { branch, .. } => {
            // Try to find session from current directory or branch
            let session_opt = session_manager
                .list_sessions()?
                .into_iter()
                .find(|s| s.worktree_path == current_dir || s.branch == branch);

            session_opt.map(|s| s.name).ok_or_else(|| {
                ParaError::session_not_found(&format!(
                    "No session found for current worktree (branch: {branch})"
                ))
            })
        }
        _ => Err(ParaError::invalid_args(
            "Current directory is not a para session worktree. Please specify a session name.",
        )),
    }
}

#[cfg(test)]
mod tests {
    use crate::cli::parser::{ResumeArgs, SandboxArgs};
    use std::path::PathBuf;

    #[test]
    fn test_resume_args_validate() {
        // Test valid cases
        let args = ResumeArgs {
            session: None,
            prompt: Some("test".to_string()),
            file: None,
            dangerously_skip_permissions: false,
            sandbox_args: SandboxArgs {
                sandbox: false,
                no_sandbox: false,
                sandbox_profile: None,
                sandbox_no_network: false,
                allowed_domains: vec![],
            },
            terminal: false,
        };
        assert!(args.validate().is_ok());

        let args = ResumeArgs {
            session: None,
            prompt: None,
            file: Some(PathBuf::from("test.md")),
            dangerously_skip_permissions: false,
            sandbox_args: SandboxArgs {
                sandbox: false,
                no_sandbox: false,
                sandbox_profile: None,
                sandbox_no_network: false,
                allowed_domains: vec![],
            },
            terminal: false,
        };
        assert!(args.validate().is_ok());

        // Test invalid case - both prompt and file
        let args = ResumeArgs {
            session: None,
            prompt: Some("test".to_string()),
            file: Some(PathBuf::from("test.md")),
            dangerously_skip_permissions: false,
            sandbox_args: SandboxArgs {
                sandbox: false,
                no_sandbox: false,
                sandbox_profile: None,
                sandbox_no_network: false,
                allowed_domains: vec![],
            },
            terminal: false,
        };
        assert!(args.validate().is_err());
        assert!(args
            .validate()
            .unwrap_err()
            .to_string()
            .contains("Cannot specify both"));
    }
}
