pub mod commands;
pub mod completion;
pub mod parser;

#[cfg(test)]
mod tests;

pub use parser::{Cli, Commands};

use crate::config::ConfigManager;
use crate::core::docker::cleanup::ContainerCleaner;
use crate::utils::{ParaError, Result};

pub fn execute_command(cli: Cli) -> Result<()> {
    // Add debug logging for completion script detection
    if std::env::var("PARA_COMPLETION_SCRIPT").is_ok() {
        crate::utils::debug_log("Running in completion script mode");
    }
    execute_command_with_config(cli, None)
}

pub fn execute_command_with_config(
    cli: Cli,
    test_config: Option<crate::config::Config>,
) -> Result<()> {
    let config = match cli.command {
        Some(Commands::Config(_))
        | Some(Commands::Completion(_))
        | Some(Commands::Init)
        | Some(Commands::Auth(_))
        | Some(Commands::CompletionSessions)
        | Some(Commands::CompletionBranches) => None,
        Some(Commands::Monitor(_)) | None => match test_config {
            Some(cfg) => Some(cfg),
            None => Some(
                ConfigManager::load_or_create()
                    .map_err(|e| ParaError::config_error(format!("Failed to load config: {e}")))?,
            ),
        },
        _ => match test_config {
            Some(cfg) => Some(cfg),
            None => Some(
                ConfigManager::load_or_create()
                    .map_err(|e| ParaError::config_error(format!("Failed to load config: {e}")))?,
            ),
        },
    };

    // Ensure daemon is running for any command that might need it
    // Skip daemon check for commands that don't need it
    let should_start_daemon = !matches!(
        &cli.command,
        Some(Commands::Config(_))
            | Some(Commands::Completion(_))
            | Some(Commands::Init)
            | Some(Commands::Auth(_))
            | Some(Commands::CompletionSessions)
            | Some(Commands::CompletionBranches)
            | Some(Commands::Daemon(_))
    );

    if should_start_daemon {
        // Try to ensure daemon is running, but don't fail if it doesn't work
        // The daemon is a best-effort service for container support
        if let Err(e) = crate::core::daemon::client::ensure_daemon_running() {
            crate::utils::debug_log(&format!("Daemon auto-start failed: {e}"));
        }
    }

    // Trigger automatic container cleanup for common commands
    if let Some(ref config) = config {
        match &cli.command {
            Some(Commands::Start(_))
            | Some(Commands::List(_))
            | Some(Commands::Status(_))
            | Some(Commands::Finish(_)) => {
                // Run cleanup in background, ignore errors
                let cleaner = ContainerCleaner::new(config.clone());
                cleaner.maybe_cleanup_async().ok();
            }
            _ => {}
        }
    }

    match cli.command {
        Some(Commands::Start(args)) => {
            args.validate()?;
            commands::start::execute(config.unwrap(), args)
        }
        Some(Commands::Dispatch(args)) => {
            args.validate()?;
            commands::dispatch::execute(config.unwrap(), args)
        }
        Some(Commands::Finish(args)) => {
            args.validate()?;
            commands::finish::execute(config.unwrap(), args)
        }
        Some(Commands::Cancel(args)) => commands::cancel::execute(config.unwrap(), args),
        Some(Commands::Clean(args)) => commands::clean::execute(config.unwrap(), args),
        Some(Commands::List(args)) => commands::list::execute(config.unwrap(), args),
        Some(Commands::Resume(args)) => commands::resume::execute(config.unwrap(), args),
        Some(Commands::Recover(args)) => commands::recover::execute(config.unwrap(), args),
        Some(Commands::Config(args)) => commands::config::execute(args),
        Some(Commands::Completion(args)) => commands::completion::execute(args),
        Some(Commands::Init) => commands::init::execute(),
        Some(Commands::Mcp(args)) => commands::mcp::handle_mcp_command(args),
        Some(Commands::CompletionSessions) => commands::completion_sessions::execute(),
        Some(Commands::CompletionBranches) => commands::completion_branches::execute(),
        Some(Commands::Monitor(args)) => commands::monitor::execute(config.unwrap(), args),
        Some(Commands::Status(args)) => commands::status::execute(config.unwrap(), args),
        Some(Commands::Auth(args)) => commands::auth::execute(args),
        Some(Commands::Daemon(args)) => commands::daemon::execute(args),
        None => commands::monitor::execute(config.unwrap(), crate::cli::parser::MonitorArgs {}),
    }
}
