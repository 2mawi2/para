use crate::cli::parser::DispatchArgs;
use crate::config::{Config, ConfigManager};
use crate::core::git::{GitOperations, GitService};
use crate::core::session::SessionManager;
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

    let mut session_manager = SessionManager::new(config.clone())?;

    let session_name = match session_name {
        Some(name) => {
            validate_session_name(&name)?;
            name
        }
        None => generate_friendly_name(),
    };

    let session_state = session_manager.create_session(session_name.clone(), None)?;

    launch_claude_code(
        &config,
        &session_state.worktree_path,
        &prompt,
        args.dangerously_skip_permissions,
    )?;

    println!("Created session '{}' with Claude Code", session_state.name);
    println!("Session path: {}", session_state.worktree_path.display());

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

// Removed old SessionState and SessionStatus - now using unified session system

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
