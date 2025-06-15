use crate::cli::parser::ResumeArgs;
use crate::config::Config;
use crate::core::git::{GitOperations, GitService, SessionEnvironment};
use crate::core::ide::IdeManager;
use crate::core::session::{SessionManager, SessionStatus};
use crate::utils::{ParaError, Result};
use dialoguer::Select;
use std::env;
use std::path::Path;

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

        launch_ide_for_session(config, &session.worktree_path)?;
        println!("✅ Resumed session '{}'", session.name);
    }

    Ok(())
}

fn launch_ide_for_session(config: &Config, path: &Path) -> Result<()> {
    let ide_manager = IdeManager::new(config);

    // Check if we should continue conversation
    let continue_conversation = should_continue_conversation(config, &ide_manager, path);

    if continue_conversation {
        println!("▶ resuming session with conversation continuation...");
        ide_manager.launch_with_options(path, false, true)
    } else {
        ide_manager.launch(path, false)
    }
}

fn should_continue_conversation(config: &Config, ide_manager: &IdeManager, path: &Path) -> bool {
    // Only continue conversation if:
    // 1. We're using Claude Code
    // 2. The IDE is not already running for this session

    if config.ide.name != "claude" {
        return false;
    }

    // Extract session name from path
    if let Some(session_name) = path.file_name().and_then(|n| n.to_str()) {
        // Check if IDE is already running for this session
        !ide_manager.is_ide_running_for_session(session_name)
    } else {
        // If we can't determine session name, don't continue conversation
        false
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
                    enabled: false,
                    name: String::new(),
                    command: String::new(),
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
        assert!(super::resume_specific_session(&config, &git_service, "test4").is_ok());
    }

    #[test]
    fn test_should_continue_conversation() {
        let config = Config {
            ide: IdeConfig {
                name: "claude".into(),
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
                state_dir: ".para_state".into(),
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

        let ide_manager = crate::core::ide::IdeManager::new(&config);
        let temp_dir = TempDir::new().unwrap();
        let session_path = temp_dir.path().join("test-session");
        fs::create_dir_all(&session_path).unwrap();

        // Test with Claude Code - should return true (in test mode always false)
        let should_continue =
            super::should_continue_conversation(&config, &ide_manager, &session_path);
        assert!(!should_continue); // In test mode, IDE is never running

        // Test with non-Claude IDE
        let mut non_claude_config = config.clone();
        non_claude_config.ide.name = "vscode".into();
        let ide_manager = crate::core::ide::IdeManager::new(&non_claude_config);
        let should_continue =
            super::should_continue_conversation(&non_claude_config, &ide_manager, &session_path);
        assert!(!should_continue);
    }
}
