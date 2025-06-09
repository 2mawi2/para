use crate::cli::parser::DispatchArgs;
use crate::config::{Config, ConfigManager};
use crate::core::git::{GitOperations, GitService};
use crate::utils::{names::*, ParaError, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn execute(args: DispatchArgs) -> Result<()> {
    args.validate()?;

    let (session_name, prompt) = args.resolve_prompt_and_session()?;

    let config = ConfigManager::load_or_create()
        .map_err(|e| ParaError::config_error(format!("Failed to load config: {}", e)))?;

    validate_claude_code_ide(&config)?;

    let git_service = GitService::discover()
        .map_err(|e| ParaError::git_error(format!("Failed to discover git repository: {}", e)))?;

    let repo_root = git_service.repository().root.clone();

    let session_name = match session_name {
        Some(name) => {
            validate_session_name(&name)?;
            name
        }
        None => {
            // TODO: Get existing session names for collision avoidance
            // For now, use simple friendly name generation
            generate_friendly_name()
        }
    };

    let branch_name = generate_branch_name(config.get_branch_prefix());
    let session_id = session_name.clone(); // Use Docker-style names without timestamp

    let subtrees_path = repo_root.join(&config.directories.subtrees_dir);
    let session_path = subtrees_path
        .join(config.get_branch_prefix())
        .join(&session_id);

    if !subtrees_path.exists() {
        fs::create_dir_all(&subtrees_path).map_err(|e| {
            ParaError::fs_error(format!("Failed to create subtrees directory: {}", e))
        })?;
    }

    git_service
        .create_worktree(&branch_name, &session_path)
        .map_err(|e| ParaError::git_error(format!("Failed to create worktree: {}", e)))?;

    let state_dir = repo_root.join(&config.directories.state_dir);
    if !state_dir.exists() {
        fs::create_dir_all(&state_dir)
            .map_err(|e| ParaError::fs_error(format!("Failed to create state directory: {}", e)))?;
    }

    let session_state_file = state_dir.join(format!("{}.json", session_id));
    let session_state = SessionState {
        id: session_id.clone(),
        name: session_name,
        branch: branch_name,
        path: session_path.clone(),
        prompt: prompt.clone(),
        created_at: chrono::Utc::now(),
        status: SessionStatus::Active,
    };

    let state_json = serde_json::to_string_pretty(&session_state)
        .map_err(|e| ParaError::json_error(format!("Failed to serialize session state: {}", e)))?;

    fs::write(&session_state_file, state_json)
        .map_err(|e| ParaError::fs_error(format!("Failed to write session state: {}", e)))?;

    launch_claude_code(
        &config,
        &session_path,
        &prompt,
        args.dangerously_skip_permissions,
    )?;

    println!("Created session '{}' with Claude Code", session_id);
    println!("Session path: {}", session_path.display());

    Ok(())
}

fn validate_claude_code_ide(config: &Config) -> Result<()> {
    if config.ide.name.to_lowercase() != "claude" && config.ide.name.to_lowercase() != "claude-code"
    {
        return Err(ParaError::invalid_config(
            format!(
                "Dispatch command requires Claude Code IDE. Current IDE: '{}'. Run 'para config' to change IDE.",
                config.ide.name
            )
        ));
    }
    Ok(())
}

fn launch_claude_code(
    config: &Config,
    session_path: &Path,
    prompt: &str,
    skip_permissions: bool,
) -> Result<()> {
    let mut cmd = Command::new(&config.ide.command);

    cmd.current_dir(session_path);

    if !prompt.is_empty() {
        cmd.arg("--prompt").arg(prompt);
    }

    if skip_permissions {
        cmd.arg("--accept-terms");
    }

    if config.is_wrapper_enabled() {
        cmd.env("PARA_WRAPPER_MODE", "true");
        cmd.env("PARA_WRAPPER_IDE", &config.ide.wrapper.name);
    }

    match cmd.spawn() {
        Ok(mut child) => {
            if let Err(e) = child.wait() {
                eprintln!("Warning: Claude Code process error: {}", e);
            }
        }
        Err(e) => {
            return Err(ParaError::ide_error(format!(
                "Failed to launch Claude Code: {}. Check that '{}' is installed and accessible.",
                e, config.ide.command
            )));
        }
    }

    Ok(())
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct SessionState {
    id: String,
    name: String,
    branch: String,
    path: PathBuf,
    prompt: String,
    created_at: chrono::DateTime<chrono::Utc>,
    status: SessionStatus,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
enum SessionStatus {
    Active,
    Finished,
    Cancelled,
}

impl DispatchArgs {
    pub fn resolve_prompt_and_session(&self) -> Result<(Option<String>, String)> {
        match (&self.name_or_prompt, &self.prompt, &self.file) {
            (_, _, Some(file_path)) => {
                let prompt = read_file_content(file_path)?;
                Ok((self.name_or_prompt.clone(), prompt))
            }

            (Some(arg), None, None) => {
                if is_likely_file_path(arg) {
                    let prompt = read_file_content(Path::new(arg))?;
                    Ok((None, prompt))
                } else {
                    Ok((None, arg.clone()))
                }
            }

            (Some(session), Some(prompt), None) => Ok((Some(session.clone()), prompt.clone())),

            (None, None, None) => Err(ParaError::invalid_args(
                "Must provide either a prompt or a file",
            )),

            _ => Err(ParaError::invalid_args(
                "Invalid argument combination for dispatch",
            )),
        }
    }
}

fn is_likely_file_path(input: &str) -> bool {
    input.contains('/')
        || input.ends_with(".txt")
        || input.ends_with(".md")
        || input.ends_with(".prompt")
        || Path::new(input).exists()
}

fn read_file_content(path: &Path) -> Result<String> {
    if !path.exists() {
        return Err(ParaError::invalid_args(format!(
            "File does not exist: {}",
            path.display()
        )));
    }

    fs::read_to_string(path).map_err(|e| {
        ParaError::invalid_args(format!("Failed to read file {}: {}", path.display(), e))
    })
}
